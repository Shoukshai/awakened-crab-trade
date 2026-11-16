use serde::Serialize;
use std::collections::HashMap;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Window {
    pub id: u32,
    pub title: String,
    pub app_id: String,
    pub active: bool,
    pub minimized: bool,
}
impl Window {
    pub fn is_game(&self) -> bool {
        self.title == "Path of Exile" && self.app_id == "steam_app_238960"
    }
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct Snapshot {
    pub windows: Vec<Window>,
}
impl Snapshot {
    pub fn game_active(&self) -> bool {
        self.windows
            .iter()
            .any(|w| w.is_game() && w.active && !w.minimized)
    }
    pub fn other_active(&self) -> bool {
        self.windows.iter().any(|w| !w.is_game() && w.active)
    }
    pub fn game_present(&self) -> bool {
        self.windows.iter().any(|w| w.is_game() && !w.minimized)
    }
    pub fn game_exists(&self) -> bool {
        self.windows.iter().any(Window::is_game)
    }
}
#[derive(Default)]
struct Entry {
    pending: Window,
    committed: Option<Window>,
}
struct Tracker {
    windows: HashMap<u32, Entry>,
    manager: Option<ZwlrForeignToplevelManagerV1>,
    manager_global: Option<u32>,
    finished: bool,
    send: Box<dyn Fn(Snapshot) + Send>,
}
impl Tracker {
    fn finish(&mut self) {
        self.finished = true;
        self.windows.clear();
        self.publish();
    }
    fn commit(&mut self, id: u32) {
        let Some(entry) = self.windows.get_mut(&id) else {
            return;
        };
        if entry.committed.as_ref() == Some(&entry.pending) {
            return;
        }
        entry.committed = Some(entry.pending.clone());
        self.publish();
    }
    fn publish(&self) {
        let mut windows: Vec<_> = self
            .windows
            .values()
            .filter_map(|e| e.committed.clone())
            .collect();
        windows.sort_by_key(|w| w.id);
        (self.send)(Snapshot { windows });
    }
}
impl Dispatch<wl_registry::WlRegistry, ()> for Tracker {
    fn event(
        s: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        q: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "zwlr_foreign_toplevel_manager_v1" {
                s.manager = Some(registry.bind(name, version.min(3), q, ()));
                s.manager_global = Some(name);
            }
        } else if let wl_registry::Event::GlobalRemove { name } = event {
            if s.manager_global == Some(name) {
                s.finish();
            }
        }
    }
}
impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for Tracker {
    fn event(
        s: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            s.windows.insert(
                toplevel.id().protocol_id(),
                Entry {
                    pending: Window {
                        id: toplevel.id().protocol_id(),
                        ..Default::default()
                    },
                    committed: None,
                },
            );
        } else if let zwlr_foreign_toplevel_manager_v1::Event::Finished = event {
            s.finish();
        }
    }
    wayland_client::event_created_child!(Tracker, ZwlrForeignToplevelManagerV1, [0 => (ZwlrForeignToplevelHandleV1, ())]);
}
impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for Tracker {
    fn event(
        s: &mut Self,
        h: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = h.id().protocol_id();
        let Some(entry) = s.windows.get_mut(&id) else {
            return;
        };
        let w = &mut entry.pending;
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => w.title = title,
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => w.app_id = app_id,
            zwlr_foreign_toplevel_handle_v1::Event::State { state } => {
                let flags: Vec<_> = state
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| u32::from_ne_bytes(*c))
                    .collect();
                w.active = flags.contains(&2);
                w.minimized = flags.contains(&1);
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                s.windows.remove(&id);
                s.publish();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => s.commit(id),
            _ => (),
        }
    }
}
pub fn track(send: impl Fn(Snapshot) + Send + 'static) -> Result<(), String> {
    let connection = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let mut queue = connection.new_event_queue();
    let mut state = Tracker {
        windows: HashMap::new(),
        manager: None,
        manager_global: None,
        finished: false,
        send: Box::new(send),
    };
    connection.display().get_registry(&queue.handle(), ());
    queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
    if state.manager.is_none() {
        return Err(
            "compositor lacks foreign-toplevel monitoring; refusing unsafe focus tracking".into(),
        );
    }
    loop {
        if state.finished {
            return Err(
                "foreign-toplevel monitoring ended; focus state is no longer trusted".into(),
            );
        }
        queue
            .blocking_dispatch(&mut state)
            .map_err(|e| e.to_string())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[test]
    fn an_unfinished_window_update_never_leaks_into_another_windows_done() {
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let received = snapshots.clone();
        let mut tracker = Tracker {
            windows: HashMap::new(),
            manager: None,
            manager_global: None,
            finished: false,
            send: Box::new(move |s| received.lock().unwrap().push(s)),
        };
        tracker.windows.insert(
            1,
            Entry {
                pending: Window {
                    id: 1,
                    title: "Path of Exile".into(),
                    app_id: "steam_app_238960".into(),
                    active: true,
                    minimized: false,
                },
                committed: None,
            },
        );
        tracker.commit(1);
        assert!(snapshots.lock().unwrap().last().unwrap().game_active());
        tracker.windows.get_mut(&1).unwrap().pending.active = false;
        tracker.windows.insert(
            2,
            Entry {
                pending: Window {
                    id: 2,
                    app_id: "fixture".into(),
                    ..Default::default()
                },
                committed: None,
            },
        );
        tracker.commit(2);
        assert!(snapshots.lock().unwrap().last().unwrap().game_active());
        tracker.commit(1);
        assert!(!snapshots.lock().unwrap().last().unwrap().game_active());
        let count = snapshots.lock().unwrap().len();
        tracker.commit(1);
        assert_eq!(snapshots.lock().unwrap().len(), count);
    }
    #[test]
    fn a_minimized_game_is_distinct_from_a_closed_or_absent_game() {
        let s = Snapshot {
            windows: vec![Window {
                title: "Path of Exile".into(),
                app_id: "steam_app_238960".into(),
                minimized: true,
                active: true,
                ..Default::default()
            }],
        };
        assert!(s.game_exists());
        assert!(!s.game_present());
        assert!(!s.game_active());
        assert!(!Snapshot::default().game_exists());
    }
    #[test]
    fn finished_monitoring_clears_previously_trusted_active_windows() {
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let received = snapshots.clone();
        let mut tracker = Tracker {
            windows: HashMap::new(),
            manager: None,
            manager_global: None,
            finished: false,
            send: Box::new(move |s| received.lock().unwrap().push(s)),
        };
        tracker.windows.insert(
            1,
            Entry {
                pending: Window {
                    id: 1,
                    title: "Path of Exile".into(),
                    app_id: "steam_app_238960".into(),
                    active: true,
                    ..Default::default()
                },
                committed: None,
            },
        );
        tracker.commit(1);
        assert!(snapshots.lock().unwrap().last().unwrap().game_active());
        tracker.finish();
        assert!(tracker.finished);
        let snapshots = snapshots.lock().unwrap();
        assert!(!snapshots.last().unwrap().game_exists());
        assert!(!snapshots.last().unwrap().game_active());
    }
}
