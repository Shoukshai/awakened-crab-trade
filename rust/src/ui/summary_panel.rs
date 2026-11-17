use crate::{
    economy::Report,
    item::{Family, Item},
};
use gtk::prelude::*;
use std::{cell::RefCell, path::Path};
pub struct Panel {
    pub container: gtk::Box,
    pub open: gtk::Button,
    image: gtk::Image,
    currency: gtk::Image,
    kind: gtk::Label,
    price: gtk::Label,
    trend: gtk::Label,
    stack: gtk::Label,
    url: RefCell<Option<String>>,
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
        line.pack_start(&image, false, false, 0);
        line.pack_start(&label("→", "muted"), false, false, 0);
        line.pack_start(&price, false, false, 0);
        line.pack_start(&currency, false, false, 0);
        line.pack_start(&trend, true, true, 0);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let stack = label("", "muted");
        let open = gtk::Button::with_label("poe.ninja");
        open.style_context().add_class("secondary");
        open.set_sensitive(false);
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
            currency,
            kind,
            price,
            trend,
            stack,
            url: RefCell::new(None),
        }
    }
    pub fn reset(&self, item: &Item) {
        self.image.clear();
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
        self.container.show();
    }
    pub fn image(&self, path: &Path) {
        if let Ok(p) = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(path, 48, 56, true) {
            self.image.set_from_pixbuf(Some(&p));
        }
    }
    pub fn show(&self, result: &Result<Report, String>) {
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
                    self.trend.set_text(
                        &q.graph
                            .last()
                            .copied()
                            .flatten()
                            .map(|v| format!("{v:+.1}% · 7 j"))
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
    }
    pub fn url(&self) -> Option<String> {
        self.url.borrow().clone()
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.container.is_visible(),"mapped":self.container.is_mapped(),"height":self.container.allocated_height(),"kind":self.kind.text().as_str(),"price":self.price.text().as_str(),"image_ready":self.image.pixbuf().is_some(),"source":self.url.borrow().clone()})
    }
}
