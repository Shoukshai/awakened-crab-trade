//! Manual graphical validation helper. Actions require --click or --scroll.
//! Coordinates are logical output coordinates; never use it over gameplay controls.
use wayland_client::{
    delegate_noop,
    protocol::{wl_pointer, wl_registry},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};
struct State {
    manager: Option<ZwlrVirtualPointerManagerV1>,
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
            if interface == "zwlr_virtual_pointer_manager_v1" {
                s.manager = Some(r.bind(name, version.min(2), q, ()));
            }
        }
    }
}
delegate_noop!(State: ignore ZwlrVirtualPointerManagerV1);
delegate_noop!(State: ignore ZwlrVirtualPointerV1);
fn timestamp() -> u32 {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    (now.tv_sec as u64 * 1000 + now.tv_nsec as u64 / 1_000_000) as u32
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        return Err(
            "usage: pointer X Y OUTPUT_WIDTH OUTPUT_HEIGHT [--click | --scroll PIXELS]".into(),
        );
    }
    let scroll = match args.get(4).map(String::as_str) {
        None | Some("--click") => None,
        Some("--scroll") => {
            let value: f64 = args.get(5).ok_or("--scroll requires pixels")?.parse()?;
            if !value.is_finite() || value.abs() > 1000. {
                return Err("scroll must be finite and at most 1000 pixels".into());
            }
            Some(value)
        }
        _ => return Err("unknown pointer action".into()),
    };
    let c = Connection::connect_to_env()?;
    let mut q = c.new_event_queue();
    let mut s = State { manager: None };
    c.display().get_registry(&q.handle(), ());
    q.roundtrip(&mut s)?;
    let p = s
        .manager
        .as_ref()
        .ok_or("virtual pointer unavailable")?
        .create_virtual_pointer(None, &q.handle(), ());
    p.motion_absolute(
        timestamp(),
        args[0].parse()?,
        args[1].parse()?,
        args[2].parse()?,
        args[3].parse()?,
    );
    p.frame();
    q.roundtrip(&mut s)?;
    if args.get(4).is_some_and(|s| s == "--click") {
        p.button(timestamp(), 0x110, wl_pointer::ButtonState::Pressed);
        p.frame();
        q.roundtrip(&mut s)?;
        p.button(timestamp(), 0x110, wl_pointer::ButtonState::Released);
        p.frame();
        q.roundtrip(&mut s)?;
    }
    if let Some(value) = scroll {
        p.axis_source(wl_pointer::AxisSource::Wheel);
        p.axis(timestamp(), wl_pointer::Axis::VerticalScroll, value);
        p.frame();
        q.roundtrip(&mut s)?;
    }
    p.destroy();
    c.flush()?;
    Ok(())
}
