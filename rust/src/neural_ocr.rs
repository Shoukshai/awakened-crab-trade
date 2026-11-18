//! Local English PP-OCRv5 recognition. No Python, GPU, network or file captures.
//! One persistent CPU session, two compute threads, bounded line tensors.
use ort::{session::Session, value::Tensor};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

pub const MODEL: &str = "en_PP-OCRv5_rec_mobile.onnx";
pub fn peak_memory_mib() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status")
            .ok()?
            .lines()
            .find_map(|line| {
                line.strip_prefix("VmHWM:")
                    .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
            })
            .map(|kib| kib / 1024.)
    }
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::System::{ProcessStatus::*, Threading::GetCurrentProcess};
        let mut memory: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let size = std::mem::size_of_val(&memory) as u32;
        memory.cb = size;
        (GetProcessMemoryInfo(GetCurrentProcess(), &mut memory, size) != 0)
            .then_some(memory.PeakWorkingSetSize as f64 / 1048576.)
    }
}
const HASH: &str = "c3461add59bb4323ecba96a492ab75e06dda42467c9e3d0c18db5d1d21924be8";
const HEIGHT: usize = 48;
const MAX_WIDTH: usize = 2048;
const MAX_LINES: usize = 96;

pub struct Image<'a> {
    pub width: usize,
    pub height: usize,
    pub bgra: &'a [u8],
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Reading {
    pub text: String,
    pub confidence: f32,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}
struct Model {
    session: Session,
    dictionary: Vec<String>,
}
static MODEL_SESSION: OnceLock<Result<Mutex<Model>, String>> = OnceLock::new();

