//! Private-desktop capability probe. It records button presses only, never keys.
use std::io::Write;
use x11rb::{
    connection::Connection,
    protocol::{
        record::{ConnectionExt as _, Range, Range8},
        xproto::{ButtonPressEvent, BUTTON_PRESS_EVENT},
    },
    x11_utils::TryParse,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")?;
    if !runtime.starts_with("/tmp/apt-overlay-desktop-")
        || std::fs::read_to_string(format!("{runtime}/display"))?.trim()
            != std::env::var("DISPLAY")?
    {
        return Err("wheel probe is restricted to the private XWayland display".into());
    }
    let (control, _) = x11rb::connect(None)?;
    let version = control.record_query_version(1, 13)?.reply()?;
    let id = control.generate_id()?;
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
        )?
        .check()?;
    let (data, _) = x11rb::connect(None)?;
    for reply in data.record_enable_context(id)? {
        let reply = reply?;
        if reply.category == 4 {
            println!(
                "{}",
                serde_json::json!({"ready":true,"record":[version.major_version,version.minor_version]})
            );
            std::io::stdout().flush()?;
        }
        if reply.category != 0 || reply.client_swapped {
            continue;
        }
        for bytes in reply.data.as_chunks::<32>().0 {
            if let Ok((e, _)) = ButtonPressEvent::try_parse(bytes) {
                if (4..=7).contains(&e.detail) {
                    println!(
                        "{}",
                        serde_json::json!({"button":e.detail,"state":u16::from(e.state),"x":e.root_x,"y":e.root_y,"time":e.time,"event":e.event})
                    );
                    std::io::stdout().flush()?;
                }
            }
        }
    }
    Ok(())
}
