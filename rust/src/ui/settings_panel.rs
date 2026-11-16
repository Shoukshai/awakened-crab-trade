//! Settings are edited in a detached draft. Only the explicit Apply action
//! submits them to the desktop transaction worker.
use crate::{
    hotkeys::{Hotkey, Hotkeys},
    settings::Settings,
};
use gtk::{gdk, prelude::*};
use std::cell::{Cell, RefCell};
pub struct Panel {
    pub container: gtk::Box,
    pub apply: gtk::Button,
    pub cancel: gtk::Button,
    pub reset: gtk::Button,
    form: gtk::Notebook,
    general_scroll: gtk::ScrolledWindow,
    league: gtk::Entry,
    online: gtk::CheckButton,
    scale: gtk::SpinButton,
    range: gtk::SpinButton,
    automatic: gtk::CheckButton,
    merchant: gtk::CheckButton,
    collapse: gtk::CheckButton,
    stock: gtk::CheckButton,
    price_currency: gtk::ComboBoxText,
    bulk_currency: gtk::ComboBoxText,
    profile: gtk::SpinButton,
    show_new: gtk::CheckButton,
    keys: Vec<gtk::Entry>,
    record: Vec<gtk::Button>,
    capture: Cell<Option<usize>>,
    message: gtk::Label,
    base: RefCell<Settings>,
}
fn row(grid: &gtk::Grid, index: i32, label: &str, widget: &impl IsA<gtk::Widget>) {
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.);
    text.set_line_wrap(true);
    grid.attach(&text, 0, index, 1, 1);
    grid.attach(widget, 1, index, 1, 1);
}
fn short_number(spin: &gtk::SpinButton) {
    spin.set_digits(20);
    spin.set_numeric(false);
    spin.connect_local("output", false, |values| {
        let spin = values[0]
            .get::<gtk::SpinButton>()
            .expect("SpinButton output receiver");
        spin.set_text(&spin.value().to_string());
        Some(true.to_value())
    });
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let form = gtk::Notebook::new();
        let general = gtk::Grid::new();
        general.set_row_spacing(10);
        general.set_column_spacing(12);
        general.set_border_width(10);
        let league = gtk::Entry::new();
        league.set_hexpand(true);
        league.set_max_length(100);
        row(&general, 0, "Ligue", &league);
        let scale = gtk::SpinButton::with_range(0.75, 2.5, 0.05);
        short_number(&scale);
        row(&general, 1, "Échelle de l’interface", &scale);
        let range = gtk::SpinButton::with_range(0., 100., 1.);
        short_number(&range);
        row(&general, 2, "Plage suggérée ± %", &range);
        let price_currency = gtk::ComboBoxText::new();
        price_currency.append(Some("any"), "Automatique (selon l’objet)");
        price_currency.append(Some("chaos_divine"), "Chaos ou Divine");
        let bulk_currency = gtk::ComboBoxText::new();
        for (tag, name) in crate::filters::PAYMENT_CURRENCIES {
            price_currency.append(Some(tag), name);
            bulk_currency.append(Some(tag), name);
        }
        row(&general, 3, "Devise des prix par défaut", &price_currency);
        row(&general, 4, "Paiement des échanges", &bulk_currency);
        let online = gtk::CheckButton::with_label("Rechercher en ligne");
        let automatic = gtk::CheckButton::with_label("Rechercher automatiquement après la copie");
        let merchant = gtk::CheckButton::with_label("Offres disponibles auprès des marchands");
        let collapse =
            gtk::CheckButton::with_label("Regrouper les offres par compte via le service Trade");
        let stock = gtk::CheckButton::with_label("Activer le filtre du stock de la pile copiée");
        for (index, w) in [&online, &automatic, &merchant, &collapse, &stock]
            .into_iter()
            .enumerate()
        {
            general.attach(w, 0, 5 + index as i32, 2, 1);
        }
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(280);
        scroll.set_max_content_height(480);
        scroll.set_propagate_natural_height(true);
        scroll.add(&general);
        form.append_page(&scroll, Some(&gtk::Label::new(Some("Général"))));
        let shortcuts = gtk::Grid::new();
        shortcuts.set_row_spacing(8);
        shortcuts.set_column_spacing(8);
        shortcuts.set_border_width(10);
        let labels = [
            "Consulter les prix",
            "Évaluation locale",
            "Autoriser les clics",
            "Masquer le panneau",
            "Ouvrir les réglages",
        ];
        let mut keys = Vec::new();
        for (i, name) in labels.into_iter().enumerate() {
            let entry = gtk::Entry::new();
            entry.set_width_chars(18);
            entry.set_max_length(96);
            entry.set_placeholder_text(Some("Désactivé"));
            entry.set_tooltip_text(Some(
                "Exemple : Ctrl + D. Vider le champ désactive ce raccourci.",
            ));
            row(&shortcuts, i as i32, name, &entry);
            keys.push(entry);
        }
        let record = keys
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let button = gtk::Button::with_label("Capturer");
                shortcuts.attach(&button, 2, i as i32, 1, 1);
                button
            })
            .collect();
        let hint=gtk::Label::new(Some("Les raccourcis agissent lorsque PoE a le clavier. Les combinaisons réservées au jeu et les doublons sont refusés. Si tous les raccourcis sont désactivés, les commandes settings et interact restent disponibles."));
        hint.set_line_wrap(true);
        hint.set_xalign(0.);
        shortcuts.attach(&hint, 0, 6, 2, 1);
        // Capture buttons are wired by the owner using arm_capture().
        form.append_page(&shortcuts, Some(&gtk::Label::new(Some("Raccourcis"))));
        let maps = gtk::Grid::new();
        maps.set_row_spacing(12);
        maps.set_column_spacing(12);
        maps.set_border_width(10);
        let profile = gtk::SpinButton::with_range(1., 3., 1.);
        row(&maps, 0, "Profil des mods de zone", &profile);
        let show_new = gtk::CheckButton::with_label("Signaler les nouveaux mods");
        maps.attach(&show_new, 0, 1, 2, 1);
        let note=gtk::Label::new(Some("Les décisions par mod se configurent dans Évaluer → Configurer les mods de zone. Changer ces champs conserve les règles des trois profils."));
        note.set_line_wrap(true);
        note.set_xalign(0.);
        maps.attach(&note, 0, 2, 2, 1);
        form.append_page(&maps, Some(&gtk::Label::new(Some("Cartes"))));
        container.pack_start(&form, true, true, 0);
        let message = gtk::Label::new(None);
        message.set_line_wrap(true);
        message.set_xalign(0.);
        container.pack_start(&message, false, false, 0);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let reset = gtk::Button::with_label("Réinitialiser ces champs");
        let cancel = gtk::Button::with_label("Annuler");
        let apply = gtk::Button::with_label("Enregistrer");
        for button in [&reset, &cancel, &apply] {
            actions.pack_start(button, true, true, 0);
        }
        container.pack_start(&actions, false, false, 0);
        Self {
            container,
            apply,
            cancel,
            reset,
            form,
            general_scroll: scroll,
            league,
            online,
            scale,
            range,
            automatic,
            merchant,
            collapse,
            stock,
            price_currency,
            bulk_currency,
            profile,
            show_new,
            keys,
            record,
            capture: Cell::new(None),
            message,
            base: RefCell::new(Settings::default()),
        }
    }
    pub fn show(&self, settings: &Settings) {
        self.base.replace(settings.clone());
        self.message
            .set_size_request(-1, (60. * settings.scale) as i32);
        self.general_scroll.vadjustment().set_value(0.);
        self.fill(settings);
        self.capture.set(None);
        self.message
            .set_text("Les modifications restent en brouillon jusqu’à Enregistrer.");
        self.busy(false);
        self.form.set_current_page(Some(0));
        self.container.show_all();
    }
    pub fn defaults(&self) {
        self.fill(&Settings::default());
        self.message.set_text("Valeurs par défaut dans ce brouillon. Les règles avancées et les décisions par mod sont conservées.");
    }
    fn fill(&self, s: &Settings) {
        self.league.set_text(&s.league);
        self.online.set_active(s.filters.online);
        self.scale.set_value(s.scale);
        self.scale.set_text(&s.scale.to_string());
        self.range.set_value(s.search_range);
        self.range.set_text(&s.search_range.to_string());
        self.automatic.set_active(s.automatic_search);
        self.merchant.set_active(s.filters.merchant_only);
        self.collapse.set_active(s.filters.collapse_listings);
        self.stock.set_active(s.filters.activate_stock_filter);
        self.price_currency
            .set_active_id(Some(s.filters.default_currency.as_deref().unwrap_or("any")));
        self.bulk_currency
            .set_active_id(Some(&s.filters.bulk_currency));
        self.profile.set_value(s.map_check.profile as f64);
        self.show_new.set_active(s.map_check.show_new_stats);
        for (entry, key) in self.keys.iter().zip([
            &s.hotkeys.price,
            &s.hotkeys.inspect,
            &s.hotkeys.interact,
            &s.hotkeys.hide,
            &s.hotkeys.settings,
        ]) {
            entry.set_text(key.as_deref().unwrap_or(""));
        }
    }
    pub fn read(&self) -> Result<Settings, String> {
        self.scale.update();
        self.range.update();
        self.profile.update();
        let mut s = self.base.borrow().clone();
        s.league = self.league.text().trim().into();
        s.filters.online = self.online.is_active();
        s.scale = self.scale.value();
        s.search_range = self.range.value();
        s.automatic_search = self.automatic.is_active();
        s.filters.merchant_only = self.merchant.is_active();
        s.filters.collapse_listings = self.collapse.is_active();
        s.filters.activate_stock_filter = self.stock.is_active();
        s.filters.default_currency = self
            .price_currency
            .active_id()
            .filter(|id| id != "any")
            .map(|s| s.to_string());
        s.filters.bulk_currency = self
            .bulk_currency
            .active_id()
            .ok_or("Choisissez une devise d’échange.")?
            .to_string();
        s.map_check.profile = self.profile.value_as_int() as u8;
        s.map_check.show_new_stats = self.show_new.is_active();
        let values: Vec<_> = self
            .keys
            .iter()
            .map(|entry| {
                let text = entry.text().trim().to_string();
                (!text.is_empty()).then_some(text)
            })
            .collect();
        s.hotkeys = Hotkeys {
            price: values[0].clone(),
            inspect: values[1].clone(),
            interact: values[2].clone(),
            hide: values[3].clone(),
            settings: values[4].clone(),
        }
        .normalized()?;
        s.validate()?;
        Ok(s)
    }
    pub fn busy(&self, busy: bool) {
        self.form.set_sensitive(!busy);
        self.apply.set_sensitive(!busy);
        self.cancel.set_sensitive(!busy);
        self.reset.set_sensitive(!busy);
        if busy {
            self.message.set_text("Enregistrement des réglages…");
        }
    }
    pub fn error(&self, error: &str) {
        self.busy(false);
        self.message.set_text(error);
    }
    pub fn arm_capture(&self, index: usize) {
        self.capture.set(Some(index));
        self.message
            .set_text("Appuyez sur le raccourci voulu ; Retour arrière efface ce champ.");
    }
    pub fn capture_key(&self, event: &gdk::EventKey) -> bool {
        let Some(index) = self.capture.get() else {
            return false;
        };
        if event.keyval() == gdk::keys::constants::BackSpace {
            self.keys[index].set_text("");
            self.capture.set(None);
            return true;
        }
        let symbol = gdk::Display::default()
            .and_then(|d| gdk::Keymap::for_display(&d))
            .and_then(|m| {
                m.translate_keyboard_state(
                    event.hardware_keycode() as u32,
                    gdk::ModifierType::empty(),
                    event.group() as i32,
                )
            })
            .map(|(key, _, _, _)| key)
            .unwrap_or_else(|| *event.keyval());
        if (0xffe1..=0xffee).contains(&symbol) && symbol != 0xffe5 {
            return true;
        }
        let mods = event.state();
        if let Some(h) = Hotkey::from_keysym(
            symbol,
            mods.contains(gdk::ModifierType::CONTROL_MASK),
            mods.contains(gdk::ModifierType::SHIFT_MASK),
            mods.contains(gdk::ModifierType::MOD1_MASK),
            mods.intersects(gdk::ModifierType::SUPER_MASK | gdk::ModifierType::MOD4_MASK),
        ) {
            self.keys[index].set_text(&h.text());
            self.capture.set(None);
            self.message
                .set_text("Raccourci capturé dans le brouillon.");
        } else {
            self.message
                .set_text("Touche non prise en charge. Choisissez une autre combinaison.");
        }
        true
    }
    pub fn capture_buttons(&self) -> Vec<gtk::Button> {
        self.record.clone()
    }
    pub fn cancel_capture(&self) {
        self.capture.set(None);
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"visible":self.container.is_mapped(),"capture":self.capture.get(),"record":self.record.iter().map(|b|rect(b.upcast_ref())).collect::<Vec<_>>(),"tabs":(0..3).map(|i|self.form.nth_page(Some(i)).and_then(|c|self.form.tab_label(&c)).and_then(|w|rect(&w))).collect::<Vec<_>>(),"page":self.form.current_page(),"message":self.message.text().as_str(),"league":{"value":self.league.text().as_str(),"focus":self.league.has_focus(),"editable":self.league.is_editable(),"selection":self.league.selection_bounds(),"position":self.league.position(),"rect":rect(self.league.upcast_ref())},"scale":{"value":self.scale.value(),"text":self.scale.text().as_str(),"focus":self.scale.has_focus(),"rect":rect(self.scale.upcast_ref())},"range":{"value":self.range.value(),"text":self.range.text().as_str(),"rect":rect(self.range.upcast_ref())},"automatic":{"value":self.automatic.is_active(),"rect":rect(self.automatic.upcast_ref())},"apply":rect(self.apply.upcast_ref()),"cancel":rect(self.cancel.upcast_ref()),"reset":rect(self.reset.upcast_ref()),"notebook":rect(self.form.upcast_ref()),"general_scroll":self.general_scroll.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":self.general_scroll.allocated_width(),"height":self.general_scroll.allocated_height(),"value":self.general_scroll.vadjustment().value(),"upper":self.general_scroll.vadjustment().upper(),"page_size":self.general_scroll.vadjustment().page_size()})),"online":{"value":self.online.is_active(),"rect":rect(self.online.upcast_ref())},"merchant":{"value":self.merchant.is_active(),"rect":rect(self.merchant.upcast_ref())},"collapse":{"value":self.collapse.is_active(),"rect":rect(self.collapse.upcast_ref())},"stock":{"value":self.stock.is_active(),"rect":rect(self.stock.upcast_ref())},"price_currency":{"value":self.price_currency.active_id().as_deref(),"rect":rect(self.price_currency.upcast_ref())},"bulk_currency":{"value":self.bulk_currency.active_id().as_deref(),"rect":rect(self.bulk_currency.upcast_ref())},"profile":{"value":self.profile.value_as_int(),"rect":rect(self.profile.upcast_ref())},"show_new":{"value":self.show_new.is_active(),"rect":rect(self.show_new.upcast_ref())},"hotkeys":self.keys.iter().map(|e|serde_json::json!({"value":e.text().as_str(),"rect":rect(e.upcast_ref())})).collect::<Vec<_>>()})
    }
}
