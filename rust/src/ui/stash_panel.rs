//! Search presets on the shared canvas. Only an explicit button emits input.
use super::widget_canvas::{Canvas, Owner};
use crate::{stash_search::Config, widget_menu::Anchor, widget_order::Key};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};
#[derive(Debug, Clone)]
pub enum Action {
    Escape(u64),
    Edit(u64, u64),
    Hide(u64, u64),
    Remove(u64, u64),
    Search(u64, u64, u64),
    Enable(u64, u64, bool),
    Move(u64, Config),
}
struct Row {
    config: RefCell<Config>,
    root: gtk::EventBox,
    body: gtk::Box,
    toolbar: gtk::Box,
    edit: gtk::Button,
    hide: gtk::Button,
    remove: gtk::Button,
    mover: gtk::ToggleButton,
    enable: gtk::CheckButton,
    scroll: gtk::ScrolledWindow,
    buttons: Vec<(u64, gtk::Button)>,
    painted: Cell<bool>,
    drawn: Cell<bool>,
    hover: Cell<bool>,
    gesture: RefCell<Option<gtk::GestureDrag>>,
    drag: Cell<Option<(Anchor, f64, f64)>>,
}
pub struct Runtime {
    canvas: Rc<Canvas>,
    css: gtk::CssProvider,
    rows: RefCell<Vec<Rc<Row>>>,
    configs: RefCell<Vec<Config>>,
    scale: Cell<f64>,
    epoch: Cell<u64>,
    interactive: Cell<bool>,
    suppressed: Cell<bool>,
    busy: Cell<bool>,
    filling: Cell<bool>,
    holds: RefCell<HashMap<u64, glib::SourceId>>,
    layout_pending: Cell<bool>,
    send: Box<dyn Fn(Action)>,
}
impl Runtime {
    pub fn new(canvas: Rc<Canvas>, send: impl Fn(Action) + 'static) -> Result<Rc<Self>, String> {
        let css = gtk::CssProvider::new();
        if let Some(screen) = GtkWindowExt::screen(&canvas.window) {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
        let r = Rc::new(Self {
            canvas,
            css,
            rows: RefCell::new(Vec::new()),
            configs: RefCell::new(Vec::new()),
            scale: Cell::new(0.),
            epoch: Cell::new(0),
            interactive: Cell::new(false),
            suppressed: Cell::new(false),
            busy: Cell::new(false),
            filling: Cell::new(false),
            holds: RefCell::new(HashMap::new()),
            layout_pending: Cell::new(false),
            send: Box::new(send),
        });
        let weak = Rc::downgrade(&r);
        r.canvas.window.connect_size_allocate(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.queue_layout();
            }
        });
        let weak = Rc::downgrade(&r);
        r.canvas.window.connect_key_press_event(move |_, e| {
            if let Some(r) = weak.upgrade() {
                if e.keyval() == gdk::keys::constants::Escape && r.accepts(r.epoch.get()) {
                    (r.send)(Action::Escape(r.epoch.get()));
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        Ok(r)
    }
    pub fn reconcile(self: &Rc<Self>, configs: &[Config], scale: f64) {
        self.filling.set(true);
        if self.scale.replace(scale) != scale {
            let style=format!(".apt-stash {{background-color:transparent;}} .apt-stash .stash-body {{background-color:rgba(26,32,44,0.94);padding:{}px;border:1px solid #b89448;border-radius:4px;}} .apt-stash button {{color:#eee;background-image:none;background-color:#283444;padding:{}px;}} .apt-stash label {{color:#eee;font:{}px Sans;}}",4.*scale,4.*scale,16.*scale);
            if let Err(e) = self.css.load_from_data(style.as_bytes()) {
                eprintln!("stash style error: {e}");
            }
        }
        if *self.configs.borrow() != configs {
            self.cancel_holds();
            let mut previous = self.rows.take();
            let mut rows = Vec::new();
            for c in configs {
                let old = previous
                    .iter()
                    .position(|r| r.config.borrow().id == c.id)
                    .map(|i| previous.remove(i));
                let row = if let Some(old) = old {
                    let reusable = {
                        let before = old.config.borrow();
                        before.title == c.title
                            && before.entries == c.entries
                            && before.enable_hotkeys == c.enable_hotkeys
                    };
                    if reusable {
                        old.config.replace(c.clone());
                        old
                    } else {
                        let moving = old.mover.is_active();
                        self.canvas.unregister(Key::Stash(c.id));
                        self.canvas.layout.remove(&old.root);
                        let row = self.build(c.clone());
                        row.mover.set_active(moving);
                        row
                    }
                } else {
                    self.build(c.clone())
                };
                rows.push(row);
            }
            for row in previous {
                self.canvas.unregister(Key::Stash(row.config.borrow().id));
                self.canvas.layout.remove(&row.root);
            }
            self.configs.replace(configs.to_vec());
            self.rows.replace(rows);
        } else {
            for row in self.rows.borrow().iter() {
                if row.drag.get().is_none() {
                    if let Some(c) = configs.iter().find(|c| c.id == row.config.borrow().id) {
                        row.config.replace(c.clone());
                        row.enable.set_active(c.enable_hotkeys);
                    }
                }
            }
        }
        self.filling.set(false);
        self.render();
    }
    fn build(self: &Rc<Self>, config: Config) -> Rc<Row> {
        let id = config.id;
        let root = gtk::EventBox::new();
        root.style_context().add_class("apt-stash");
        root.set_visible_window(true);
        root.add_events(gdk::EventMask::ENTER_NOTIFY_MASK | gdk::EventMask::LEAVE_NOTIFY_MASK);
        let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
        root.add(&column);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        let edit = gtk::Button::with_label("Configurer");
        let mover = gtk::ToggleButton::with_label("Déplacer");
        let hide = gtk::Button::with_label("Masquer");
        let remove = gtk::Button::with_label("Supprimer");
        remove.set_tooltip_text(Some("Maintenir une seconde pour supprimer."));
        for b in [
            edit.upcast_ref::<gtk::Widget>(),
            mover.upcast_ref(),
            hide.upcast_ref(),
            remove.upcast_ref(),
        ] {
            toolbar.pack_start(b, false, false, 0);
        }
        column.pack_start(&toolbar, false, false, 0);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
        body.style_context().add_class("stash-body");
        column.pack_start(&body, false, false, 0);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let title = gtk::Label::new(Some(if config.title.is_empty() {
            "Untitled"
        } else {
            &config.title
        }));
        title.set_xalign(0.);
        title.set_max_width_chars(24);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        header.pack_start(&title, true, true, 0);
        let enable = gtk::CheckButton::with_label("Raccourcis");
        enable.set_active(config.enable_hotkeys);
        header.pack_start(&enable, false, false, 0);
        body.pack_start(&header, false, false, 0);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let mut buttons = Vec::new();
        for e in &config.entries {
            let b = gtk::Button::new();
            let label = gtk::Label::new(Some(&format!(
                "{}{}",
                e.label(),
                if config.enable_hotkeys {
                    e.hotkey
                        .as_ref()
                        .map(|h| format!("   [{h}] "))
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            )));
            label.set_xalign(0.);
            label.set_max_width_chars(40);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            b.add(&label);
            b.set_tooltip_text(Some(&e.text));
            let weak = Rc::downgrade(self);
            let entry = e.id;
            b.connect_clicked(move |_| {
                if let Some(r) = weak.upgrade() {
                    (r.send)(Action::Search(r.epoch.get(), id, entry));
                }
            });
            list.pack_start(&b, false, false, 0);
            buttons.push((e.id, b));
        }
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_propagate_natural_height(true);
        scroll.set_propagate_natural_width(true);
        scroll.add(&list);
        body.pack_start(&scroll, false, false, 0);
        let row = Rc::new(Row {
            config: RefCell::new(config),
            root,
            body,
            toolbar,
            edit,
            hide,
            remove,
            mover,
            enable,
            scroll,
            buttons,
            painted: Cell::new(false),
            drawn: Cell::new(false),
            hover: Cell::new(false),
            gesture: RefCell::new(None),
            drag: Cell::new(None),
        });
        for (button, edit) in [(&row.edit, true), (&row.hide, false)] {
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(r) = weak.upgrade() {
                    (r.send)(if edit {
                        Action::Edit(r.epoch.get(), id)
                    } else {
                        Action::Hide(r.epoch.get(), id)
                    });
                }
            });
        }
        let weak = Rc::downgrade(self);
        row.enable.connect_toggled(move |b| {
            if let Some(r) = weak.upgrade() {
                if !r.filling.get() {
                    (r.send)(Action::Enable(r.epoch.get(), id, b.is_active()));
                }
            }
        });
        let weak = Rc::downgrade(self);
        row.remove.connect_button_press_event(move |_, e| {
            if e.button() == 1 {
                if let Some(r) = weak.upgrade() {
                    r.start_hold(id);
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        row.remove.connect_button_release_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.cancel_hold(id);
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        row.remove.connect_leave_notify_event(move |_, e| {
            if e.detail() != gdk::NotifyType::Inferior {
                if let Some(r) = weak.upgrade() {
                    r.cancel_hold(id);
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        row.root.connect_enter_notify_event(move |_, _| {
            if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                row.hover.set(true);
                r.render();
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        row.root.connect_leave_notify_event(move |_, e| {
            if e.detail() != gdk::NotifyType::Inferior {
                if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                    row.hover.set(false);
                    r.render();
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        row.mover.connect_toggled(move |_| {
            if let Some(r) = weak.upgrade() {
                r.render();
            }
        });
        let wr = Rc::downgrade(&row);
        row.root.connect_draw(move |_, _| {
            if let Some(row) = wr.upgrade() {
                row.drawn.set(true);
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        row.root.connect_realize(move |w| {
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                let wr = wr.clone();
                clock.connect_after_paint(move |_| {
                    if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                        if row.root.is_visible() && row.drawn.replace(false) {
                            row.painted.set(true);
                            r.apply_input();
                        }
                    }
                });
            }
        });
        let weak = Rc::downgrade(self);
        row.root.connect_size_allocate(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.queue_layout();
            }
        });
        let gesture = gtk::GestureDrag::new(&self.canvas.layout);
        gesture.set_button(1);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        gesture.connect_drag_begin(move |g, x, y| {
            if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                if r.accepts(r.epoch.get())
                    && row.mover.is_active()
                    && r.canvas.hit(x, y) == Some(Key::Stash(id))
                {
                    if let Some((bx, by)) = row.body.translate_coordinates(&r.canvas.window, 0, 0) {
                        for a in Anchor::ALL {
                            let (fx, fy) = a.fractions();
                            if (x - f64::from(bx) - fx * f64::from(row.body.allocated_width()))
                                .abs()
                                <= 14.
                                && (y - f64::from(by) - fy * f64::from(row.body.allocated_height()))
                                    .abs()
                                    <= 14.
                            {
                                row.drag.set(Some((a, x, y)));
                                r.move_row(&row, x, y);
                                g.set_state(gtk::EventSequenceState::Claimed);
                                return;
                            }
                        }
                    }
                }
            }
            g.set_state(gtk::EventSequenceState::Denied);
        });
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        gesture.connect_drag_update(move |_, dx, dy| {
            if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                if let Some((_, x, y)) = row.drag.get() {
                    r.move_row(&row, x + dx, y + dy);
                }
            }
        });
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        gesture.connect_drag_end(move |_, _, _| {
            if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                if row.drag.take().is_some() {
                    (r.send)(Action::Move(r.epoch.get(), row.config.borrow().clone()));
                }
            }
        });
        let wr = Rc::downgrade(&row);
        gesture.connect_cancel(move |_, _| {
            if let Some(row) = wr.upgrade() {
                row.drag.set(None);
            }
        });
        row.gesture.replace(Some(gesture));
        let wr = Rc::downgrade(&row);
        row.body.connect_draw(move |w, cr| {
            if let Some(row) = wr.upgrade() {
                if row.mover.is_active() {
                    cr.set_source_rgb(0.9, 0.7, 0.3);
                    for a in Anchor::ALL {
                        let (x, y) = a.fractions();
                        cr.rectangle(
                            (x * f64::from(w.allocated_width()) - 6.).max(0.),
                            (y * f64::from(w.allocated_height()) - 6.).max(0.),
                            12.,
                            12.,
                        );
                        let _ = cr.fill();
                    }
                }
            }
            glib::Propagation::Proceed
        });
        self.canvas.register(Key::Stash(id), &row.root);
        self.canvas.layout.put(&row.root, 0, 0);
        row.root.show_all();
        row.root.hide();
        row
    }
    fn move_row(&self, row: &Row, x: f64, y: f64) {
        if let Some((a, _, _)) = row.drag.get() {
            let mut p = row.config.borrow().position();
            if p.move_to(
                a,
                x,
                y,
                f64::from(self.canvas.window.allocated_width()),
                f64::from(self.canvas.window.allocated_height()),
            )
            .is_ok()
            {
                let mut c = row.config.borrow_mut();
                c.x = p.x;
                c.y = p.y;
                c.anchor = p.anchor;
                drop(c);
                self.layout();
            }
        }
    }
    pub fn set_context(self: &Rc<Self>, interactive: bool, exclusive: bool, epoch: u64) {
        if !interactive || exclusive || self.epoch.get() != epoch {
            self.cancel_holds();
            for row in self.rows.borrow().iter() {
                row.drag.set(None);
                row.painted.set(false);
                row.drawn.set(false);
                row.hover.set(false);
            }
        }
        self.epoch.set(epoch);
        self.interactive.set(interactive);
        self.suppressed.set(exclusive);
        self.render();
    }
    pub fn accepts(&self, epoch: u64) -> bool {
        epoch == self.epoch.get()
            && self.interactive.get()
            && !self.suppressed.get()
            && !self.busy.get()
            && self
                .rows
                .borrow()
                .iter()
                .any(|r| r.root.is_visible() && r.painted.get())
    }
    pub fn owns_focus(&self) -> bool {
        self.canvas.visible(Owner::Stash) && self.canvas.window.is_active()
    }
    pub fn set_busy(self: &Rc<Self>, busy: bool) {
        self.busy.set(busy);
        if busy {
            self.cancel_holds();
        }
        self.render();
    }
    fn queue_layout(self: &Rc<Self>) {
        if !self.layout_pending.replace(true) {
            let weak = Rc::downgrade(self);
            glib::idle_add_local_once(move || {
                if let Some(r) = weak.upgrade() {
                    r.layout_pending.set(false);
                    r.layout();
                }
            });
        }
    }
    fn layout(&self) {
        let w = self.canvas.window.allocated_width().max(1);
        let h = self.canvas.window.allocated_height().max(1);
        for row in self.rows.borrow().iter().filter(|r| r.root.is_visible()) {
            row.scroll
                .set_max_content_height((f64::from(h) * 0.55) as i32);
            if let Some(child) = row.scroll.child() {
                let needed = child
                    .preferred_height_for_width(row.scroll.allocated_width().max(1))
                    .1;
                row.scroll.set_min_content_height(needed.clamp(
                    (32. * self.scale.get()) as i32,
                    (f64::from(h) * 0.55).max(32. * self.scale.get()) as i32,
                ));
            }
            row.scroll
                .set_max_content_width((f64::from(w) * 0.85) as i32);
            let (width, height) = if row.body.allocated_width() > 1 {
                (row.body.allocated_width(), row.body.allocated_height())
            } else {
                let p = row.body.preferred_size().1;
                (p.width, p.height)
            };
            let (x, y) = row.config.borrow().position().origin(
                f64::from(w),
                f64::from(h),
                f64::from(width),
                f64::from(height),
            );
            let toolbar = row.toolbar.preferred_size().1.height + 2;
            let (x, y) = (
                (x.round() as i32).clamp(0, (w - row.root.allocated_width()).max(0)),
                (y.round() as i32 - toolbar).clamp(0, (h - height - toolbar).max(0)),
            );
            if self.canvas.layout.child_x(&row.root) != x
                || self.canvas.layout.child_y(&row.root) != y
            {
                self.canvas.layout.move_(&row.root, x, y);
                self.canvas.layout.queue_resize();
            }
        }
        self.apply_input();
    }
    fn render(self: &Rc<Self>) {
        let mut any = false;
        for row in self.rows.borrow().iter() {
            let c = row.config.borrow();
            let visible = self.interactive.get() && !self.suppressed.get() && c.wanted;
            any |= visible;
            row.enable
                .set_visible(c.entries.iter().any(|e| e.hotkey.is_some()));
            row.toolbar
                .set_opacity(if row.hover.get() || row.mover.is_active() {
                    1.
                } else {
                    0.
                });
            row.root.set_sensitive(!self.busy.get());
            if row.root.is_visible() != visible {
                row.painted.set(false);
                row.drawn.set(false);
            }
            row.root.set_visible(visible);
        }
        self.canvas.set_visible(Owner::Stash, any);
        self.queue_layout();
        self.apply_input();
    }
    fn apply_input(&self) {
        let region = gtk::cairo::Region::create();
        if self.interactive.get() && !self.suppressed.get() {
            for row in self
                .rows
                .borrow()
                .iter()
                .filter(|r| r.root.is_visible() && r.painted.get())
            {
                if let Some((x, y)) = row.root.translate_coordinates(&self.canvas.window, 0, 0) {
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
                self.canvas.window.allocated_width(),
                self.canvas.window.allocated_height(),
            ));
        }
        self.canvas.set_input(Owner::Stash, region);
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
        if !self.accepts(self.epoch.get()) {
            return;
        }
        self.cancel_hold(id);
        let weak = Rc::downgrade(self);
        let epoch = self.epoch.get();
        let s = glib::timeout_add_local_once(Duration::from_secs(1), move || {
            if let Some(r) = weak.upgrade() {
                r.holds.borrow_mut().remove(&id);
                if r.accepts(epoch) {
                    (r.send)(Action::Remove(epoch, id));
                }
            }
        });
        self.holds.borrow_mut().insert(id, s);
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(&self.canvas.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        let rows:Vec<_>=self.rows.borrow().iter().map(|r|{
            let c=r.config.borrow();let mut config=serde_json::to_value(&*c).unwrap_or_default();let visible=r.root.is_visible();
            if !visible{config.as_object_mut().unwrap().remove("entries");}
            serde_json::json!({"config":config,"entry_count":c.entries.len(),"entries_omitted":!visible,"visible":visible,"painted":r.painted.get(),"rect":rect(r.root.upcast_ref()),"body":rect(r.body.upcast_ref()),"edit":rect(r.edit.upcast_ref()),"hide":rect(r.hide.upcast_ref()),"remove":rect(r.remove.upcast_ref()),"move":rect(r.mover.upcast_ref()),"moving":r.mover.is_active(),"enable":rect(r.enable.upcast_ref()),"enable_value":r.enable.is_active(),"scroll":r.scroll.translate_coordinates(&self.canvas.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":r.scroll.allocated_width(),"height":r.scroll.allocated_height(),"value":r.scroll.vadjustment().value(),"upper":r.scroll.vadjustment().upper(),"mapped":r.scroll.is_mapped()})),"buttons":if visible{r.buttons.iter().map(|(id,b)|serde_json::json!({"id":id,"rect":rect(b.upcast_ref())})).collect::<Vec<_>>()}else{Vec::new()}})
        }).collect();
        serde_json::json!({"visible":self.canvas.visible(Owner::Stash),"rendered":self.rows.borrow().iter().any(|r|r.root.is_visible()&&r.painted.get()),"epoch":self.epoch.get(),"keyboard_mode":if self.canvas.input(Owner::Stash){"OnDemand"}else{"None"},"holds":self.holds.borrow().len(),"rows":rows})
    }
}
