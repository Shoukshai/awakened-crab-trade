//! Detached per-instance configuration; the owner commits it atomically with
//! the global profile and shortcut registry.
use crate::{
    hotkeys::{Hotkey, StopwatchKeys},
    stopwatch::{Config, MAX_ID, MAX_INSTANCES},
};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Row {
    config: Config,
    root: gtk::Box,
    title: gtk::Entry,
    x: gtk::SpinButton,
    y: gtk::SpinButton,
    wanted: gtk::CheckButton,
    toggle: gtk::Entry,
    reset: gtk::Entry,
    record_toggle: gtk::Button,
    record_reset: gtk::Button,
    remove: gtk::Button,
}
pub struct Editor {
    pub container: gtk::Box,
    scroll: gtk::ScrolledWindow,
    list: gtk::Box,
    add: gtk::Button,
    message: gtk::Label,
    rows: RefCell<Vec<Row>>,
    next_id: Cell<u64>,
    capture: Cell<Option<(u64, bool)>>,
    global_capture: Rc<Cell<Option<usize>>>,
}
impl Editor {
    pub fn new(global_capture: Rc<Cell<Option<usize>>>) -> Rc<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let note=gtk::Label::new(Some("Les durées repartent de zéro au lancement. Les noms, positions et raccourcis sont enregistrés avec le profil. Les modifications restent en brouillon jusqu’à Enregistrer."));
        note.set_line_wrap(true);
        note.set_xalign(0.);
        container.pack_start(&note, false, false, 0);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(280);
        scroll.set_max_content_height(480);
        scroll.set_propagate_natural_height(true);
        scroll.add(&list);
        container.pack_start(&scroll, true, true, 0);
        let add = gtk::Button::with_label("Ajouter un chronomètre au brouillon");
        container.pack_start(&add, false, false, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        message.set_xalign(0.);
        container.pack_start(&message, false, false, 0);
        let editor = Rc::new(Self {
            container,
            scroll,
            list,
            add,
            message,
            rows: RefCell::new(Vec::new()),
            next_id: Cell::new(1),
            capture: Cell::new(None),
            global_capture,
        });
        let weak = Rc::downgrade(&editor);
        editor.add.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                let id = e.next_id.get();
                if e.rows.borrow().len() >= MAX_INSTANCES || id > MAX_ID {
                    return;
                }
                e.next_id.set(id + 1);
                e.append(
                    Config::new(
                        id,
                        glib::random_double_range(0., 1.),
                        glib::random_double_range(0., 1.),
                    ),
                    None,
                );
            }
        });
        editor
    }
    pub fn fill(self: &Rc<Self>, configs: &[Config], keys: &[StopwatchKeys]) {
        self.cancel_capture();
        self.rows.borrow_mut().clear();
        for w in self.list.children() {
            self.list.remove(&w);
        }
        self.next_id
            .set(configs.iter().map(|c| c.id).max().unwrap_or(0) + 1);
        for c in configs {
            self.append(c.clone(), keys.iter().find(|k| k.id == c.id));
        }
        self.scroll.vadjustment().set_value(0.);
        self.message.set_text("");
        self.add
            .set_sensitive(configs.len() < MAX_INSTANCES && self.next_id.get() <= MAX_ID);
    }
    fn append(self: &Rc<Self>, config: Config, keys: Option<&StopwatchKeys>) {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 5);
        root.set_border_width(8);
        let title = gtk::Entry::new();
        title.set_max_length(100);
        title.set_text(&config.title);
        title.set_placeholder_text(Some(&format!("Chronomètre #{}", config.id)));
        let label = gtk::Label::new(Some(&format!("Chronomètre #{}", config.id)));
        label.set_xalign(0.);
        root.pack_start(&label, false, false, 0);
        root.pack_start(&title, false, false, 0);
        let position = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let x = gtk::SpinButton::with_range(0., 100., 0.1);
        let y = gtk::SpinButton::with_range(0., 100., 0.1);
        super::settings_panel::short_number(&x);
        super::settings_panel::short_number(&y);
        x.set_value(config.x);
        y.set_value(config.y);
        position.pack_start(&gtk::Label::new(Some("Centre X %")), false, false, 0);
        position.pack_start(&x, true, true, 0);
        position.pack_start(&gtk::Label::new(Some("Y %")), false, false, 0);
        position.pack_start(&y, true, true, 0);
        root.pack_start(&position, false, false, 0);
        let wanted = gtk::CheckButton::with_label("Afficher ce widget");
        wanted.set_active(config.wanted);
        root.pack_start(&wanted, false, false, 0);
        let toggle = gtk::Entry::new();
        let reset = gtk::Entry::new();
        for entry in [&toggle, &reset] {
            entry.set_max_length(96);
            entry.set_placeholder_text(Some("Désactivé"));
        }
        toggle.set_text(keys.and_then(|k| k.toggle.as_deref()).unwrap_or(""));
        reset.set_text(keys.and_then(|k| k.reset.as_deref()).unwrap_or(""));
        let record_toggle = gtk::Button::with_label("Capturer");
        let record_reset = gtk::Button::with_label("Capturer");
        for (label, entry, button, is_reset) in [
            ("Démarrer / pause", &toggle, &record_toggle, false),
            ("Remettre à zéro", &reset, &record_reset, true),
        ] {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.pack_start(&gtk::Label::new(Some(label)), false, false, 0);
            row.pack_start(entry, true, true, 0);
            row.pack_start(button, false, false, 0);
            root.pack_start(&row, false, false, 0);
            let weak = Rc::downgrade(self);
            let id = config.id;
            button.connect_clicked(move |_|if let Some(e)=weak.upgrade(){e.global_capture.set(None);e.capture.set(Some((id,is_reset)));e.message.set_text("Appuyez sur la combinaison voulue ; Retour arrière désactive le raccourci.");});
            let weak = Rc::downgrade(self);
            button.connect_focus_out_event(move |_, _| {
                if let Some(e) = weak.upgrade() {
                    if e.capture.get() == Some((id, is_reset)) {
                        e.cancel_capture();
                    }
                }
                glib::Propagation::Proceed
            });
        }
        let remove = gtk::Button::with_label("Retirer ce chronomètre du brouillon");
        let weak = Rc::downgrade(self);
        let id = config.id;
        remove.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.cancel_capture();
                let mut rows = e.rows.borrow_mut();
                if let Some(index) = rows.iter().position(|r| r.config.id == id) {
                    let row = rows.remove(index);
                    e.list.remove(&row.root);
                }
                e.add
                    .set_sensitive(rows.len() < MAX_INSTANCES && e.next_id.get() <= MAX_ID);
            }
        });
        root.pack_start(&remove, false, false, 0);
        self.list.pack_start(&root, false, false, 0);
        self.rows.borrow_mut().push(Row {
            config,
            root,
            title,
            x,
            y,
            wanted,
            toggle,
            reset,
            record_toggle,
            record_reset,
            remove,
        });
        self.list.show_all();
        self.add.set_sensitive(
            self.rows.borrow().len() < MAX_INSTANCES && self.next_id.get() <= MAX_ID,
        );
    }
    pub fn read(&self) -> Result<(Vec<Config>, Vec<StopwatchKeys>), String> {
        let mut configs = Vec::new();
        let mut keys = Vec::new();
        for row in self.rows.borrow().iter() {
            row.x.update();
            row.y.update();
            let mut c = row.config.clone();
            c.title = row.title.text().into();
            c.x = row.x.value();
            c.y = row.y.value();
            c.wanted = row.wanted.is_active();
            c.validate()?;
            configs.push(c);
            let text = |e: &gtk::Entry| {
                let s = e.text().trim().to_owned();
                (!s.is_empty()).then_some(s)
            };
            let k = StopwatchKeys {
                id: row.config.id,
                toggle: text(&row.toggle),
                reset: text(&row.reset),
            };
            if k.toggle.is_some() || k.reset.is_some() {
                keys.push(k);
            }
        }
        Ok((configs, keys))
    }
    pub fn cancel_capture(&self) {
        self.capture.set(None);
    }
    pub fn capture_key(&self, event: &gdk::EventKey) -> bool {
        let Some((id, reset)) = self.capture.get() else {
            return false;
        };
        let entry = self
            .rows
            .borrow()
            .iter()
            .find(|r| r.config.id == id)
            .map(|r| {
                if reset {
                    r.reset.clone()
                } else {
                    r.toggle.clone()
                }
            });
        let Some(entry) = entry else {
            self.cancel_capture();
            return false;
        };
        if event.keyval() == gdk::keys::constants::BackSpace {
            entry.set_text("");
            self.cancel_capture();
            return true;
        }
        let symbol = super::settings_panel::unmodified_keyval(event);
        if (0xffe1..=0xffee).contains(&symbol) && symbol != 0xffe5 {
            return true;
        }
        let m = event.state();
        if let Some(h) = Hotkey::from_keysym(
            symbol,
            m.contains(gdk::ModifierType::CONTROL_MASK),
            m.contains(gdk::ModifierType::SHIFT_MASK),
            m.contains(gdk::ModifierType::MOD1_MASK),
            m.intersects(gdk::ModifierType::SUPER_MASK | gdk::ModifierType::MOD4_MASK),
        ) {
            entry.set_text(&h.text());
            self.cancel_capture();
            self.message
                .set_text("Raccourci capturé dans le brouillon.");
        }
        true
    }
    pub fn diagnostics(&self, window: &gtk::Window, details: bool) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"count":self.rows.borrow().len(),"capture":self.capture.get(),"message":self.message.text().as_str(),"add":rect(self.add.upcast_ref()),"scroll":rect(self.scroll.upcast_ref()),"rows":if details {self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.config.id,"title":{"value":r.title.text().as_str(),"rect":rect(r.title.upcast_ref())},"x":{"value":r.x.value(),"rect":rect(r.x.upcast_ref())},"y":{"value":r.y.value(),"rect":rect(r.y.upcast_ref())},"wanted":{"value":r.wanted.is_active(),"rect":rect(r.wanted.upcast_ref())},"toggle":{"value":r.toggle.text().as_str(),"rect":rect(r.toggle.upcast_ref())},"reset":{"value":r.reset.text().as_str(),"rect":rect(r.reset.upcast_ref())},"record_toggle":rect(r.record_toggle.upcast_ref()),"record_reset":rect(r.record_reset.upcast_ref()),"remove":rect(r.remove.upcast_ref())})).collect::<Vec<_>>()}else{Vec::new()}})
    }
}
