//! A single worker owns this client. No automatic retries, bounded timeouts,
//! conservative pacing, server rate-limit headers and short-lived query cache.
use reqwest::{
    blocking::{Client, Response},
    header::HeaderMap,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub amount: f64,
    pub currency: String,
    pub seller: String,
    pub bulk: Option<BulkOffer>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOffer {
    pub exchange_amount: f64,
    pub item_amount: f64,
    pub stock: f64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchKind {
    Item,
    Bulk,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prices {
    pub kind: SearchKind,
    pub total: u64,
    pub listings: Vec<Listing>,
    pub url: String,
    pub network_ms: u128,
    pub cached: bool,
}
pub struct Trade {
    client: Client,
    base: String,
    next: Instant,
    cache: HashMap<String, (Instant, Prices)>,
}
impl Trade {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            client: Client::builder()
                .user_agent(concat!(
                    "Awakened-PoE-Trade-Rust/",
                    env!("CARGO_PKG_VERSION"),
                    " (native Linux development)"
                ))
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?,
            base: "https://www.pathofexile.com".into(),
            next: Instant::now(),
            cache: HashMap::new(),
        })
    }
    fn decode(&mut self, response: Response) -> Result<Value, String> {
        let status = response.status();
        self.next = self
            .next
            .max(Instant::now() + header_cooldown(response.headers()));
        if status.as_u16() == 429 {
            return Err(format!(
                "Trade rate limit (429). Wait {} s, then retry explicitly.",
                self.next
                    .saturating_duration_since(Instant::now())
                    .as_secs()
                    + 1
            ));
        }
        if !status.is_success() {
            return Err(format!(
                "Trade HTTP {}. No retry was sent.",
                status.as_u16()
            ));
        }
        let value: Value = response
            .json()
            .map_err(|e| format!("Invalid Trade JSON: {e}"))?;
        if let Some(error) = value.get("error").filter(|v| !v.is_null()) {
            return Err(format!("Trade: {error}"));
        }
        Ok(value)
    }
    pub fn search(
        &mut self,
        league: &str,
        query: &Value,
        generation: u64,
        current: &Arc<AtomicU64>,
    ) -> Result<Prices, String> {
        let valid = || current.load(Ordering::Acquire) == generation;
        if !valid() {
            return Err("Cancelled".into());
        }
        if league.is_empty() || league.len() > 100 {
            return Err("Choose a valid league".into());
        }
        let key = format!("{league}:{query}");
        if let Some((at, prices)) = self
            .cache
            .get(&key)
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(60))
        {
            let _ = at;
            let mut p = prices.clone();
            p.cached = true;
            p.network_ms = 0;
            return Ok(p);
        }
        if self.next > Instant::now() {
            return Err(format!(
                "Trade cooldown: retry in {} s",
                self.next.duration_since(Instant::now()).as_secs() + 1
            ));
        }
        let start = Instant::now();
        let kind = if query["engine"] == "new" {
            SearchKind::Bulk
        } else {
            SearchKind::Item
        };
        let route = if kind == SearchKind::Bulk {
            "exchange"
        } else {
            "search"
        };
        let mut url = reqwest::Url::parse(&format!("{}/api/trade/{route}/", self.base))
            .map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "Invalid Trade URL")?
            .pop_if_empty()
            .push(league);
        self.next = Instant::now() + Duration::from_secs(5);
        let response = self
            .client
            .post(url)
            .json(query)
            .send()
            .map_err(|e| format!("Trade search: {e}"))?;
        let result = self.decode(response)?;
        if !valid() {
            return Err("Cancelled".into());
        }
        let id = result["id"]
            .as_str()
            .ok_or("Trade search response missing id")?;
        if !valid_search_id(id) {
            return Err("Invalid search ID".into());
        }
        if kind == SearchKind::Bulk {
            let mut prices = decode_bulk(&result, query)?;
            prices.url = format!(
                "{}/trade/exchange/{}/{}",
                self.base,
                encode_segment(league),
                id
            );
            prices.network_ms = start.elapsed().as_millis();
            self.remember(key, &prices);
            return Ok(prices);
        }
        let ids = result["result"]
            .as_array()
            .ok_or("Trade search response missing results")?
            .iter()
            .take(10)
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        if ids
            .iter()
            .any(|id| !id.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err("Invalid listing ID".into());
        }
        let mut prices = Prices {
            kind,
            total: result["total"]
                .as_u64()
                .ok_or("Trade search missing total")?,
            listings: vec![],
            url: format!(
                "{}/trade/search/{}/{}",
                self.base,
                encode_segment(league),
                id
            ),
            network_ms: 0,
            cached: false,
        };
        if !ids.is_empty() {
            let response = self
                .client
                .get(format!("{}/api/trade/fetch/{}", self.base, ids.join(",")))
                .query(&[("query", id)])
                .send()
                .map_err(|e| format!("Trade fetch: {e}"))?;
            let data = self.decode(response)?;
            if !valid() {
                return Err("Cancelled".into());
            }
            for entry in data["result"]
                .as_array()
                .ok_or("Trade fetch missing listings")?
            {
                let price = &entry["listing"]["price"];
                if let (Some(amount), Some(currency)) =
                    (price["amount"].as_f64(), price["currency"].as_str())
                {
                    prices.listings.push(Listing {
                        amount,
                        currency: currency.into(),
                        seller: entry["listing"]["account"]["name"]
                            .as_str()
                            .unwrap_or("?")
                            .into(),
                        bulk: None,
                    });
                }
            }
        }
        prices.network_ms = start.elapsed().as_millis();
        self.remember(key, &prices);
        Ok(prices)
    }
    fn remember(&mut self, key: String, prices: &Prices) {
        self.cache
            .retain(|_, (at, _)| at.elapsed() < Duration::from_secs(60));
        if self.cache.len() >= 64 {
            self.cache.clear();
        }
        self.cache.insert(key, (Instant::now(), prices.clone()));
    }
}
fn decode_bulk(data: &Value, query: &Value) -> Result<Prices, String> {
    let results = data["result"]
        .as_object()
        .ok_or("Exchange response missing result object")?;
    let have = query["query"]["have"][0]
        .as_str()
        .ok_or("Exchange payment currency missing")?;
    let want = query["query"]["want"][0]
        .as_str()
        .ok_or("Exchange wanted currency missing")?;
    let minimum = query["query"]["minimum"].as_f64().unwrap_or(0.);
    let mut listings = Vec::new();
    for entry in results.values() {
        let Some(offers) = entry["listing"]["offers"]
            .as_array()
            .filter(|a| a.len() == 1)
        else {
            continue;
        };
        let offer = &offers[0];
        if offer["exchange"]["currency"] != have || offer["item"]["currency"] != want {
            continue;
        }
        let (Some(paid), Some(received), Some(stock)) = (
            offer["exchange"]["amount"].as_f64(),
            offer["item"]["amount"].as_f64(),
            offer["item"]["stock"].as_f64(),
        ) else {
            continue;
        };
        if !paid.is_finite()
            || !received.is_finite()
            || !stock.is_finite()
            || paid <= 0.
            || received <= 0.
            || stock < minimum
            || stock < received
        {
            continue;
        }
        let unit = paid / received;
        if !unit.is_finite() || unit <= 0. {
            continue;
        }
        listings.push(Listing {
            amount: unit,
            currency: have.into(),
            seller: entry["listing"]["account"]["name"]
                .as_str()
                .unwrap_or("?")
                .into(),
            bulk: Some(BulkOffer {
                exchange_amount: paid,
                item_amount: received,
                stock,
            }),
        });
    }
    listings.sort_by(|a, b| a.amount.total_cmp(&b.amount));
    listings.truncate(20);
    Ok(Prices {
        kind: SearchKind::Bulk,
        total: data["total"]
            .as_u64()
            .ok_or("Exchange response missing total")?,
        listings,
        url: String::new(),
        network_ms: 0,
        cached: false,
    })
}
fn valid_search_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 8192
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
}
pub(crate) fn encode_segment(text: &str) -> String {
    text.bytes()
        .map(|c| {
            if c.is_ascii_alphanumeric() || b"-._~".contains(&c) {
                (c as char).to_string()
            } else {
                format!("%{c:02X}")
            }
        })
        .collect()
}
pub(crate) fn header_cooldown(headers: &HeaderMap) -> Duration {
    let mut seconds = headers
        .get("retry-after")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    if let Some(rules) = headers
        .get("x-rate-limit-rules")
        .and_then(|v| v.to_str().ok())
    {
        for rule in rules.split(',').map(str::trim) {
            let limits = headers
                .get(format!("x-rate-limit-{}", rule.to_ascii_lowercase()))
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let states = headers
                .get(format!("x-rate-limit-{}-state", rule.to_ascii_lowercase()))
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            for (limit, state) in limits.split(',').zip(states.split(',')) {
                let l: Vec<u64> = limit.split(':').filter_map(|n| n.parse().ok()).collect();
                let s: Vec<u64> = state.split(':').filter_map(|n| n.parse().ok()).collect();
                if l.len() == 3 && s.len() == 3 {
                    seconds = seconds.max(s[2]);
                    if s[0] >= l[0].saturating_sub(1) {
                        seconds = seconds.max(l[1]);
                    }
                }
            }
        }
    }
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    fn read_request(socket: &mut std::net::TcpStream) -> String {
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).unwrap();
            headers.push(byte[0]);
            assert!(headers.len() < 8192);
        }
        let headers = String::from_utf8(headers).unwrap();
        let length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|s| s.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        assert!(length < 8192);
        socket.read_exact(&mut vec![0; length]).unwrap();
        headers
    }
    fn json_response(value: &str) -> String {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",value.len(),value)
    }
    #[test]
    fn exchange_uses_one_request_and_preserves_price_direction_and_cache() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let request = read_request(&mut socket);
            assert!(request.starts_with("POST /api/trade/exchange/Hardcore%20Covens "));
            socket.write_all(json_response(r#"{"id":"bulk_123-a","total":1,"result":{"row":{"listing":{"account":{"name":"seller"},"offers":[{"exchange":{"currency":"chaos","amount":1},"item":{"currency":"wisdom","amount":125,"stock":1000}}]}}}}"#).as_bytes()).unwrap();
        });
        let mut trade = Trade::new().unwrap();
        trade.base = url;
        let query =
            serde_json::json!({"engine":"new","query":{"have":["chaos"],"want":["wisdom"]}});
        let current = Arc::new(AtomicU64::new(1));
        let p = trade
            .search("Hardcore Covens", &query, 1, &current)
            .unwrap();
        thread.join().unwrap();
        assert_eq!(p.kind, SearchKind::Bulk);
        assert_eq!(p.listings.len(), 1);
        assert_eq!(p.listings[0].amount, 0.008);
        let offer = p.listings[0].bulk.as_ref().unwrap();
        assert_eq!(offer.item_amount, 125.);
        assert_eq!(offer.exchange_amount, 1.);
        assert_eq!(offer.stock, 1000.);
        assert!(p
            .url
            .contains("/trade/exchange/Hardcore%20Covens/bulk_123-a"));
        assert!(
            trade
                .search("Hardcore Covens", &query, 1, &current)
                .unwrap()
                .cached
        );
    }
    #[test]
    fn exchange_rejects_invalid_lots_and_applies_stock_and_currency_filters() {
        let offer = |paid: f64, received: f64, stock: f64, currency: &str| serde_json::json!({"listing":{"offers":[{"exchange":{"currency":currency,"amount":paid},"item":{"currency":"wisdom","amount":received,"stock":stock}}]}});
        let data = serde_json::json!({"total":5,"result":{"cheap":offer(1.,100.,500.,"chaos"),"expensive":offer(2.,100.,500.,"chaos"),"zero":offer(1.,0.,500.,"chaos"),"stock":offer(1.,10.,20.,"chaos"),"other":offer(1.,100.,500.,"divine")}});
        let query = serde_json::json!({"query":{"have":["chaos"],"want":["wisdom"],"minimum":100}});
        let p = decode_bulk(&data, &query).unwrap();
        assert_eq!(p.listings.len(), 2);
        assert_eq!(p.listings[0].amount, 0.01);
        assert_eq!(p.listings[1].amount, 0.02);
        assert!(decode_bulk(&serde_json::json!({"result":[],"total":0}), &query).is_err());
    }
    #[test]
    fn search_fetch_and_cache_preserve_listing_currencies() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            for response in [
                r#"{"id":"H4s_abc-def","total":1,"result":["abc123"]}"#,
                r#"{"result":[{"listing":{"price":{"amount":1.5,"currency":"divine"},"account":{"name":"example"}}}]}"#,
            ] {
                let (mut socket, _) = listener.accept().unwrap();
                read_request(&mut socket);
                socket
                    .write_all(json_response(response).as_bytes())
                    .unwrap();
            }
        });
        let mut t = Trade::new().unwrap();
        t.base = url;
        let current = Arc::new(AtomicU64::new(1));
        let p = t
            .search("Hardcore Covens", &serde_json::json!({}), 1, &current)
            .unwrap();
        assert_eq!(p.total, 1);
        assert_eq!(p.listings[0].amount, 1.5);
        assert_eq!(p.listings[0].currency, "divine");
        assert!(p.url.contains("Hardcore%20Covens/H4s_abc-def"));
        thread.join().unwrap();
        assert!(
            t.search("Hardcore Covens", &serde_json::json!({}), 1, &current)
                .unwrap()
                .cached
        );
    }
    #[test]
    fn cancellation_after_search_prevents_fetch() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let current = Arc::new(AtomicU64::new(1));
        let cancel = current.clone();
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            read_request(&mut socket);
            cancel.store(2, Ordering::Release);
            socket
                .write_all(
                    json_response(r#"{"id":"abc","total":1,"result":["abc123"]}"#).as_bytes(),
                )
                .unwrap();
        });
        let mut t = Trade::new().unwrap();
        t.base = url;
        assert_eq!(
            t.search("Covens", &serde_json::json!({}), 1, &current)
                .unwrap_err(),
            "Cancelled"
        );
        thread.join().unwrap();
    }
    fn server(response: &'static str) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            read_request(&mut s);
            s.write_all(response.as_bytes()).unwrap();
        });
        (url, h)
    }
    #[test]
    fn search_accepts_current_base64url_query_ids() {
        assert!(valid_search_id("H4sIAAAA-Gf6_bUHAOvWlXOzAN"));
        assert!(!valid_search_id(""));
        assert!(!valid_search_id("../../bad"));
    }
    #[test]
    fn rate_limit_obeys_retry_after_without_retrying() {
        let (url,h) = server("HTTP/1.1 429 Too Many Requests\r\nContent-Length: 2\r\nRetry-After: 60\r\nConnection: close\r\n\r\n{}");
        let mut t = Trade::new().unwrap();
        t.base = url;
        let generation = Arc::new(AtomicU64::new(1));
        assert!(t
            .search("Standard", &serde_json::json!({}), 1, &generation)
            .unwrap_err()
            .contains("429"));
        h.join().unwrap();
        assert!(t.next > Instant::now() + Duration::from_secs(55));
        assert!(t
            .search("Standard", &serde_json::json!({}), 1, &generation)
            .unwrap_err()
            .contains("cooldown"));
    }
    #[test]
    fn malformed_json_is_an_error() {
        let (url, h) =
            server("HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nbad");
        let mut t = Trade::new().unwrap();
        t.base = url;
        assert!(t
            .search(
                "Standard",
                &serde_json::json!({}),
                1,
                &Arc::new(AtomicU64::new(1))
            )
            .unwrap_err()
            .contains("JSON"));
        h.join().unwrap();
    }
    #[test]
    fn cancellation_prevents_network_access() {
        let mut t = Trade::new().unwrap();
        t.base = "http://127.0.0.1:1".into();
        assert_eq!(
            t.search("Standard", &Value::Null, 1, &Arc::new(AtomicU64::new(2)))
                .unwrap_err(),
            "Cancelled"
        );
    }
    #[test]
    fn server_limit_windows_and_penalties_are_respected() {
        let mut h = HeaderMap::new();
        h.insert("x-rate-limit-rules", "Ip".parse().unwrap());
        h.insert("x-rate-limit-ip", "5:10:60,15:60:120".parse().unwrap());
        h.insert("x-rate-limit-ip-state", "4:10:0,3:60:23".parse().unwrap());
        assert_eq!(header_cooldown(&h), Duration::from_secs(23));
    }
}
