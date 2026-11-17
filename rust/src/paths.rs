//! Per-user storage; Windows does not depend on Unix HOME/XDG variables.
use std::path::PathBuf;
pub fn config_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("AwakenedPoETradeRust"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            })
    }
}
pub fn cache_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
            })
    }
}

#[cfg(target_os = "windows")]
pub fn private_windows_test_root() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("APT_RS_WINDOWS_TEST_DIR")?);
    if !path
        .file_name()?
        .to_string_lossy()
        .starts_with("apt-windows-desktop-")
    {
        return None;
    }
    let token = std::env::var("APT_RS_WINDOWS_TEST_TOKEN").ok()?;
    if token.len() < 32
        || std::fs::read_to_string(path.join("fixture.guard"))
            .ok()?
            .trim()
            != token
    {
        return None;
    }
    path.canonicalize().ok()
}
