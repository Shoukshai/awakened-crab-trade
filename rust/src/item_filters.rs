//! Item-level search policy from upstream create-item-filters.ts. The model is
//! separate from stat filters and is editable per copied item by the native UI.
use crate::item::{Item, Rarity};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_trade: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_type_trade: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator_trade: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<Box<Search>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Numeric {
    pub value: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    pub disabled: bool,
}
impl Numeric {
    pub fn new(value: u32, disabled: bool) -> Self {
        Self {
            value,
            max: None,
            disabled,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toggle {
    pub disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub value: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corrupted {
    pub value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub value: String,
    pub disabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blighted {
    pub value: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Named {
    pub name: String,
    pub name_trade: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Veiled {
    pub stat_refs: Vec<String>,
    pub disabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub offline: bool,
    pub online_in_league: bool,
    pub merchant_only: bool,
    pub listed: Option<String>,
    pub currency: Option<String>,
    pub league: String,
    pub collapse_listings: String,
    pub collapse_merchant: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub search_exact: Search,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_relaxed: Option<Search>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rarity: Option<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_sockets: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_sockets: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrupted: Option<Corrupted>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractured: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imbued_gem: Option<Toggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirrored: Option<Toggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<Toggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foil: Option<Toggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foulborn: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vestigial: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub influences: Option<Vec<Choice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gem_level: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_tier: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_blighted: Option<Blighted>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_completion_reward: Option<Named>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_level: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_size: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unidentified: Option<FlagToggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veiled: Option<Veiled>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_level: Option<Numeric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentinel_charge: Option<Numeric>,
    pub trade: Trade,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagToggle {
    pub value: bool,
    pub disabled: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Options {
    pub league: String,
    pub merchant_only: bool,
    pub currency: Option<String>,
    pub collapse_listings: String,
    pub activate_stock_filter: bool,
    pub exact: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            league: "Standard".into(),
            merchant_only: true,
            currency: None,
            collapse_listings: "api".into(),
            activate_stock_filter: false,
            exact: false,
        }
    }
}
pub const CATEGORIES: &[(&str, &str)] = &[
    ("Map", "map"),
    ("Abyss Jewel", "jewel.abyss"),
    ("Amulet", "accessory.amulet"),
    ("Belt", "accessory.belt"),
    ("Body Armour", "armour.chest"),
    ("Boots", "armour.boots"),
    ("Bow", "weapon.bow"),
    ("Claw", "weapon.claw"),
    ("Dagger", "weapon.dagger"),
    ("Fishing Rod", "weapon.rod"),
    ("Flask", "flask"),
    ("Gloves", "armour.gloves"),
    ("Helmet", "armour.helmet"),
    ("Jewel", "jewel"),
    ("One-Handed Axe", "weapon.oneaxe"),
    ("One-Handed Mace", "weapon.onemace"),
    ("One-Handed Sword", "weapon.onesword"),
    ("Quiver", "armour.quiver"),
    ("Ring", "accessory.ring"),
    ("Rune Dagger", "weapon.runedagger"),
    ("Sceptre", "weapon.sceptre"),
    ("Shield", "armour.shield"),
    ("Staff", "weapon.staff"),
    ("Two-Handed Axe", "weapon.twoaxe"),
    ("Two-Handed Mace", "weapon.twomace"),
    ("Two-Handed Sword", "weapon.twosword"),
    ("Wand", "weapon.wand"),
    ("Warstaff", "weapon.warstaff"),
    ("Cluster Jewel", "jewel.cluster"),
    ("Heist Blueprint", "heistmission.blueprint"),
    ("Heist Contract", "heistmission.contract"),
    ("Heist Tool", "heistequipment.heisttool"),
    ("Heist Brooch", "heistequipment.heistreward"),
    ("Heist Gear", "heistequipment.heistweapon"),
    ("Heist Cloak", "heistequipment.heistutility"),
    ("Trinket", "accessory.trinket"),
    ("Sanctum Relic", "sanctum.relic"),
    ("Tincture", "tincture"),
    ("Charm", "azmeri.charm"),
    ("Idol", "idol"),
    ("Graft", "graft"),
    ("Chart", "chart"),
];
const CONSUMABLE: &[&str] = &[
    "Map",
    "Heist Blueprint",
    "Heist Contract",
    "Invitation",
    "Memory Line",
    "Expedition Logbook",
    "Chart",
];
const MAGIC_ONLY: &[&str] = &["Flask", "Tincture", "Voidstone", "Sanctum Relic", "Charm"];
fn base(name: &str) -> Search {
    Search {
        base_type: Some(name.into()),
        base_type_trade: Some(name.into()),
        ..Default::default()
    }
}
pub fn floor_bracket(value: u32, brackets: &[u32]) -> u32 {
    let mut prev = brackets[0];
    for n in brackets {
        if *n > value {
            return prev;
        }
        prev = *n;
    }
    prev
}
fn ceil_bracket(value: u32, brackets: &[u32]) -> u32 {
    let mut prev = brackets[0];
    for n in brackets {
        if *n < value {
            return prev;
        }
        prev = *n;
    }
    prev
}
pub fn exact_default(item: &Item) -> bool {
    let c = item.category.as_deref().unwrap_or("");
    (!item.market.craftable && item.rarity != Rarity::Unique)
        || item.unidentified
        || item.rarity == Rarity::Normal
        || (["Flask", "Tincture", "Sanctum Relic", "Idol"].contains(&c)
            && item.rarity != Rarity::Unique)
        || [
            "Charm",
            "Map",
            "Memory Line",
            "Invitation",
            "Heist Contract",
            "Heist Blueprint",
            "Chart",
            "Sentinel",
        ]
        .contains(&c)
        || ["Expedition Logbook", "Mercenary Warrant"].contains(&item.reference.as_str())
}
pub fn create(item: &Item, opts: &Options) -> Result<Plan, String> {
    if !item.market.known {
        return Err("Base d’objet inconnue ; recherche refusée.".into());
    }
    if !["api", "app"].contains(&opts.collapse_listings.as_str()) {
        return Err("Mode de regroupement inconnu.".into());
    }
    let m = &item.market;
    let c = item.category.as_deref().unwrap_or("");
    let r = &item.rarity;
    let reference = item.reference.as_str();
    let mut p = Plan {
        search_exact: Search::default(),
        search_relaxed: None,
        rarity: None,
        linked_sockets: None,
        white_sockets: None,
        corrupted: None,
        fractured: None,
        imbued_gem: None,
        mirrored: None,
        split: None,
        foil: None,
        foulborn: None,
        vestigial: None,
        influences: None,
        quality: None,
        gem_level: None,
        map_tier: None,
        map_blighted: None,
        map_completion_reward: None,
        item_level: None,
        stack_size: None,
        unidentified: None,
        veiled: None,
        area_level: None,
        sentinel_charge: None,
        trade: Trade {
            offline: false,
            online_in_league: false,
            merchant_only: opts.merchant_only && !(m.exchangeable && item.trade_tag.is_none()),
            listed: None,
            currency: opts.currency.clone(),
            league: opts.league.clone(),
            collapse_listings: opts.collapse_listings.clone(),
            collapse_merchant: false,
        },
    };
    if (!m.craftable || CONSUMABLE.contains(&c)) && *r != Rarity::Unique {
        if opts.currency.is_none() {
            p.trade.currency = Some("chaos_divine".into());
        }
        if reference != "Mercenary Warrant" {
            p.trade.collapse_merchant = true;
        }
    }
    if c == "Gem" {
        let gem = m.gem.as_ref().ok_or("Métadonnées de gemme absentes.")?;
        p.search_exact = base(&m.name);
        if gem.transfigured {
            p.search_exact.base_type_trade = gem.normal_variant.clone();
            p.search_exact.discriminator_trade = item.trade_discriminator.clone();
        }
        if let Some(vaal) = &m.vaal_gem {
            let mut sub = base(&vaal.name);
            sub.base_type_trade = Some(vaal.reference.clone());
            sub.discriminator_trade = item.trade_discriminator.clone();
            sub.disabled = Some(false);
            p.search_exact.sub = Some(Box::new(sub));
        }
        p.corrupted = Some(Corrupted {
            value: item.corrupted,
            exact: None,
        });
        if !m.imbued_gem && item.corrupted && item.gem_level.is_some_and(|v| v >= 20) {
            p.imbued_gem = Some(Toggle {
                disabled: true,
                hidden: None,
            });
        }
        let level = item
            .gem_level
            .ok_or("Niveau de gemme absent de la copie.")?;
        p.gem_level = Some(Numeric::new(level, level < gem.max_level));
        if let Some(q) = item.quality.filter(|v| *v != 0) {
            p.quality = Some(Numeric::new(
                q,
                if gem.max_level == 1 {
                    false
                } else if gem.max_level == 20 && !gem.transfigured {
                    q < 16
                } else {
                    q < 20
                },
            ));
        }
        return Ok(p);
    }
    if c == "Captured Beast" {
        p.search_exact = base(&m.name);
        return Ok(p);
    }
    if item.stack_size.is_some_and(|v| v != 0) || item.trade_tag.is_some() || m.exchangeable {
        p.stack_size = Some(Numeric::new(
            item.stack_size.filter(|v| *v != 0).unwrap_or(1),
            !(item.stack_size.is_some_and(|v| v > 1) && opts.activate_stock_filter),
        ));
    }
    if c == "Invitation" {
        p.search_exact = base(&m.name);
        return Ok(p);
    }
    if c == "Metamorph Sample" {
        p.search_exact = base(&m.name);
        p.item_level = item.item_level.map(|v| Numeric::new(v, false));
        return Ok(p);
    }
    if reference == "Scrying Orb" {
        p.search_exact = base(&m.name);
        p.search_exact.sub = Some(Box::new(area_search(item)?));
        return Ok(p);
    }
    if ["Divination Card", "Currency"].contains(&c) || reference == "Charged Compass" {
        p.search_exact = base(&m.name);
        if reference == "Chronicle of Atzoatl" {
            p.area_level = item
                .area
                .level
                .map(|v| Numeric::new(floor_bracket(v, &[1, 68, 73, 75, 78, 80]), false));
        } else if ["Mirrored Tablet", "Forbidden Tome"].contains(&reference) {
            p.area_level = item.area.level.map(|v| Numeric {
                value: v,
                max: (reference == "Forbidden Tome" && v < 83).then_some(v),
                disabled: false,
            });
        } else if let Some(v) = item.item_level.filter(|v| *v != 0) {
            p.item_level = Some(Numeric::new(v, false));
        }
        return Ok(p);
    }
    if c == "Map" {
        if ["Blighted Map", "Blight-ravaged Map"].contains(&reference) {
            p.search_exact = base(&m.name);
            p.search_exact.base_type_trade = Some("Map".into());
            p.search_exact.discriminator_trade = Some("map".into());
            p.search_exact.sub = Some(Box::new(area_search(item)?));
        } else if *r == Rarity::Unique && m.unique_base.is_some() {
            p.search_exact = unique_search(item);
        } else {
            p.search_exact = base(&m.name);
        }
        if reference == "Map" || m.unique_base.as_deref() == Some("Map") {
            p.search_exact.discriminator_trade = Some("map".into());
        }
        p.map_blighted = match reference {
            "Map" => Some(Blighted {
                value: json!(false),
            }),
            "Blighted Map" => Some(Blighted {
                value: json!("Blighted"),
            }),
            "Blight-ravaged Map" => Some(Blighted {
                value: json!("Blight-ravaged"),
            }),
            _ => None,
        };
        p.map_completion_reward = item.area.completion_reward.as_ref().map(|name| Named {
            name: name.clone(),
            name_trade: name.clone(),
        });
        p.map_tier = item
            .map_tier
            .filter(|v| *v != 0)
            .map(|v| Numeric::new(v, false));
    } else if reference == "Expedition Logbook" {
        p.search_exact = base(&m.name);
        p.area_level = item
            .area
            .level
            .map(|v| Numeric::new(floor_bracket(v, &[1, 68, 73, 78, 81, 83]), false));
    } else if c == "Chart" {
        p.search_exact = base(&m.name);
        p.search_relaxed = Some(Search {
            category: Some(c.into()),
            disabled: Some(false),
            sub: Some(Box::new(area_search(item)?)),
            ..Default::default()
        });
    } else if *r == Rarity::Unique && m.unique_base.is_some() {
        p.search_exact = unique_search(item);
    } else {
        p.search_exact = base(&m.name);
        if CATEGORIES.iter().any(|(name, _)| *name == c) {
            let disabled = if ["Cluster Jewel", "Idol", "Graft", "Heist Blueprint"].contains(&c) {
                true
            } else if ["Sanctum Relic", "Charm", "Heist Contract"].contains(&c) {
                false
            } else {
                opts.exact
            };
            p.search_relaxed = Some(Search {
                category: Some(c.into()),
                disabled: Some(disabled),
                ..Default::default()
            });
        }
    }
    p.sentinel_charge = m.sentinel_charge.map(|v| Numeric::new(v, false));
    if let Some(q) = item
        .quality
        .filter(|q| *q >= 20 && !["Amulet", "Ring", "Belt"].contains(&c))
    {
        let no_quality = item.modifiers.iter().any(|m| {
            [
                "Quality does not increase Defences",
                "Quality does not increase Physical Damage",
            ]
            .contains(&m.reference.as_str())
        });
        if ["Flask", "Tincture"].contains(&c) || no_quality || opts.exact {
            p.quality = Some(Numeric::new(q, q <= 20));
        }
    }
    p.linked_sockets = m
        .linked_sockets
        .filter(|v| *v != 0)
        .map(|v| Numeric::new(v, false));
    let adorned = *r == Rarity::Magic && ["Jewel", "Abyss Jewel"].contains(&c);
    if !item.unmodifiable && (m.craftable || *r == Rarity::Unique) {
        p.corrupted = Some(Corrupted {
            value: item.corrupted,
            exact: Some(adorned),
        });
    }
    if adorned {
        p.rarity = Some(Choice {
            value: "magic".into(),
            disabled: false,
        });
    } else if opts.exact
        && *r == Rarity::Magic
        && !CONSUMABLE.contains(&c)
        && !MAGIC_ONLY.contains(&c)
    {
        p.rarity = Some(Choice {
            value: "magic".into(),
            disabled: true,
        });
    } else if m.craftable && [Rarity::Normal, Rarity::Magic, Rarity::Rare].contains(r) {
        p.rarity = Some(Choice {
            value: "nonunique".into(),
            disabled: false,
        });
    }
    if item.mirrored {
        p.mirrored = Some(Toggle {
            disabled: false,
            hidden: Some(false),
        });
    } else if m.craftable && !item.corrupted {
        p.mirrored = Some(Toggle {
            disabled: true,
            hidden: Some(true),
        });
    }
    if m.split {
        p.split = Some(Toggle {
            disabled: false,
            hidden: Some(false),
        });
    } else if (!["Standard", "標準模式"].contains(&opts.league.as_str()) || opts.exact)
        && m.craftable
        && !item.corrupted
        && !item.mirrored
        && !m.synthesised
        && !m.fractured
        && item.influences.is_empty()
    {
        p.split = Some(Toggle {
            disabled: true,
            hidden: Some(true),
        });
    }
    if !m.fractured && m.craftable && !item.corrupted && !item.mirrored {
        p.fractured = Some(Flag { value: false });
    }
    if m.foil {
        p.foil = Some(Toggle {
            disabled: false,
            hidden: None,
        });
    }
    if !item.influences.is_empty() && item.influences.len() <= 2 {
        p.influences = Some(
            item.influences
                .iter()
                .map(|name| Choice {
                    value: name.clone(),
                    disabled: !opts.exact,
                })
                .collect(),
        );
    }
    if let Some(level) = item.item_level.filter(|v| *v != 0) {
        if *r != Rarity::Unique
            && ![
                "Map",
                "Jewel",
                "Heist Blueprint",
                "Heist Contract",
                "Chart",
                "Memory Line",
                "Sanctum Relic",
                "Charm",
                "Idol",
            ]
            .contains(&c)
            && reference != "Expedition Logbook"
        {
            p.item_level = Some(if c == "Cluster Jewel" {
                Numeric {
                    value: floor_bracket(level, &[1, 50, 68, 75, 84]),
                    max: Some(ceil_bracket(level, &[100, 74, 67, 49])),
                    disabled: !opts.exact,
                }
            } else {
                Numeric::new(
                    level.min(86),
                    !opts.exact || ["Flask", "Tincture"].contains(&c),
                )
            });
        }
        if *r == Rarity::Unique {
            if item.unidentified && reference == "Watcher's Eye" {
                p.item_level = Some(Numeric::new(level, false));
            }
            if level >= 75
                && [
                    "Agnerod",
                    "Agnerod East",
                    "Agnerod North",
                    "Agnerod South",
                    "Agnerod West",
                ]
                .contains(&reference)
            {
                p.item_level = Some(Numeric::new(
                    if level >= 82 {
                        82
                    } else if level >= 80 {
                        80
                    } else if level >= 78 {
                        78
                    } else {
                        75
                    },
                    false,
                ));
            }
        }
    }
    if item.unidentified {
        p.unidentified = Some(FlagToggle {
            value: true,
            disabled: *r != Rarity::Unique,
        });
    }
    if m.veiled {
        p.veiled = Some(Veiled {
            stat_refs: m.veiled_refs.clone(),
            disabled: false,
        });
        if *r != Rarity::Unique {
            if let Some(f) = &mut p.item_level {
                f.disabled = false;
            }
        }
    }
    if *r == Rarity::Unique {
        p.foulborn = Some(Flag { value: m.foulborn });
        p.vestigial = Some(Flag { value: m.vestigial });
    }
    if ["Heist Contract", "Heist Blueprint", "Chart"].contains(&c) && *r != Rarity::Unique {
        p.area_level = item.area.level.map(|v| Numeric::new(v, false));
    }
    Ok(p)
}
fn unique_search(item: &Item) -> Search {
    Search {
        name: Some(item.market.name.clone()),
        name_trade: Some(item.reference.clone()),
        base_type_trade: item.market.unique_base.clone(),
        ..Default::default()
    }
}
fn area_search(item: &Item) -> Result<Search, String> {
    Ok(Search {
        base_type: Some(item.map_area.clone().ok_or("Zone absente de la copie.")?),
        base_type_trade: Some(
            item.area
                .trade_disc
                .clone()
                .ok_or("Zone inconnue de la copie.")?,
        ),
        discriminator_trade: item.trade_discriminator.clone(),
        disabled: Some(false),
        ..Default::default()
    })
}

impl Plan {
    pub fn active_search(&self) -> &Search {
        let search = self
            .search_relaxed
            .as_ref()
            .filter(|s| s.disabled != Some(true))
            .unwrap_or(&self.search_exact);
        search
            .sub
            .as_deref()
            .filter(|s| s.disabled != Some(true))
            .unwrap_or(search)
    }
    pub fn append_query(&self, q: &mut Value) -> Result<(), String> {
        self.validate()?;
        let t = &self.trade;
        q["status"] = json!({"option":if t.offline {"any"}else if t.merchant_only {"securable"}else{"available"}});
        if let Some(currency) = &t.currency {
            q["filters"]["trade_filters"]["filters"]["price"] = json!({"option":currency});
        }
        if t.collapse_listings == "api" && (t.offline || !t.merchant_only || t.collapse_merchant) {
            q["filters"]["trade_filters"]["filters"]["collapse"] = json!({"option":"true"});
        }
        if let Some(listed) = &t.listed {
            q["filters"]["trade_filters"]["filters"]["indexed"] = json!({"option":listed});
        }
        let search = self.active_search();
        for (key, value) in [
            ("name", search.name_trade.as_ref().or(search.name.as_ref())),
            (
                "type",
                search
                    .base_type_trade
                    .as_ref()
                    .or(search.base_type.as_ref()),
            ),
        ] {
            if let Some(value) = value {
                q[key] = if let Some(disc) = &search.discriminator_trade {
                    json!({"option":value,"discriminator":disc})
                } else {
                    json!(value)
                };
            }
        }
        if self.foil.as_ref().is_some_and(|f| !f.disabled) {
            q["filters"]["type_filters"]["filters"]["rarity"] = json!({"option":"uniquefoil"});
        } else if let Some(rarity) = &self.rarity {
            q["filters"]["type_filters"]["filters"]["rarity"] =
                json!({"option":if rarity.disabled {"nonunique"}else{&rarity.value}});
        }
        if let Some(category) = &search.category {
            let id = CATEGORIES
                .iter()
                .find(|(name, _)| *name == category)
                .map(|(_, id)| *id)
                .ok_or("Catégorie de recherche inconnue.")?;
            q["filters"]["type_filters"]["filters"]["category"] = json!({"option":id});
        }
        if let Some(c) = &self.corrupted {
            if !c.value || c.exact == Some(true) {
                q["filters"]["misc_filters"]["filters"]["corrupted"] =
                    json!({"option":c.value.to_string()});
            }
        }
        for (key, exclude) in [
            (
                "fractured_item",
                self.fractured.as_ref().is_some_and(|f| !f.value),
            ),
            (
                "gem_imbued",
                self.imbued_gem.as_ref().is_some_and(|f| f.disabled),
            ),
            ("split", self.split.as_ref().is_some_and(|f| f.disabled)),
            (
                "foulborn_item",
                self.foulborn.as_ref().is_some_and(|f| !f.value),
            ),
            (
                "vestigial",
                self.vestigial.as_ref().is_some_and(|f| !f.value),
            ),
            (
                "mirrored",
                self.mirrored.as_ref().is_some_and(|f| f.disabled),
            ),
        ] {
            if exclude {
                q["filters"]["misc_filters"]["filters"][key] = json!({"option":"false"});
            }
        }
        for (field, group, key, range) in [
            (&self.gem_level, "misc_filters", "gem_level", false),
            (&self.quality, "misc_filters", "quality", false),
            (&self.item_level, "misc_filters", "ilvl", true),
            (&self.stack_size, "misc_filters", "stack_size", false),
            (&self.linked_sockets, "socket_filters", "links", false),
            (&self.area_level, "map_filters", "area_level", true),
            (
                &self.sentinel_charge,
                "sentinel_filters",
                "sentinel_durability",
                false,
            ),
        ] {
            if let Some(f) = field.as_ref().filter(|f| !f.disabled) {
                let mut value = json!({"min":f.value});
                if range {
                    if let Some(max) = f.max.filter(|v| *v != 0) {
                        value["max"] = json!(max);
                    }
                }
                q["filters"][group]["filters"][key] = value;
            }
        }
        if let Some(f) = self.white_sockets.as_ref().filter(|f| !f.disabled) {
            q["filters"]["socket_filters"]["filters"]["sockets"] = json!({"w":f.value});
        }
        if let Some(f) = self.map_tier.as_ref().filter(|f| !f.disabled) {
            q["filters"]["map_filters"]["filters"]["map_tier"] =
                json!({"min":f.value,"max":f.value});
        }
        if let Some(f) = &self.map_blighted {
            for (key, value) in match f.value.as_str() {
                Some("Blighted") => vec![("map_blighted", true)],
                Some("Blight-ravaged") => vec![("map_uberblighted", true)],
                _ if f.value == json!(false) => {
                    vec![("map_blighted", false), ("map_uberblighted", false)]
                }
                _ => return Err("Variante de carte inconnue.".into()),
            } {
                q["filters"]["map_filters"]["filters"][key] = json!({"option":value.to_string()});
            }
        }
        if let Some(f) = &self.map_completion_reward {
            q["filters"]["map_filters"]["filters"]["map_completion_reward"] =
                json!({"option":f.name_trade});
        }
        if self.unidentified.as_ref().is_some_and(|f| !f.disabled) {
            q["filters"]["misc_filters"]["filters"]["identified"] = json!({"option":"false"});
        }
        let and = q["stats"][0]["filters"]
            .as_array_mut()
            .ok_or("Groupe de filtres absent.")?;
        if let Some(veiled) = &self.veiled {
            for reference in &veiled.stat_refs {
                let m = crate::modifiers::translation::exclusion(reference, "veiled", None, false)
                    .ok_or("Mod voilé inconnu.")?;
                let values: Vec<_> = m
                    .ids
                    .iter()
                    .map(|id| json!({"id":id,"value":{},"disabled":veiled.disabled}))
                    .collect();
                and.extend(values);
            }
        }
        if let Some(influences) = &self.influences {
            for f in influences {
                let m =
                    crate::modifiers::translation::pseudo(&format!("Has {} Influence", f.value))
                        .ok_or("Influence inconnue.")?;
                and.push(json!({"id":m.id,"value":{},"disabled":f.disabled}));
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        let s = self.active_search();
        if [
            s.name.as_ref(),
            s.name_trade.as_ref(),
            s.base_type.as_ref(),
            s.base_type_trade.as_ref(),
            s.category.as_ref(),
        ]
        .into_iter()
        .all(|v| v.is_none_or(|s| s.trim().is_empty()))
        {
            return Err("Aucune identité d’objet sélectionnée.".into());
        }
        for numeric in [&self.item_level, &self.area_level]
            .into_iter()
            .flatten()
            .filter(|f| !f.disabled)
        {
            if numeric
                .max
                .is_some_and(|max| max != 0 && numeric.value > max)
            {
                return Err("Niveau minimum supérieur au maximum.".into());
            }
        }
        if !["api", "app"].contains(&self.trade.collapse_listings.as_str()) {
            return Err("Mode de regroupement inconnu.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gem_thresholds_do_not_overrestrict_a_leveling_gem() {
        let text="Item Class: Skill Gems\nRarity: Gem\nFireball\n--------\nFire, Spell\nLevel: 19\nQuality: +15%\n--------";
        let item = Item::parse(text).unwrap();
        let p = create(&item, &Options::default()).unwrap();
        assert!(p.gem_level.as_ref().unwrap().disabled);
        assert!(p.quality.as_ref().unwrap().disabled);
        let mut q = json!({"filters":{},"stats":[{"type":"and","filters":[]}]});
        p.append_query(&mut q).unwrap();
        assert!(q["filters"]["misc_filters"]["filters"]
            .get("gem_level")
            .is_none());
        assert!(q["filters"]["misc_filters"]["filters"]
            .get("quality")
            .is_none());
        let item = Item::parse(
            &text
                .replace("Level: 19", "Level: 20")
                .replace("+15%", "+16%"),
        )
        .unwrap();
        let p = create(&item, &Options::default()).unwrap();
        assert!(!p.gem_level.unwrap().disabled);
        assert!(!p.quality.unwrap().disabled);
    }
    #[test]
    fn derived_plan_is_never_persisted_as_a_global_filter() {
        let item = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let plan = create(
            &item,
            &Options {
                exact: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(plan.item_level.as_ref().unwrap().value, 1);
        let filters = crate::filters::Filters {
            item: Some(plan),
            ..Default::default()
        };
        let json = serde_json::to_value(&filters).unwrap();
        assert!(json.get("item").is_none());
        let reloaded: crate::filters::Filters = serde_json::from_value(json).unwrap();
        assert!(reloaded.item.is_none());
        assert!(reloaded.min_item_level.is_none());
    }
    #[test]
    fn missing_identity_or_reversed_range_never_reaches_the_service() {
        let item = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let mut p = create(
            &item,
            &Options {
                exact: true,
                ..Default::default()
            },
        )
        .unwrap();
        p.item_level = Some(Numeric {
            value: 80,
            max: Some(79),
            disabled: false,
        });
        assert!(p.validate().is_err());
        p.item_level.as_mut().unwrap().disabled = true;
        assert!(p.validate().is_ok());
        p.search_exact = Search::default();
        p.search_relaxed = None;
        assert!(p.validate().is_err());
    }
    #[test]
    fn quality_changing_enchant_and_magic_jewel_keep_their_source_policy() {
        let mut item=Item::parse("Item Class: Wands\nRarity: Rare\nFixture\nDriftwood Wand\n--------\nQuality: +30%\nPhysical Damage: 5-9\nAttacks per Second: 1.50\n--------\nItem Level: 83\n--------\nQuality does not increase Physical Damage (enchant)").unwrap();
        let p = create(&item, &Options::default()).unwrap();
        assert!(!p.quality.unwrap().disabled);
        item=Item::parse("Item Class: Jewels\nRarity: Magic\nCobalt Jewel\n--------\nItem Level: 83\n--------\nCorrupted").unwrap();
        let p = create(&item, &Options::default()).unwrap();
        assert_eq!(p.rarity.unwrap().value, "magic");
        assert_eq!(p.corrupted.unwrap().exact, Some(true));
    }
}
