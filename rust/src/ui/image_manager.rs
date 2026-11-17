use crate::image_strip::Config;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
pub enum Action {
    Add,
    Wanted(u64, bool),
    Edit(u64),
}
struct Row {
    id: u64,
    wanted: gtk::Button,
    edit: gtk::Button,
}
pub struct Panel {
    pub container: gtk::Box,
    list: gtk::Box,
    add: gtk::Button,
    message: gtk::Label,
    rows: RefCell<Vec<Row>>,
    send: Rc<dyn Fn(Action)>,
}
impl Panel {
    pub fn new(send: impl Fn(Action) + 'static) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let note=gtk::Label::new(Some("Les bandes d’images apparaissent pendant l’interaction. Cliquez une vignette pour l’agrandir ; quitter la vignette ferme l’aperçu. Configurer permet d’importer et de réordonner des images locales."));
        note.set_line_wrap(true);
        note.set_xalign(0.);
        container.pack_start(&note, false, false, 0);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(150);
        scroll.set_max_content_height(400);
        scroll.set_propagate_natural_height(true);
        scroll.add(&list);
        container.pack_start(&scroll, true, true, 0);
        let add = gtk::Button::with_label("Ajouter une bande d’images");
        container.pack_start(&add, false, false, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        message.set_xalign(0.);
        container.pack_start(&message, false, false, 0);
        let send = Rc::new(send);
        let callback = send.clone();
        add.connect_clicked(move |_| callback(Action::Add));
        Self {
            container,
            list,
            add,
            message,
            rows: RefCell::new(Vec::new()),
            send,
        }
    }
    pub fn configure(&self, configs: &[Config]) {
        self.rows.borrow_mut().clear();
        for w in self.list.children() {
            self.list.remove(&w);
        }
        for c in configs {
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let text = gtk::Label::new(Some(&format!(
                "{} · {} image(s)",
                if c.title.is_empty() {
                    format!("Bande #{}", c.id)
                } else {
                    c.title.clone()
                },
                c.images.len()
            )));
            text.set_line_wrap(true);
            text.set_xalign(0.);
            line.pack_start(&text, true, true, 0);
            let wanted = gtk::Button::with_label(if c.wanted { "Masquer" } else { "Afficher" });
            let edit = gtk::Button::with_label("Configurer");
            line.pack_start(&wanted, false, false, 0);
            line.pack_start(&edit, false, false, 0);
            self.list.pack_start(&line, false, false, 0);
            let id = c.id;
            let next = !c.wanted;
            let callback = self.send.clone();
            wanted.connect_clicked(move |_| callback(Action::Wanted(id, next)));
            let callback = self.send.clone();
            edit.connect_clicked(move |_| callback(Action::Edit(id)));
            self.rows.borrow_mut().push(Row { id, wanted, edit });
        }
        self.add
            .set_sensitive(configs.len() < crate::image_strip::MAX_STRIPS);
        self.list.show_all();
    }
    pub fn busy(&self, busy: bool) {
        self.container.set_sensitive(!busy);
        if busy {
            self.message.set_text("Enregistrement des widgets…");
        }
    }
    pub fn message(&self, s: &str) {
        self.message.set_text(s);
    }
    pub fn diagnostics(&self, window: &gtk::Window, details: bool) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"visible":self.container.is_mapped(),"add":rect(self.add.upcast_ref()),"message":self.message.text().as_str(),"count":self.rows.borrow().len(),"rows":if details{self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.id,"wanted":rect(r.wanted.upcast_ref()),"edit":rect(r.edit.upcast_ref())})).collect::<Vec<_>>()}else{Vec::new()}})
    }
}
