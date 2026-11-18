//! All capture and recognition work runs on the bounded capture worker, never GTK.
use super::{game_input::Context, native};
use crate::ocr::{Draft, Line};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use windows::{
    core::h,
    Globalization::Language,
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}
impl Image {
    fn crop(&self, r: [f64; 4]) -> Self {
        let x = (r[0] * self.width as f64 / 100.) as usize;
        let y = (r[1] * self.height as f64 / 100.) as usize;
        let width = ((r[2] * self.width as f64 / 100.) as usize)
            .min(self.width - x)
            .max(1);
        let height = ((r[3] * self.height as f64 / 100.) as usize)
            .min(self.height - y)
            .max(1);
        let mut pixels = Vec::with_capacity(width * height * 4);
        for row in y..y + height {
            pixels.extend_from_slice(
                &self.pixels[(row * self.width + x) * 4..(row * self.width + x + width) * 4],
            );
        }
        Self {
            width,
            height,
            pixels,
        }
    }
    /// Find the yellow two-line rare header before OCR. This keeps inventory
    /// and sales-history text out of the expensive fallback pass. Leave enough
    /// width for advanced modifier descriptions, which exceed the base name.
    fn rare_hint(&self) -> Option<Self> {
        let mut bands: Vec<(usize, usize, usize, usize, usize)> = Vec::new();
        for y in 0..(self.height / 2).min(600) {
            let mut start = 0;
            let mut last = 0;
            let mut count = 0;
            let finish =
                |start: usize,
                 last: usize,
                 count: usize,
                 bands: &mut Vec<(usize, usize, usize, usize, usize)>| {
                    if count < 12 || last - start < 65 || last - start > 650 {
                        return;
                    }
                    if let Some(b) = bands.iter_mut().rev().find(|b| {
                        y - b.1 < 30 && ((start + last) as isize - (b.2 + b.3) as isize).abs() < 100
                    }) {
                        b.1 = y;
                        b.2 = b.2.min(start);
                        b.3 = b.3.max(last);
                        b.4 += count;
                    } else {
                        bands.push((y, y, start, last, count));
                    }
                };
            for x in 0..self.width {
                let i = (y * self.width + x) * 4;
                let b = self.pixels[i] as u32;
                let g = self.pixels[i + 1] as u32;
                let r = self.pixels[i + 2] as u32;
                if r > 185 && g > 185 && b < 170 && r * 10 < g * 13 {
                    if count > 0 && x - last > 48 {
                        finish(start, last, count, &mut bands);
                        count = 0;
                    }
                    if count == 0 {
                        start = x;
                    }
                    last = x;
                    count += 1;
                }
            }
            finish(start, last, count, &mut bands);
        }
        let b = bands
            .iter()
            .filter(|b| b.1 - b.0 >= 16)
            .max_by_key(|b| b.4)?;
        let left = b.2.saturating_sub(300);
        let right = (b.3 + 300).min(self.width);
        let top = b.0.saturating_sub(22);
        let mut bottom = (top + 1000).min(self.height);
        let mut streak = 0;
        for y in (b.1 + 120)..bottom {
            let start = left + (right - left) / 6;
            let end = right - (right - left) / 6;
            let dark = (start..end)
                .filter(|x| {
                    self.pixels[(y * self.width + x) * 4..][..3]
                        .iter()
                        .all(|v| *v < 65)
                })
                .count();
            if dark * 100 < (end - start) * 35 {
                streak += 1;
            } else {
                streak = 0;
            }
            if streak >= 16 {
                bottom = y - 12;
                break;
            }
        }
        let mut image = self.crop([
            left as f64 / self.width as f64 * 100.,
            top as f64 / self.height as f64 * 100.,
            (right - left) as f64 / self.width as f64 * 100.,
            (bottom - top) as f64 / self.height as f64 * 100.,
        ]);
        // Influence ornaments and the header frame can make the OCR engine
        // discard the base line. Keep the observed gold text, clear its sides.
        let text_left = b.2.saturating_sub(left + 8);
        let text_right = (b.3 + 9 - left).min(image.width);
        for y in 0..(b.1 + 5 - top).min(image.height) {
            for x in (0..text_left).chain(text_right..image.width) {
                image.pixels[(y * image.width + x) * 4..][..3].fill(0);
            }
        }
        Some(image)
    }
    fn rarity(&self, x: f64, y: f64, width: f64, height: f64) -> Option<crate::item::Rarity> {
        let mut votes = [0usize; 4];
        for yy in (y.max(0.) as usize)..((y + height).max(0.) as usize).min(self.height) {
            for xx in (x.max(0.) as usize)..((x + width).max(0.) as usize).min(self.width) {
                let i = (yy * self.width + xx) * 4;
                let b = self.pixels[i] as i32;
                let g = self.pixels[i + 1] as i32;
                let r = self.pixels[i + 2] as i32;
                if r > 160 && g > 145 && b < g * 3 / 4 {
                    votes[0] += 1;
                } else if r > 150 && g > 65 && g < r * 3 / 4 && b < g {
                    votes[1] += 1;
                } else if b > 150 && b > r * 6 / 5 && b > g * 6 / 5 {
                    votes[2] += 1;
                } else if r > 170 && g > 170 && b > 170 {
                    votes[3] += 1;
                }
            }
        }
        let (index, count) = votes.iter().enumerate().max_by_key(|(_, n)| **n)?;
        (*count > 10).then_some(match index {
            0 => crate::item::Rarity::Rare,
            1 => crate::item::Rarity::Unique,
            2 => crate::item::Rarity::Magic,
            _ => crate::item::Rarity::Normal,
        })
    }
    fn append_word(&self, lines: &mut Vec<Line>, word: Line, same_group: bool) {
        let nearby = lines
            .last()
            .is_some_and(|l| word.x - l.x - l.width <= word.height.max(l.height) * 2.);
        if same_group && nearby {
            let line = lines.last_mut().unwrap();
            line.text.push(' ');
            line.text.push_str(&word.text);
            line.width = line.width.max(word.x + word.width - line.x);
            line.height = line.height.max(word.y + word.height - line.y);
            line.rarity = self.rarity(line.x, line.y, line.width, line.height);
            line.source = self
                .line(String::new(), line.x, line.y, line.width, line.height)
                .source;
        } else {
            lines.push(word);
        }
    }
    fn line(&self, text: String, x: f64, y: f64, width: f64, height: f64) -> Line {
        let mut sources = [0usize; 3];
        for yy in y.max(0.) as usize..((y + height).max(0.) as usize).min(self.height) {
            for xx in x.max(0.) as usize..((x + width).max(0.) as usize).min(self.width) {
                let p = &self.pixels[(yy * self.width + xx) * 4..];
                let (b, g, r) = (p[0] as i32, p[1] as i32, p[2] as i32);
                if r > 110 && g > 100 && r >= g && g - b > 22 && r - g < 32 {
                    sources[0] += 1;
                } else if b > 140 && g > 130 && g - r > 12 && b - g > 5 && b - g < 30 {
                    sources[1] += 1;
                } else if b > 110 && b - r > 22 && b - g > 18 {
                    sources[2] += 1;
                }
            }
        }
        let source = if sources[0] > 30 && sources[0] > (sources[1] + sources[2]) * 3 {
            Some("fractured".into())
        } else if sources[1] > 30 && sources[1] > (sources[0] + sources[2]) * 3 {
            Some("crafted".into())
        } else {
            None
        };
        Line {
            rarity: self.rarity(x, y, width, height),
            text,
            x,
            y,
            width,
            height,
            source,
            readings: Vec::new(),
        }
    }

