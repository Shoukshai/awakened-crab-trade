//! Painted outside-click surface. Input is enabled only after both surfaces paint.
use gtk::{gdk, prelude::*};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{cell::Cell, rc::Rc};
pub struct Backdrop {
    window: gtk::Window,
    panel: Cell<crate::price_layout::Rect>,
    painted: Cell<bool>,
    main_ready: Cell<bool>,
    applied: Cell<Option<(bool, crate::price_layout::Rect, i32, i32)>>,
}
impl Backdrop {
    pub fn new(close: impl Fn() + 'static) -> Rc<Self> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Awakened PoE Trade — backdrop");
        window.set_app_paintable(true);
        window.set_accept_focus(false);
        if let Some(v) = GtkWindowExt::screen(&window).and_then(|s| s.rgba_visual()) {
            window.set_visual(Some(&v));
        }
        window.init_layer_shell();
        window.set_namespace("awakened-poe-trade-rs-backdrop");
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_exclusive_zone(-1);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
        }
        window.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        let this = Rc::new(Self {
            window,
            panel: Cell::new(crate::price_layout::Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }),
            painted: Cell::new(false),
            main_ready: Cell::new(false),
            applied: Cell::new(None),
        });
        this.window.connect_draw(|_, cr| {
            cr.set_operator(gtk::cairo::Operator::Source);
            cr.set_source_rgba(0., 0., 0., 0.18);
            let _ = cr.paint();
            glib::Propagation::Stop
        });
        let weak = Rc::downgrade(&this);
        this.window.connect_realize(move |w| {
            if let Some(s) = weak.upgrade() {
                s.apply();
            }
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                clock.connect_after_paint(move |_| {
                    if let Some(s) = weak.upgrade() {
                        s.painted.set(true);
                        s.apply();
                    }
                });
            }
        });
        this.window.connect_button_press_event(move |_, _| {
            close();
            glib::Propagation::Stop
        });
        this
    }
    fn apply(&self) {
        if let Some(w) = self.window.window() {
            let ready = self.window.is_visible() && self.painted.get() && self.main_ready.get();
            let key = (
                ready,
                self.panel.get(),
                self.window.allocated_width(),
                self.window.allocated_height(),
            );
            if self.applied.get() == Some(key) {
                return;
            }
            self.applied.set(Some(key));
            let region = gtk::cairo::Region::create();
            if self.window.is_visible() && self.painted.get() && self.main_ready.get() {
                let _ = region.union_rectangle(&gtk::cairo::RectangleInt::new(
                    0,
                    0,
                    self.window.allocated_width(),
                    self.window.allocated_height(),
                ));
                let p = self.panel.get();
                let _ = region.subtract_rectangle(&gtk::cairo::RectangleInt::new(
                    p.x, p.y, p.width, p.height,
                ));
            }
            w.input_shape_combine_region(&region, 0, 0);
            // Wayland applies the new region with the next surface commit.
            self.window.queue_draw();
        }
    }
    pub fn position(&self, monitor: &gdk::Monitor, panel: crate::price_layout::Rect) {
        self.window.set_monitor(monitor);
        self.panel.set(panel);
        self.apply();
    }
    pub fn show(&self) {
        self.painted.set(false);
        self.main_ready.set(false);
        self.apply();
        self.window.show_all();
        self.window.queue_draw();
    }
    pub fn main_painted(&self) {
        self.main_ready.set(true);
        self.apply();
    }
    pub fn hide(&self) {
        self.main_ready.set(false);
        self.apply();
        self.window.hide();
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.window.is_visible(),"painted":self.painted.get(),"input_ready":self.window.is_visible() && self.painted.get() && self.main_ready.get(),"opacity":0.18,"keyboard_mode":format!("{:?}",self.window.keyboard_mode())})
    }
}
