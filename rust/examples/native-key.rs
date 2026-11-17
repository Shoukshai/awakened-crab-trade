//! Private-compositor keyboard fixture with a complete evdev XKB map.
//! Unlike per-character synthetic maps, this also exercises GTK accelerators.
use std::{
    io::{Seek, Write},
    os::fd::AsFd,
    time::Duration,
};
use wayland_client::{
    delegate_noop,
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
#[allow(clippy::all)]
#[allow(
    dead_code,
    non_camel_case_types,
    unused_unsafe,
    unused_variables,
    non_upper_case_globals,
    non_snake_case,
    unused_imports
)]
mod protocol {
    use wayland_client;
    use wayland_client::protocol::*;
    pub mod __interfaces {
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!(
            "examples/protocols/virtual-keyboard-unstable-v1.xml"
        );
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("examples/protocols/virtual-keyboard-unstable-v1.xml");
}
use protocol::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
#[derive(Default)]
struct State {
    manager: Option<ZwpVirtualKeyboardManagerV1>,
    seat: Option<wl_seat::WlSeat>,
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
                "zwp_virtual_keyboard_manager_v1" => {
                    s.manager = Some(r.bind(name, version.min(1), q, ()))
                }
                "wl_seat" => s.seat = Some(r.bind(name, version.min(7), q, ())),
                _ => (),
            }
        }
    }
}
delegate_noop!(State: ignore ZwpVirtualKeyboardManagerV1);
delegate_noop!(State: ignore ZwpVirtualKeyboardV1);
delegate_noop!(State: ignore wl_seat::WlSeat);
fn stamp() -> u32 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    (t.tv_sec as u64 * 1000 + t.tv_nsec as u64 / 1_000_000) as u32
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")?;
    if !runtime.starts_with("/tmp/apt-overlay-desktop-") {
        return Err("keyboard fixture is restricted to the private compositor".into());
    }
    let display = std::env::var("WAYLAND_DISPLAY")?;
    if display.contains('/')
        || std::fs::read_to_string(std::path::Path::new(&runtime).join("wayland"))?.trim()
            != display
    {
        return Err(
            "keyboard fixture requires the compositor socket recorded by its private init".into(),
        );
    }
    let action = std::env::args()
        .nth(1)
        .ok_or("usage: native-key ctrl-v|escape")?;
    let (code, mods) = match action.as_str() {
        "ctrl-v" => (47, 4),
        "escape" => (1, 0),
        _ => return Err("unsupported fixture action".into()),
    };
    let output = std::process::Command::new("xkbcli")
        .args(["compile-keymap", "--layout", "us"])
        .output()?;
    if !output.status.success() {
        return Err("xkbcli compile-keymap failed".into());
    }
    let mut bytes = output.stdout;
    bytes.push(0);
    let path = std::path::Path::new(&runtime).join(format!("keyboard-map-{}", std::process::id()));
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    std::fs::remove_file(&path)?;
    file.write_all(&bytes)?;
    file.rewind()?;
    let connection = Connection::connect_to_env()?;
    let mut q = connection.new_event_queue();
    let mut state = State::default();
    connection.display().get_registry(&q.handle(), ());
    q.roundtrip(&mut state)?;
    let keyboard = state
        .manager
        .as_ref()
        .ok_or("virtual keyboard unavailable")?
        .create_virtual_keyboard(
            state.seat.as_ref().ok_or("seat unavailable")?,
            &q.handle(),
            (),
        );
    keyboard.keymap(1, file.as_fd(), bytes.len() as u32);
    q.roundtrip(&mut state)?;
    std::thread::sleep(Duration::from_millis(100));
    keyboard.modifiers(mods, 0, 0, 0);
    keyboard.key(stamp(), code, 1);
    q.roundtrip(&mut state)?;
    std::thread::sleep(Duration::from_millis(30));
    keyboard.key(stamp(), code, 0);
    keyboard.modifiers(0, 0, 0, 0);
    q.roundtrip(&mut state)?;
    std::thread::sleep(Duration::from_millis(100));
    keyboard.destroy();
    q.roundtrip(&mut state)?;
    Ok(())
}
