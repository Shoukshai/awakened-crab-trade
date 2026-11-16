use crate::economy::Quote;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub struct EconomyPanel {
    pub container: gtk::Box,
    pub open: gtk::Button,
    label: gtk::Label,
    graph: gtk::DrawingArea,
    points: Rc<RefCell<Vec<Option<f64>>>>,
    url: RefCell<Option<String>>,
}
impl EconomyPanel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 5);
        let label = gtk::Label::new(Some("Prix indicatif poe.ninja non chargé"));
        label.set_line_wrap(true);
        label.set_xalign(0.);
        let graph = gtk::DrawingArea::new();
        graph.set_size_request(-1, 85);
        graph.set_no_show_all(true);
        graph.set_tooltip_text(Some("Variations relatives fournies par poe.ninja ; les trous restent des données manquantes."));
        let points: Rc<RefCell<Vec<Option<f64>>>> = Rc::new(RefCell::new(Vec::new()));
        let data = points.clone();
        graph.connect_draw(move |widget, cr| {
            let points = data.borrow();
            let values: Vec<_> = points.iter().flatten().copied().collect();
            if values.len() < 2 {
                return glib::Propagation::Proceed;
            }
            let min = values.iter().copied().fold(0., f64::min);
            let max = values.iter().copied().fold(0., f64::max);
            let span = if max - min < 0.001 { 1. } else { max - min };
            let width = (widget.allocated_width() as f64 - 70.).max(1.);
            let height = (widget.allocated_height() as f64 - 25.).max(1.);
            cr.set_source_rgba(0.75, 0.8, 0.85, 0.35);
            cr.set_line_width(1.);
            let zero = 8. + height * (max / span);
            cr.move_to(55., zero);
            cr.line_to(width + 55., zero);
            let _ = cr.stroke();
            cr.set_source_rgb(0.8, 0.84, 0.9);
            cr.set_font_size(10.);
            cr.move_to(1., 12.);
            let _ = cr.show_text(&format!("{max:.1}%"));
            cr.move_to(1., height + 8.);
            let _ = cr.show_text(&format!("{min:.1}%"));
            cr.set_source_rgb(0.85, 0.7, 0.35);
            cr.set_line_width(2.);
            let mut started = false;
            for (index, value) in points.iter().enumerate() {
                if let Some(value) = value {
                    let x = 55. + width * index as f64 / (points.len() - 1) as f64;
                    let y = 8. + height * (max - value) / span;
                    if started {
                        cr.line_to(x, y);
                    } else {
                        cr.move_to(x, y);
                        started = true;
                    }
                    cr.arc(x, y, 1.5, 0., std::f64::consts::TAU);
                    cr.move_to(x, y);
                } else {
                    let _ = cr.stroke();
                    started = false;
                }
            }
            let _ = cr.stroke();
            glib::Propagation::Proceed
        });
        let open = gtk::Button::with_label("Voir poe.ninja");
        open.set_sensitive(false);
        container.pack_start(&label, false, false, 0);
        container.pack_start(&graph, false, false, 0);
        container.pack_start(&open, false, false, 0);
        Self {
            container,
            open,
            label,
            graph,
            points,
            url: RefCell::new(None),
        }
    }
    pub fn reset(&self) {
        self.label
            .set_text("Chargement du prix indicatif poe.ninja…");
        self.graph.hide();
        self.points.borrow_mut().clear();
        self.url.replace(None);
        self.open.set_sensitive(false);
    }
    pub fn show(&self, result: &Result<Option<Quote>, String>, stack: Option<u32>) {
        self.url.replace(None);
        self.open.set_sensitive(false);
        match result {
            Ok(Some(q)) => {
                let unit = if q.category == "BaseType" {
                    "Prix de la base seule"
                } else {
                    "Prix indicatif"
                };
                let stack = stack
                    .filter(|n| *n > 1)
                    .map(|n| format!(" · pile de {n} ≈ {:.5} chaos", q.chaos * n as f64))
                    .unwrap_or_default();
                self.label.set_text(&format!(
                    "{unit} poe.ninja : {} chaos / objet{stack}\n{}{} · âge du cache à la consultation : {} min",
                    format_amount(q.chaos),
                    q.category,
                    q.variant
                        .as_ref()
                        .map(|v| format!(" · {v}"))
                        .unwrap_or_default(),
                    q.age_seconds / 60
                ));
                self.points.replace(q.graph.clone());
                if q.graph.iter().flatten().count() >= 2 {
                    self.graph.show();
                    self.graph.queue_draw();
                } else {
                    self.graph.hide();
                }
                self.url.replace(Some(q.url.clone()));
                self.open.set_sensitive(true);
            }
            Ok(None) => {
                self.label
                    .set_text("Aucun prix poe.ninja pour cette identité et variante.");
                self.graph.hide();
            }
            Err(e) => {
                self.label
                    .set_text(&format!("Prix indicatif indisponible : {e}"));
                self.graph.hide();
            }
        }
    }
    pub fn url(&self) -> Option<String> {
        self.url.borrow().clone()
    }
}
pub fn format_amount(value: f64) -> String {
    if value > 0. && value < 0.00001 {
        return format!("{value:.3e}");
    }
    let text = format!("{value:.5}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}
