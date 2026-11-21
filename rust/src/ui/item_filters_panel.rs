//! Native per-item editor. Programmatic resets suppress change events; edits
//! only invalidate the current result and never start HTTP or request focus.
use crate::item_filters::{self, Numeric, Plan};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Debug, Clone, Copy)]
enum Number {
    ItemLevel,
    Quality,
    GemLevel,
    Links,
    White,
    Stack,
    MapTier,
    AreaLevel,
    Charge,
}
impl Number {
    fn label(self) -> &'static str {
        match self {
            Self::ItemLevel => "ilvl minimum",
            Self::Quality => "Qualité",
            Self::GemLevel => "Niveau",
            Self::Links => "Liens",
            Self::White => "Sockets blancs minimum",
            Self::Stack => "Stock minimum",
            Self::MapTier => "Tier",
            Self::AreaLevel => "Zone minimum",
            Self::Charge => "Charges minimum",
        }
    }
    fn get(self, p: &Plan) -> &Option<Numeric> {
        match self {
            Self::ItemLevel => &p.item_level,
            Self::Quality => &p.quality,
            Self::GemLevel => &p.gem_level,
            Self::Links => &p.linked_sockets,
            Self::White => &p.white_sockets,
            Self::Stack => &p.stack_size,
            Self::MapTier => &p.map_tier,
            Self::AreaLevel => &p.area_level,
            Self::Charge => &p.sentinel_charge,
        }
    }
    fn get_mut(self, p: &mut Plan) -> &mut Option<Numeric> {
        match self {
            Self::ItemLevel => &mut p.item_level,
            Self::Quality => &mut p.quality,
            Self::GemLevel => &mut p.gem_level,
            Self::Links => &mut p.linked_sockets,
            Self::White => &mut p.white_sockets,
            Self::Stack => &mut p.stack_size,
            Self::MapTier => &mut p.map_tier,
            Self::AreaLevel => &mut p.area_level,
            Self::Charge => &mut p.sentinel_charge,
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum Flag {
    Fractured,
    Imbued,
    Mirrored,
    Split,
    Foulborn,
    Vestigial,
    Unidentified,
    Veiled,
    Influence(usize),
}
struct NumberRow {
    kind: Number,
    group: gtk::Box,
    enabled: gtk::ToggleButton,
    min: gtk::SpinButton,
    value_entry: Option<gtk::Entry>,
    classic_value: Option<gtk::Entry>,
    exact: Option<gtk::CheckButton>,
    max_enabled: Option<gtk::CheckButton>,
    max: Option<gtk::SpinButton>,
}
pub struct Panel {
    pub container: gtk::Box,
    pub classic_name: gtk::ToggleButton,
    pub classic_corruption: gtk::Button,
    exact_name: RefCell<String>,
    relaxed_name: RefCell<String>,
    summary: gtk::Label,
    pub expander: gtk::Expander,
    fields: gtk::Box,
    quick: gtk::Box,
    classic_chips: gtk::FlowBox,
    classic_advanced: gtk::Box,
    quick_order: RefCell<Vec<gtk::Widget>>,
    theme: Cell<crate::appearance::Theme>,
    model: RefCell<Option<Plan>>,
    numbers: RefCell<Vec<NumberRow>>,
    flags: RefCell<Vec<(Flag, gtk::CheckButton)>>,
    scope: RefCell<Option<gtk::ToggleButton>>,
    sub: RefCell<Option<gtk::CheckButton>>,
    corruption: RefCell<Option<gtk::ComboBoxText>>,
    rarity: RefCell<Option<gtk::ComboBoxText>>,
    currency: RefCell<Option<gtk::ComboBoxText>>,
    merchant: RefCell<Option<gtk::CheckButton>>,
    collapse: RefCell<Option<gtk::CheckButton>>,
    merchant_allowed: Cell<bool>,
    bulk_only: Cell<bool>,
    updating: Cell<bool>,
    last_snapshot: RefCell<Option<String>>,
    send: Rc<dyn Fn()>,
}
fn label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.);
    l.set_line_wrap(true);
    l
}
fn combo(values: &[(&str, &str)], active: &str) -> gtk::ComboBoxText {
    let c = gtk::ComboBoxText::new();
    super::wheel::protect_combo(&c);
    for (id, name) in values {
        c.append(Some(id), name);
    }
    c.set_active_id(Some(active));
    c
}
impl Panel {
    pub fn new(send: impl Fn() + 'static) -> Rc<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let summary = label("");
        summary.set_no_show_all(true);
        let expander = gtk::Expander::new(Some("Options"));
        expander.style_context().add_class("filter-section");
        expander.set_label_fill(true);
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
        fields.set_margin_top(8);
        expander.add(&fields);
        let quick = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let classic_chips = gtk::FlowBox::new();
        classic_chips.set_selection_mode(gtk::SelectionMode::None);
        classic_chips.set_min_children_per_line(1);
        classic_chips.set_max_children_per_line(20);
        classic_chips.set_column_spacing(8);
        classic_chips.set_row_spacing(8);
        classic_chips.set_no_show_all(true);
        container.pack_start(&classic_chips, false, false, 0);
        container.pack_start(&summary, false, false, 0);
        container.pack_start(&quick, false, false, 0);
        let classic_advanced = gtk::Box::new(gtk::Orientation::Vertical, 0);
        classic_advanced.set_no_show_all(true);
        container.pack_start(&classic_advanced, false, false, 0);
        container.pack_start(&expander, false, false, 0);
        let classic_name = gtk::ToggleButton::new();
        classic_name.style_context().add_class("classic-name");
        classic_name.set_hexpand(true);
        classic_name.set_halign(gtk::Align::Fill);
        let classic_corruption = gtk::Button::new();
        classic_corruption
            .style_context()
            .add_class("classic-corruption");
        classic_corruption.set_no_show_all(true);
        let panel = Rc::new(Self {
            container,
            classic_name,
            classic_corruption,
            exact_name: RefCell::new(String::new()),
            relaxed_name: RefCell::new(String::new()),
            summary,
            expander,
            fields,
            quick,
            classic_chips,
            classic_advanced,
            quick_order: RefCell::new(vec![]),
            theme: Cell::new(crate::appearance::Theme::Tide),
            model: RefCell::new(None),
            numbers: RefCell::new(vec![]),
            flags: RefCell::new(vec![]),
            scope: RefCell::new(None),
            sub: RefCell::new(None),
            corruption: RefCell::new(None),
            rarity: RefCell::new(None),
            currency: RefCell::new(None),
            merchant: RefCell::new(None),
            collapse: RefCell::new(None),
            merchant_allowed: Cell::new(true),
            bulk_only: Cell::new(false),
            updating: Cell::new(false),
            last_snapshot: RefCell::new(None),
            send: Rc::new(send),
        });
        let weak = Rc::downgrade(&panel);
        panel.classic_name.connect_toggled(move |button| {
            if let Some(p) = weak.upgrade() {
                if !p.updating.get() {
                    if let Some(scope) = p.scope.borrow().as_ref() {
                        scope.set_active(button.is_active());
                    }
                }
            }
        });
        let weak = Rc::downgrade(&panel);
        panel.classic_corruption.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                if let Some(combo) = p.corruption.borrow().as_ref() {
                    let next = match combo.active_id().as_deref() {
                        Some("false") => "true",
                        Some("true") => "all",
                        _ => "false",
                    };
                    combo.set_active_id(Some(next));
                }
            }
        });
        let weak = Rc::downgrade(&panel);
        panel.expander.connect_expanded_notify(move |expander| {
            if let Some(p) = weak.upgrade() {
                if p.theme.get() == crate::appearance::Theme::Awakened {
                    p.classic_advanced.set_no_show_all(!expander.is_expanded());
                    p.classic_advanced.set_visible(expander.is_expanded());
                }
            }
        });
        panel
    }
    fn refresh_classic_identity(&self) {
        let was_updating = self.updating.replace(true);
        let exact = self.scope.borrow().as_ref().is_some_and(|s| s.is_active());
        let adjustable = self
            .model
            .borrow()
            .as_ref()
            .is_some_and(|p| p.search_relaxed.is_some());
        self.classic_name.set_active(exact);
        self.classic_name.set_sensitive(adjustable);
        let name = self
            .model
            .borrow()
            .as_ref()
            .and_then(|p| {
                let search = if exact {
                    &p.search_exact
                } else {
                    p.search_relaxed.as_ref().unwrap_or(&p.search_exact)
                };
                search
                    .name
                    .clone()
                    .or_else(|| search.base_type.clone())
                    .or_else(|| search.base_type_trade.clone())
            })
            .unwrap_or_else(|| {
                if exact || !adjustable {
                    self.exact_name.borrow().clone()
                } else {
                    format!(
                        "{}: {}",
                        crate::i18n::tr("Catégorie"),
                        self.relaxed_name.borrow()
                    )
                }
            });
        self.classic_name.set_label(&name);
        if let Some(label) = self
            .classic_name
            .child()
            .and_then(|w| w.downcast::<gtk::Label>().ok())
        {
            label.set_xalign(0.);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_max_width_chars(36);
        }
        let id = self
            .corruption
            .borrow()
            .as_ref()
            .and_then(|c| c.active_id());
        self.classic_corruption.set_visible(id.is_some());
        self.classic_corruption
            .style_context()
            .remove_class("corrupted");
        let text = match id.as_deref() {
            Some("false") => "Non corrompu",
            Some("true") => {
                self.classic_corruption
                    .style_context()
                    .add_class("corrupted");
                "Corrompu"
            }
            _ => "Corruption : tous",
        };
        self.classic_corruption.set_label(&crate::i18n::tr(text));
        self.updating.set(was_updating);
    }
    fn changed(&self) {
        if self.updating.get() {
            return;
        }
        self.refresh_classic_identity();
        self.update_scope();
        self.update_trade();
        self.update_summary();
        let snapshot = serde_json::to_string(&self.read()).ok();
        if *self.last_snapshot.borrow() == snapshot {
            return;
        }
        self.last_snapshot.replace(snapshot);
        (self.send)();
    }
    fn wire_check(self: &Rc<Self>, check: &gtk::CheckButton) {
        let weak = Rc::downgrade(self);
        check.connect_toggled(move |_| {
            if let Some(p) = weak.upgrade() {
                p.changed();
            }
        });
    }
    fn wire_toggle(self: &Rc<Self>, check: &gtk::ToggleButton) {
        let weak = Rc::downgrade(self);
        check.connect_toggled(move |_| {
            if let Some(p) = weak.upgrade() {
                p.changed();
            }
        });
    }
    pub fn set_theme(&self, theme: crate::appearance::Theme) {
        self.theme.set(theme);
        let classic = theme == crate::appearance::Theme::Awakened;
        if let Some(parent) = self
            .expander
            .parent()
            .and_then(|p| p.downcast::<gtk::Container>().ok())
        {
            parent.remove(&self.expander);
        }
        if let Some(parent) = self
            .fields
            .parent()
            .and_then(|p| p.downcast::<gtk::Container>().ok())
        {
            parent.remove(&self.fields);
        }
        if classic {
            self.classic_advanced
                .pack_start(&self.fields, false, false, 0);
        } else {
            self.expander.add(&self.fields);
        }
        self.classic_advanced
            .set_no_show_all(!classic || !self.expander.is_expanded());
        self.classic_advanced
            .set_visible(classic && self.expander.is_expanded());
        self.quick.set_no_show_all(classic);
        self.quick.set_visible(!classic);
        for widget in self.quick_order.borrow().iter() {
            if let Some(parent) = widget
                .parent()
                .and_then(|p| p.downcast::<gtk::Container>().ok())
            {
                parent.remove(widget);
            }
            if classic
                && self
                    .scope
                    .borrow()
                    .as_ref()
                    .is_some_and(|s| s.upcast_ref::<gtk::Widget>() == widget)
            {
                continue;
            }
            if classic && widget.style_context().has_class("quick-choice") {
                self.fields.pack_start(widget, false, false, 0);
                self.fields.reorder_child(widget, 0);
            } else if classic {
                self.classic_chips.insert(widget, -1);
            } else {
                self.quick.pack_start(widget, false, false, 0);
            }
        }
        if classic {
            self.classic_chips.insert(&self.expander, -1);
        } else {
            self.container.pack_start(&self.expander, false, false, 0);
        }
        for child in self.classic_chips.children() {
            if child
                .downcast_ref::<gtk::FlowBoxChild>()
                .is_some_and(|c| c.child().is_none())
            {
                self.classic_chips.remove(&child);
            }
        }
        for child in self.classic_chips.children() {
            child.show_all();
        }
        self.classic_chips.set_visible(classic);
        for row in self.numbers.borrow().iter() {
            row.group.set_spacing(if classic { 0 } else { 8 });
            row.group.style_context().add_class("number-chip");
            row.min.set_width_chars(if classic { 2 } else { 4 });
            if let Some(flat) = &row.classic_value {
                let invalid = flat.text().parse::<u32>().is_err();
                let show_flat = classic || invalid;
                flat.set_no_show_all(!show_flat);
                flat.set_visible(show_flat);
                row.min.set_no_show_all(show_flat);
                row.min.set_visible(!show_flat);
            }
            if let Some(entry) = &row.value_entry {
                entry.set_width_chars(if classic { 2 } else { 5 });
            }
            if let Some(exact) = &row.exact {
                exact.set_label(if classic { "=" } else { "Exact" });
                exact.set_tooltip_text(Some(&crate::i18n::tr("Exact")));
            }
        }
        self.refresh_classic_identity();
    }
    pub fn fit_classic_chips(&self, width: i32) {
        let children: Vec<_> = self
            .classic_chips
            .children()
            .into_iter()
            .filter(|c| c.is_visible())
            .collect();
        let needed = children.iter().map(|c| c.preferred_width().1).sum::<i32>()
            + self.classic_chips.column_spacing() as i32 * children.len().saturating_sub(1) as i32;
        let columns = if !children.is_empty() && needed <= width {
            children.len() as u32
        } else {
            1
        };
        if self.classic_chips.min_children_per_line() != columns {
            self.classic_chips.set_min_children_per_line(columns);
        }
    }
    pub fn commit(&self) {
        for row in self.numbers.borrow().iter() {
            row.min.update();
            if let Some(max) = &row.max {
                max.update();
            }
        }
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)| serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"options":{"rect":rect(self.expander.upcast_ref()),"expanded":self.expander.is_expanded(),"fields_mapped":self.fields.is_mapped()},"corruption":self.corruption.borrow().as_ref().map(|c|serde_json::json!({"rect":rect(if self.theme.get()==crate::appearance::Theme::Awakened{self.classic_corruption.upcast_ref()}else{c.upcast_ref()}),"active":c.active_id().as_deref()!=Some("all")})),"rarity":self.rarity.borrow().as_ref().map(|c|serde_json::json!({"id":c.active_id().as_deref(),"rect":rect(c.upcast_ref()),"active":c.active_id().as_deref()!=Some("all")})),"base":self.scope.borrow().as_ref().map(|b|serde_json::json!({"rect":rect(if self.theme.get()==crate::appearance::Theme::Awakened{self.classic_name.upcast_ref()}else{b.upcast_ref()}),"label":if self.theme.get()==crate::appearance::Theme::Awakened{self.classic_name.label().map(|s|s.to_string())}else{b.label().map(|s|s.to_string())},"active":b.is_active(),"sensitive":b.is_sensitive()})),"numbers":self.numbers.borrow().iter().map(|r|serde_json::json!({"name":format!("{:?}",r.kind),"group":rect(r.group.upcast_ref()),"rect":rect(r.enabled.upcast_ref()),"active":r.enabled.is_active(),"value":r.value_entry.as_ref().or_else(||r.classic_value.as_ref().filter(|e|e.upcast_ref::<gtk::Widget>().is_visible())).map(|e|e.text().to_string()),"field":r.value_entry.as_ref().or_else(||r.classic_value.as_ref().filter(|e|e.upcast_ref::<gtk::Widget>().is_visible())).and_then(|e|rect(e.upcast_ref())).or_else(||rect(r.min.upcast_ref())),"field_sensitive":r.value_entry.as_ref().or_else(||r.classic_value.as_ref().filter(|e|e.upcast_ref::<gtk::Widget>().is_visible())).map(|e|e.is_sensitive())})).collect::<Vec<_>>()})
    }
    fn quick_choice(&self, text: &str, choice: &gtk::ComboBoxText) {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.style_context().add_class("quick-choice");
        let caption = label(text);
        caption.set_width_chars(10);
        row.pack_start(&caption, false, false, 0);
        row.pack_start(choice, true, true, 0);
        self.quick.pack_start(&row, false, false, 0);
    }
    fn wire_combo(self: &Rc<Self>, combo: &gtk::ComboBoxText) {
        let weak = Rc::downgrade(self);
        combo.connect_changed(move |_| {
            if let Some(p) = weak.upgrade() {
                p.changed();
            }
        });
    }
    fn flag(self: &Rc<Self>, kind: Flag, text: &str, active: bool) {
        let check = gtk::CheckButton::with_label(text);
        check.set_active(active);
        self.wire_check(&check);
        self.fields.pack_start(&check, false, false, 0);
        self.flags.borrow_mut().push((kind, check));
    }
    pub fn clear(&self, message: &str) {
        self.updating.set(true);
        // FlowBox removes/destroys its wrapper children on reset. Keep this
        // persistent expander alive by detaching it before removing the wrappers.
        if let Some(parent) = self
            .expander
            .parent()
            .and_then(|p| p.downcast::<gtk::Container>().ok())
        {
            parent.remove(&self.expander);
        }
        self.quick_order.borrow_mut().clear();
        for child in self.classic_chips.children() {
            self.classic_chips.remove(&child);
        }
        self.classic_chips.hide();
        self.model.replace(None);
        self.last_snapshot.replace(None);
        for child in self.quick.children() {
            self.quick.remove(&child);
        }
        for child in self.fields.children() {
            self.fields.remove(&child);
        }
        self.numbers.borrow_mut().clear();
        self.flags.borrow_mut().clear();
        self.scope.replace(None);
        for field in [&self.corruption, &self.rarity, &self.currency] {
            field.replace(None);
        }
        self.sub.replace(None);
        self.merchant.replace(None);
        self.collapse.replace(None);
        self.summary.set_text(message);
        self.summary.show();
        self.expander.hide();
        self.updating.set(false);
    }
    pub fn show(
        self: &Rc<Self>,
        mut plan: Plan,
        item: &crate::item::Item,
        merchant_allowed: bool,
        bulk_only: bool,
    ) {
        self.clear("");
        self.updating.set(true);
        plan.editor_defaults(item);
        self.merchant_allowed.set(merchant_allowed);
        self.bulk_only.set(bulk_only);
        self.model.replace(Some(plan.clone()));
        self.exact_name.replace(
            if matches!(
                item.family(),
                crate::item::Family::Unique | crate::item::Family::Gem
            ) {
                if item.name.is_empty() {
                    item.base.clone()
                } else {
                    item.name.clone()
                }
            } else if item.base.is_empty() {
                item.name.clone()
            } else {
                item.base.clone()
            },
        );
        self.relaxed_name.replace(item.class.clone());
        let scope = gtk::ToggleButton::with_label(if item.base.is_empty() {
            &item.name
        } else {
            &item.base
        });
        scope.style_context().add_class("chip");
        scope.set_halign(gtk::Align::Start);
        if let Some(label) = scope.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_max_width_chars(36);
        }
        scope.set_active(false);
        scope.set_sensitive(plan.search_relaxed.is_some());

        self.wire_toggle(&scope);
        if plan.search_relaxed.is_some() {
            self.quick.pack_start(&scope, false, false, 0);
        }
        self.scope.replace(Some(scope));
        if plan.search_exact.sub.is_some()
            || plan
                .search_relaxed
                .as_ref()
                .is_some_and(|s| s.sub.is_some())
        {
            let active = plan
                .search_relaxed
                .as_ref()
                .filter(|s| s.disabled != Some(true))
                .unwrap_or(&plan.search_exact)
                .sub
                .as_ref()
                .is_some_and(|s| s.disabled != Some(true));
            let sub = gtk::CheckButton::with_label("Limiter à la zone copiée");
            sub.set_active(active);
            self.wire_check(&sub);
            self.fields.pack_start(&sub, false, false, 0);
            self.sub.replace(Some(sub));
        }
        let corruption = combo(
            &[
                ("all", "Tous"),
                ("false", "Non corrompu"),
                ("true", "Corrompu"),
            ],
            match &plan.corrupted {
                Some(c) if !c.value => "false",
                Some(c) if c.exact == Some(true) => "true",
                _ => "all",
            },
        );
        if plan.corrupted.is_some() {
            self.wire_combo(&corruption);
            self.quick_choice("Corruption", &corruption);
            self.corruption.replace(Some(corruption));
        }
        let rarity = combo(
            &[
                ("all", "Toutes"),
                ("nonunique", "Non uniques"),
                ("normal", "Normaux"),
                ("magic", "Magiques"),
                ("rare", "Rares"),
                ("unique", "Uniques"),
                ("uniquefoil", "Uniques foil"),
            ],
            if plan.foil.as_ref().is_some_and(|f| !f.disabled) {
                "uniquefoil"
            } else {
                plan.rarity
                    .as_ref()
                    .map(|f| {
                        if f.disabled {
                            "nonunique"
                        } else {
                            f.value.as_str()
                        }
                    })
                    .unwrap_or("all")
            },
        );
        if (plan.rarity.is_some() || plan.foil.is_some())
            && !matches!(
                item.family(),
                crate::item::Family::Gem
                    | crate::item::Family::Unique
                    | crate::item::Family::Viscera
            )
        {
            self.wire_combo(&rarity);
            self.quick_choice("Rareté", &rarity);
            self.rarity.replace(Some(rarity));
        }
        for kind in [
            Number::ItemLevel,
            Number::Quality,
            Number::GemLevel,
            Number::Links,
            Number::White,
            Number::Stack,
            Number::MapTier,
            Number::AreaLevel,
            Number::Charge,
        ] {
            let Some(n) = kind.get(&plan) else {
                continue;
            };
            let group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            group.style_context().add_class("number-chip");
            if !n.disabled {
                group.style_context().add_class("active");
            }
            let enabled = gtk::ToggleButton::with_label(if matches!(kind, Number::ItemLevel) {
                "ilvl"
            } else {
                kind.label()
            });
            enabled.style_context().add_class("chip");
            enabled.set_active(!n.disabled);
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let min = gtk::SpinButton::with_range(0., 1_000_000., 1.);
            min.set_width_chars(4);
            min.set_value(n.value as f64);
            min.set_sensitive(!n.disabled);
            let value_entry = if matches!(kind, Number::ItemLevel) {
                let entry = gtk::Entry::new();
                let attrs = gtk::pango::AttrList::new();
                attrs.insert(gtk::pango::AttrFontFeatures::new("tnum=1"));
                entry.set_attributes(&attrs);
                entry.set_width_chars(5);
                entry.set_max_width_chars(5);
                entry.set_input_purpose(gtk::InputPurpose::Digits);
                entry.set_text(&n.value.to_string());
                entry.set_sensitive(!n.disabled);
                entry.set_tooltip_text(Some("Niveau d’objet minimum"));
                let field = entry.clone();
                enabled.connect_toggled(move |b| field.set_sensitive(b.is_active()));
                let weak = Rc::downgrade(self);
                entry.connect_changed(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.changed();
                    }
                });
                row.pack_start(&entry, false, false, 0);
                Some(entry)
            } else {
                row.pack_start(&min, false, false, 0);
                None
            };
            let classic_value = if value_entry.is_none() {
                let entry = gtk::Entry::new();
                entry.style_context().add_class("classic-number-value");
                entry.set_width_chars(2);
                entry.set_max_width_chars(n.value.to_string().len().clamp(2, 8) as i32);
                entry.set_alignment(0.5);
                entry.set_text(&n.value.to_string());
                entry.set_input_purpose(gtk::InputPurpose::Digits);
                entry.set_sensitive(!n.disabled);
                entry.set_tooltip_text(Some(&crate::i18n::tr(kind.label())));
                entry.set_no_show_all(true);
                let field = entry.downgrade();
                enabled.connect_toggled(move |b| {
                    if let Some(e) = field.upgrade() {
                        e.set_sensitive(b.is_active());
                    }
                });
                let field = entry.downgrade();
                min.connect_value_changed(move |spin| {
                    if let Some(entry) = field.upgrade().filter(|e| !e.has_focus()) {
                        entry.set_text(&(spin.value().round() as u32).to_string());
                    }
                });
                let spin = min.downgrade();
                let weak = Rc::downgrade(self);
                entry.connect_changed(move |entry| {
                    let width = entry.text().len().clamp(2, 8) as i32;
                    entry.set_width_chars(width);
                    entry.set_max_width_chars(width);
                    if let Some(spin) = spin.upgrade() {
                        if let Ok(value) = entry.text().parse::<u32>() {
                            if spin.value() != value as f64 {
                                spin.set_value(value as f64);
                            }
                        }
                    }
                    if let Some(p) = weak.upgrade() {
                        p.changed();
                    }
                });
                row.pack_start(&entry, false, false, 0);
                Some(entry)
            } else {
                None
            };
            let (max_enabled, max) = if matches!(kind, Number::AreaLevel) {
                let check = gtk::CheckButton::with_label("Max");
                check.set_active(n.max.is_some());
                let spin = gtk::SpinButton::with_range(1., 1_000_000., 1.);
                spin.set_width_chars(4);
                spin.set_value(n.max.unwrap_or(n.value.max(1)) as f64);
                spin.set_sensitive(!n.disabled && n.max.is_some());
                row.pack_start(&check, false, false, 0);
                row.pack_start(&spin, false, false, 0);
                (Some(check), Some(spin))
            } else {
                (None, None)
            };
            {
                let min = min.clone();
                let max = max.clone();
                let max_enabled = max_enabled.clone();
                enabled.connect_toggled(move |c| {
                    min.set_sensitive(c.is_active());
                    if let Some(s) = &max {
                        s.set_sensitive(
                            c.is_active() && max_enabled.as_ref().is_some_and(|m| m.is_active()),
                        );
                    }
                });
            }
            let group_weak = group.downgrade();
            enabled.connect_toggled(move |button| {
                if let Some(group) = group_weak.upgrade() {
                    if button.is_active() {
                        group.style_context().add_class("active");
                    } else {
                        group.style_context().remove_class("active");
                    }
                }
            });
            self.wire_toggle(&enabled);
            {
                let weak = Rc::downgrade(self);
                min.connect_value_changed(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.changed();
                    }
                });
            }
            if let (Some(check), Some(spin)) = (&max_enabled, &max) {
                let spin = spin.clone();
                let enabled = enabled.downgrade();
                check.connect_toggled(move |c| {
                    spin.set_sensitive(
                        c.is_active() && enabled.upgrade().is_some_and(|e| e.is_active()),
                    )
                });
                self.wire_check(check);
            }
            if let Some(spin) = &max {
                let weak = Rc::downgrade(self);
                spin.connect_value_changed(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.changed();
                    }
                });
            }
            group.pack_start(&enabled, false, false, 0);
            group.pack_start(&row, false, false, 0);
            self.quick.pack_start(&group, false, false, 0);
            let exact = if item.family() == crate::item::Family::Gem
                && matches!(kind, Number::GemLevel | Number::Quality)
            {
                let check = gtk::CheckButton::with_label("Exact");
                check.set_active(n.max.is_some());
                self.wire_check(&check);
                group.pack_start(&check, false, false, 0);
                Some(check)
            } else {
                None
            };
            self.numbers.borrow_mut().push(NumberRow {
                kind,
                group,
                enabled,
                min,
                value_entry,
                classic_value,
                exact,
                max_enabled,
                max,
            });
        }
        if let Some(f) = &plan.fractured {
            self.flag(Flag::Fractured, "Exclure les objets fracturés", !f.value);
        }
        if let Some(f) = &plan.imbued_gem {
            self.flag(Flag::Imbued, "Exclure les gemmes imprégnées", f.disabled);
        }
        if let Some(f) = &plan.mirrored {
            self.flag(Flag::Mirrored, "Exclure les copies miroir", f.disabled);
        }
        if let Some(f) = &plan.split {
            self.flag(Flag::Split, "Exclure les objets Split", f.disabled);
        }
        if let Some(f) = &plan.foulborn {
            self.flag(Flag::Foulborn, "Exclure les variantes Foulborn", !f.value);
        }
        if let Some(f) = &plan.vestigial {
            self.flag(Flag::Vestigial, "Exclure les variantes Vestigial", !f.value);
        }
        if let Some(f) = &plan.unidentified {
            self.flag(Flag::Unidentified, "Non identifié seulement", !f.disabled);
        }
        if let Some(f) = &plan.veiled {
            self.flag(Flag::Veiled, "Mods voilés reconnus", !f.disabled);
        }
        if let Some(influences) = &plan.influences {
            for (i, f) in influences.iter().enumerate() {
                self.flag(
                    Flag::Influence(i),
                    &format!("Influence {}", f.value),
                    !f.disabled,
                );
            }
        }
        let merchant = gtk::CheckButton::with_label("Marchands uniquement");
        merchant.set_active(plan.trade.merchant_only);
        merchant.set_sensitive(merchant_allowed && !bulk_only);
        self.wire_check(&merchant);
        self.fields.pack_start(&merchant, false, false, 0);
        self.merchant.replace(Some(merchant));
        let collapse = gtk::CheckButton::with_label("Regrouper par compte");
        collapse.set_active(plan.trade.collapse_listings == "api");
        self.wire_check(&collapse);
        self.fields.pack_start(&collapse, false, false, 0);
        self.collapse.replace(Some(collapse));
        let currency = combo(
            &[
                ("all", "Prix : toutes devises"),
                ("chaos_divine", "Prix : chaos et divines"),
                ("chaos", "Prix : chaos"),
                ("divine", "Prix : divines"),
                ("exalted", "Prix : exalted"),
                ("alch", "Prix : alchemy"),
                ("alt", "Prix : alteration"),
            ],
            plan.trade.currency.as_deref().unwrap_or("all"),
        );
        self.wire_combo(&currency);
        self.fields.pack_start(&currency, false, false, 0);
        self.currency.replace(Some(currency));
        if bulk_only {
            if let Some(c) = self.currency.borrow().as_ref() {
                c.set_sensitive(false);
            }
            self.fields.pack_start(&label("Échange en lot : utilisez le stock minimum et la devise de paiement au-dessus."), false, false, 0);
        }

        self.quick_order.replace(self.quick.children());
        self.set_theme(self.theme.get());
        self.quick.show_all();
        self.fields.show_all();
        self.summary.hide();
        self.expander.show_all();
        self.update_scope();
        self.update_trade();
        self.update_summary();
        self.last_snapshot
            .replace(serde_json::to_string(&self.read()).ok());
        self.updating.set(false);
    }
    fn update_scope(&self) {
        let Some(plan) = self.model.borrow().clone() else {
            return;
        };
        let relaxed = self.scope.borrow().as_ref().is_some_and(|c| !c.is_active());
        let search = if relaxed {
            plan.search_relaxed.as_ref().unwrap_or(&plan.search_exact)
        } else {
            &plan.search_exact
        };
        if let Some(check) = self.sub.borrow().as_ref() {
            check.set_visible(search.sub.is_some());
        }
    }
    fn update_trade(&self) {
        let Some(plan) = self.model.borrow().clone() else {
            return;
        };
        let merchant = self
            .merchant
            .borrow()
            .as_ref()
            .is_some_and(|c| c.is_active());
        if let Some(check) = self.collapse.borrow().as_ref() {
            check.set_sensitive(
                !self.bulk_only.get()
                    && (plan.trade.offline || !merchant || plan.trade.collapse_merchant),
            );
        }
    }
    pub fn set_offline(&self, offline: bool) {
        if let Some(p) = self.model.borrow_mut().as_mut() {
            p.trade.offline = offline;
        }
        self.update_trade();
    }
    pub fn merchant_preference(&self) -> Option<bool> {
        (self.merchant_allowed.get() && !self.bulk_only.get()).then(|| {
            self.merchant
                .borrow()
                .as_ref()
                .is_some_and(|c| c.is_active())
        })
    }
    pub fn read(&self) -> Result<Plan, String> {
        let mut p = self
            .model
            .borrow()
            .clone()
            .ok_or("Copie d’objet non reconnue.")?;
        if let Some(relaxed) = &mut p.search_relaxed {
            relaxed.disabled = Some(self.scope.borrow().as_ref().is_none_or(|c| c.is_active()));
        }
        let search = if p
            .search_relaxed
            .as_ref()
            .is_some_and(|s| s.disabled != Some(true))
        {
            p.search_relaxed.as_mut().unwrap()
        } else {
            &mut p.search_exact
        };
        if let (Some(sub), Some(check)) = (search.sub.as_mut(), self.sub.borrow().as_ref()) {
            sub.disabled = Some(!check.is_active());
        }
        if let Some(c) = self.corruption.borrow().as_ref() {
            p.corrupted = match c.active_id().as_deref() {
                Some("true") => Some(item_filters::Corrupted {
                    value: true,
                    exact: Some(true),
                }),
                Some("false") => Some(item_filters::Corrupted {
                    value: false,
                    exact: Some(true),
                }),
                _ => None,
            };
        }
        if let Some(c) = self.rarity.borrow().as_ref() {
            p.rarity = c
                .active_id()
                .filter(|s| s != "all")
                .map(|s| item_filters::Choice {
                    value: s.into(),
                    disabled: false,
                });
            if let Some(foil) = &mut p.foil {
                foil.disabled = true;
            }
        }
        for row in self.numbers.borrow().iter() {
            if let Some(f) = row.kind.get_mut(&mut p) {
                f.disabled = !row.enabled.is_active();
                f.value = if let Some(entry) = row.value_entry.as_ref().or_else(|| {
                    row.classic_value
                        .as_ref()
                        .filter(|e| e.upcast_ref::<gtk::Widget>().is_visible())
                }) {
                    if f.disabled {
                        entry.text().parse::<u32>().unwrap_or(f.value)
                    } else {
                        entry
                            .text()
                            .parse::<u32>()
                            .map_err(|_| "Valeur minimum invalide.".to_string())?
                    }
                } else {
                    row.min.value().round() as u32
                };
                f.max = if let Some(exact) = &row.exact {
                    exact.is_active().then_some(f.value)
                } else if let (Some(check), Some(max)) = (&row.max_enabled, &row.max) {
                    check.is_active().then(|| max.value().round() as u32)
                } else {
                    None
                };
                if !f.disabled && f.max.is_some_and(|max| f.value > max) {
                    return Err(format!(
                        "{} : minimum supérieur au maximum.",
                        row.kind.label()
                    ));
                }
            }
        }
        for (kind, check) in self.flags.borrow().iter() {
            let enabled = check.is_active();
            match kind {
                Flag::Fractured => {
                    if let Some(f) = &mut p.fractured {
                        f.value = !enabled;
                    }
                }
                Flag::Imbued => {
                    if let Some(f) = &mut p.imbued_gem {
                        f.disabled = enabled;
                    }
                }
                Flag::Mirrored => {
                    if let Some(f) = &mut p.mirrored {
                        f.disabled = enabled;
                    }
                }
                Flag::Split => {
                    if let Some(f) = &mut p.split {
                        f.disabled = enabled;
                    }
                }
                Flag::Foulborn => {
                    if let Some(f) = &mut p.foulborn {
                        f.value = !enabled;
                    }
                }
                Flag::Vestigial => {
                    if let Some(f) = &mut p.vestigial {
                        f.value = !enabled;
                    }
                }
                Flag::Unidentified => {
                    if let Some(f) = &mut p.unidentified {
                        f.disabled = !enabled;
                    }
                }
                Flag::Veiled => {
                    if let Some(f) = &mut p.veiled {
                        f.disabled = !enabled;
                    }
                }
                Flag::Influence(i) => {
                    if let Some(f) = p.influences.as_mut().and_then(|v| v.get_mut(*i)) {
                        f.disabled = !enabled;
                    }
                }
            }
        }
        if let Some(c) = self.merchant.borrow().as_ref() {
            p.trade.merchant_only = c.is_active() && self.merchant_allowed.get();
        }
        if let Some(c) = self.collapse.borrow().as_ref() {
            p.trade.collapse_listings = if c.is_active() { "api" } else { "app" }.into();
        }
        if let Some(c) = self.currency.borrow().as_ref() {
            p.trade.currency = c.active_id().filter(|s| s != "all").map(|s| s.into());
        }
        Ok(p)
    }
    fn update_summary(&self) {
        if let Err(error) = self.read() {
            self.summary.set_text(&error);
            self.summary.show();
        } else {
            self.summary.set_text("");
            self.summary.hide();
        }
    }
}