impl Model {
    fn load(root: &Path) -> Result<Self, String> {
        let started = Instant::now();
        let path = root.join("share/ocr").join(MODEL);
        if std::fs::metadata(&path)
            .map_err(|e| format!("Neural OCR model: {e}"))?
            .len()
            != 7_872_351
        {
            return Err("Neural OCR model size mismatch.".into());
        }
        let data = std::fs::read(&path).map_err(|e| format!("Neural OCR model: {e}"))?;
        if data.len() > 12_000_000 || format!("{:x}", Sha256::digest(&data)) != HASH {
            return Err("Neural OCR model checksum mismatch.".into());
        }
        let library = if cfg!(windows) {
            "onnxruntime.dll"
        } else {
            "libonnxruntime.so"
        };
        let library = root.join(library);
        if !library.is_file() {
            return Err("Neural OCR runtime is missing. Extract the complete package.".into());
        }
        // ort loads the DLL directly, with no Python interpreter or subprocess.
        // Disable telemetry and idle spinning; do not occupy cloud video threads.
        ort::init_from(library.display().to_string())
            .with_telemetry(false)
            .commit()
            .map_err(|e| e.to_string())?;
        let session = Session::builder()
            .map_err(|e| e.to_string())?
            .with_intra_threads(2)
            .map_err(|e| e.to_string())?
            .with_inter_threads(1)
            .map_err(|e| e.to_string())?
            .with_parallel_execution(false)
            .map_err(|e| e.to_string())?
            .with_intra_op_spinning(false)
            .map_err(|e| e.to_string())?
            .with_inter_op_spinning(false)
            .map_err(|e| e.to_string())?
            .with_memory_pattern(false)
            .map_err(|e| e.to_string())?
            .commit_from_memory(&data)
            .map_err(|e| e.to_string())?;
        let chars = session
            .metadata()
            .map_err(|e| e.to_string())?
            .custom("character")
            .map_err(|e| e.to_string())?
            .ok_or("Neural OCR dictionary is missing.")?;
        let mut dictionary = vec![String::new()]; // CTC blank
        dictionary.extend(chars.lines().map(str::to_owned));
        dictionary.push(" ".into()); // Paddle's use_space_char
        eprintln!(
            "ocr neural model=PP-OCRv5 runtime=ONNX threads=2 load_ms={}",
            started.elapsed().as_millis()
        );
        Ok(Self {
            session,
            dictionary,
        })
    }
    fn read(
        &mut self,
        image: &Image<'_>,
        r: [usize; 4],
        colored: bool,
    ) -> Result<(String, f32), String> {
        let [x, y, w, h] = r;
        let width = ((w * HEIGHT).div_ceil(h)).clamp(8, MAX_WIDTH).div_ceil(8) * 8;
        let mut data = vec![0f32; 3 * HEIGHT * width];
        // Bilinear, aspect preserving line resize. All channels are identical
        // after max-channel grayscale, avoiding BGR/RGB layout ambiguities.
        for yy in 0..HEIGHT {
            let fy = ((yy as f32 + 0.5) * h as f32 / HEIGHT as f32 - 0.5).max(0.);
            let y0 = (fy as usize).min(h - 1);
            let y1 = (y0 + 1).min(h - 1);
            for xx in 0..width {
                let fx = ((xx as f32 + 0.5) * w as f32 / width as f32 - 0.5).max(0.);
                let x0 = (fx as usize).min(w - 1);
                let x1 = (x0 + 1).min(w - 1);
                let pixel = |dx, dy| brightness(image, x + dx, y + dy, colored) as f32;
                let a = pixel(x0, y0) * (1. - fx.fract()) + pixel(x1, y0) * fx.fract();
                let b = pixel(x0, y1) * (1. - fx.fract()) + pixel(x1, y1) * fx.fract();
                let v = 1. - (a * (1. - fy.fract()) + b * fy.fract()) / 127.5;
                for c in 0..3 {
                    data[c * HEIGHT * width + yy * width + xx] = v;
                }
            }
        }
        let input =
            Tensor::from_array(([1usize, 3, HEIGHT, width], data)).map_err(|e| e.to_string())?;
        let result = self
            .session
            .run(ort::inputs![input])
            .map_err(|e| e.to_string())?;
        let (shape, scores) = result[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        if shape.len() != 3 || shape[0] != 1 || shape[2] as usize != self.dictionary.len() {
            return Err("Unexpected neural OCR output shape.".into());
        }
        ctc(scores, self.dictionary.len(), &self.dictionary)
    }
}

fn brightness(image: &Image<'_>, x: usize, y: usize, colored: bool) -> u8 {
    let p = &image.bgra[(y * image.width + x) * 4..];
    let (b, g, r) = (p[0] as i32, p[1] as i32, p[2] as i32);
    if colored && !(b > 100 && b - r > 18 && b - g > 12) {
        0
    } else {
        p[0].max(p[1]).max(p[2])
    }
}
fn glyph(image: &Image<'_>, x: usize, y: usize, colored: bool) -> bool {
    let p = &image.bgra[(y * image.width + x) * 4..];
    let (b, g, r) = (p[0] as i32, p[1] as i32, p[2] as i32);
    let blue = b > 100 && b - r > 18 && b - g > 12;
    let neutral = r.min(g).min(b) > 95 && r.max(g).max(b) - r.min(g).min(b) < 40;
    let gold = r > 185 && g > 165 && b < 155 && r - g < 40;
    let red = r > 150 && r > g * 2 && r > b * 2;
    if !blue && (colored || (!neutral && !gold && !red)) {
        return false;
    }
    let v = brightness(image, x, y, colored);
    if v < if colored { 105 } else { 100 } {
        return false;
    }
    // Ignore flat bright panels/frames; retain glyphs with a nearby dark edge.
    [(-2isize, 0isize), (2, 0), (0, -2), (0, 2)]
        .iter()
        .any(|&(dx, dy)| {
            let xx = x.saturating_add_signed(dx).min(image.width - 1);
            let yy = y.saturating_add_signed(dy).min(image.height - 1);
            v.saturating_sub(brightness(image, xx, yy, colored)) > 35
        })
}
fn regions(image: &Image<'_>, colored: bool) -> Result<Vec<[usize; 4]>, String> {
    let mut out = Vec::new();
    let mut start = None;
    let mut last = 0;
    let threshold = (image.width / 80).max(6);
    let mut left = image.width;
    let mut right = 0;
    for y in 0..=image.height + 2 {
        let xs: Vec<_> = if y < image.height {
            (0..image.width)
                .filter(|&x| glyph(image, x, y, colored))
                .collect()
        } else {
            Vec::new()
        };
        if xs.len() >= threshold && xs.len() < image.width * 4 / 5 {
            start.get_or_insert(y);
            last = y;
            left = left.min(xs[0]);
            right = right.max(*xs.last().unwrap());
        } else if start.is_some() && y > last + 1 {
            let first = start.take().unwrap();
            let height = last - first + 1;
            if (4..=50).contains(&height) && right > left + 12 {
                let x = left.saturating_sub(3);
                let yy = first.saturating_sub(2);
                out.push([
                    x,
                    yy,
                    (right + 4).min(image.width) - x,
                    (last + 3).min(image.height) - yy,
                ]);
                if out.len() > MAX_LINES {
                    return Err("Too many OCR lines in the tooltip.".into());
                }
            }
            left = image.width;
            right = 0;
        }
    }
    Ok(out)
}

fn ctc(scores: &[f32], classes: usize, dictionary: &[String]) -> Result<(String, f32), String> {
    if classes == 0 || classes != dictionary.len() || !scores.len().is_multiple_of(classes) {
        return Err("Invalid CTC output.".into());
    }
    let mut text = String::new();
    let mut previous = 0;
    let (mut total, mut count) = (0., 0usize);
    for step in scores.chunks_exact(classes) {
        if step.iter().any(|v| !v.is_finite()) {
            return Err("Non-finite OCR scores.".into());
        }
        let (best, &confidence) = step
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        if best != 0 && best != previous {
            text.push_str(&dictionary[best]);
            total += confidence;
            count += 1;
        }
        previous = best;
    }
    Ok((
        text.trim().into(),
        if count == 0 { 0. } else { total / count as f32 },
    ))
}

pub fn recognize(
    root: &Path,
    image: Image<'_>,
    colored: bool,
    check: &impl Fn() -> Result<(), String>,
) -> Result<Vec<Reading>, String> {
    recognize_seeded(root, image, colored, &[], check)
}

/// Positioned scout lines supplement projection when translucent backgrounds
/// join adjacent baselines. Coordinates always come from automatic OCR.
pub fn recognize_seeded(
    root: &Path,
    image: Image<'_>,
    colored: bool,
    hints: &[[usize; 4]],
    check: &impl Fn() -> Result<(), String>,
) -> Result<Vec<Reading>, String> {
    check()?;
    if image.width < 2
        || image.height < 2
        || image.width > 8192
        || image.height > 8192
        || image.width * image.height > 24_000_000
        || image.bgra.len() != image.width * image.height * 4
    {
        return Err("Invalid neural OCR image.".into());
    }
    let started = Instant::now();
    let model = MODEL_SESSION
        .get_or_init(|| {
            std::panic::catch_unwind(|| Model::load(root))
                .unwrap_or_else(|_| Err("Neural OCR runtime could not be loaded.".into()))
                .map(Mutex::new)
        })
        .as_ref()
        .map_err(Clone::clone)?;
    let mut model = model
        .lock()
        .map_err(|_| "Neural OCR session unavailable.")?;
    check()?;
    let mut boxes = if hints.is_empty() {
        regions(&image, colored)?
    } else {
        hinted_regions(&image, hints, colored)?
    };
    boxes.sort_by_key(|r| r[1]);
    if boxes.is_empty() {
        return Err("No text lines found by neural segmentation.".into());
    }
    let region_count = boxes.len();
    let mut out = Vec::new();
    for r in boxes {
        check()?;
        if started.elapsed() > Duration::from_secs(15) {
            return Err("Neural OCR timed out.".into());
        }
        let (text, confidence) = model.read(&image, r, colored)?;
        check()?;
        if !text.is_empty() {
            out.push(Reading {
                text,
                confidence,
                x: r[0],
                y: r[1],
                width: r[2],
                height: r[3],
            });
        }
    }
    eprintln!(
        "ocr neural colored={colored} hints={} regions={region_count} lines={} elapsed_ms={}",
        hints.len(),
        out.len(),
        started.elapsed().as_millis()
    );
    Ok(out)
}

fn hinted_regions(
    image: &Image<'_>,
    hints: &[[usize; 4]],
    colored: bool,
) -> Result<Vec<[usize; 4]>, String> {
    if hints.len() > MAX_LINES {
        return Err("Too many automatic OCR line hints.".into());
    }
    let mut boxes: Vec<[usize; 4]> = Vec::new();
    for &[x, y, w, h] in hints {
        if w == 0 || h == 0 || x >= image.width || y >= image.height {
            continue;
        }
        let left = x.saturating_sub(3);
        let top = y.saturating_sub(2);
        let right = x.saturating_add(w).saturating_add(3).min(image.width);
        let bottom = y.saturating_add(h).saturating_add(2).min(image.height);
        if bottom - top > 64 {
            continue;
        }
        let (left, top, right, bottom) = if colored {
            // A scout row can include neighboring gold UI text. Masked pixels
            // must not shift the centered tooltip row out of its text column.
            let mut bounds = [right, bottom, left, top];
            for yy in top..bottom {
                let xs: Vec<_> = (left..right)
                    .filter(|&xx| brightness(image, xx, yy, true) > 105)
                    .collect();
                if xs.len() >= 6 {
                    bounds[0] = bounds[0].min(xs[0]);
                    bounds[1] = bounds[1].min(yy);
                    bounds[2] = bounds[2].max(xs[xs.len() - 1] + 1);
                    bounds[3] = bounds[3].max(yy + 1);
                }
            }
            if bounds[2] <= bounds[0] || bounds[3] <= bounds[1] {
                continue;
            }
            (
                bounds[0].saturating_sub(3).max(left),
                bounds[1].saturating_sub(2).max(top),
                (bounds[2] + 3).min(right),
                (bounds[3] + 2).min(bottom),
            )
        } else {
            (left, top, right, bottom)
        };
        let r = [left, top, right - left, bottom - top];
        if let Some(old) = boxes.iter_mut().find(|b| {
            let overlap = (b[1] + b[3]).min(bottom).saturating_sub(b[1].max(top));
            overlap * 2 > b[3].min(r[3])
                && (b[1] as f64 + b[3] as f64 / 2. - top as f64 - r[3] as f64 / 2.).abs()
                    < b[3].max(r[3]) as f64 * 0.6
        }) {
            let x2 = (old[0] + old[2]).max(right);
            let y2 = (old[1] + old[3]).max(bottom);
            old[0] = old[0].min(left);
            old[1] = old[1].min(top);
            old[2] = x2 - old[0];
            old[3] = y2 - old[1];
        } else {
            boxes.push(r);
        }
    }
    Ok(boxes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scout_hints_retain_lines_on_a_flat_background_and_clip_outside_boxes() {
        let pixels = vec![200u8; 100 * 80 * 4];
        let image = Image {
            width: 100,
            height: 80,
            bgra: &pixels,
        };
        assert!(regions(&image, false).unwrap().is_empty());
        let boxes = hinted_regions(
            &image,
            &[[4, 10, 80, 12], [8, 35, 120, 12], [120, 20, 9, 9]],
            false,
        )
        .unwrap();
        assert_eq!(boxes.len(), 2);
        assert!(boxes
            .iter()
            .all(|r| r[0] + r[2] <= 100 && r[1] + r[3] <= 80));
        assert!(hinted_regions(&image, &[[4, 10, 80, 12]], true)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn colored_hint_bounds_ignore_masked_neighboring_text() {
        let mut pixels = vec![0u8; 200 * 60 * 4];
        for y in 20..32 {
            for x in 100..180 {
                let p = &mut pixels[(y * 200 + x) * 4..][..4];
                p.copy_from_slice(&[240, 170, 140, 255]);
            }
            for x in 5..70 {
                pixels[(y * 200 + x) * 4..][..4].copy_from_slice(&[90, 170, 200, 255]);
            }
        }
        let image = Image {
            width: 200,
            height: 60,
            bgra: &pixels,
        };
        let boxes = hinted_regions(&image, &[[0, 18, 190, 18]], true).unwrap();
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0][0], 97);
        assert_eq!(boxes[0][2], 86);
    }
    #[test]
    fn ctc_keeps_repeated_digits_separated_by_blank() {
        let dict = vec!["".into(), "1".into(), "4".into()];
        let (text, _) = ctc(
            &[
                0., 0., 1., 0., 0., 1., 1., 0., 0., 0., 1., 0., 1., 0., 0., 0., 1., 0.,
            ],
            3,
            &dict,
        )
        .unwrap();
        assert_eq!(text, "411");
        assert!(ctc(&[f32::NAN, 0., 0.], 3, &dict).is_err());
        assert!(ctc(&[1., 0.], 3, &dict).is_err());
    }
    #[test]
    fn cancellation_precedes_model_loading() {
        assert_eq!(
            recognize(
                Path::new("missing"),
                Image {
                    width: 0,
                    height: 0,
                    bgra: &[]
                },
                false,
                &|| Err("cancelled".into())
            )
            .unwrap_err(),
            "cancelled"
        );
    }
}
