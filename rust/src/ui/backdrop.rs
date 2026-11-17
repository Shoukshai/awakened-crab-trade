//! Outside-click surfaces occupy only the space beside the full-height panel.
//! No input-region hole: GTK decorations and popup grabs cannot turn it into
//! an invisible full-screen click catcher.
use gtk::{gdk, prelude::*};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{cell::Cell, rc::Rc};
struct Side {
    window: gtk::Window,
    width: Cell<i32>,
    painted: Cell<bool>,
    main_ready: Rc<Cell<bool>>,
    applied: Cell<Option<bool>>,
    pressed: Cell<Option<u32>>,
}
impl Side {
    fn apply(&self) {
        let ready = self.width.get() > 0
            && self.window.is_visible()
            && self.painted.get()
            && self.main_ready.get();
        if self.applied.get() == Some(ready) {
            return;
        }
        if let Some(w) = self.window.window() {
            self.applied.set(Some(ready));
            let region = if ready {
                gtk::cairo::Region::create_rectangle(&gtk::cairo::RectangleInt::new(
                    0,
                    0,
                    self.window.allocated_width(),
                    self.window.allocated_height(),
                ))
            } else {
                gtk::cairo::Region::create()
            };
            w.input_shape_combine_region(&region, 0, 0);
            self.window.queue_draw();
        }
    }
}
pub struct Backdrop {
    sides: Vec<Rc<Side>>,
    main_ready: Rc<Cell<bool>>,
    visible: Cell<bool>,
    epoch: Rc<Cell<u64>>,
}
impl Backdrop {
    pub fn new(close: impl Fn() + 'static) -> Rc<Self> {
        let main_ready = Rc::new(Cell::new(false));
        let epoch = Rc::new(Cell::new(0u64));
        let close: Rc<dyn Fn()> = Rc::new(close);
        let sides = [Edge::Left, Edge::Right]
            .into_iter()
            .map(|edge| {
                let window = gtk::Window::new(gtk::WindowType::Toplevel);
                window.set_title("Awakened PoE Trade — backdrop");
                window.set_decorated(false);
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
                for e in [Edge::Top, Edge::Bottom, edge] {
                    window.set_anchor(e, true);
                }
                window.add_events(
                    gdk::EventMask::BUTTON_PRESS_MASK | gdk::EventMask::BUTTON_RELEASE_MASK,
                );
                let side = Rc::new(Side {
                    window,
                    width: Cell::new(0),
                    painted: Cell::new(false),
                    main_ready: main_ready.clone(),
                    applied: Cell::new(None),
                    pressed: Cell::new(None),
                });
                side.window.connect_draw(|_, cr| {
                    cr.set_operator(gtk::cairo::Operator::Source);
                    cr.set_source_rgba(0., 0., 0., 0.18);
                    let _ = cr.paint();
                    glib::Propagation::Stop
                });
                let weak = Rc::downgrade(&side);
                side.window.connect_realize(move |w| {
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
                let weak = Rc::downgrade(&side);
                side.window.connect_size_allocate(move |_, _| {
                    if let Some(s) = weak.upgrade() {
                        s.applied.set(None);
                        s.apply();
                    }
                });
                let weak = Rc::downgrade(&side);
                side.window.connect_button_press_event(move |w, event| {
                    if let Some(s) = weak.upgrade() {
                        s.pressed.set(
                            (event.window() == w.window() && gtk::grab_get_current().is_none())
                                .then_some(event.button()),
                        );
                    }
                    glib::Propagation::Stop
                });
                let weak = Rc::downgrade(&side);
                let close = close.clone();
                let epoch = epoch.clone();
                side.window.connect_button_release_event(move |w, event| {
                    if let Some(s) = weak.upgrade() {
                        if s.pressed.take() == Some(event.button()) && event.window() == w.window()
                        {
                            // Hiding on press leaves GDK's device grab associated
                            // with this surface. Wait for release dispatch to end.
                            let close = close.clone();
                            let epoch = epoch.clone();
                            let token = epoch.get();
                            glib::idle_add_local_once(move || {
                                if epoch.get() == token {
                                    close();
                                }
                            });
                        }
                    }
                    glib::Propagation::Stop
                });
                side
            })
            .collect();
        Rc::new(Self {
            sides,
            main_ready,
            visible: Cell::new(false),
            epoch,
        })
    }
    pub fn position(&self, monitor: &gdk::Monitor, panel: crate::price_layout::Rect) {
        let g = monitor.geometry();
        for (side, width) in self
            .sides
            .iter()
            .zip([panel.x.max(0), (g.width() - panel.x - panel.width).max(0)])
        {
            side.window.set_monitor(monitor);
            side.width.set(width);
            side.applied.set(None);
            side.window.set_size_request(width.max(1), 0);
            side.window.resize(width.max(1), g.height());
            if width == 0 {
                side.window.hide();
            } else if self.visible.get() {
                side.window.show_all();
            }
            side.apply();
        }
    }
    pub fn show(&self) {
        self.epoch.set(self.epoch.get() + 1);
        self.visible.set(true);
        self.main_ready.set(false);
        for s in &self.sides {
            s.pressed.set(None);
            s.painted.set(false);
            s.apply();
            if s.width.get() > 0 {
                s.window.show_all();
                s.window.queue_draw();
            }
        }
    }
    pub fn main_painted(&self) {
        self.main_ready.set(true);
        for s in &self.sides {
            s.apply();
        }
    }
    pub fn hide(&self) {
        self.epoch.set(self.epoch.get() + 1);
        self.visible.set(false);
        self.main_ready.set(false);
        for s in &self.sides {
            s.pressed.set(None);
            s.apply();
            s.window.hide();
        }
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.visible.get(),"painted":self.sides.iter().filter(|s|s.width.get()>0).all(|s|s.painted.get()),"input_ready":self.visible.get() && self.main_ready.get() && self.sides.iter().filter(|s|s.width.get()>0).all(|s|s.painted.get()),"opacity":0.18,"keyboard_mode":"None","sides":self.sides.iter().map(|s|serde_json::json!({"width":s.width.get(),"allocated_width":s.window.allocated_width(),"visible":s.window.is_visible()})).collect::<Vec<_>>()})
    }
}
