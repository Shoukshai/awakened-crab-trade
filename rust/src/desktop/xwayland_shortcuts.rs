//! X11 passive grabs apply only within XWayland. They cannot observe native
//! Wayland apps. Unrelated X11 applications receive replayed key events.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{Allow, AtomEnum, ConnectionExt, GrabMode, KeyButMask, ModMask},
        Event,
    },
    CURRENT_TIME,
};

pub fn run(
    active: Arc<AtomicBool>,
    send: impl Fn(&'static str),
    ready: impl FnOnce(),
) -> Result<(), String> {
    let (connection, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let root = connection.setup().roots[screen].root;
    let first = connection.setup().min_keycode;
    let count = connection.setup().max_keycode - first + 1;
    let mapping = connection
        .get_keyboard_mapping(first, count)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    let code = |symbol: u32| {
        mapping
            .keysyms
            .chunks(mapping.keysyms_per_keycode as usize)
            .position(|row| row.contains(&symbol))
            .map(|i| first + i as u8)
            .ok_or_else(|| format!("XWayland key symbol {symbol} missing"))
    };
    let d = code('d' as u32)?;
    let space = code(' ' as u32)?;
    let bindings = [
        (d, ModMask::CONTROL, "copy"),
        (space, ModMask::SHIFT, "interact"),
        (d, ModMask::CONTROL | ModMask::SHIFT, "hide"),
    ];
    let mut ignored = ModMask::LOCK;
    if let Ok(numlock) = code(0xff7f) {
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
    let ignored_bits = u16::from(ignored);
    let masks: Vec<_> = (0..256u16).filter(|m| m & !ignored_bits == 0).collect();
    for (key, modifiers, _) in bindings {
        for extra in &masks {
            connection
                .grab_key(
                    false,
                    root,
                    modifiers | ModMask::from(*extra),
                    key,
                    GrabMode::ASYNC,
                    GrabMode::SYNC,
                )
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| format!("XWayland shortcut conflict; no shortcuts registered: {e}"))?;
        }
    }
    connection.flush().map_err(|e| e.to_string())?;
    eprintln!("shortcuts backend=XWayland keys=Ctrl+D,Shift+Space,Ctrl+Shift+D scope=X11-only replay_other_apps=true");
    ready();
    let mut armed = None;
    let mut last = Instant::now() - Duration::from_secs(1);
    loop {
        match connection.wait_for_event().map_err(|e| e.to_string())? {
            Event::KeyPress(e) => {
                // The Wayland snapshot can lag a just-completed desktop switch.
                // Verify X input focus before consuming a key from another X11 app.
                let focused_game = active.load(Ordering::Acquire) && focused_game(&connection);
                if !focused_game {
                    connection
                        .allow_events(Allow::REPLAY_KEYBOARD, CURRENT_TIME)
                        .map_err(|e| e.to_string())?;
                    armed = None;
                } else {
                    connection
                        .allow_events(Allow::ASYNC_KEYBOARD, CURRENT_TIME)
                        .map_err(|e| e.to_string())?;
                    let mods = u16::from(e.state)
                        & !ignored_bits
                        & !u16::from(
                            KeyButMask::BUTTON1
                                | KeyButMask::BUTTON2
                                | KeyButMask::BUTTON3
                                | KeyButMask::BUTTON4
                                | KeyButMask::BUTTON5,
                        );
                    armed = bindings
                        .iter()
                        .find(|(key, mask, _)| *key == e.detail && u16::from(*mask) == mods)
                        .map(|(key, _, command)| (*key, *command));
                }
                connection.flush().map_err(|e| e.to_string())?;
            }
            Event::KeyRelease(e) => {
                if let Some((key, command)) = armed {
                    if key == e.detail {
                        armed = None;
                        if active.load(Ordering::Acquire)
                            && focused_game(&connection)
                            && (command != "copy" || last.elapsed() >= Duration::from_millis(300))
                        {
                            send(command);
                            if command == "copy" {
                                last = Instant::now();
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
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
