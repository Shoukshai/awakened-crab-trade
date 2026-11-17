use crate::{trade::Prices, trade_item::OfferedItem};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct OfferRow {
    widget: gtk::EventBox,
    tooltip: Option<String>,
    popover: Option<gtk::Popover>,
}
pub struct Panel {
    pub container: gtk::Grid,
    rows: RefCell<Vec<OfferRow>>,
    tooltip_queries: Rc<Cell<u64>>,
}
fn tooltip_text(item: &OfferedItem) -> String {
    let mut lines = Vec::new();
    if !item.name.is_empty() {
        lines.push(item.name.clone());
    }
    if !item.base.is_empty() {
        lines.push(item.base.clone());
    }
    if let Some(level) = item.item_level {
        lines.push(format!("Niveau d’objet : {level}"));
    }
    if !item.properties.is_empty() {
        lines.push(String::new());
        lines.extend(item.properties.clone());
    }
    if !item.requirements.is_empty() {
        lines.push(String::new());
        lines.push("Prérequis".into());
        lines.extend(item.requirements.clone());
    }
    for group in &item.mods {
        let label = match group.kind.as_str() {
            "implicitMods" => "Implicites",
            "explicitMods" => "Mods",
            "craftedMods" => "Craftés",
            "fracturedMods" => "Fracturés",
            "enchantMods" => "Enchantements",
            "runeMods" => "Runes",
            "scourgeMods" => "Scourge",
            "mutatedMods" => "Mutés",
            "veiledMods" => "Voilés",
            _ => "Autres mods",
        };
        lines.push(String::new());
        lines.push(label.into());
        lines.extend(group.lines.clone());
    }
    if item.identified == Some(false) {
        lines.push("Objet non identifié".into());
    }
    if item.corrupted {
        lines.push("Corrompu".into());
    }
    lines.join("\n")
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Grid::new();
        container.set_column_spacing(12);
        container.set_column_homogeneous(true);
        container.set_row_spacing(6);
        container.set_no_show_all(true);
        Self {
            container,
            rows: RefCell::new(Vec::new()),
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
    pub fn show(&self, prices: &Prices) {
        self.clear();
        for (column, text) in ["Prix / objet", "Vendeur"].into_iter().enumerate() {
            let l = gtk::Label::new(Some(text));
            l.set_xalign(if column == 0 { 0. } else { 1. });
            self.container.attach(&l, column as i32, 0, 1, 1);
        }
        for (index, listing) in prices.listings.iter().enumerate() {
            let widget = gtk::EventBox::new();
            widget.set_hexpand(true);
            widget.set_above_child(true);
            widget.add_events(
                gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK,
            );
            widget.style_context().add_class("trade-row");
            let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
            body.set_border_width(8);
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let currency = crate::filters::PAYMENT_CURRENCIES
                .iter()
                .find(|(id, _)| *id == listing.currency)
                .map(|(_, label)| *label)
                .unwrap_or(&listing.currency);
            let price = gtk::Label::new(Some(&format!(
                "{} {}",
                super::economy_panel::format_amount(listing.amount),
                currency
            )));
            price.set_xalign(0.);
            let seller = gtk::Label::new(Some(&listing.seller));
            seller.set_xalign(1.);
            seller.set_ellipsize(gtk::pango::EllipsizeMode::End);
            seller.set_max_width_chars(20);
            line.pack_start(&price, true, true, 0);
            line.pack_start(&seller, true, true, 0);
            body.pack_start(&line, false, false, 0);
            if let Some(item) = &listing.item {
                let text = if item.name.is_empty() {
                    item.base.clone()
                } else {
                    format!("{} · {}", item.name, item.base)
                };
                let name = gtk::Label::new(Some(&text));
                name.set_xalign(0.);
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                name.set_max_width_chars(44);
                body.pack_start(&name, false, false, 0);
            } else if let Some(b) = &listing.bulk {
                let stock = gtk::Label::new(Some(&format!(
                    "Stock {} · lot {} pour {}",
                    b.stock, b.item_amount, b.exchange_amount
                )));
                stock.set_xalign(0.);
                body.pack_start(&stock, false, false, 0);
            }
            let text = listing.item.as_ref().map(tooltip_text);
            let popover = text.as_ref().map(|text| {
                let popover = gtk::Popover::new(Some(&widget));
                popover.set_modal(false);
                popover.set_position(gtk::PositionType::Left);
                popover.set_widget_name("apt-offer-details");
                let label = gtk::Label::new(Some(text));
                label.set_xalign(0.);
                label.set_line_wrap(true);
                label.set_max_width_chars(44);
                label.set_margin_start(12);
                label.set_margin_end(12);
                label.set_margin_top(10);
                label.set_margin_bottom(10);
                popover.add(&label);
                let epoch = Rc::new(Cell::new(0u64));
                let entered = epoch.clone();
                let weak = popover.downgrade();
                let queries = self.tooltip_queries.clone();
                widget.connect_enter_notify_event(move |_, _| {
                    entered.set(entered.get() + 1);
                    let token = entered.get();
                    let epoch = entered.clone();
                    let weak = weak.clone();
                    let queries = queries.clone();
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(120),
                        move || {
                            if epoch.get() == token {
                                if let Some(p) = weak.upgrade() {
                                    p.show_all();
                                    queries.set(queries.get() + 1);
                                }
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
            self.container.attach(&widget, 0, index as i32 + 1, 2, 1);
            self.rows.borrow_mut().push(OfferRow {
                widget,
                tooltip: text,
                popover,
            });
        }
        for child in self.container.children() {
            child.show_all();
        }
        self.container.show();
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rows = self.rows.borrow();
        let tooltip_visible = rows
            .iter()
            .any(|r| r.popover.as_ref().is_some_and(|p| p.is_mapped()));
        serde_json::json!({"visible":self.container.is_visible(),"rows":rows.len(),"mapped_rows":rows.iter().filter(|r|r.widget.is_mapped()).count(),"tooltip_queries":self.tooltip_queries.get(),"tooltip_visible":tooltip_visible,"offers":rows.iter().enumerate().map(|(i,r)|serde_json::json!({"rect":r.widget.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":r.widget.allocated_width(),"height":r.widget.allocated_height(),"mapped":r.widget.is_mapped()})),"has_item_details":r.tooltip.is_some(),"tooltip":if i==0{r.tooltip.as_deref()}else{None}})).collect::<Vec<_>>()})
    }
}
