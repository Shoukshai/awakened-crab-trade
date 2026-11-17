//! Passive XRecord wheel observation, limited to the tracked XWayland game.
//! The original wheel event is never grabbed or consumed; no keyboard is recorded.
use crate::{
    hotkeys::{Action, ScrollChord},
    stash_scroll::{self, Bounds},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use x11rb::{
    connection::Connection,
    protocol::{
        record::{ConnectionExt as _, Range, Range8},
        xproto::{ButtonPressEvent, ConnectionExt as _, KeyButMask, BUTTON_PRESS_EVENT},
    },
    rust_connection::RustConnection,
    x11_utils::TryParse,
};
pub struct Monitor {
    control: Arc<RustConnection>,
    id: u32,
    pub enabled: Arc<AtomicBool>,
    started: AtomicBool,
}
impl Monitor {
    pub fn new() -> Result<Self, String> {
        let (control, _) = x11rb::connect(None).map_err(|e| e.to_string())?;
        control
            .record_query_version(1, 13)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| format!("XRecord unavailable: {e}"))?;
        let id = control.generate_id().map_err(|e| e.to_string())?;
        control
            .record_create_context(
                id,
                0u8,
                &[3],
                &[Range {
                    device_events: Range8 {
                        first: BUTTON_PRESS_EVENT,
                        last: BUTTON_PRESS_EVENT,
                    },
                    ..Default::default()
                }],
            )
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            control: Arc::new(control),
            id,
            enabled: Arc::new(AtomicBool::new(false)),
            started: AtomicBool::new(false),
        })
    }
    pub fn start(
        &self,
        active: Arc<AtomicBool>,
        alive: Arc<AtomicBool>,
        send: impl Fn(Action) + Send + 'static,
        report: impl Fn(Result<(), String>) + Send + 'static,
    ) {
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let control = self.control.clone();
        let enabled = self.enabled.clone();
        let id = self.id;
        std::thread::spawn(move || {
            let result = observe(&control, id, enabled.clone(), active, alive, send, &report);
            enabled.store(false, Ordering::Release);
            if let Err(e) = result {
                eprintln!("wheel backend=XRecord stopped: {e}");
                report(Err(e));
            }
        });
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.enabled.store(false, Ordering::Release);
        if let Ok(c) = self.control.record_disable_context(self.id) {
            let _ = c.check();
        }
        let _ = self.control.record_free_context(self.id);
        let _ = self.control.flush();
    }
}
fn observe(
    control: &RustConnection,
    id: u32,
    enabled: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    send: impl Fn(Action),
    report: &impl Fn(Result<(), String>),
) -> Result<(), String> {
    let (data, _) = x11rb::connect(None).map_err(|e| e.to_string())?;
    for reply in data.record_enable_context(id).map_err(|e| e.to_string())? {
        let reply = reply.map_err(|e| e.to_string())?;
        if !alive.load(Ordering::Acquire) {
            return Ok(());
        }
        if reply.category == 4 {
            eprintln!(
                "wheel backend=XRecord available=true scope=XWayland-only original_wheel=unchanged"
            );
            report(Ok(()));
        }
        if reply.category != 0 {
            continue;
        }
        if reply.client_swapped || reply.data.len() % 32 != 0 {
            return Err("Unsupported XRecord device event encoding".into());
        }
        for bytes in reply.data.as_chunks::<32>().0 {
            let (e, _) = ButtonPressEvent::try_parse(bytes).map_err(|e| e.to_string())?;
            if !(4..=7).contains(&e.detail)
                || !e.state.contains(KeyButMask::CONTROL)
                || !enabled.load(Ordering::Acquire)
                || !active.load(Ordering::Acquire)
            {
                continue;
            }
            let Some(window) = super::xwayland_shortcuts::focused_game_window(control) else {
                continue;
            };
            // A target can disappear between its identity check and geometry.
            // Drop that step; the passive observer must survive the next game.
            let Ok(geom) = control
                .get_geometry(window)
                .map_err(|e| e.to_string())?
                .reply()
            else {
                continue;
            };
            let Ok(origin) = control
                .translate_coordinates(window, geom.root, 0, 0)
                .map_err(|e| e.to_string())?
                .reply()
            else {
                continue;
            };
            let bounds = Bounds {
                x: f64::from(origin.dst_x),
                y: f64::from(origin.dst_y),
                width: f64::from(geom.width),
                height: f64::from(geom.height),
            };
            let rotation = if e.detail == 4 || e.detail == 6 {
                -1
            } else {
                1
            };
            let direction = stash_scroll::action(
                true,
                active.load(Ordering::Acquire),
                enabled.load(Ordering::Acquire),
                f64::from(e.root_x),
                f64::from(e.root_y),
                rotation,
                Some(bounds),
            );
            if let Some(direction) = direction {
                send(Action::ScrollStash(ScrollChord {
                    direction,
                    shift: e.state.contains(KeyButMask::SHIFT),
                    alt: e.state.contains(KeyButMask::MOD1),
                    super_key: e.state.contains(KeyButMask::MOD4),
                }));
            }
        }
    }
    Err("XRecord wheel observation ended".into())
}
