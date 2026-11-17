//! Native image strips, bounded asynchronous assets, and pointer-transparent
//! fullscreen previews. Every surface maps with empty input until its first paint.
use super::widget_canvas::{Canvas, Owner};
use crate::widget_order::Key;
use crate::{
    desktop::images as files,
    image_strip::{Anchor, Config},
};
use gtk::{
    gdk,
    gdk_pixbuf::{Pixbuf, PixbufAnimationIter, PixbufLoader},
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant, SystemTime},
};
#[derive(Debug, Clone)]
pub enum Action {
    FocusChanged,
    Escape(u64),
    Edit(u64, u64),
    Hide(u64, u64),
    Remove(u64, u64),
    Move { epoch: u64, config: Config },
}
struct Card {
    id: u64,
    url: String,
    root: gtk::EventBox,
    area: gtk::DrawingArea,
    hover: RefCell<crate::image_strip::Hover>,
}
struct Row {
    config: Config,
    root: gtk::EventBox,
    body: gtk::Box,
    toolbar: gtk::Box,
    edit: gtk::Button,
    hide: gtk::Button,
    remove: gtk::Button,
    mover: gtk::ToggleButton,
    cards: Vec<Card>,
    hover: Cell<bool>,
}
struct Asset {
    pixbuf: Pixbuf,
    animation: Option<PixbufAnimationIter>,
    origin: Instant,
    wall: SystemTime,
}
impl Asset {
    fn new(data: files::Decoded) -> Self {
        let started = Instant::now();
        let animated = data.animation.is_some();
        let pixbuf = data.pixbuf();
        let wall = SystemTime::now();
        let animation = data.animation.and_then(|bytes| {
            let loader = PixbufLoader::new();
            loader.connect_size_prepared(move |l, _, _| l.set_size(data.width, data.height));
            loader.write(&bytes).ok()?;
            loader.close().ok()?;
            Some(loader.animation()?.iter(Some(wall)))
        });
        if animated {
            eprintln!(
                "images animation_init_us={} animated={} network=false",
                started.elapsed().as_micros(),
                animation.is_some()
            );
        }
        Self {
            pixbuf,
            animation,
            origin: Instant::now(),
            wall,
        }
    }
    fn advance(&mut self) -> bool {
        if let Some(a) = &self.animation {
            if a.advance(self.wall + self.origin.elapsed()) {
                self.pixbuf = a.pixbuf();
                return true;
            }
        }
        false
    }
    fn delay(&self) -> Option<Duration> {
        self.animation
            .as_ref()
            .and_then(|a| a.delay_time())
            .map(|d| d.max(Duration::from_millis(10)))
    }
}
struct Preview {
    strip: u64,
    image: u64,
    url: String,
    token: u64,
    asset: Option<Asset>,
    error: Option<String>,
}
#[derive(Clone, Copy)]
struct Drag {
    id: u64,
    anchor: Anchor,
    x: f64,
    y: f64,
}
struct Job {
    revision: u64,
    url: String,
    width: i32,
    height: i32,
    cover: bool,
}
struct Loaded {
    revision: u64,
    url: String,
    result: Result<files::Decoded, String>,
}
#[allow(deprecated)] // Same pinned GLib actor channel as the other native panels.
fn actor() -> (mpsc::SyncSender<Job>, glib::Receiver<Loaded>) {
    let (tx, rx) = mpsc::sync_channel::<Job>(1);
    let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
    std::thread::spawn(move || {
        while let Ok(job) = rx.recv() {
            let started = Instant::now();
            let result = files::load(&job.url, job.width, job.height, job.cover);
            eprintln!(
                "images decode_us={} cover={} target={}x{} success={} network=false",
                started.elapsed().as_micros(),
                job.cover,
                job.width,
                job.height,
                result.is_ok()
            );
            if sender
                .send(Loaded {
                    revision: job.revision,
                    url: job.url,
                    result,
                })
                .is_err()
            {
                break;
            }
        }
    });
    (tx, receiver)
}
pub struct Runtime {
    canvas: Rc<Canvas>,
    window: gtk::Window,
    layout: gtk::Layout,
    preview_area: gtk::DrawingArea,
    css: gtk::CssProvider,
    rows: RefCell<Vec<Row>>,
    assets: RefCell<HashMap<String, Result<Asset, String>>>,
    preview: RefCell<Option<Preview>>,
    thumb_worker: mpsc::SyncSender<Job>,
    preview_worker: mpsc::SyncSender<Job>,
    thumb_pending: RefCell<VecDeque<String>>,
    thumb_busy: Cell<bool>,
    preview_pending: RefCell<Option<Job>>,
    preview_busy: Cell<bool>,
    revision: Cell<u64>,
    preview_token: Cell<u64>,
    epoch: Cell<u64>,
    interactive: Cell<bool>,
    exclusive: Cell<bool>,
    dismissed: Cell<bool>,
    busy: Cell<bool>,
    rendered: Cell<bool>,
    layout_pending: Cell<bool>,
    last_viewport: Cell<(i32, i32)>,
    buffer_scale: Cell<i32>,
    scale: Cell<f64>,
    holds: RefCell<HashMap<u64, glib::SourceId>>,
    drag: Cell<Option<Drag>>,
    gesture: RefCell<Option<gtk::GestureDrag>>,
    pointer: Cell<Option<(f64, f64)>>,
    animation_tick: RefCell<Option<glib::SourceId>>,
    frames: Cell<u64>,
    draws: Cell<u64>,
    send: Box<dyn Fn(Action)>,
}
impl Runtime {
    pub fn new(canvas: Rc<Canvas>, send: impl Fn(Action) + 'static) -> Result<Rc<Self>, String> {
        let window = canvas.window.clone();
        let layout = canvas.layout.clone();
        let overlay = canvas.overlay.clone();
        let preview_area = gtk::DrawingArea::new();
        preview_area.set_hexpand(true);
        preview_area.set_vexpand(true);
        preview_area.set_no_show_all(true);
        preview_area.set_sensitive(false);
        preview_area.connect_realize(|w| {
            if w.has_window() {
                if let Some(g) = w.window() {
                    g.input_shape_combine_region(&gtk::cairo::Region::create(), 0, 0);
                }
            }
        });
        preview_area.connect_size_allocate(|w, _| {
            if w.has_window() {
                if let Some(g) = w.window() {
                    g.input_shape_combine_region(&gtk::cairo::Region::create(), 0, 0);
                }
            }
        });
        overlay.add_overlay(&preview_area);
        overlay.set_overlay_pass_through(&preview_area, true);
        let css = gtk::CssProvider::new();
        if let Some(s) = GtkWindowExt::screen(&window) {
            gtk::StyleContext::add_provider_for_screen(
                &s,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
        css.load_from_data(b"#apt-image-strips, #apt-image-strips layout, #apt-image-strips eventbox {background-color:transparent;} #apt-image-strips label {color:#eee;font:16px Sans;} #apt-image-strips button {color:white;background-image:none;background-color:rgba(29,29,29,0.86);padding:4px;}").map_err(|e|e.to_string())?;
        let (thumb_worker, thumb_results) = actor();
        let (preview_worker, preview_results) = actor();
        let r = Rc::new(Self {
            canvas,
            window,
            layout,
            preview_area,
            css,
            rows: RefCell::new(Vec::new()),
            assets: RefCell::new(HashMap::new()),
            preview: RefCell::new(None),
            thumb_worker,
            preview_worker,
            thumb_pending: RefCell::new(VecDeque::new()),
            thumb_busy: Cell::new(false),
            preview_pending: RefCell::new(None),
            preview_busy: Cell::new(false),
            revision: Cell::new(1),
            preview_token: Cell::new(1),
            epoch: Cell::new(1),
            interactive: Cell::new(false),
            exclusive: Cell::new(false),
            dismissed: Cell::new(false),
            busy: Cell::new(false),
            rendered: Cell::new(false),
            layout_pending: Cell::new(false),
            last_viewport: Cell::new((1, 1)),
            buffer_scale: Cell::new(1),
            scale: Cell::new(1.),
            holds: RefCell::new(HashMap::new()),
            drag: Cell::new(None),
            gesture: RefCell::new(None),
            pointer: Cell::new(None),
            animation_tick: RefCell::new(None),
            frames: Cell::new(0),
            draws: Cell::new(0),
            send: Box::new(send),
        });
        let weak = Rc::downgrade(&r);
        thumb_results.attach(None, move |loaded| {
            if let Some(r) = weak.upgrade() {
                r.thumb_busy.set(false);
                if loaded.revision == r.revision.get() {
                    r.assets
                        .borrow_mut()
                        .insert(loaded.url, loaded.result.map(Asset::new));
                    r.redraw_assets();
                    r.schedule_animation();
                }
                r.next_thumbnail();
            }
            glib::ControlFlow::Continue
        });
        let weak = Rc::downgrade(&r);
        preview_results.attach(None, move |loaded| {
            if let Some(r) = weak.upgrade() {
                r.preview_busy.set(false);
                if let Some(p) = r.preview.borrow_mut().as_mut() {
                    if p.token == loaded.revision && p.url == loaded.url && r.can_input() {
                        match loaded.result {
                            Ok(a) => p.asset = Some(Asset::new(a)),
                            Err(e) => p.error = Some(e),
                        }
                        r.preview_area.show();
                        r.preview_area.queue_draw();
                    }
                }
                r.next_preview();
                r.schedule_animation();
            }
            glib::ControlFlow::Continue
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_draw(move |_, _| {
            if let Some(r) = weak.upgrade() {
                if r.canvas.visible(Owner::Image) {
                    r.draws.set(r.draws.get() + 1);
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_realize(move |w| {
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                clock.connect_after_paint(move |_| {
                    if let Some(r) = weak.upgrade() {
                        if r.canvas.visible(Owner::Image) {
                            r.frames.set(r.frames.get() + 1);
                            if !r.rendered.replace(true){eprintln!("images first_frame=painted geometry={}x{} buffer_scale={} activation_requested=false",r.window.allocated_width(),r.window.allocated_height(),r.window.scale_factor());}
                            r.apply_input();
                        }
                    }
                });
            }
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_size_allocate(move |w, _| {
            if let Some(r) = weak.upgrade() {
                let size = (w.allocated_width(), w.allocated_height());
                if r.last_viewport.replace(size) != size {
                    r.clear_preview();
                }
                if !r.layout_pending.replace(true) {
                    let weak = Rc::downgrade(&r);
                    glib::idle_add_local_once(move || {
                        if let Some(r) = weak.upgrade() {
                            r.layout_pending.set(false);
                            r.layout_rows();
                            r.load_thumbnails();
                        }
                    });
                }
            }
        });
        let weak = Rc::downgrade(&r);
        r.window.connect_scale_factor_notify(move |w| {
            if let Some(r) = weak.upgrade() {
                if r.buffer_scale.replace(w.scale_factor()) != w.scale_factor() {
                    r.revision.set(r.revision.get() + 1);
                    r.assets.borrow_mut().clear();
                    r.thumb_pending.borrow_mut().clear();
                    r.clear_preview();
                    r.load_thumbnails();
                    r.redraw_assets();
                }
            }
        });
        let weak = Rc::downgrade(&r);
        r.preview_area.connect_draw(move |w, cr| {
            if let Some(r) = weak.upgrade() {
                if let Some(p) = r.preview.borrow().as_ref() {
                    let width = f64::from(w.allocated_width());
                    let height = f64::from(w.allocated_height());
                    if let Some(a) = &p.asset {
                        let pw = f64::from(a.pixbuf.width());
                        let ph = f64::from(a.pixbuf.height());
                        let factor = ((width - 32.) / pw)
                            .min((height - 32.) / ph)
                            .clamp(0.001, 1.);
                        let x = (width - pw * factor) / 2.;
                        let y = (height - ph * factor) / 2.;
                        cr.set_source_rgb(0.1, 0.12, 0.16);
                        cr.rectangle(x - 4., y - 4., pw * factor + 8., ph * factor + 8.);
                        let _ = cr.fill();
                        paint(cr, &a.pixbuf, x, y, pw * factor, ph * factor, false);
                    } else if let Some(e) = &p.error {
                        cr.set_source_rgba(0., 0., 0., 0.85);
                        cr.rectangle(32., 32., width - 64., 48.);
                        let _ = cr.fill();
                        cr.set_source_rgb(1., 0.7, 0.7);
                        cr.move_to(45., 60.);
                        let _ = cr.show_text(e);
                    }
                }
            }
            glib::Propagation::Proceed
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
        r.window.connect_key_press_event(move |_, e| {
            if e.keyval() == gdk::keys::constants::Escape
                && weak.upgrade().is_some_and(|r| r.can_input())
            {
                if let Some(r) = weak.upgrade() {
                    (r.send)(Action::Escape(r.epoch.get()));
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        let gesture = gtk::GestureDrag::new(&r.layout);
        gesture.set_button(1);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        r.gesture.replace(Some(gesture.clone()));
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_begin(move |g, x, y| {
            if let Some(r) = weak.upgrade() {
                if r.can_input() && !r.busy.get() {
                    let target = r
                        .rows
                        .borrow()
                        .iter()
                        .rev()
                        .filter(|row| {
                            row.root.is_visible()
                                && row.mover.is_active()
                                && r.canvas.hit(x, y) == Some(Key::Image(row.config.id))
                        })
                        .find_map(|row| {
                            let (bx, by) = row.body.translate_coordinates(&r.window, 0, 0)?;
                            Anchor::ALL.into_iter().find_map(|a| {
                                let (fx, fy) = a.fractions();
                                let hx = f64::from(bx) + fx * f64::from(row.body.allocated_width());
                                let hy =
                                    f64::from(by) + fy * f64::from(row.body.allocated_height());
                                ((x - hx).abs() <= 12. && (y - hy).abs() <= 12.)
                                    .then_some((row.config.id, a))
                            })
                        });
                    if let Some((id, anchor)) = target {
                        r.clear_preview();
                        r.drag.set(Some(Drag { id, anchor, x, y }));
                        r.move_drag(x, y);
                        g.set_state(gtk::EventSequenceState::Claimed);
                        return;
                    }
                }
                g.set_state(gtk::EventSequenceState::Denied);
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_update(move |_, dx, dy| {
            if let Some(r) = weak.upgrade() {
                if let Some(d) = r.drag.get() {
                    r.move_drag(d.x + dx, d.y + dy);
                }
            }
        });
        let weak = Rc::downgrade(&r);
        gesture.connect_drag_end(move |_, _, _| {
            if let Some(r) = weak.upgrade() {
                if let Some(d) = r.drag.take() {
                    if let Some(config) = r
                        .rows
                        .borrow()
                        .iter()
                        .find(|r| r.config.id == d.id)
                        .map(|r| r.config.clone())
                    {
                        (r.send)(Action::Move {
                            epoch: r.epoch.get(),
                            config,
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
    fn can_input(&self) -> bool {
        self.interactive.get()
            && !self.exclusive.get()
            && !self.dismissed.get()
            && self.canvas.visible(Owner::Image)
            && self.rendered.get()
    }
    pub fn accepts(&self, epoch: u64) -> bool {
        epoch == self.epoch.get() && self.can_input()
    }
    pub fn owns_focus(&self) -> bool {
        self.canvas.visible(Owner::Image) && self.window.is_active()
    }
    pub fn has_wanted(&self) -> bool {
        self.rows.borrow().iter().any(|r| r.config.wanted)
    }
    pub fn prepare_edit(&self, id: u64) {
        if let Some(r) = self.rows.borrow().iter().find(|r| r.config.id == id) {
            r.mover.set_active(false);
        }
    }
    pub fn resume(self: &Rc<Self>) {
        self.dismissed.set(false);
        self.render();
    }
    pub fn dismiss(self: &Rc<Self>) {
        self.dismissed.set(true);
        self.set_context(false, true);
    }
    pub fn set_context(self: &Rc<Self>, interactive: bool, exclusive: bool) {
        if self.interactive.replace(interactive) != interactive || self.exclusive.get() != exclusive
        {
            self.epoch.set(self.epoch.get() + 1);
            self.cancel_holds();
            self.drag.set(None);
            self.clear_preview();
            if !interactive || exclusive {
                for row in self.rows.borrow().iter() {
                    row.hover.set(false);
                    for c in &row.cards {
                        c.hover.borrow_mut().leave();
                    }
                }
                self.pointer.set(None);
            }
        }
        self.exclusive.set(exclusive);
        self.render();
    }
    pub fn set_busy(self: &Rc<Self>, busy: bool) {
        self.busy.set(busy);
        self.render();
    }
    pub fn reconcile(self: &Rc<Self>, configs: &[Config], scale: f64) {
        self.epoch.set(self.epoch.get() + 1);
        self.cancel_holds();
        self.drag.set(None);
        self.clear_preview();
        let scale_changed = (scale - self.scale.get()).abs() > f64::EPSILON;
        self.scale.set(scale);
        let _=self.css.load_from_data(format!("#apt-image-strips, #apt-image-strips layout, #apt-image-strips eventbox {{background-color:transparent;}} #apt-image-strips label {{color:#eee;font:{}px Sans;}} #apt-image-strips button {{color:white;background-image:none;background-color:rgba(29,29,29,0.86);padding:4px;}}",(16.*scale) as i32).as_bytes());
        let mut old = self.rows.take();
        for c in configs {
            if let Some(i) = old.iter().position(|r| r.config.id == c.id) {
                let mut row = old.remove(i);
                if !scale_changed && row.config.title == c.title && row.config.images == c.images {
                    row.config = c.clone();
                    self.rows.borrow_mut().push(row);
                } else {
                    self.canvas.unregister(Key::Image(row.config.id));
                    self.layout.remove(&row.root);
                    self.add_row(c.clone());
                }
            } else {
                self.add_row(c.clone());
            }
        }
        for r in old {
            self.canvas.unregister(Key::Image(r.config.id));
            self.layout.remove(&r.root);
        }
        self.revision.set(self.revision.get() + 1);
        self.thumb_pending.borrow_mut().clear();
        if scale_changed {
            self.assets.borrow_mut().clear();
        }
        let used: std::collections::HashSet<_> = configs
            .iter()
            .flat_map(|c| c.images.iter().map(|i| i.url.clone()))
            .collect();
        self.assets
            .borrow_mut()
            .retain(|url, result| used.contains(url) && result.is_ok());
        self.render();
    }
    fn add_row(self: &Rc<Self>, config: Config) {
        let root = gtk::EventBox::new();
        root.set_widget_name("apt-image-strips");
        root.set_visible_window(true);
        root.add_events(
            gdk::EventMask::ENTER_NOTIFY_MASK
                | gdk::EventMask::LEAVE_NOTIFY_MASK
                | gdk::EventMask::BUTTON_PRESS_MASK,
        );
        let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        let hide = gtk::Button::with_label("Masquer");
        let edit = gtk::Button::with_label("Configurer");
        let mover = gtk::ToggleButton::with_label("Déplacer");
        let remove = gtk::Button::with_label("Supprimer");
        remove.set_tooltip_text(Some("Maintenir une seconde pour supprimer la bande."));
        for b in [
            hide.upcast_ref::<gtk::Widget>(),
            edit.upcast_ref(),
            mover.upcast_ref(),
            remove.upcast_ref(),
        ] {
            toolbar.pack_start(b, false, false, 0);
        }
        column.pack_start(&toolbar, false, false, 0);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 4);

        let title = gtk::Label::new(Some(if config.title.is_empty() {
            "Untitled"
        } else {
            &config.title
        }));
        title.set_margin_top(4);
        body.pack_start(&title, false, false, 0);
        let strip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        strip.set_border_width(4);
        body.pack_start(&strip, false, false, 0);
        column.pack_start(&body, false, false, 0);
        root.add(&column);
        let weak = Rc::downgrade(self);
        body.connect_draw(move |w, cr| {
            cr.set_source_rgba(0.1, 0.12, 0.16, 0.94);
            cr.rectangle(
                0.,
                0.,
                f64::from(w.allocated_width()),
                f64::from(w.allocated_height()),
            );
            let _ = cr.fill();
            if let Some(r) = weak.upgrade() {
                if let Some(row) = r.rows.borrow().iter().find(|row| row.body == *w) {
                    if row.mover.is_active() && r.can_input() {
                        for a in Anchor::ALL {
                            let (fx, fy) = a.fractions();
                            cr.set_source_rgb(0.8, 0.65, 0.3);
                            cr.rectangle(
                                fx * f64::from(w.allocated_width()) - 7.,
                                fy * f64::from(w.allocated_height()) - 7.,
                                14.,
                                14.,
                            );
                            let _ = cr.fill();
                        }
                    }
                }
            }
            glib::Propagation::Proceed
        });
        let mut cards = Vec::new();
        for image in &config.images {
            let card = gtk::EventBox::new();
            card.add_events(
                gdk::EventMask::POINTER_MOTION_MASK
                    | gdk::EventMask::LEAVE_NOTIFY_MASK
                    | gdk::EventMask::BUTTON_PRESS_MASK,
            );
            let area = gtk::DrawingArea::new();
            area.set_size_request(
                (192. * self.scale.get()) as i32,
                (104. * self.scale.get()) as i32,
            );
            card.add(&area);
            strip.pack_start(&card, false, false, 0);
            let url = image.url.clone();
            let weak = Rc::downgrade(self);
            area.connect_draw(move |w, cr| {
                cr.set_source_rgb(0.15, 0.2, 0.27);
                let _ = cr.paint();
                if let Some(r) = weak.upgrade() {
                    if let Some(a) = r.assets.borrow().get(&url) {
                        match a {
                            Ok(a) => paint(
                                cr,
                                &a.pixbuf,
                                4.,
                                4.,
                                f64::from(w.allocated_width() - 8),
                                f64::from(w.allocated_height() - 8),
                                true,
                            ),
                            Err(_) => {
                                cr.set_source_rgb(1., 0.7, 0.7);
                                cr.move_to(8., 24.);
                                let _ = cr.show_text("Image indisponible");
                            }
                        }
                    }
                }
                glib::Propagation::Proceed
            });
            let id = config.id;
            let image_id = image.id;
            let weak = Rc::downgrade(self);
            card.connect_motion_notify_event(move |w, e| {
                if let Some(r) = weak.upgrade() {
                    if let Some((ox, oy)) = w.translate_coordinates(&r.window, 0, 0) {
                        let (x, y) = e.position();
                        r.card_motion(id, image_id, f64::from(ox) + x, f64::from(oy) + y);
                    }
                }
                glib::Propagation::Proceed
            });
            let weak = Rc::downgrade(self);
            card.connect_leave_notify_event(move |w, e| {
                if e.detail() != gdk::NotifyType::Inferior {
                    if let Some(r) = weak.upgrade() {
                        if let Some((ox, oy)) = w.translate_coordinates(&r.window, 0, 0) {
                            let (x, y) = e.position();
                            r.pointer.set(Some((f64::from(ox) + x, f64::from(oy) + y)));
                        }
                        r.card_leave(id, image_id);
                    }
                }
                glib::Propagation::Proceed
            });
            let weak = Rc::downgrade(self);
            card.connect_button_press_event(move |_, e| {
                if e.button() == 1 {
                    if let Some(r) = weak.upgrade() {
                        r.preview_image(id, image_id, true);
                    }
                }
                glib::Propagation::Proceed
            });
            cards.push(Card {
                id: image.id,
                url: image.url.clone(),
                root: card,
                area,
                hover: RefCell::new(crate::image_strip::Hover::default()),
            });
        }
        if cards.is_empty() {
            strip.pack_start(
                &gtk::Label::new(Some("Bande vide · Configurer pour importer des images")),
                false,
                false,
                0,
            );
        }
        let id = config.id;
        for (button, kind) in [(&hide, 0), (&edit, 1)] {
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(r) = weak.upgrade() {
                    (r.send)(if kind == 0 {
                        Action::Hide(r.epoch.get(), id)
                    } else {
                        Action::Edit(r.epoch.get(), id)
                    });
                }
            });
        }
        let weak = Rc::downgrade(self);
        mover.connect_toggled(move |_| {
            if let Some(r) = weak.upgrade() {
                r.clear_preview();
                r.render();
                r.redraw_assets();
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
        let weak = Rc::downgrade(self);
        remove.connect_leave_notify_event(move |_, e| {
            if e.detail() != gdk::NotifyType::Inferior {
                if let Some(r) = weak.upgrade() {
                    r.cancel_hold(id);
                }
            }
            glib::Propagation::Proceed
        });
        self.canvas.register(Key::Image(config.id), &root);
        self.layout.put(&root, 0, 0);
        root.show_all();
        let (_, toolbar_size) = toolbar.preferred_size();
        let (_, body_size) = body.preferred_size();
        body.set_size_request(toolbar_size.width.max(body_size.width), -1);
        self.rows.borrow_mut().push(Row {
            config,
            root,
            body,
            toolbar,
            edit,
            hide,
            remove,
            mover,
            cards,
            hover: Cell::new(false),
        });
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
            let (_, size) = row.body.preferred_size();
            let (x, y) = row
                .config
                .origin(w, h, f64::from(size.width), f64::from(size.height));
            let toolbar = row.toolbar.preferred_size().1.height + 2;
            let (x, y) = (x.round() as i32, y.round() as i32 - toolbar);
            if self.layout.child_x(&row.root) != x || self.layout.child_y(&row.root) != y {
                self.layout.move_(&row.root, x, y);
                self.layout.queue_resize();
            }
        }
    }
    fn move_drag(&self, x: f64, y: f64) {
        if let Some(d) = self.drag.get() {
            let (w, h) = self.viewport();
            if let Some(row) = self
                .rows
                .borrow_mut()
                .iter_mut()
                .find(|row| row.config.id == d.id)
            {
                let _ = row.config.move_to(d.anchor, x, y, w, h);
            }
            self.layout_rows();
        }
    }
    fn render(self: &Rc<Self>) {
        let allowed = self.interactive.get() && !self.exclusive.get() && !self.dismissed.get();
        let mut any = false;
        for row in self.rows.borrow().iter() {
            let show = allowed && row.config.wanted;
            row.root.set_visible(show);
            any |= show;
            for b in [&row.hide, &row.edit, &row.remove] {
                b.set_sensitive(!self.busy.get());
            }
            row.mover.set_sensitive(!self.busy.get());
            row.toolbar
                .set_opacity(if row.hover.get() || row.mover.is_active() {
                    1.
                } else {
                    0.
                });
        }
        if any {
            if !self.canvas.visible(Owner::Image) {
                self.rendered.set(false);
                self.canvas.set_visible(Owner::Image, true);
            }
            self.layout_rows();
            self.load_thumbnails();
        } else {
            self.canvas.set_visible(Owner::Image, false);
            self.rendered.set(false);
            self.clear_preview();
        }
        self.apply_input();
        self.schedule_animation();
    }
    fn apply_input(&self) {
        if !self.can_input() {
            self.canvas
                .set_input(Owner::Image, gtk::cairo::Region::create());
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
        self.canvas.set_input(Owner::Image, region);
    }
    fn load_thumbnails(&self) {
        if !self.canvas.visible(Owner::Image) {
            return;
        }
        let (w, h) = self.viewport();
        let mut queue = self.thumb_pending.borrow_mut();
        for row in self.rows.borrow().iter().filter(|r| r.root.is_visible()) {
            for c in &row.cards {
                if let Some((x, y)) = c.root.translate_coordinates(&self.window, 0, 0) {
                    if f64::from(x) < w
                        && x + c.root.allocated_width() > 0
                        && f64::from(y) < h
                        && y + c.root.allocated_height() > 0
                        && !self.assets.borrow().contains_key(&c.url)
                        && !queue.contains(&c.url)
                    {
                        queue.push_back(c.url.clone());
                    }
                }
            }
        }
        drop(queue);
        self.next_thumbnail();
    }
    fn next_thumbnail(&self) {
        if self.thumb_busy.get() {
            return;
        }
        if let Some(url) = self.thumb_pending.borrow_mut().pop_front() {
            let job = Job {
                revision: self.revision.get(),
                url,
                width: ((192. * self.scale.get() * f64::from(self.window.scale_factor())) as i32)
                    .clamp(1, 4096),
                height: ((104. * self.scale.get() * f64::from(self.window.scale_factor())) as i32)
                    .clamp(1, 4096),
                cover: true,
            };
            if self.thumb_worker.try_send(job).is_ok() {
                self.thumb_busy.set(true);
            }
        }
    }
    fn next_preview(&self) {
        if self.preview_busy.get() {
            return;
        }
        if let Some(job) = self.preview_pending.borrow_mut().take() {
            if self.preview_worker.try_send(job).is_ok() {
                self.preview_busy.set(true);
            }
        }
    }
    fn preview_image(self: &Rc<Self>, id: u64, image_id: u64, click: bool) {
        if !self.can_input() || self.busy.get() {
            return;
        }
        let url = self
            .rows
            .borrow()
            .iter()
            .find(|r| r.config.id == id && !r.mover.is_active())
            .and_then(|r| r.cards.iter().find(|c| c.id == image_id))
            .and_then(|c| {
                if click {
                    c.hover.borrow_mut().click(false);
                }
                c.hover.borrow().hovered.then(|| c.url.clone())
            });
        let Some(url) = url else {
            return;
        };
        if self
            .preview
            .borrow()
            .as_ref()
            .is_some_and(|p| p.strip == id && p.image == image_id)
        {
            return;
        }
        let token = self.preview_token.get() + 1;
        self.preview_token.set(token);
        let (w, h) = self.viewport();
        eprintln!("images preview=request strip={id} image={image_id} token={token}");
        self.preview.replace(Some(Preview {
            strip: id,
            image: image_id,
            url: url.clone(),
            token,
            asset: None,
            error: None,
        }));
        self.preview_pending.replace(Some(Job {
            revision: token,
            url,
            width: ((w as i32 - 32) * self.window.scale_factor()).clamp(1, 4096),
            height: ((h as i32 - 32) * self.window.scale_factor()).clamp(1, 4096),
            cover: false,
        }));
        self.next_preview();
    }
    fn card_motion(self: &Rc<Self>, id: u64, image_id: u64, x: f64, y: f64) {
        let previous = self.pointer.replace(Some((x, y)));
        if !self.can_input() {
            return;
        }
        if let Some(card) = self
            .rows
            .borrow()
            .iter()
            .find(|r| r.config.id == id && !r.mover.is_active())
            .and_then(|r| r.cards.iter().find(|c| c.id == image_id))
        {
            if let Some((cx, cy)) = card.root.translate_coordinates(&self.window, 0, 0) {
                let (px, py) = previous.unwrap_or((x, y));
                card.hover.borrow_mut().motion(
                    false,
                    [
                        f64::from(cx),
                        f64::from(cy),
                        f64::from(cx + card.root.allocated_width()),
                        f64::from(cy + card.root.allocated_height()),
                    ],
                    [x, y],
                    [x - px, y - py],
                );
            }
        }
        self.preview_image(id, image_id, false);
    }
    fn card_leave(&self, id: u64, image_id: u64) {
        if let Some(card) = self
            .rows
            .borrow()
            .iter()
            .find(|r| r.config.id == id)
            .and_then(|r| r.cards.iter().find(|c| c.id == image_id))
        {
            card.hover.borrow_mut().leave();
        }
        if self
            .preview
            .borrow()
            .as_ref()
            .is_some_and(|p| p.strip == id && p.image == image_id)
        {
            eprintln!("images preview=leave strip={id} image={image_id}");
            self.clear_preview();
        }
    }
    fn clear_preview(&self) {
        if self.preview.borrow().is_some() {
            eprintln!("images preview=clear");
        }
        self.preview_token.set(self.preview_token.get() + 1);
        self.preview.take();
        self.preview_pending.take();
        self.preview_area.hide();
    }
    fn redraw_assets(&self) {
        for r in self.rows.borrow().iter() {
            r.body.queue_draw();
            for c in &r.cards {
                c.area.queue_draw();
            }
        }
        self.preview_area.queue_draw();
    }
    fn visible_urls(&self) -> std::collections::HashSet<String> {
        let (w, h) = self.viewport();
        self.rows
            .borrow()
            .iter()
            .filter(|r| r.root.is_visible())
            .flat_map(|r| r.cards.iter())
            .filter(|c| {
                c.root
                    .translate_coordinates(&self.window, 0, 0)
                    .is_some_and(|(x, y)| {
                        f64::from(x) < w
                            && x + c.root.allocated_width() > 0
                            && f64::from(y) < h
                            && y + c.root.allocated_height() > 0
                    })
            })
            .map(|c| c.url.clone())
            .collect()
    }
    fn schedule_animation(self: &Rc<Self>) {
        if let Some(t) = self.animation_tick.borrow_mut().take() {
            t.remove();
        }
        if !self.canvas.visible(Owner::Image) {
            return;
        }
        let visible = self.visible_urls();
        let thumb_delay = self
            .assets
            .borrow()
            .iter()
            .filter(|(url, _)| visible.contains(*url))
            .filter_map(|(_, a)| a.as_ref().ok().and_then(Asset::delay))
            .min();
        let preview_delay = self
            .preview
            .borrow()
            .as_ref()
            .and_then(|p| p.asset.as_ref())
            .and_then(Asset::delay);
        if let Some(delay) = thumb_delay.into_iter().chain(preview_delay).min() {
            let weak = Rc::downgrade(self);
            self.animation_tick
                .replace(Some(glib::timeout_add_local_once(delay, move || {
                    if let Some(r) = weak.upgrade() {
                        r.animation_tick.borrow_mut().take();
                        let visible = r.visible_urls();
                        for (url, result) in r.assets.borrow_mut().iter_mut() {
                            if visible.contains(url) {
                                if let Ok(a) = result {
                                    a.advance();
                                }
                            }
                        }
                        if let Some(a) = r
                            .preview
                            .borrow_mut()
                            .as_mut()
                            .and_then(|p| p.asset.as_mut())
                        {
                            a.advance();
                        }
                        r.redraw_assets();
                        r.schedule_animation();
                    }
                })));
        }
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
    pub fn diagnostics(&self, details: bool) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(&self.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"available":true,"visible":self.canvas.visible(Owner::Image),"focused":self.canvas.visible(Owner::Image)&&self.window.is_active(),"keyboard_mode":if self.canvas.input(Owner::Image) {"OnDemand"} else {"None"},"rendered":self.rendered.get(),"epoch":self.epoch.get(),"buffer_scale":self.window.scale_factor(),"viewport":[self.window.allocated_width(),self.window.allocated_height()],"frames":self.frames.get(),"draws":self.draws.get(),"scheduled_animation":self.animation_tick.borrow().is_some(),"pending_thumbnails":self.thumb_pending.borrow().len(),"loaded_assets":self.assets.borrow().len(),"preview":self.preview.borrow().as_ref().map(|p|serde_json::json!({"strip":p.strip,"image":p.image,"loaded":p.asset.is_some(),"error":p.error,"visible":self.preview_area.is_visible()})),"rows":self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.config.id,"title":r.config.title,"wanted":r.config.wanted,"x":r.config.x,"y":r.config.y,"anchor":r.config.anchor,"visible":r.root.is_visible(),"rect":rect(r.root.upcast_ref()),"body":rect(r.body.upcast_ref()),"moving":r.mover.is_active(),"controls":if details{Some(serde_json::json!({"edit":rect(r.edit.upcast_ref()),"hide":rect(r.hide.upcast_ref()),"move":rect(r.mover.upcast_ref()),"remove":rect(r.remove.upcast_ref())}))}else{None},"cards":if details{r.cards.iter().take(32).map(|c|serde_json::json!({"id":c.id,"url":c.url,"rect":rect(c.root.upcast_ref()),"loaded":self.assets.borrow().get(&c.url).map(|a|a.is_ok()),"animated":self.assets.borrow().get(&c.url).and_then(|a|a.as_ref().ok()).is_some_and(|a|a.animation.is_some()),"error":self.assets.borrow().get(&c.url).and_then(|a|a.as_ref().err()),"requires_leave":c.hover.borrow().requires_leave})).collect::<Vec<_>>()}else{Vec::new()}})).collect::<Vec<_>>()})
    }
}

fn paint(cr: &gtk::cairo::Context, pixbuf: &Pixbuf, x: f64, y: f64, w: f64, h: f64, cover: bool) {
    if w <= 0. || h <= 0. {
        return;
    }
    let _ = cr.save();
    cr.rectangle(x, y, w, h);
    cr.clip();
    let sx = w / f64::from(pixbuf.width());
    let sy = h / f64::from(pixbuf.height());
    let factor = if cover { sx.max(sy) } else { sx.min(sy) };
    cr.translate(
        x + (w - f64::from(pixbuf.width()) * factor) / 2.,
        y + (h - f64::from(pixbuf.height()) * factor) / 2.,
    );
    cr.scale(factor, factor);
    cr.set_source_pixbuf(pixbuf, 0., 0.);
    let _ = cr.paint();
    let _ = cr.restore();
}
