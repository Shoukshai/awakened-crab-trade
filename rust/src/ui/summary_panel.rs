use crate::{
    economy::Report,
    item::{Family, Item},
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    path::Path,
};
pub struct Panel {
    pub container: gtk::Box,
    pub open: gtk::Button,
    image: gtk::Image,
    image_source: RefCell<Option<gtk::gdk_pixbuf::Pixbuf>>,
    quote_line: gtk::Box,
    quote: gtk::Box,
    footer: gtk::Box,
    trend_source: RefCell<String>,
    currency: gtk::Image,
    kind: gtk::Label,
    price: gtk::Label,
    trend: gtk::Label,
    stack: gtk::Label,
    url: RefCell<Option<String>>,
    classic: Cell<bool>,
    equipment: Cell<bool>,
    has_quote: Cell<bool>,
}
fn label(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.);
    l.style_context().add_class(class);
    let a = gtk::pango::AttrList::new();
    a.insert(gtk::pango::AttrFontFeatures::new("tnum=1"));
    l.set_attributes(Some(&a));
    l
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let kind = label("", "muted");
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let image = gtk::Image::new();
        image.set_size_request(48, 56);
        let price = label("—", "summary-price");
        let currency = gtk::Image::new();
        currency.set_no_show_all(true);
        let trend = label("", "muted");
        trend.set_xalign(1.);
        let quote = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        quote.pack_start(&image, false, false, 0);
        quote.pack_start(&label("→", "muted"), false, false, 0);
        quote.pack_start(&price, false, false, 0);
        quote.pack_start(&currency, false, false, 0);
        line.pack_start(&quote, false, false, 0);
        let trend_target = gtk::EventBox::new();
        trend_target.set_visible_window(false);
        trend_target.add(&trend);
        line.pack_start(&trend_target, true, true, 0);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let stack = label("", "muted");
        let open = gtk::Button::with_label("poe.ninja");
        open.style_context().add_class("secondary");
        open.set_sensitive(false);
        let link = open.downgrade();
        trend_target.connect_button_press_event(move |_, event| {
            if event.button() == 1 {
                if let Some(link) = link.upgrade().filter(|b| b.is_sensitive()) {
                    link.emit_clicked();
                }
            }
            glib::Propagation::Proceed
        });
        trend_target.set_tooltip_text(Some("poe.ninja"));
        footer.pack_start(&stack, true, true, 0);
        footer.pack_start(&open, false, false, 0);
        container.pack_start(&kind, false, false, 0);
        container.pack_start(&line, false, false, 0);
        container.pack_start(&footer, false, false, 0);
        container.show_all();
        container.set_no_show_all(true);
        container.hide();
        Self {
            container,
            open,
            image,
            image_source: RefCell::new(None),
            quote_line: line,
            quote,
            footer,
            trend_source: RefCell::new(String::new()),
            currency,
            kind,
            price,
            trend,
            stack,
            url: RefCell::new(None),
            classic: Cell::new(false),
            equipment: Cell::new(false),
            has_quote: Cell::new(false),
        }
    }
    pub fn reset(&self, item: &Item) {
        self.equipment.set(item.family() == Family::Equipment);
        self.has_quote.set(false);
        self.image.clear();
        self.image_source.replace(None);
        self.trend_source.replace(String::new());
        self.currency.hide();
        self.price.set_text("…");
        self.trend.set_text("");
        self.stack.set_text("");
        self.url.replace(None);
        self.open.set_sensitive(false);
        let category = match item.family() {
            Family::Currency => "Monnaie",
            Family::Gem => "Gemme",
            Family::Map => "Carte",
            Family::Unique => "Unique",
            Family::Viscera => "Viscera",
            Family::Wombgift => "Wombgift",
            Family::Equipment => "Base seule",
            Family::Other => item.category.as_deref().unwrap_or("Objet"),
        };
        let mut kind = category.to_string();
        if item.family() == Family::Gem {
            kind += &format!(
                " · niveau {} · qualité {}%",
                item.gem_level.unwrap_or(1),
                item.quality.unwrap_or(0)
            );
        }
        if item.family() == Family::Map {
            kind += &format!(" · T{}", item.map_tier.unwrap_or(0));
        }
        if let Some(slot) = &item.market.viscera_slot {
            kind += &format!(
                " · {}",
                match slot.as_str() {
                    "Head" => "Tête",
                    "Arm" => "Bras",
                    "Body" => "Corps",
                    "Leg" => "Jambe",
                    _ => slot,
                }
            );
        }
        if item.corrupted {
            kind += " · Corrompu";
        }
        self.kind.set_text(&kind);
        self.refresh_visibility();
    }
    pub fn image(&self, path: &Path) {
        if let Ok(p) = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(path, 48, 56, true) {
            self.image_source.replace(Some(p));
            self.refresh_style();
        }
    }
    pub fn show(&self, result: &Result<Report, String>) {
        self.has_quote
            .set(result.as_ref().is_ok_and(|r| r.price.is_some()));
        match result {
            Ok(report) => {
                if let Some(price) = &report.price {
                    let full = super::economy_panel::money(price);
                    self.price.set_text(
                        full.rsplit_once(' ')
                            .map(|(value, _)| value)
                            .unwrap_or(&full),
                    );
                    self.price.set_tooltip_text(Some(&full));
                    self.currency
                        .set_from_pixbuf(super::icons::currency(&price.currency).as_ref());
                    self.currency
                        .set_tooltip_text(Some(if price.currency == "div" {
                            "Divine Orb"
                        } else {
                            "Chaos Orb"
                        }));
                    self.currency.show();
                } else {
                    self.price.set_text("—");
                }
                if let Some(q) = &report.quote {
                    self.url.replace(Some(q.url.clone()));
                    self.open.set_sensitive(true);
                    self.trend_source.replace(
                        q.graph
                            .last()
                            .copied()
                            .flatten()
                            .map(|v| format!("{v:+.1}%"))
                            .unwrap_or_default(),
                    );
                }
                self.stack.set_text(
                    &report
                        .stack
                        .as_ref()
                        .filter(|s| s.have > 1)
                        .map(|s| format!("×{} · {}", s.have, super::economy_panel::money(&s.price)))
                        .unwrap_or_default(),
                );
                self.container.set_tooltip_text(report.warning.as_deref());
            }
            Err(error) => {
                self.price.set_text("—");
                self.container.set_tooltip_text(Some(error));
            }
        }
        self.refresh_visibility();
    }
    pub fn set_theme(&self, theme: crate::appearance::Theme) {
        self.classic
            .set(theme == crate::appearance::Theme::Awakened);
        self.refresh_visibility();
    }
    fn refresh_style(&self) {
        let classic = self.classic.get();
        let price = self.price.text().trim_end_matches(" ×").to_owned();
        self.price.set_text(&if classic && self.has_quote.get() {
            format!("{price} ×")
        } else {
            price
        });
        self.container.set_spacing(if classic { 0 } else { 8 });
        self.kind.set_no_show_all(classic);
        self.kind.set_visible(!classic);
        self.quote.set_halign(if classic {
            gtk::Align::Center
        } else {
            gtk::Align::Start
        });
        self.quote_line
            .set_child_packing(&self.quote, classic, classic, 0, gtk::PackType::Start);
        if let Some(trend_target) = self.trend.parent() {
            self.quote_line.set_child_packing(
                &trend_target,
                !classic,
                !classic,
                0,
                gtk::PackType::Start,
            );
        }
        self.quote_line
            .set_height_request(if classic { 48 } else { -1 });
        self.image
            .set_size_request(if classic { 32 } else { 48 }, if classic { 32 } else { 56 });
        if let Some(source) = self.image_source.borrow().as_ref() {
            let p = if classic {
                source.scale_simple(
                    (source.width() as f64 * (32. / source.width().max(source.height()) as f64))
                        .round()
                        .max(1.) as i32,
                    (source.height() as f64 * (32. / source.width().max(source.height()) as f64))
                        .round()
                        .max(1.) as i32,
                    gtk::gdk_pixbuf::InterpType::Bilinear,
                )
            } else {
                Some(source.clone())
            };
            self.image.set_from_pixbuf(p.as_ref());
        }
        self.open.set_no_show_all(classic);
        self.open.set_visible(!classic);
        let footer = !classic || !self.stack.text().is_empty();
        self.footer.set_no_show_all(!footer);
        self.footer.set_visible(footer);
        let trend = self.trend_source.borrow();
        self.trend.set_text(&if trend.is_empty() {
            String::new()
        } else if classic {
            format!("{trend}\n{}", crate::i18n::tr("7 derniers jours"))
        } else {
            format!("{trend} · 7 j")
        });
    }
    fn refresh_visibility(&self) {
        self.refresh_style();
        self.container
            .set_visible(!self.classic.get() || !self.equipment.get() || self.has_quote.get());
    }
    pub fn url(&self) -> Option<String> {
        self.url.borrow().clone()
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.container.is_visible(),"mapped":self.container.is_mapped(),"height":self.container.allocated_height(),"kind":self.kind.text().as_str(),"price":self.price.text().as_str(),"image_ready":self.image.pixbuf().is_some(),"source":self.url.borrow().clone()})
    }
}
