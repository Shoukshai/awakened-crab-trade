//! Per-user local control. Unix socket on Linux, authenticated loopback on Windows.
use std::{path::PathBuf, sync::mpsc, time::Duration};
pub struct Request {
    pub command: String,
    pub reply: Option<mpsc::SyncSender<String>>,
}
#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::os::unix::net::UnixDatagram;
    pub fn path() -> Result<PathBuf, String> {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .map(|p| p.join(format!("{}.sock", crate::edition::id())))
            .ok_or("XDG_RUNTIME_DIR is required".into())
    }
    pub struct Server {
        socket: UnixDatagram,
    }
    impl Server {
        pub fn bind() -> Result<Self, String> {
            let path = path()?;
            if path.exists() {
                if UnixDatagram::unbound()
                    .and_then(|s| s.connect(&path))
                    .is_ok()
                {
                    return Err("daemon already running".into());
                }
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
            Ok(Self {
                socket: UnixDatagram::bind(path).map_err(|e| e.to_string())?,
            })
        }
        pub fn run(self, send: impl Fn(Request) -> Result<(), String>) -> Result<(), String> {
            let mut buf = [0u8; 4096];
            loop {
                let (n, address) = self.socket.recv_from(&mut buf).map_err(|e| e.to_string())?;
                let Ok(command) = std::str::from_utf8(&buf[..n]) else {
                    continue;
                };
                if command == "status" {
                    if let Some(path) = address.as_pathname() {
                        let (tx, rx) = mpsc::sync_channel(1);
                        send(Request {
                            command: command.into(),
                            reply: Some(tx),
                        })?;
                        if let Ok(body) = rx.recv_timeout(Duration::from_secs(2)) {
                            let _ = self.socket.send_to(body.as_bytes(), path);
                        }
                    }
                } else {
                    send(Request {
                        command: command.into(),
                        reply: None,
                    })?;
                }
            }
        }
    }
    pub fn command(command: &str) -> Result<Option<String>, String> {
        if command != "status" {
            UnixDatagram::unbound()
                .and_then(|s| s.send_to(command.as_bytes(), path().map_err(std::io::Error::other)?))
                .map_err(|e| format!("start daemon first: {e}"))?;
            return Ok(None);
        }
        let client_path = path()?.with_extension(format!("client-{}", std::process::id()));
        let client = UnixDatagram::bind(&client_path).map_err(|e| e.to_string())?;
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let result = (|| {
            client
                .send_to(command.as_bytes(), path()?)
                .map_err(|e| e.to_string())?;
            let mut buf = [0u8; 65536];
            let n = client.recv(&mut buf).map_err(|e| e.to_string())?;
            Ok(Some(String::from_utf8_lossy(&buf[..n]).into_owned()))
        })();
        let _ = std::fs::remove_file(client_path);
        result
    }
    pub fn cleanup() {
        if let Ok(path) = path() {
            let _ = std::fs::remove_file(path);
        }
    }
}
#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::{Ipv4Addr, UdpSocket};
    use windows_sys::Win32::{Foundation::*, Security::Cryptography::*, System::Threading::*};
    #[derive(Serialize, Deserialize)]
    struct Endpoint {
        port: u16,
        token: String,
        pid: u32,
    }
    #[derive(Serialize, Deserialize)]
    struct Envelope {
        token: String,
        command: String,
    }
    fn path() -> PathBuf {
        crate::paths::cache_root()
            .join(crate::edition::id())
            .join("control.json")
    }
    pub struct Server {
        socket: UdpSocket,
        token: String,
        _mutex: usize,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self._mutex as HANDLE);
            }
        }
    }
    impl Server {
        pub fn bind() -> Result<Self, String> {
            unsafe {
                let name: Vec<u16> = crate::edition::mutex_name()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                let mutex = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
                if mutex.is_null() {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    CloseHandle(mutex);
                    return Err("Le price checker est déjà lancé.".into());
                }
                let result = (|| {
                    let socket =
                        UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
                    let mut bytes = [0u8; 32];
                    let status = BCryptGenRandom(
                        std::ptr::null_mut(),
                        bytes.as_mut_ptr(),
                        bytes.len() as u32,
                        BCRYPT_USE_SYSTEM_PREFERRED_RNG,
                    );
                    if status < 0 {
                        return Err(format!("Initialisation IPC Windows : {status:#x}"));
                    }
                    let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                    let endpoint = Endpoint {
                        port: socket.local_addr().map_err(|e| e.to_string())?.port(),
                        token: token.clone(),
                        pid: std::process::id(),
                    };
                    let path = path();
                    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                    std::fs::write(
                        path,
                        serde_json::to_vec(&endpoint).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(Self {
                        socket,
                        token,
                        _mutex: mutex as usize,
                    })
                })();
                if result.is_err() {
                    CloseHandle(mutex);
                }
                result
            }
        }
        pub fn run(self, send: impl Fn(Request) -> Result<(), String>) -> Result<(), String> {
            let mut buf = [0u8; 8192];
            loop {
                let (n, peer) = self.socket.recv_from(&mut buf).map_err(|e| e.to_string())?;
                if !peer.ip().is_loopback() {
                    continue;
                }
                let Ok(message) = serde_json::from_slice::<Envelope>(&buf[..n]) else {
                    continue;
                };
                if message.token != self.token || message.command.len() > 4096 {
                    continue;
                }
                if message.command == "status" {
                    let (tx, rx) = mpsc::sync_channel(1);
                    send(Request {
                        command: message.command,
                        reply: Some(tx),
                    })?;
                    if let Ok(body) = rx.recv_timeout(Duration::from_secs(2)) {
                        let _ = self.socket.send_to(body.as_bytes(), peer);
                    }
                } else {
                    send(Request {
                        command: message.command,
                        reply: None,
                    })?;
                    let _ = self.socket.send_to(b"ok", peer);
                }
            }
        }
    }
    pub fn command(command: &str) -> Result<Option<String>, String> {
        let bytes =
            std::fs::read(path()).map_err(|e| format!("Le price checker n’est pas lancé : {e}"))?;
        if bytes.len() > 8192 {
            return Err("Contrôle local Windows invalide.".into());
        }
        let endpoint: Endpoint = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if endpoint.token.len() != 64 {
            return Err("Contrôle local Windows invalide.".into());
        }
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        socket
            .connect((Ipv4Addr::LOCALHOST, endpoint.port))
            .map_err(|e| e.to_string())?;
        socket
            .send(
                &serde_json::to_vec(&Envelope {
                    token: endpoint.token,
                    command: command.into(),
                })
                .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        let mut buf = [0u8; 65536];
        let n = socket.recv(&mut buf).map_err(|e| e.to_string())?;
        if command == "status" {
            Ok(Some(String::from_utf8_lossy(&buf[..n]).into_owned()))
        } else if &buf[..n] == b"ok" {
            Ok(None)
        } else {
            Err("Réponse du contrôle local invalide.".into())
        }
    }
    pub fn cleanup() {
        let path = path();
        if std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Endpoint>(&b).ok())
            .is_some_and(|e| e.pid == std::process::id())
        {
            let _ = std::fs::remove_file(path);
        }
    }
}
pub use platform::{cleanup, command, Server};
