//! Windows process setup and portable GTK runtime discovery.
use std::hash::{Hash, Hasher};
use windows_sys::Win32::{Foundation::*, System::Console::*, UI::HiDpi::*};
pub fn initialize() -> Result<(), String> {
    glib::setenv("GDK_BACKEND", "win32", true).map_err(|e| e.to_string())?;
    // Let GDK own its per-pixel layered backing surfaces and backdrop alpha.
    glib::setenv("GDK_WIN32_LAYERED", "1", true).map_err(|e| e.to_string())?;
    // Update the UCRT environment too: font libraries use getenv(), whereas
    // Rust set_var only changes the Win32 environment block.
    // Fontconfig/FreeType keeps glyph rasterization consistent across Windows and Wine.
    glib::setenv("PANGOCAIRO_BACKEND", "fc", true).map_err(|e| e.to_string())?;
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    // GUI launches do not have a stderr handle; retain diagnostics in user storage.
    unsafe {
        let handle = GetStdHandle(STD_ERROR_HANDLE);
        if handle.is_null()
            || handle == INVALID_HANDLE_VALUE
            || std::env::args().nth(1).is_none_or(|a| a == "daemon")
        {
            use std::os::windows::io::IntoRawHandle;
            let dir = crate::paths::cache_root().join("awakened-crab-trade");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(dir.join("checker.log"))
                .map_err(|e| e.to_string())?;
            SetStdHandle(STD_ERROR_HANDLE, file.into_raw_handle());
        }
    }
    let root = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Répertoire du programme introuvable.")?
        .to_path_buf();
    let fonts = root.join("etc/fonts/fonts.conf");
    if fonts.is_file() {
        glib::setenv("FONTCONFIG_FILE", fonts, true).map_err(|e| e.to_string())?;
    }
    let schemas = root.join("share/glib-2.0/schemas");
    if schemas.is_dir() {
        glib::setenv("GSETTINGS_SCHEMA_DIR", schemas, true).map_err(|e| e.to_string())?;
    }
    let modules = root.join("lib/gdk-pixbuf-2.0/2.10.0/loaders");
    let query = root.join("gdk-pixbuf-query-loaders.exe");
    if modules.is_dir() && query.is_file() {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        root.hash(&mut hash);
        env!("CARGO_PKG_VERSION").hash(&mut hash);
        let cache = crate::paths::cache_root()
            .join("awakened-crab-trade")
            .join(format!("loaders-{:016x}.cache", hash.finish()));
        if !cache.is_file() {
            use std::os::windows::process::CommandExt;
            let result = std::process::Command::new(query)
                .env("GDK_PIXBUF_MODULEDIR", &modules)
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
            if !result.status.success() || result.stdout.is_empty() {
                return Err("Impossible de charger les bibliothèques d’images. Décompressez le ZIP entièrement.".into());
            }
            std::fs::create_dir_all(cache.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&cache, result.stdout).map_err(|e| e.to_string())?;
        }
        glib::setenv("GDK_PIXBUF_MODULE_FILE", cache, true).map_err(|e| e.to_string())?;
        glib::setenv("GDK_PIXBUF_MODULEDIR", modules, true).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn report_error(message: &str) {
    eprintln!("error: {message}");
    if std::env::args().nth(1).is_none_or(|a| a == "daemon") {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                crate::desktop::native::wide(message).as_ptr(),
                crate::desktop::native::wide("Awakened Crab Trade").as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}
