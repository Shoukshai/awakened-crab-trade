//! Two-tone roll selector. Bounds are copied item rolls, never invented tiers.
use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: f64,
    pub max: f64,
    pub roll: f64,
    step: f64,
}
impl Bounds {
    pub fn new(min: f64, max: f64, roll: f64, decimal: bool) -> Option<Self> {
        if ![min, max, roll].iter().all(|n| n.is_finite()) || min > max || !(max - min).is_finite()
        {
            return None;
        }
        Some(Self {
            min,
            max,
            roll,
            step: if decimal || max - min < 1. { 0.01 } else { 1. },
        })
    }
    fn fraction(self, value: f64) -> f64 {
        if self.max == self.min {
            0.5
        } else {
            ((value - self.min) / (self.max - self.min)).clamp(0., 1.)
        }
    }
    fn value(self, fraction: f64) -> f64 {
        let raw = (self.max - self.min) * fraction.clamp(0., 1.) + self.min;
        ((self.min + ((raw - self.min) / self.step).round() * self.step) * 100.)
            .round()
            .clamp(self.min * 100., self.max * 100.)
            / 100.
    }
}
pub fn read(entry: &gtk::Entry) -> Result<Option<f64>, String> {
    let text = entry.text();
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && n.abs() <= 1e12)
        .map(Some)
        .ok_or_else(|| "Valeur Min/Max invalide.".to_string())
}
fn set(entry: &gtk::Entry, value: f64) {
    entry.set_text(&super::economy_panel::format_amount(value));
}
#[derive(Clone, Copy, PartialEq)]
enum End {
    Min,
    Max,
}
fn choose(min: Option<f64>, max: Option<f64>, value: f64, prefer_max: bool) -> End {
    match (min, max) {
        (Some(_), None) => End::Min,
        (None, Some(_)) => End::Max,
        (Some(min), Some(max)) if (value - max).abs() < (value - min).abs() => End::Max,
        (None, None) if prefer_max => End::Max,
        _ => End::Min,
    }
}
pub fn new(
    bounds: Bounds,
    min: &gtk::Entry,
    max: &gtk::Entry,
    selected: &gtk::ToggleButton,
    prefer_max: bool,
) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_size_request(96, 30);
    area.set_hexpand(true);
    area.set_can_focus(true);
    area.add_events(
        gtk::gdk::EventMask::BUTTON_PRESS_MASK
            | gtk::gdk::EventMask::BUTTON_RELEASE_MASK
            | gtk::gdk::EventMask::POINTER_MOTION_MASK,
    );
    area.set_tooltip_text(Some(&format!(
        "{} – {} · valeur {}",
        super::economy_panel::format_amount(bounds.min),
        super::economy_panel::format_amount(bounds.max),
        super::economy_panel::format_amount(bounds.roll)
    )));
    if let Some(accessible) = area.accessible() {
        use gtk::atk::prelude::*;
        accessible.set_role(gtk::atk::Role::Slider);
        accessible.set_name("Plage de recherche du mod");
    }
    let dragging = Rc::new(Cell::new(None::<End>));
    let active = Rc::new(Cell::new(if prefer_max { End::Max } else { End::Min }));
    {
        let min = min.downgrade();
        let max = max.downgrade();
        let selected = selected.downgrade();
        area.connect_draw(move |area, cr| {
            let (Some(min), Some(max), Some(selected)) =
                (min.upgrade(), max.upgrade(), selected.upgrade())
            else {
                return glib::Propagation::Proceed;
            };
            let w = (area.allocated_width() as f64 - 12.).max(1.);
            let h = area.allocated_height() as f64;
            let low = read(&min).ok().flatten();
            let high = read(&max).ok().flatten();
            cr.set_source_rgb(0.16, 0.19, 0.23);
            cr.rectangle(6., 4., w, h - 8.);
            let _ = cr.fill();
            if selected.is_active() && (low.is_some() || high.is_some()) {
                let l = bounds.fraction(low.unwrap_or(bounds.min));
                let r = bounds.fraction(high.unwrap_or(bounds.max));
                if l <= r {
                    cr.set_source_rgb(0.39, 0.45, 0.52);
                    cr.rectangle(6. + l * w, 4., ((r - l) * w).max(1.), h - 8.);
                    let _ = cr.fill();
                }
            }
            cr.set_source_rgb(0.9, 0.91, 0.93);
            cr.set_line_width(1.);
            let roll = 6. + bounds.fraction(bounds.roll) * w;
            cr.move_to(roll, 1.);
            cr.line_to(roll, 5.);
            cr.move_to(roll, h - 5.);
            cr.line_to(roll, h - 1.);
            let _ = cr.stroke();
            for value in [low, high].into_iter().flatten() {
                let x = 6. + bounds.fraction(value) * w;
                cr.move_to(x, 4.);
                cr.line_to(x, h - 4.);
                let _ = cr.stroke();
            }
            cr.set_font_size(11.);
            cr.move_to(10., h / 2. + 4.);
            let _ = cr.show_text(&super::economy_panel::format_amount(bounds.min));
            let text = super::economy_panel::format_amount(bounds.max);
            if let Ok(size) = cr.text_extents(&text) {
                cr.move_to(2. + w - size.x_advance(), h / 2. + 4.);
                let _ = cr.show_text(&text);
            }
            if area.has_focus() {
                cr.set_source_rgb(0.77, 0.65, 0.42);
                cr.rectangle(3., 1., w + 6., h - 2.);
                let _ = cr.stroke();
            }
            glib::Propagation::Proceed
        });
    }
    for entry in [min, max] {
        let weak = area.downgrade();
        entry.connect_changed(move |_| {
            if let Some(area) = weak.upgrade() {
                area.queue_draw();
            }
        });
    }
    let weak = area.downgrade();
    selected.connect_toggled(move |_| {
        if let Some(area) = weak.upgrade() {
            area.queue_draw();
        }
    });
    {
        let min = min.downgrade();
        let max = max.downgrade();
        let selected = selected.downgrade();
        let dragging = dragging.clone();
        let active = active.clone();
        area.connect_button_press_event(move |area, event| {
            if event.button() != 1 || bounds.min == bounds.max {
                return glib::Propagation::Proceed;
            }
            let (Some(min), Some(max), Some(selected)) =
                (min.upgrade(), max.upgrade(), selected.upgrade())
            else {
                return glib::Propagation::Proceed;
            };
            let value = bounds
                .value((event.position().0 - 6.) / (area.allocated_width() as f64 - 12.).max(1.));
            let end = if event.state().contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                End::Max
            } else {
                choose(
                    read(&min)
                        .ok()
                        .flatten()
                        .map(|v| v.clamp(bounds.min, bounds.max)),
                    read(&max)
                        .ok()
                        .flatten()
                        .map(|v| v.clamp(bounds.min, bounds.max)),
                    value,
                    prefer_max,
                )
            };
            dragging.set(Some(end));
            active.set(end);
            area.grab_focus();
            selected.set_active(true);
            set(if end == End::Min { &min } else { &max }, value);
            glib::Propagation::Stop
        });
    }
    {
        let min = min.downgrade();
        let max = max.downgrade();
        let dragging = dragging.clone();
        area.connect_motion_notify_event(move |area, event| {
            let Some(end) = dragging.get() else {
                return glib::Propagation::Proceed;
            };
            let (Some(min), Some(max)) = (min.upgrade(), max.upgrade()) else {
                return glib::Propagation::Proceed;
            };
            let mut value = bounds
                .value((event.position().0 - 6.) / (area.allocated_width() as f64 - 12.).max(1.));
            if end == End::Min {
                value = value.min(read(&max).ok().flatten().unwrap_or(bounds.max));
            } else {
                value = value.max(read(&min).ok().flatten().unwrap_or(bounds.min));
            }
            set(if end == End::Min { &min } else { &max }, value);
            glib::Propagation::Stop
        });
    }
    let release = dragging.clone();
    area.connect_button_release_event(move |_, event| {
        if event.button() == 1 {
            release.set(None);
        }
        glib::Propagation::Proceed
    });
    area.connect_unmap(move |_| {
        dragging.set(None);
    });
    {
        let min = min.downgrade();
        let max = max.downgrade();
        let selected = selected.downgrade();
        area.connect_key_press_event(move |_, event| {
            use gtk::gdk::keys::constants;
            let (Some(min), Some(max), Some(selected)) =
                (min.upgrade(), max.upgrade(), selected.upgrade())
            else {
                return glib::Propagation::Proceed;
            };
            if event.keyval() == constants::space {
                active.set(if active.get() == End::Min {
                    End::Max
                } else {
                    End::Min
                });
                return glib::Propagation::Stop;
            }
            let entry = if active.get() == End::Min { &min } else { &max };
            let old = read(entry)
                .ok()
                .flatten()
                .unwrap_or(if active.get() == End::Min {
                    bounds.min
                } else {
                    bounds.max
                });
            let value = match event.keyval() {
                constants::Left | constants::Down => old - bounds.step,
                constants::Right | constants::Up => old + bounds.step,
                constants::Home => bounds.min,
                constants::End => bounds.max,
                _ => return glib::Propagation::Proceed,
            };
            selected.set_active(true);
            set(entry, value.clamp(bounds.min, bounds.max));
            glib::Propagation::Stop
        });
    }
    area
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slider_snaps_clamps_and_preserves_negative_and_decimal_rolls() {
        let wand = Bounds::new(8., 12., 8., false).unwrap();
        assert_eq!(wand.value(0.5), 10.);
        assert_eq!(wand.value(-1.), 8.);
        assert_eq!(wand.value(2.), 12.);
        let negative = Bounds::new(-12., -9., -9., false).unwrap();
        assert_eq!(negative.value(0.5), -10.);
        assert_eq!(negative.fraction(-9.), 1.);
        let decimal = Bounds::new(0.1, 0.2, 0.13, true).unwrap();
        assert_eq!(decimal.value(0.5), 0.15);
        assert!(Bounds::new(f64::NAN, 1., 0., false).is_none());
        assert!(Bounds::new(2., 1., 1., false).is_none());
        let fixed = Bounds::new(5., 5., 5., false).unwrap();
        assert_eq!(fixed.value(0.8), 5.);
    }
}
