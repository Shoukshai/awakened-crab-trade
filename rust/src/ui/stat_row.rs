use crate::{filters::StatFilter, modifiers::Modifier};
use gtk::prelude::*;

pub struct StatRow {
    pub container: gtk::Box,
    selected: gtk::CheckButton,
    min_enabled: gtk::CheckButton,
    max_enabled: gtk::CheckButton,
    min: gtk::SpinButton,
    max: gtk::SpinButton,
    modifier: Modifier,
}
impl StatRow {
    pub fn new(modifier: &Modifier) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let selected = gtk::CheckButton::new();
        let label = gtk::Label::new(Some(&modifier.text));
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
            spin.set_value(modifier.value.unwrap_or(0.));
        }
        min_enabled.set_active(modifier.better >= 0);
        max_enabled.set_active(modifier.better <= 0);
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
            container.pack_start(
                &gtk::Label::new(Some(&format!("Variante : {option}"))),
                false,
                false,
                0,
            );
        } else if modifier.value.is_some() {
            let note = gtk::Label::new(Some(
                "Valeur Trade · les plages de dégâts utilisent leur moyenne",
            ));
            note.set_line_wrap(true);
            note.set_xalign(0.);
            container.pack_start(&note, false, false, 0);
            container.pack_start(&controls, false, false, 0);
        }
        Self {
            container,
            selected,
            min_enabled,
            max_enabled,
            min,
            max,
            modifier: modifier.clone(),
        }
    }
    pub fn selected_filter(&self) -> Option<StatFilter> {
        if !self.selected.is_active() {
            return None;
        }
        let m = &self.modifier;
        let numeric = !m.option && !m.ids.iter().any(|id| id.contains('|')) && m.value.is_some();
        Some(StatFilter {
            id: m.id.clone(),
            alternatives: m.ids.iter().skip(1).cloned().collect(),
            min: (numeric && self.min_enabled.is_active()).then(|| self.min.value()),
            max: (numeric && self.max_enabled.is_active()).then(|| self.max.value()),
            option: m.option.then_some(m.value).flatten(),
            roll_value: m.value,
            inverted: m.inverted,
        })
    }
}
