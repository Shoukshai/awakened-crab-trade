use crate::item::{unidentified::Choice, Item};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub struct Panel {
    pub container: gtk::Box,
    pub close: gtk::Button,
    label: gtk::Label,
    rows: gtk::Box,
    scroll: gtk::ScrolledWindow,
    buttons: RefCell<Vec<(Choice, gtk::Button)>>,
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let label = gtk::Label::new(None);
        label.set_line_wrap(true);
        label.set_xalign(0.);
        container.pack_start(&label, false, false, 0);
        let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(120);
        scroll.set_max_content_height(300);
        scroll.set_propagate_natural_height(true);
        scroll.add(&rows);
        container.pack_start(&scroll, true, true, 0);
        let close = gtk::Button::with_label("Fermer");
        container.pack_start(&close, false, false, 0);
        Self {
            container,
            close,
            label,
            rows,
            scroll,
            buttons: RefCell::new(Vec::new()),
        }
    }
    pub fn show(&self, item: &Item, epoch: u64, choose: impl Fn(u64, String) + 'static) {
        for child in self.rows.children() {
            self.rows.remove(&child);
        }
        self.buttons.borrow_mut().clear();
        self.scroll.vadjustment().set_value(0.);
        let choices = item.unique_identity_choices();
        self.label.set_text(if choices.is_empty() {
            "Aucune identité unique pour cette base dans le catalogue. Aucune recherche n’est envoyée."
        } else {
            "Choisissez l’identité à consulter. L’objet reste non identifié ; aucun mod caché n’est déduit. Les variantes d’un même unique ne sont pas distinguées ici."
        });
        let choose = Rc::new(choose);
        for choice in choices {
            let button = gtk::Button::with_label(&choice.name);
            let callback = choose.clone();
            let reference = choice.reference.clone();
            button.connect_clicked(move |_| callback(epoch, reference.clone()));
            self.rows.pack_start(&button, false, false, 0);
            self.buttons.borrow_mut().push((choice, button));
        }
        self.container.show_all();
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        serde_json::json!({"visible":self.container.is_mapped(),"scroll":self.scroll.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":self.scroll.allocated_width(),"height":self.scroll.allocated_height(),"value":self.scroll.vadjustment().value(),"upper":self.scroll.vadjustment().upper(),"page_size":self.scroll.vadjustment().page_size()})),"message":self.label.text().as_str(),"choices":self.buttons.borrow().iter().map(|(choice,button)|serde_json::json!({"reference":choice.reference,"name":choice.name,"rect":button.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":button.allocated_width(),"height":button.allocated_height(),"mapped":button.is_mapped()}))})).collect::<Vec<_>>()})
    }
}
