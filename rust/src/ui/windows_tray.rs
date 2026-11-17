//! Native notification icon supplied by GTK's Win32 backend.
use glib::translate::*;
use gtk::prelude::*;
#[allow(deprecated)]
pub fn new(sender: glib::Sender<super::Event>) -> Result<glib::Object, String> {
    let loader = gtk::gdk_pixbuf::PixbufLoader::with_type("png").map_err(|e| e.to_string())?;
    loader
        .write(include_bytes!("../../../main/build/icons/128x128.png"))
        .map_err(|e| e.to_string())?;
    loader.close().map_err(|e| e.to_string())?;
    let pixbuf = loader.pixbuf().ok_or("Icône du programme indisponible.")?;
    let raw = unsafe { gtk::ffi::gtk_status_icon_new_from_pixbuf(pixbuf.to_glib_none().0) };
    if raw.is_null() {
        return Err("Zone de notification indisponible.".into());
    }
    let object: glib::Object = unsafe { from_glib_full(raw as *mut glib::gobject_ffi::GObject) };
    unsafe {
        gtk::ffi::gtk_status_icon_set_tooltip_text(raw, c"Awakened PoE Trade".as_ptr());
        gtk::ffi::gtk_status_icon_set_visible(raw, 1);
    }
    let actions = sender.clone();
    object.connect_local("activate", false, move |_| {
        let _ = actions.send(super::Event::Hotkey(crate::hotkeys::Action::Command(
            "settings",
        )));
        None
    });
    let menu = gtk::Menu::new();
    for (label, command) in [("Réglages", "settings"), ("Quitter", "quit")] {
        let item = gtk::MenuItem::with_label(label);
        let sender = sender.clone();
        item.connect_activate(move |_| {
            let _ = sender.send(super::Event::Hotkey(crate::hotkeys::Action::Command(
                command,
            )));
        });
        menu.append(&item);
    }
    menu.show_all();
    object.connect_local("popup-menu", false, move |args| {
        let button = args.get(1).and_then(|v| v.get::<u32>().ok()).unwrap_or(3);
        let time = args.get(2).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
        menu.popup_easy(button, time);
        None
    });
    Ok(object)
}
