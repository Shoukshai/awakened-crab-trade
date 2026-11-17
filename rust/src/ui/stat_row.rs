use crate::{calculated::Candidate, filters::StatFilter};
use glib::translate::IntoGlib;
use gtk::{atk::prelude::*, prelude::*};

pub struct StatRow {
    pub container: gtk::EventBox,
    selected: gtk::ToggleButton,
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
        let container = gtk::EventBox::new();
        container.set_visible_window(true);
        container.set_above_child(false);
        container.set_can_focus(true);
        container.style_context().add_class("mod-row");
        let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
        body.set_border_width(8);
        container.add(&body);
        let selected = gtk::ToggleButton::new();
        selected.set_active(!candidate.disabled);
        let text = if candidate.exclude {
            format!("Exclure : {}", modifier.text)
        } else {
            modifier.text.clone()
        };
        let label = gtk::Label::new(Some(&text));
        label.set_line_wrap(true);
        label.set_max_width_chars(32);
        label.set_xalign(0.);
        body.pack_start(&label, false, false, 0);
        container.set_tooltip_text(Some(&candidate.display_reference));
        if let Some(accessible) = container.accessible() {
            accessible.set_role(gtk::atk::Role::CheckBox);
            accessible.set_name(&text);
        }
        let weak = container.downgrade();
        selected.connect_toggled(move |selected| {
            if let Some(row) = weak.upgrade() {
                if selected.is_active() {
                    row.style_context().add_class("selected");
                } else {
                    row.style_context().remove_class("selected");
                }
                if let Some(accessible) = row.accessible() {
                    accessible.notify_state_change(
                        gtk::atk::StateType::Checked.into_glib() as u64,
                        selected.is_active(),
                    );
                }
            }
        });
        if selected.is_active() {
            container.style_context().add_class("selected");
        }
        let model = selected.clone();
        container.connect_key_press_event(move |row, event| {
            if row.has_focus()
                && matches!(
                    event.keyval(),
                    gtk::gdk::keys::constants::space | gtk::gdk::keys::constants::Return
                )
            {
                model.set_active(!model.is_active());
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let min_enabled = gtk::CheckButton::with_label("Min");
        let max_enabled = gtk::CheckButton::with_label("Max");
        let min = gtk::SpinButton::with_range(-1e12, 1e12, 1.);
        let max = gtk::SpinButton::with_range(-1e12, 1e12, 1.);
        for spin in [&min, &max] {
            spin.set_digits(if modifier.decimal { 2 } else { 1 });
            spin.set_width_chars(6);
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
            body.pack_start(&gtk::Label::new(Some(&option_label)), false, false, 0);
        } else if modifier.value.is_some() {
            body.pack_start(&controls, false, false, 0);
        }
        if modifier.reference == "Reflection of Delirium (Difficulty #)" && modifier.value.is_none()
        {
            body.pack_start(
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
            body.pack_start(&label, false, false, 0);
        }
        let editable = [
            min.upcast_ref::<gtk::Widget>(),
            max.upcast_ref::<gtk::Widget>(),
            min_enabled.upcast_ref::<gtk::Widget>(),
            max_enabled.upcast_ref::<gtk::Widget>(),
        ]
        .map(|w| w.downgrade());
        let model = selected.clone();
        container.connect_button_press_event(move |row, event| {
            if event.button() != 1 {
                return glib::Propagation::Proceed;
            }
            // GTK can report the row as event owner even for a bubbled Entry press.
            // Keep numeric controls and bound switches independent from row selection.
            let pointer = row.display().default_seat().and_then(|s| s.pointer());
            if let (Some(window), Some(pointer)) = (row.window(), pointer) {
                let (_, x, y, _) = window.device_position(&pointer);
                for control in &editable {
                    if let Some(control) = control.upgrade() {
                        if let Some((left, top)) = control.translate_coordinates(row, 0, 0) {
                            if x >= left
                                && x < left + control.allocated_width()
                                && y >= top
                                && y < top + control.allocated_height()
                            {
                                return glib::Propagation::Proceed;
                            }
                        }
                    }
                }
            }
            let mut target = gtk::event_widget(&mut (**event).clone());
            while let Some(widget) = target {
                if widget == *row.upcast_ref::<gtk::Widget>() {
                    break;
                }
                if widget.is::<gtk::Entry>()
                    || widget.is::<gtk::SpinButton>()
                    || widget.is::<gtk::CheckButton>()
                {
                    return glib::Propagation::Proceed;
                }
                target = widget.parent();
            }
            row.grab_focus();
            model.set_active(!model.is_active());
            glib::Propagation::Stop
        });
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
    pub fn commit(&self) {
        self.min.update();
        self.max.update();
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
        serde_json::json!({"id":self.template.id,"selected":self.selected.is_active(),"min_value":self.min.value(),"max_value":self.max.value(),"min_enabled":self.min_enabled.is_active(),"max_enabled":self.max_enabled.is_active(),"select":rect(self.container.upcast_ref()),"min":rect(self.min.upcast_ref()),"max":rect(self.max.upcast_ref())})
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
        for check in [
            &self.selected,
            self.min_enabled.upcast_ref::<gtk::ToggleButton>(),
            self.max_enabled.upcast_ref::<gtk::ToggleButton>(),
        ] {
            let changed = changed.clone();
            check.connect_toggled(move |_| changed());
        }
        for spin in [&self.min, &self.max] {
            let changed = changed.clone();
            spin.connect_value_changed(move |_| changed());
        }
    }
}
