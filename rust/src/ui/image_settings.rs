//! Detached image-strip editor with an embedded native file chooser. Import IO
//! is serialized off GTK; canceling a draft cannot publish an old completion.
use crate::image_strip::{Anchor, Config, MAX_STRIPS};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
};
struct ImageRow {
    id: u64,
    up: gtk::Button,
    down: gtk::Button,
    remove: gtk::Button,
}
pub struct Editor {
    pub container: gtk::Box,
    selector: gtk::ComboBoxText,
    add: gtk::Button,
    remove: gtk::Button,
    title: gtk::Entry,
    x: gtk::SpinButton,
    y: gtk::SpinButton,
    anchor: gtk::ComboBoxText,
    wanted: gtk::CheckButton,
    list: gtk::Box,
    scroll: gtk::ScrolledWindow,
    import: gtk::Button,
    message: gtk::Label,
    stack: gtk::Stack,
    chooser: gtk::FileChooserWidget,
    choose: gtk::Button,
    back: gtk::Button,
    configs: RefCell<Vec<Config>>,
    rows: RefCell<Vec<ImageRow>>,
    selected: Cell<Option<u64>>,
    filling: Cell<bool>,
    next_id: Cell<u64>,
    epoch: Cell<u64>,
    pending: Cell<bool>,
    worker: mpsc::SyncSender<(u64, u64, std::path::PathBuf)>,
}
impl Editor {
    #[allow(deprecated)] // Same pinned GLib actor channel as the other native panels.
    pub fn new() -> Rc<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let selector = gtk::ComboBoxText::new();
        let add = gtk::Button::with_label("Ajouter une bande");
        let remove = gtk::Button::with_label("Retirer la bande");
        top.pack_start(&selector, true, true, 0);
        top.pack_start(&add, false, false, 0);
        top.pack_start(&remove, false, false, 0);
        container.pack_start(&top, false, false, 0);
        let stack = gtk::Stack::new();
        stack.set_vhomogeneous(false);
        stack.set_hhomogeneous(false);
        let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let grid = gtk::Grid::new();
        grid.set_row_spacing(6);
        grid.set_column_spacing(6);
        let title = gtk::Entry::new();
        title.set_max_length(100);
        title.set_placeholder_text(Some("Sans titre"));
        let x = gtk::SpinButton::with_range(0., 100., 0.1);
        let y = gtk::SpinButton::with_range(0., 100., 0.1);
        super::settings_panel::short_number(&x);
        super::settings_panel::short_number(&y);
        let anchor = gtk::ComboBoxText::new();
        for a in Anchor::ALL {
            let s = format!("{a:?}").to_ascii_lowercase();
            let label = match a {
                Anchor::Tl => "Haut gauche",
                Anchor::Tc => "Haut centre",
                Anchor::Tr => "Haut droite",
                Anchor::Bl => "Bas gauche",
                Anchor::Bc => "Bas centre",
                Anchor::Br => "Bas droite",
            };
            anchor.append(Some(&s), label);
        }
        for (n, label, w) in [
            (0, "Titre", title.upcast_ref::<gtk::Widget>()),
            (1, "X %", x.upcast_ref()),
            (2, "Y %", y.upcast_ref()),
            (3, "Ancrage", anchor.upcast_ref()),
        ] {
            let l = gtk::Label::new(Some(label));
            l.set_xalign(0.);
            grid.attach(&l, 0, n, 1, 1);
            grid.attach(w, 1, n, 1, 1);
            w.set_hexpand(true);
        }
        form.pack_start(&grid, false, false, 0);
        let wanted = gtk::CheckButton::with_label("Afficher pendant l’interaction");
        form.pack_start(&wanted, false, false, 0);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(150);
        scroll.set_max_content_height(280);
        scroll.set_propagate_natural_height(true);
        scroll.add(&list);
        form.pack_start(&scroll, true, true, 0);
        let import = gtk::Button::with_label("Importer une image locale…");
        form.pack_start(&import, false, false, 0);
        stack.add_named(&form, "form");
        form.show_all();
        let file_page = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let chooser = gtk::FileChooserWidget::new(gtk::FileChooserAction::Open);
        chooser.set_size_request(500, 360);
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Images"));
        filter.add_pixbuf_formats();
        chooser.add_filter(filter);
        file_page.pack_start(&chooser, true, true, 0);
        let file_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let choose = gtk::Button::with_label("Importer ce fichier");
        let back = gtk::Button::with_label("Annuler le choix");
        file_actions.pack_start(&choose, true, true, 0);
        file_actions.pack_start(&back, true, true, 0);
        file_page.pack_start(&file_actions, false, false, 0);
        stack.add_named(&file_page, "file");
        file_page.show_all();
        stack.set_visible_child_name("form");
        container.pack_start(&stack, true, true, 0);
        let message=gtk::Label::new(Some("Les fichiers importés sont copiés localement. Les bandes et leur ordre restent au brouillon jusqu’à Enregistrer."));
        message.set_line_wrap(true);
        message.set_xalign(0.);
        container.pack_start(&message, false, false, 0);
        let (worker, jobs) = mpsc::sync_channel::<(u64, u64, std::path::PathBuf)>(1);
        let (sender, receiver) = glib::MainContext::channel::<(u64, u64, Result<String, String>)>(
            glib::Priority::DEFAULT,
        );
        std::thread::spawn(move || {
            while let Ok((epoch, id, path)) = jobs.recv() {
                if sender
                    .send((epoch, id, crate::desktop::images::import(&path)))
                    .is_err()
                {
                    break;
                }
            }
        });
        let e = Rc::new(Self {
            container,
            selector,
            add,
            remove,
            title,
            x,
            y,
            anchor,
            wanted,
            list,
            scroll,
            import,
            message,
            stack,
            chooser,
            choose,
            back,
            configs: RefCell::new(Vec::new()),
            rows: RefCell::new(Vec::new()),
            selected: Cell::new(None),
            filling: Cell::new(false),
            next_id: Cell::new(1),
            epoch: Cell::new(1),
            pending: Cell::new(false),
            worker,
        });
        let weak = Rc::downgrade(&e);
        receiver.attach(None,move |(epoch,id,result)|{if let Some(e)=weak.upgrade(){if e.epoch.get()==epoch {e.pending.set(false);e.choose.set_sensitive(true);match result{Ok(url)=>{let result=e.configs.borrow_mut().iter_mut().find(|c|c.id==id).map(|c|c.append(url));match result{Some(Ok(_))=>{e.message.set_text("Image importée localement. Enregistrez le brouillon pour l’utiliser.");e.set_page("form");e.show_selected();},Some(Err(err))=>e.message.set_text(&err),None=>()}}Err(err)=>e.message.set_text(&err)}}}glib::ControlFlow::Continue});
        let weak = Rc::downgrade(&e);
        e.selector.connect_changed(move |s| {
            if let Some(e) = weak.upgrade() {
                if !e.filling.get() {
                    e.selected.set(s.active_id().and_then(|s| s.parse().ok()));
                    e.show_selected();
                }
            }
        });
        let weak = Rc::downgrade(&e);
        e.add.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                let id = e.next_id.get();
                if e.configs.borrow().len() < MAX_STRIPS && id <= crate::image_strip::MAX_ID {
                    e.next_id.set(id + 1);
                    e.configs.borrow_mut().push(Config::new(
                        id,
                        glib::random_double(),
                        glib::random_double(),
                    ));
                    e.selected.set(Some(id));
                    e.refresh_selector();
                    e.show_selected();
                }
            }
        });
        let weak = Rc::downgrade(&e);
        e.remove.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                let id = e.selected.get();
                e.configs.borrow_mut().retain(|c| Some(c.id) != id);
                e.selected.set(e.configs.borrow().first().map(|c| c.id));
                e.refresh_selector();
                e.show_selected();
            }
        });
        let weak = Rc::downgrade(&e);
        e.title.connect_changed(move |_| {
            if let Some(e) = weak.upgrade() {
                e.update_fields();
            }
        });
        for spin in [&e.x, &e.y] {
            let weak = Rc::downgrade(&e);
            spin.connect_value_changed(move |_| {
                if let Some(e) = weak.upgrade() {
                    e.update_fields();
                }
            });
        }
        let weak = Rc::downgrade(&e);
        e.anchor.connect_changed(move |_| {
            if let Some(e) = weak.upgrade() {
                e.update_fields();
            }
        });
        let weak = Rc::downgrade(&e);
        e.wanted.connect_toggled(move |_| {
            if let Some(e) = weak.upgrade() {
                e.update_fields();
            }
        });
        let weak = Rc::downgrade(&e);
        e.import.connect_clicked(move |_| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(e) = weak.upgrade() {
                    if !e.container.is_mapped() {
                        return;
                    }
                    e.set_page("file");
                    let chooser = e.chooser.clone();
                    glib::idle_add_local_once(move || {
                        if chooser.is_mapped() {
                            chooser.child_focus(gtk::DirectionType::TabForward);
                            // Create the location entry outside key-event propagation.
                            // https://docs.gtk.org/gtk3/signal.FileChooserWidget.location-popup.html
                            chooser.emit_by_name::<()>("location-popup", &[&""]);
                        }
                    });
                }
            });
        });
        let weak = Rc::downgrade(&e);
        e.back.connect_clicked(move |_| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(e) = weak.upgrade() {
                    e.cancel_import();
                }
            });
        });
        let weak = Rc::downgrade(&e);
        e.choose.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                if e.pending.get() {
                    return;
                }
                let Some(id) = e.selected.get() else {
                    return;
                };
                let Some(path) = e.chooser.filename() else {
                    e.message.set_text("Choisissez une image locale.");
                    return;
                };
                e.pending.set(true);
                e.choose.set_sensitive(false);
                if let Err(err) = e.worker.try_send((e.epoch.get(), id, path)) {
                    e.pending.set(false);
                    e.choose.set_sensitive(true);
                    e.message.set_text(&err.to_string());
                }
            }
        });
        e
    }
    pub fn fill(self: &Rc<Self>, configs: &[Config]) {
        self.cancel_import();
        self.configs.replace(configs.to_vec());
        self.next_id
            .set(configs.iter().map(|c| c.id).max().unwrap_or(0) + 1);
        self.selected.set(configs.first().map(|c| c.id));
        self.refresh_selector();
        self.show_selected();
    }
    fn refresh_selector(&self) {
        self.filling.set(true);
        self.selector.remove_all();
        for c in self.configs.borrow().iter() {
            self.selector.append(
                Some(&c.id.to_string()),
                &if c.title.is_empty() {
                    format!("Bande #{}", c.id)
                } else {
                    c.title.clone()
                },
            );
        }
        self.selector
            .set_active_id(self.selected.get().map(|id| id.to_string()).as_deref());
        self.add
            .set_sensitive(self.configs.borrow().len() < MAX_STRIPS);
        self.filling.set(false);
    }
    fn update_fields(&self) {
        if self.filling.get() {
            return;
        }
        if let Some(c) = self
            .configs
            .borrow_mut()
            .iter_mut()
            .find(|c| Some(c.id) == self.selected.get())
        {
            c.title = self.title.text().into();
            c.x = self.x.value();
            c.y = self.y.value();
            c.wanted = self.wanted.is_active();
            if let Some(a) = self
                .anchor
                .active_id()
                .and_then(|s| serde_json::from_value(serde_json::Value::String(s.into())).ok())
            {
                c.anchor = a;
            }
        }
    }
    pub fn select(self: &Rc<Self>, id: u64) {
        if self.configs.borrow().iter().any(|c| c.id == id) {
            self.selected.set(Some(id));
            self.refresh_selector();
            self.show_selected();
        }
    }
    fn show_selected(self: &Rc<Self>) {
        let c = self
            .configs
            .borrow()
            .iter()
            .find(|c| Some(c.id) == self.selected.get())
            .cloned();
        self.filling.set(true);
        for w in [
            &self.title.clone().upcast::<gtk::Widget>(),
            &self.x.clone().upcast(),
            &self.y.clone().upcast(),
            &self.anchor.clone().upcast(),
            &self.wanted.clone().upcast(),
            &self.import.clone().upcast(),
            &self.remove.clone().upcast(),
        ] {
            w.set_sensitive(c.is_some());
        }
        self.rows.borrow_mut().clear();
        for w in self.list.children() {
            self.list.remove(&w);
        }
        if let Some(c) = c {
            self.title.set_text(&c.title);
            self.x.set_value(c.x);
            self.y.set_value(c.y);
            self.wanted.set_active(c.wanted);
            self.anchor
                .set_active_id(Some(&format!("{:?}", c.anchor).to_ascii_lowercase()));
            for image in &c.images {
                let line = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                let name = gtk::Label::new(Some(&if image.url == "syndicate.jpg" {
                    "Syndicate (fourni)".into()
                } else {
                    format!("Image #{}", image.id)
                }));
                name.set_xalign(0.);
                line.pack_start(&name, true, true, 0);
                let up = gtk::Button::with_label("←");
                let down = gtk::Button::with_label("→");
                let remove = gtk::Button::with_label("Retirer");
                for b in [&up, &down, &remove] {
                    line.pack_start(b, false, false, 0);
                }
                self.list.pack_start(&line, false, false, 0);
                for (b, delta) in [(&up, -1), (&down, 1), (&remove, 0)] {
                    let weak = Rc::downgrade(self);
                    let id = image.id;
                    b.connect_clicked(move |_| {
                        if let Some(e) = weak.upgrade() {
                            if let Some(c) = e
                                .configs
                                .borrow_mut()
                                .iter_mut()
                                .find(|c| Some(c.id) == e.selected.get())
                            {
                                if delta == 0 {
                                    c.remove(id);
                                } else if let Some(i) = c.images.iter().position(|x| x.id == id) {
                                    let j = i as i32 + delta;
                                    if j >= 0 && (j as usize) < c.images.len() {
                                        let mut order: Vec<_> =
                                            c.images.iter().map(|i| i.id).collect();
                                        order.swap(i, j as usize);
                                        let _ = c.reorder(&order);
                                    }
                                }
                            }
                            e.show_selected();
                        }
                    });
                }
                self.rows.borrow_mut().push(ImageRow {
                    id: image.id,
                    up,
                    down,
                    remove,
                });
            }
            self.import
                .set_sensitive(c.images.len() < crate::image_strip::MAX_IMAGES);
        } else {
            self.title.set_text("");
        }
        self.list.show_all();
        self.filling.set(false);
    }
    pub fn cancel_import(&self) {
        self.epoch.set(self.epoch.get() + 1);
        self.pending.set(false);
        self.choose.set_sensitive(true);
        self.set_page("form");
    }
    fn set_page(&self, page: &str) {
        // GTK must not retain a focused descendant of the page being unmapped.
        // This changes focus within an already owned window; it never activates it.
        let window = self
            .container
            .toplevel()
            .and_then(|w| w.downcast::<gtk::Window>().ok());
        if let Some(w) = &window {
            if w.focused_widget()
                .is_some_and(|f| f.is_ancestor(&self.stack))
            {
                w.set_focus(None::<&gtk::Widget>);
            }
        }
        self.stack.set_visible_child_name(page);
        if window.is_some_and(|w| w.is_active()) && page == "form" && self.title.is_mapped() {
            self.title.grab_focus();
        }
    }
    pub fn read(&self) -> Result<Vec<Config>, String> {
        if self.pending.get() {
            return Err("Attendez la fin de l’import de l’image.".into());
        }
        self.x.update();
        self.y.update();
        self.update_fields();
        let cs = self.configs.borrow().clone();
        for c in &cs {
            c.validate()?;
        }
        Ok(cs)
    }
    pub fn diagnostics(&self, window: &gtk::Window, details: bool) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"count":self.configs.borrow().len(),"selected":self.selected.get(),"pending":self.pending.get(),"page":self.stack.visible_child_name().as_deref(),"add":rect(self.add.upcast_ref()),"remove_strip":rect(self.remove.upcast_ref()),"selector":rect(self.selector.upcast_ref()),"title":{"value":self.title.text().as_str(),"rect":rect(self.title.upcast_ref())},"x":{"value":self.x.value(),"rect":rect(self.x.upcast_ref())},"y":{"value":self.y.value(),"rect":rect(self.y.upcast_ref())},"anchor":{"value":self.anchor.active_id().as_deref(),"rect":rect(self.anchor.upcast_ref())},"wanted":{"value":self.wanted.is_active(),"rect":rect(self.wanted.upcast_ref())},"import":rect(self.import.upcast_ref()),"choose":rect(self.choose.upcast_ref()),"back":rect(self.back.upcast_ref()),"filename":if details&&self.stack.visible_child_name().as_deref()==Some("file")&&self.chooser.is_mapped(){self.chooser.filename()}else{None},"chooser":rect(self.chooser.upcast_ref()),"scroll":rect(self.scroll.upcast_ref()),"message":self.message.text().as_str(),"rows":if details{self.rows.borrow().iter().map(|r|serde_json::json!({"id":r.id,"up":rect(r.up.upcast_ref()),"down":rect(r.down.upcast_ref()),"remove":rect(r.remove.upcast_ref())})).collect::<Vec<_>>()}else{vec![]}})
    }
}
