//! Read-only geometry of the same focused XWayland PoE used by shortcuts.
use x11rb::protocol::xproto::ConnectionExt;
pub fn focused() -> Option<(u32, crate::price_layout::Rect)> {
    let (c, _) = x11rb::connect(None).ok()?;
    let id = super::xwayland_shortcuts::focused_game_window(&c)?;
    let g = c.get_geometry(id).ok()?.reply().ok()?;
    let o = c
        .translate_coordinates(id, g.root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    Some((
        id,
        crate::price_layout::Rect {
            x: i32::from(o.dst_x),
            y: i32::from(o.dst_y),
            width: i32::from(g.width),
            height: i32::from(g.height),
        },
    ))
}

/// Configure events update the column even while the native overlay owns focus.
/// No polling, grabs, window movement or activation is performed.
pub fn watch(mut report: impl FnMut(u32, crate::price_layout::Rect)) -> Result<(), String> {
    use x11rb::{
        connection::Connection,
        protocol::{
            xproto::{AtomEnum, ChangeWindowAttributesAux, EventMask},
            Event,
        },
    };
    let (c, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let root = c.setup().roots[screen].root;
    c.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::SUBSTRUCTURE_NOTIFY),
    )
    .map_err(|e| e.to_string())?
    .check()
    .map_err(|e| e.to_string())?;
    c.flush().map_err(|e| e.to_string())?;
    loop {
        let event = c.wait_for_event().map_err(|e| e.to_string())?;
        let id = match event {
            Event::ConfigureNotify(e) => e.window,
            Event::MapNotify(e) => e.window,
            _ => continue,
        };
        let class = c
            .get_property(false, id, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 128)
            .map_err(|e| e.to_string())?
            .reply();
        if !class.is_ok_and(|p| p.value.split(|b| *b == 0).any(|s| s == b"steam_app_238960")) {
            continue;
        }
        let Ok(geometry) = c.get_geometry(id).map_err(|e| e.to_string())?.reply() else {
            continue;
        };
        let Ok(origin) = c
            .translate_coordinates(id, geometry.root, 0, 0)
            .map_err(|e| e.to_string())?
            .reply()
        else {
            continue;
        };
        report(
            id,
            crate::price_layout::Rect {
                x: i32::from(origin.dst_x),
                y: i32::from(origin.dst_y),
                width: i32::from(geometry.width),
                height: i32::from(geometry.height),
            },
        );
    }
}
