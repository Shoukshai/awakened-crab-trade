//! Draft-only stash widget editor. Files and shortcut grabs change on Apply.
use crate::{
    hotkeys::Hotkey,
    stash_search::{Config, Entry, MAX_ENTRIES, MAX_ID, MAX_WIDGETS},
    widget_menu::Anchor,
};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub const CAPTURE: usize = usize::MAX;
struct Filter {
    config: Entry,
    root: gtk::Box,
    text: gtk::Entry,
    name: gtk::Entry,
    key: gtk::Entry,
    record: gtk::Button,
    up: gtk::Button,
    down: gtk::Button,
    remove: gtk::Button,
}
struct Row {
    config: Config,
    root: gtk::Box,
    title: gtk::Entry,
    x: gtk::SpinButton,
    y: gtk::SpinButton,
    anchor: gtk::ComboBoxText,
    wanted: gtk::CheckButton,
    enable: gtk::CheckButton,
    list: gtk::Box,
    entries: RefCell<Vec<Filter>>,
    add: gtk::Button,
    remove: gtk::Button,
    next: Cell<u64>,
}
pub struct Editor {
    pub container: gtk::Box,
    scroll: gtk::ScrolledWindow,
    list: gtk::Box,
    add: gtk::Button,
    rows: RefCell<Vec<Rc<Row>>>,
    next: Cell<u64>,
    capture: Cell<Option<(u64, u64)>>,
    global_capture: Rc<Cell<Option<usize>>>,
    message: gtk::Label,
}
impl Editor {
    pub fn new(global_capture: Rc<Cell<Option<usize>>>) -> Rc<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(280);
        scroll.set_max_content_height(480);
        scroll.set_propagate_natural_height(true);
        scroll.add(&list);
        container.pack_start(&scroll, true, true, 0);
        let add = gtk::Button::with_label("Ajouter une recherche coffre au brouillon");
        container.pack_start(&add, false, false, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        container.pack_start(&message, false, false, 0);
        let r = Rc::new(Self {
            container,
            scroll,
            list,
            add,
            rows: RefCell::new(Vec::new()),
            next: Cell::new(1),
            capture: Cell::new(None),
            global_capture,
            message,
        });
        let weak = Rc::downgrade(&r);
        r.add.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                if r.rows.borrow().len() < MAX_WIDGETS && r.next.get() <= MAX_ID {
                    let id = r.next.get();
                    r.next.set(id + 1);
                    r.append(Config::new(
                        id,
                        glib::random_double(),
                        glib::random_double(),
                    ));
                }
            }
        });
        r
    }
    pub fn fill(self: &Rc<Self>, configs: &[Config]) {
        self.cancel_capture();
        for c in self.list.children() {
            self.list.remove(&c);
        }
        self.rows.borrow_mut().clear();
        self.next
            .set(configs.iter().map(|c| c.id).max().unwrap_or(0) + 1);
        for c in configs {
            self.append(c.clone());
        }
        self.scroll.vadjustment().set_value(0.);
        self.message.set_text("");
        self.update_add();
    }
    fn update_add(&self) {
        self.add
            .set_sensitive(self.rows.borrow().len() < MAX_WIDGETS && self.next.get() <= MAX_ID);
    }
    fn append(self: &Rc<Self>, config: Config) {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 5);
        root.set_border_width(8);
        let title = gtk::Entry::new();
        title.set_max_length(100);
        title.set_text(&config.title);
        title.set_placeholder_text(Some("Titre de recherche coffre"));
        root.pack_start(&title, false, false, 0);
        let pos = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let x = gtk::SpinButton::with_range(0., 100., 0.1);
        let y = gtk::SpinButton::with_range(0., 100., 0.1);
        super::settings_panel::short_number(&x);
        super::settings_panel::short_number(&y);
        x.set_value(config.x);
        y.set_value(config.y);
        let anchor = gtk::ComboBoxText::new();
        for (id, label) in [
            ("tl", "Haut gauche"),
            ("tr", "Haut droite"),
            ("bl", "Bas gauche"),
            ("br", "Bas droite"),
        ] {
            anchor.append(Some(id), label);
        }
        anchor.set_active_id(Some(anchor_id(config.anchor)));
        pos.pack_start(&gtk::Label::new(Some("X %")), false, false, 0);
        pos.pack_start(&x, true, true, 0);
        pos.pack_start(&gtk::Label::new(Some("Y %")), false, false, 0);
        pos.pack_start(&y, true, true, 0);
        pos.pack_start(&anchor, false, false, 0);
        root.pack_start(&pos, false, false, 0);
        let wanted = gtk::CheckButton::with_label("Afficher ce widget");
        wanted.set_active(config.wanted);
        let enable = gtk::CheckButton::with_label("Activer les raccourcis des filtres");
        enable.set_active(config.enable_hotkeys);
        root.pack_start(&wanted, false, false, 0);
        root.pack_start(&enable, false, false, 0);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
        root.pack_start(&list, false, false, 0);
        let add = gtk::Button::with_label("Ajouter un filtre");
        root.pack_start(&add, false, false, 0);
        let remove = gtk::Button::with_label("Retirer cette recherche coffre du brouillon");
        root.pack_start(&remove, false, false, 0);
        let next = config.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        let row = Rc::new(Row {
            config,
            root,
            title,
            x,
            y,
            anchor,
            wanted,
            enable,
            list,
            entries: RefCell::new(Vec::new()),
            add,
            remove,
            next: Cell::new(next),
        });
        for e in &row.config.entries {
            self.append_entry(&row, e.clone());
        }
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(&row);
        row.add.connect_clicked(move |_| {
            if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                let mut c = row.config.clone();
                c.entries = row
                    .entries
                    .borrow()
                    .iter()
                    .map(|e| e.config.clone())
                    .collect();
                if let Ok(id) = c.add_entry() {
                    row.next.set(id + 1);
                    r.append_entry(&row, c.entries.pop().unwrap());
                }
            }
        });
        let weak = Rc::downgrade(self);
        let id = row.config.id;
        row.remove.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                r.cancel_capture();
                let mut rows = r.rows.borrow_mut();
                if let Some(i) = rows.iter().position(|row| row.config.id == id) {
                    let row = rows.remove(i);
                    r.list.remove(&row.root);
                }
                drop(rows);
                r.update_add();
            }
        });
        self.list.pack_start(&row.root, false, false, 0);
        self.rows.borrow_mut().push(row);
        self.list.show_all();
        self.update_add();
    }
    fn append_entry(self: &Rc<Self>, row: &Rc<Row>, config: Entry) {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let text = gtk::Entry::new();
        text.set_max_length(250);
        text.set_text(&config.text);
        text.set_placeholder_text(Some("Filtre de recherche (250 caractères UTF-16 maximum)"));
        root.pack_start(&text, false, false, 0);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        let name = gtk::Entry::new();
        name.set_max_length(100);
        name.set_text(&config.name);
        name.set_placeholder_text(Some("Nom facultatif"));
        line.pack_start(&name, true, true, 0);
        let key = gtk::Entry::new();
        key.set_width_chars(14);
        key.set_max_length(96);
        key.set_text(config.hotkey.as_deref().unwrap_or(""));
        key.set_placeholder_text(Some("Raccourci désactivé"));
        line.pack_start(&key, true, true, 0);
        let record = gtk::Button::with_label("Capturer");
        let up = gtk::Button::with_label("↑");
        let down = gtk::Button::with_label("↓");
        let remove = gtk::Button::with_label("Retirer");
        for b in [&record, &up, &down, &remove] {
            line.pack_start(b, false, false, 0);
        }
        root.pack_start(&line, false, false, 0);
        let id = config.id;
        let widget = row.config.id;
        let weak = Rc::downgrade(self);
        record.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                r.capture.set(Some((widget, id)));
                r.global_capture.set(Some(CAPTURE));
                r.message
                    .set_text("Appuyez sur le raccourci ; Retour arrière le désactive.");
            }
        });
        let weak = Rc::downgrade(self);
        record.connect_focus_out_event(move |_, _| {
            if let Some(r) = weak.upgrade() {
                if r.capture.get() == Some((widget, id)) {
                    r.cancel_capture();
                }
            }
            glib::Propagation::Proceed
        });
        for (button, offset) in [(&up, -1isize), (&down, 1)] {
            let wr = Rc::downgrade(row);
            button.connect_clicked(move |_| {
                if let Some(row) = wr.upgrade() {
                    let mut entries = row.entries.borrow_mut();
                    if let Some(i) = entries.iter().position(|e| e.config.id == id) {
                        let target =
                            (i as isize + offset).clamp(0, entries.len() as isize - 1) as usize;
                        entries.swap(i, target);
                        for (i, e) in entries.iter().enumerate() {
                            row.list.reorder_child(&e.root, i as i32);
                        }
                    }
                }
            });
        }
        let weak = Rc::downgrade(self);
        let wr = Rc::downgrade(row);
        remove.connect_clicked(move |_| {
            if let (Some(r), Some(row)) = (weak.upgrade(), wr.upgrade()) {
                r.cancel_capture();
                let mut entries = row.entries.borrow_mut();
                if let Some(i) = entries.iter().position(|e| e.config.id == id) {
                    let e = entries.remove(i);
                    row.list.remove(&e.root);
                }
                row.next
                    .set(entries.iter().map(|e| e.config.id).max().unwrap_or(0) + 1);
                row.add
                    .set_sensitive(entries.len() < MAX_ENTRIES && row.next.get() <= MAX_ID);
            }
        });
        row.list.pack_start(&root, false, false, 0);
        row.entries.borrow_mut().push(Filter {
            config,
            root,
            text,
            name,
            key,
            record,
            up,
            down,
            remove,
        });
        row.list.show_all();
        row.add
            .set_sensitive(row.entries.borrow().len() < MAX_ENTRIES && row.next.get() <= MAX_ID);
    }
    pub fn read(&self) -> Result<Vec<Config>, String> {
        self.rows
            .borrow()
            .iter()
            .map(|r| {
                r.x.update();
                r.y.update();
                let mut c = r.config.clone();
                c.title = r.title.text().into();
                c.x = r.x.value();
                c.y = r.y.value();
                c.anchor = match r.anchor.active_id().as_deref() {
                    Some("tr") => Anchor::Tr,
                    Some("bl") => Anchor::Bl,
                    Some("br") => Anchor::Br,
                    _ => Anchor::Tl,
                };
                c.wanted = r.wanted.is_active();
                c.enable_hotkeys = r.enable.is_active();
                c.entries = r
                    .entries
                    .borrow()
                    .iter()
                    .map(|e| Entry {
                        id: e.config.id,
                        name: e.name.text().into(),
                        text: e.text.text().into(),
                        hotkey: (!e.key.text().trim().is_empty())
                            .then(|| e.key.text().trim().to_owned()),
                    })
                    .collect();
                c.validate()?;
                Ok(c)
            })
            .collect()
    }
    pub fn cancel_capture(&self) {
        self.capture.set(None);
        if self.global_capture.get() == Some(CAPTURE) {
            self.global_capture.set(None);
        }
    }
    pub fn capture_key(&self, event: &gdk::EventKey) -> bool {
        let Some((widget, id)) = self.capture.get() else {
            return false;
        };
        let entry = self
            .rows
            .borrow()
            .iter()
            .find(|r| r.config.id == widget)
            .and_then(|r| {
                r.entries
                    .borrow()
                    .iter()
                    .find(|e| e.config.id == id)
                    .map(|e| e.key.clone())
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
    pub fn select(&self, id: u64) {
        if let Some(row) = self.rows.borrow().iter().find(|r| r.config.id == id) {
            if let Some((_, y)) = row.root.translate_coordinates(&self.list, 0, 0) {
                self.scroll.vadjustment().set_value(f64::from(y));
            }
        }
    }
    pub fn diagnostics(&self, window: &gtk::Window, details: bool) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"count":self.rows.borrow().len(),"capture":self.capture.get(),"add":rect(self.add.upcast_ref()),"scroll":rect(self.scroll.upcast_ref()),"rows":if details{self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.config.id,"title":{"value":r.title.text().as_str(),"rect":rect(r.title.upcast_ref())},"x":rect(r.x.upcast_ref()),"y":rect(r.y.upcast_ref()),"anchor":rect(r.anchor.upcast_ref()),"wanted":rect(r.wanted.upcast_ref()),"enable":rect(r.enable.upcast_ref()),"add":rect(r.add.upcast_ref()),"remove":rect(r.remove.upcast_ref()),"entries":r.entries.borrow().iter().map(|e|serde_json::json!({"id":e.config.id,"text":{"value":e.text.text().as_str(),"rect":rect(e.text.upcast_ref())},"name":{"value":e.name.text().as_str(),"rect":rect(e.name.upcast_ref())},"hotkey":{"value":e.key.text().as_str(),"rect":rect(e.key.upcast_ref())},"record":rect(e.record.upcast_ref()),"up":rect(e.up.upcast_ref()),"down":rect(e.down.upcast_ref()),"remove":rect(e.remove.upcast_ref())})).collect::<Vec<_>>()})).collect::<Vec<_>>()}else{Vec::new()}})
    }
}
fn anchor_id(a: Anchor) -> &'static str {
    match a {
        Anchor::Tl => "tl",
        Anchor::Tr => "tr",
        Anchor::Bl => "bl",
        Anchor::Br => "br",
    }
}
