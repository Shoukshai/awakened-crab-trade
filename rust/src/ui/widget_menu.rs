//! Floating native menu on the shared widget canvas. No game input is sent here.
use super::widget_canvas::{Canvas, Owner};
use crate::{
    widget_menu::{Anchor, Config, Entry, Label, Target},
    widget_order::Key,
};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Debug, Clone)]
pub enum Action {
    Painted(u64),
    Escape(u64),
    Toggle(u64, Target),
    Add(u64, bool),
    Configure(u64, Config),
    Paste(u64, String),
}
pub struct Menu {
    canvas: Rc<Canvas>,
    css: gtk::CssProvider,
    scale: Cell<f64>,
    message: gtk::Label,
    held_size: Cell<(i32, i32)>,
    viewport: Cell<(i32, i32)>,
    root: gtk::EventBox,
    body: gtk::Box,
    flow: gtk::FlowBox,
    scroll: gtk::ScrolledWindow,
    toolbar: gtk::Box,
    edit: gtk::ToggleButton,
    mover: gtk::ToggleButton,
    always: gtk::CheckButton,
    paste: gtk::TextView,
    plus: gtk::ToggleButton,
    add_box: gtk::Box,
    add_timer: gtk::Button,
    add_images: gtk::Button,
    rows: RefCell<Vec<(Target, gtk::Button)>>,
    labels: RefCell<Vec<Label>>,
    config: RefCell<Config>,
    epoch: Cell<u64>,
    visible: Cell<bool>,
    interactive: Cell<bool>,
    painted: Cell<bool>,
    busy: Cell<bool>,
    filling: Cell<bool>,
    hover: Cell<bool>,
    layout_pending: Cell<bool>,
    gesture: RefCell<Option<gtk::GestureDrag>>,
    drag: Cell<Option<(Anchor, f64, f64)>>,
    send: Box<dyn Fn(Action)>,
}
impl Menu {
    pub fn new(canvas: Rc<Canvas>, send: impl Fn(Action) + 'static) -> Result<Rc<Self>, String> {
        let root = gtk::EventBox::new();
        root.set_widget_name("apt-widget-menu");
        root.set_visible_window(true);
        root.add_events(gdk::EventMask::ENTER_NOTIFY_MASK | gdk::EventMask::LEAVE_NOTIFY_MASK);
        let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
        root.add(&column);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        let edit = gtk::ToggleButton::with_label("Configurer");
        let mover = gtk::ToggleButton::with_label("Déplacer");
        toolbar.pack_start(&edit, false, false, 0);
        toolbar.pack_start(&mover, false, false, 0);
        column.pack_start(&toolbar, false, false, 0);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
        body.style_context().add_class("menu-body");
        column.pack_start(&body, false, false, 0);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        body.pack_start(&row, false, false, 0);
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_max_children_per_line(6);
        flow.set_min_children_per_line(1);
        flow.set_column_spacing(3);
        flow.set_row_spacing(3);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_propagate_natural_height(true);
        scroll.set_propagate_natural_width(true);
        scroll.set_max_content_height(400);
        scroll.set_min_content_height(36);
        scroll.add(&flow);
        row.pack_start(&scroll, true, true, 0);
        let plus = gtk::ToggleButton::with_label("+");
        plus.set_tooltip_text(Some("Ajouter un widget"));
        row.pack_start(&plus, false, false, 0);
        let add_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let add_timer = gtk::Button::with_label("Chronomètre");
        let add_images = gtk::Button::with_label("Bande d’images");
        let stash = gtk::Button::with_label("Recherche de coffre");
        stash.set_sensitive(false);
        stash.set_tooltip_text(Some("Pas encore disponible dans cet aperçu."));
        for b in [&add_timer, &stash, &add_images] {
            add_box.pack_start(b, false, false, 0);
        }
        body.pack_start(&add_box, false, false, 0);
        let always = gtk::CheckButton::with_label("Toujours afficher les boutons des widgets");
        body.pack_start(&always, false, false, 0);
        let paste = gtk::TextView::new();
        paste.set_wrap_mode(gtk::WrapMode::WordChar);
        paste.set_accepts_tab(false);
        paste.set_size_request(300, 40);
        paste.set_tooltip_text(Some(
            "Coller ici le texte d’un objet pour consulter ses prix.",
        ));
        body.pack_start(&paste, false, false, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        message.set_max_width_chars(44);
        body.pack_start(&message, false, false, 0);
        let css = gtk::CssProvider::new();
        css.load_from_data(b"#apt-widget-menu {background-color:transparent;} #apt-widget-menu .menu-body {background-color:rgba(26,32,44,0.94);padding:6px;border:1px solid #b89448;border-radius:4px;} #apt-widget-menu button {color:#eee;background-image:none;background-color:#283444;padding:4px;} #apt-widget-menu label {color:#eee;} #apt-widget-menu textview text {background-color:#283444;color:#eee;} #apt-widget-menu .wanted {border-color:#b89448;}").map_err(|e|e.to_string())?;
        if let Some(s) = GtkWindowExt::screen(&canvas.window) {
            gtk::StyleContext::add_provider_for_screen(
                &s,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
        let r = Rc::new(Self {
            canvas,
            css,
            scale: Cell::new(0.),
            message,
            held_size: Cell::new((0, 0)),
            viewport: Cell::new((0, 0)),
            root,
            body,
            flow,
            scroll,
            toolbar,
            edit,
            mover,
            always,
            paste,
            plus,
            add_box,
            add_timer,
            add_images,
            rows: RefCell::new(Vec::new()),
            labels: RefCell::new(Vec::new()),
            config: RefCell::new(Config::default()),
            epoch: Cell::new(0),
            visible: Cell::new(false),
            interactive: Cell::new(false),
            painted: Cell::new(false),
            busy: Cell::new(false),
            filling: Cell::new(false),
            hover: Cell::new(false),
            layout_pending: Cell::new(false),
            gesture: RefCell::new(None),
            drag: Cell::new(None),
            send: Box::new(send),
        });
        r.canvas.register(Key::Menu, &r.root);
        r.canvas.layout.put(&r.root, 0, 0);
        r.root.show_all();
        r.root.hide();
        let weak = Rc::downgrade(&r);
        r.edit.connect_toggled(move |b| {
            if let Some(r) = weak.upgrade() {
                if b.is_active() {
                    r.mover.set_active(false);
                }
                r.render();
            }
        });
        let weak = Rc::downgrade(&r);
        r.mover.connect_toggled(move |b| {
            if let Some(r) = weak.upgrade() {
                if b.is_active() {
                    r.edit.set_active(false);
                }
                r.render();
            }
        });
        let weak = Rc::downgrade(&r);
        r.plus.connect_toggled(move |_| {
            if let Some(r) = weak.upgrade() {
                r.render();
            }
        });
        let weak = Rc::downgrade(&r);
        r.always.connect_toggled(move |b| {
            if let Some(r) = weak.upgrade() {
                if !r.filling.get() && r.accepts(r.epoch.get()) {
                    let mut c = r.config.borrow().clone();
                    c.always_show = b.is_active();
                    (r.send)(Action::Configure(r.epoch.get(), c));
                }
            }
        });
        for (button, images) in [(&r.add_timer, false), (&r.add_images, true)] {
            let weak = Rc::downgrade(&r);
            button.connect_clicked(move |_| {
                if let Some(r) = weak.upgrade() {
                    let epoch = r.epoch.get();
                    r.plus.set_active(false);
                    (r.send)(Action::Add(epoch, images));
                }
            });
        }
        r.reset_paste();
        let weak = Rc::downgrade(&r);
        r.root.connect_enter_notify_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.hover.set(true);
                r.render();
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.root.connect_leave_notify_event(move |_, e| {
            if e.detail() != gdk::NotifyType::Inferior {
                if let Some(r) = weak.upgrade() {
                    r.hover.set(false);
                    r.render();
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.canvas.window.connect_size_allocate(move |_, _| {
            if let Some(r) = weak.upgrade() {
                let size = (
                    r.canvas.window.allocated_width(),
                    r.canvas.window.allocated_height(),
                );
                if r.viewport.replace(size) != size {
                    r.reset_size();
                }
                r.queue_layout();
            }
        });
        let weak = Rc::downgrade(&r);
        r.root.connect_size_allocate(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.queue_layout();
            }
        });
        let weak = Rc::downgrade(&r);
        r.canvas.window.connect_realize(move |w| {
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                clock.connect_after_paint(move |_| {
                    if let Some(r) = weak.upgrade() {
                        if r.visible.get() {
                            r.retain_size();
                            if !r.painted.replace(true) {
                                (r.send)(Action::Painted(r.epoch.get()));
                            }
                            r.apply_input();
                        }
                    }
                });
            }
        });
        let weak = Rc::downgrade(&r);
        r.canvas.window.connect_key_press_event(move |_, e| {
            if e.keyval() == gdk::keys::constants::Escape {
                if let Some(r) = weak.upgrade() {
                    if r.accepts(r.epoch.get()) {
                        (r.send)(Action::Escape(r.epoch.get()));
                        return glib::Propagation::Stop;
                    }
                }
            }
            glib::Propagation::Proceed
        });
        let gesture = gtk::GestureDrag::new(&r.canvas.layout);
        gesture.set_button(1);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_begin(move |g, x, y| {
            if let Some(r) = weak.upgrade() {
                if r.accepts(r.epoch.get())
                    && r.mover.is_active()
                    && r.canvas.hit(x, y) == Some(Key::Menu)
                {
                    if let Some((bx, by)) = r.body.translate_coordinates(&r.canvas.window, 0, 0) {
                        for a in Anchor::ALL {
                            let (fx, fy) = a.fractions();
                            let hx = f64::from(bx) + fx * f64::from(r.body.allocated_width());
                            let hy = f64::from(by) + fy * f64::from(r.body.allocated_height());
                            if (x - hx).abs() <= 14. && (y - hy).abs() <= 14. {
                                r.drag.set(Some((a, x, y)));
                                r.move_drag(x, y);
                                g.set_state(gtk::EventSequenceState::Claimed);
                                return;
                            }
                        }
                    }
                }
            }
            g.set_state(gtk::EventSequenceState::Denied);
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_update(move |_, dx, dy| {
            if let Some(r) = weak.upgrade() {
                if let Some((_, x, y)) = r.drag.get() {
                    r.move_drag(x + dx, y + dy);
                }
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_end(move |_, _, _| {
            if let Some(r) = weak.upgrade() {
                if r.drag.take().is_some() {
                    (r.send)(Action::Configure(r.epoch.get(), r.config.borrow().clone()));
                }
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_cancel(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.drag.set(None);
            }
        });
        r.gesture.replace(Some(gesture));
        let weak = Rc::downgrade(&r);
        r.body.connect_draw(move |w, cr| {
            if let Some(r) = weak.upgrade() {
                if r.mover.is_active() {
                    cr.set_source_rgb(0.9, 0.7, 0.3);
                    for a in Anchor::ALL {
                        let (fx, fy) = a.fractions();
                        let x = fx * f64::from(w.allocated_width());
                        let y = fy * f64::from(w.allocated_height());
                        cr.rectangle((x - 6.).max(0.), (y - 6.).max(0.), 12., 12.);
                        let _ = cr.fill();
                    }
                }
            }
            glib::Propagation::Proceed
        });
        Ok(r)
    }
    fn reset_paste(self: &Rc<Self>) {
        // GTK owns asynchronous clipboard requests on their original buffer.
        // A new menu session must not accept a previous session's late paste.
        let weak = Rc::downgrade(self);
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        buffer.connect_changed(move |b| {
            if let Some(r) = weak.upgrade() {
                if r.paste.buffer().as_ref() != Some(b) || r.filling.get() {
                    return;
                }
                let text = b
                    .text(&b.start_iter(), &b.end_iter(), true)
                    .unwrap_or_default()
                    .to_string();
                if text.is_empty() {
                    return;
                }
                r.filling.set(true);
                b.set_text("");
                r.filling.set(false);
                if text.len() > 1024 * 1024 {
                    r.error("Texte trop volumineux (1 Mo maximum).");
                    return;
                }
                if r.accepts(r.epoch.get()) {
                    (r.send)(Action::Paste(r.epoch.get(), text));
                }
            }
        });
        self.paste.set_buffer(Some(&buffer));
    }
    pub fn accepts(&self, epoch: u64) -> bool {
        epoch == self.epoch.get()
            && self.visible.get()
            && self.interactive.get()
            && self.painted.get()
            && !self.busy.get()
    }
    pub fn owns_focus(&self) -> bool {
        self.visible.get() && self.canvas.window.is_active()
    }
    pub fn set_context(self: &Rc<Self>, visible: bool, interactive: bool, epoch: u64) {
        let changed = self.visible.get() != visible || self.epoch.get() != epoch;
        self.epoch.set(epoch);
        self.interactive.set(interactive);
        if changed {
            self.reset_paste();
            self.painted.set(false);
            self.drag.set(None);
            self.plus.set_active(false);
            if visible {
                self.canvas.window.queue_draw();
            }
        }
        if !visible {
            self.hover.set(false);
            self.reset_size();
        }
        self.visible.set(visible);
        self.render();
    }
    pub fn reconcile(self: &Rc<Self>, c: &Config, items: &[Entry], scale: f64) {
        if self.scale.replace(scale) != scale {
            let style=format!("#apt-widget-menu {{background-color:transparent;}} #apt-widget-menu .menu-body {{background-color:rgba(26,32,44,0.94);padding:{}px;border:1px solid #b89448;border-radius:4px;}} #apt-widget-menu button {{color:#eee;background-image:none;background-color:#283444;padding:{}px;}} #apt-widget-menu label {{color:#eee;font:{}px Sans;}} #apt-widget-menu textview text {{background-color:#283444;color:#eee;font:{}px Sans;}} #apt-widget-menu .wanted {{border-color:#b89448;}}",6.*scale,4.*scale,16.*scale,16.*scale);
            if let Err(e) = self.css.load_from_data(style.as_bytes()) {
                eprintln!("menu style error: {e}");
            }
            self.paste
                .set_size_request((300. * scale) as i32, (40. * scale) as i32);
        }
        self.config.replace(c.clone());
        self.add_timer.set_sensitive(
            items
                .iter()
                .filter(|e| matches!(e.target, Target::Stopwatch(_)))
                .count()
                < crate::stopwatch::MAX_INSTANCES,
        );
        self.add_images.set_sensitive(
            items
                .iter()
                .filter(|e| matches!(e.target, Target::Image(_)))
                .count()
                < crate::image_strip::MAX_STRIPS,
        );
        self.filling.set(true);
        self.always.set_active(c.always_show);
        self.filling.set(false);
        let labels = crate::widget_menu::entries(items, c.always_show);
        if *self.labels.borrow() != labels {
            self.rows.borrow_mut().clear();
            for child in self.flow.children() {
                self.flow.remove(&child);
            }
            for item in &labels {
                let b = gtk::Button::new();
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 3);
                if let Some(icon) = &item.icon {
                    let name = match icon.as_str() {
                        "fa-cog" => "preferences-system-symbolic",
                        "fa-search" => "system-search-symbolic",
                        _ => "applications-graphics-symbolic",
                    };
                    row.pack_start(
                        &gtk::Image::from_icon_name(Some(name), gtk::IconSize::Button),
                        false,
                        false,
                        0,
                    );
                }
                if let Some(title) = &item.title {
                    let label = gtk::Label::new(Some(title));
                    label.set_max_width_chars(24);
                    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    row.pack_start(&label, false, false, 0);
                }
                b.add(&row);
                if item.wanted {
                    b.style_context().add_class("wanted");
                }
                let tip = match &item.target {
                    Target::Settings => "Réglages".into(),
                    Target::Catalogue => "Recherche des cibles Heist".into(),
                    Target::Stopwatch(id) => format!("Chronomètre #{id}"),
                    Target::Image(id) => format!("Bande d’images #{id}"),
                };
                b.set_tooltip_text(Some(&tip));
                let target = item.target.clone();
                let weak = Rc::downgrade(self);
                b.connect_clicked(move |_| {
                    if let Some(r) = weak.upgrade() {
                        (r.send)(Action::Toggle(r.epoch.get(), target.clone()));
                    }
                });
                self.flow.insert(&b, -1);
                self.rows.borrow_mut().push((item.target.clone(), b));
            }
            self.flow.show_all();
            self.labels.replace(labels);
        }
        self.render();
    }
    pub fn set_busy(self: &Rc<Self>, busy: bool) {
        self.busy.set(busy);
        self.root.set_sensitive(!busy);
    }
    fn move_drag(&self, x: f64, y: f64) {
        if let Some((a, _, _)) = self.drag.get() {
            let w = f64::from(self.canvas.window.allocated_width().max(1));
            let h = f64::from(self.canvas.window.allocated_height().max(1));
            let _ = self.config.borrow_mut().move_to(a, x, y, w, h);
            self.layout();
        }
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
    pub fn error(&self, text: &str) {
        self.message.set_text(text);
        self.message.set_visible(!text.is_empty());
    }
    fn reset_size(&self) {
        if self.held_size.replace((0, 0)) != (0, 0) {
            self.body.set_size_request(-1, -1);
        }
    }
    fn retain_size(&self) {
        // Keep the painted menu under the pointer when its buttons disappear.
        // This matters with focus-follows-cursor and a bounded input union.
        // Reset on hide/output change; there is no periodic resizing or timer.
        let old = self.held_size.get();
        let next = (
            old.0
                .max(self.body.allocated_width())
                .min((f64::from(self.canvas.window.allocated_width()) * 0.9) as i32),
            old.1
                .max(self.body.allocated_height())
                .min((f64::from(self.canvas.window.allocated_height()) * 0.8) as i32),
        );
        if next != old {
            self.held_size.set(next);
            self.body.set_size_request(next.0, next.1);
        }
    }
    fn layout(&self) {
        if !self.visible.get() {
            return;
        }
        let w = f64::from(self.canvas.window.allocated_width().max(1));
        let h = f64::from(self.canvas.window.allocated_height().max(1));
        let columns = (((w * 0.85) / (180. * self.scale.get())).floor() as u32).clamp(1, 6);
        self.flow.set_max_children_per_line(columns);
        self.scroll.set_max_content_width((w * 0.82) as i32);
        self.scroll
            .set_max_content_height((h * 0.5).max(36.) as i32);
        // FlowBox/ScrolledWindow are height-for-width widgets. Their generic
        // preferred height need not equal the allocation that was painted.
        let preferred = self.body.preferred_size().1;
        let (width, height) = if self.root.is_mapped()
            && self.body.allocated_width() > 1
            && self.body.allocated_height() > 1
        {
            (self.body.allocated_width(), self.body.allocated_height())
        } else {
            (preferred.width, preferred.height)
        };
        let (x, y) = self
            .config
            .borrow()
            .origin(w, h, f64::from(width), f64::from(height));
        let toolbar = self.toolbar.preferred_size().1.height + 2;
        let x = (x.round() as i32).clamp(0, (w as i32 - width).max(0));
        let y = (y.round() as i32 - toolbar).clamp(0, (h as i32 - height - toolbar).max(0));
        if self.canvas.layout.child_x(&self.root) != x
            || self.canvas.layout.child_y(&self.root) != y
        {
            self.canvas.layout.move_(&self.root, x, y);
            self.canvas.layout.queue_resize();
        }
    }
    fn render(self: &Rc<Self>) {
        self.message.set_visible(!self.message.text().is_empty());
        self.paste.set_visible(!self.edit.is_active());
        self.always.set_visible(self.edit.is_active());
        self.add_box.set_visible(self.plus.is_active());
        self.toolbar.set_opacity(
            if self.hover.get() || self.mover.is_active() || self.edit.is_active() {
                1.
            } else {
                0.
            },
        );
        self.root.set_visible(self.visible.get());
        self.canvas.set_visible(Owner::Menu, self.visible.get());
        if self.visible.get() {
            self.layout();
        }
        self.apply_input();
    }
    fn apply_input(&self) {
        let region = gtk::cairo::Region::create();
        if self.visible.get() && self.painted.get() && self.interactive.get() {
            if let Some((x, y)) = self.root.translate_coordinates(&self.canvas.window, 0, 0) {
                let _ = region.union_rectangle(&gtk::cairo::RectangleInt::new(
                    x,
                    y,
                    self.root.allocated_width(),
                    self.root.allocated_height(),
                ));
            }
            let _ = region.intersect_rectangle(&gtk::cairo::RectangleInt::new(
                0,
                0,
                self.canvas.window.allocated_width(),
                self.canvas.window.allocated_height(),
            ));
        }
        self.canvas.set_input(Owner::Menu, region);
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(&self.canvas.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"visible":self.visible.get(),"rendered":self.painted.get(),"epoch":self.epoch.get(),"interactive":self.interactive.get(),"config":self.config.borrow().clone(),"scale":self.scale.get(),"buffer_scale":self.canvas.window.scale_factor(),"viewport":[self.canvas.window.allocated_width(),self.canvas.window.allocated_height()],"message":self.message.text().as_str(),"always_value":self.always.is_active(),"paste_length":self.paste.buffer().map(|b|b.char_count()),"rect":rect(self.root.upcast_ref()),"body":rect(self.body.upcast_ref()),"edit":rect(self.edit.upcast_ref()),"move":rect(self.mover.upcast_ref()),"moving":self.mover.is_active(),"always":rect(self.always.upcast_ref()),"plus":rect(self.plus.upcast_ref()),"add_timer":rect(self.add_timer.upcast_ref()),"add_images":rect(self.add_images.upcast_ref()),"paste":rect(self.paste.upcast_ref()),"paste_focused":self.paste.has_focus(),"scroll":self.scroll.translate_coordinates(&self.canvas.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":self.scroll.allocated_width(),"height":self.scroll.allocated_height(),"mapped":self.scroll.is_mapped(),"value":self.scroll.vadjustment().value(),"upper":self.scroll.vadjustment().upper(),"page":self.scroll.vadjustment().page_size()})),"rows":self.rows.borrow().iter().map(|(t,b)|serde_json::json!({"target":t,"rect":rect(b.upcast_ref())})).collect::<Vec<_>>()})
    }
}
