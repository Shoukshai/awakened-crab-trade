//! Local, immutable uploads, file IO and initial decoding off the GTK thread.
//! Animated results retain bytes for GTK's native animation iterator.
use gtk::{
    gdk_pixbuf::{Colorspace, Pixbuf, PixbufLoader},
    prelude::*,
};
use std::{
    cell::Cell,
    fs,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};
pub const MAX_BYTES: usize = 16_000_000;
pub const MAX_PIXELS: i64 = 48_000_000;
#[derive(Debug, Clone)]
pub struct Decoded {
    pub pixels: Arc<[u8]>,
    pub width: i32,
    pub height: i32,
    pub rowstride: i32,
    pub alpha: bool,
    pub animation: Option<Arc<[u8]>>,
}
impl Decoded {
    pub fn pixbuf(&self) -> Pixbuf {
        Pixbuf::from_bytes(
            &glib::Bytes::from_owned(self.pixels.clone()),
            Colorspace::Rgb,
            self.alpha,
            8,
            self.width,
            self.height,
            self.rowstride,
        )
    }
}
pub fn directory() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("awakened-poe-trade-rs/images")
}
fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let f = fs::File::open(path).map_err(|e| format!("Lecture de l’image : {e}"))?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Choisissez un fichier image.".into());
    }
    let mut bytes = Vec::new();
    f.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Image limitée à 16 Mo.".into());
    }
    Ok(bytes)
}
pub fn read(url: &str) -> Result<Vec<u8>, String> {
    if !crate::image_strip::valid_url(url) {
        return Err("Référence d’image locale invalide.".into());
    }
    if url == "syndicate.jpg" {
        return Ok(include_bytes!("../../../renderer/public/images/syndicate.jpg").to_vec());
    }
    let bytes = read_bounded(&directory().join(url))?;
    let hash = glib::compute_checksum_for_data(glib::ChecksumType::Sha256, &bytes)
        .ok_or("Empreinte d’image indisponible.")?;
    if !url.starts_with(&format!("{hash}.")) {
        return Err("Le fichier d’image local est endommagé. Réimportez l’original.".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], width: i32, height: i32, cover: bool) -> Result<Decoded, String> {
    if bytes.len() > MAX_BYTES || !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
        return Err("Dimensions d’image demandées hors limites.".into());
    }
    let loader = PixbufLoader::new();
    let invalid = Rc::new(Cell::new(false));
    let checked = invalid.clone();
    loader.connect_size_prepared(move |loader, w, h| {
        if w <= 0 || h <= 0 || i64::from(w) * i64::from(h) > MAX_PIXELS {
            checked.set(true);
            loader.set_size(1, 1);
            return;
        }
        let sx = f64::from(width) / f64::from(w);
        let sy = f64::from(height) / f64::from(h);
        let factor = (if cover { sx.max(sy) } else { sx.min(sy) }).min(1.);
        let nw = (f64::from(w) * factor).ceil().max(1.) as i32;
        let nh = (f64::from(h) * factor).ceil().max(1.) as i32;
        // Extreme aspect ratios are cropped only after a bounded decode.
        let limit = (4096. / f64::from(nw.max(nh))).min(1.);
        loader.set_size(
            (f64::from(nw) * limit).round().max(1.) as i32,
            (f64::from(nh) * limit).round().max(1.) as i32,
        );
    });
    let result = loader.write(bytes);
    let close = loader.close();
    if invalid.get() {
        return Err("Image limitée à 48 millions de pixels.".into());
    }
    result.map_err(|e| format!("Image illisible : {e}"))?;
    close.map_err(|e| format!("Image incomplète : {e}"))?;
    let pixbuf = loader.pixbuf().ok_or("Image sans pixels décodables.")?;
    if pixbuf.width() > 4096 || pixbuf.height() > 4096 {
        return Err("Le décodeur n’a pas respecté la taille demandée.".into());
    }
    let animated = loader.animation().is_some_and(|a| !a.is_static_image());
    Ok(Decoded {
        pixels: Arc::from(pixbuf.read_pixel_bytes().as_ref()),
        width: pixbuf.width(),
        height: pixbuf.height(),
        rowstride: pixbuf.rowstride(),
        alpha: pixbuf.has_alpha(),
        animation: animated.then(|| Arc::from(bytes)),
    })
}
pub fn load(url: &str, width: i32, height: i32, cover: bool) -> Result<Decoded, String> {
    decode(&read(url)?, width, height, cover)
}
pub fn import(path: &Path) -> Result<String, String> {
    import_into(path, &directory())
}
fn import_into(path: &Path, dir: &Path) -> Result<String, String> {
    let bytes = read_bounded(path)?;
    decode(&bytes, 192, 104, true)?;
    let hash = glib::compute_checksum_for_data(glib::ChecksumType::Sha256, &bytes)
        .ok_or("Empreinte d’image indisponible.")?;
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .filter(|x| !x.is_empty() && x.len() <= 8 && x.bytes().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("img")
        .to_ascii_lowercase();
    let name = format!("{hash}.{ext}");
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if !read_bounded(&dir.join(&name)).is_ok_and(|old| old == bytes) {
        use std::os::unix::fs::OpenOptionsExt;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let temporary = dir.join(format!(".import-{}-{nonce}", std::process::id()));
        let mut created = false;
        let result = (|| {
            use std::io::Write;
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            created = true;
            f.write_all(&bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temporary, dir.join(&name)).map_err(|e| e.to_string())
        })();
        if result.is_err() && created {
            let _ = fs::remove_file(temporary);
        }
        result?;
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PIXEL: &[u8] = include_bytes!("../fixtures/image-import.png");
    #[test]
    fn decode_rejects_invalid_bytes_and_request_bounds_without_a_window() {
        assert!(decode(b"not an image", 192, 104, true).is_err());
        assert!(decode(PIXEL, 0, 104, true).is_err());
        let data = decode(PIXEL, 192, 104, true).unwrap();
        assert_eq!((data.width, data.height), (8, 8));
        assert!(data.animation.is_none());
    }
    #[test]
    fn import_is_deduplicated_repairs_a_corrupt_cache_and_preserves_source_on_failure() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("apt-image-import-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let source = root.join("source.PNG");
        fs::write(&source, PIXEL).unwrap();
        let cache = root.join("cache");
        let name = import_into(&source, &cache).unwrap();
        assert!(crate::image_strip::valid_url(&name));
        assert_eq!(import_into(&source, &cache).unwrap(), name);
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
        fs::write(cache.join(&name), b"corrupt").unwrap();
        assert_eq!(import_into(&source, &cache).unwrap(), name);
        assert_eq!(fs::read(cache.join(name)).unwrap(), PIXEL);
        let impossible = root.join("file-not-directory");
        fs::write(&impossible, b"keep").unwrap();
        assert!(import_into(&source, &impossible).is_err());
        assert_eq!(fs::read(&source).unwrap(), PIXEL);
        assert_eq!(fs::read(impossible).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }
}
