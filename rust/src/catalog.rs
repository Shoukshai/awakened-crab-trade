//! Versioned offline reference library. All disk/network refresh runs off GTK.
//! Curated bundled definitions remain authoritative; Trade adds missing entries.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, OnceLock, RwLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LIMIT: u64 = 32 * 1024 * 1024;
const DAY: u64 = 24 * 60 * 60;
static REVISION: AtomicU64 = AtomicU64::new(1);
static DATA: OnceLock<RwLock<Arc<Snapshot>>> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);
static AUTOMATIC: AtomicBool = AtomicBool::new(false);

pub struct Snapshot {
    pub revision: u64,
    pub items: Vec<Value>,
    pub stats: Vec<Value>,
}
/// Parsed consumers rebuild once per published generation, never per OCR line.
pub struct Index<T> {
    inner: RwLock<Option<(u64, Arc<T>)>>,
}
impl<T> Index<T> {
    pub const fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }
    pub fn get(&self, build: impl FnOnce(&Snapshot) -> T) -> Arc<T> {
        let snapshot = current();
        if let Some((revision, value)) = self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|(r, _)| *r == snapshot.revision)
        {
            let _ = revision;
            return value.clone();
        }
        let value = Arc::new(build(&snapshot));
        let mut index = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if index
            .as_ref()
            .is_none_or(|(revision, _)| *revision <= snapshot.revision)
        {
            *index = Some((snapshot.revision, value.clone()));
        }
        value
    }
}
impl Snapshot {
    fn new(items: Vec<Value>, stats: Vec<Value>) -> Self {
        Self {
            revision: REVISION.fetch_add(1, Ordering::Relaxed),
            items,
            stats,
        }
    }
}
fn bundled() -> Snapshot {
    let decode = |a: &str, b: &str| {
        a.lines()
            .chain(b.lines())
            .map(|s| serde_json::from_str(s).expect("bundled catalogue"))
            .collect()
    };
    Snapshot::new(
        decode(
            include_str!("data/base-items.ndjson"),
            include_str!("data/current-items.ndjson"),
        ),
        decode(
            include_str!("data/base-stats.ndjson"),
            include_str!("data/current-stats.ndjson"),
        ),
    )
}
pub fn current() -> Arc<Snapshot> {
    DATA.get_or_init(|| RwLock::new(Arc::new(bundled())))
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
fn publish(snapshot: Snapshot) {
    let _ = current();
    *DATA
        .get()
        .unwrap()
        .write()
        .unwrap_or_else(|e| e.into_inner()) = Arc::new(snapshot);
}
fn root() -> PathBuf {
    crate::paths::cache_root()
        .join(crate::edition::id())
        .join("catalog/v1")
        .join(env!("CARGO_PKG_VERSION"))
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Catalogue exceeds size limit".into());
    }
    Ok(bytes)
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    app_version: String,
    generation: String,
    checked_at: u64,
    items_sha256: String,
    mods_sha256: String,
}
fn valid_text(s: &str, multiline: bool) -> bool {
    !s.trim().is_empty()
        && s.len() <= 8192
        && !s
            .chars()
            .any(|c| c.is_control() && !(multiline && c == '\n'))
}
fn item_key(v: &Value) -> String {
    format!(
        "{}\n{}\n{}",
        v["namespace"].as_str().unwrap_or_default(),
        v["name"].as_str().unwrap_or_default(),
        v["unique"]["base"].as_str().unwrap_or_default()
    )
}
fn validate(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.items.len() < 1000 || snapshot.stats.len() < 1000 {
        return Err("Incomplete catalogue".into());
    }
    for item in &snapshot.items {
        if !item["name"].as_str().is_some_and(|s| valid_text(s, false))
            || item["namespace"].as_str().is_none()
            || item["refName"].as_str().is_none()
        {
            return Err("Invalid item catalogue record".into());
        }
    }
    for row in &snapshot.stats {
        for stat in row
            .get("stats")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(std::slice::from_ref(row))
        {
            if !stat["ref"].as_str().is_some_and(|s| valid_text(s, true))
                || stat["trade"]["ids"].as_object().is_none()
                || stat["matchers"].as_array().is_none_or(|a| {
                    a.is_empty()
                        || a.iter()
                            .any(|m| !m["string"].as_str().is_some_and(|s| valid_text(s, true)))
                })
            {
                return Err("Invalid modifier catalogue record".into());
            }
        }
    }
    Ok(())
}
fn load(root: &Path) -> Result<(Snapshot, u64), String> {
    let manifest: Manifest =
        serde_json::from_slice(&read(&root.join("active.json"))?).map_err(|e| e.to_string())?;
    if manifest.format != 1
        || manifest.app_version != env!("CARGO_PKG_VERSION")
        || !manifest.generation.starts_with("g-")
        || !manifest
            .generation
            .bytes()
            .all(|c| c.is_ascii_digit() || c == b'g' || c == b'-')
    {
        return Err("Invalid catalogue manifest".into());
    }
    let dir = root.join(&manifest.generation);
    let items = read(&dir.join("items.json"))?;
    let mods = read(&dir.join("mods.json"))?;
    if digest(&items) != manifest.items_sha256 || digest(&mods) != manifest.mods_sha256 {
        return Err("Catalogue checksum mismatch".into());
    }
    let snapshot = Snapshot::new(
        serde_json::from_slice(&items).map_err(|e| e.to_string())?,
        serde_json::from_slice(&mods).map_err(|e| e.to_string())?,
    );
    validate(&snapshot)?;
    Ok((snapshot, manifest.checked_at))
}
fn save(root: &Path, snapshot: &Snapshot, checked_at: u64) -> Result<(), String> {
    validate(snapshot)?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let generation = format!(
        "g-{}-{}-{}",
        now(),
        std::process::id(),
        REVISION.fetch_add(1, Ordering::Relaxed)
    );
    let dir = root.join(&generation);
    std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
    let items = serde_json::to_vec(&snapshot.items).map_err(|e| e.to_string())?;
    let mods = serde_json::to_vec(&snapshot.stats).map_err(|e| e.to_string())?;
    for (name, bytes) in [("items.json", &items), ("mods.json", &mods)] {
        use std::io::Write;
        let mut file = std::fs::File::create(dir.join(name)).map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    let manifest = Manifest {
        format: 1,
        app_version: env!("CARGO_PKG_VERSION").into(),
        generation,
        checked_at,
        items_sha256: digest(&items),
        mods_sha256: digest(&mods),
    };
    // Only this final small rename makes both checked files visible together.
    let tmp = root.join(format!("active-{}.tmp", std::process::id()));
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, root.join("active.json")).map_err(|e| e.to_string())?;
    // Keep one previous generation; leave recent, potentially active writers alone.
    let mut old: Vec<_> = std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_ok_and(|t| t.is_dir())
                && e.file_name().to_string_lossy().starts_with("g-")
                && e.file_name() != manifest.generation.as_str()
        })
        .collect();
    old.sort_by_key(|e| {
        std::cmp::Reverse(
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH),
        )
    });
    for entry in old.into_iter().skip(1) {
        if entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > DAY)
        {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}
