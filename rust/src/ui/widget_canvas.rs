//! One layer-shell surface and input union for all native floating widgets.
use crate::widget_order::{self, Key};
use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
#[derive(Clone, Copy)]
pub enum Owner {
    Stopwatch = 0,
    Image = 1,
    Menu = 2,
    Stash = 3,
}
pub struct Canvas {
    pub window: gtk::Window,
    pub layout: gtk::Layout,
    pub overlay: gtk::Overlay,
    visible: Cell<[bool; 4]>,
    regions: RefCell<[gtk::cairo::Region; 4]>,
    applied_region: RefCell<gtk::cairo::Region>,
    roots: RefCell<HashMap<Key, gtk::EventBox>>,
    known: RefCell<Vec<Key>>,
    order: RefCell<Vec<Key>>,
    dirty: Cell<bool>,
    gesture: RefCell<Option<gtk::GestureMultiPress>>,
    frames: Cell<u64>,
}
impl Canvas {
    pub fn new(
        changed: impl Fn() + 'static,
        closed: impl Fn() + 'static,
    ) -> Result<Rc<Self>, String> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Awakened PoE Trade — Widgets");
        window.set_widget_name("apt-widgets");
        let visual = GtkWindowExt::screen(&window)
            .and_then(|s| s.rgba_visual())
            .ok_or("Surface RGBA indisponible pour les widgets.")?;
        window.set_visual(Some(&visual));
        window.set_app_paintable(true);
        window.set_decorated(false);
        window.set_focus_on_map(false);
        window.set_accept_focus(false);
        window.init_layer_shell();
        window.set_namespace("awakened-poe-trade-rs-widgets");
        window.set_layer(Layer::Overlay);
        window.set_exclusive_zone(-1);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
        }
        window.set_keyboard_mode(KeyboardMode::None);
        let css = gtk::CssProvider::new();
        css.load_from_data(b"#apt-widgets, #apt-widgets layout {background-color:transparent;}")
            .map_err(|e| e.to_string())?;
        if let Some(screen) = GtkWindowExt::screen(&window) {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
        let overlay = gtk::Overlay::new();
        let layout = gtk::Layout::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        layout.set_hexpand(true);
        layout.set_vexpand(true);
        overlay.add(&layout);
        window.add(&overlay);
        layout.show();
        overlay.show();
        let r = Rc::new(Self {
            window,
            layout,
            overlay,
            visible: Cell::new([false; 4]),
            regions: RefCell::new([
                gtk::cairo::Region::create(),
                gtk::cairo::Region::create(),
                gtk::cairo::Region::create(),
                gtk::cairo::Region::create(),
            ]),
            applied_region: RefCell::new(gtk::cairo::Region::create()),
            roots: RefCell::new(HashMap::new()),
            known: RefCell::new(Vec::new()),
            order: RefCell::new(Vec::new()),
            dirty: Cell::new(false),
            gesture: RefCell::new(None),
            frames: Cell::new(0),
        });
        r.window.connect_draw(|_, cr| {
            cr.set_operator(gtk::cairo::Operator::Source);
            cr.set_source_rgba(0., 0., 0., 0.);
            let _ = cr.paint();
            cr.set_operator(gtk::cairo::Operator::Over);
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_realize(move |w| {
            w.set_keyboard_mode(KeyboardMode::None);
            w.set_accept_focus(false);
            if let Some(g) = w.window() {
                g.input_shape_combine_region(&gtk::cairo::Region::create(), 0, 0);
            }
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                clock.connect_after_paint(move |_| {
                    if let Some(r) = weak.upgrade() {
                        if r.window.is_visible() {
                            r.frames.set(r.frames.get() + 1);
                        }
                    }
                });
            }
        });
        // A capture gesture also sees button children which consume press events.
        // Restack on press, save after release so persistence cannot disable a
        // pressed button before GTK has delivered its clicked signal.
        let gesture = gtk::GestureMultiPress::new(&r.layout);
        gesture.set_button(1);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&r);
        gesture.connect_pressed(move |_, _, x, y| {
            if let Some(r) = weak.upgrade() {
                if r.window.keyboard_mode() == KeyboardMode::OnDemand {
                    if let Some(key) = r.hit(x, y) {
                        eprintln!("widgets press key={key:?} x={x} y={y}");
                        r.raise(key);
                    }
                }
            }
        });
        let changed: Rc<dyn Fn()> = Rc::new(changed);
        let weak = Rc::downgrade(&r);
        gesture.connect_released(move |_, _, _, _| {
            if let Some(r) = weak.upgrade() {
                if r.dirty.get() {
                    let changed = changed.clone();
                    glib::idle_add_local_once(move || changed());
                }
            }
        });
        r.gesture.replace(Some(gesture));
        r.window.connect_delete_event(move |_, _| {
            closed();
            glib::Propagation::Stop
        });
        Ok(r)
    }
    pub fn register(self: &Rc<Self>, key: Key, root: &gtk::EventBox) {
        self.roots.borrow_mut().insert(key, root.clone());
        let weak = Rc::downgrade(self);
        root.connect_realize(move |_| {
            if let Some(r) = weak.upgrade() {
                r.restack();
            }
        });
        let weak = Rc::downgrade(self);
        root.connect_map(move |_| {
            if let Some(r) = weak.upgrade() {
                r.restack();
            }
        });
        self.restack();
    }
    pub fn unregister(&self, key: Key) {
        self.roots.borrow_mut().remove(&key);
    }
    pub fn configure(&self, saved: &[Key], known: &[Key]) {
        self.known.replace(known.to_vec());
        let order = if self.dirty.get() {
            widget_order::reconcile(&self.order.borrow(), known)
        } else {
            widget_order::reconcile(saved, known)
        };
        self.order.replace(order);
        self.restack();
    }
    pub fn ack(&self, saved: &[Key]) {
        if self.saved() == saved {
            self.dirty.set(false);
        }
    }
    pub fn rollback(&self, saved: &[Key], known: &[Key]) {
        self.dirty.set(false);
        self.configure(saved, known);
    }
    pub fn dirty(&self) -> bool {
        self.dirty.get()
    }
    pub fn saved(&self) -> Vec<Key> {
        widget_order::saved(&self.order.borrow(), &self.known.borrow())
    }
    pub fn order(&self) -> Vec<Key> {
        self.order.borrow().clone()
    }
    pub fn raise(&self, key: Key) {
        if widget_order::raise(&mut self.order.borrow_mut(), key) {
            self.dirty.set(true);
            self.restack();
        }
    }
    fn restack(&self) {
        for key in self.order.borrow().iter() {
            if let Some(root) = self.roots.borrow().get(key) {
                if let Some(w) = root.window() {
                    w.raise();
                }
            }
        }
    }
    pub fn hit(&self, x: f64, y: f64) -> Option<Key> {
        self.order.borrow().iter().rev().copied().find(|key| {
            self.roots.borrow().get(key).is_some_and(|root| {
                root.is_visible()
                    && root
                        .translate_coordinates(&self.window, 0, 0)
                        .is_some_and(|(rx, ry)| {
                            x >= f64::from(rx)
                                && y >= f64::from(ry)
                                && x < f64::from(rx + root.allocated_width())
                                && y < f64::from(ry + root.allocated_height())
                        })
            })
        })
    }
    pub fn visible(&self, owner: Owner) -> bool {
        self.visible.get()[owner as usize]
    }
    pub fn set_visible(&self, owner: Owner, visible: bool) {
        let mut owners = self.visible.get();
        let changed = owners[owner as usize] != visible;
        owners[owner as usize] = visible;
        self.visible.set(owners);
        if !visible {
            self.set_input(owner, gtk::cairo::Region::create());
        }
        if owners.iter().any(|v| *v) {
            if !self.window.is_visible() {
                self.window.show();
            }
            if changed {
                self.window.queue_draw();
            }
        } else {
            self.window.hide();
        }
    }
    pub fn set_input(&self, owner: Owner, region: gtk::cairo::Region) {
        self.regions.borrow_mut()[owner as usize] = region;
        let union = gtk::cairo::Region::create();
        for (i, region) in self.regions.borrow().iter().enumerate() {
            if self.visible.get()[i] {
                let _ = union.union(region);
            }
        }
        let interactive = !union.is_empty() && self.window.is_visible();
        if let Some(w) = self.window.window() {
            w.input_shape_combine_region(&union, 0, 0);
        }
        // GDK sends a changed Wayland input region with the next surface commit.
        // after-paint alone does not schedule that commit for static widgets.
        // Queue exactly one frame for a changed union, never a perpetual tick.
        if *self.applied_region.borrow() != union {
            self.applied_region.replace(union);
            if self.window.is_visible() {
                self.window.queue_draw();
            }
        }
        self.window.set_accept_focus(interactive);
        self.window.set_keyboard_mode(if interactive {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        });
    }
    pub fn input(&self, owner: Owner) -> bool {
        !self.regions.borrow()[owner as usize].is_empty() && self.visible(owner)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let regions: Vec<_> = self
            .regions
            .borrow()
            .iter()
            .map(|r| {
                (0..r.num_rectangles())
                    .map(|i| {
                        let v = r.rectangle(i);
                        [v.x(), v.y(), v.width(), v.height()]
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        serde_json::json!({"visible":self.window.is_visible(),"focused":self.window.is_active(),"keyboard_mode":format!("{:?}",self.window.keyboard_mode()),"order":self.order.borrow().clone(),"dirty":self.dirty.get(),"frames":self.frames.get(),"one_surface":true,"input_regions":regions})
    }
}
