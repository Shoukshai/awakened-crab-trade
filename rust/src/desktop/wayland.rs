use std::collections::HashMap;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

#[derive(Debug, Clone, Default)]
pub struct Window {
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
#[derive(Debug, Clone)]
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
}
struct Tracker {
    windows: HashMap<u32, Window>,
    manager: Option<ZwlrForeignToplevelManagerV1>,
    send: Box<dyn Fn(Snapshot) + Send>,
}
impl Tracker {
    fn publish(&self) {
        (self.send)(Snapshot {
            windows: self.windows.values().cloned().collect(),
        });
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
            s.windows
                .insert(toplevel.id().protocol_id(), Window::default());
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
        let w = s.windows.entry(id).or_default();
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
            zwlr_foreign_toplevel_handle_v1::Event::Done => s.publish(),
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
        queue
            .blocking_dispatch(&mut state)
            .map_err(|e| e.to_string())?;
    }
}
