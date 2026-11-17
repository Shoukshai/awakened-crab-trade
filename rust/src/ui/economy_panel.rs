use crate::economy::{model::CurrencyValue, Report};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub struct EconomyPanel {
    pub container: gtk::Box,
    pub open: gtk::Button,
    label: gtk::Label,
    graph: gtk::DrawingArea,
    related: gtk::Expander,
    related_rows: gtk::Box,
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
        let related_rows = gtk::Box::new(gtk::Orientation::Vertical, 5);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(100);
        scroll.set_max_content_height(200);
        scroll.set_propagate_natural_height(true);
        scroll.add(&related_rows);
        let related = gtk::Expander::new(Some("Objets liés et prix indicatifs"));
        related.add(&scroll);
        // The optional expander opts out of the window's show_all. Its child
        // must still be visible before expansion can map the scrolling list.
        scroll.show_all();
        related.set_no_show_all(true);
        related.hide();
        container.pack_start(&related, false, false, 0);

        Self {
            container,
            open,
            label,
            graph,
            related,
            related_rows,
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
        self.related.hide();
        self.related.set_expanded(false);
        for child in self.related_rows.children() {
            self.related_rows.remove(&child);
        }
    }
    pub fn not_requested(&self) {
        self.reset();
        self.label
            .set_text("Prix indicatif non chargé : cliquez Rechercher.");
    }
    pub fn league_changed(&self) {
        self.reset();
        self.label
            .set_text("Ligue modifiée : cliquez Rechercher pour charger la cote.");
    }
    pub fn show(&self, result: &Result<Report, String>) {
        self.url.replace(None);
        self.open.set_sensitive(false);
        self.graph.hide();
        self.related.hide();
        for child in self.related_rows.children() {
            self.related_rows.remove(&child);
        }
        match result {
            Ok(report) => {
                let mut text = if let Some(q) = &report.quote {
                    let unit = if q.category == "BaseType" {
                        "Prix de la base seule"
                    } else {
                        "Prix indicatif"
                    };
                    let price = report
                        .price
                        .as_ref()
                        .map(money)
                        .unwrap_or_else(|| format!("{} chaos", format_amount(q.chaos)));
                    let mut text=format!("{unit} poe.ninja : {price} / objet\n{}{} · âge du cache à la consultation : {} min",q.category,q.variant.as_ref().map(|v|format!(" · {v}")).unwrap_or_default(),q.age_seconds/60);
                    if let Some(stack) = &report.stack {
                        if stack.have > 1 {
                            text += &format!(
                                "\nPile copiée × {} ≈ {}",
                                stack.have,
                                money(&stack.price)
                            );
                        }
                        if let Some(maximum) = stack.maximum.zip(stack.full_stack_price.as_ref()) {
                            text +=
                                &format!(" · pile complète × {} ≈ {}", maximum.0, money(maximum.1));
                        }
                    }
                    if let Some(trend) = &report.trend {
                        text += &format!(
                            "\nTendance {} · dispersion {}",
                            match trend.forecast.as_str() {
                                "up" => "en hausse",
                                "down" => "en baisse",
                                _ => "stable",
                            },
                            trend.text
                        );
                    }
                    self.points
                        .replace(q.graph.iter().take(64).copied().collect());
                    if q.graph.iter().flatten().count() >= 2 {
                        self.graph.show();
                        self.graph.queue_draw();
                    }
                    self.url.replace(Some(q.url.clone()));
                    self.open.set_sensitive(true);
                    text
                } else {
                    "Aucun prix poe.ninja pour cette identité et variante.".into()
                };
                if let Some(warning) = &report.warning {
                    text += &format!("\n{warning}");
                }
                self.label.set_text(&text);
                if let Some(related) = &report.related {
                    if let Some(error) = &related.error {
                        self.related_rows.pack_start(
                            &row_label(&format!("Catalogue lié : {error}")),
                            false,
                            false,
                            0,
                        );
                    } else {
                        for (title, rows) in [
                            ("Ensemble lié", &related.related),
                            ("Objets associés", &related.items),
                        ] {
                            if rows.is_empty() {
                                continue;
                            }
                            self.related_rows
                                .pack_start(&row_label(title), false, false, 0);
                            for row in rows {
                                let value = row
                                    .price
                                    .as_ref()
                                    .map(money)
                                    .unwrap_or_else(|| "prix indisponible".into());
                                let variant = row
                                    .key
                                    .variant
                                    .as_ref()
                                    .map(|v| format!(" · {v}"))
                                    .unwrap_or_default();
                                self.related_rows.pack_start(
                                    &row_label(&format!(
                                        "{}{}{variant} : {value}",
                                        if row.highlight { "→ " } else { "" },
                                        row.name
                                    )),
                                    false,
                                    false,
                                    0,
                                );
                            }
                        }
                    }
                    self.related_rows.show_all();
                    self.related.show();
                }
            }
            Err(error) => self
                .label
                .set_text(&format!("Prix indicatif indisponible : {error}")),
        }
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        serde_json::json!({"text":self.label.text().to_string(),"graph_visible":self.graph.is_visible(),"source_enabled":self.open.is_sensitive(),"url":*self.url.borrow(),"related":self.related.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":self.related.allocated_width(),"height":self.related.allocated_height(),"visible":self.related.is_visible(),"expanded":self.related.is_expanded()})),"related_rows":self.related_rows.children().len()})
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

fn row_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.);
    label.set_line_wrap(true);
    label.set_max_width_chars(60);
    label
}
pub(super) fn money(value: &CurrencyValue) -> String {
    format!("{} {}", format_amount(value.min), value.currency)
}
