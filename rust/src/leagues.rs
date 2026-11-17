//! Current PoE1 PC softcore challenge league from poe.ninja’s public economy list.
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Selection {
    pub id: String,
    pub from_cache: bool,
}
#[derive(Debug, Deserialize, Serialize)]
struct Rule {
    id: String,
}
#[derive(Debug, Default, Deserialize, Serialize)]
struct Category {
    #[serde(default)]
    current: bool,
}
#[derive(Debug, Deserialize, Serialize)]
struct League {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    event: bool,
    #[serde(default)]
    rules: Vec<Rule>,
    #[serde(default)]
    category: Option<Category>,
}
#[derive(Deserialize, Serialize)]
struct Cache {
    fetched: u64,
    leagues: Vec<League>,
}
fn select(leagues: &[League]) -> Result<String, String> {
    let eligible = |l: &&League| {
        let id = l.id.to_ascii_lowercase();
        !l.event
            && !matches!(id.as_str(), "hardcore" | "solo self-found" | "ruthless")
            && !id.starts_with("hardcore ")
            && !id.starts_with("hc ")
            && !id.starts_with("ssf ")
            && !id.contains("ruthless")
            && !l
                .rules
                .iter()
                .any(|r| matches!(r.id.as_str(), "Hardcore" | "NoParties" | "HardMode"))
    };
    if let Some(l) = leagues
        .iter()
        .filter(eligible)
        .find(|l| l.category.as_ref().is_some_and(|c| c.current))
    {
        return valid_id(&l.id);
    }
    // The economy endpoint lists only its currently supported leagues. Refuse
    // ambiguity instead of silently selecting a historical or hardcore league.
    let challenge: Vec<_> = leagues
        .iter()
        .filter(eligible)
        .filter(|l| {
            l.category.is_none() && l.name.as_deref() == Some(l.id.as_str()) && l.id != "Standard"
        })
        .collect();
    match challenge.as_slice() {
        [league] => valid_id(&league.id),
        [] => leagues
            .iter()
            .find(|l| l.id == "Standard")
            .ok_or_else(|| "Aucune ligue actuelle disponible.".to_string())
            .and_then(|l| valid_id(&l.id)),
        _ => Err(
            "Plusieurs ligues actuelles disponibles ; choisissez une ligue dans Réglages.".into(),
        ),
    }
}
fn valid_id(id: &str) -> Result<String, String> {
    if id.is_empty() || id.len() > 100 {
        Err("Identifiant de ligue invalide.".into())
    } else {
        Ok(id.to_string())
    }
}
fn cache_path() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache")
        })
        .join("awakened-poe-trade-rs/leagues.json")
}
pub fn resolve() -> Result<Selection, String> {
    resolve_at(
        "https://poe.ninja/poe1/api/economy/leagues",
        &cache_path(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
    )
}
fn resolve_at(url: &str, path: &std::path::Path, now: u64) -> Result<Selection, String> {
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(c) = serde_json::from_slice::<Cache>(&bytes) {
            if now >= c.fetched && now - c.fetched < 300 {
                if let Ok(id) = select(&c.leagues) {
                    return Ok(Selection {
                        id,
                        from_cache: true,
                    });
                }
            }
        }
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(3))
        .user_agent(concat!(
            "Awakened-PoE-Trade-Rust/",
            env!("CARGO_PKG_VERSION"),
            " (native Linux development)"
        ))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("Détection de ligue indisponible : {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Détection de ligue : HTTP {}. Réessayez plus tard ou choisissez une ligue dans Réglages.",response.status()));
    }
    let mut bytes = Vec::new();
    response
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("Liste des ligues trop volumineuse.".into());
    }
    let leagues: Vec<League> =
        serde_json::from_slice(&bytes).map_err(|e| format!("Liste des ligues invalide : {e}"))?;
    let id = select(&leagues)?;
    let cache = Cache {
        fetched: now,
        leagues,
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_ok() {
            let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
            if let Ok(data) = serde_json::to_vec(&cache) {
                if std::fs::write(&tmp, data).is_ok() {
                    let _ = std::fs::rename(tmp, path);
                }
            }
        }
    }
    Ok(Selection {
        id,
        from_cache: false,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn list(s: &str) -> Vec<League> {
        serde_json::from_str(s).unwrap()
    }
    #[test]
    fn current_marker_selects_softcore_not_hardcore_ssf_ruthless_or_events() {
        let leagues = list(
            r#"[{"id":"Old","category":{"current":false}},{"id":"HC New","category":{"current":true},"rules":[{"id":"Hardcore"}]},{"id":"SSF New","category":{"current":true},"rules":[{"id":"NoParties"}]},{"id":"R New","category":{"current":true},"rules":[{"id":"HardMode"}]},{"id":"Race","event":true,"category":{"current":true}},{"id":"New","category":{"current":true}},{"id":"Standard"}]"#,
        );
        assert_eq!(select(&leagues).unwrap(), "New");
    }
    #[test]
    fn current_economy_list_selects_softcore_challenge_independently_of_order() {
        let data = list(
            r#"[{"id":"Standard","name":"Standard"},{"id":"Hardcore Covens","name":"Hardcore Covens"},{"id":"Covens","name":"Covens"},{"id":"Hardcore","name":"Hardcore"}]"#,
        );
        assert_eq!(select(&data).unwrap(), "Covens");
        let ambiguous = list(
            r#"[{"id":"One","name":"One"},{"id":"Two","name":"Two"},{"id":"Standard","name":"Standard"}]"#,
        );
        assert!(select(&ambiguous).is_err());
    }
    #[test]
    fn no_challenge_falls_back_to_permanent_only_when_available() {
        assert_eq!(
            select(&list(
                r#"[{"id":"Hardcore","rules":[{"id":"Hardcore"}]},{"id":"Standard"}]"#
            ))
            .unwrap(),
            "Standard"
        );
        assert!(select(&list("[]")).is_err());
        assert!(select(&list(r#"[{"id":"Expired"}]"#)).is_err());
    }
    #[test]
    fn fresh_cache_avoids_network_but_stale_or_future_cache_is_rejected() {
        let path = std::env::temp_dir().join(format!("apt-leagues-{}.json", std::process::id()));
        std::fs::write(
            &path,
            r#"{"fetched":100,"leagues":[{"id":"Current","category":{"current":true}}]}"#,
        )
        .unwrap();
        let selected = resolve_at("http://127.0.0.1:9", &path, 101).unwrap();
        assert_eq!(selected.id, "Current");
        assert!(selected.from_cache);
        assert!(resolve_at("http://127.0.0.1:9", &path, 401).is_err());
        assert!(resolve_at("http://127.0.0.1:9", &path, 99).is_err());
        let _ = std::fs::remove_file(path);
    }
}
