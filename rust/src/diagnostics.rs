//! Local diagnostic reports. Never send logs or machine information anywhere.
#[cfg(target_os = "windows")]
use std::{path::PathBuf, sync::OnceLock};
#[cfg(target_os = "windows")]
static SESSION: OnceLock<(PathBuf, u64)> = OnceLock::new();

#[cfg(target_os = "windows")]
pub fn start(path: PathBuf, offset: u64) {
    let _ = SESSION.set((path, offset));
}

pub fn cpu() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        let mut bytes = Vec::new();
        if __cpuid(0x80000000).eax >= 0x80000004 {
            for leaf in 0x80000002..=0x80000004 {
                let r = __cpuid(leaf);
                for v in [r.eax, r.ebx, r.ecx, r.edx] {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        let brand = String::from_utf8_lossy(&bytes)
            .trim_matches('\0')
            .trim()
            .to_owned();
        let mut features = Vec::new();
        for (name, present) in [
            ("sse2", std::is_x86_feature_detected!("sse2")),
            ("avx", std::is_x86_feature_detected!("avx")),
            ("avx2", std::is_x86_feature_detected!("avx2")),
            ("fma", std::is_x86_feature_detected!("fma")),
            ("avx512f", std::is_x86_feature_detected!("avx512f")),
            ("avx512bw", std::is_x86_feature_detected!("avx512bw")),
            ("avx512vl", std::is_x86_feature_detected!("avx512vl")),
        ] {
            if present {
                features.push(name);
            }
        }
        format!("{brand}; features={}", features.join(","))
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        std::env::consts::ARCH.to_owned()
    }
}

#[allow(deprecated)] // Read the worker's floating-point controls without changing them.
pub fn math_state() -> String {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        format!("mxcsr=0x{:08x}", std::arch::x86_64::_mm_getcsr())
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        "mxcsr=unavailable".into()
    }
}

#[cfg(any(target_os = "windows", test))]
fn read_from(path: &std::path::Path, offset: u64) -> Result<String, String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    file.seek(SeekFrom::Start(if offset <= length { offset } else { 0 }))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(target_os = "windows")]
pub fn snapshot() -> String {
    use windows_sys::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};
    let os = unsafe {
        let mut v: OSVERSIONINFOW = std::mem::zeroed();
        v.dwOSVersionInfoSize = std::mem::size_of_val(&v) as u32;
        if GetVersionExW(&mut v) != 0 {
            format!(
                "Windows {}.{} build {}",
                v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber
            )
        } else {
            "Windows (version unavailable)".into()
        }
    };
    let body = match SESSION.get() {
        Some((path, offset)) => {
            read_from(path, *offset).unwrap_or_else(|e| format!("Cannot read session log: {e}\n"))
        }
        None => "No file-backed session log is available.\n".into(),
    };
    format!("Awakened Crab Trade diagnostics\nVersion: {}\nEdition: {}\nOS: {os}\nCPU: {}\nProcess: {} ({})\nMath: {}\n--- Complete current session log ---\n{body}",
        env!("CARGO_PKG_VERSION"), crate::edition::id(), cpu(), std::process::id(),
        std::env::consts::ARCH, math_state())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_report_reads_every_new_line_and_excludes_older_runs() {
        let p = std::env::temp_dir().join(format!("crab-session-{}.log", std::process::id()));
        std::fs::write(&p, "previous run\ncurrent run\nfirst\nlast\n").unwrap();
        assert_eq!(read_from(&p, 13).unwrap(), "current run\nfirst\nlast\n");
        std::fs::write(&p, "new\n").unwrap();
        assert_eq!(read_from(&p, 13).unwrap(), "new\n");
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn hardware_description_and_math_controls_are_available_without_inference() {
        assert!(!cpu().is_empty());
        assert!(math_state().starts_with("mxcsr="));
    }
}
