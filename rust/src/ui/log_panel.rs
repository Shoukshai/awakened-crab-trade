//! Refresh and copy the complete current process log without blocking GTK.
use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};
pub struct Panel {
    pub container: gtk::Box,
    refresh: gtk::Button,
    copy: gtk::Button,
    text: gtk::TextView,
    status: gtk::Label,
    busy: Rc<Cell<bool>>,
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        container.set_border_width(12);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let refresh = gtk::Button::with_label("Actualiser le journal");
        let copy = gtk::Button::with_label("Copier tout le journal");
        actions.pack_start(&refresh, false, false, 0);
        actions.pack_start(&copy, true, true, 0);
        container.pack_start(&actions, false, false, 0);
        let text = gtk::TextView::new();
        text.set_editable(false);
        text.set_monospace(true);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        scroll.add(&text);
        container.pack_start(&scroll, true, true, 0);
        let status = gtk::Label::new(None);
        status.set_xalign(0.);
        status.set_line_wrap(true);
        container.pack_start(&status, false, false, 0);
        let panel = Self {
            container,
            refresh,
            copy,
            text,
            status,
            busy: Rc::new(Cell::new(false)),
        };
        let controls = panel.controls();
        panel
            .refresh
            .connect_clicked(move |_| Self::load(controls.clone(), false));
        let controls = panel.controls();
        panel
            .copy
            .connect_clicked(move |_| Self::load(controls.clone(), true));
        panel
    }
    fn controls(&self) -> Controls {
        (
            self.refresh.clone(),
            self.copy.clone(),
            self.text.clone(),
            self.status.clone(),
            self.busy.clone(),
        )
    }
    pub fn refresh(&self) {
        Self::load(self.controls(), false);
    }
    #[allow(deprecated)] // glib 0.18 channel wakes GTK after a bounded local file read.
    fn load((refresh, copy, text, status, busy): Controls, copy_after: bool) {
        if busy.replace(true) {
            return;
        }
        refresh.set_sensitive(false);
        copy.set_sensitive(false);
        status.set_text(&crate::i18n::tr("Lecture du journal…"));
        let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
        std::thread::spawn(move || {
            let _ = sender.send(crate::diagnostics::snapshot());
        });
        receiver.attach(None, move |report| {
            text.buffer().unwrap().set_text(&report);
            if copy_after {
                gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(&report);
                status.set_text(&crate::i18n::tr("Journal complet copié."));
            } else {
                status.set_text(&crate::i18n::tr("Journal de la session actuelle."));
            }
            refresh.set_sensitive(true);
            copy.set_sensitive(true);
            busy.set(false);
            glib::ControlFlow::Break
        });
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)| serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        let b = self.text.buffer().unwrap();
        let text = b
            .text(&b.start_iter(), &b.end_iter(), false)
            .unwrap_or_default();
        serde_json::json!({"mapped":self.container.is_mapped(),"busy":self.busy.get(),"bytes":text.len(),"prefix":text.chars().take(1024).collect::<String>(),"copy":rect(self.copy.upcast_ref()),"refresh":rect(self.refresh.upcast_ref()),"status":self.status.text().as_str()})
    }
}
type Controls = (
    gtk::Button,
    gtk::Button,
    gtk::TextView,
    gtk::Label,
    Rc<Cell<bool>>,
);

#[cfg(feature = "cloud-ocr")]
#[allow(deprecated)]
pub fn copy_session(button: &gtk::Button) {
    let button = button.clone();
    button.set_sensitive(false);
    let (sender, receiver) = glib::MainContext::channel::<String>(glib::Priority::DEFAULT);
    std::thread::spawn(move || {
        let _ = sender.send(crate::diagnostics::snapshot());
    });
    receiver.attach(None, move |report| {
        gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(&report);
        button.set_sensitive(true);
        glib::ControlFlow::Break
    });
}
