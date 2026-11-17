use crate::inspection::{self, Edit, MapConfig, Report};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Panel {
    pub container: gtk::Box,
    pub price: gtk::Button,
    pub close: gtk::Button,
    pub wiki: gtk::Button,
    pub poedb: gtk::Button,
    pub coe: gtk::Button,
    pub stash: gtk::Button,
    profiles: Vec<gtk::Button>,
    show_new: gtk::CheckButton,
    summary: gtk::Label,
    rows: gtk::Box,
    scroll: gtk::ScrolledWindow,
    unknown: gtk::Label,
    details: gtk::Label,
    links: gtk::Box,
    error: gtk::Label,
    search: gtk::Entry,
    only_selected: gtk::CheckButton,
    catalog_expander: gtk::Expander,
    store: gtk::ListStore,
    tree: gtk::TreeView,
    catalog_count: gtk::Label,
    config: RefCell<MapConfig>,
    report: RefCell<Option<Report>>,
    updating: Cell<bool>,
    send: Rc<dyn Fn(Edit)>,
}
fn label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.);
    label.set_line_wrap(true);
    label
}
impl Panel {
    pub fn new(send: impl Fn(Edit) + 'static) -> Rc<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.pack_start(&label("Profil de cartes"), false, false, 0);
        let profiles = vec![
            gtk::Button::with_label("I"),
            gtk::Button::with_label("II"),
            gtk::Button::with_label("III"),
        ];
        for button in &profiles {
            header.pack_start(button, false, false, 0);
        }
        let show_new = gtk::CheckButton::with_label("Signaler les nouveaux mods");
        header.pack_start(&show_new, true, true, 0);
        let summary = label("");
        let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_min_content_height(100);
        scroll.set_max_content_height(380);
        scroll.set_propagate_natural_height(true);
        scroll.add(&rows);
        let details = label("");
        let unknown = label("");
        unknown.set_no_show_all(true);
        unknown.style_context().add_class("map-warning");
        let error = label("");
        error.style_context().add_class("map-warning");
        let links = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let wiki = gtk::Button::with_label("Wiki");
        let poedb = gtk::Button::with_label("PoEDB");
        let coe = gtk::Button::with_label("Craft of Exile");
        let stash = gtk::Button::with_label("Trouver dans le coffre");
        for b in [&wiki, &poedb, &coe, &stash] {
            links.pack_start(b, true, true, 0);
        }
        links.set_no_show_all(true);
        let catalog_expander = gtk::Expander::new(Some("Configurer les mods de zone"));
        let catalog_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let search = gtk::Entry::new();
        search.set_placeholder_text(Some("Rechercher par mots…"));
        let only_selected = gtk::CheckButton::with_label("Seulement les règles colorées du profil");
        let catalog_count = label("");
        let store = gtk::ListStore::new(&[
            String::static_type(),
            String::static_type(),
            bool::static_type(),
            bool::static_type(),
            bool::static_type(),
        ]);
        let tree = gtk::TreeView::with_model(&store);
        let matcher = gtk::CellRendererText::new();
        matcher.set_wrap_width(220);
        matcher.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        let column = gtk::TreeViewColumn::new();
        column.set_title("Modificateur");
        TreeViewColumnExt::pack_start(&column, &matcher, true);
        TreeViewColumnExt::add_attribute(&column, &matcher, "text", 0);
        column.set_expand(true);
        tree.append_column(&column);
        let tag = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        column.set_title("Type");
        TreeViewColumnExt::pack_start(&column, &tag, false);
        TreeViewColumnExt::add_attribute(&column, &tag, "text", 1);
        tree.append_column(&column);
        let mut radios = Vec::new();
        for (i, (title, value)) in [("Danger", 'd'), ("Prudence", 'w'), ("Bon", 'g')]
            .into_iter()
            .enumerate()
        {
            let cell = gtk::CellRendererToggle::new();
            cell.set_radio(true);
            cell.set_activatable(true);
            let column = gtk::TreeViewColumn::new();
            column.set_title(title);
            TreeViewColumnExt::pack_start(&column, &cell, false);
            TreeViewColumnExt::add_attribute(&column, &cell, "active", i as i32 + 2);
            tree.append_column(&column);
            radios.push((cell, value));
        }
        let catalog_scroll =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        catalog_scroll.set_min_content_height(230);
        catalog_scroll.set_max_content_height(350);
        catalog_scroll.set_propagate_natural_height(true);
        catalog_scroll.add(&tree);
        let clear = gtk::Button::with_label("Effacer la règle sélectionnée pour ce profil");
        for widget in [
            search.clone().upcast::<gtk::Widget>(),
            only_selected.clone().upcast(),
            catalog_count.clone().upcast(),
            catalog_scroll.upcast(),
            clear.clone().upcast(),
        ] {
            catalog_box.pack_start(&widget, false, false, 0);
        }
        catalog_expander.add(&catalog_box);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let price = gtk::Button::with_label("Consulter les prix");
        let close = gtk::Button::with_label("Fermer");
        actions.pack_start(&price, true, true, 0);
        actions.pack_start(&close, true, true, 0);
        for widget in [
            header.upcast::<gtk::Widget>(),
            summary.clone().upcast(),
            scroll.clone().upcast(),
            unknown.clone().upcast(),
            details.clone().upcast(),
            links.clone().upcast(),
            catalog_expander.clone().upcast(),
            error.clone().upcast(),
            actions.upcast(),
        ] {
            container.pack_start(&widget, false, false, 0);
        }
        let panel = Rc::new(Self {
            container,
            price,
            close,
            wiki,
            poedb,
            coe,
            stash,
            profiles,
            show_new,
            summary,
            rows,
            scroll,
            unknown,
            details,
            links,
            error,
            search,
            only_selected,
            catalog_expander,
            store,
            tree,
            catalog_count,
            config: RefCell::new(MapConfig::default()),
            report: RefCell::new(None),
            updating: Cell::new(false),
            send: Rc::new(send),
        });
        for (i, button) in panel.profiles.iter().enumerate() {
            let send = panel.send.clone();
            button.connect_clicked(move |_| send(Edit::Profile(i as u8 + 1)));
        }
        {
            let weak = Rc::downgrade(&panel);
            panel.show_new.connect_toggled(move |c| {
                if let Some(p) = weak.upgrade() {
                    if !p.updating.get() {
                        (p.send)(Edit::ShowNew(c.is_active()));
                    }
                }
            });
        }
        {
            let weak = Rc::downgrade(&panel);
            panel.search.connect_changed(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.refresh_catalog();
                }
            });
        }
        {
            let weak = Rc::downgrade(&panel);
            panel.only_selected.connect_toggled(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.refresh_catalog();
                }
            });
        }
        {
            let weak = Rc::downgrade(&panel);
            panel.catalog_expander.connect_expanded_notify(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.refresh_catalog();
                }
            });
        }
        for (cell, value) in radios {
            let weak = Rc::downgrade(&panel);
            cell.connect_toggled(move |_, path| {
                if let Some(p) = weak.upgrade() {
                    if let Some(iter) = p.store.iter(&path) {
                        if let Ok(matcher) = p.store.value(&iter, 0).get::<String>() {
                            (p.send)(Edit::Set(matcher, value));
                        }
                    }
                }
            });
        }
        {
            let weak = Rc::downgrade(&panel);
            clear.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    if let Some((model, iter)) = p.tree.selection().selected() {
                        if let Ok(matcher) = model.value(&iter, 0).get::<String>() {
                            (p.send)(Edit::Clear(matcher));
                        }
                    }
                }
            });
        }
        panel
    }
    pub fn show(&self, report: Report, config: &MapConfig) {
        self.updating.set(true);
        self.config.replace(config.clone());
        for (i, button) in self.profiles.iter().enumerate() {
            let style = button.style_context();
            style.remove_class("profile-active");
            if config.profile == i as u8 + 1 {
                style.add_class("profile-active");
            }
        }
        self.show_new.set_active(config.show_new_stats);
        for child in self.rows.children() {
            self.rows.remove(&child);
        }
        if report.map_like {
            self.summary.set_text(&format!("Zone : {}\nCliquez un mod : danger → prudence → favorable → vu. Les décisions dépendent de votre build.", report.title));
            self.scroll
                .set_min_content_height((report.stats.len() as i32 * 58 + 40).clamp(100, 380));
            if report.unidentified {
                self.rows.pack_start(
                    &label("Objet non identifié : les mods ne sont pas tous visibles."),
                    false,
                    false,
                    0,
                );
            }
            if report.maven_reminder {
                self.rows.pack_start(
                    &label("Pensez au témoin de la Maven pour cette zone."),
                    false,
                    false,
                    0,
                );
            }
            if report.stats.is_empty() {
                self.rows
                    .pack_start(&label("Aucun mod explicite reconnu."), false, false, 0);
            }
            for stat in &report.stats {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                let icon = match stat.decision {
                    'd' => "Danger",
                    'w' => "Prudence",
                    'g' => "Favorable",
                    's' => "Vu",
                    _ => "Non classé",
                };
                let button = gtk::Button::new();
                let text = label(&format!("{icon} · {}", stat.text));
                button.add(&text);
                button.set_hexpand(true);
                button.style_context().add_class(match stat.decision {
                    'd' => "map-danger",
                    'w' => "map-warning",
                    'g' => "map-good",
                    _ => "map-normal",
                });
                {
                    let send = self.send.clone();
                    let matcher = stat.matcher.clone();
                    button.connect_clicked(move |_| send(Edit::Cycle(matcher.clone())));
                }
                let seen = gtk::Button::with_label(if stat.is_new { "Nouveau" } else { "Vu" });
                seen.set_sensitive(config.show_new_stats);
                seen.set_tooltip_text(Some("Basculer entre nouveau et déjà vu pour ce profil"));
                {
                    let send = self.send.clone();
                    let matcher = stat.matcher.clone();
                    seen.connect_clicked(move |_| send(Edit::Seen(matcher.clone())));
                }
                row.pack_start(&button, true, true, 0);
                row.pack_start(&seen, false, false, 0);
                self.rows.pack_start(&row, false, false, 0);
            }
            if !report.unknown.is_empty() {
                self.unknown.set_text(&format!(
                    "{} ligne(s) non reconnue(s) — vérifiez l’objet en jeu :\n{}",
                    report.unknown.len(),
                    report.unknown.join("\n")
                ));
            }
            self.unknown.set_visible(!report.unknown.is_empty());
            self.details.set_text(if report.outdated.is_empty(){""}else{"Certaines règles colorées de ce profil ont une traduction obsolète. Consultez la configuration."});
            self.links.hide();
        } else {
            self.unknown.hide();
            self.scroll.set_min_content_height(100);
            self.summary
                .set_text("Informations de l’objet · calcul local, sans consultation des prix.");
            if let Some(dps) = &report.dps {
                self.rows.pack_start(&label(&format!("DPS de l’objet à sa qualité copiée\nPhysique : {} · élémentaire : {} · total : {}",dps.physical,dps.elemental,dps.total)),false,false,0);
            }
            if let Some(dust) = report.dust {
                self.rows.pack_start(
                    &label(&format!("Poussière de désenchantement : {dust}")),
                    false,
                    false,
                    0,
                );
            }
            if report.dps.is_none() && report.dust.is_none() {
                self.rows.pack_start(
                    &label("Aucune propriété calculée supplémentaire pour cet objet."),
                    false,
                    false,
                    0,
                );
            }
            self.details.set_text("");
            self.coe
                .set_sensitive(report.links.craft_of_exile.is_some());
            for child in self.links.children() {
                child.show_all();
            }
            self.links.show();
        }
        self.rows.show_all();
        self.report.replace(Some(report));
        self.error.set_text("");
        self.updating.set(false);
        self.refresh_catalog();
    }
    pub fn error(&self, error: &str) {
        self.error.set_text(error);
    }
    fn refresh_catalog(&self) {
        if !self.catalog_expander.is_expanded() {
            return;
        }
        let selected = self
            .tree
            .selection()
            .selected()
            .and_then(|(model, iter)| model.value(&iter, 0).get::<String>().ok());
        self.store.clear();
        let config = self.config.borrow();
        let entries = inspection::filtered_catalog(
            &config,
            &self.search.text(),
            self.only_selected.is_active(),
        );
        self.catalog_count.set_text(&format!(
            "{} traductions · choisir une couleur ou effacer la sélection",
            entries.len()
        ));
        for entry in entries {
            let decision = config.decision(&entry.matcher);
            let tag = match entry.tag.as_str() {
                "outdated" => "Obsolète",
                "heist_exclusive" => "Heist",
                "ubermap_exclusive" => "Uber",
                _ => "",
            };
            let iter = self.store.append();
            self.store.set(
                &iter,
                &[
                    (0, &entry.matcher),
                    (1, &tag),
                    (2, &(decision == 'd')),
                    (3, &(decision == 'w')),
                    (4, &(decision == 'g')),
                ],
            );
            if selected.as_deref() == Some(&entry.matcher) {
                self.tree.selection().select_iter(&iter);
            }
        }
    }
}
