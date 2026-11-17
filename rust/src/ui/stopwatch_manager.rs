//! A native widget menu for creating/showing/configuring stopwatch instances.
use crate::stopwatch::{Config, Reading};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
pub enum Action {
    Add,
    Toggle(u64),
    Reset(u64),
    Wanted(u64, bool),
    Edit,
}
struct Row {
    id: u64,
    label: gtk::Label,
    toggle: gtk::Button,
    reset: gtk::Button,
    wanted: gtk::Button,
}
pub struct Manager {
    pub container: gtk::Box,
    list: gtk::Box,
    add: gtk::Button,
    edit: gtk::Button,
    message: gtk::Label,
    rows: RefCell<Vec<Row>>,
    send: Rc<dyn Fn(Action)>,
}
impl Manager {
    pub fn new(send: impl Fn(Action) + 'static) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let note=gtk::Label::new(Some("Chaque chronomètre garde son temps indépendamment. En dehors de l’interaction, les widgets affichés laissent le clavier et les clics au jeu. Les noms, positions et raccourcis se configurent dans Réglages → Chronomètres."));
        note.set_line_wrap(true);
        note.set_xalign(0.);
        container.pack_start(&note, false, false, 0);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(200);
        scroll.set_max_content_height(480);
        scroll.set_propagate_natural_height(true);
        scroll.add(&list);
        container.pack_start(&scroll, true, true, 0);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let add = gtk::Button::with_label("Ajouter un chronomètre");
        let edit = gtk::Button::with_label("Configurer les chronomètres");
        actions.pack_start(&add, true, true, 0);
        actions.pack_start(&edit, true, true, 0);
        container.pack_start(&actions, false, false, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        message.set_xalign(0.);
        container.pack_start(&message, false, false, 0);
        let send = Rc::new(send);
        let callback = send.clone();
        add.connect_clicked(move |_| callback(Action::Add));
        let callback = send.clone();
        edit.connect_clicked(move |_| callback(Action::Edit));
        Self {
            container,
            list,
            add,
            edit,
            message,
            rows: RefCell::new(Vec::new()),
            send,
        }
    }
    pub fn configure(&self, configs: &[Config]) {
        self.rows.borrow_mut().clear();
        for child in self.list.children() {
            self.list.remove(&child);
        }
        for c in configs {
            let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
            let name = gtk::Label::new(Some(&if c.title.is_empty() {
                format!("Chronomètre #{}", c.id)
            } else {
                c.title.clone()
            }));
            name.set_xalign(0.);
            card.pack_start(&name, false, false, 0);
            let label = gtk::Label::new(Some("00:00:00 · En pause"));
            label.set_xalign(0.);
            card.pack_start(&label, false, false, 0);
            let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let toggle = gtk::Button::with_label("Démarrer");
            let reset = gtk::Button::with_label("Remettre à zéro");
            let wanted = gtk::Button::with_label(if c.wanted { "Masquer" } else { "Afficher" });
            for b in [&toggle, &reset, &wanted] {
                buttons.pack_start(b, true, true, 0);
            }
            card.pack_start(&buttons, false, false, 0);
            self.list.pack_start(&card, false, false, 0);
            let id = c.id;
            let cb = self.send.clone();
            toggle.connect_clicked(move |_| cb(Action::Toggle(id)));
            let cb = self.send.clone();
            reset.connect_clicked(move |_| cb(Action::Reset(id)));
            let cb = self.send.clone();
            let next = !c.wanted;
            wanted.connect_clicked(move |_| cb(Action::Wanted(id, next)));
            self.rows.borrow_mut().push(Row {
                id,
                label,
                toggle,
                reset,
                wanted,
            });
        }
        self.add
            .set_sensitive(configs.len() < crate::stopwatch::MAX_INSTANCES);
        self.list.show_all();
    }
    pub fn update(&self, values: &[Reading]) {
        for row in self.rows.borrow().iter() {
            if let Some(value) = values.iter().find(|v| v.id == row.id) {
                let text = format!(
                    "{} · {}",
                    value.formatted,
                    if value.running {
                        "En marche"
                    } else {
                        "En pause"
                    }
                );
                if row.label.text() != text {
                    row.label.set_text(&text);
                }
                row.toggle
                    .set_label(if value.running { "Pause" } else { "Démarrer" });
            }
        }
    }
    pub fn busy(&self, busy: bool) {
        self.container.set_sensitive(!busy);
        if busy {
            self.message.set_text("Enregistrement des chronomètres…");
        }
    }
    pub fn message(&self, text: &str) {
        self.message.set_text(text);
    }
    pub fn diagnostics(&self, window: &gtk::Window, details: bool) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"visible":self.container.is_mapped(),"add":rect(self.add.upcast_ref()),"edit":rect(self.edit.upcast_ref()),"message":self.message.text().as_str(),"count":self.rows.borrow().len(),"rows":if details{self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.id,"text":r.label.text().as_str(),"toggle":rect(r.toggle.upcast_ref()),"reset":rect(r.reset.upcast_ref()),"wanted":rect(r.wanted.upcast_ref())})).collect::<Vec<_>>()}else{Vec::new()}})
    }
}
