//! Native editor for Mercenary groups, support modes, six links and exclusions.
use crate::item::mercenary::{Filter, Node, Preset};
use gtk::prelude::*;
use std::rc::Rc;
struct Control {
    node: usize,
    child: Option<usize>,
    id: String,
    reference: String,
    selected: gtk::CheckButton,
    option: Option<gtk::ComboBoxText>,
    min_enabled: Option<gtk::CheckButton>,
    minimum: Option<gtk::SpinButton>,
    expander: Option<gtk::Expander>,
}
pub struct Panel {
    pub container: gtk::Box,
    template: Preset,
    controls: Vec<Control>,
}
fn rect(widget: &gtk::Widget, window: &gtk::Window) -> Option<serde_json::Value> {
    widget.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":widget.allocated_width(),"height":widget.allocated_height(),"mapped":widget.is_mapped()}))
}
impl Panel {
    pub fn new(preset: Preset, changed: impl Fn() + 'static) -> Self {
        let changed: Rc<dyn Fn()> = Rc::new(changed);
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let note=gtk::Label::new(Some("Les compétences principales restent recherchées. Cochez une compétence pour appliquer ses supports ; N−1 accepte tous les supports optionnels sauf un."));
        note.set_line_wrap(true);
        note.set_xalign(0.);
        note.set_max_width_chars(60);
        container.pack_start(&note, false, false, 0);
        let mut controls = Vec::new();
        for (index, node) in preset.nodes.iter().enumerate() {
            match node {
                Node::Stat(f) => {
                    let (row, control) = Self::row(index, None, f, changed.clone());
                    if f.hidden.is_some() {
                        let expander =
                            gtk::Expander::new(Some("Compétence principale sans support"));
                        expander.add(&row);
                        container.pack_start(&expander, false, false, 0);
                    } else {
                        container.pack_start(&row, false, false, 0);
                    }
                    controls.push(control);
                }
                Node::Group(g) => {
                    let (row, control) = Self::row(index, None, &g.meta, changed.clone());
                    container.pack_start(&row, false, false, 0);
                    let children = gtk::Box::new(gtk::Orientation::Vertical, 8);
                    children.set_margin_start(18);
                    children.set_sensitive(!g.meta.disabled);
                    let weak = children.downgrade();
                    control.selected.connect_toggled(move |c| {
                        if let Some(box_) = weak.upgrade() {
                            box_.set_sensitive(c.is_active());
                        }
                    });
                    let control_index = controls.len();
                    controls.push(control);
                    for (child, f) in g.stats.iter().enumerate() {
                        let (row, control) = Self::row(index, Some(child), f, changed.clone());
                        children.pack_start(&row, false, false, 0);
                        controls.push(control);
                    }
                    let expander = gtk::Expander::new(Some(if g.group == "not" {
                        "Compétences à exclure"
                    } else {
                        "Supports et six liens"
                    }));
                    expander.add(&children);
                    controls[control_index].expander = Some(expander.clone());
                    container.pack_start(&expander, false, false, 0);
                }
            }
        }
        Self {
            container,
            template: preset,
            controls,
        }
    }
    fn row(
        node: usize,
        child: Option<usize>,
        f: &Filter,
        changed: Rc<dyn Fn()>,
    ) -> (gtk::Box, Control) {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let role = match f.tag.as_str() {
            "mercenary-primary" => "Principale · ",
            "mercenary-secondary" => "Secondaire · ",
            "mercenary-utility" => "Utilitaire · ",
            "brick" => "Déconseillée · ",
            _ => "",
        };
        let text = if f.ids.first().is_some_and(|s| s == "item.not_group") {
            "Exclure d’autres compétences".into()
        } else {
            format!(
                "{}{}{}{}",
                if f.exclude { "Exclure · " } else { "" },
                role,
                f.text,
                f.tier.map(|t| format!(" · Tier {t}")).unwrap_or_default()
            )
        };
        let selected = gtk::CheckButton::with_label(&text);
        selected.set_active(!f.disabled);
        selected.set_tooltip_text(Some(&f.ids.join("\n")));
        row.pack_start(&selected, false, false, 0);
        let option = f.option.map(|v| {
            let combo = gtk::ComboBoxText::new();
            combo.append(Some("1"), "Support requis");
            combo.append(Some("0"), "Optionnel · choix N−1");
            combo.set_active_id(Some(&v.to_string()));
            combo.set_sensitive(!f.disabled);
            super::wheel::protect_combo(&combo);
            let weak = combo.downgrade();
            selected.connect_toggled(move |c| {
                if let Some(w) = weak.upgrade() {
                    w.set_sensitive(c.is_active());
                }
            });
            let send = changed.clone();
            combo.connect_changed(move |_| send());
            row.pack_start(&combo, false, false, 0);
            combo
        });
        let (min_enabled, minimum) = if f.ids.first().is_some_and(|s| s == "item.mercenary_6link") {
            let inputs = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let enabled = gtk::CheckButton::with_label("Minimum de supports au meilleur tier");
            enabled.set_active(f.min.is_some());
            let minimum = gtk::SpinButton::with_range(0., 5., 1.);
            minimum.set_value(f.min.unwrap_or(0.));
            minimum.connect_scroll_event(|w, e| super::wheel::scroll_parent(w.upcast_ref(), e));
            minimum.set_sensitive(enabled.is_active());
            inputs.pack_start(&enabled, false, false, 0);
            inputs.pack_start(&minimum, false, false, 0);
            inputs.set_sensitive(!f.disabled);
            let weak = inputs.downgrade();
            selected.connect_toggled(move |c| {
                if let Some(w) = weak.upgrade() {
                    w.set_sensitive(c.is_active());
                }
            });
            let weak = minimum.downgrade();
            let send = changed.clone();
            enabled.connect_toggled(move |c| {
                if let Some(w) = weak.upgrade() {
                    w.set_sensitive(c.is_active());
                }
                send();
            });
            let send = changed.clone();
            minimum.connect_value_changed(move |_| send());
            row.pack_start(&inputs, false, false, 0);
            (Some(enabled), Some(minimum))
        } else {
            (None, None)
        };
        selected.connect_toggled(move |_| changed());
        (
            row,
            Control {
                node,
                child,
                id: f.ids.first().cloned().unwrap_or_default(),
                reference: f.reference.clone(),
                selected,
                option,
                min_enabled,
                minimum,
                expander: None,
            },
        )
    }
    pub fn read(&self) -> Preset {
        let mut preset = self.template.clone();
        for c in &self.controls {
            let f = match &mut preset.nodes[c.node] {
                Node::Stat(f) => f,
                Node::Group(g) => {
                    if let Some(child) = c.child {
                        &mut g.stats[child]
                    } else {
                        &mut g.meta
                    }
                }
            };
            f.disabled = !c.selected.is_active();
            if let Some(option) = &c.option {
                f.option = option.active_id().and_then(|s| s.parse().ok());
            }
            if let (Some(check), Some(spin)) = (&c.min_enabled, &c.minimum) {
                f.min = check.is_active().then(|| spin.value());
            }
        }
        // A disabled native group suspends all its child choices. Retain the
        // widget choices so enabling the group restores them without a request.
        for node in &mut preset.nodes {
            if let Node::Group(group) = node {
                if group.meta.disabled {
                    for child in &mut group.stats {
                        child.disabled = true;
                    }
                }
            }
        }
        preset
    }
    pub fn active_count(&self) -> usize {
        self.controls
            .iter()
            .filter(|c| c.selected.is_active())
            .count()
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> Vec<serde_json::Value> {
        self.controls.iter().map(|c|serde_json::json!({"node":c.node,"child":c.child,"id":c.id,"reference":c.reference,"expander":c.expander.as_ref().map(|w|serde_json::json!({"expanded":w.is_expanded(),"rect":rect(w.upcast_ref(),window)})),"selected":c.selected.is_active(),"select":rect(c.selected.upcast_ref(),window),"option":c.option.as_ref().map(|w|serde_json::json!({"value":w.active_id().map(|s|s.to_string()),"rect":rect(w.upcast_ref(),window)})),"minimum":c.minimum.as_ref().map(|w|serde_json::json!({"enabled":c.min_enabled.as_ref().is_some_and(|c|c.is_active()),"value":w.value(),"rect":rect(w.upcast_ref(),window)}))})).collect()
    }
}
