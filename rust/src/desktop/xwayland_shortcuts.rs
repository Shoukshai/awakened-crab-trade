//! XWayland-only shortcuts. Changes acquire new grabs before releasing old ones.
use crate::hotkeys::{Action, Hotkeys};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{Allow, AtomEnum, ConnectionExt, GrabMode, KeyButMask, ModMask},
        Event,
    },
    rust_connection::RustConnection,
    CURRENT_TIME,
};
type Binding = (u8, ModMask, Action);
struct Bindings {
    revision: u64,
    keys: Vec<Binding>,
}
pub struct Backend {
    connection: RustConnection,
    root: u32,
    ignored: u16,
    masks: Vec<u16>,
    bindings: Mutex<Bindings>,
    update_gate: Mutex<()>,
    alive: AtomicBool,
}
impl Backend {
    pub fn new(hotkeys: &Hotkeys) -> Result<Self, String> {
        let (connection, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
        let root = connection.setup().roots[screen].root;
        let mut ignored = ModMask::LOCK;
        if let Ok(numlock) = keycode(&connection, 0xff7f) {
            let mods = connection
                .get_modifier_mapping()
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?;
            let per_mod = mods.keycodes.len() / 8;
            if per_mod > 0 {
                for (index, keys) in mods.keycodes.chunks(per_mod).enumerate() {
                    if keys.contains(&numlock) {
                        ignored |= ModMask::from(1u16 << index);
                    }
                }
            }
        }
        let ignored = u16::from(ignored);
        let backend = Self {
            connection,
            root,
            ignored,
            masks: (0..256u16).filter(|m| m & !ignored == 0).collect(),
            bindings: Mutex::new(Bindings {
                revision: 0,
                keys: Vec::new(),
            }),
            update_gate: Mutex::new(()),
            alive: AtomicBool::new(true),
        };
        backend.replace(hotkeys, || Ok(()))?;
        Ok(backend)
    }
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
    fn native(&self, hotkeys: &Hotkeys) -> Result<Vec<Binding>, String> {
        let mut bindings = Vec::new();
        let mut seen = HashSet::new();
        for (key, command) in hotkeys.bindings()? {
            let mut mods = ModMask::default();
            for (yes, mask) in [
                (key.ctrl, ModMask::CONTROL),
                (key.shift, ModMask::SHIFT),
                (key.alt, ModMask::M1),
                (key.super_key, ModMask::M4),
            ] {
                if yes {
                    mods |= mask;
                }
            }
            if u16::from(mods) & self.ignored != 0 {
                return Err(format!(
                    "Le modificateur de {} partage le verrouillage du clavier.",
                    key.text()
                ));
            }
            let code = keycode(&self.connection, key.keysym)
                .map_err(|e| format!("{} : {e}", key.text()))?;
            if !seen.insert((code, u16::from(mods))) {
                return Err("Deux raccourcis désignent la même touche sur ce clavier.".into());
            }
            bindings.push((code, mods, command));
        }
        Ok(bindings)
    }
    fn pairs(&self, keys: &[Binding]) -> HashSet<(u8, u16)> {
        keys.iter()
            .flat_map(|(key, mods, _)| {
                self.masks
                    .iter()
                    .map(move |extra| (*key, u16::from(*mods) | extra))
            })
            .collect()
    }
    fn release(&self, pairs: &[(u8, u16)]) -> Result<(), String> {
        for (key, mods) in pairs {
            self.connection
                .ungrab_key(*key, self.root, ModMask::from(*mods))
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
        }
        self.connection.flush().map_err(|e| e.to_string())
    }
    /// The commit closure persists the candidate settings. On conflict or IO
    /// failure, only newly acquired grabs are released; old bindings stay live.
    pub fn replace<T>(
        &self,
        hotkeys: &Hotkeys,
        commit: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.replace_and_notify(hotkeys, commit, |_, _| {})
    }
    pub fn replace_and_notify<T>(
        &self,
        hotkeys: &Hotkeys,
        commit: impl FnOnce() -> Result<T, String>,
        notify: impl FnOnce(&T, bool),
    ) -> Result<T, String> {
        let _transaction = self.update_gate.lock().map_err(|e| e.to_string())?;
        let keys = self.native(hotkeys)?;
        let old = {
            let state = self.bindings.lock().map_err(|e| e.to_string())?;
            self.pairs(&state.keys)
        };
        let new = self.pairs(&keys);
        let mut acquired = Vec::new();
        let mut additions: Vec<_> = new.difference(&old).copied().collect();
        additions.sort_unstable();
        for (key, mods) in &additions {
            let result = self
                .connection
                .grab_key(
                    false,
                    self.root,
                    ModMask::from(*mods),
                    *key,
                    GrabMode::ASYNC,
                    GrabMode::SYNC,
                )
                .map_err(|e| e.to_string())
                .and_then(|c| c.check().map_err(|e| e.to_string()));
            if let Err(error) = result {
                self.release(&acquired)?;
                return Err(format!(
                    "XWayland shortcut conflict; previous shortcuts retained: {error}"
                ));
            }
            acquired.push((*key, *mods));
        }
        let value = match commit() {
            Ok(value) => value,
            Err(error) => {
                self.release(&acquired)?;
                return Err(error);
            }
        };
        // Only publication holds the input-reader lock. File IO and server
        // round trips must never prevent replaying keys to another application.
        let (revision, count) = {
            let mut state = self.bindings.lock().map_err(|e| e.to_string())?;
            state.keys = keys;
            state.revision += 1;
            notify(&value, self.is_alive());
            (state.revision, state.keys.len())
        };
        eprintln!("shortcuts backend=XWayland revision={revision} bindings={count} scope=X11-only replay_other_apps=true");
        let removed: Vec<_> = old.difference(&new).copied().collect();
        if let Err(error) = self.release(&removed) {
            self.alive.store(false, Ordering::Release);
            eprintln!("shortcuts cleanup failed after commit: {error}");
        }
        Ok(value)
    }
    pub fn listen(&self, active: Arc<AtomicBool>, send: impl Fn(Action)) -> Result<(), String> {
        let result = self.listen_inner(active, send);
        self.alive.store(false, Ordering::Release);
        result
    }
    fn listen_inner(&self, active: Arc<AtomicBool>, send: impl Fn(Action)) -> Result<(), String> {
        let mut armed = None;
        let mut last_revision = 0;
        let mut last_copy: HashMap<Action, Instant> = HashMap::new();
        loop {
            match self
                .connection
                .wait_for_event()
                .map_err(|e| e.to_string())?
            {
                Event::KeyPress(e) => {
                    armed = None;
                    if active.load(Ordering::Acquire) && focused_game(&self.connection) {
                        let mods = u16::from(e.state)
                            & !self.ignored
                            & !u16::from(
                                KeyButMask::BUTTON1
                                    | KeyButMask::BUTTON2
                                    | KeyButMask::BUTTON3
                                    | KeyButMask::BUTTON4
                                    | KeyButMask::BUTTON5,
                            );
                        let state = self.bindings.lock().map_err(|e| e.to_string())?;
                        if state.revision != last_revision {
                            last_copy.clear();
                            last_revision = state.revision;
                        }
                        armed = state
                            .keys
                            .iter()
                            .find(|(key, mask, _)| *key == e.detail && u16::from(*mask) == mods)
                            .map(|(key, _, command)| (*key, *command, state.revision));
                    }
                    // A newly acquired key is still a game key until commit;
                    // an old, removed binding is a game key immediately after.
                    let mode = if armed.is_some() {
                        Allow::ASYNC_KEYBOARD
                    } else {
                        Allow::REPLAY_KEYBOARD
                    };
                    self.connection
                        .allow_events(mode, CURRENT_TIME)
                        .map_err(|e| e.to_string())?;
                    self.connection.flush().map_err(|e| e.to_string())?;
                }
                Event::KeyRelease(e) => {
                    if let Some((key, command, revision)) = armed {
                        if key == e.detail {
                            armed = None;
                            let state = self.bindings.lock().map_err(|e| e.to_string())?;
                            if state.revision == revision
                                && active.load(Ordering::Acquire)
                                && focused_game(&self.connection)
                                && (!command.is_copy()
                                    || last_copy.get(&command).is_none_or(|last| {
                                        last.elapsed() >= Duration::from_millis(300)
                                    }))
                            {
                                send(command);
                                if command.is_copy() {
                                    last_copy.insert(command, Instant::now());
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
fn keycode(connection: &impl Connection, symbol: u32) -> Result<u8, String> {
    let first = connection.setup().min_keycode;
    let count = connection.setup().max_keycode - first + 1;
    let mapping = connection
        .get_keyboard_mapping(first, count)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    mapping
        .keysyms
        .chunks(mapping.keysyms_per_keycode as usize)
        .position(|row| row.contains(&symbol))
        .map(|n| first + n as u8)
        .ok_or_else(|| format!("XWayland key symbol {symbol} missing"))
}
#[allow(dead_code)] // Standalone entry point retained for the isolated regression probe.
pub fn run(
    active: Arc<AtomicBool>,
    send: impl Fn(Action),
    ready: impl FnOnce(),
) -> Result<(), String> {
    let backend = Backend::new(&Hotkeys::default())?;
    ready();
    backend.listen(active, send)
}

fn focused_game(connection: &impl Connection) -> bool {
    let Ok(cookie) = connection.get_input_focus() else {
        return false;
    };
    let Ok(reply) = cookie.reply() else {
        return false;
    };
    let mut window = reply.focus;
    for _ in 0..8 {
        if window <= 1 {
            return false;
        }
        let Ok(cookie) =
            connection.get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 128)
        else {
            return false;
        };
        let Ok(class) = cookie.reply() else {
            return false;
        };
        if !class.value.is_empty() {
            return class
                .value
                .split(|b| *b == 0)
                .any(|s| s == b"steam_app_238960");
        }
        let Ok(cookie) = connection.query_tree(window) else {
            return false;
        };
        let Ok(tree) = cookie.reply() else {
            return false;
        };
        if tree.parent == window || tree.parent == tree.root {
            return false;
        }
        window = tree.parent;
    }
    false
}
