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
            container.pack_start(
                &gtk::Label::new(Some(&format!("Variante : {option}"))),
                false,
                false,
                0,
            );
        } else if modifier.value.is_some() {
            let note = gtk::Label::new(Some(&format!(
                "Stat Trade : {}\n{}",
                candidate.display_reference,
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
}
