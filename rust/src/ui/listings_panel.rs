use crate::{
    trade::{Listing, Prices},
    trade_item::OfferedItem,
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct OfferRow {
    widget: gtk::EventBox,
    body: gtk::Box,
    identity: gtk::Box,
    classic_cells: gtk::Box,
    classic_price: String,
    normal_price: String,
    price: gtk::Label,
    tooltip: Option<String>,
    popover: Option<gtk::Popover>,
}
#[derive(Clone, Copy, Default)]
struct Columns {
    gem: bool,
    quality: bool,
    ilvl: bool,
}
fn column(text: &str, width: i32, class: &str) -> gtk::Label {
    let l = label(text, class);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    l.set_size_request(width, -1);
    l.set_max_width_chars(if width > 0 { 8 } else { 20 });
    l
}
pub struct Panel {
    pub container: gtk::Grid,
    rows: RefCell<Vec<OfferRow>>,
    classic_header: gtk::Box,
    tooltip_queries: Rc<Cell<u64>>,
}
fn label(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.);
    l.style_context().add_class(class);
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrFontFeatures::new("tnum=1"));
    l.set_attributes(Some(&attrs));
    l
}
fn price_text(listing: &Listing) -> String {
    let currency = crate::filters::PAYMENT_CURRENCIES
        .iter()
        .find(|(id, _)| *id == listing.currency)
        .map(|(_, s)| *s)
        .unwrap_or(&listing.currency);
    format!(
        "{} {}",
        super::economy_panel::format_amount(listing.amount),
        currency
    )
}
fn rarity_class(item: &OfferedItem) -> &'static str {
    match item.rarity {
        Some(1) => "rarity-magic",
        Some(2) => "rarity-rare",
        Some(3 | 9) => "rarity-unique",
        Some(4) => "rarity-gem",
        Some(5) => "rarity-currency",
        _ => "rarity-normal",
    }
}
fn details(item: &OfferedItem, price: &str) -> (gtk::Box, String) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_border_width(12);
    let mut plain = Vec::new();
    for text in [&item.name, &item.base]
        .into_iter()
        .filter(|s| !s.is_empty())
        .enumerate()
        .filter(|(i, s)| *i == 0 || **s != item.name)
        .map(|(_, s)| s)
    {
        let l = label(text, rarity_class(item));
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        l.set_max_width_chars(40);
        body.pack_start(&l, false, false, 0);
        plain.push(text.clone());
    }
    let meta = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    if let Some(level) = item.gem_level {
        let text = format!("Niv. {level} · Q. {}%", item.quality.unwrap_or(0));
        meta.pack_start(&label(&text, "muted"), false, false, 0);
        plain.push(text);
    } else if let Some(tier) = item.map_tier {
        let text = format!("T{tier} · Q. {}%", item.quality.unwrap_or(0));
        meta.pack_start(&label(&text, "muted"), false, false, 0);
        plain.push(text);
    } else if let Some(level) = item.item_level {
        let text = format!("ilvl {level}");
        meta.pack_start(&label(&text, "muted"), false, false, 0);
        plain.push(text);
    }
    if item.corrupted {
        meta.pack_start(&label("Corrompu", "corrupted"), false, false, 0);
        plain.push("Corrompu".into());
    }
    body.pack_start(&meta, false, false, 0);
    let implicit: Vec<_> = item
        .mods
        .iter()
        .filter(|m| m.kind == "implicitMods")
        .flat_map(|m| m.lines.iter())
        .collect();
    let explicit: Vec<_> = item
        .mods
        .iter()
        .filter(|m| m.kind != "implicitMods")
        .flat_map(|m| m.lines.iter())
        .collect();
    for (index, group) in [&implicit, &explicit].into_iter().enumerate() {
        if index == 1 && !implicit.is_empty() && !explicit.is_empty() {
            body.pack_start(
                &gtk::Separator::new(gtk::Orientation::Horizontal),
                false,
                false,
                0,
            );
            plain.push("────────".into());
        }
        for text in group {
            let l = label(text, "mod");
            l.set_line_wrap(true);
            l.set_line_wrap_mode(gtk::pango::WrapMode::WordChar);
            l.set_max_width_chars(40);
            body.pack_start(&l, false, false, 0);
            plain.push((*text).clone());
        }
    }
    let value = label(price, "price");
    value.set_xalign(1.);
    body.pack_start(&value, false, false, 0);
    plain.push(price.into());
    (body, plain.join("\n"))
}
fn rect(w: &impl IsA<gtk::Widget>, window: &gtk::Window) -> Option<serde_json::Value> {
    w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Grid::new();
        container.set_row_spacing(4);
        container.set_no_show_all(true);
        Self {
            container,
            rows: RefCell::new(Vec::new()),
            classic_header: {
                let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                header.style_context().add_class("trade-heading");
                header.set_no_show_all(true);
                header
            },
            tooltip_queries: Rc::new(Cell::new(0)),
        }
    }
    pub fn clear(&self) {
        for row in self.rows.borrow().iter() {
            if let Some(p) = &row.popover {
                p.hide();
                p.set_relative_to(None::<&gtk::Widget>);
            }
        }
        for child in self.container.children() {
            self.container.remove(&child);
        }
        self.rows.borrow_mut().clear();
        self.container.hide();
    }
    pub fn show(
        &self,
        prices: &Prices,
        family: Option<crate::item::Family>,
        plan: Option<&crate::item_filters::Plan>,
    ) {
        self.clear();
        let cols = Columns {
            gem: family == Some(crate::item::Family::Gem),
            quality: family == Some(crate::item::Family::Gem)
                || plan.is_some_and(|p| p.quality.is_some()),
            ilvl: plan.is_some_and(|p| p.item_level.is_some()),
        };
        for child in self.classic_header.children() {
            self.classic_header.remove(&child);
        }
        self.classic_header.pack_start(
            &column(&crate::i18n::tr("Prix"), 88, "heading-price"),
            false,
            false,
            0,
        );
        if cols.ilvl {
            self.classic_header
                .pack_start(&column("ilvl", 56, "heading-ilvl"), false, false, 0);
        }
        if cols.gem {
            self.classic_header.pack_start(
                &column(&crate::i18n::tr("Niveau"), 72, "heading-level"),
                false,
                false,
                0,
            );
        }
        if cols.quality {
            self.classic_header.pack_start(
                &column(&crate::i18n::tr("Qualité"), 72, "heading-quality"),
                false,
                false,
                0,
            );
        }
        self.classic_header.pack_start(
            &column(&crate::i18n::tr("Mise en vente"), -1, "heading-listed"),
            true,
            true,
            0,
        );
        for child in self.classic_header.children() {
            child.show_all();
        }
        for (index, listing) in prices.listings.iter().enumerate() {
            let widget = gtk::EventBox::new();
            widget.set_hexpand(true);
            widget.set_above_child(true);
            widget.add_events(
                gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK,
            );
            widget.style_context().add_class("trade-row");
            if index % 2 == 0 {
                widget.style_context().add_class("even");
            }
            let body = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            body.set_border_width(8);
            let identity = gtk::Box::new(gtk::Orientation::Vertical, 4);
            identity.set_hexpand(true);
            let name = listing
                .item
                .as_ref()
                .map(|i| {
                    if i.name.is_empty() {
                        i.base.clone()
                    } else if i.base == i.name {
                        i.name.clone()
                    } else {
                        format!("{} · {}", i.name, i.base)
                    }
                })
                .unwrap_or_else(|| {
                    listing
                        .bulk
                        .as_ref()
                        .map(|b| format!("Stock {}", b.stock))
                        .unwrap_or_default()
                });
            let name = match (family, listing.item.as_ref()) {
                (Some(crate::item::Family::Gem), Some(item)) => format!(
                    "Niv. {} · Q. {}%{}",
                    item.gem_level
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "—".into()),
                    item.quality
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "—".into()),
                    if item.corrupted { " · Corrompu" } else { "" }
                ),
                (Some(crate::item::Family::Map), Some(item)) => format!(
                    "{} · T{}",
                    name,
                    item.map_tier
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "—".into())
                ),
                _ => name,
            };
            let name = label(&name, "offer-name");
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name.set_max_width_chars(28);
            let age = listing
                .indexed
                .as_ref()
                .and_then(|s| glib::DateTime::from_iso8601(s, None).ok())
                .and_then(|date| {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()?
                        .as_secs() as i64;
                    let elapsed = (now - date.to_unix()).max(0);
                    Some(if elapsed < 60 {
                        "à l’instant".into()
                    } else if elapsed < 3600 {
                        format!("{} min", elapsed / 60)
                    } else if elapsed < 86400 {
                        format!("{} h", elapsed / 3600)
                    } else {
                        format!("{} j", elapsed / 86400)
                    })
                });
            let listed = age.clone().unwrap_or_else(|| "—".into());
            let seller = label(
                &age.map(|age| format!("{} · {age}", listing.seller))
                    .unwrap_or_else(|| listing.seller.clone()),
                "seller",
            );
            seller.set_ellipsize(gtk::pango::EllipsizeMode::End);
            seller.set_max_width_chars(28);
            identity.pack_start(&name, false, false, 0);
            identity.pack_start(&seller, false, false, 0);
            body.pack_start(&identity, true, true, 0);
            let classic_cells = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            classic_cells.set_hexpand(true);
            classic_cells.set_no_show_all(true);
            let item = listing.item.as_ref();
            let number =
                |value: Option<u64>| value.map(|n| n.to_string()).unwrap_or_else(|| "—".into());
            if cols.ilvl {
                classic_cells.pack_start(
                    &column(&number(item.and_then(|i| i.item_level)), 56, "offer-ilvl"),
                    false,
                    false,
                    0,
                );
            }
            if cols.gem {
                classic_cells.pack_start(
                    &column(&number(item.and_then(|i| i.gem_level)), 72, "offer-level"),
                    false,
                    false,
                    0,
                );
            }
            if cols.quality {
                classic_cells.pack_start(
                    &column(
                        &item
                            .and_then(|i| i.quality)
                            .map(|n| format!("+{n}%"))
                            .unwrap_or_else(|| "—".into()),
                        72,
                        "offer-quality",
                    ),
                    false,
                    false,
                    0,
                );
            }
            classic_cells.pack_start(
                &column(&crate::i18n::tr(&listed), -1, "offer-listed"),
                true,
                true,
                0,
            );
            body.pack_start(&classic_cells, true, true, 0);
            let price = price_text(listing);
            let value = label(&price, "price");
            value.set_xalign(1.);
            value.set_width_chars(10);
            body.pack_start(&value, false, false, 0);
            let mut text = None;
            let popover = listing.item.as_ref().map(|item| {
                let (content, plain) = details(item, &price);
                text = Some(plain);
                let scroll =
                    gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
                scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                scroll.set_propagate_natural_height(true);
                scroll.set_propagate_natural_width(true);
                scroll.set_max_content_width(390);
                scroll.add(&content);
                content.show_all();
                super::localization::refresh(&content);
                let popover = gtk::Popover::new(Some(&widget));
                popover.set_modal(false);
                popover.set_constrain_to(gtk::PopoverConstraint::Window);
                popover.set_widget_name("apt-offer-details");
                popover.add(&scroll);
                let epoch = Rc::new(Cell::new(0u64));
                let entered = epoch.clone();
                let weak = popover.downgrade();
                let row = widget.downgrade();
                let queries = self.tooltip_queries.clone();
                widget.connect_enter_notify_event(move |_, _| {
                    entered.set(entered.get() + 1);
                    let token = entered.get();
                    let epoch = entered.clone();
                    let weak = weak.clone();
                    let row = row.clone();
                    let queries = queries.clone();
                    let scroll = scroll.clone();
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(100),
                        move || {
                            if epoch.get() != token {
                                return;
                            }
                            if let (Some(p), Some(row)) = (weak.upgrade(), row.upgrade()) {
                                if !row.is_mapped() {
                                    return;
                                }
                                if let Some(top) = row.toplevel() {
                                    if let Some((_, y)) = row.translate_coordinates(&top, 0, 0) {
                                        let above = y - 12;
                                        let below = top.allocated_height()
                                            - y
                                            - row.allocated_height()
                                            - 12;
                                        let needed = content_height(&scroll);
                                        p.set_position(if above >= needed || above >= below {
                                            gtk::PositionType::Top
                                        } else {
                                            gtk::PositionType::Bottom
                                        });
                                        scroll.set_max_content_height(
                                            (above.max(below) - 24).max(80),
                                        );
                                    }
                                }
                                p.show_all();
                                queries.set(queries.get() + 1);
                            }
                        },
                    );
                    glib::Propagation::Proceed
                });
                let weak = popover.downgrade();
                let left = epoch.clone();
                widget.connect_leave_notify_event(move |_, _| {
                    left.set(left.get() + 1);
                    if let Some(p) = weak.upgrade() {
                        p.hide();
                    }
                    glib::Propagation::Proceed
                });
                let weak = popover.downgrade();
                widget.connect_unmap(move |_| {
                    epoch.set(epoch.get() + 1);
                    if let Some(p) = weak.upgrade() {
                        p.hide();
                    }
                });
                popover
            });
            widget.add(&body);
            self.container.attach(&widget, 0, index as i32 + 1, 1, 1);
            self.rows.borrow_mut().push(OfferRow {
                widget,
                body,
                identity,
                classic_cells,
                classic_price: format!(
                    "{} {}",
                    super::economy_panel::format_amount(listing.amount),
                    listing.currency
                ),
                normal_price: price.clone(),
                price: value,
                tooltip: text,
                popover,
            });
        }
        for child in self.container.children() {
            child.show_all();
        }
        self.container.show();
    }
    pub fn set_theme(&self, theme: crate::appearance::Theme) {
        use crate::appearance::Theme;
        let classic = theme == Theme::Awakened;
        self.container.set_row_spacing(if classic { 0 } else { 4 });
        if self.classic_header.parent().is_some() {
            self.container.remove(&self.classic_header);
        }
        if classic && !self.rows.borrow().is_empty() {
            self.container.attach(&self.classic_header, 0, 0, 1, 1);
            self.classic_header.show();
        }
        for row in self.rows.borrow().iter() {
            if let Some(p) = &row.popover {
                p.hide();
            }
            let vertical = theme == Theme::Split;
            row.body.set_orientation(if vertical {
                gtk::Orientation::Vertical
            } else {
                gtk::Orientation::Horizontal
            });
            row.identity.set_no_show_all(classic);
            row.identity.set_visible(!classic);
            for child in row.classic_cells.children() {
                child.show_all();
            }
            row.classic_cells.set_visible(classic);
            row.price.set_text(if classic {
                &row.classic_price
            } else {
                &row.normal_price
            });
            row.body.set_margin_start(if classic { 8 } else { 0 });
            row.body.set_margin_end(if classic { 8 } else { 0 });
            row.identity.set_orientation(if classic {
                gtk::Orientation::Horizontal
            } else {
                gtk::Orientation::Vertical
            });
            row.identity.set_spacing(if classic { 12 } else { 4 });
            row.body.set_spacing(if classic {
                0
            } else if vertical {
                4
            } else {
                12
            });
            row.body.set_border_width(if classic {
                0
            } else if matches!(theme, Theme::Compact | Theme::Terminal) {
                4
            } else {
                8
            });
            row.body.reorder_child(
                &row.price,
                if matches!(theme, Theme::Merchant | Theme::Split | Theme::Awakened) {
                    0
                } else {
                    1
                },
            );
            row.price.set_xalign(
                if matches!(theme, Theme::Merchant | Theme::Split | Theme::Awakened) {
                    0.
                } else {
                    1.
                },
            );
            row.price
                .set_size_request(if classic { 88 } else { -1 }, -1);
            row.price.set_max_width_chars(if classic { 10 } else { -1 });
            row.price.set_width_chars(if classic || vertical {
                1
            } else if theme == Theme::Merchant {
                8
            } else {
                10
            });
            row.identity.set_hexpand(true);
        }
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rows = self.rows.borrow();
        let tooltip_visible = rows
            .iter()
            .any(|r| r.popover.as_ref().is_some_and(|p| p.is_mapped()));
        let cells = |body: &gtk::Box| {
            body.children()
                .into_iter()
                .filter_map(|w| w.downcast::<gtk::Label>().ok())
                .map(|l| serde_json::json!({"text":l.text().as_str(),"rect":rect(&l,window)}))
                .collect::<Vec<_>>()
        };
        serde_json::json!({"classic_table":{"visible":self.classic_header.is_mapped(),"headings":cells(&self.classic_header)},"visible":self.container.is_visible(),"rows":rows.len(),"mapped_rows":rows.iter().filter(|r|r.widget.is_mapped()).count(),"tooltip_queries":self.tooltip_queries.get(),"tooltip_visible":tooltip_visible,"offers":rows.iter().enumerate().map(|(i,r)|serde_json::json!({"rect":rect(&r.widget,window),"detail_rect":r.popover.as_ref().filter(|p|p.is_mapped()).and_then(|p|rect(p,window)),"classic_cells":cells(&r.classic_cells),"price_rect":rect(&r.price,window),"has_item_details":r.tooltip.is_some(),"tooltip":if i==0{r.tooltip.as_deref().map(crate::i18n::tr)}else{None}})).collect::<Vec<_>>()})
    }
}
fn content_height(scroll: &gtk::ScrolledWindow) -> i32 {
    scroll
        .child()
        .map(|w| w.preferred_height().1)
        .unwrap_or(300)
}
