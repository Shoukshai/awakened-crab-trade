//! The user-supplied project icon, shared by windows and the notification area.
use gtk::{gdk_pixbuf, prelude::*};

pub fn icon() -> Result<gdk_pixbuf::Pixbuf, String> {
    let loader = gdk_pixbuf::PixbufLoader::with_type("png").map_err(|e| e.to_string())?;
    loader
        .write(include_bytes!("../../assets/branding/icon-256.png"))
        .map_err(|e| e.to_string())?;
    loader.close().map_err(|e| e.to_string())?;
    loader
        .pixbuf()
        .ok_or_else(|| "Application icon unavailable.".into())
}
