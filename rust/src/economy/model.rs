//! Values and relationships from Prices.ts, PriceTrend.vue and RelatedItems.vue.
use super::{Cached, Key, Quote};
use crate::item::Item;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrencyValue {
    pub min: f64,
    pub max: f64,
    pub currency: String,
}
pub fn auto_currency(value: f64, max: Option<f64>, rate: Option<f64>) -> CurrencyValue {
    let rate = rate.unwrap_or(9999.);
    if let Some(max) = max {
        if max > rate {
            CurrencyValue {
                min: value / rate,
                max: max / rate,
                currency: "div".into(),
            }
        } else {
            CurrencyValue {
                min: value,
                max,
                currency: "chaos".into(),
            }
        }
    } else if value > rate * 0.94 {
        let v = if value < rate * 1.06 {
            1.
        } else {
            value / rate
        };
        CurrencyValue {
            min: v,
            max: v,
            currency: "div".into(),
        }
    } else {
        CurrencyValue {
            min: value,
            max: value,
            currency: "chaos".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trend {
    pub points: Vec<f64>,
    pub draw_min: f64,
    pub draw_max: f64,
    pub forecast: String,
    pub text: String,
}
impl Trend {
    pub fn of(graph: &[Option<f64>]) -> Option<Self> {
        let points: Vec<_> = graph
            .iter()
            .flatten()
            .copied()
            .filter(|v| v.is_finite())
            .collect();
        if points.len() < 2 {
            return None;
        }
        let forecast = if points.len() == 7 {
            if points.iter().filter(|p| **p > 0.).count() >= 4
                || points[4..].iter().all(|p| *p > 0.)
            {
                "up"
            } else if points.iter().filter(|p| **p < 0.).count() >= 4
                || points[4..].iter().all(|p| *p < 0.)
            {
                "down"
            } else {
                "const"
            }
        } else {
            "const"
        };
        let mean = points.iter().sum::<f64>() / points.len() as f64;
        let change = (points.iter().map(|p| (p - mean).powi(2)).sum::<f64>()
            / (points.len() - 1) as f64)
            .sqrt();
        let padding = if change == 0. { 1. } else { change };
        Some(Self {
            draw_min: points.iter().copied().fold(f64::INFINITY, f64::min) - padding,
            draw_max: points.iter().copied().fold(f64::NEG_INFINITY, f64::max) + padding,
            text: format!("{}\u{2009}%", (change * 2. + 0.5).floor()),
            points,
            forecast: forecast.into(),
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackValue {
    pub have: u32,
    pub price: CurrencyValue,
    pub maximum: Option<u32>,
    pub full_stack_price: Option<CurrencyValue>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedItem {
    pub key: Key,
    pub name: String,
    pub quote: Option<Quote>,
    pub price: Option<CurrencyValue>,
    pub highlight: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Related {
    pub error: Option<String>,
    pub related: Vec<RelatedItem>,
    pub items: Vec<RelatedItem>,
}
#[derive(Deserialize)]
struct Drop {
    query: Vec<String>,
    items: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    pub identity: Option<Key>,
    pub quote: Option<Quote>,
    pub price: Option<CurrencyValue>,
    pub trend: Option<Trend>,
    pub stack: Option<StackValue>,
    pub related: Option<Related>,
    pub divine_rate: Option<f64>,
    pub warning: Option<String>,
    pub age_seconds: Option<u64>,
    pub from_cache: bool,
}
pub fn related_keys(key: &Key) -> Option<(Vec<Key>, Vec<Key>)> {
    static DROPS: OnceLock<Vec<Drop>> = OnceLock::new();
    let drops = DROPS.get_or_init(|| {
        serde_json::from_str(include_str!("../data/item-drop.json"))
            .expect("upstream drop relationships")
    });
    let id = key.query_id();
    let group = drops.iter().find(|d| d.query.contains(&id))?;
    Some((
        group
            .query
            .iter()
            .filter_map(|s| Key::from_query_id(s))
            .collect(),
        group
            .items
            .iter()
            .filter_map(|s| Key::from_query_id(s))
            .collect(),
    ))
}
impl Cached {
    pub(crate) fn related(
        &self,
        key: &Key,
        now: u64,
        from_cache: bool,
        divine_rate: Option<f64>,
    ) -> Option<Related> {
        related_keys(key).map(|(query, items)| {
            let rows = |keys: Vec<Key>| -> Result<Vec<RelatedItem>, String> {
                keys.into_iter()
                    .map(|k| {
                        let name = crate::item::catalogue_name(
                            &k.namespace,
                            &k.name,
                            k.variant.as_deref(),
                        )
                        .ok_or_else(|| format!("Can't find \"{}\".", k.query_id()))?;
                        let quote = self.find(&k, now, from_cache);
                        let price = quote
                            .as_ref()
                            .map(|q| auto_currency(q.chaos, None, divine_rate));
                        Ok(RelatedItem {
                            highlight: k == *key,
                            key: k,
                            name,
                            quote,
                            price,
                        })
                    })
                    .collect()
            };
            match rows(query).and_then(|related| {
                rows(items).map(|items| Related {
                    error: None,
                    related,
                    items,
                })
            }) {
                Ok(r) => r,
                Err(e) => Related {
                    error: Some(e),
                    ..Default::default()
                },
            }
        })
    }
    pub(crate) fn report(&self, key: &Key, item: &Item, now: u64, from_cache: bool) -> Report {
        let divine_rate = self
            .find(
                &Key {
                    namespace: "ITEM".into(),
                    name: "Divine Orb".into(),
                    variant: None,
                },
                now,
                from_cache,
            )
            .filter(|q| q.chaos >= 30.)
            .map(|q| q.chaos);
        let quote = self.find(key, now, from_cache);
        let money = |amount: f64| {
            if key.name == "Divine Orb" {
                CurrencyValue {
                    min: amount,
                    max: amount,
                    currency: "chaos".into(),
                }
            } else {
                auto_currency(amount, None, divine_rate)
            }
        };
        let price = quote.as_ref().map(|q| money(q.chaos));
        let stack = quote
            .as_ref()
            .zip(item.stack_size)
            .map(|(q, n)| StackValue {
                have: n,
                price: money(q.chaos * n as f64),
                maximum: item.stack_max,
                full_stack_price: item.stack_max.map(|n| money(q.chaos * n as f64)),
            });
        let related = self.related(key, now, from_cache, divine_rate);
        let mut warning = if item.market.foulborn || item.market.vestigial || item.market.foil {
            Some("Cote de l’identité ordinaire : cette correspondance ne distingue pas Foulborn, Vestigial ou Foil. Vérifiez les offres Trade.".into())
        } else if item.reference == "Vessel of Vinktar"
            && key.variant.is_none()
            && item
                .modifiers
                .iter()
                .any(|m| m.reference.ends_with("during Effect"))
        {
            Some("Cote générique du Vessel of Vinktar : les anciennes variantes du flacon ne sont pas distinguées par cette identité.".into())
        } else {
            None
        };
        if item.unidentified {
            let message = "Cote de l’identité choisie : l’objet reste non identifié, les mods cachés ne sont pas évalués.";
            warning = Some(match warning {
                Some(other) => format!("{message}\n{other}"),
                None => message.into(),
            });
        }
        Report {
            identity: Some(key.clone()),
            trend: quote.as_ref().and_then(|q| Trend::of(&q.graph)),
            quote,
            price,
            stack,
            related,
            divine_rate,
            warning,
            age_seconds: Some(now.saturating_sub(self.fetched)),
            from_cache,
        }
    }
}