    /// Remove neutral map/NPC labels while retaining colored modifier glyphs.
    /// This reading supplements the original; it never replaces properties.
    fn colored(&self) -> Self {
        let mut pixels = self.pixels.clone();
        for p in pixels.as_chunks_mut::<4>().0 {
            let (b, g, r) = (p[0] as i32, p[1] as i32, p[2] as i32);
            let blue = b > 105 && b - r > 22 && b - g > 18;
            let crafted = b > 130 && g > 130 && r > 95 && b - r > 12;
            // Gold map/UI labels can touch a blue stat's first digit. Gold
            // fractured rolls are retained by the independent original pass.
            if !blue && !crafted {
                p[..3].fill(0);
            }
        }
        Self {
            width: self.width,
            height: self.height,
            pixels,
        }
    }
}
pub(super) struct Apartment;
impl Apartment {
    pub(super) fn new() -> Result<Self, String> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|e| e.to_string())?;
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() }
    }
}
struct Target<'a> {
    context: &'a Context<'a>,
    window: HWND,
    pid: u32,
    path: PathBuf,
}
impl<'a> Target<'a> {
    fn new(context: &'a Context<'a>) -> Result<Self, String> {
        context.check()?;
        let window = native::focused_target()
            .ok_or("The selected cloud client must be in the foreground.")?;
        let (pid, path) = native::process(window).ok_or("Cloud client process unavailable.")?;
        Ok(Self {
            context,
            window,
            pid,
            path,
        })
    }
    fn check(&self) -> Result<(), String> {
        self.context.check()?;
        if unsafe { GetForegroundWindow() } != self.window
            || !native::is_target(self.window)
            || native::process(self.window) != Some((self.pid, self.path.clone()))
        {
            return Err("Cloud client changed; OCR cancelled.".into());
        }
        Ok(())
    }
    fn alt(&self) -> Result<(), String> {
        self.check()?;
        if unsafe { GetAsyncKeyState(i32::from(VK_MENU)) } >= 0 {
            Err("Hold Alt before Ctrl+D and keep Alt held until the image is captured.".into())
        } else {
            Ok(())
        }
    }
}
// Client DC fallback supports ordinary rendered windows; DXGI is preferred
// for hardware-decoded streams. Never use PrintWindow on the cloud client.
fn gdi(window: HWND, rect: crate::price_layout::Rect, desktop: bool) -> Result<Image, String> {
    unsafe {
        let owner = if desktop {
            std::ptr::null_mut()
        } else {
            window
        };
        let screen = GetDC(owner);
        if screen.is_null() {
            return Err(native::error("Screen capture"));
        }
        let dc = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, rect.width, rect.height);
        if dc.is_null() || bitmap.is_null() {
            if !dc.is_null() {
                DeleteDC(dc);
            }
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            ReleaseDC(owner, screen);
            return Err(native::error("Capture buffer"));
        }
        let old = SelectObject(dc, bitmap);
        let ok = BitBlt(
            dc,
            0,
            0,
            rect.width,
            rect.height,
            screen,
            if desktop { rect.x } else { 0 },
            if desktop { rect.y } else { 0 },
            SRCCOPY | CAPTUREBLT,
        );
        SelectObject(dc, old);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = rect.width;
        info.bmiHeader.biHeight = -rect.height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = vec![0; rect.width as usize * rect.height as usize * 4];
        let rows = if ok != 0 {
            GetDIBits(
                dc,
                bitmap,
                0,
                rect.height as u32,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };
        DeleteObject(bitmap);
        DeleteDC(dc);
        ReleaseDC(owner, screen);
        if rows != rect.height {
            return Err(native::error("Read screen pixels"));
        }
        Ok(Image {
            width: rect.width as usize,
            height: rect.height as usize,
            pixels,
        })
    }
}
fn native_ocr(image: &Image, check: &impl Fn() -> Result<(), String>) -> Result<Vec<Line>, String> {
    let _apartment = Apartment::new()?;
    let language = Language::CreateLanguage(h!("en-US")).map_err(|e| e.to_string())?;
    let engine = OcrEngine::TryCreateFromLanguage(&language)
        .map_err(|e| format!("Windows English OCR unavailable: {e}"))?;
    if image.width.max(image.height)
        > OcrEngine::MaxImageDimension().map_err(|e| e.to_string())? as usize
    {
        return Err("Image exceeds Windows OCR limit.".into());
    }
    let writer = DataWriter::new().map_err(|e| e.to_string())?;
    writer
        .WriteBytes(&image.pixels)
        .map_err(|e| e.to_string())?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &writer.DetachBuffer().map_err(|e| e.to_string())?,
        BitmapPixelFormat::Bgra8,
        image.width as i32,
        image.height as i32,
        BitmapAlphaMode::Ignore,
    )
    .map_err(|e| e.to_string())?;
    let operation = engine.RecognizeAsync(&bitmap).map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        if let Err(e) = check() {
            let _ = operation.Cancel();
            return Err(e);
        }
        if start.elapsed() > Duration::from_secs(6) {
            let _ = operation.Cancel();
            return Err("Windows OCR timed out.".into());
        }
        let status = operation.Status().map_err(|e| e.to_string())?;
        if status.0 != 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = operation.GetResults().map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    for line in result.Lines().map_err(|e| e.to_string())? {
        let mut first = true;
        for word in line.Words().map_err(|e| e.to_string())? {
            let r = word.BoundingRect().map_err(|e| e.to_string())?;
            image.append_word(
                &mut lines,
                image.line(
                    word.Text().map_err(|e| e.to_string())?.to_string(),
                    r.X as f64,
                    r.Y as f64,
                    r.Width as f64,
                    r.Height as f64,
                ),
                !first,
            );
            first = false;
        }
    }
    Ok(lines)
}
/// The OS also terminates local OCR if the checker exits while it is running.
struct ChildJob(HANDLE);
impl ChildJob {
    fn new() -> Result<Self, String> {
        unsafe {
            use windows_sys::Win32::System::JobObjects::*;
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(native::error("OCR process job"));
            }
            let job = Self(handle);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
            ) == 0
            {
                return Err(native::error("OCR process lifetime"));
            }
            Ok(job)
        }
    }
    fn assign(&self, child: &std::process::Child) -> Result<(), String> {
        use std::os::windows::io::AsRawHandle;
        if unsafe {
            windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(
                self.0,
                child.as_raw_handle(),
            )
        } == 0
        {
            Err(native::error("Attach OCR process"))
        } else {
            Ok(())
        }
    }
}
impl Drop for ChildJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
pub fn fallback(
    image: &Image,
    check: &impl Fn() -> Result<(), String>,
) -> Result<Vec<Line>, String> {
    fallback_pass(image, check, false, 2)
}
fn neural_pass(
    image: &Image,
    colored: bool,
    check: &impl Fn() -> Result<(), String>,
) -> Result<Vec<Line>, String> {
    let root = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Runtime directory unavailable")?
        .to_owned();
    crate::neural_ocr::recognize(
        &root,
        crate::neural_ocr::Image {
            width: image.width,
            height: image.height,
            bgra: &image.pixels,
        },
        colored,
        check,
    )
    .map(|readings| {
        readings
            .into_iter()
            .map(|r| {
                let text = if r.confidence < 0.55 {
                    format!("Uncertain OCR: {}", r.text)
                } else {
                    r.text
                };
                image.line(
                    text,
                    r.x as f64,
                    r.y as f64,
                    r.width as f64,
                    r.height as f64,
                )
            })
            .collect()
    })
}
fn fallback_pass(
    image: &Image,
    check: &impl Fn() -> Result<(), String>,
    scout: bool,
    detail_scale: usize,
) -> Result<Vec<Line>, String> {
    use std::os::windows::process::CommandExt;
    let runtime = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Runtime directory unavailable")?
        .to_owned();
    let exe = runtime.join("tesseract.exe");
    let mut f = Vec::new();
    // Upscale small PoE text, invert bright glyphs for the English LSTM model.
    let scale = if !scout && image.width <= 2560 && image.height <= 1600 {
        detail_scale
    } else {
        1
    };
    write!(
        f,
        "P5\n{} {}\n255\n",
        image.width * scale,
        image.height * scale
    )
    .map_err(|e| e.to_string())?;
    let mut row = Vec::with_capacity(image.width * scale);
    for y in 0..image.height {
        row.clear();
        for x in 0..image.width {
            let p = &image.pixels[(y * image.width + x) * 4..][..3];
            let light = *p.iter().max().unwrap();
            let value = 255 - light;
            for _ in 0..scale {
                row.push(value);
            }
        }
        for _ in 0..scale {
            f.write_all(&row).map_err(|e| e.to_string())?;
        }
    }
    check()?;
    let job = ChildJob::new()?;
    let mut child = Command::new(exe)
        .args(["stdin", "stdout"])
        .args(["--tessdata-dir"])
        .arg(runtime.join("share/tessdata"))
        .args([
            "-l",
            "eng",
            "--psm",
            if !scout && image.width < 1100 {
                "6"
            } else {
                "11"
            },
            "tsv",
        ])
        .env("OMP_THREAD_LIMIT", "2")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| format!("Local OCR could not start: {e}"))?;
    if let Err(error) = job.assign(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let mut input = child.stdin.take().ok_or("OCR input pipe unavailable")?;
    let output = child.stdout.take().ok_or("OCR output pipe unavailable")?;
    let writer = std::thread::spawn(move || input.write_all(&f));
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        output.take(2_000_001).read_to_end(&mut data).map(|_| data)
    });
    let started = Instant::now();
    let result = loop {
        if let Err(error) = check().and_then(|_| {
            if started.elapsed() < Duration::from_secs(18) {
                Ok(())
            } else {
                Err("OCR timed out. Keep the advanced tooltip visible and try again.".into())
            }
        }) {
            break Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err("Local OCR failed. Check the bundled English language data.".into())
                }
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(error) => break Err(error.to_string()),
        }
    };
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let written = writer.join();
    let read = reader.join();
    result?;
    written
        .map_err(|_| "OCR input worker failed")?
        .map_err(|e| e.to_string())?;
    let data = read
        .map_err(|_| "OCR output worker failed")?
        .map_err(|e| e.to_string())?;
    if data.len() > 2_000_000 {
        return Err("OCR output too large.".into());
    }
    let text = String::from_utf8(data).map_err(|e| e.to_string())?;
    let mut lines: Vec<Line> = Vec::new();
    let mut previous = String::new();
    for row in text.lines().skip(1) {
        let c: Vec<_> = row.splitn(12, '\t').collect();
        if c.len() != 12 || c[0] != "5" || c[11].trim().is_empty() {
            continue;
        }
        let group = c[1..5].join("/");
        let n = |i: usize| c[i].parse::<f64>().unwrap_or(0.) / scale as f64;
        let (x, y, w, h) = (n(6), n(7), n(8), n(9));
        image.append_word(
            &mut lines,
            image.line(c[11].into(), x, y, w, h),
            group == previous,
        );
        previous = group;
    }
    Ok(lines)
}
pub fn recognize(image: &Image, check: &impl Fn() -> Result<(), String>) -> Result<Draft, String> {
    check()?;
    let rough = image.rare_hint();
    let scout = rough.as_ref().unwrap_or(image);
    let mut passes = Vec::new();
    if let Ok(lines) = native_ocr(scout, check) {
        passes.push((lines, "Windows OCR"));
    }
    check()?;
    if passes
        .first()
        .is_none_or(|(lines, _)| crate::ocr::tooltip_bounds(lines).is_err())
    {
        passes.push((
            fallback_pass(scout, check, rough.is_none(), 2)?,
            "Local OCR",
        ));
    }
    let mut source = scout;
    let mut detailed_scout = rough.is_some();
    let mut located = passes.into_iter().find_map(|(lines, engine)| {
        crate::ocr::tooltip_bounds(&lines)
            .ok()
            .map(|r| (lines, engine, r))
    });
    if located.is_none() && rough.is_some() {
        check()?;
        source = image;
        detailed_scout = false;
        let (lines, engine) = match native_ocr(image, check) {
            Ok(lines) if crate::ocr::tooltip_bounds(&lines).is_ok() => (lines, "Windows OCR"),
            _ => (fallback_pass(image, check, true, 2)?, "Local OCR"),
        };
        let r = crate::ocr::tooltip_bounds(&lines)?;
        located = Some((lines, engine, r));
    }
    let (lines,mut engine,r)=located.ok_or("No unambiguous item tooltip found. Hover the item, keep Alt held until its details appear, then try Ctrl+D again (English game text).")?;
    let x = r[0].min((source.width - 1) as f64);
    let y = r[1].min((source.height - 1) as f64);
    let region = [
        x / source.width as f64 * 100.,
        y / source.height as f64 * 100.,
        r[2].min(source.width as f64 - x) / source.width as f64 * 100.,
        r[3].min(source.height as f64 - y) / source.height as f64 * 100.,
    ];
    let focused = source.crop(region);
    eprintln!("ocr automatic tooltip={}x{}", focused.width, focused.height);
    let translated = || {
        lines
            .iter()
            .cloned()
            .map(|mut l| {
                l.x -= x;
                l.y -= y;
                l
            })
            .collect()
    };
    let refined = if engine == "Local OCR" && detailed_scout {
        // The enlarged rare-header hint already contains a detailed reading.
        Ok(translated())
    } else if engine == "Windows OCR" {
        native_ocr(&focused, check)
    } else {
        fallback(&focused, check)
    };
    check()?;
    let mut refined = refined.unwrap_or_else(|_| translated());
    // The small model reads the isolated tooltip, not the whole desktop.
    // Scout text remains independent evidence for obscured glyphs/properties.
    let neural = match neural_pass(&focused, false, check) {
        Ok(mut neural) if !neural.is_empty() => {
            crate::ocr::merge_stats(&mut neural, refined.clone(), 3);
            let mut anchors = crate::ocr::anchors(&lines);
            for anchor in &mut anchors {
                anchor.x -= x;
                anchor.y -= y;
            }
            if let Some(base) = anchors.last() {
                neural.retain(|l| l.y > base.y + base.height);
                neural.extend(anchors);
            }
            refined = neural;
            engine = "PP-OCRv5 English (CPU)";
            true
        }
        Ok(_) => false,
        Err(error) => {
            check()?;
            eprintln!("ocr neural fallback: {error}");
            false
        }
    };
    if crate::ocr::tooltip_bounds(&refined).is_err() {
        let mut anchors = crate::ocr::anchors(&lines);
        for anchor in &mut anchors {
            anchor.x -= x;
            anchor.y -= y;
        }
        if let Some(base) = anchors.last().cloned() {
            let title = refined
                .iter()
                .find(|l| {
                    l.y < base.y
                        && base.y - l.y < 70.
                        && l.rarity == Some(crate::item::Rarity::Rare)
                        && l.width > 50.
                        && (l.x + l.width / 2. - base.x - base.width / 2.).abs() < 42.
                        && l.text
                            .chars()
                            .all(|c| c.is_alphabetic() || c.is_whitespace() || c == '\'')
                })
                .cloned();
            refined.retain(|l| l.y > base.y + base.height);
            if let Some(title) = title {
                anchors = vec![title, base];
            }
        }
        refined.extend(anchors);
    }
    // Independent colored glyphs recover stats obscured by translucent map
    // labels. Only exact catalogue stat matches are merged into the reading.
    match if neural {
        neural_pass(&focused, true, check)
    } else {
        fallback(&focused.colored(), check)
    } {
        Ok(extra) => crate::ocr::merge_stats(&mut refined, extra, 1),
        Err(_) => check()?,
    }
    if crate::ocr::needs_retry(&refined) {
        if neural {
            match fallback(&focused.colored(), check) {
                Ok(extra) => crate::ocr::merge_stats(&mut refined, extra, 4),
                Err(_) => check()?,
            }
        }
        // A separate enlarged grayscale reading recovers faded source labels,
        // gold socket lines and split properties, and can break a value tie.
        match fallback_pass(&focused, check, false, 3) {
            Ok(extra) => crate::ocr::merge_stats(&mut refined, extra, 2),
            Err(_) => check()?,
        }
    }
    check()?;
    crate::ocr::verify_defence_conflicts(&mut refined);
    crate::ocr::draft(refined, engine.into()).or_else(|_| crate::ocr::draft(lines, engine.into()))
}
pub fn capture(
    context: &Context<'_>,
    settings: &crate::capture::Settings,
    expected: Option<u32>,
) -> Result<Draft, String> {
    settings.validate()?;
    let target = Target::new(context)?;
    if expected != Some(native::id(target.window)) {
        return Err("Cloud client changed; OCR cancelled.".into());
    }
    target.alt()?;
    let rect = native::rect(target.window).ok_or("Cloud client has no visible area.")?;
    if rect.width < 100
        || rect.height < 100
        || i64::from(rect.width) * i64::from(rect.height) > 24_000_000
    {
        return Err("Unsupported capture size.".into());
    }
    let check = || {
        target.alt()?;
        if native::rect(target.window) != Some(rect) {
            return Err("Cloud window moved or resized; OCR cancelled.".into());
        }
        Ok(())
    };
    let (image, backend) = crate::capture::frame::first_usable(
        |backend| {
            // Fault injection is available only on the authenticated private desktop.
            if native::fixture_root().is_some_and(|root| {
                root.join("capture-all-blank").exists()
                    || (root.join("capture-black-accelerated").exists()
                        && crate::capture::frame::BACKENDS[..2].contains(&backend))
            }) {
                eprintln!("ocr {backend}: blank frame (private fixture)");
                return Ok(Image {
                    width: rect.width as usize,
                    height: rect.height as usize,
                    pixels: vec![0; rect.width as usize * rect.height as usize * 4],
                });
            }
            let image = match backend {
                "Windows Graphics Capture" => super::window_capture::capture(
                    windows::Win32::Foundation::HWND(target.window),
                    rect,
                    &check,
                ),
                "Desktop Duplication" => super::screen_capture::capture(rect, &check),
                "Desktop GDI" => gdi(target.window, rect, true),
                _ => gdi(target.window, rect, false),
            };
            match &image {
                Ok(i) if !crate::capture::frame::has_content(&i.pixels) => {
                    eprintln!("ocr {backend}: blank frame")
                }
                Err(error) => eprintln!("ocr {backend}: {error}"),
                _ => {}
            }
            image
        },
        |i| crate::capture::frame::has_content(&i.pixels),
        check,
    )?;
    target.alt()?;
    eprintln!("ocr capture backend={backend}");
    // Diagnostics are restricted to the authenticated private Windows fixture.
    if let Some(root) = native::fixture_root() {
        if let Ok(f) = std::fs::File::create(root.join("ocr-snapshot.ppm")) {
            let mut f = std::io::BufWriter::new(f);
            let _ = write!(f, "P6\n{} {}\n255\n", image.width, image.height);
            for p in image.pixels.as_chunks::<4>().0 {
                let _ = f.write_all(&[p[2], p[1], p[0]]);
            }
        }
    }
    // Alt can be released after this point; pixels stay in memory until review.
    eprintln!(
        "ocr snapshot={}x{} alt_held=true",
        image.width, image.height
    );
    let result = recognize(&image, &|| target.check());
    let start = Instant::now();
    while unsafe { GetAsyncKeyState(i32::from(VK_MENU)) } < 0 {
        target.check()?;
        if start.elapsed() > Duration::from_secs(10) {
            return Err("Release Alt to open OCR review.".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    target.check()?;
    result
}
