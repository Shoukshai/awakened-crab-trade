//! Wheel gestures scroll the filter list; closed dropdowns never change value.
use gtk::{gdk, prelude::*};

pub fn protect_combo(combo: &gtk::ComboBoxText) {
    combo.add_events(gdk::EventMask::SCROLL_MASK | gdk::EventMask::SMOOTH_SCROLL_MASK);
    combo.connect_scroll_event(|widget, event| {
        if widget.is_popup_shown() {
            glib::Propagation::Proceed
        } else {
            scroll_parent(widget.upcast_ref(), event)
        }
    });
}

pub fn scroll_parent(widget: &gtk::Widget, event: &gdk::EventScroll) -> glib::Propagation {
    if let Some(list) = widget
        .ancestor(gtk::ScrolledWindow::static_type())
        .and_then(|w| w.downcast::<gtk::ScrolledWindow>().ok())
    {
        let adjustment = list.vadjustment();
        let step = adjustment
            .step_increment()
            .max(adjustment.page_size() / 10.)
            .max(32.);
        let delta = match event.direction() {
            gdk::ScrollDirection::Up => -step,
            gdk::ScrollDirection::Down => step,
            gdk::ScrollDirection::Smooth => event.delta().1 * step,
            _ => 0.,
        };
        adjustment.set_value((adjustment.value() + delta).clamp(
            adjustment.lower(),
            (adjustment.upper() - adjustment.page_size()).max(adjustment.lower()),
        ));
    }
    glib::Propagation::Stop
}
