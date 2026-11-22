//! Prepare capture independently of OCR and latch immutable pixels at key-down.
use super::{
    native,
    ocr::{self, Image},
    screen_capture,
};
use crate::{
    capture::latch::{self, Frame},
    price_layout::Rect,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU8, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::POINT,
    System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::GetCursorPos},
};

static SERVICE: OnceLock<Arc<Shared>> = OnceLock::new();
static ALT_STARTED: AtomicI64 = AtomicI64::new(0);
static ALT_KEYS: AtomicU8 = AtomicU8::new(0);
static ALT_OBSERVED: AtomicBool = AtomicBool::new(false);
fn now() -> i64 {
    let mut n = 0;
    unsafe {
        QueryPerformanceCounter(&mut n);
    }
    n
}
fn frequency() -> i64 {
    static F: OnceLock<i64> = OnceLock::new();
    *F.get_or_init(|| {
        let mut n = 0;
        unsafe {
            QueryPerformanceFrequency(&mut n);
        }
        n.max(1)
    })
}
fn alt_down() -> bool {
    unsafe { GetAsyncKeyState(VK_MENU as i32) < 0 }
}
pub fn modifiers_held() -> bool {
    unsafe {
        [VK_MENU, VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN]
            .iter()
            .any(|&k| GetAsyncKeyState(k as i32) < 0)
    }
}
pub fn alt_changed(key: u32, down: bool) {
    let bit = if key == u32::from(VK_LMENU) {
        1
    } else if key == u32::from(VK_RMENU) {
        2
    } else {
        4
    };
    let was_held = alt_down();
    let previous = if down && !was_held {
        ALT_KEYS.swap(bit, Ordering::AcqRel)
    } else if down {
        ALT_KEYS.fetch_or(bit, Ordering::AcqRel)
    } else {
        ALT_KEYS.fetch_and(!bit, Ordering::AcqRel)
    };
    let held = if down {
        previous | bit
    } else {
        previous & !bit
    };
    if down && (previous == 0 || !was_held) {
        ALT_STARTED.store(now(), Ordering::Release);
    }
    if held == 0 {
        ALT_STARTED.store(0, Ordering::Release);
        if let Some(shared) = SERVICE.get() {
            if let Ok(mut c) = shared.cache.lock() {
                c.frame = None;
            }
        }
    }
    // A low-level hook precedes GetAsyncKeyState's update. Do not overwrite
    // these event timestamps with a sampler poll of the previous key state.
    ALT_OBSERVED.store(true, Ordering::Release);
    if let Some(shared) = SERVICE.get() {
        if let Some(t) = shared.thread.get() {
            t.unpark();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    window: u32,
    pid: u32,
    rect: Rect,
    cursor: (i32, i32),
    faults: u8,
}
fn key() -> Option<Key> {
    let w = native::focused_target()?;
    let (pid, _) = native::process(w)?;
    let rect = native::rect(w)?;
    if rect.width < 100
        || rect.height < 100
        || i64::from(rect.width) * i64::from(rect.height) > 24_000_000
    {
        return None;
    }
    let mut p = POINT::default();
    unsafe {
        GetCursorPos(&mut p);
    }
    let faults = native::fixture_root()
        .map(|r| {
            u8::from(r.join("capture-black-accelerated").exists())
                | (u8::from(r.join("capture-all-blank").exists()) << 1)
        })
        .unwrap_or(0);
    Some(Key {
        window: native::id(w),
        pid,
        rect,
        cursor: (p.x, p.y),
        faults,
    })
}
#[derive(Clone)]
pub struct Snapshot {
    pub image: Arc<Image>,
    pub window: u32,
    pub pid: u32,
    pub rect: Rect,
    pub backend: &'static str,
    presented: i64,
    sampled: i64,
}
struct Request {
    key: Key,
    alt_started: i64,
    pressed: i64,
    started: Instant,
    send: mpsc::SyncSender<Result<Snapshot, String>>,
}
struct Cache {
    frame: Option<Frame<Key, Snapshot>>,
    checked: i64,
    error: Option<String>,
}
struct Shared {
    alive: AtomicBool,
    processing: AtomicBool,
    thread: OnceLock<thread::Thread>,
    cache: Mutex<Cache>,
    requests: Mutex<Vec<Request>>,
}
pub struct Sampler {
    shared: Arc<Shared>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
        if let Some(t) = self.shared.thread.get() {
            t.unpark();
        }
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
pub struct Processing(Option<Arc<Shared>>);
impl Drop for Processing {
    fn drop(&mut self) {
        if let Some(s) = &self.0 {
            s.processing.store(false, Ordering::Release);
            if let Some(t) = s.thread.get() {
                t.unpark();
            }
        }
    }
}
pub fn processing() -> Processing {
    let shared = SERVICE.get().cloned();
    if let Some(s) = &shared {
        s.processing.store(true, Ordering::Release);
        s.cache.lock().unwrap().frame = None;
    }
    Processing(shared)
}
pub struct Ticket {
    ready: Option<Result<Snapshot, String>>,
    receiver: Option<mpsc::Receiver<Result<Snapshot, String>>>,
    pressed: i64,
}
impl Ticket {
    pub fn resolve(mut self, check: &impl Fn() -> Result<(), String>) -> Result<Snapshot, String> {
        check()?;
        let result = if let Some(ready) = self.ready.take() {
            ready
        } else {
            let receiver = self.receiver.take().ok_or("Capture request unavailable")?;
            let started = Instant::now();
            loop {
                check()?;
                match receiver.recv_timeout(Duration::from_millis(10)) {
                    Ok(result) => break result,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break Err("Capture worker stopped.".into())
                    }
                    Err(_) if started.elapsed() > Duration::from_millis(250) => {
                        break Err(not_ready())
                    }
                    Err(_) => {}
                }
            }
        }?;
        check()?;
        eprintln!(
            "ocr snapshot_latched backend={} frame_age_ms={} queue_ms={}",
            result.backend,
            (self.pressed - result.presented).max(0) * 1000 / frequency(),
            (now() - self.pressed).max(0) * 1000 / frequency()
        );
        Ok(result)
    }
}
fn not_ready() -> String {
    "Capture was not ready at the shortcut. Keep the advanced tooltip visible, then press the shortcut again.".into()
}
fn selected(cache: &Cache, key: &Key, alt: i64, pressed: i64) -> Option<Snapshot> {
    if pressed - cache.checked > frequency() / 8 {
        return None;
    }
    latch::select(cache.frame.as_ref(), key, alt, pressed, i64::MAX)
}
pub fn freeze() -> Ticket {
    let pressed = now();
    let fail = |s: String| Ticket {
        ready: Some(Err(s)),
        receiver: None,
        pressed,
    };
    let Some(shared) = SERVICE.get() else {
        return fail(not_ready());
    };
    let Some(key) = key() else {
        return fail("Target application changed; OCR cancelled.".into());
    };
    if !alt_down() {
        return fail(
            "Hold Alt until the detailed tooltip is visible, then press the OCR shortcut.".into(),
        );
    }
    let alt = ALT_STARTED.load(Ordering::Acquire);
    // The timestamp and immutable frame reference form one atomic latch. A
    // sampler update between keyboard dispatch and this lock cannot discard
    // the frame merely because the first timestamp preceded that update.
    let (pressed, frame) = if let Ok(cache) = shared.cache.lock() {
        let pressed = now();
        (pressed, selected(&cache, &key, alt, pressed))
    } else {
        (now(), None)
    };
    if let Some(frame) = frame {
        return Ticket {
            ready: Some(Ok(frame)),
            receiver: None,
            pressed,
        };
    }
    let (send, receiver) = mpsc::sync_channel(1);
    if let Ok(mut requests) = shared.requests.lock() {
        if requests.len() >= 2 {
            let old = requests.remove(0);
            let _ = old.send.try_send(Err("Capture superseded.".into()));
        }
        requests.push(Request {
            key,
            alt_started: alt,
            pressed,
            started: Instant::now(),
            send,
        });
    }
    if let Some(t) = shared.thread.get() {
        t.unpark();
    }
    Ticket {
        ready: None,
        receiver: Some(receiver),
        pressed,
    }
}
pub fn diagnostics() -> serde_json::Value {
    let Some(s) = SERVICE.get() else {
        return serde_json::json!({"ready":false});
    };
    let c = s.cache.lock().unwrap();
    serde_json::json!({"ready":c.frame.is_some(),"alt_held":alt_down(),"error":c.error,"backend":c.frame.as_ref().map(|f|f.value.backend),"frame_age_ms":c.frame.as_ref().map(|f|(now()-f.value.sampled).max(0)*1000/frequency())})
}
pub fn start() -> Sampler {
    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        processing: AtomicBool::new(false),
        thread: OnceLock::new(),
        cache: Mutex::new(Cache {
            frame: None,
            checked: 0,
            error: None,
        }),
        requests: Mutex::new(Vec::new()),
    });
    assert!(
        SERVICE.set(shared.clone()).is_ok(),
        "Snapshot sampler started twice"
    );
    let state = shared.clone();
    let worker = thread::spawn(move || run(&state));
    Sampler {
        shared,
        worker: Some(worker),
    }
}
fn run(shared: &Shared) {
    let _ = shared.thread.set(thread::current());
    let mut binding = None;
    let mut stream = None;
    let mut pending: Vec<Request> = Vec::new();
    while shared.alive.load(Ordering::Acquire) {
        pending.extend(shared.requests.lock().unwrap().drain(..));
        let identity = key();
        let observed = ALT_OBSERVED.load(Ordering::Acquire);
        let held = alt_down();
        if !observed && held {
            let _ = ALT_STARTED.compare_exchange(0, now(), Ordering::AcqRel, Ordering::Acquire);
        }
        if let Some(k) = identity {
            // Preparing duplication while idle does not copy pixels to RAM.
            let source = (k.window, k.pid, k.rect, k.faults);
            if binding != Some(source) {
                binding = Some(source);
                stream = if k.faults == 0 {
                    screen_capture::Stream::open(k.rect).ok()
                } else {
                    None
                };
                shared.cache.lock().unwrap().frame = None;
            }
            if (held || !pending.is_empty())
                && !native::panel_visible()
                && !shared.processing.load(Ordering::Acquire)
            {
                let check = || {
                    if !shared.alive.load(Ordering::Acquire) || key() != Some(k) {
                        Err("Target changed while preparing a frame.".into())
                    } else {
                        Ok(())
                    }
                };
                let frame = if let Some(s) = &stream {
                    s.poll(0, &check).map(|sample| {
                        sample.map(|(image, presented)| (image, "Desktop Duplication", presented))
                    })
                } else if held {
                    let start = now();
                    ocr::sample_frame(native::hwnd(k.window), k.rect, &|| {
                        check()?;
                        if !alt_down() {
                            Err("Alt released before a frame was available.".into())
                        } else {
                            Ok(())
                        }
                    })
                    .map(|(image, backend)| Some((image, backend, start)))
                } else {
                    Ok(None)
                };
                let mut c = shared.cache.lock().unwrap();
                match frame {
                    Ok(Some((image, backend, presented)))
                        if crate::capture::frame::has_content(&image.pixels) =>
                    {
                        let sample = Snapshot {
                            image: Arc::new(image),
                            window: k.window,
                            pid: k.pid,
                            rect: k.rect,
                            backend,
                            presented,
                            sampled: now(),
                        };
                        let alt = ALT_STARTED.load(Ordering::Acquire);
                        // Requests retain their own Alt epoch after key-up.
                        for req in pending.iter().filter(|r| r.key == k) {
                            let candidate = Frame {
                                key: k,
                                alt_started: req.alt_started,
                                presented,
                                value: sample.clone(),
                            };
                            if let Some(f) = latch::select(
                                Some(&candidate),
                                &req.key,
                                req.alt_started,
                                req.pressed,
                                i64::MAX,
                            ) {
                                let _ = req.send.try_send(Ok(f));
                            }
                        }
                        if held && alt > 0 && presented >= alt {
                            c.frame = Some(Frame {
                                key: k,
                                alt_started: alt,
                                presented,
                                value: sample,
                            });
                        }
                        c.checked = now();
                        c.error = None;
                    }
                    Ok(None) => {
                        c.checked = now();
                    }
                    Ok(Some(_)) => {
                        c.frame = None;
                        c.error = Some("Captured frame is blank.".into());
                        stream = None;
                    }
                    Err(error) => {
                        c.frame = None;
                        let moved = key().is_some_and(|n| {
                            (n.window, n.pid, n.rect, n.faults) == source && n.cursor != k.cursor
                        });
                        c.error = Some(error);
                        if !moved {
                            stream = None;
                        }
                    }
                }
            } else if !held {
                shared.cache.lock().unwrap().frame = None;
            }
        } else {
            binding = None;
            stream = None;
            shared.cache.lock().unwrap().frame = None;
        }
        let mut waiting = Vec::new();
        for request in pending.drain(..) {
            let cache = shared.cache.lock().unwrap();
            let result = if identity != Some(request.key) {
                Some(Err("Target changed; capture cancelled.".into()))
            } else if let Some(f) =
                selected(&cache, &request.key, request.alt_started, request.pressed)
            {
                Some(Ok(f))
            } else if request.started.elapsed() > Duration::from_millis(120) {
                Some(Err(cache.error.clone().unwrap_or_else(not_ready)))
            } else {
                None
            };
            drop(cache);
            if let Some(result) = result {
                let _ = request.send.try_send(result);
            } else {
                waiting.push(request);
            }
        }
        pending = waiting;
        thread::park_timeout(Duration::from_millis(16));
    }
}
