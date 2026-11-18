//! One-way import of older profiles; the current executable, storage and IPC
//! use the project name. Legacy files are never removed or overwritten.
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
const LEGACY_APP_ID: &str = "awakened-poe-trade-rs";
pub fn legacy_settings(root: &Path) -> PathBuf {
    root.join(LEGACY_APP_ID).join("config.json")
}
pub fn publish_import(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    let parent = path.parent().ok_or("Invalid settings path")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(".import-{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    let result = match std::fs::hard_link(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(_) => match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(mut file) => file.write_all(bytes).map_err(|e| e.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error.to_string()),
        },
    };
    let _ = std::fs::remove_file(temporary);
    result
}
pub fn active_legacy_instance() -> bool {
    legacy_status().is_some_and(|value| value.get("state").is_some())
}
#[cfg(target_os = "linux")]
fn legacy_status() -> Option<serde_json::Value> {
    use std::os::unix::net::UnixDatagram;
    let root = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    let server = root.join(format!("{LEGACY_APP_ID}.sock"));
    if !server.exists() {
        return None;
    }
    let path = root.join(format!(
        "awakened-crab-trade-import-{}.sock",
        std::process::id()
    ));
    let socket = UnixDatagram::bind(&path).ok()?;
    let result = (|| {
        socket
            .set_read_timeout(Some(Duration::from_millis(300)))
            .ok()?;
        socket.send_to(b"status", server).ok()?;
        let mut bytes = [0; 65536];
        let n = socket.recv(&mut bytes).ok()?;
        serde_json::from_slice(&bytes[..n]).ok()
    })();
    let _ = std::fs::remove_file(path);
    result
}
#[cfg(target_os = "windows")]
fn legacy_status() -> Option<serde_json::Value> {
    let bytes = std::fs::read(
        crate::paths::cache_root()
            .join(LEGACY_APP_ID)
            .join("control.json"),
    )
    .ok()?;
    if bytes.len() > 8192 {
        return None;
    }
    let endpoint: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let token = endpoint["token"].as_str()?;
    if token.len() != 64 {
        return None;
    }
    let port = u16::try_from(endpoint["port"].as_u64()?).ok()?;
    let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0)).ok()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(300)))
        .ok()?;
    socket.connect((std::net::Ipv4Addr::LOCALHOST, port)).ok()?;
    socket
        .send(&serde_json::to_vec(&serde_json::json!({"token":token,"command":"status"})).ok()?)
        .ok()?;
    let mut bytes = [0; 65536];
    let n = socket.recv(&mut bytes).ok()?;
    serde_json::from_slice(&bytes[..n]).ok()
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn legacy_status() -> Option<serde_json::Value> {
    None
}
