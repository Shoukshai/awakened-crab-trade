//! Test-only fake game. Requires a guarded private test directory and never runs normally.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows fixture only");
}
#[cfg(target_os = "windows")]
fn main() {
    if let Err(e) = test::run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(target_os = "windows")]
mod test {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU32, Ordering},
    };
    use windows_sys::Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{DataExchange::*, LibraryLoader::*, Memory::*},
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    };
    static CLOUD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static COPIES: AtomicU32 = AtomicU32::new(0);
    static KEYS: AtomicU32 = AtomicU32::new(0);
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn root() -> PathBuf {
        PathBuf::from(std::env::var_os("APT_RS_WINDOWS_TEST_DIR").unwrap())
    }
    fn state(window: HWND) {
        let data = serde_json::json!({"ready":true,"window":window as usize,"copies":COPIES.load(Ordering::Relaxed),"keys":KEYS.load(Ordering::Relaxed),"foreground":unsafe{GetForegroundWindow()}==window,"topmost":unsafe{GetWindowLongPtrW(window,GWL_EXSTYLE)} as u32 & WS_EX_TOPMOST!=0});
        let file = root().join(if std::env::args().any(|a| a == "--foreign") {
            "foreign.state.json"
        } else if CLOUD.load(Ordering::Relaxed) {
            "cloud.state.json"
        } else {
            "fixture.state.json"
        });
        let tmp = file.with_extension("tmp");
        let _ = std::fs::write(&tmp, data.to_string());
        let _ = std::fs::rename(tmp, file);
    }
    unsafe fn clipboard(window: HWND, text: &str) {
        let text = wide(text);
        if OpenClipboard(window) == 0 {
            let _ = std::fs::write(
                root().join("clipboard-error.txt"),
                format!("OpenClipboard: {}", std::io::Error::last_os_error()),
            );
            return;
        }
        let h = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2);
        if !h.is_null() {
            let p = GlobalLock(h) as *mut u16;
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(text.as_ptr(), p, text.len());
                GlobalUnlock(h);
                EmptyClipboard();
                if SetClipboardData(13, h).is_null() {
                    let _ = std::fs::write(
                        root().join("clipboard-error.txt"),
                        format!("SetClipboardData: {}", std::io::Error::last_os_error()),
                    );
                    GlobalFree(h);
                }
            } else {
                GlobalFree(h);
            }
        }
        CloseClipboard();
    }
    unsafe extern "system" fn proc(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        match message {
            WM_PAINT => {
                if !CLOUD.load(Ordering::Relaxed) {
                    return DefWindowProcW(window, message, w, l);
                }
                let mut paint: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(window, &mut paint);
                let file = root().join(if GetAsyncKeyState(i32::from(VK_MENU)) < 0 {
                    "fixture-ocr-advanced.bmp"
                } else {
                    "fixture-ocr.bmp"
                });
                if file.exists() {
                    let bitmap = LoadImageW(
                        std::ptr::null_mut(),
                        wide(&file.to_string_lossy()).as_ptr(),
                        IMAGE_BITMAP,
                        0,
                        0,
                        LR_LOADFROMFILE,
                    ) as HBITMAP;
                    if !bitmap.is_null() {
                        let mem = CreateCompatibleDC(dc);
                        let old = SelectObject(mem, bitmap);
                        let mut b: BITMAP = std::mem::zeroed();
                        GetObjectW(
                            bitmap,
                            std::mem::size_of::<BITMAP>() as i32,
                            (&mut b as *mut BITMAP).cast(),
                        );
                        BitBlt(dc, 0, 0, b.bmWidth, b.bmHeight, mem, 0, 0, SRCCOPY);
                        SelectObject(mem, old);
                        DeleteDC(mem);
                        DeleteObject(bitmap);
                    }
                }
                EndPaint(window, &paint);
                0
            }
            WM_KEYUP | WM_SYSKEYUP => {
                if CLOUD.load(Ordering::Relaxed) {
                    InvalidateRect(window, std::ptr::null(), 1);
                    UpdateWindow(window);
                }
                DefWindowProcW(window, message, w, l)
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if CLOUD.load(Ordering::Relaxed) {
                    InvalidateRect(window, std::ptr::null(), 1);
                    UpdateWindow(window);
                }
                KEYS.fetch_add(1, Ordering::Relaxed);
                if w == b'C' as usize
                    && GetKeyState(i32::from(VK_CONTROL)) < 0
                    && GetKeyState(i32::from(VK_MENU)) < 0
                {
                    if let Ok(text) = std::fs::read_to_string(root().join("fixture-item.txt")) {
                        clipboard(window, &text);
                        COPIES.fetch_add(1, Ordering::Relaxed);
                    }
                }
                if w == usize::from(VK_F12) && OpenClipboard(window) != 0 {
                    let h = GetClipboardData(13);
                    if !h.is_null() {
                        let p = GlobalLock(h) as *const u16;
                        if !p.is_null() {
                            let text = std::slice::from_raw_parts(p, GlobalSize(h) / 2);
                            let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
                            let _ = std::fs::write(
                                root().join("clipboard-read.txt"),
                                String::from_utf16_lossy(&text[..end]),
                            );
                            GlobalUnlock(h);
                        }
                    }
                    CloseClipboard();
                }
                state(window);
                0
            }
            WM_ACTIVATE | WM_SHOWWINDOW => {
                state(window);
                DefWindowProcW(window, message, w, l)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, w, l),
        }
    }
    pub fn run() -> Result<(), String> {
        let root = root();
        let token = std::env::var("APT_RS_WINDOWS_TEST_TOKEN").map_err(|e| e.to_string())?;
        if !root
            .file_name()
            .is_some_and(|s| s.to_string_lossy().starts_with("apt-windows-desktop-"))
            || token.len() < 32
            || std::fs::read_to_string(root.join("fixture.guard"))
                .ok()
                .as_deref()
                != Some(token.as_str())
            || std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().and_then(|p| p.canonicalize().ok()))
                != root.canonicalize().ok()
        {
            return Err("private Windows fixture only".into());
        }
        unsafe {
            let foreign = std::env::args().any(|a| a == "--foreign");
            let cloud = std::env::args().any(|a| a == "--cloud");
            CLOUD.store(cloud, Ordering::Relaxed);
            let windowed = std::env::args().any(|a| a == "--windowed");
            let class = wide(if foreign {
                "APTForeignFixture"
            } else if cloud {
                "APTCloudFixture"
            } else {
                "POEWindowClass"
            });
            let instance = GetModuleHandleW(std::ptr::null());
            let brush = CreateSolidBrush(if foreign { 0x665544 } else { 0x302820 });
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                hbrBackground: brush,
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                ..std::mem::zeroed()
            };
            if RegisterClassW(&wc) == 0 {
                return Err("RegisterClass failed".into());
            }
            let w = CreateWindowExW(
                if cloud { WS_EX_TOPMOST } else { 0 },
                class.as_ptr(),
                wide(if foreign {
                    "APT foreign fixture"
                } else if cloud {
                    "Boosteroid (private fixture)"
                } else {
                    "Path of Exile"
                })
                .as_ptr(),
                if foreign || windowed {
                    WS_OVERLAPPEDWINDOW
                } else {
                    WS_POPUP
                },
                if foreign || windowed { 40 } else { 0 },
                if foreign || windowed { 40 } else { 0 },
                if foreign {
                    500
                } else if windowed {
                    1400
                } else {
                    GetSystemMetrics(SM_CXSCREEN)
                },
                if foreign {
                    260
                } else if windowed {
                    900
                } else {
                    GetSystemMetrics(SM_CYSCREEN)
                },
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if w.is_null() {
                return Err("CreateWindow failed".into());
            }
            ShowWindow(w, SW_SHOW);
            SetForegroundWindow(w);
            UpdateWindow(w);
            if !foreign {
                clipboard(w, "APT private clipboard sentinel");
            }
            state(w);
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            DeleteObject(brush);
        }
        Ok(())
    }
}