fn download(client: &reqwest::blocking::Client, url: &str) -> Result<Value, String> {
    let response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > LIMIT) {
        return Err("Trade catalogue exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    response
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Trade catalogue exceeds size limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn merge_trade(base: &Snapshot, mods: &Value, items: &Value) -> Result<Snapshot, String> {
    let groups = mods["result"]
        .as_array()
        .ok_or("Missing Trade modifier groups")?;
    let mut rows = BTreeMap::<(String, bool), Value>::new();
    let mut count = 0;
    for group in groups {
        for e in group["entries"]
            .as_array()
            .ok_or("Missing Trade modifier entries")?
        {
            let text = e["text"].as_str().ok_or("Missing Trade wording")?;
            let kind = e["type"].as_str().ok_or("Missing Trade modifier type")?;
            let id = e["id"].as_str().ok_or("Missing Trade modifier ID")?;
            if !valid_text(text, true)
                || !id.starts_with(&format!("{kind}."))
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._|-".contains(&b))
            {
                return Err("Invalid Trade modifier".into());
            }
            let options = e["option"]["options"].as_array().filter(|a| !a.is_empty());
            let row=rows.entry((text.into(),options.is_some())).or_insert_with(||json!({"ref":text,"better":0,"matchers":[],"trade":{"ids":{},"option":options.is_some()}}));
            let ids = row["trade"]["ids"]
                .as_object_mut()
                .unwrap()
                .entry(kind.to_string())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap();
            if !ids.contains(&json!(id)) {
                ids.push(json!(id));
            }
            let matchers: Vec<Value> = if let Some(options) = options {
                options.iter().filter_map(|o|Some(json!({"string":text.replacen('#',o["text"].as_str()?,1).replace("+#","#"),"value":o["id"].as_i64().or_else(||o["id"].as_str()?.parse::<i64>().ok())?}))).collect()
            } else {
                vec![json!({"string":text.replace("+#","#")})]
            };
            let target = row["matchers"].as_array_mut().unwrap();
            for m in matchers {
                if !target.contains(&m) {
                    target.push(m);
                }
            }
            count += 1;
        }
    }
    if count < 1000 {
        return Err("Incomplete Trade modifier response".into());
    }
    let mut item_rows = base.items.clone();
    let mut seen: HashSet<_> = item_rows.iter().map(item_key).collect();
    let mut item_count = 0;
    for group in items["result"]
        .as_array()
        .ok_or("Missing Trade item groups")?
    {
        let group = group.as_object().ok_or("Invalid Trade item group")?;
        let family = group
            .get("id")
            .and_then(Value::as_str)
            .ok_or("Missing Trade item group ID")?;
        for e in group
            .get("entries")
            .and_then(Value::as_array)
            .ok_or("Missing Trade item entries")?
        {
            let typ = e["type"].as_str().ok_or("Missing Trade item base")?;
            let name = e["name"].as_str().unwrap_or(typ);
            if !valid_text(name, false) || !valid_text(typ, false) {
                return Err("Invalid Trade item name".into());
            }
            let unique = e.get("name").is_some() || e["flags"]["unique"].as_bool() == Some(true);
            let ns = if unique {
                "UNIQUE"
            } else {
                match family {
                    "gem" => "GEM",
                    "card" => "DIVINATION_CARD",
                    "monster" => "CAPTURED_BEAST",
                    _ => "ITEM",
                }
            };
            let mut row = json!({"name":name,"refName":name,"namespace":ns});
            if unique {
                row["unique"] = json!({"base":typ});
            }
            if let Some(disc) = e["disc"].as_str() {
                row["tradeDisc"] = json!(disc);
            }
            let known = base
                .items
                .iter()
                .find(|v| v["namespace"] == "ITEM" && v["name"] == typ);
            let category = known
                .and_then(|v| {
                    v["category"]
                        .as_str()
                        .or_else(|| v["craftable"]["category"].as_str())
                })
                .or(match family {
                    "gem" => Some("Gem"),
                    "currency" => Some("Currency"),
                    "card" => Some("Divination Card"),
                    "map" => Some("Map"),
                    "viscera" => Some("Viscera"),
                    "wombgift" => Some("Wombgift"),
                    _ => None,
                });
            if let Some(category) = category {
                row["category"] = json!(category);
            }
            if family == "currency" {
                row["exchangeable"] = json!(true);
            }
            if !unique {
                if let Some(known) = known {
                    for key in ["craftable", "armour", "visceraSlot", "gem"] {
                        if let Some(v) = known.get(key) {
                            row[key] = v.clone();
                        }
                    }
                }
            }
            if seen.insert(item_key(&row)) {
                item_rows.push(row);
            }
            item_count += 1;
        }
    }
    if item_count < 500 {
        return Err("Incomplete Trade item response".into());
    }
    let mut covered = HashSet::new();
    for row in &base.stats {
        for stat in row
            .get("stats")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(std::slice::from_ref(row))
        {
            if let (Some(matchers), Some(kinds)) = (
                stat["matchers"].as_array(),
                stat["trade"]["ids"].as_object(),
            ) {
                for m in matchers {
                    if let Some(text) = m["string"].as_str() {
                        for (kind, ids) in kinds {
                            for id in ids
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                            {
                                covered.insert((text.to_string(), kind.clone(), id.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut stat_rows = base.stats.clone();
    for row in rows.into_values() {
        let Some(matchers) = row["matchers"].as_array().filter(|a| !a.is_empty()) else {
            continue;
        };
        let needed = matchers.iter().any(|m| {
            row["trade"]["ids"]
                .as_object()
                .unwrap()
                .iter()
                .any(|(kind, ids)| {
                    ids.as_array().unwrap().iter().any(|id| {
                        !covered.contains(&(
                            m["string"].as_str().unwrap().to_string(),
                            kind.clone(),
                            id.as_str().unwrap().to_string(),
                        ))
                    })
                })
        });
        if needed {
            stat_rows.push(row);
        }
    }
    let snapshot = Snapshot::new(item_rows, stat_rows);
    validate(&snapshot)?;
    Ok(snapshot)
}
pub fn start_background(update: bool) {
    AUTOMATIC.store(update, Ordering::Release);
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let result = std::thread::Builder::new()
        .name("reference-catalogue".into())
        .spawn(move || {
            if let Err(e) = initialize(&root(), update) {
                eprintln!("catalogue refresh failed; keeping offline references: {e}");
            }
            STARTED.store(false, Ordering::Release);
        });
    if let Err(e) = result {
        STARTED.store(false, Ordering::Release);
        eprintln!("catalogue worker: {e}");
    }
}
pub fn prepare(update: bool) -> Result<Value, String> {
    AUTOMATIC.store(update, Ordering::Release);
    initialize(&root(), update)?;
    let snapshot = current();
    Ok(
        json!({"directory":root(),"items":snapshot.items.len(),"modifier_records":snapshot.stats.len(),"revision":snapshot.revision,"edition":crate::edition::id()}),
    )
}
fn initialize(root: &Path, update: bool) -> Result<(), String> {
    let checked = match load(root) {
        Ok((data, checked)) => {
            publish(data);
            checked
        }
        Err(error) => {
            if root.join("active.json").exists() {
                eprintln!("catalogue cache rejected; rebuilding offline references: {error}");
            }
            let data = bundled();
            save(root, &data, 0)?;
            publish(data);
            0
        }
    };
    let data = current();
    eprintln!(
        "catalogue ready items={} mods={} directory={} offline=true",
        data.items.len(),
        data.stats.len(),
        root.display()
    );
    drop(data);
    let time = now();
    if !update
        || !AUTOMATIC.load(Ordering::Acquire)
        || (checked <= time && time.saturating_sub(checked) < DAY)
    {
        return Ok(());
    }
    let attempt = root.join("last-attempt.txt");
    if std::fs::read_to_string(&attempt)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|t| t <= time && time - t < 3600)
    {
        return Ok(());
    }
    std::fs::write(attempt, time.to_string()).map_err(|e| e.to_string())?;
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("AwakenedCrabTrade/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mods = download(&client, "https://www.pathofexile.com/api/trade/data/stats")?;
    if !AUTOMATIC.load(Ordering::Acquire) {
        return Ok(());
    }
    let items = download(&client, "https://www.pathofexile.com/api/trade/data/items")?;
    if !AUTOMATIC.load(Ordering::Acquire) {
        return Ok(());
    }
    let snapshot = merge_trade(&bundled(), &mods, &items)?;
    save(root, &snapshot, time)?;
    eprintln!(
        "catalogue updated items={} mods={}",
        snapshot.items.len(),
        snapshot.stats.len()
    );
    publish(snapshot);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_cache_is_complete_atomic_and_rejects_corruption() {
        let dir = std::env::temp_dir().join(format!(
            "act-catalog-{}-{}",
            std::process::id(),
            REVISION.fetch_add(1, Ordering::Relaxed)
        ));
        let data = bundled();
        save(&dir, &data, 123).unwrap();
        let (loaded, time) = load(&dir).unwrap();
        assert_eq!(time, 123);
        assert_eq!(loaded.items, data.items);
        assert_eq!(loaded.stats, data.stats);
        save(&dir, &data, 456).unwrap();
        assert_eq!(load(&dir).unwrap().1, 456);
        let pointer: Manifest =
            serde_json::from_slice(&read(&dir.join("active.json")).unwrap()).unwrap();
        std::fs::write(dir.join(pointer.generation).join("mods.json"), b"[]").unwrap();
        assert!(load(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn incomplete_network_data_never_replaces_bundled_definitions() {
        assert!(merge_trade(&bundled(), &json!({"result":[]}), &json!({"result":[]})).is_err());
    }
    #[test]
    fn current_trade_additions_keep_curated_bases_options_and_source_ids() {
        let base = bundled();
        let mut stats:Vec<_>=(0..1000).map(|i|json!({"id":format!("explicit.stat_fixture_{i}"),"text":format!("# to Catalogue Fixture {i}"),"type":"explicit"})).collect();
        stats.push(json!({"id":"implicit.stat_fixture_option","text":"Allocates #","type":"implicit","option":{"options":[{"id":"23","text":"Catalogue Notable"}]}}));
        let mods = json!({"result":[{"id":"explicit","entries":stats}]});
        let mut entries: Vec<_> = (0..500)
            .map(|i| json!({"type":format!("Catalogue Currency {i}")}))
            .collect();
        entries.push(
            json!({"type":"Leather Belt","name":"Catalogue Unique Belt","flags":{"unique":true}}),
        );
        let items = json!({"result":[{"id":"currency","entries":entries}]});
        let merged = merge_trade(&base, &mods, &items).unwrap();
        assert_eq!(&merged.items[..base.items.len()], &base.items);
        assert_eq!(&merged.stats[..base.stats.len()], &base.stats);
        let unique = merged
            .items
            .iter()
            .find(|v| v["name"] == "Catalogue Unique Belt")
            .unwrap();
        assert_eq!(unique["namespace"], "UNIQUE");
        assert_eq!(unique["unique"]["base"], "Leather Belt");
        assert_eq!(unique["category"], "Belt");
        let option = merged
            .stats
            .iter()
            .find(|v| {
                v["trade"]["ids"]["implicit"]
                    .as_array()
                    .is_some_and(|ids| ids.contains(&json!("implicit.stat_fixture_option")))
            })
            .unwrap();
        assert_eq!(
            option["matchers"][0]["string"],
            "Allocates Catalogue Notable"
        );
        assert_eq!(option["matchers"][0]["value"], 23);
        let second = merge_trade(&merged, &mods, &items).unwrap();
        assert_eq!(second.items, merged.items);
        assert_eq!(second.stats, merged.stats);
    }
}
