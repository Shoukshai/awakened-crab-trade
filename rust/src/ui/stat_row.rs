use crate::{calculated::Candidate, filters::StatFilter};
use glib::translate::IntoGlib;
use gtk::{atk::prelude::*, prelude::*};

pub struct StatRow {
    pub container: gtk::EventBox,
    body: gtk::Box,
    label: gtk::Label,
    normal_text: String,
    classic_text: String,
    caption: gtk::Box,
    top: gtk::Box,
    controls: gtk::Box,
    inputs: gtk::Box,
    tag: gtk::Label,
    check: gtk::Label,
    selected: gtk::ToggleButton,
    min: gtk::Entry,
    max: gtk::Entry,
    slider: Option<gtk::DrawingArea>,
    bounds: Option<super::roll_slider::Bounds>,
    numeric: bool,
    influence_icons: usize,
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
        let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
        body.set_border_width(8);
        container.add(&body);
        let selected = gtk::ToggleButton::new();
        selected.set_active(!candidate.disabled);
        let text = if candidate.exclude {
            format!("Exclure : {}", modifier.text)
        } else {
            modifier.text.clone()
        };
        let classic_text =
            if !modifier.option && candidate.display_reference.matches('#').count() == 1 {
                candidate
                    .display_value
                    .map(|v| {
                        let n = super::economy_panel::format_amount(v);
                        let rendered = if n.starts_with('-') {
                            candidate
                                .display_reference
                                .replacen("+#", &n, 1)
                                .replacen('#', &n, 1)
                        } else {
                            candidate.display_reference.replacen('#', &n, 1)
                        };
                        if candidate.exclude {
                            format!("Exclure : {rendered}")
                        } else {
                            rendered
                        }
                    })
                    .unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            };
        let label = gtk::Label::new(Some(&text));
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(32);
        label.set_xalign(0.);
        let caption = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let check = gtk::Label::new(if selected.is_active() {
            Some("✓")
        } else {
            Some(" ")
        });
        check.set_width_chars(1);
        check.set_valign(gtk::Align::Center);
        check.style_context().add_class("mod-selection");
        caption.pack_start(&check, false, false, 0);
        let checked = check.downgrade();
        selected.connect_toggled(move |s| {
            if let Some(check) = checked.upgrade() {
                check.set_text(if s.is_active() { "✓" } else { " " });
            }
        });
        let mut seen = std::collections::HashSet::new();
        let mut influence_icons = 0;
        for mechanic in modifier
            .sources
            .iter()
            .filter_map(|s| s.influence.as_deref().or(s.mechanic.as_deref()))
            .chain(std::iter::once(candidate.tag.as_str()))
        {
            if seen.insert(mechanic) {
                if let Some(image) = super::icons::influence(mechanic) {
                    caption.pack_start(&image, false, false, 0);
                    influence_icons += 1;
                }
            }
        }
        caption.pack_start(&label, true, true, 0);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        top.pack_start(&caption, true, true, 0);
        body.pack_start(&top, false, false, 0);
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
        let tag = gtk::Label::new(Some(&candidate.tag));
        tag.style_context().add_class("mod-tag");
        tag.set_xalign(0.);
        tag.set_valign(gtk::Align::Center);
        tag.set_no_show_all(true);
        controls.pack_start(&tag, false, false, 0);
        let min = gtk::Entry::new();
        let max = gtk::Entry::new();
        for (entry, name, value) in [
            (&min, "Min", candidate.initial_min),
            (&max, "Max", candidate.initial_max),
        ] {
            entry.set_width_chars(5);
            entry.set_max_width_chars(7);
            entry.set_alignment(0.5);
            entry.set_placeholder_text(Some(name));
            entry.set_tooltip_text(Some(name));
            if let Some(accessible) = entry.accessible() {
                accessible.set_name(name);
            }
            let attrs = gtk::pango::AttrList::new();
            attrs.insert(gtk::pango::AttrFontFeatures::new("tnum=1"));
            entry.set_attributes(&attrs);
            if let Some(value) = value {
                entry.set_text(&super::economy_panel::format_amount(value.round()));
            }
        }
        let bounds = modifier
            .min
            .zip(modifier.max)
            .zip(candidate.display_value)
            .and_then(|((min, max), roll)| {
                let negate = crate::modifiers::translation::display(modifier).1;
                let (min, max) = if negate { (-max, -min) } else { (min, max) };
                super::roll_slider::Bounds::new(min, max, roll)
            });
        let slider = bounds.map(|bounds| {
            super::roll_slider::new(
                bounds,
                &min,
                &max,
                &selected,
                candidate.initial_min.is_none() && candidate.initial_max.is_some(),
            )
        });
        if let Some(slider) = &slider {
            controls.pack_start(slider, true, true, 0);
        } else {
            controls.pack_start(
                &gtk::Box::new(gtk::Orientation::Horizontal, 0),
                true,
                true,
                0,
            );
        }
        let inputs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        inputs.style_context().add_class("mod-bounds");
        min.style_context().add_class("bound-min");
        max.style_context().add_class("bound-max");
        inputs.pack_start(&min, false, false, 0);
        inputs.pack_start(&max, false, false, 0);
        controls.pack_end(&inputs, false, false, 0);
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
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_xalign(0.);
            body.pack_start(&label, false, false, 0);
        }
        let mut editable = vec![
            min.upcast_ref::<gtk::Widget>().downgrade(),
            max.upcast_ref::<gtk::Widget>().downgrade(),
        ];
        if let Some(slider) = &slider {
            editable.push(slider.upcast_ref::<gtk::Widget>().downgrade());
        }
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
            body,
            label,
            normal_text: text,
            classic_text,
            caption,
            top,
            controls,
            inputs,
            tag,
            check,
            selected,
            min,
            max,
            slider,
            bounds,
            numeric: !modifier.option
                && !modifier.ids.iter().any(|id| id.contains('|'))
                && modifier.value.is_some(),
            influence_icons,
            template: StatFilter::from_candidate(candidate),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.selected.is_active() || !self.numeric {
            return Ok(());
        }
        let min = super::roll_slider::read(&self.min)?;
        let max = super::roll_slider::read(&self.max)?;
        if min.zip(max).is_some_and(|(a, b)| a > b) {
            return Err("Min doit être inférieur ou égal à Max.".into());
        }
        Ok(())
    }
    pub fn selected_filter(&self) -> Option<StatFilter> {
        if !self.selected.is_active() {
            return None;
        }
        let mut filter = self.template.clone();
        filter.min = if self.numeric {
            super::roll_slider::read(&self.min).ok().flatten()
        } else {
            None
        };
        filter.max = if self.numeric {
            super::roll_slider::read(&self.max).ok().flatten()
        } else {
            None
        };
        Some(filter)
    }
    pub fn set_theme(&self, theme: crate::appearance::Theme) {
        let classic = theme == crate::appearance::Theme::Awakened;
        self.label.set_text(if classic {
            &self.classic_text
        } else {
            &self.normal_text
        });
        if let Some(parent) = self
            .inputs
            .parent()
            .and_then(|w| w.downcast::<gtk::Container>().ok())
        {
            parent.remove(&self.inputs);
        }
        self.caption.set_hexpand(true);
        if classic && self.numeric {
            self.top.pack_end(&self.inputs, false, false, 0);
        } else {
            self.controls.pack_end(&self.inputs, false, false, 0);
        }
        self.body.set_border_width(if classic { 0 } else { 8 });
        self.tag.set_visible(classic);
        self.controls.set_margin_end(if classic { 96 } else { 0 });
        if let Some(slider) = &self.slider {
            slider.style_context().remove_class("classic-slider");
            if classic {
                slider.style_context().add_class("classic-slider");
            }
            slider.set_size_request(
                if classic { 200 } else { 96 },
                if classic { 20 } else { 30 },
            );
            slider.set_hexpand(!classic);
            self.controls.set_child_packing(
                slider,
                !classic,
                !classic,
                0,
                if classic {
                    gtk::PackType::End
                } else {
                    gtk::PackType::Start
                },
            );
        }
        self.check.style_context().add_class("mod-selection");
        for entry in [&self.min, &self.max] {
            entry.set_width_chars(if classic { 1 } else { 5 });
            entry.set_size_request(if classic { 48 } else { -1 }, -1);
            entry.set_max_width_chars(if classic { 4 } else { 7 });
        }
        self.container.queue_draw();
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"id":self.template.id,"influence_icons":self.influence_icons,"selected":self.selected.is_active(),"min_value":super::roll_slider::read(&self.min).ok().flatten(),"max_value":super::roll_slider::read(&self.max).ok().flatten(),"min_enabled":!self.min.text().trim().is_empty(),"max_enabled":!self.max.text().trim().is_empty(),"select":rect(self.container.upcast_ref()),"min":rect(self.min.upcast_ref()),"max":rect(self.max.upcast_ref()),"slider":self.slider.as_ref().and_then(|s|rect(s.upcast_ref())),"bounds":self.bounds.map(|b|serde_json::json!({"min":b.min,"max":b.max,"roll":b.roll}))})
    }
    pub fn connect_changed(&self, send: impl Fn() + 'static) {
        use std::{cell::RefCell, rc::Rc};
        let selected = self.selected.downgrade();
        let min = self.min.downgrade();
        let max = self.max.downgrade();
        let read = Rc::new(move || {
            let (Some(selected), Some(min), Some(max)) =
                (selected.upgrade(), min.upgrade(), max.upgrade())
            else {
                return String::new();
            };
            serde_json::json!([
                selected.is_active(),
                min.text().as_str(),
                max.text().as_str()
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
        let toggled = changed.clone();
        self.selected.connect_toggled(move |_| toggled());
        for entry in [&self.min, &self.max] {
            let changed = changed.clone();
            let selected = self.selected.clone();
            entry.connect_changed(move |entry| {
                if super::roll_slider::read(entry).is_err() {
                    entry.style_context().add_class("error");
                } else {
                    entry.style_context().remove_class("error");
                }
                selected.set_active(true);
                changed();
            });
        }
    }
}
