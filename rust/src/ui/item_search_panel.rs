//! Native Heist-target search. A selected identity is a virtual consultation;
//! typing and clearing are local. Prices are requested by the owner explicitly.
use crate::{
    economy::Report,
    item::search::{self, Basket, Choice, Kind},
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
#[derive(Clone)]
pub enum Action {
    Select {
        session: u64,
        revision: u64,
        id: usize,
    },
    Consult {
        session: u64,
        id: usize,
    },
    Reset(u64),
    Refresh(u64),
    Close(u64),
}
impl Action {
    pub fn session(&self) -> u64 {
        match self {
            Self::Select { session, .. }
            | Self::Consult { session, .. }
            | Self::Reset(session)
            | Self::Refresh(session)
            | Self::Close(session) => *session,
        }
    }
}
pub struct Panel {
    pub container: gtk::Box,
    entry: gtk::Entry,
    kind: gtk::ComboBoxText,
    results: gtk::Box,
    selected: gtk::Box,
    message: gtk::Label,
    search_message: gtk::Label,
    reset: gtk::Button,
    refresh: gtk::Button,
    close: gtk::Button,
    session: Cell<u64>,
    revision: Cell<u64>,
    basket: RefCell<Basket>,
    quotes: RefCell<HashMap<usize, Report>>,
    choices: RefCell<Vec<(Choice, gtk::Button)>>,
    selected_buttons: RefCell<Vec<(usize, gtk::Button)>>,
    send: Box<dyn Fn(Action)>,
    league: RefCell<String>,
}
impl Panel {
    pub fn new(send: impl Fn(Action) + 'static) -> Rc<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let explanation=gtk::Label::new(Some("Cibles de Heist : répliques d’uniques et anciennes gemmes alternatives. Sélectionnez jusqu’à cinq objets pour comparer leurs cotes ; cliquez sur une sélection pour consulter Trade. Ces objets sont virtuels et ne proviennent pas de votre inventaire."));
        explanation.set_line_wrap(true);
        explanation.set_xalign(0.);
        container.pack_start(&explanation, false, false, 0);
        let selected = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let selected_scroll =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        selected_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        selected_scroll.set_min_content_height(70);
        selected_scroll.set_max_content_height(230);
        selected_scroll.set_propagate_natural_height(true);
        selected_scroll.add(&selected);
        container.pack_start(&selected_scroll, false, false, 0);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some(
            "Au moins 3 caractères ; tous les mots doivent correspondre",
        ));
        entry.set_max_length(256);
        row.pack_start(&entry, true, true, 0);
        let kind = gtk::ComboBoxText::new();
        kind.append(Some("gem"), "Anciennes gemmes");
        kind.append(Some("replica"), "Répliques");
        kind.set_active_id(Some("gem"));
        row.pack_start(&kind, false, false, 0);
        container.pack_start(&row, false, false, 0);
        let search_message = gtk::Label::new(None);
        search_message.set_line_wrap(true);
        search_message.set_xalign(0.);
        container.pack_start(&search_message, false, false, 0);
        let results = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(100);
        scroll.set_max_content_height(260);
        scroll.set_propagate_natural_height(true);
        scroll.add(&results);
        container.pack_start(&scroll, true, true, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        message.set_xalign(0.);
        container.pack_start(&message, false, false, 0);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let reset = gtk::Button::with_label("Effacer la sélection");
        let refresh = gtk::Button::with_label("Actualiser les cotes");
        let close = gtk::Button::with_label("Fermer");
        for b in [&reset, &refresh, &close] {
            actions.pack_start(b, true, true, 0);
        }
        container.pack_start(&actions, false, false, 0);
        let panel = Rc::new(Self {
            container,
            entry,
            kind,
            results,
            selected,
            message,
            search_message,
            reset,
            refresh,
            close,
            session: Cell::new(0),
            revision: Cell::new(0),
            basket: RefCell::new(Basket::default()),
            quotes: RefCell::new(HashMap::new()),
            choices: RefCell::new(Vec::new()),
            selected_buttons: RefCell::new(Vec::new()),
            send: Box::new(send),
            league: RefCell::new(String::new()),
        });
        let weak = Rc::downgrade(&panel);
        panel.entry.connect_changed(move |_| {
            if let Some(p) = weak.upgrade() {
                p.render_results();
            }
        });
        let weak = Rc::downgrade(&panel);
        panel.kind.connect_changed(move |_| {
            if let Some(p) = weak.upgrade() {
                p.render_results();
            }
        });
        for (button, action) in [
            (&panel.reset, Action::Reset as fn(u64) -> Action),
            (&panel.refresh, Action::Refresh),
            (&panel.close, Action::Close),
        ] {
            let weak = Rc::downgrade(&panel);
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    (p.send)(action(p.session.get()));
                }
            });
        }
        panel.render_results();
        panel.render_selected();
        panel
    }
    pub fn session(&self) -> u64 {
        self.session.get()
    }
    pub fn show(self: &Rc<Self>, league: &str, session: u64) {
        self.session.set(session);
        self.league.replace(league.into());
        self.clear_quotes();
        self.render_results();
        self.container.show_all();
    }
    pub fn set_search(&self, kind: Kind, query: &str) {
        self.kind.set_active_id(Some(if kind == Kind::Replica {
            "replica"
        } else {
            "gem"
        }));
        self.entry.set_text(query);
    }
    pub fn clear_quotes(self: &Rc<Self>) {
        self.quotes.borrow_mut().clear();
        for i in &mut self.basket.borrow_mut().items {
            i.chaos = None;
        }
        self.message.set_text("Cotes non chargées. Sélectionner un objet ou Actualiser consulte poe.ninja, sans recherche Trade.");
        self.render_selected();
    }
    fn current_kind(&self) -> Kind {
        if self.kind.active_id().as_deref() == Some("replica") {
            Kind::Replica
        } else {
            Kind::Gem
        }
    }
    fn render_results(self: &Rc<Self>) {
        self.revision.set(self.revision.get() + 1);
        self.choices.borrow_mut().clear();
        for c in self.results.children() {
            self.results.remove(&c);
        }
        let kind = self.current_kind();
        let result = search::find(&self.entry.text(), kind);
        self.search_message.set_text(if kind==Kind::Gem && search::candidate_count(kind)==0 {"Le catalogue actuel ne contient aucune ancienne gemme à qualité alternative. Les gemmes transfigurées ne font pas partie de cette recherche upstream."}else{match &result {None=>"Précisez la recherche : au moins trois caractères et au plus cinq résultats.",Some(v) if v.is_empty()=>"Aucun objet trouvé.",_=>"Sélectionnez une cible ; une sixième sélection remplace les cinq précédentes."}});
        for choice in result.unwrap_or_default() {
            let label = format!(
                "Sélectionner {}{}",
                choice.name,
                choice
                    .base
                    .as_ref()
                    .map(|b| format!(" · {b}"))
                    .unwrap_or_default()
            );
            let button = gtk::Button::with_label(&label);
            let weak = Rc::downgrade(self);
            let session = self.session.get();
            let revision = self.revision.get();
            let id = choice.id;
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    (p.send)(Action::Select {
                        session,
                        revision,
                        id,
                    });
                }
            });
            self.results.pack_start(&button, false, false, 0);
            self.choices.borrow_mut().push((choice, button));
        }
        self.results.show_all();
    }
    pub fn select(self: &Rc<Self>, revision: u64, id: usize) -> bool {
        if revision != self.revision.get() {
            return false;
        }
        let choice = self
            .choices
            .borrow()
            .iter()
            .find(|(c, _)| c.id == id)
            .map(|(c, _)| c.clone());
        let Some(choice) = choice else {
            return false;
        };
        let chaos = self
            .quotes
            .borrow()
            .get(&id)
            .and_then(|r| r.quote.as_ref())
            .map(|q| q.chaos);
        let added = self.basket.borrow_mut().add(choice, chaos);
        self.entry.set_text("");
        self.render_selected();
        added
    }
    pub fn choices(&self) -> Vec<Choice> {
        self.basket
            .borrow()
            .items
            .iter()
            .map(|i| i.choice.clone())
            .collect()
    }
    pub fn selected_choice(&self, id: usize) -> Option<Choice> {
        self.basket
            .borrow()
            .items
            .iter()
            .find(|i| i.choice.id == id)
            .map(|i| i.choice.clone())
    }
    pub fn reset(self: &Rc<Self>) {
        self.basket.replace(Basket::default());
        self.entry.set_text("");
        self.clear_quotes();
    }
    pub fn loading(&self) {
        self.message
            .set_text("Chargement des cotes indicatives poe.ninja…");
    }
    pub fn show_quotes(self: &Rc<Self>, quotes: Result<Vec<(usize, Report)>, String>) {
        match quotes {
            Err(error) => {
                self.clear_quotes();
                self.message.set_text(&format!(
                    "Cotes indisponibles : {error}. Actualiser permet de réessayer."
                ));
            }
            Ok(quotes) => {
                self.quotes.replace(quotes.into_iter().collect());
                for i in &mut self.basket.borrow_mut().items {
                    i.chaos = self
                        .quotes
                        .borrow()
                        .get(&i.choice.id)
                        .and_then(|r| r.quote.as_ref())
                        .map(|q| q.chaos);
                }
                self.basket.borrow_mut().sort();
                self.message.set_text(&format!("Cotes indicatives · {} · identité du catalogue, sans mods ni sockets. Un prix absent reste inconnu.",self.league.borrow()));
            }
        }
        self.render_selected();
    }
    fn render_selected(self: &Rc<Self>) {
        self.selected_buttons.borrow_mut().clear();
        for c in self.selected.children() {
            self.selected.remove(&c);
        }
        for i in &self.basket.borrow().items {
            let quotes = self.quotes.borrow();
            let r = quotes.get(&i.choice.id);
            let price = r
                .and_then(|r| r.price.as_ref())
                .map(|p| {
                    format!(
                        "{} {}",
                        super::economy_panel::format_amount(p.min),
                        p.currency
                    )
                })
                .unwrap_or_else(|| "Cote inconnue".into());
            let button = gtk::Button::with_label(&format!(
                "{price} · {} · Consulter les prix",
                i.choice.name
            ));
            let session = self.session.get();
            let id = i.choice.id;
            let weak = Rc::downgrade(self);
            button.set_tooltip_text(Some(
                "Objet virtuel du catalogue : aucune propriété de votre inventaire n’est supposée.",
            ));
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    (p.send)(Action::Consult { session, id });
                }
            });
            self.selected.pack_start(&button, false, false, 0);
            self.selected_buttons.borrow_mut().push((id, button));
        }
        self.refresh
            .set_sensitive(!self.basket.borrow().items.is_empty());
        self.reset
            .set_sensitive(!self.basket.borrow().items.is_empty());
        self.selected.show_all();
    }
    fn quote_summary(&self, id: usize) -> Option<serde_json::Value> {
        self.quotes.borrow().get(&id).map(|r|serde_json::json!({"identity":r.identity,"price":r.price,"quote":r.quote.as_ref().map(|q|serde_json::json!({"chaos":q.chaos,"age_seconds":q.age_seconds,"from_cache":q.from_cache,"category":q.category,"variant":q.variant})),"age_seconds":r.age_seconds,"from_cache":r.from_cache}))
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"visible":self.container.is_mapped(),"session":self.session.get(),"revision":self.revision.get(),"league":self.league.borrow().as_str(),"kind":self.kind.active_id().as_deref(),"kind_control":rect(self.kind.upcast_ref()),"entry":{"value":self.entry.text().as_str(),"rect":rect(self.entry.upcast_ref())},"message":self.message.text().as_str(),"search_message":self.search_message.text().as_str(),"results":self.choices.borrow().iter().map(|(c,b)|serde_json::json!({"choice":c,"rect":rect(b.upcast_ref())})).collect::<Vec<_>>(),"selected":self.basket.borrow().items.iter().map(|i|serde_json::json!({"choice":i.choice,"chaos":i.chaos,"quote":self.quote_summary(i.choice.id),"rect":self.selected_buttons.borrow().iter().find(|(id,_)|*id==i.choice.id).and_then(|(_,b)|rect(b.upcast_ref()))})).collect::<Vec<_>>(),"reset":rect(self.reset.upcast_ref()),"refresh":rect(self.refresh.upcast_ref()),"close":rect(self.close.upcast_ref())})
    }
}
