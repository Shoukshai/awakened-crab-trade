#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
};

const PAYLOAD: &[u8] = include_bytes!(env!("APT_PORTABLE_PAYLOAD_PATH"));
const ID: &str = env!("APT_PORTABLE_ID");
const MAGIC: &[u8; 8] = b"APTSELF1";
const APP: &str = "awakened-crab-trade.exe";

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            #[cfg(windows)]
            if std::env::args_os().nth(1).is_none() {
                let text: Vec<u16> = format!("Impossible de démarrer le price checker.\n\n{error}")
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                let title: Vec<u16> = "Awakened Crab Trade"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                        std::ptr::null_mut(),
                        text.as_ptr(),
                        title.as_ptr(),
                        0x10,
                    );
                }
            }
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<u8, String> {
    if PAYLOAD.is_empty() || ID.len() != 64 || !ID.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Le fichier portable ne contient pas de programme valide.".into());
    }
    let local = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .ok_or("Le dossier de données Windows est introuvable.")?;
    let root = PathBuf::from(local).join("awakened-crab-trade/portable");
    let cache =
        prepare(&root, PAYLOAD, ID).map_err(|e| format!("Préparation du programme : {e}"))?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--extract-only") {
        println!("{}", cache.display());
        return Ok(0);
    }
    let mut child = Command::new(cache.join(APP))
        .args(&args)
        .spawn()
        .map_err(|e| format!("Lancement du price checker : {e}"))?;
    // A normal double-click leaves only the checker running. Diagnostic commands
    // wait and preserve stdout, stderr and exit status for scripts and tests.
    if args.is_empty() {
        return Ok(0);
    }
    Ok(child
        .wait()
        .map_err(|e| e.to_string())?
        .code()
        .unwrap_or(1)
        .clamp(0, 255) as u8)
}
fn prepare(root: &Path, payload: &[u8], id: &str) -> io::Result<PathBuf> {
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("Invalid payload identity"));
    }
    fs::create_dir_all(root)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("extract.lock"))?;
    lock.lock()?;
    let destination = root.join(format!("{}-{}", env!("CARGO_PKG_VERSION"), &id[..16]));
    if fs::read_to_string(destination.join(".ready")).is_ok_and(|s| s == id)
        && destination.join(APP).is_file()
    {
        return Ok(destination);
    }
    if format!("{:x}", Sha256::digest(payload)) != id {
        return Err(invalid("Fichier portable endommagé."));
    }
    let staging = Stage(root.join(format!(".{}-{}.tmp", &id[..16], std::process::id())));
    if staging.0.exists() {
        fs::remove_dir_all(&staging.0)?;
    }
    fs::create_dir(&staging.0)?;
    unpack(GzDecoder::new(payload), &staging.0)?;
    if !staging.0.join(APP).is_file() {
        return Err(invalid("Programme absent du fichier portable."));
    }
    fs::write(staging.0.join(".ready"), id)?;
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::rename(&staging.0, &destination)?;
    Ok(destination)
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn bytes<const N: usize>(input: &mut impl Read) -> io::Result<[u8; N]> {
    let mut value = [0; N];
    input.read_exact(&mut value)?;
    Ok(value)
}
fn relative(name: &str) -> io::Result<&Path> {
    if name.is_empty()
        || name == ".ready"
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
    {
        return Err(invalid("Invalid archive path"));
    }
    let path = Path::new(name);
    if !path.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(invalid("Invalid archive path"));
    }
    Ok(path)
}
fn unpack(mut input: impl Read, destination: &Path) -> io::Result<()> {
    if &bytes::<8>(&mut input)? != MAGIC {
        return Err(invalid("Invalid archive header"));
    }
    let count = u32::from_le_bytes(bytes(&mut input)?);
    if count == 0 || count > 10000 {
        return Err(invalid("Invalid file count"));
    }
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    for _ in 0..count {
        let length = u32::from_le_bytes(bytes(&mut input)?) as usize;
        let size = u64::from_le_bytes(bytes(&mut input)?);
        if length == 0 || length > 4096 || size > 512 * 1024 * 1024 {
            return Err(invalid("Invalid file size"));
        }
        total = total
            .checked_add(size)
            .filter(|n| *n <= 1024 * 1024 * 1024)
            .ok_or_else(|| invalid("Archive too large"))?;
        let expected = bytes::<32>(&mut input)?;
        let mut name = vec![0; length];
        input.read_exact(&mut name)?;
        let name = std::str::from_utf8(&name).map_err(|_| invalid("Invalid filename"))?;
        let file = destination.join(relative(name)?);
        fs::create_dir_all(file.parent().unwrap())?;
        let mut output = OpenOptions::new().create_new(true).write(true).open(file)?;
        let mut remaining = size;
        let mut hash = Sha256::new();
        while remaining > 0 {
            let take = buffer.len().min(remaining as usize);
            input.read_exact(&mut buffer[..take])?;
            hash.update(&buffer[..take]);
            output.write_all(&buffer[..take])?;
            remaining -= take as u64;
        }
        let actual: [u8; 32] = hash.finalize().into();
        if actual != expected {
            return Err(invalid("Archive checksum mismatch"));
        }
    }
    if input.read(&mut [0u8; 1])? != 0 {
        return Err(invalid("Unexpected archive data"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut data = Vec::from(*MAGIC);
        data.extend((files.len() as u32).to_le_bytes());
        for (name, body) in files {
            data.extend((name.len() as u32).to_le_bytes());
            data.extend((body.len() as u64).to_le_bytes());
            data.extend(Sha256::digest(body));
            data.extend(name.as_bytes());
            data.extend(*body);
        }
        let mut zip = GzEncoder::new(Vec::new(), Compression::fast());
        zip.write_all(&data).unwrap();
        zip.finish().unwrap()
    }
    fn temp() -> Stage {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        Stage(std::env::temp_dir().join(format!(
            "apt-portable-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    #[test]
    fn extraction_reuses_complete_cache_and_rebuilds_interrupted_cache() {
        let root = temp();
        let payload = archive(&[(APP, b"exe"), ("lib/image.dll", b"dll")]);
        let id = format!("{:x}", Sha256::digest(&payload));
        let path = prepare(&root.0, &payload, &id).unwrap();
        assert_eq!(fs::read(path.join("lib/image.dll")).unwrap(), b"dll");
        fs::write(path.join("sentinel"), b"keep").unwrap();
        assert_eq!(prepare(&root.0, &payload, &id).unwrap(), path);
        assert!(path.join("sentinel").exists());
        fs::remove_file(path.join(".ready")).unwrap();
        prepare(&root.0, &payload, &id).unwrap();
        assert!(!path.join("sentinel").exists());
    }
    #[test]
    fn rejects_paths_that_escape_the_cache_and_cleans_up_failure() {
        for name in [
            "../outside",
            "/outside",
            "C:/outside",
            "a\\outside",
            "a/../outside",
            "a.",
            "a/",
            ".ready",
        ] {
            let root = temp();
            let data = archive(&[(APP, b"exe"), (name, b"bad")]);
            let id = format!("{:x}", Sha256::digest(&data));
            assert!(prepare(&root.0, &data, &id).is_err(), "{name}");
            assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
        }
    }
    #[test]
    fn corrupted_or_truncated_payload_never_creates_a_ready_cache() {
        let root = temp();
        let mut data = archive(&[(APP, b"exe")]);
        let id = format!("{:x}", Sha256::digest(&data));
        data[10] ^= 1;
        assert!(prepare(&root.0, &data, &id).is_err());
        let data = archive(&[(APP, b"exe")]);
        let data = &data[..data.len() - 4];
        let id = format!("{:x}", Sha256::digest(data));
        assert!(prepare(&root.0, data, &id).is_err());
    }

    #[test]
    fn concurrent_launches_publish_one_complete_cache() {
        let root = temp();
        let payload = std::sync::Arc::new(archive(&[(APP, b"exe"), ("lib/image.dll", b"dll")]));
        let id = format!("{:x}", Sha256::digest(&*payload));
        let workers: Vec<_> = (0..3)
            .map(|_| {
                let (path, data, id) = (root.0.clone(), payload.clone(), id.clone());
                std::thread::spawn(move || prepare(&path, &data, &id).unwrap())
            })
            .collect();
        let paths: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert!(paths.iter().all(|p| p == &paths[0]));
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 2);
        assert_eq!(fs::read(paths[0].join("lib/image.dll")).unwrap(), b"dll");
    }
}
