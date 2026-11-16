//! Explicit desktop validation tool, not used by the overlay's focus policy.
use std::collections::HashMap;
use wayland_client::{
    delegate_noop,
    protocol::{wl_registry, wl_seat::WlSeat},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};
struct State {
    seat: Option<WlSeat>,
    handles: HashMap<u32, (ZwlrForeignToplevelHandleV1, String, String)>,
}
impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        s: &mut Self,
        r: &wl_registry::WlRegistry,
        e: wl_registry::Event,
        _: &(),
        _: &Connection,
        q: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = e
        {
            match interface.as_str() {
                "zwlr_foreign_toplevel_manager_v1" => {
                    r.bind::<ZwlrForeignToplevelManagerV1, _, _>(name, version.min(3), q, ());
                }
                "wl_seat" => s.seat = Some(r.bind(name, version.min(7), q, ())),
                _ => (),
            }
        }
    }
}
impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        s: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        e: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = e {
            s.handles.insert(
                toplevel.id().protocol_id(),
                (toplevel, String::new(), String::new()),
            );
        }
    }
    wayland_client::event_created_child!(State,ZwlrForeignToplevelManagerV1,[0=>(ZwlrForeignToplevelHandleV1,())]);
}
impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
    fn event(
        s: &mut Self,
        h: &ZwlrForeignToplevelHandleV1,
        e: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some((_, title, app)) = s.handles.get_mut(&h.id().protocol_id()) {
            match e {
                zwlr_foreign_toplevel_handle_v1::Event::Title { title: t } => *title = t,
                zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => *app = app_id,
                _ => (),
            }
        }
    }
}
delegate_noop!(State: ignore WlSeat);
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: window-control APP_ID activate|minimize|restore".into());
    }
    let c = Connection::connect_to_env()?;
    let mut q = c.new_event_queue();
    let mut s = State {
        seat: None,
        handles: HashMap::new(),
    };
    c.display().get_registry(&q.handle(), ());
    q.roundtrip(&mut s)?;
    q.roundtrip(&mut s)?;
    let (h, title, _) = s
        .handles
        .values()
        .find(|(_, _, app)| app == &args[0])
        .ok_or("target missing")?;
    println!("request={} target={title}", args[1]);
    match args[1].as_str() {
        "activate" => h.activate(s.seat.as_ref().ok_or("seat missing")?),
        "minimize" => h.set_minimized(),
        "restore" => h.unset_minimized(),
        _ => return Err("unknown action".into()),
    }
    q.roundtrip(&mut s)?;
    Ok(())
}
