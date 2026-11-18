use gtk::prelude::*;
use std::{
    hash::{Hash, Hasher},
    io::Read,
    path::PathBuf,
};
pub fn influence(mechanic: &str) -> Option<gtk::Image> {
    let data: &[u8] = match mechanic {
        "explicit-shaper" => include_bytes!("../../assets/influence/influence-Shaper.png"),
        "explicit-elder" => include_bytes!("../../assets/influence/influence-Elder.png"),
        "explicit-crusader" => include_bytes!("../../assets/influence/influence-Crusader.png"),
        "explicit-hunter" => include_bytes!("../../assets/influence/influence-Hunter.png"),
        "explicit-redeemer" => include_bytes!("../../assets/influence/influence-Redeemer.png"),
        "explicit-warlord" => include_bytes!("../../assets/influence/influence-Warlord.png"),
        "eldritch-eater" => include_bytes!("../../assets/influence/influence-Eater.png"),
        "eldritch-exarch" => include_bytes!("../../assets/influence/influence-Exarch.png"),
        _ => return None,
    };
    let loader = gtk::gdk_pixbuf::PixbufLoader::with_type("png").ok()?;
    loader.write(data).ok()?;
    loader.close().ok()?;
    let pixbuf = loader
        .pixbuf()?
        .scale_simple(18, 18, gtk::gdk_pixbuf::InterpType::Bilinear)?;
    let image = gtk::Image::from_pixbuf(Some(&pixbuf));
    image.set_tooltip_text(Some(mechanic.strip_prefix("explicit-").unwrap_or(mechanic)));
    Some(image)
}
pub fn currency(value: &str) -> Option<gtk::gdk_pixbuf::Pixbuf> {
    let bytes: &[u8] = match value {
        "div" => include_bytes!("../../assets/currency/divine.png"),
        "chaos" => include_bytes!("../../assets/currency/chaos.png"),
        _ => return None,
    };
    let loader = gtk::gdk_pixbuf::PixbufLoader::with_type("png").ok()?;
    loader.write(bytes).ok()?;
    loader.close().ok()?;
    loader
        .pixbuf()?
        .scale_simple(22, 22, gtk::gdk_pixbuf::InterpType::Bilinear)
}
pub fn fetch(client: &reqwest::blocking::Client, url: &str) -> Result<PathBuf, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https"
        || !matches!(parsed.host_str(), Some("web.poecdn.com" | "cdn.poedb.tw"))
    {
        return Err("Unsupported image source".into());
    }
    let cache = crate::paths::cache_root().join("awakened-crab-trade/icons");
    let mut key = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut key);
    let path = cache.join(format!("{:016x}.img", key.finish()));
    if path
        .metadata()
        .is_ok_and(|m| m.len() > 0 && m.len() <= 2 * 1024 * 1024)
    {
        return Ok(path);
    }
    let request = crate::trade::fixture_endpoint("APT_RS_TEST_ECONOMY_BASE")?
        .map(|base| format!("{base}/image.png?url={}", crate::trade::encode_segment(url)))
        .unwrap_or_else(|| url.to_string());
    let response = client
        .get(request)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.is_empty() || data.len() > 2 * 1024 * 1024 {
        return Err("Invalid image size".into());
    }
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, &data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path)
}
