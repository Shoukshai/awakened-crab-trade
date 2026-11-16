use crate::{calculated::Candidate, filters::StatFilter};
use gtk::prelude::*;

pub struct StatRow {
    pub container: gtk::Box,
    selected: gtk::CheckButton,
    min_enabled: gtk::CheckButton,
    max_enabled: gtk::CheckButton,
    min: gtk::SpinButton,
    max: gtk::SpinButton,
    numeric: bool,
    template: StatFilter,
}
impl StatRow {
    pub fn new(candidate: &Candidate) -> Self {
        let modifier = &candidate.modifier;
        let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let selected = gtk::CheckButton::new();
        selected.set_active(!candidate.disabled);
        let text = if candidate.exclude {
            format!("Exclure : {}", modifier.text)
        } else {
            modifier.text.clone()
        };
        let label = gtk::Label::new(Some(&text));
        label.set_line_wrap(true);
        label.set_max_width_chars(58);
        label.set_xalign(0.);
        selected.add(&label);
        selected.set_tooltip_text(Some(&format!(
            "{} · {}\n{}",
            modifier.kind,
            modifier.reference,
            modifier.ids.join("\n")
        )));
        container.pack_start(&selected, false, false, 0);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let min_enabled = gtk::CheckButton::with_label("Min");
        let max_enabled = gtk::CheckButton::with_label("Max");
        let min = gtk::SpinButton::with_range(-1e12, 1e12, 1.);
        let max = gtk::SpinButton::with_range(-1e12, 1e12, 1.);
        for spin in [&min, &max] {
            spin.set_digits(if modifier.decimal { 2 } else { 1 });
            spin.set_width_chars(9);
            spin.set_value(candidate.display_value.unwrap_or(0.));
        }
        min_enabled.set_active(candidate.initial_min.is_some());
        max_enabled.set_active(candidate.initial_max.is_some());
        if let Some(value) = candidate.initial_min {
            min.set_value(value);
        }
        if let Some(value) = candidate.initial_max {
            max.set_value(value);
        }
        for (check, spin) in [(&min_enabled, &min), (&max_enabled, &max)] {
            spin.set_sensitive(check.is_active());
            let spin = spin.clone();
            check.connect_toggled(move |check| spin.set_sensitive(check.is_active()));
        }
        controls.pack_start(&min_enabled, false, false, 0);
        controls.pack_start(&min, false, false, 0);
        controls.pack_start(&max_enabled, false, false, 0);
        controls.pack_start(&max, false, false, 0);
        if modifier.option || modifier.ids.iter().any(|id| id.contains('|')) {
            let option = modifier
                .ids
                .first()
                .and_then(|id| id.split_once('|'))
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| modifier.value.map(|v| v.to_string()).unwrap_or_default());
            let option_label = if modifier.id == "item.has_empty_modifier" {
                match modifier.value {
                    Some(1.) => "Un préfixe libre ou crafté".to_string(),
                    Some(2.) => "Un suffixe libre ou crafté".to_string(),
                    _ => "Un emplacement libre ou crafté".to_string(),
                }
            } else if modifier.id.starts_with("pseudo.pseudo_temple_") {
                if modifier.value == Some(1.) {
                    "Salle accessible".to_string()
                } else if modifier.value.is_none() {
                    "Salle accessible ou bloquée (explosifs présents)".to_string()
                } else {
                    "Salle bloquée".to_string()
                }
            } else {
                format!("Variante : {option}")
            };
            container.pack_start(&gtk::Label::new(Some(&option_label)), false, false, 0);
        } else if modifier.value.is_some() {
            let note = gtk::Label::new(Some(&format!(
                "{}{}",
                if modifier.kind == "property" {
                    String::new()
                } else {
                    format!("Stat Trade : {}\n", candidate.display_reference)
                },
                if [
                    "item.armour",
                    "item.evasion_rating",
                    "item.energy_shield",
                    "item.ward",
                    "item.physical_dps",
                    "item.total_dps"
                ]
                .contains(&modifier.id.as_str())
                {
                    "Défenses/dégâts physiques : qualité normalisée selon Trade."
                } else if modifier.kind == "property" {
                    "Propriété calculée pour les filtres Trade."
                } else {
                    "Valeur canonique ; les plages de dégâts utilisent leur moyenne."
                }
            )));
            note.set_line_wrap(true);
            note.set_xalign(0.);
            container.pack_start(&note, false, false, 0);
            container.pack_start(&controls, false, false, 0);
        }
        if modifier.reference == "Reflection of Delirium (Difficulty #)" && modifier.value.is_none()
        {
            container.pack_start(
                &gtk::Label::new(Some("Sans condition de difficulté.")),
                false,
                false,
                0,
            );
        }
        if let Some(oils) = &candidate.oils {
            let label = gtk::Label::new(Some(&format!("Anointment : {}", oils.join(" + "))));
            label.set_line_wrap(true);
            label.set_xalign(0.);
            container.pack_start(&label, false, false, 0);
        }
        Self {
            container,
            selected,
            min_enabled,
            max_enabled,
            min,
            max,
            numeric: !modifier.option
                && !modifier.ids.iter().any(|id| id.contains('|'))
                && modifier.value.is_some(),
            template: StatFilter::from_candidate(candidate),
        }
    }
    pub fn selected_filter(&self) -> Option<StatFilter> {
        if !self.selected.is_active() {
            return None;
        }
        let mut filter = self.template.clone();
        filter.min = (self.numeric && self.min_enabled.is_active()).then(|| self.min.value());
        filter.max = (self.numeric && self.max_enabled.is_active()).then(|| self.max.value());
        Some(filter)
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"id":self.template.id,"selected":self.selected.is_active(),"select":rect(self.selected.upcast_ref()),"min":rect(self.min.upcast_ref()),"max":rect(self.max.upcast_ref())})
    }
    pub fn connect_changed(&self, send: impl Fn() + 'static) {
        use std::{cell::RefCell, rc::Rc};
        let id = self.template.id.clone();
        self.selected.connect_button_press_event(move |_, e| {
            eprintln!(
                "stat pointer-press id={id:?} root={:?} button={} time={} kind={:?}",
                e.root(),
                e.button(),
                e.time(),
                e.event_type()
            );
            glib::Propagation::Proceed
        });
        let selected = self.selected.downgrade();
        let min_enabled = self.min_enabled.downgrade();
        let max_enabled = self.max_enabled.downgrade();
        let min = self.min.downgrade();
        let max = self.max.downgrade();
        let numeric = self.numeric;
        let read = Rc::new(move || {
            let (Some(selected), Some(min_enabled), Some(max_enabled), Some(min), Some(max)) = (
                selected.upgrade(),
                min_enabled.upgrade(),
                max_enabled.upgrade(),
                min.upgrade(),
                max.upgrade(),
            ) else {
                return "null".into();
            };
            if !selected.is_active() {
                return "null".into();
            }
            serde_json::json!([
                (numeric && min_enabled.is_active()).then(|| min.value()),
                (numeric && max_enabled.is_active()).then(|| max.value())
            ])
            .to_string()
        });
        let previous = RefCell::new(read());
        let changed = Rc::new(move || {
            let value = read();
            if *previous.borrow() != value {
                previous.replace(value);
                send();
            }
        });
        for check in [&self.selected, &self.min_enabled, &self.max_enabled] {
            let changed = changed.clone();
            check.connect_toggled(move |_| changed());
        }
        for spin in [&self.min, &self.max] {
            let changed = changed.clone();
            spin.connect_value_changed(move |_| changed());
        }
    }
}
