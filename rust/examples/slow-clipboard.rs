//! Delayed clipboard provider for the private desktop lifecycle tests only.
use std::{io::Write, time::Duration};
use wayland_client::{
    delegate_noop,
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
    zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
};
#[derive(Default)]
struct State {
    manager: Option<ZwlrDataControlManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    bytes: Vec<u8>,
    finished: bool,
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
                "zwlr_data_control_manager_v1" => {
                    s.manager = Some(r.bind(name, version.min(2), q, ()))
                }
                "wl_seat" => s.seat = Some(r.bind(name, version.min(7), q, ())),
                _ => (),
            }
        }
    }
}
delegate_noop!(State: ignore ZwlrDataControlManagerV1);
delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ignore ZwlrDataControlOfferV1);
impl Dispatch<ZwlrDataControlDeviceV1, ()> for State {
    fn event(
        s: &mut Self,
        _: &ZwlrDataControlDeviceV1,
        e: zwlr_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(e, zwlr_data_control_device_v1::Event::Finished) {
            s.finished = true;
        }
    }
    wayland_client::event_created_child!(State,ZwlrDataControlDeviceV1,[0=>(ZwlrDataControlOfferV1,())]);
}
impl Dispatch<ZwlrDataControlSourceV1, ()> for State {
    fn event(
        s: &mut Self,
        _: &ZwlrDataControlSourceV1,
        e: zwlr_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match e {
            zwlr_data_control_source_v1::Event::Send { fd, .. } => {
                println!("REQUEST");
                let _ = std::io::stdout().flush();
                std::thread::sleep(Duration::from_millis(600));
                let mut file = std::fs::File::from(fd);
                let _ = file.write_all(&s.bytes);
            }
            zwlr_data_control_source_v1::Event::Cancelled => s.finished = true,
            _ => (),
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")?;
    let display = std::env::var("WAYLAND_DISPLAY")?;
    if !runtime.starts_with("/tmp/apt-overlay-desktop-")
        || display.contains('/')
        || std::fs::read_to_string(std::path::Path::new(&runtime).join("wayland"))?.trim()
            != display
    {
        return Err("delayed clipboard is restricted to the recorded private compositor".into());
    }
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: slow-clipboard FIXTURE")?;
    let bytes = std::fs::read(path)?;
    if bytes.len() > 65536 {
        return Err("fixture exceeds clipboard limit".into());
    }
    let connection = Connection::connect_to_env()?;
    let mut queue = connection.new_event_queue();
    let mut state = State {
        bytes,
        ..Default::default()
    };
    connection.display().get_registry(&queue.handle(), ());
    queue.roundtrip(&mut state)?;
    let manager = state.manager.as_ref().ok_or("data-control unavailable")?;
    let source = manager.create_data_source(&queue.handle(), ());
    source.offer("text/plain;charset=utf-8".into());
    source.offer("text/plain".into());
    let device = manager.get_data_device(
        state.seat.as_ref().ok_or("seat unavailable")?,
        &queue.handle(),
        (),
    );
    device.set_selection(Some(&source));
    connection.flush()?;
    println!("READY");
    std::io::stdout().flush()?;
    while !state.finished {
        queue.blocking_dispatch(&mut state)?;
    }
    Ok(())
}
