//! Native multi-instance stopwatch HUD. A single canvas owns the widgets so
//! transparent pixels stay click-through and all instances share one idle clock.
use crate::stopwatch::{Config, State};
use gtk::{gdk, prelude::*};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Debug, Clone)]
pub enum Action {
    Toggle(u64, u64),
    Reset(u64, u64),
    Edit(u64, u64),
    Remove(u64, u64),
    Move { epoch: u64, id: u64, x: f64, y: f64 },
    FocusChanged,
    Tick,
    Escape(u64),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Context {
    game: bool,
    interactive: bool,
    exclusive: bool,
    manager: bool,
}
struct Row {
    config: Config,
    state: State,
    root: gtk::EventBox,
    label: gtk::Label,
    paused: gtk::Label,
    controls: gtk::Box,
    toolbar: gtk::Box,
    toggle: gtk::Button,
    reset: gtk::Button,
    edit: gtk::Button,
    mover: gtk::ToggleButton,
    handle: gtk::Label,
    remove: gtk::Button,
    hover: Cell<bool>,
}
#[derive(Clone, Copy)]
struct Drag {
    id: u64,
    start_x: f64,
    start_y: f64,
}
pub struct Runtime {
    window: gtk::Window,
    layout: gtk::Layout,
    css: gtk::CssProvider,
    rows: RefCell<Vec<Row>>,
    origin: Instant,
    context: Cell<Context>,
    dismissed: Cell<bool>,
    busy: Cell<bool>,
    epoch: Cell<u64>,
    rendered: Cell<bool>,
    layout_pending: Cell<bool>,
    timer: RefCell<Option<glib::SourceId>>,
    holds: RefCell<HashMap<u64, glib::SourceId>>,
    drag: Cell<Option<Drag>>,
    gesture: RefCell<Option<gtk::GestureDrag>>,
    scale: Cell<f64>,
    frames: Cell<u64>,
    draws: Cell<u64>,
    send: Box<dyn Fn(Action)>,
}
impl Runtime {
    pub fn new(send: impl Fn(Action) + 'static) -> Result<Rc<Self>, String> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Awakened PoE Trade — Chronomètres");
        window.set_widget_name("apt-stopwatches");
        let visual = GtkWindowExt::screen(&window)
            .and_then(|s| s.rgba_visual())
            .ok_or("Surface RGBA indisponible pour les chronomètres.")?;
        window.set_visual(Some(&visual));
        window.set_app_paintable(true);
        window.set_decorated(false);
        window.set_focus_on_map(false);
        window.set_accept_focus(false);
        window.init_layer_shell();
        window.set_namespace("awakened-poe-trade-rs-stopwatches");
        window.set_layer(Layer::Overlay);
        window.set_exclusive_zone(-1);
        for e in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(e, true);
        }
        window.set_keyboard_mode(KeyboardMode::None);
        let layout = gtk::Layout::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        layout.set_hexpand(true);
        layout.set_vexpand(true);
        window.add(&layout);
        let css = gtk::CssProvider::new();
        if let Some(screen) = GtkWindowExt::screen(&window) {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
        let r = Rc::new(Self {
            window,
            layout,
            css,
            rows: RefCell::new(Vec::new()),
            origin: Instant::now(),
            context: Cell::new(Context::default()),
            dismissed: Cell::new(false),
            busy: Cell::new(false),
            epoch: Cell::new(1),
            rendered: Cell::new(false),
            layout_pending: Cell::new(false),
            timer: RefCell::new(None),
            holds: RefCell::new(HashMap::new()),
            drag: Cell::new(None),
            gesture: RefCell::new(None),
            scale: Cell::new(1.),
            frames: Cell::new(0),
            draws: Cell::new(0),
            send: Box::new(send),
        });
        r.style(1.)?;
        let weak = Rc::downgrade(&r);
        r.window.connect_draw(move |_, cr| {
            if let Some(r) = weak.upgrade() {
                r.draws.set(r.draws.get() + 1);
            }
            cr.set_operator(gtk::cairo::Operator::Source);
            cr.set_source_rgba(0., 0., 0., 0.);
            let _ = cr.paint();
            cr.set_operator(gtk::cairo::Operator::Over);
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_realize(move |w| {
            empty(w);
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                clock.connect_after_paint(move |_| {
                    if let Some(r) = weak.upgrade() {
                        if r.window.is_visible() {
                            r.rendered.set(true);
                            r.frames.set(r.frames.get() + 1);
                            r.apply_input();
                        }
                    }
                });
            }
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_size_allocate(move |_, _| {
            if let Some(r) = weak.upgrade() {
                // Gtk ignores resize requests made inside the parent's allocation.
                // Reposition children after that allocation has completed.
                if !r.layout_pending.replace(true) {
                    let weak = Rc::downgrade(&r);
                    glib::idle_add_local_once(move || {
                        if let Some(r) = weak.upgrade() {
                            r.layout_pending.set(false);
                            r.layout_rows();
                        }
                    });
                }
            }
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_focus_in_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                (r.send)(Action::FocusChanged);
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_focus_out_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                (r.send)(Action::FocusChanged);
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_delete_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.dismiss();
            }
            glib::Propagation::Stop
        });
        // The gesture is attached to the unmoving canvas, so moving its child does
        // not change the coordinate system used to measure drag offsets.
        let weak = Rc::downgrade(&r);
        r.window.connect_key_press_event(move |_, e| {
            if e.keyval() == gdk::keys::constants::Escape {
                if let Some(r) = weak.upgrade() {
                    (r.send)(Action::Escape(r.epoch.get()));
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        let gesture = gtk::GestureDrag::new(&r.layout);
        r.gesture.replace(Some(gesture.clone()));
        gesture.set_button(1);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_begin(move |gesture, x, y| {
            if let Some(r) = weak.upgrade() {
                let target = r
                    .rows
                    .borrow()
                    .iter()
                    .rev()
                    .find(|row| {
                        row.mover.is_active()
                            && row.root.is_visible()
                            && center_hit(row, &r.window, x, y)
                    })
                    .map(|row| row.config.id);
                if r.can_input() && !r.busy.get() {
                    if let Some(id) = target {
                        r.drag.set(Some(Drag {
                            id,
                            start_x: x,
                            start_y: y,
                        }));
                        let (w, h) = r.viewport();
                        if let Some(row) = r
                            .rows
                            .borrow_mut()
                            .iter_mut()
                            .find(|row| row.config.id == id)
                        {
                            let _ = row.config.move_to(x, y, w, h);
                        }
                        r.layout_rows();
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                        return;
                    }
                }
                gesture.set_state(gtk::EventSequenceState::Denied);
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_update(move |_, dx, dy| {
            if let Some(r) = weak.upgrade() {
                if let Some(d) = r.drag.get() {
                    let (w, h) = r.viewport();
                    if let Some(row) = r
                        .rows
                        .borrow_mut()
                        .iter_mut()
                        .find(|row| row.config.id == d.id)
                    {
                        let _ = row.config.move_to(d.start_x + dx, d.start_y + dy, w, h);
                    }
                    r.layout_rows();
                }
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_end(move |_, _, _| {
            if let Some(r) = weak.upgrade() {
                if let Some(d) = r.drag.take() {
                    if let Some(c) = r
                        .rows
                        .borrow()
                        .iter()
                        .find(|row| row.config.id == d.id)
                        .map(|row| row.config.clone())
                    {
                        (r.send)(Action::Move {
                            epoch: r.epoch.get(),
                            id: d.id,
                            x: c.x,
                            y: c.y,
                        });
                    }
                }
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_cancel(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.drag.set(None);
            }
        });
        Ok(r)
    }
    fn style(&self, scale: f64) -> Result<(), String> {
        self.scale.set(scale);
        self.css.load_from_data(format!("#apt-stopwatches, #apt-stopwatches layout, #apt-stopwatches eventbox {{background-color: transparent;}} #apt-stopwatches .clock-face {{ background-color: rgba(226,232,240,0.3);border-radius: 5px;padding: {}px; }} #apt-stopwatches .clock-text {{font: {}px Monospace;color:white;}} #apt-stopwatches .clock-paused {{background-color:#c05621;color:white;border-radius:3px;padding:2px;}} #apt-stopwatches button {{color:white;background-image:none;background-color:rgba(29,29,29,0.86);padding:4px;}}",(4.*scale) as i32,(32.*scale) as i32).as_bytes()).map_err(|e|e.to_string())
    }
    fn now(&self) -> u64 {
        self.origin.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
    pub fn accepts(&self, epoch: u64) -> bool {
        self.epoch.get() == epoch && self.can_input()
    }
    fn can_input(&self) -> bool {
        let c = self.context.get();
        c.interactive
            && !c.exclusive
            && !self.dismissed.get()
            && self.window.is_visible()
            && self.rendered.get()
    }
    pub fn owns_focus(&self) -> bool {
        self.window.is_visible() && self.window.is_active()
    }
    pub fn resume(self: &Rc<Self>) {
        self.dismissed.set(false);
        self.render();
    }
    pub fn dismiss(self: &Rc<Self>) {
        self.dismissed.set(true);
        self.epoch.set(self.epoch.get() + 1);
        self.cancel_holds();
        self.drag.set(None);
        self.render();
    }
    pub fn set_context(
        self: &Rc<Self>,
        game: bool,
        interactive: bool,
        exclusive: bool,
        manager: bool,
    ) {
        let next = Context {
            game,
            interactive,
            exclusive,
            manager,
        };
        if next != self.context.get() {
            if !interactive || exclusive {
                self.epoch.set(self.epoch.get() + 1);
                self.cancel_holds();
                self.drag.set(None);
            }
            self.context.set(next);
        }
        self.render();
    }
    pub fn set_busy(self: &Rc<Self>, busy: bool) {
        self.busy.set(busy);
        self.render();
    }
    pub fn reconcile(self: &Rc<Self>, configs: &[Config], scale: f64) {
        self.cancel_holds();
        self.drag.set(None);
        self.epoch.set(self.epoch.get() + 1);
        let rebuild = (scale - self.scale.get()).abs() > f64::EPSILON;
        let _ = self.style(scale);
        let mut old = self.rows.take();
        for config in configs {
            if let Some(index) = old.iter().position(|r| r.config.id == config.id) {
                let mut row = old.remove(index);
                if rebuild {
                    let state = row.state.clone();
                    self.layout.remove(&row.root);
                    self.add_row(config.clone(), state);
                } else {
                    row.config = config.clone();
                    self.rows.borrow_mut().push(row);
                }
            } else {
                self.add_row(config.clone(), State::default());
            }
        }
        for row in old {
            self.layout.remove(&row.root);
        }
        self.render();
    }
    pub fn has_configs(&self) -> bool {
        !self.rows.borrow().is_empty()
    }
    pub fn visible_rendered(&self) -> bool {
        self.window.is_visible() && self.rendered.get()
    }
    fn add_row(self: &Rc<Self>, config: Config, state: State) {
        let root = gtk::EventBox::new();
        root.set_visible_window(true);
        root.add_events(
            gdk::EventMask::ENTER_NOTIFY_MASK
                | gdk::EventMask::LEAVE_NOTIFY_MASK
                | gdk::EventMask::BUTTON_PRESS_MASK,
        );
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        toolbar.set_size_request(-1, (30. * self.scale.get()) as i32);
        let edit = gtk::Button::with_label("Réglages");
        let mover = gtk::ToggleButton::with_label("Déplacer");
        let remove = gtk::Button::with_label("Supprimer");
        remove.set_tooltip_text(Some("Maintenir une seconde pour supprimer ce chronomètre."));
        for b in [
            edit.upcast_ref::<gtk::Widget>(),
            mover.upcast_ref(),
            remove.upcast_ref(),
        ] {
            toolbar.pack_start(b, true, true, 0);
        }
        content.pack_start(&toolbar, false, false, 0);
        let face = gtk::Overlay::new();
        face.style_context().add_class("clock-face");
        face.connect_draw(|w, cr| {
            cr.set_source_rgba(226. / 255., 232. / 255., 240. / 255., 0.3);
            cr.rectangle(
                0.,
                0.,
                f64::from(w.allocated_width()),
                f64::from(w.allocated_height()),
            );
            let _ = cr.fill();
            glib::Propagation::Proceed
        });
        let label = gtk::Label::new(Some("00:00:00"));
        label.style_context().add_class("clock-text");
        face.add(&label);
        let paused = gtk::Label::new(Some("En pause"));
        paused.style_context().add_class("clock-paused");
        paused.set_halign(gtk::Align::End);
        paused.set_valign(gtk::Align::Start);
        face.add_overlay(&paused);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        controls.set_halign(gtk::Align::Center);
        controls.set_valign(gtk::Align::Center);
        let toggle = gtk::Button::with_label("Démarrer");
        let reset = gtk::Button::with_label("↻");
        reset.set_tooltip_text(Some("Remettre à zéro"));
        controls.pack_start(&toggle, false, false, 0);
        controls.pack_start(&reset, false, false, 0);
        face.add_overlay(&controls);
        let handle = gtk::Label::new(Some("✥"));
        handle.set_halign(gtk::Align::Center);
        handle.set_valign(gtk::Align::Center);
        handle.set_no_show_all(true);
        face.add_overlay(&handle);
        content.pack_start(&face, false, false, 0);
        root.add(&content);
        // Keep a fixed toolbar slot: the body, not the controls, is center-anchored.
        let toolbar_slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
        toolbar_slot.set_size_request(-1, (30. * self.scale.get()) as i32);
        content.remove(&toolbar);
        toolbar_slot.pack_start(&toolbar, false, false, 0);
        content.pack_start(&toolbar_slot, false, false, 0);
        content.reorder_child(&toolbar_slot, 0);
        toolbar.set_no_show_all(true);
        controls.set_no_show_all(true);
        paused.set_no_show_all(true);
        let id = config.id;
        let weak = Rc::downgrade(self);
        toggle.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                (r.send)(Action::Toggle(r.epoch.get(), id));
            }
        });
        let weak = Rc::downgrade(self);
        reset.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                (r.send)(Action::Reset(r.epoch.get(), id));
            }
        });
        let weak = Rc::downgrade(self);
        edit.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                (r.send)(Action::Edit(r.epoch.get(), id));
            }
        });
        let weak = Rc::downgrade(self);
        mover.connect_toggled(move |_| {
            if let Some(r) = weak.upgrade() {
                r.render();
            }
        });
        let weak = Rc::downgrade(self);
        root.connect_enter_notify_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                if let Some(row) = r.rows.borrow().iter().find(|row| row.config.id == id) {
                    row.hover.set(true);
                }
                r.render();
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        root.connect_leave_notify_event(move |_, e| {
            if e.detail() != gdk::NotifyType::Inferior {
                if let Some(r) = weak.upgrade() {
                    if let Some(row) = r.rows.borrow().iter().find(|row| row.config.id == id) {
                        row.hover.set(false);
                    }
                    r.cancel_hold(id);
                    r.render();
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        root.connect_button_press_event(move |w, _| {
            if let Some(r) = weak.upgrade() {
                if r.can_input() {
                    if let Some(w) = w.window() {
                        w.raise();
                    }
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        remove.connect_button_press_event(move |_, e| {
            if e.button() == 1 {
                if let Some(r) = weak.upgrade() {
                    r.start_hold(id);
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        remove.connect_button_release_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.cancel_hold(id);
            }
            glib::Propagation::Proceed
        });
        self.layout.put(&root, 0, 0);
        root.show_all();
        for child in toolbar.children() {
            child.show_all();
        }
        toolbar.show();
        let (_, natural) = toolbar.preferred_size();
        toolbar_slot.set_size_request(
            natural.width,
            natural.height.max((30. * self.scale.get()) as i32),
        );
        toolbar.hide();
        for child in controls.children() {
            child.show_all();
        }
        controls.hide();
        paused.hide();
        self.rows.borrow_mut().push(Row {
            config,
            state,
            root,
            label,
            paused,
            controls,
            toolbar,
            toggle,
            reset,
            edit,
            mover,
            handle,
            remove,
            hover: Cell::new(false),
        });
    }
    fn cancel_hold(&self, id: u64) {
        if let Some(s) = self.holds.borrow_mut().remove(&id) {
            s.remove();
        }
    }
    fn cancel_holds(&self) {
        for (_, s) in self.holds.borrow_mut().drain() {
            s.remove();
        }
    }
    fn start_hold(self: &Rc<Self>, id: u64) {
        if !self.can_input() || self.busy.get() {
            return;
        }
        self.cancel_hold(id);
        let epoch = self.epoch.get();
        let weak = Rc::downgrade(self);
        let source = glib::timeout_add_local_once(Duration::from_secs(1), move || {
            if let Some(r) = weak.upgrade() {
                r.holds.borrow_mut().remove(&id);
                if r.accepts(epoch) && !r.busy.get() {
                    (r.send)(Action::Remove(epoch, id));
                }
            }
        });
        self.holds.borrow_mut().insert(id, source);
    }
    pub fn time_action(self: &Rc<Self>, id: u64, reset: bool) {
        let now = self.now();
        let mut rows = self.rows.borrow_mut();
        let Some(row) = rows.iter_mut().find(|r| r.config.id == id) else {
            return;
        };
        {
            if reset {
                row.state.reset(now)
            } else {
                row.state.toggle(now)
            }
        }
        drop(rows);
        self.dismissed.set(false);
        self.render();
    }
    fn viewport(&self) -> (f64, f64) {
        (
            f64::from(self.window.allocated_width().max(1)),
            f64::from(self.window.allocated_height().max(1)),
        )
    }
    fn layout_rows(&self) {
        let (w, h) = self.viewport();
        self.layout.set_size(w as u32, h as u32);
        for row in self.rows.borrow().iter() {
            let (_, size) = row.root.preferred_size();
            let body = row
                .label
                .parent()
                .map(|w| w.preferred_size().1.height)
                .unwrap_or(1)
                .max(1);
            let toolbar = (size.height - body).max(0);
            let x = (row.config.x / 100. * w - f64::from(size.width) / 2.).round() as i32;
            let y = (row.config.y / 100. * h - f64::from(body) / 2. - f64::from(toolbar)).round()
                as i32;
            if self.layout.child_x(&row.root) != x || self.layout.child_y(&row.root) != y {
                self.layout.move_(&row.root, x, y);
                self.layout.queue_resize();
            }
        }
    }
    fn render(self: &Rc<Self>) {
        let c = self.context.get();
        let now = self.now();
        let allowed = !self.dismissed.get() && (c.game || c.interactive) && !c.exclusive;
        let mut any = false;
        let mut running = false;
        for row in self.rows.borrow().iter() {
            let visible = allowed
                && row
                    .state
                    .visible(row.config.wanted, c.interactive, c.exclusive);
            row.root.set_visible(visible);
            if visible {
                any = true;
                running |= row.state.running;
                let text = State::formatted(row.state.elapsed(now));
                if row.label.text() != text {
                    row.label.set_text(&text);
                }
                row.paused.set_visible(!row.state.running);
                let hover = c.interactive && row.hover.get();
                row.toolbar
                    .set_visible(c.interactive && (hover || row.mover.is_active()));
                row.handle
                    .set_visible(c.interactive && row.mover.is_active());
                row.controls.set_visible(hover && !row.mover.is_active());
                row.toggle.set_label(if row.state.running {
                    "Pause"
                } else {
                    "Démarrer"
                });
                for b in [&row.toggle, &row.reset, &row.edit, &row.remove] {
                    b.set_sensitive(!self.busy.get());
                }
                row.mover.set_sensitive(!self.busy.get());
            }
        }
        if any {
            if !self.window.is_visible() {
                self.rendered.set(false);
                empty(&self.window);
                self.window.show();
                self.layout.show();
            }
            self.layout_rows();
            if !c.interactive {
                empty(&self.window);
            }
        } else {
            self.rendered.set(false);
            empty(&self.window);
            self.window.hide();
        }
        if let Some(source) = self.timer.borrow_mut().take() {
            source.remove();
        }
        if (any && running) || (c.manager && self.rows.borrow().iter().any(|r| r.state.running)) {
            let weak = Rc::downgrade(self);
            let delay = self
                .rows
                .borrow()
                .iter()
                .filter(|r| r.root.is_visible() && r.state.running)
                .map(|r| 1000 - r.state.elapsed(now) % 1000)
                .min()
                .unwrap_or(1000)
                .max(1);
            let source = glib::timeout_add_local_once(Duration::from_millis(delay), move || {
                if let Some(r) = weak.upgrade() {
                    r.timer.borrow_mut().take();
                    let now = r.now();
                    for row in r.rows.borrow_mut().iter_mut() {
                        row.state.update(now);
                    }
                    r.render();
                }
            });
            self.timer.replace(Some(source));
        }
        self.apply_input();
        if c.manager {
            (self.send)(Action::Tick);
        }
    }
    fn apply_input(&self) {
        if !self.can_input() {
            empty(&self.window);
            return;
        }
        let region = gtk::cairo::Region::create();
        for row in self.rows.borrow().iter().filter(|r| r.root.is_visible()) {
            if let Some((x, y)) = row.root.translate_coordinates(&self.window, 0, 0) {
                let _ = region.union_rectangle(&gtk::cairo::RectangleInt::new(
                    x,
                    y,
                    row.root.allocated_width(),
                    row.root.allocated_height(),
                ));
            }
        }
        let _ = region.intersect_rectangle(&gtk::cairo::RectangleInt::new(
            0,
            0,
            self.window.allocated_width(),
            self.window.allocated_height(),
        ));
        if let Some(w) = self.window.window() {
            w.input_shape_combine_region(&region, 0, 0);
        }
        self.window.set_accept_focus(true);
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
    }
    pub fn readings(&self) -> Vec<crate::stopwatch::Reading> {
        let now = self.now();
        self.rows
            .borrow()
            .iter()
            .map(|r| {
                let millis = r.state.elapsed(now);
                crate::stopwatch::Reading {
                    id: r.config.id,
                    running: r.state.running,
                    millis,
                    formatted: State::formatted(millis),
                }
            })
            .collect()
    }
    pub fn snapshot(&self, details: bool) -> serde_json::Value {
        let now = self.now();
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(&self.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"available":true,"visible":self.window.is_visible(),"focused":self.window.is_active(),"keyboard_mode":format!("{:?}",self.window.keyboard_mode()),"rendered":self.rendered.get(),"epoch":self.epoch.get(),"dismissed":self.dismissed.get(),"scheduled_tick":self.timer.borrow().is_some(),"frames":self.frames.get(),"draws":self.draws.get(),"viewport":[self.window.allocated_width(),self.window.allocated_height()],"rows":self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.config.id,"title":r.config.title,"running":r.state.running,"millis":r.state.elapsed(now),"formatted":State::formatted(r.state.elapsed(now)),"moving":r.mover.is_active(),"wanted":r.config.wanted,"invisible_on_blur":r.state.invisible_on_blur,"x":r.config.x,"y":r.config.y,"visible":r.root.is_visible(),"layout_position":[self.layout.child_x(&r.root),self.layout.child_y(&r.root)],"rect":rect(r.root.upcast_ref()),"controls":if details {Some(serde_json::json!({"toggle":rect(r.toggle.upcast_ref()),"reset":rect(r.reset.upcast_ref()),"edit":rect(r.edit.upcast_ref()),"move":rect(r.mover.upcast_ref()),"remove":rect(r.remove.upcast_ref())}))}else{None}})).collect::<Vec<_>>()})
    }
}
fn empty(w: &gtk::Window) {
    w.set_keyboard_mode(KeyboardMode::None);
    w.set_accept_focus(false);
    if let Some(g) = w.window() {
        g.input_shape_combine_region(&gtk::cairo::Region::create(), 0, 0);
    }
}
fn center_hit(row: &Row, window: &gtk::Window, x: f64, y: f64) -> bool {
    let Some((left, top)) = row.label.translate_coordinates(window, 0, 0) else {
        return false;
    };
    (x - f64::from(left) - f64::from(row.label.allocated_width()) / 2.).abs() <= 12.
        && (y - f64::from(top) - f64::from(row.label.allocated_height()) / 2.).abs() <= 12.
}
