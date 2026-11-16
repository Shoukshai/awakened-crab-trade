//! Fake X11 game and foreign window, restricted to a private headless compositor.
use std::io::{self, BufRead, Write};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{AtomEnum, ConnectionExt, CreateWindowAux, EventMask, PropMode, WindowClass},
        Event,
    },
    wrapper::ConnectionExt as _,
    COPY_DEPTH_FROM_PARENT,
};
fn create(
    c: &x11rb::rust_connection::RustConnection,
    root: u32,
    game: bool,
) -> Result<u32, Box<dyn std::error::Error>> {
    let id = c.generate_id()?;
    c.create_window(
        COPY_DEPTH_FROM_PARENT,
        id,
        root,
        0,
        0,
        1000,
        700,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new()
            .background_pixel(if game { 0x223344 } else { 0x442233 })
            .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
    )?
    .check()?;
    c.change_property8(
        PropMode::REPLACE,
        id,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        if game {
            b"steam_app_238960\0steam_app_238960\0"
        } else {
            b"apt_foreign_fixture\0apt_foreign_fixture\0"
        },
    )?
    .check()?;
    c.change_property8(
        PropMode::REPLACE,
        id,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        if game {
            b"Path of Exile"
        } else {
            b"Foreign focus fixture"
        },
    )?
    .check()?;
    let mut hints = x11rb::properties::WmHints::new();
    hints.input = Some(true);
    hints.set(c, id)?.check()?;
    Ok(id)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")?;
    if !std::env::args().any(|a| a == "--isolated")
        || !std::path::Path::new(&runtime)
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("apt-overlay-desktop-"))
        || std::fs::read_to_string(format!("{runtime}/display"))?.trim()
            != std::env::var("DISPLAY")?
    {
        return Err("Use scripts/test-overlay-lifecycle.py with its private compositor".into());
    }
    let (c, screen) = x11rb::connect(None)?;
    let root = c.setup().roots[screen].root;
    let mut game = create(&c, root, true)?;
    let other = create(&c, root, false)?;
    c.map_window(game)?.check()?;
    c.map_window(other)?.check()?;
    c.flush()?;
    println!("{}", serde_json::json!({"game":game,"other":other}));
    io::stdout().flush()?;
    let mut clicks = 0u64;
    let mut other_clicks = 0u64;
    let mut last_click = None;
    for line in io::stdin().lock().lines() {
        match line?.as_str() {
            "unmap" => {
                c.unmap_window(game)?.check()?;
            }
            "map" => {
                c.map_window(game)?.check()?;
            }
            "destroy" => {
                c.destroy_window(game)?.check()?;
                game = 0;
            }
            "create" => {
                if game != 0 {
                    return Err("game already exists".into());
                }
                game = create(&c, root, true)?;
                c.map_window(game)?.check()?;
            }
            "input" => (),
            "quit" => break,
            _ => return Err("unknown fixture command".into()),
        }
        c.flush()?;
        c.get_input_focus()?.reply()?;
        while let Some(event) = c.poll_for_event()? {
            if let Event::ButtonPress(event) = event {
                if event.event == other {
                    other_clicks += 1;
                } else {
                    clicks += 1;
                }
                last_click = Some(
                    serde_json::json!({"window":event.event,"button":event.detail,"x":event.event_x,"y":event.event_y}),
                );
            }
        }
        println!(
            "{}",
            serde_json::json!({"game":game,"clicks":clicks,"other_clicks":other_clicks,"last_click":last_click})
        );
        io::stdout().flush()?;
    }
    if game != 0 {
        c.destroy_window(game)?.check()?;
    }
    c.destroy_window(other)?.check()?;
    c.flush()?;
    Ok(())
}
