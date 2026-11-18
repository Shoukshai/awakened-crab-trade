//! poe.ninja's dense economy feed, matching upstream identity/variant rules.
//! This is an indicative market aggregate, never a valuation of a rare's mods.
use crate::{
    item::Item,
    trade::{encode_segment, header_cooldown},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TTL: u64 = 31 * 60;
const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[path = "economy/identity.rs"]
mod identity;
pub use identity::Key;
#[path = "economy/model.rs"]
pub mod model;
pub use model::Report;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub chaos: f64,
    pub graph: Vec<Option<f64>>,
    pub category: String,
    pub variant: Option<String>,
    pub age_seconds: u64,
    pub url: String,
    pub from_cache: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Line {
    name: String,
    variant: Option<String>,
    chaos: f64,
    #[serde(default)]
    graph: Vec<Option<f64>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Overview {
    #[serde(rename = "type")]
    kind: String,
    lines: Vec<Line>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Snapshot {
    #[serde(rename = "currencyOverviews", default)]
    currency: Vec<Overview>,
    #[serde(rename = "itemOverviews", default)]
    items: Vec<Overview>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Cached {
    version: u32,
    league: String,
    fetched: u64,
    data: Snapshot,
}
impl Cached {
    fn fresh(&self, league: &str, now: u64) -> bool {
        self.version == 1
            && self.league == league
            && self.fetched <= now
            && now - self.fetched < TTL
    }
    pub(crate) fn find(&self, key: &Key, now: u64, from_cache: bool) -> Option<Quote> {
        self.data
            .currency
            .iter()
            .chain(&self.data.items)
            .find_map(|overview| {
                let (namespace, route) = category(&overview.kind)?;
                if key.namespace != namespace {
                    return None;
                }
                let line = overview
                    .lines
                    .iter()
                    .find(|line| {
                        line.name == key.name
                            && line.variant == key.variant
                            && line.chaos.is_finite()
                            && line.chaos >= 0.
                    })
                    .or_else(|| {
                        if overview.kind != "Map" || key.name != "Map" {
                            return None;
                        }
                        let (tier, series) = key.variant.as_deref()?.split_once(", ")?;
                        let tier = tier.strip_prefix('T')?;
                        overview.lines.iter().find(|line| {
                            line.chaos.is_finite()
                                && line.chaos >= 0.
                                && line.name == format!("Map (Tier {tier})")
                                && line.variant.as_deref() == Some(format!("T0, {series}").as_str())
                        })
                    })?;

                let mut league_slug = self.league.replacen("Hardcore ", "", 1).to_lowercase();
                if self.league.starts_with("Hardcore ") {
                    league_slug.push_str("hc");
                }
                Some(Quote {
                    chaos: line.chaos,
                    graph: line
                        .graph
                        .iter()
                        .map(|p| p.filter(|n| n.is_finite()))
                        .collect(),
                    category: overview.kind.clone(),
                    variant: line.variant.clone(),
                    age_seconds: now.saturating_sub(self.fetched),
                    url: format!(
                        "https://poe.ninja/poe1/economy/{}/{}/{}",
                        encode_segment(&league_slug),
                        route,
                        if overview.kind == "Map" && key.name == "Map" && line.name != key.name {
                            Key {
                                namespace: key.namespace.clone(),
                                name: line.name.clone(),
                                variant: line.variant.clone(),
                            }
                            .slug()
                        } else {
                            key.slug()
                        }
                    ),
                    from_cache,
                })
            })
    }
}
fn category(kind: &str) -> Option<(&'static str, &'static str)> {
    Some(match kind {
        "Currency" => ("ITEM", "currency"),
        "Fragment" => ("ITEM", "fragments"),
        "BaseType" => ("ITEM", "base-types"),
        "Map" => ("ITEM", "maps"),
        "DivinationCard" => ("DIVINATION_CARD", "divination-cards"),
        "SkillGem" => ("GEM", "skill-gems"),
        "Beast" => ("CAPTURED_BEAST", "beasts"),
        "UniqueWeapon" => ("UNIQUE", "unique-weapons"),
        "UniqueArmour" => ("UNIQUE", "unique-armours"),
        "UniqueAccessory" => ("UNIQUE", "unique-accessories"),
        "UniqueFlask" => ("UNIQUE", "unique-flasks"),
        "UniqueJewel" => ("UNIQUE", "unique-jewels"),
        "UniqueMap" => ("UNIQUE", "unique-maps"),
        "UniqueRelic" => ("UNIQUE", "unique-relics"),
        "UniqueTincture" => ("UNIQUE", "unique-tinctures"),
        "DeliriumOrb" => ("ITEM", "delirium-orbs"),
        "Scarab" => ("ITEM", "scarabs"),
        "Artifact" => ("ITEM", "artifacts"),
        "Fossil" => ("ITEM", "fossils"),
        "Resonator" => ("ITEM", "resonators"),
        "Incubator" => ("ITEM", "incubators"),
        "Oil" => ("ITEM", "oils"),
        "Vial" => ("ITEM", "vials"),
        "Invitation" => ("ITEM", "invitations"),
        "BlightedMap" => ("ITEM", "blighted-maps"),
        "BlightRavagedMap" => ("ITEM", "blight-ravaged-maps"),
        "Essence" => ("ITEM", "essences"),
        "Tattoo" => ("ITEM", "tattoos"),
        "Omen" => ("ITEM", "omens"),
        "Coffin" => ("ITEM", "coffins"),
        "AllflameEmber" => ("ITEM", "allflame-embers"),
        "DjinnCoin" => ("ITEM", "djinn-coins"),
        "Astrolabe" => ("ITEM", "astrolabes"),
        "Runegraft" => ("ITEM", "runegrafts"),
        "Ducat" => ("ITEM", "ducats"),
        "EnshroudingCrystal" => ("ITEM", "enshrouding-crystals"),
        "Corpse" => ("ITEM", "corpses"),
        "Viscera" => ("ITEM", "viscera"),
        "Wombgift" => ("ITEM", "wombgifts"),
        _ => return None,
    })
}
pub struct Economy {
    client: Client,
    base: String,
    cache_dir: PathBuf,
    memory: HashMap<String, Cached>,
    next: Instant,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl Economy {
    pub fn new() -> Result<Self, String> {
        let cache = crate::paths::cache_root();
        Ok(Self {
            client: Client::builder()
                .user_agent(concat!(
                    "Awakened-PoE-Trade-Rust/",
                    env!("CARGO_PKG_VERSION")
                ))
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?,
            base: crate::trade::fixture_endpoint("APT_RS_TEST_ECONOMY_BASE")?
                .unwrap_or_else(|| "https://poe.ninja".into()),
            cache_dir: cache.join("awakened-crab-trade/economy"),
            memory: HashMap::new(),
            next: Instant::now(),
        })
    }
    pub fn quote(
        &mut self,
        league: &str,
        item: &Item,
        generation: u64,
        current: &Arc<AtomicU64>,
    ) -> Result<Option<Quote>, String> {
        self.report(league, item, generation, current)
            .map(|r| r.quote)
    }
    pub fn report(
        &mut self,
        league: &str,
        item: &Item,
        generation: u64,
        current: &Arc<AtomicU64>,
    ) -> Result<Report, String> {
        let valid = || current.load(Ordering::Acquire) == generation;
        if !valid() {
            return Err("Cancelled".into());
        }
        let Some(key) = Key::for_item(item) else {
            return Ok(Report::default());
        };
        if league.trim().is_empty() || league.len() > 100 {
            return Err("Invalid economy league".into());
        }
        let timestamp = now();
        let path = self
            .cache_dir
            .join(format!("{}.json", encode_segment(league)));
        if !self.memory.contains_key(league) {
            let read_started = Instant::now();
            if let Ok(file) = std::fs::File::open(&path) {
                if file.metadata().is_ok_and(|m| m.len() <= MAX_BYTES) {
                    if let Ok(cached) = serde_json::from_reader::<_, Cached>(BufReader::new(file)) {
                        if cached.fresh(league, timestamp) {
                            self.memory.insert(league.into(), cached);
                            eprintln!(
                                "economy disk_read_us={}",
                                read_started.elapsed().as_micros()
                            );
                        }
                    }
                }
            }
        }
        if let Some(cached) = self
            .memory
            .get(league)
            .filter(|c| c.fresh(league, timestamp))
        {
            return Ok(cached.report(&key, item, timestamp, true));
        }
        if self.next > Instant::now() {
            return Err("poe.ninja cooldown; retry later".into());
        }
        self.next = Instant::now() + Duration::from_secs(30);
        let network_started = Instant::now();
        let response = self
            .client
            .get(format!(
                "{}/poe1/api/economy/current/dense/overviews",
                self.base
            ))
            .query(&[("league", league), ("language", "en")])
            .send()
            .map_err(|e| format!("poe.ninja: {e}"))?;
        self.next = self
            .next
            .max(Instant::now() + header_cooldown(response.headers()));
        if !response.status().is_success() {
            return Err(format!(
                "poe.ninja HTTP {}; no automatic retry",
                response.status().as_u16()
            ));
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("poe.ninja response exceeds 32 MiB".into());
        }
        let network_us = network_started.elapsed().as_micros();
        let decode_started = Instant::now();
        let data: Snapshot =
            serde_json::from_slice(&bytes).map_err(|e| format!("poe.ninja JSON: {e}"))?;
        if data.currency.is_empty() && data.items.is_empty() {
            return Err("poe.ninja returned no economy overviews".into());
        }
        if !valid() {
            return Err("Cancelled".into());
        }
        let cached = Cached {
            version: 1,
            league: league.into(),
            fetched: now(),
            data,
        };
        let decode_us = decode_started.elapsed().as_micros();
        let write_started = Instant::now();
        // Cache IO failures leave a usable in-memory result and are logged separately.
        if let Err(e) = self.save(&path, &cached) {
            eprintln!("economy cache write error={e}");
        }
        eprintln!(
            "economy network_us={network_us} decode_us={decode_us} cache_write_us={}",
            write_started.elapsed().as_micros()
        );
        let report = cached.report(&key, item, now(), false);
        if self.memory.len() >= 4 {
            self.memory.clear();
        }
        self.memory.insert(league.into(), cached);
        Ok(report)
    }
    fn save(&self, path: &std::path::Path, data: &Cached) -> Result<(), String> {
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| e.to_string())?;
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        {
            let file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer(&mut writer, data).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
        }
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(rarity: &str, name: &str, body: &str) -> Item {
        Item::parse(&format!(
            "Item Class: Test\nRarity: {rarity}\n{name}\n--------\n{body}"
        ))
        .unwrap()
    }
    fn server(
        status: &str,
        body: &str,
        extra: &str,
        current: Option<Arc<AtomicU64>>,
    ) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{BufRead, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let response=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",body.len());
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&mut socket);
            let mut line = String::new();
            loop {
                line.clear();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
            }
            if let Some(current) = current {
                current.store(2, Ordering::Release);
            }
            socket.write_all(response.as_bytes()).unwrap();
        });
        (url, thread)
    }
    #[test]
    fn ninja_variant_links_preserve_gem_separators_and_collapse_empty_tokens() {
        let gem = Item::parse(include_str!("fixtures/price-gem.txt")).unwrap();
        assert_eq!(Key::for_item(&gem).unwrap().slug(), "impale-support-20-20c");
        let viscera = Item::parse(include_str!("fixtures/price-viscera.txt")).unwrap();
        assert_eq!(Key::for_item(&viscera).unwrap().slug(), "greater-severed-rhoa-head-every-second-inflict-withered-on-enemies-within-metres-for-seconds");
    }
    #[test]
    fn generic_map_quote_matches_tier_and_series_and_rejects_invalid_prices() {
        let data: Snapshot = serde_json::from_value(serde_json::json!({"itemOverviews":[{"type":"Map","lines":[{"name":"Map (Tier 16)","variant":"T0, Gen-24","chaos":3}]}]})).unwrap();
        let mut cache = Cached {
            version: 1,
            league: "Covens".into(),
            fetched: 100,
            data,
        };
        let item = Item::parse(include_str!("fixtures/price-map.txt")).unwrap();
        let key = Key::for_item(&item).unwrap();
        let quote = cache.find(&key, 101, false).unwrap();
        assert_eq!(quote.chaos, 3.);
        assert!(quote.url.ends_with("/map-tier-16-t0-gen-24"));
        assert!(cache
            .find(
                &Key {
                    variant: Some("T15, Gen-24".into()),
                    ..key.clone()
                },
                101,
                false
            )
            .is_none());
        for invalid in [-1., f64::NAN, f64::INFINITY] {
            cache.data.items[0].lines[0].chaos = invalid;
            assert!(cache.find(&key, 101, false).is_none());
        }
    }
    #[test]
    fn base_quotes_keep_item_level_and_influence_and_reject_double_influence() {
        let mut i = item("Normal", "Vaal Regalia", "Item Level: 87\nShaper Item");
        assert_eq!(
            Key::for_item(&i).unwrap().variant.as_deref(),
            Some("86+, Shaper")
        );
        i.influences.push("Elder".into());
        assert!(Key::for_item(&i).is_none());
        let i = item("Unique", "Lifesprig\nDriftwood Wand", "Unidentified");
        assert!(Key::for_item(&i).is_none());
    }
    #[test]
    fn economy_fetch_persists_cache_and_reuses_it_without_network() {
        let body = r#"{"currencyOverviews":[{"type":"Currency","lines":[{"name":"Scroll of Wisdom","chaos":0.008,"graph":[null,0,1]}]}]}"#;
        let (url, thread) = server("200 OK", body, "", None);
        let mut economy = Economy::new().unwrap();
        economy.base = url;
        let path = std::env::temp_dir().join(format!(
            "apt-economy-cache-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        economy.cache_dir = path.clone();
        let current = Arc::new(AtomicU64::new(1));
        let i = item("Currency", "Scroll of Wisdom", "");
        let q = economy.quote("Covens", &i, 1, &current).unwrap().unwrap();
        thread.join().unwrap();
        assert_eq!(q.chaos, 0.008);
        assert!(!q.from_cache);
        let mut restarted = Economy::new().unwrap();
        restarted.base = "http://127.0.0.1:1".into();
        restarted.cache_dir = path.clone();
        assert!(
            restarted
                .quote("Covens", &i, 1, &current)
                .unwrap()
                .unwrap()
                .from_cache
        );
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn economy_429_blocks_retries_and_cancelled_fetch_does_not_cache() {
        let (url, thread) = server("429 Too Many Requests", "{}", "Retry-After: 60\r\n", None);
        let mut economy = Economy::new().unwrap();
        economy.base = url;
        economy.cache_dir = std::env::temp_dir().join(format!(
            "apt-economy-errors-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let current = Arc::new(AtomicU64::new(1));
        let i = item("Currency", "Scroll of Wisdom", "");
        assert!(economy
            .quote("Covens", &i, 1, &current)
            .unwrap_err()
            .contains("429"));
        thread.join().unwrap();
        assert!(economy.next > Instant::now() + Duration::from_secs(55));
        assert!(economy
            .quote("Covens", &i, 1, &current)
            .unwrap_err()
            .contains("cooldown"));
        let (url, thread) = server(
            "200 OK",
            r#"{"currencyOverviews":[{"type":"Currency","lines":[]}]}"#,
            "",
            Some(current.clone()),
        );
        economy.base = url;
        economy.next = Instant::now();
        assert_eq!(
            economy.quote("Covens", &i, 1, &current).unwrap_err(),
            "Cancelled"
        );
        thread.join().unwrap();
        assert!(economy.memory.is_empty());
    }
    #[test]
    fn gem_variants_do_not_mix_quality_or_corruption() {
        let i = item("Gem", "Fireball", "Level: 20\nQuality: +18%\nCorrupted");
        assert_eq!(
            Key::for_item(&i).unwrap().variant.as_deref(),
            Some("20/20c")
        );
        let i = item("Gem", "Empower Support", "Level: 4\nQuality: +20%");
        assert_eq!(Key::for_item(&i).unwrap().variant.as_deref(), Some("4"));
    }
    #[test]
    fn unique_links_and_base_are_part_of_the_identity() {
        let i = item(
            "Unique",
            "Lifesprig\nDriftwood Wand",
            "Sockets: R-G-B-R-G-B",
        );
        assert_eq!(
            Key::for_item(&i).unwrap().variant.as_deref(),
            Some("Driftwood Wand, 6L")
        );
    }
    #[test]
    fn cache_never_crosses_leagues_and_does_not_survive_clock_rollback() {
        let cache = Cached {
            version: 1,
            league: "Covens".into(),
            fetched: 100,
            data: Snapshot {
                currency: vec![],
                items: vec![],
            },
        };
        assert!(cache.fresh("Covens", 101));
        assert!(!cache.fresh("Standard", 101));
        assert!(!cache.fresh("Covens", 99));
        assert!(!cache.fresh("Covens", 100 + TTL));
    }
    #[test]
    fn dense_quote_matches_namespace_and_exact_variant_preserving_gaps() {
        let data:Snapshot = serde_json::from_value(serde_json::json!({"itemOverviews":[{"type":"SkillGem","lines":[{"name":"Fireball","variant":"20/20","chaos":20,"graph":[null,0,-10]},{"name":"Fireball","variant":"20/20c","chaos":8.7}]}]})).unwrap();
        let cache = Cached {
            version: 1,
            league: "Covens".into(),
            fetched: 100,
            data,
        };
        let key = Key {
            namespace: "GEM".into(),
            name: "Fireball".into(),
            variant: Some("20/20".into()),
        };
        let q = cache.find(&key, 103, true).unwrap();
        assert_eq!(q.chaos, 20.);
        assert_eq!(q.graph, vec![None, Some(0.), Some(-10.)]);
        assert_eq!(q.age_seconds, 3);
        assert!(cache
            .find(
                &Key {
                    namespace: "ITEM".into(),
                    ..key
                },
                103,
                true
            )
            .is_none());
    }
    #[test]
    fn cancellation_prevents_any_economy_request() {
        let mut economy = Economy::new().unwrap();
        economy.base = "http://127.0.0.1:1".into();
        assert_eq!(
            economy
                .quote(
                    "Covens",
                    &item("Currency", "Chaos Orb", ""),
                    1,
                    &Arc::new(AtomicU64::new(2))
                )
                .unwrap_err(),
            "Cancelled"
        );
    }
}

#[cfg(test)]
mod variant_metadata_tests {
    use super::*;
    #[test]
    fn special_unique_traits_keep_an_explicit_ordinary_reference_warning() {
        for text in [
            "Foulborn Lifesprig\nDriftwood Wand",
            "Lifesprig\nVestigial Driftwood Wand",
            "Lifesprig\nDriftwood Wand\n--------\nFoil Unique",
        ] {
            let item = Item::parse(&format!(
                "Item Class: Wands\nRarity: Unique\n{text}\n--------\nItem Level: 83"
            ))
            .unwrap();
            let key = Key::for_item(&item).unwrap();
            let cache:Cached=serde_json::from_value(serde_json::json!({"version":1,"league":"Covens","fetched":100,"data":{"itemOverviews":[{"type":"UniqueWeapon","lines":[{"name":"Lifesprig","variant":"Driftwood Wand","chaos":5}]}]}})).unwrap();
            let report = cache.report(&key, &item, 103, true);
            assert_eq!(report.quote.unwrap().chaos, 5.);
            assert!(report.warning.unwrap().contains("identité ordinaire"));
        }
    }
    #[test]
    fn missing_base_price_does_not_hide_the_related_set_or_invent_missing_prices() {
        let item=Item::parse("Item Class: Map Fragments\nRarity: Normal\nSacrifice at Dawn\n--------\nStack Size: 1/10").unwrap();
        let cache:Cached=serde_json::from_value(serde_json::json!({"version":1,"league":"Covens","fetched":100,"data":{"currencyOverviews":[{"type":"Fragment","lines":[{"name":"Sacrifice at Midnight","chaos":2}]}]}})).unwrap();
        let report = cache.report(&Key::for_item(&item).unwrap(), &item, 103, true);
        assert!(report.quote.is_none());
        let related = report.related.unwrap();
        assert_eq!(related.related.len(), 4);
        assert_eq!(
            related.related.iter().filter(|r| r.price.is_some()).count(),
            1
        );
        assert!(related
            .related
            .iter()
            .find(|r| r.highlight)
            .unwrap()
            .price
            .is_none());
    }
    #[test]
    fn abyssal_sockets_use_the_first_source_and_detail_urls_normalize_unicode() {
        let item=Item::parse("Item Class: Gloves\nRarity: Unique\nTombfist\nSteelscale Gauntlets\n--------\n{ Modifier \"Fixture\" }\nHas 1 Abyssal Socket\n{ Modifier \"Fixture\" }\nHas 1 Abyssal Socket").unwrap();
        assert_eq!(item.modifiers[0].value, Some(2.));
        assert_eq!(item.modifiers[0].sources[0].value, Some(1.));
        assert_eq!(
            Key::for_item(&item).unwrap().variant.as_deref(),
            Some("1 Jewel, Steelscale Gauntlets")
        );
        assert_eq!(
            Key {
                namespace: "ITEM".into(),
                name: "Maëlström of Chaos".into(),
                variant: None
            }
            .slug(),
            "maelstrom-of-chaos"
        );
    }
}
