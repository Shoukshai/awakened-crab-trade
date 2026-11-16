//! XWayland regression probe restricted to scripts/test-shortcut-replay.sh's
//! private headless server. Never run keyboard injection on the user's desktop.
#[path = "../src/desktop/xwayland_shortcuts.rs"]
mod shortcuts;
use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{
            AtomEnum, ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, GrabMode, InputFocus,
            ModMask, PropMode, WindowClass,
        },
        Event,
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
    COPY_DEPTH_FROM_PARENT, CURRENT_TIME,
};

struct Probe {
    connection: RustConnection,
    window: u32,
}
impl Drop for Probe {
    fn drop(&mut self) {
        let _ = self.connection.destroy_window(self.window);
        let _ = self.connection.flush();
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")?;
    if !std::env::args().any(|a| a == "--isolated")
        || !std::path::Path::new(&runtime)
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("apt-replay-desktop-"))
        || std::fs::read_to_string(format!("{runtime}/display"))?.trim()
            != std::env::var("DISPLAY")?
    {
        return Err("Use scripts/test-shortcut-replay.sh with its private headless server".into());
    }
    let (connection, screen) = x11rb::connect(None)?;
    let root = connection.setup().roots[screen].root;
    let window = connection.generate_id()?;
    connection
        .create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            root,
            800,
            150,
            400,
            125,
            3,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixel(0x224466)
                .border_pixel(0x88bbff)
                .override_redirect(0)
                .event_mask(EventMask::EXPOSURE | EventMask::KEY_PRESS | EventMask::KEY_RELEASE),
        )?
        .check()?;
    connection
        .change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            b"apt_shortcut_probe\0apt_shortcut_probe\0",
        )?
        .check()?;
    connection
        .change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"APT shortcut replay probe",
        )?
        .check()?;
    let type_atom = connection
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE")?
        .reply()?
        .atom;
    let dialog_atom = connection
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE_DIALOG")?
        .reply()?
        .atom;
    connection
        .change_property32(
            PropMode::REPLACE,
            window,
            type_atom,
            AtomEnum::ATOM,
            &[dialog_atom],
        )?
        .check()?;
    let mut hints = x11rb::properties::WmHints::new();
    hints.input = Some(true);
    hints.set(&connection, window)?.check()?;
    let probe = Probe { connection, window };
    // Fail on the third binding: earlier grabs must be released on error.
    let first = probe.connection.setup().min_keycode;
    let count = probe.connection.setup().max_keycode - first + 1;
    let mapping = probe
        .connection
        .get_keyboard_mapping(first, count)?
        .reply()?;
    let d = mapping
        .keysyms
        .chunks(mapping.keysyms_per_keycode as usize)
        .position(|r| r.contains(&('d' as u32)))
        .ok_or("missing d key")? as u8
        + first;
    probe
        .connection
        .grab_key(
            false,
            root,
            ModMask::CONTROL | ModMask::SHIFT,
            d,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?
        .check()?;
    let conflict = shortcuts::run(
        Arc::new(AtomicBool::new(false)),
        |_| {},
        || panic!("conflict accepted"),
    );
    assert!(conflict.is_err_and(|e| e.contains("shortcut conflict")));
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if probe
            .connection
            .grab_key(
                false,
                root,
                ModMask::CONTROL,
                d,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )?
            .check()
            .is_ok()
        {
            break;
        }
        if Instant::now() >= deadline {
            return Err("partial grabs leaked after conflict".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    probe
        .connection
        .ungrab_key(d, root, ModMask::CONTROL)?
        .check()?;
    probe
        .connection
        .ungrab_key(d, root, ModMask::CONTROL | ModMask::SHIFT)?
        .check()?;
    println!("PASS: conflicting shortcut rejected; partial grabs released");
    let active = Arc::new(AtomicBool::new(false));
    let worker_active = active.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (action_tx, action_rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Err(e) = shortcuts::run(
            worker_active,
            move |command| {
                let _ = action_tx.send(command);
            },
            move || {
                let _ = ready_tx.send(());
            },
        ) {
            eprintln!("probe shortcuts: {e}");
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(3))?;
    probe.connection.map_window(window)?.check()?;
    probe.connection.flush()?;
    loop {
        if let Event::Expose(_) = probe.connection.wait_for_event()? {
            break;
        }
    }
    let font = probe.connection.generate_id()?;
    if probe.connection.open_font(font, b"fixed")?.check().is_ok() {
        let gc = probe.connection.generate_id()?;
        probe
            .connection
            .create_gc(
                gc,
                window,
                &CreateGCAux::new()
                    .foreground(0xffffff)
                    .background(0x224466)
                    .font(font),
            )?
            .check()?;
        probe
            .connection
            .image_text8(
                window,
                gc,
                15,
                35,
                b"Awakened PoE Trade: shortcut replay test",
            )?
            .check()?;
        probe
            .connection
            .image_text8(window, gc, 15, 65, b"Private headless server only.")?
            .check()?;
        probe
            .connection
            .image_text8(
                window,
                gc,
                15,
                95,
                b"No gameplay or chat commands are sent.",
            )?
            .check()?;
    }
    probe.connection.flush()?;
    let pos = probe
        .connection
        .translate_coordinates(window, root, 0, 0)?
        .reply()?;
    let size = probe.connection.get_geometry(window)?.reply()?;
    println!(
        "READY: isolated probe at {},{},{}x{}",
        pos.dst_x, pos.dst_y, size.width, size.height
    );
    std::thread::sleep(Duration::from_millis(150));
    probe
        .connection
        .set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?
        .check()?;
    if probe.connection.get_input_focus()?.reply()?.focus != window {
        return Err("Isolated window did not receive focus; no keys sent".into());
    }
    let first = probe.connection.setup().min_keycode;
    let count = probe.connection.setup().max_keycode - first + 1;
    let mapping = probe
        .connection
        .get_keyboard_mapping(first, count)?
        .reply()?;
    let code = |symbol: u32| {
        mapping
            .keysyms
            .chunks(mapping.keysyms_per_keycode as usize)
            .position(|r| r.contains(&symbol))
            .map(|n| first + n as u8)
            .unwrap()
    };
    let d = code('d' as u32);
    let space = code(' ' as u32);
    for stale_game_active in [false, true] {
        active.store(stale_game_active, Ordering::Release);
        if probe.connection.get_input_focus()?.reply()?.focus != window {
            return Err("User changed focus; test stopped".into());
        }
        if !Command::new("xdotool")
            .args(["key", "ctrl+d", "shift+space", "ctrl+shift+d", "ctrl+alt+d"])
            .status()?
            .success()
        {
            return Err("xdotool test failed".into());
        }
        if probe.connection.get_input_focus()?.reply()?.focus != window {
            return Err("Compositor changed focus during test; replay result unverified".into());
        }
        let mut pressed = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && pressed.len() < 4 {
            while let Some(event) = probe.connection.poll_for_event()? {
                if let Event::KeyPress(e) = event {
                    if e.detail == d || e.detail == space {
                        pressed.push(e.detail);
                    }
                }
            }
            if pressed.len() < 4 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert_eq!(
            pressed,
            vec![d, space, d, d],
            "keys swallowed by shortcut backend"
        );
        assert!(
            action_rx.try_recv().is_err(),
            "game action incorrectly triggered in test app"
        );
        println!("PASS: other X11 app received all 4 shortcuts; cached_game_active={stale_game_active}; no overlay action");
    }
    println!("PASS: closing probe on isolated desktop");
    Ok(())
}
