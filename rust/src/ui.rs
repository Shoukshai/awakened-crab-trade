use crate::{
    desktop::{
        clipboard,
        wayland::{self, Snapshot},
    },
    economy::{Economy, Report},
    filters,
    item::Item,
    overlay::{DesktopState, HideReason, Overlay, State},
    settings::Settings,
    trade::{Prices, SearchKind, Trade},
};
mod delve_grid;
mod economy_panel;
mod image_manager;
mod image_panel;
mod image_settings;
mod inspection_panel;
mod item_filters_panel;
mod item_search_panel;
mod mercenary_panel;
mod settings_panel;
mod stat_row;
mod stopwatch_manager;
mod stopwatch_panel;
mod stopwatch_settings;
mod unidentified_panel;
mod widget_canvas;
use gtk::{gdk, prelude::*};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    os::unix::{io::AsRawFd, net::UnixDatagram},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};

type EconomyResult = Result<Report, String>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Price,
    Inspection,
    Settings,
    Catalogue,
    Stopwatches,
    Images,
}
enum Event {
    WidgetOrder,
    Shortcut(&'static str),
    Hotkey(crate::hotkeys::Action),
    Stopwatch(stopwatch_panel::Action),
    StopwatchManager(stopwatch_manager::Action),
    Image(image_panel::Action),
    ImageManager(image_manager::Action),
    Windows(Snapshot),
    TrackerError(String),
    Captured(u64, View, Box<Result<(Item, u128), String>>),
    Prices(u64, Result<Prices, String>),
    Economy(u64, Box<EconomyResult>),
    MapSettings(u64, crate::inspection::Edit),
    Preferences(Box<crate::desktop::preferences::Update>),
    FiltersChanged(u64, String),
    IdentifyUnique(u64, String),
    Catalogue(item_search_panel::Action),
    CataloguePrices(u64, Box<Result<Vec<(usize, Report)>, String>>),
}
enum EconomyJob {
    Price {
        generation: u64,
        league: String,
        item: Box<Item>,
    },
    Catalogue {
        generation: u64,
        league: String,
        choices: Vec<crate::item::search::Choice>,
    },
}
struct CaptureJob {
    generation: u64,
    copy: bool,
    view: View,
}
struct Job {
    generation: u64,
    league: String,
    query: serde_json::Value,
}
struct Ui {
    widget_canvas: Result<Rc<widget_canvas::Canvas>, String>,
    window: gtk::Window,
    delve: Result<Rc<delve_grid::Grid>, String>,
    stopwatches: Result<Rc<stopwatch_panel::Runtime>, String>,
    stopwatch_manager: stopwatch_manager::Manager,
    images: Result<Rc<image_panel::Runtime>, String>,
    image_manager: image_manager::Panel,
    pending_image_edit: Cell<Option<(u64, u64)>>,
    widget_transaction: Cell<bool>,
    order_transaction: Cell<bool>,
    pending_interaction: Cell<Option<u64>>,
    pending_stopwatch_edit: Cell<Option<u64>>,
    started: Instant,
    desktop: RefCell<Snapshot>,
    title: gtk::Label,
    text: gtk::Label,
    hint: gtk::Label,
    league: gtk::Entry,
    online: gtk::CheckButton,
    search_range: gtk::SpinButton,
    currency: gtk::ComboBoxText,
    economy_panel: economy_panel::EconomyPanel,
    unidentified: unidentified_panel::Panel,
    catalogue: Rc<item_search_panel::Panel>,
    identity_epoch: Cell<u64>,
    economy_worker: SyncSender<EconomyJob>,
    economy_result: RefCell<Option<(u64, EconomyResult)>>,
    mod_box: gtk::Box,
    mod_scroll: gtk::ScrolledWindow,
    mod_expander: gtk::Expander,
    stack: gtk::Stack,
    inspection: Rc<inspection_panel::Panel>,
    inspection_result: RefCell<Option<crate::inspection::Report>>,
    view: Cell<View>,
    stat_rows: RefCell<Vec<stat_row::StatRow>>,
    mercenary_panel: RefCell<Option<mercenary_panel::Panel>>,
    preset: gtk::ComboBoxText,
    item_filters: Rc<item_filters_panel::Panel>,
    events: glib::Sender<Event>,
    updating_preset: Cell<bool>,
    state: RefCell<Overlay>,
    settings: RefCell<Settings>,
    preferences: settings_panel::Panel,
    preferences_worker: SyncSender<crate::desktop::preferences::Request>,
    preferences_pending: Cell<bool>,
    preferences_epoch: Cell<u64>,
    preferences_return: Cell<View>,
    backend_connection: Cell<u64>,
    backend_status: RefCell<Result<(), String>>,
    updating_settings: Cell<bool>,
    css: gtk::CssProvider,
    item: RefCell<Option<Item>>,
    current: Arc<AtomicU64>,
    game_active: Arc<AtomicBool>,
    worker: SyncSender<Job>,
    capture_worker: SyncSender<CaptureJob>,
    url: RefCell<Option<String>>,
    shown: RefCell<Option<Instant>>,
    result: RefCell<Option<(u64, Result<Prices, String>)>>,
    request: RefCell<Option<(u64, serde_json::Value)>>,
}
fn set_style(window: &gtk::Window, css: &gtk::CssProvider, scale: f64) -> Result<(), String> {
    window.set_size_request((620. * scale) as i32, (420. * scale) as i32);
    css.load_from_data(format!("#apt-native {{ background-color: rgba(14,20,29,0.97); color: #eeeeef; border: 2px solid #b99a5b; border-radius: 10px; }} #apt-native label {{ font: {}px Sans; color: #eeeeef; }} #apt-native button {{ padding: 7px; }} #apt-native button, #apt-native entry, #apt-native spinbutton {{ color: #eeeeef; background-image: none; background-color: #263343; }} #apt-native button.map-danger {{ background-image: none; background-color: #7c2935; }} #apt-native button.map-warning {{ background-image: none; background-color: #865320; }} #apt-native button.map-good {{ background-image: none; background-color: #28603e; }} #apt-native .profile-active {{ border: 2px solid #d9b86b; }} #apt-native treeview {{ background-color: #182230; color: #eeeeef; }} #apt-native notebook, #apt-native notebook > stack, #apt-native notebook > header, #apt-native viewport {{ background-color: #182230; color: #eeeeef; }} #apt-native notebook > header > tabs > tab {{ background-color: #263343; }}",(16.0*scale) as u32).as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}
fn socket_path() -> Result<std::path::PathBuf, String> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("awakened-poe-trade-rs.sock"))
        .ok_or("XDG_RUNTIME_DIR is required".into())
}
fn apply_input(window: &gtk::Window, interactive: bool) {
    window.set_keyboard_mode(if interactive {
        KeyboardMode::OnDemand
    } else {
        KeyboardMode::None
    });
    if let Some(w) = window.window() {
        let region = if interactive {
            gtk::cairo::Region::create_rectangle(&gtk::cairo::RectangleInt::new(
                0,
                0,
                window.allocated_width(),
                window.allocated_height(),
            ))
        } else {
            gtk::cairo::Region::create()
        };
        w.input_shape_combine_region(&region, 0, 0);
    }
}
fn status_payload(value: &serde_json::Value) -> String {
    let encoded = value.to_string();
    if encoded.len() <= 60 * 1024 {
        return encoded;
    }
    // The local client reads one 64 KiB datagram. A large collection of widget
    // controls must still produce a useful, valid response instead of EMSGSIZE.
    serde_json::json!({
        "diagnostics_truncated":true,"diagnostics_bytes":encoded.len(),
        "state":value["state"],"view":value["view"],"game_active":value["game_active"],
        "visible":value["visible"],"rendered":value["rendered"],"keyboard_mode":value["keyboard_mode"],
        "generation":value["generation"],"origin":value["origin"],"width":value["width"],"height":value["height"],
        "shortcut_backend":value["shortcut_backend"],"preferences_pending":value["preferences_pending"],
        "counts":{"image_strips":value["images"]["rows"].as_array().map(Vec::len),"stopwatches":value["stopwatches"]["rows"].as_array().map(Vec::len),"stat_controls":value["stat_controls"].as_array().map(Vec::len)}
    }).to_string()
}
fn belongs_to_window(widget: &gtk::Widget, window: &gtk::Window, depth: u8) -> bool {
    if widget == window.upcast_ref::<gtk::Widget>() {
        return true;
    }
    if depth >= 8 {
        return false;
    }
    if let Some(menu) = widget.downcast_ref::<gtk::Menu>() {
        if menu
            .attach_widget()
            .is_some_and(|w| belongs_to_window(&w, window, depth + 1))
        {
            return true;
        }
    }
    if let Some(w) = widget.downcast_ref::<gtk::Window>() {
        if w.attached_to()
            .is_some_and(|a| belongs_to_window(&a, window, depth + 1))
            || w.transient_for()
                .is_some_and(|p| belongs_to_window(p.upcast_ref(), window, depth + 1))
        {
            return true;
        }
    }
    widget
        .toplevel()
        .filter(|top| top != widget)
        .is_some_and(|top| belongs_to_window(&top, window, depth + 1))
}
fn owned_popup(window: &gtk::Window) -> Option<gtk::Widget> {
    let grab = gtk::grab_get_current()?;
    let popup = grab.is::<gtk::Menu>()
        || grab.is::<gtk::Popover>()
        || grab
            .toplevel()
            .is_some_and(|top| top != *window.upcast_ref::<gtk::Widget>());
    (popup && belongs_to_window(&grab, window, 0)).then_some(grab)
}
fn dismiss_owned_popup(window: &gtk::Window) {
    if let Some(grab) = owned_popup(window) {
        if let Some(menu) = grab.downcast_ref::<gtk::Menu>() {
            menu.popdown();
        } else if let Some(popover) = grab.downcast_ref::<gtk::Popover>() {
            popover.popdown();
            popover.hide();
        } else {
            grab.grab_remove();
            if let Some(top) = grab.toplevel() {
                top.hide();
            }
        }
    }
}
impl Ui {
    fn defer_focus_check(self: &Rc<Self>) {
        let Some(token) = self.state.borrow_mut().focus_lost() else {
            return;
        };
        let weak = Rc::downgrade(self);
        // GTK menus temporarily move keyboard focus to another native surface.
        // Observe settled state; never request activation or focus to compensate.
        glib::timeout_add_local_once(Duration::from_millis(30), move || {
            if let Some(ui) = weak.upgrade() {
                let active = ui.window.is_active()
                    || ui.stopwatches.as_ref().is_ok_and(|r| r.owns_focus())
                    || ui.images.as_ref().is_ok_and(|r| r.owns_focus());
                let popup = owned_popup(&ui.window).is_some();
                let hide = ui
                    .state
                    .borrow()
                    .should_hide_after_focus_loss(token, active, popup);
                eprintln!("overlay event=focus-check token={token} elapsed_ms={} main_active={active} owned_popup={popup} hide={hide}",ui.started.elapsed().as_millis());
                if hide {
                    ui.hide(HideReason::FocusLost);
                }
            }
        });
    }
    fn refresh_delve(&self) {
        if let Ok(grid) = &self.delve {
            let desktop = self.desktop.borrow();
            grid.update(
                desktop.game_present(),
                desktop.game_active(),
                desktop.other_active(),
                self.state.borrow().state == State::Interactive,
                self.window.is_visible()
                    && matches!(
                        self.view.get(),
                        View::Price | View::Inspection | View::Settings
                    ),
            );
        }
        if let Ok(r) = &self.stopwatches {
            let d = self.desktop.borrow();
            r.set_context(
                d.game_present() && d.game_active() && !d.other_active(),
                self.state.borrow().state == State::Interactive
                    && d.game_present()
                    && !d.other_active(),
                self.window.is_visible()
                    && matches!(
                        self.view.get(),
                        View::Price | View::Inspection | View::Settings
                    ),
                self.window.is_visible() && self.view.get() == View::Stopwatches,
            );
        }
        if let Ok(r) = &self.images {
            let d = self.desktop.borrow();
            r.set_context(
                self.state.borrow().state == State::Interactive
                    && d.game_present()
                    && !d.other_active(),
                self.window.is_visible()
                    && matches!(
                        self.view.get(),
                        View::Price | View::Inspection | View::Settings
                    ),
            );
        }
    }

    fn toggle_delve(&self) {
        if self.preferences_pending.get() {
            return;
        }
        let Ok(grid) = &self.delve else {
            eprintln!("delve unavailable: {:?}", self.delve.as_ref().err());
            return;
        };
        let (present, active, other) = {
            let d = self.desktop.borrow();
            (d.game_present(), d.game_active(), d.other_active())
        };
        let owned = self.window.is_visible() && self.state.borrow().state == State::Interactive;
        let allowed = present && !other && (active || owned);
        if allowed && !grid.requested() && self.window.is_visible() {
            self.hide(HideReason::Button);
        }
        grid.toggle(allowed, owned);
        self.refresh_delve();
    }
    fn hide_delve(&self, reason: crate::delve::Reason) {
        if let Ok(grid) = &self.delve {
            grid.hide(reason);
        }
    }
    fn sync_generation(&self) {
        self.current
            .store(self.state.borrow().generation, Ordering::Release);
    }
    fn hide(&self, reason: HideReason) {
        self.preferences.images.cancel_import();
        self.pending_interaction.set(None);
        self.pending_image_edit.set(None);
        self.pending_stopwatch_edit.set(None);
        if matches!(reason, HideReason::Explicit | HideReason::Quit) {
            if let Ok(r) = &self.stopwatches {
                r.dismiss();
            }
            if let Ok(r) = &self.images {
                r.dismiss();
            }
        }
        match reason {
            HideReason::OtherApplication => self.hide_delve(crate::delve::Reason::OtherApplication),
            HideReason::GameGone | HideReason::GameMinimized => {
                self.hide_delve(crate::delve::Reason::GameGone)
            }
            HideReason::GameInactive => self.hide_delve(crate::delve::Reason::GameInactive),
            HideReason::TrackerFailure => self.hide_delve(crate::delve::Reason::TrackerFailure),
            HideReason::Quit => self.hide_delve(crate::delve::Reason::Quit),
            _ => (),
        }
        let previous = self.state.borrow().state;
        let focused = self.window.is_active();
        self.state.borrow_mut().hide(reason);
        self.sync_generation();
        apply_input(&self.window, false);
        dismiss_owned_popup(&self.window);
        self.window.hide();
        self.refresh_delve();
        let state = self.state.borrow();
        let desktop = self.desktop.borrow();
        eprintln!("overlay state=Hidden reason={reason:?} previous={previous:?} generation={} elapsed_ms={} was_focused={focused} game_active={} game_exists={} game_visible={} other_active={} input=empty keyboard=None", state.generation,self.started.elapsed().as_millis(),state.game_active,desktop.game_exists(),desktop.game_present(),desktop.other_active());
    }
    fn show(&self) -> bool {
        let generation = self.state.borrow_mut().show();
        if generation.is_none() {
            eprintln!("show refused: PoE is not active");
            return false;
        }
        self.sync_generation();
        self.shown.replace(Some(Instant::now()));
        apply_input(&self.window, false);
        self.window.show_all();
        self.refresh_delve();
        self.window.queue_draw();
        let settings = self.settings.borrow();
        self.hint.set_text(&format!(
            "Passif · {} pour interagir · {} pour fermer",
            settings
                .hotkeys
                .interact
                .as_deref()
                .unwrap_or("commande interact"),
            settings.hotkeys.hide.as_deref().unwrap_or("commande hide")
        ));
        eprintln!(
            "overlay state=Preparing generation={} scale={}",
            self.state.borrow().generation,
            self.window.scale_factor()
        );
        true
    }
    fn invalidate_prices(&self) -> u64 {
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        self.result.replace(None);
        self.request.replace(None);
        self.url.replace(None);
        self.economy_result.replace(None);
        self.economy_panel.not_requested();
        self.state.borrow().generation
    }
    fn open_catalogue(&self) {
        if self.preferences_pending.get()
            || (self.view.get() == View::Catalogue && self.window.is_visible())
        {
            return;
        }
        let generation = self.invalidate_prices();
        self.view.set(View::Catalogue);
        self.title.set_text("Recherche des cibles de Heist");
        self.catalogue
            .show(&self.settings.borrow().league, generation);
        self.stack.set_visible_child_name("catalogue");
        self.refresh_delve();
        if !self.window.is_visible() {
            self.show();
        }
    }
    fn catalogue_action(&self, action: item_search_panel::Action) {
        if self.view.get() != View::Catalogue
            || !self.window.is_visible()
            || self.state.borrow().state != State::Interactive
            || self.preferences_pending.get()
            || action.session() != self.catalogue.session()
        {
            return;
        }
        use item_search_panel::Action;
        match action {
            Action::Select { revision, id, .. } => {
                if self.catalogue.select(revision, id) {
                    self.invalidate_prices();
                    self.catalogue_prices();
                }
            }
            Action::Refresh(_) => {
                self.invalidate_prices();
                self.catalogue.clear_quotes();
                self.catalogue_prices();
            }
            Action::Reset(_) => {
                self.invalidate_prices();
                self.catalogue.reset();
            }
            Action::Close(_) => self.hide(HideReason::Button),
            Action::Consult { id, .. } => {
                let Some(choice) = self.catalogue.selected_choice(id) else {
                    return;
                };
                match choice.trade_item() {
                    Ok(item) => {
                        let generation = self.invalidate_prices();
                        self.prepare_item(item, View::Price, generation);
                        self.search();
                    }
                    Err(error) => self.catalogue.show_quotes(Err(error)),
                }
            }
        }
    }
    fn catalogue_prices(&self) {
        let choices = self.catalogue.choices();
        if choices.is_empty() {
            return;
        }
        self.catalogue.loading();
        let generation = self.state.borrow().generation;
        let job = EconomyJob::Catalogue {
            generation,
            league: self.settings.borrow().league.clone(),
            choices,
        };
        if self.economy_worker.try_send(job).is_err() {
            self.catalogue
                .show_quotes(Err("Une demande indicative est déjà en cours".into()));
        } else {
            eprintln!("catalogue economy generation={generation} trade_http=false");
        }
    }
    fn open_images(&self) {
        if self.preferences_pending.get() {
            return;
        }
        self.invalidate_prices();
        self.view.set(View::Images);
        self.title.set_text("Bandes d’images");
        self.stack.set_visible_child_name("images");
        if let Ok(r) = &self.images {
            r.resume();
        }
        if !self.window.is_visible() {
            self.show();
        }
        self.refresh_delve();
    }
    fn image_manager_action(&self, action: image_manager::Action) {
        if self.view.get() != View::Images
            || !self.window.is_visible()
            || self.state.borrow().state != State::Interactive
            || self.preferences_pending.get()
        {
            return;
        }
        match action {
            image_manager::Action::Edit(id) => {
                if let Ok(r) = &self.images {
                    r.prepare_edit(id);
                }
                self.open_settings();
                self.preferences.open_images(Some(id));
            }
            image_manager::Action::Add => {
                let mut s = self.settings.borrow().clone();
                if s.image_strips.len() >= crate::image_strip::MAX_STRIPS {
                    return;
                }
                let id = s.image_strips.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                s.image_strips.push(crate::image_strip::Config::new(
                    id,
                    glib::random_double(),
                    glib::random_double(),
                ));
                self.commit_widgets(s);
            }
            image_manager::Action::Wanted(id, wanted) => {
                let mut s = self.settings.borrow().clone();
                if let Some(c) = s.image_strips.iter_mut().find(|c| c.id == id) {
                    c.wanted = wanted;
                    self.commit_widgets(s);
                }
            }
        }
    }
    fn image_action(self: &Rc<Self>, action: image_panel::Action) {
        let Ok(r) = &self.images else {
            return;
        };
        use image_panel::Action;
        match action {
            Action::FocusChanged => {
                if r.owns_focus() {
                    self.state.borrow_mut().focus_restored();
                } else {
                    self.defer_focus_check();
                }
            }
            Action::Escape(epoch) if r.accepts(epoch) => self.hide(HideReason::Escape),
            Action::Edit(epoch, id) if r.accepts(epoch) && !self.preferences_pending.get() => {
                r.prepare_edit(id);
                self.hide(HideReason::Button);
                if self.desktop.borrow().game_active() {
                    self.open_settings();
                    self.preferences.open_images(Some(id));
                } else {
                    self.pending_image_edit
                        .set(Some((self.state.borrow().generation, id)));
                }
            }
            Action::Hide(epoch, id) | Action::Remove(epoch, id)
                if r.accepts(epoch) && !self.preferences_pending.get() =>
            {
                let remove = matches!(action, Action::Remove(..));
                let mut s = self.settings.borrow().clone();
                if remove {
                    s.image_strips.retain(|c| c.id != id);
                } else if let Some(c) = s.image_strips.iter_mut().find(|c| c.id == id) {
                    c.wanted = false;
                }
                self.commit_widgets(s);
            }
            Action::Move { epoch, config }
                if r.accepts(epoch) && !self.preferences_pending.get() =>
            {
                let mut s = self.settings.borrow().clone();
                if let Some(c) = s.image_strips.iter_mut().find(|c| c.id == config.id) {
                    *c = config;
                    self.commit_widgets(s);
                }
            }
            _ => (),
        }
    }
    fn open_stopwatches(&self) {
        if self.preferences_pending.get() {
            return;
        }
        self.invalidate_prices();
        self.view.set(View::Stopwatches);
        self.title.set_text("Chronomètres");
        self.stack.set_visible_child_name("stopwatches");
        if let Ok(r) = &self.stopwatches {
            r.resume();
        }
        self.update_stopwatch_manager();
        if !self.window.is_visible() {
            self.show();
        }
        self.refresh_delve();
    }
    fn update_stopwatch_manager(&self) {
        if let Ok(r) = &self.stopwatches {
            self.stopwatch_manager.update(&r.readings());
        }
    }
    fn stopwatch_time(&self, id: u64, reset: bool) {
        let d = self.desktop.borrow();
        if !d.game_present()
            || d.other_active()
            || !(d.game_active() || self.state.borrow().state == State::Interactive)
        {
            return;
        }
        drop(d);
        if let Ok(r) = &self.stopwatches {
            r.time_action(id, reset);
        }
        self.update_stopwatch_manager();
    }
    fn save_widget_order(&self) {
        if !self.preferences_pending.get() && self.widget_canvas.as_ref().is_ok_and(|c| c.dirty()) {
            let settings = self.settings.borrow().clone();
            self.commit_widgets(settings);
        }
    }
    fn commit_widgets(&self, mut candidate: Settings) {
        if let Ok(canvas) = &self.widget_canvas {
            candidate.widget_order =
                crate::widget_order::saved(&canvas.order(), &candidate.widget_keys());
        }
        if self.preferences_pending.get() {
            return;
        }
        if let Err(e) = candidate.validate() {
            self.stopwatch_manager.message(&e);
            return;
        }
        self.preferences_epoch.set(self.preferences_epoch.get() + 1);
        self.preferences_pending.set(true);
        self.widget_transaction.set(true);
        let order_only = {
            let old = self.settings.borrow();
            candidate.stopwatches == old.stopwatches && candidate.image_strips == old.image_strips
        };
        self.order_transaction.set(order_only);
        if !order_only {
            self.stopwatch_manager.busy(true);
            self.image_manager.busy(true);
            if let Ok(r) = &self.images {
                r.set_busy(true);
            }
            if let Ok(r) = &self.stopwatches {
                r.set_busy(true);
            }
        }
        if let Err(e) = self
            .preferences_worker
            .try_send(crate::desktop::preferences::Request {
                epoch: self.preferences_epoch.get(),
                settings: candidate,
            })
        {
            self.preferences_pending.set(false);
            self.widget_transaction.set(false);
            if let Ok(c) = &self.widget_canvas {
                let s = self.settings.borrow();
                c.rollback(&s.widget_order, &s.widget_keys());
            }
            if self.order_transaction.replace(false) {
                self.stopwatch_manager
                    .message(&format!("Modifications non enregistrées : {e}"));
                self.image_manager
                    .message(&format!("Modifications non enregistrées : {e}"));
                return;
            }
            self.stopwatch_manager.busy(false);
            self.image_manager.busy(false);
            self.image_manager.message(&e.to_string());
            if let Ok(r) = &self.images {
                r.set_busy(false);
                r.reconcile(
                    &self.settings.borrow().image_strips,
                    self.settings.borrow().scale,
                );
            }
            self.stopwatch_manager.message(&e.to_string());
            if let Ok(r) = &self.stopwatches {
                r.set_busy(false);
                r.reconcile(
                    &self.settings.borrow().stopwatches,
                    self.settings.borrow().scale,
                );
            }
        }
    }
    fn stopwatch_manager_action(&self, action: stopwatch_manager::Action) {
        if self.view.get() != View::Stopwatches
            || !self.window.is_visible()
            || self.state.borrow().state != State::Interactive
            || self.preferences_pending.get()
        {
            return;
        }
        use stopwatch_manager::Action;
        match action {
            Action::Toggle(id) => self.stopwatch_time(id, false),
            Action::Reset(id) => self.stopwatch_time(id, true),
            Action::Edit => {
                self.open_settings();
                self.preferences.open_stopwatches();
            }
            Action::Add => {
                let mut s = self.settings.borrow().clone();
                if s.stopwatches.len() >= crate::stopwatch::MAX_INSTANCES {
                    return;
                }
                let id = s
                    .stopwatches
                    .iter()
                    .map(|c| c.id)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                s.stopwatches.push(crate::stopwatch::Config::new(
                    id,
                    glib::random_double(),
                    glib::random_double(),
                ));
                self.commit_widgets(s);
            }
            Action::Wanted(id, wanted) => {
                let mut s = self.settings.borrow().clone();
                if let Some(c) = s.stopwatches.iter_mut().find(|c| c.id == id) {
                    c.wanted = wanted;
                    self.commit_widgets(s);
                }
            }
        }
    }
    fn stopwatch_action(self: &Rc<Self>, action: stopwatch_panel::Action) {
        use stopwatch_panel::Action;
        let Ok(r) = &self.stopwatches else {
            return;
        };
        match action {
            Action::Tick => self.update_stopwatch_manager(),
            Action::Escape(epoch) if r.accepts(epoch) => self.hide(HideReason::Escape),
            Action::FocusChanged => {
                if r.owns_focus() {
                    self.state.borrow_mut().focus_restored();
                } else {
                    self.defer_focus_check();
                }
            }
            Action::Toggle(epoch, id) if r.accepts(epoch) => self.stopwatch_time(id, false),
            Action::Reset(epoch, id) if r.accepts(epoch) => self.stopwatch_time(id, true),
            Action::Edit(epoch, id) if r.accepts(epoch) && !self.preferences_pending.get() => {
                eprintln!("stopwatch edit id={id}");
                // Release this surface before opening an exclusive editor. Wait for
                // the compositor's game-focus snapshot; never request activation.
                self.hide(HideReason::Button);
                if self.desktop.borrow().game_active() {
                    self.open_settings();
                    self.preferences.open_stopwatches();
                } else {
                    self.pending_stopwatch_edit
                        .set(Some(self.state.borrow().generation));
                }
            }
            Action::Remove(epoch, id) if r.accepts(epoch) && !self.preferences_pending.get() => {
                let mut s = self.settings.borrow().clone();
                s.stopwatches.retain(|c| c.id != id);
                s.hotkeys.stopwatch_keys.retain(|c| c.id != id);
                self.commit_widgets(s);
            }
            Action::Move { epoch, id, x, y }
                if r.accepts(epoch) && !self.preferences_pending.get() =>
            {
                let mut s = self.settings.borrow().clone();
                if let Some(c) = s.stopwatches.iter_mut().find(|c| c.id == id) {
                    c.x = x;
                    c.y = y;
                    self.commit_widgets(s);
                }
            }
            _ => (),
        }
    }
    fn open_settings(&self) {
        if self.preferences_pending.get()
            || (self.view.get() == View::Settings && self.window.is_visible())
        {
            return;
        }
        if self.view.get() != View::Settings {
            self.preferences_return.set(self.view.get());
        }
        self.preferences_epoch.set(self.preferences_epoch.get() + 1);
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        self.result.replace(None);
        self.request.replace(None);
        self.url.replace(None);
        self.economy_result.replace(None);
        self.economy_panel.not_requested();
        self.preferences.show(&self.settings.borrow());
        self.view.set(View::Settings);
        self.title.set_text("Réglages");
        self.stack.set_visible_child_name("settings");
        self.refresh_delve();
        if !self.window.is_visible() {
            self.show();
        }
    }
    fn cancel_settings(&self) {
        if self.preferences_pending.get() {
            return;
        }
        self.preferences.images.cancel_import();
        let view = self.preferences_return.get();
        self.view.set(view);
        if view == View::Images {
            self.open_images();
            return;
        }
        if view == View::Stopwatches {
            self.open_stopwatches();
            return;
        }
        if view == View::Catalogue {
            self.title.set_text("Recherche des cibles de Heist");
            self.stack.set_visible_child_name("catalogue");
            self.refresh_delve();
            return;
        }
        let item = self.item.borrow();
        self.title.set_text(
            &item
                .as_ref()
                .map(Item::description)
                .unwrap_or_else(|| "AWAKENED POE TRADE · RUST".into()),
        );
        self.stack.set_visible_child_name(
            if item.as_ref().is_some_and(Item::needs_unique_identity) {
                "identify"
            } else if view == View::Inspection {
                "inspection"
            } else {
                "price"
            },
        );
        if view == View::Price && item.is_some() {
            self.text
                .set_text("Réglages annulés. Cliquez Rechercher pour consulter les prix.");
        }
    }
    fn save_settings(&self) {
        if self.view.get() != View::Settings
            || self.preferences_pending.get()
            || self.state.borrow().state != State::Interactive
        {
            return;
        }
        let mut candidate = match self.preferences.read() {
            Ok(s) => s,
            Err(e) => {
                self.preferences.error(&e);
                return;
            }
        };
        if let Ok(canvas) = &self.widget_canvas {
            candidate.widget_order =
                crate::widget_order::saved(&canvas.order(), &candidate.widget_keys());
        }
        self.widget_transaction.set(false);
        self.preferences_pending.set(true);
        self.preferences.busy(true);
        self.stack.set_sensitive(false);
        let request = crate::desktop::preferences::Request {
            epoch: self.preferences_epoch.get(),
            settings: candidate,
        };
        if let Err(e) = self.preferences_worker.try_send(request) {
            self.preferences_pending.set(false);
            self.stack.set_sensitive(true);
            self.preferences
                .error(&format!("Service de réglages indisponible : {e}"));
        }
    }
    fn preferences_update(&self, update: crate::desktop::preferences::Update) {
        use crate::desktop::preferences::Update;
        match update {
            Update::Backend { connection, result } => {
                if connection < self.backend_connection.get() {
                    return;
                }
                self.backend_connection.set(connection);
                if let Err(e) = &result {
                    eprintln!("shortcuts unavailable: {e}");
                    if self.view.get() == View::Settings && !self.preferences_pending.get() {
                        self.preferences.error(e);
                    }
                }
                let _ = self.backend_status.replace(result);
            }
            Update::Failed { epoch, error } => {
                self.preferences_pending.set(false);
                self.stack.set_sensitive(true);
                eprintln!("settings transaction rejected epoch={epoch} error={error}");
                if let Ok(c) = &self.widget_canvas {
                    let s = self.settings.borrow();
                    c.rollback(&s.widget_order, &s.widget_keys());
                }
                if self.order_transaction.replace(false) {
                    self.widget_transaction.set(false);
                    self.stopwatch_manager
                        .message(&format!("Modifications non enregistrées : {error}"));
                    self.image_manager
                        .message(&format!("Modifications non enregistrées : {error}"));
                    return;
                }
                if self.widget_transaction.replace(false) {
                    self.image_manager.busy(false);
                    self.image_manager
                        .message(&format!("Modifications non enregistrées : {error}"));
                    if let Ok(r) = &self.images {
                        r.set_busy(false);
                        r.reconcile(
                            &self.settings.borrow().image_strips,
                            self.settings.borrow().scale,
                        );
                    }
                    self.stopwatch_manager.busy(false);
                    self.stopwatch_manager
                        .message(&format!("Modifications non enregistrées : {error}"));
                    if let Ok(r) = &self.stopwatches {
                        r.set_busy(false);
                        r.reconcile(
                            &self.settings.borrow().stopwatches,
                            self.settings.borrow().scale,
                        );
                    }
                } else if epoch == self.preferences_epoch.get() {
                    self.preferences
                        .error(&format!("Modifications non enregistrées : {error}"));
                }
            }
            Update::Applied {
                epoch,
                settings,
                connection,
                available,
            } => {
                let settings = *settings;
                self.preferences_pending.set(false);
                self.stack.set_sensitive(true);
                self.backend_connection.set(connection);
                let _ = self.backend_status.replace(if available {
                    Ok(())
                } else {
                    Err("Réglages enregistrés, mais la connexion aux raccourcis est perdue.".into())
                });
                self.settings.replace(settings.clone());
                if let Ok(c) = &self.widget_canvas {
                    c.configure(&settings.widget_order, &settings.widget_keys());
                    c.ack(&settings.widget_order);
                }
                if self.order_transaction.replace(false) {
                    self.widget_transaction.set(false);
                    eprintln!("widgets order saved epoch={epoch} activation_requested=false");
                    return;
                }
                if let Ok(r) = &self.images {
                    r.set_busy(false);
                    r.reconcile(&settings.image_strips, settings.scale);
                }
                self.image_manager.configure(&settings.image_strips);
                self.image_manager.busy(false);
                if let Ok(r) = &self.stopwatches {
                    r.set_busy(false);
                    r.reconcile(&settings.stopwatches, settings.scale);
                }
                self.stopwatch_manager.configure(&settings.stopwatches);
                self.stopwatch_manager.busy(false);
                if self.widget_transaction.replace(false) {
                    self.stopwatch_manager.message("Widgets enregistrés.");
                    self.image_manager.message("Widgets enregistrés.");
                    self.update_stopwatch_manager();
                    self.refresh_delve();
                    return;
                }
                self.state.borrow_mut().generation += 1;
                self.sync_generation();
                self.result.replace(None);
                self.request.replace(None);
                self.url.replace(None);
                self.economy_result.replace(None);
                self.economy_panel.not_requested();
                self.updating_settings.set(true);
                self.league.set_text(&settings.league);
                self.online.set_active(settings.filters.online);
                self.search_range.set_value(settings.search_range);
                self.currency
                    .set_active_id(Some(&settings.filters.bulk_currency));
                self.updating_settings.set(false);
                if let Err(e) = set_style(&self.window, &self.css, settings.scale) {
                    eprintln!("settings style error: {e}");
                }
                eprintln!("settings committed epoch={epoch} scale={} shortcuts_available={available} automatic_http=false",settings.scale);
                let current_item = self.item.borrow().clone();
                let generation = self.state.borrow().generation;
                if self.view.get() == View::Settings && epoch == self.preferences_epoch.get() {
                    if self.window.is_visible() {
                        self.hide(HideReason::Button);
                    }
                } else if let Some(item) = current_item
                    .filter(|_| matches!(self.view.get(), View::Price | View::Inspection))
                {
                    self.prepare_item(item, self.view.get(), generation);
                    if self.view.get() == View::Price {
                        self.text.set_text(
                            "Réglages appliqués. Vérifiez les filtres puis cliquez Rechercher.",
                        );
                    }
                }
            }
        }
    }
    fn price_after_copy(&self) {
        if self.settings.borrow().automatic_search {
            self.search();
        } else {
            self.text
                .set_text("Objet copié. Vérifiez les filtres puis cliquez Rechercher.");
            self.economy_panel.not_requested();
            eprintln!("price automatic_search=false automatic_http=false");
        }
    }
    fn command(&self, command: &str) {
        if let Some(value) = command.strip_prefix("catalogue ") {
            let (kind, query) = value.split_once(' ').unwrap_or((value, ""));
            let kind = match kind {
                "replica" => crate::item::search::Kind::Replica,
                "gem" => crate::item::search::Kind::Gem,
                _ => {
                    eprintln!("catalogue requires gem or replica");
                    return;
                }
            };
            if query.chars().count() > 256 || self.preferences_pending.get() {
                eprintln!("catalogue query refused");
                return;
            }
            self.open_catalogue();
            self.catalogue.set_search(kind, query);
            return;
        }
        if let Some(league) = command.strip_prefix("league ") {
            if self.preferences_pending.get() {
                eprintln!("league update refused: settings transaction pending");
                return;
            }
            let mut settings = self.settings.borrow().clone();
            settings.league = league.trim().to_string();
            match settings.save() {
                Ok(()) => {
                    self.league.set_text(&settings.league);
                    self.settings.replace(settings);
                    if self.view.get() == View::Catalogue {
                        let generation = self.invalidate_prices();
                        self.catalogue
                            .show(&self.settings.borrow().league, generation);
                    }
                    eprintln!("league updated");
                }
                Err(e) => eprintln!("settings error={e}"),
            }
            return;
        }
        match command {
            "demo" => {
                self.view.set(View::Price);
                self.stack.set_visible_child_name("price");
                self.inspection_result.replace(None);
                self.item.replace(None);
                self.preset.hide();
                self.item_filters.container.hide();
                self.url.replace(None);
                self.economy_panel.reset();
                self.economy_panel.show(&Ok(Report::default()));
                self.title.set_text("AWAKENED POE TRADE · RUST");
                self.text.set_text("Panneau natif de validation\n\nClavier et clics restent au jeu.\nAucun prix simulé n'est affiché.");
                self.show();
            }
            "check" | "copy" | "inspect-check" | "inspect-copy" => {
                if !self.state.borrow().game_active {
                    eprintln!("capture refused: PoE is not active");
                    return;
                }
                self.hide(HideReason::Capture);
                let generation = self.state.borrow().generation;
                if let Err(e) = self.capture_worker.try_send(CaptureJob {
                    generation,
                    copy: command.ends_with("copy"),
                    view: if command.starts_with("inspect-") {
                        View::Inspection
                    } else {
                        View::Price
                    },
                }) {
                    eprintln!("capture worker busy: {e}");
                }
            }
            "interact" => {
                if self.state.borrow().state == State::Hidden && self.state.borrow().game_active {
                    if self
                        .stopwatches
                        .as_ref()
                        .is_ok_and(|r| r.visible_rendered())
                    {
                        self.state.borrow_mut().auxiliary_painted();
                    } else if self.stopwatches.as_ref().is_ok_and(|r| r.has_configs()) {
                        self.open_stopwatches();
                        if self.state.borrow().state == State::Preparing {
                            self.pending_interaction
                                .set(Some(self.state.borrow().generation));
                        }
                        return;
                    } else if self.images.as_ref().is_ok_and(|r| r.has_wanted()) {
                        self.open_images();
                        if self.state.borrow().state == State::Preparing {
                            self.pending_interaction
                                .set(Some(self.state.borrow().generation));
                        }
                        return;
                    }
                }
                if self.state.borrow_mut().interact() {
                    self.hide_delve(crate::delve::Reason::Interactive);
                    apply_input(&self.window, self.window.is_visible());
                    self.refresh_delve();
                    self.hint.set_text(
                        "Interactif · cliquez dans le panneau · Échap ou Fermer pour quitter",
                    );
                    eprintln!("overlay state=Interactive keyboard=OnDemand explicit=true");
                } else {
                    eprintln!("interact refused: first frame not ready or PoE inactive");
                }
            }
            "hide" => {
                self.hide_delve(crate::delve::Reason::Explicit);
                self.hide(HideReason::Explicit);
            }
            "delve" => self.toggle_delve(),
            "delve-hide" => self.hide_delve(crate::delve::Reason::Explicit),
            "settings" => self.open_settings(),
            "catalogue" => self.open_catalogue(),
            "stopwatches" => self.open_stopwatches(),
            "images" => self.open_images(),
            "evaluate" => self.change_view(View::Inspection),
            "prices" => self.change_view(View::Price),
            "quit" => {
                self.hide(HideReason::Quit);
                gtk::main_quit();
            }
            _ => eprintln!("unknown command: {command}"),
        }
    }
    fn change_view(&self, view: View) {
        let Some(item) = self.item.borrow().clone() else {
            return;
        };
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        self.request.replace(None);
        self.url.replace(None);
        self.view.set(view);
        if item.needs_unique_identity() {
            self.stack.set_visible_child_name("identify");
            if !self.window.is_visible() {
                self.show();
            }
            return;
        }
        if view == View::Inspection {
            let report = crate::inspection::report(&item, &self.settings.borrow().map_check);
            self.inspection
                .show(report.clone(), &self.settings.borrow().map_check);
            self.inspection_result.replace(Some(report));
            self.stack.set_visible_child_name("inspection");
        } else {
            self.inspection_result.replace(None);
            self.stack.set_visible_child_name("price");
        }
        self.refresh_delve();
        if !self.window.is_visible() && !self.show() {
            return;
        }
        if view == View::Price {
            self.search();
        }
    }
    fn prepare_item(&self, mut item: Item, view: View, epoch: u64) {
        self.identity_epoch.set(epoch);
        let choices = item.unique_identity_choices();
        if choices.len() == 1 {
            if let Ok(chosen) = item.with_unique_identity(&choices[0].reference) {
                eprintln!(
                    "unique identity automatic=true reference={:?} unidentified=true",
                    chosen.reference
                );
                item = chosen;
            }
        }
        self.result.replace(None);
        self.request.replace(None);
        self.url.replace(None);
        self.economy_result.replace(None);
        self.economy_panel.reset();
        self.inspection_result.replace(None);
        if item.needs_unique_identity() {
            self.title.set_text(&item.description());
            self.view.set(view);
            let sender = self.events.clone();
            self.unidentified
                .show(&item, epoch, move |epoch, reference| {
                    let _ = sender.send(Event::IdentifyUnique(epoch, reference));
                });
            self.stack.set_visible_child_name("identify");
            self.stat_rows.borrow_mut().clear();
            self.mercenary_panel.replace(None);
            for child in self.mod_box.children() {
                self.mod_box.remove(&child);
            }
            self.item.replace(Some(item));
            return;
        }
        self.title.set_text(&item.description());
        self.currency.set_sensitive(item.trade_tag.is_some());
        if item.trade_tag.as_deref() == self.currency.active_id().as_deref() {
            self.currency
                .set_active_id(Some(if item.trade_tag.as_deref() == Some("chaos") {
                    "divine"
                } else {
                    "chaos"
                }));
        }
        self.updating_preset.set(true);
        self.preset.remove_all();
        let modes = crate::presets::available(&item);
        if modes.is_empty() {
            self.preset.append(Some("calculated"), "Filtres calculés");
        } else {
            for mode in &modes {
                self.preset.append(Some(mode.id()), mode.label());
            }
        }
        self.preset.set_sensitive(modes.len() > 1);
        self.preset.set_active(Some(0));
        self.updating_preset.set(false);
        self.build_stat_rows(&item);
        self.reset_item_filters(&item);
        self.item.replace(Some(item));
        self.view.set(view);
        if view == View::Inspection {
            let report = crate::inspection::report(
                self.item.borrow().as_ref().unwrap(),
                &self.settings.borrow().map_check,
            );
            eprintln!(
                "inspection prepared map_like={} stats={} unknown={} offline=true",
                report.map_like,
                report.stats.len(),
                report.unknown.len()
            );
            self.inspection
                .show(report.clone(), &self.settings.borrow().map_check);
            self.inspection_result.replace(Some(report));
            self.stack.set_visible_child_name("inspection");
        } else {
            self.inspection_result.replace(None);
            self.stack.set_visible_child_name("price");
        }
    }
    fn identify_unique(&self, epoch: u64, reference: &str) {
        if epoch != self.identity_epoch.get()
            || !self.window.is_visible()
            || self.state.borrow().state != State::Interactive
        {
            eprintln!("discarded unique identity epoch={epoch}");
            return;
        }
        let Some(item) = self.item.borrow().clone() else {
            return;
        };
        match item.with_unique_identity(reference) {
            Ok(chosen) => {
                self.state.borrow_mut().generation += 1;
                self.sync_generation();
                let generation = self.state.borrow().generation;
                eprintln!("unique identity automatic=false reference={reference:?} generation={generation} unidentified=true");
                self.prepare_item(chosen, self.view.get(), generation);
                if self.view.get() == View::Price {
                    self.price_after_copy();
                }
            }
            Err(error) => eprintln!("unique identity rejected: {error}"),
        }
    }
    fn build_stat_rows(&self, item: &Item) {
        for child in self.mod_box.children() {
            self.mod_box.remove(&child);
        }
        self.stat_rows.borrow_mut().clear();
        self.mercenary_panel.replace(None);
        if let Some(parsed) = &item.mercenary {
            match parsed.preset() {
                Ok(preset) => {
                    let sender = self.events.clone();
                    let current = self.current.clone();
                    let panel = mercenary_panel::Panel::new(preset, move || {
                        let _ = sender.send(Event::FiltersChanged(
                            current.load(Ordering::Acquire),
                            "mercenary".into(),
                        ));
                    });
                    self.mod_expander.set_label(Some(&format!(
                        "Mercenaire · {} choix cochés",
                        panel.active_count()
                    )));
                    self.mod_box.pack_start(&panel.container, false, false, 0);
                    self.mercenary_panel.replace(Some(panel));
                }
                Err(error) => {
                    self.mod_box
                        .pack_start(&gtk::Label::new(Some(&error)), false, false, 0)
                }
            }
            self.mod_scroll.vadjustment().set_value(0.);
            self.mod_scroll.set_min_content_height(300);
            self.mod_box.show_all();
            self.preset.show();
            return;
        }
        let mode = crate::presets::available(item)
            .into_iter()
            .find(|m| Some(m.id()) == self.preset.active_id().as_deref());
        let mut candidates = if let Some(mode) = mode {
            crate::presets::create(item, self.search_range.value(), mode).filters
        } else {
            crate::calculated::candidates(item, self.search_range.value())
        };
        if mode == Some(crate::presets::Mode::Calculated) {
            candidates.extend(crate::calculated::exact_sources(
                item,
                &candidates,
                self.search_range.value(),
            ));
        }
        self.preset.show();
        if candidates.iter().any(|c| c.group.is_some()) {
            let label = gtk::Label::new(Some(
                "Timeless : au moins une des options cochées ci-dessous doit correspondre.",
            ));
            label.set_line_wrap(true);
            label.set_xalign(0.);
            self.mod_box.pack_start(&label, false, false, 0);
        }
        let advanced = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let expander = gtk::Expander::new(Some("Autres filtres et sources exactes"));
        expander.add(&advanced);
        for candidate in &candidates {
            let row = stat_row::StatRow::new(candidate);
            let sender = self.events.clone();
            let current = self.current.clone();
            let origin = format!("stat:{}", candidate.modifier.id);
            row.connect_changed(move || {
                let _ = sender.send(Event::FiltersChanged(
                    current.load(Ordering::Acquire),
                    origin.clone(),
                ));
            });
            if candidate.hidden.is_some() {
                advanced.pack_start(&row.container, false, false, 0);
            } else {
                self.mod_box.pack_start(&row.container, false, false, 0);
            }
            self.stat_rows.borrow_mut().push(row);
        }
        if !advanced.children().is_empty() {
            self.mod_box.pack_start(&expander, false, false, 0);
        }
        if candidates.is_empty() {
            self.mod_box.pack_start(
                &gtk::Label::new(Some("Aucun filtre de mod pour ce preset.")),
                false,
                false,
                0,
            );
        }
        self.mod_scroll.vadjustment().set_value(0.);
        self.mod_scroll
            .set_min_content_height(if candidates.len() > 2 { 300 } else { 180 });
        self.mod_expander.set_label(Some(&format!(
            "Filtres · {} actifs",
            candidates.iter().filter(|c| !c.disabled).count()
        )));
        self.mod_box.show_all();
    }
    fn reset_item_filters(&self, item: &Item) {
        let settings = self.settings.borrow();
        let mut current = settings.filters.clone();
        current.online = self.online.is_active();
        let mode = crate::presets::available(item)
            .into_iter()
            .find(|m| Some(m.id()) == self.preset.active_id().as_deref());
        let result = filters::plan_for_preset(
            item,
            &current,
            self.league.text().trim(),
            mode.is_none_or(|m| m.exact()),
        );
        match result {
            Ok(p) => self.item_filters.show(
                p,
                !(item.market.exchangeable && item.trade_tag.is_none()),
                item.trade_tag.is_some()
                    && self.stat_rows.borrow().is_empty()
                    && settings.filters.stats.is_empty(),
            ),
            Err(error) => self.item_filters.clear(&error),
        }
        self.item_filters.container.show();
    }
    fn filters_changed(&self, observed_generation: u64, origin: &str) {
        if self.updating_settings.get() {
            return;
        }
        eprintln!("filters change origin={origin:?} observed_generation={observed_generation} current_generation={} state={:?} elapsed_ms={}",self.state.borrow().generation,self.state.borrow().state,self.started.elapsed().as_millis());
        if self.view.get() != View::Price || !self.window.is_visible() {
            return;
        }
        if !self.state.borrow_mut().edited(observed_generation) {
            return;
        }
        self.sync_generation();
        self.request.replace(None);
        self.url.replace(None);
        if origin == "league" {
            self.economy_result.replace(None);
            self.economy_panel.league_changed();
        }
        self.mod_expander.set_label(Some(&format!(
            "Filtres · {} actifs",
            self.stat_rows
                .borrow()
                .iter()
                .filter(|r| r.selected_filter().is_some())
                .count()
                + self.settings.borrow().filters.stats.len()
        )));
        if let Some(panel) = self.mercenary_panel.borrow().as_ref() {
            self.mod_expander.set_label(Some(&format!(
                "Mercenaire · {} choix cochés",
                panel.active_count()
            )));
        }
        self.text
            .set_text("Filtres modifiés. Vérifiez la portée puis cliquez Rechercher.");
        eprintln!(
            "filters edited generation={} automatic_http=false",
            self.state.borrow().generation
        );
    }
    fn preset_changed(&self) {
        if self.updating_preset.get() {
            return;
        }
        let Some(item) = self.item.borrow().clone() else {
            return;
        };
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        self.request.replace(None);
        self.url.replace(None);
        self.build_stat_rows(&item);
        self.reset_item_filters(&item);
        self.text
            .set_text("Preset modifié. Vérifiez les filtres puis cliquez Rechercher.");
        eprintln!(
            "preset changed id={:?} generation={}",
            self.preset.active_id(),
            self.state.borrow().generation
        );
    }
    fn search(&self) {
        if self.view.get() != View::Price || self.preferences_pending.get() {
            return;
        }
        let Some(item) = self.item.borrow().clone() else {
            return;
        };
        if item.needs_unique_identity() {
            eprintln!("search refused: unresolved unique identity; automatic_http=false");
            return;
        }
        // Even a locally rejected edit supersedes pending results. Otherwise an
        // older response could replace the validation error with stale prices.
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        let generation = self.state.borrow().generation;
        self.url.replace(None);
        self.request.replace(None);
        self.economy_panel.reset();
        self.economy_panel.show(&Ok(Report::default()));
        let mut settings = self.settings.borrow().clone();
        settings.league = self.league.text().trim().to_string();
        settings.filters.online = self.online.is_active();
        settings.search_range = self.search_range.value();
        let mut plan = match self.item_filters.read() {
            Ok(p) => p,
            Err(e) => {
                self.text.set_text(&e);
                return;
            }
        };
        plan.trade.offline = !self.online.is_active();
        plan.trade.league = settings.league.clone();
        if let Some(value) = self.item_filters.merchant_preference() {
            settings.filters.merchant_only = value;
        }
        settings.filters.collapse_listings = plan.trade.collapse_listings == "api";
        settings.filters.bulk_currency = self
            .currency
            .active_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "chaos".into());

        if let Err(e) = settings.save() {
            self.text.set_text(&e);
            return;
        }
        self.settings.replace(settings.clone());
        let mut selected = settings.filters.clone();
        selected.item = Some(plan);
        selected.stats.extend(
            self.stat_rows
                .borrow()
                .iter()
                .filter_map(stat_row::StatRow::selected_filter),
        );
        self.mod_expander
            .set_label(Some(&format!("Filtres · {} actifs", selected.stats.len())));
        selected.mercenary = self
            .mercenary_panel
            .borrow()
            .as_ref()
            .map(mercenary_panel::Panel::read);
        if let Some(panel) = self.mercenary_panel.borrow().as_ref() {
            self.mod_expander.set_label(Some(&format!(
                "Mercenaire · {} choix cochés",
                panel.active_count()
            )));
        }
        let query = match filters::search_query(&item, &selected) {
            Ok(query) => query,
            Err(e) => {
                self.text.set_text(&e);
                return;
            }
        };
        self.url
            .replace(crate::trade::query_link(&settings.league, &query).ok());
        self.request.replace(Some((generation, query.clone())));
        self.economy_panel.reset();
        if self
            .economy_worker
            .try_send(EconomyJob::Price {
                generation,
                league: settings.league.clone(),
                item: Box::new(item.clone()),
            })
            .is_err()
        {
            self.economy_panel.show(&Err(
                "une requête indicative est déjà en cours ; réessayez".into()
            ));
        }
        self.text.set_text(&format!("Recherche dans {}…\n\nComparaison selon la portée et les filtres sélectionnés.\nVérifiez les filtres actifs dans le panneau.",settings.league));
        match self.worker.try_send(Job {
            generation,
            league: settings.league.clone(),
            query,
        }) {
            Ok(()) => eprintln!(
                "trade search generation={generation} league={:?} base={:?}",
                settings.league, item.base
            ),
            Err(e) => self
                .text
                .set_text(&format!("Une recherche est en cours. Réessayez : {e}")),
        }
    }
    fn event(self: &Rc<Self>, event: Event) {
        match event {
            Event::WidgetOrder => self.save_widget_order(),
            Event::Shortcut(command) => self.command(command),
            Event::Hotkey(crate::hotkeys::Action::Command(command)) => self.command(command),
            Event::Hotkey(crate::hotkeys::Action::StopwatchToggle(id)) => {
                self.stopwatch_time(id, false)
            }
            Event::Hotkey(crate::hotkeys::Action::StopwatchReset(id)) => {
                self.stopwatch_time(id, true)
            }
            Event::Stopwatch(action) => self.stopwatch_action(action),
            Event::StopwatchManager(action) => self.stopwatch_manager_action(action),
            Event::Image(action) => self.image_action(action),
            Event::ImageManager(action) => self.image_manager_action(action),
            Event::Catalogue(action) => self.catalogue_action(action),
            Event::CataloguePrices(generation, result) => {
                if self.view.get() == View::Catalogue && self.state.borrow().accepts(generation) {
                    self.catalogue.show_quotes(*result);
                } else {
                    eprintln!("discarded catalogue economy generation={generation}");
                }
            }
            Event::Preferences(update) => {
                self.preferences_update(*update);
                self.save_widget_order();
            }
            Event::IdentifyUnique(epoch, reference) => self.identify_unique(epoch, &reference),
            Event::Windows(s) => {
                let active = s.game_active();
                if self.state.borrow().game_active != active {
                    eprintln!(
                        "game active={active} present={} other_active={}",
                        s.game_present(),
                        s.other_active()
                    );
                }
                let reason = self.state.borrow_mut().desktop_changed(DesktopState {
                    game_exists: s.game_exists(),
                    game_visible: s.game_present(),
                    game_active: active,
                    other_active: s.other_active(),
                });
                if s.other_active() || !s.game_present() {
                    self.pending_stopwatch_edit.set(None);
                    self.pending_image_edit.set(None);
                    self.pending_interaction.set(None);
                }
                self.game_active.store(active, Ordering::Release);
                self.desktop.replace(s);
                self.refresh_delve();
                if let Some(reason) = reason {
                    self.hide(reason);
                }
                if active {
                    if let Some((token, id)) = self.pending_image_edit.get() {
                        if token == self.state.borrow().generation {
                            self.pending_image_edit.set(None);
                            self.open_settings();
                            self.preferences.open_images(Some(id));
                        }
                    }
                }
                if active
                    && self.pending_stopwatch_edit.get() == Some(self.state.borrow().generation)
                {
                    self.pending_stopwatch_edit.set(None);
                    self.open_settings();
                    self.preferences.open_stopwatches();
                }
            }
            Event::TrackerError(e) => {
                self.game_active.store(false, Ordering::Release);
                self.state.borrow_mut().game_active = false;
                self.hide(HideReason::TrackerFailure);
                self.state.borrow_mut().fail();
                eprintln!("tracker error={e}");
            }
            Event::Captured(generation, view, result) => {
                if generation != self.state.borrow().generation || !self.state.borrow().game_active
                {
                    eprintln!("discarded capture generation={generation}");
                    return;
                }
                match *result {
                    Ok((item, elapsed)) => {
                        let filters_started = Instant::now();
                        eprintln!(
                            "capture parsed processing_ms={elapsed} clipboard_before_overlay=true"
                        );
                        self.prepare_item(item, view, generation);
                        eprintln!(
                            "filters prepared count={} ui_ms={}",
                            self.stat_rows.borrow().len(),
                            filters_started.elapsed().as_millis()
                        );
                        if self.show() && view == View::Price {
                            self.price_after_copy();
                        }
                    }
                    Err(e) => {
                        self.view.set(View::Price);
                        self.stack.set_visible_child_name("price");
                        self.inspection_result.replace(None);
                        self.item.replace(None);
                        self.preset.hide();
                        self.item_filters.container.hide();
                        self.economy_panel.reset();
                        self.economy_panel.show(&Ok(Report::default()));
                        self.title.set_text("Impossible de lire l'objet");
                        self.text.set_text(&e);
                        eprintln!("capture error={e}");
                        self.show();
                    }
                }
            }
            Event::FiltersChanged(generation, origin) => self.filters_changed(generation, &origin),
            Event::MapSettings(observed, edit) => {
                if self.view.get() != View::Inspection
                    || self.preferences_pending.get()
                    || observed != self.state.borrow().generation
                {
                    eprintln!("discarded map settings callback");
                    return;
                }
                let mut settings = self.settings.borrow().clone();
                let old = settings.map_check.clone();
                if let Err(e) = settings.map_check.edit(edit) {
                    self.inspection.error(&e);
                    return;
                }
                if old == settings.map_check {
                    return;
                }
                if let Err(e) = settings.save() {
                    self.inspection.error(&e);
                    return;
                }
                self.settings.replace(settings);
                if let Some(item) = self.item.borrow().as_ref() {
                    let report = crate::inspection::report(item, &self.settings.borrow().map_check);
                    self.inspection
                        .show(report.clone(), &self.settings.borrow().map_check);
                    self.inspection_result.replace(Some(report));
                }
            }
            Event::Prices(generation, result) => {
                if !self.state.borrow().accepts(generation) {
                    eprintln!("discarded network generation={generation}");
                    return;
                }
                self.result.replace(Some((generation, result.clone())));
                match result {
                    Ok(p) => {
                        let rows = p
                            .listings
                            .iter()
                            .map(|l| match &l.bulk {
                                Some(b) => format!(
                                    "{} {}/objet · {} pour {} · stock {} · {}",
                                    economy_panel::format_amount(l.amount),
                                    l.currency,
                                    b.exchange_amount,
                                    b.item_amount,
                                    b.stock,
                                    l.seller
                                ),
                                None => format!("{:>8} {:<16} {}", l.amount, l.currency, l.seller),
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        self.text.set_text(&format!("{} · {} résultats · {} offres\n{}\n\n{}\n\n{}",if p.kind==SearchKind::Bulk {"Échange en gros"} else {"Recherche d'objet"},p.total,p.listings.len(),if p.cached {"Cache (< 60 s)".into()} else {format!("Réseau : {} ms",p.network_ms)},if rows.is_empty() {"Aucune offre avec prix."} else {&rows},if p.kind==SearchKind::Bulk {"Prix par objet, lot et stock conservés. Vérifiez la devise choisie."} else {"Comparaison selon la portée et les filtres sélectionnés. Vérifiez les mods avant d'estimer un rare."}));
                        self.url.replace(Some(p.url));
                        eprintln!(
                            "trade result generation={generation} total={} network_ms={} cached={}",
                            p.total, p.network_ms, p.cached
                        );
                    }
                    Err(e) => {
                        if e.contains("Query is too complex") {
                            self.text.set_text(&format!("{e}\n\nRéduisez les critères, ou utilisez Ouvrir Trade pour reprendre cette requête dans votre navigateur."));
                        } else {
                            self.text.set_text(&e);
                        }
                        eprintln!("trade error generation={generation} error={e}");
                    }
                }
            }
            Event::Economy(generation, result) => {
                let result = *result;
                if !self.state.borrow().accepts(generation) {
                    eprintln!("discarded economy generation={generation}");
                    return;
                }
                if let Err(e) = &result {
                    eprintln!("economy error generation={generation} error={e}");
                }
                if let Some(q) = result.as_ref().ok().and_then(|r| r.quote.as_ref()) {
                    eprintln!(
                        "economy result generation={generation} chaos={} age_s={} cached={}",
                        q.chaos, q.age_seconds, q.from_cache
                    );
                }
                self.economy_panel.show(&result);
                self.economy_result.replace(Some((generation, result)));
            }
        }
    }
}
#[allow(deprecated)] // GTK3's matching glib 0.18 channel; wakes the main loop without polling.
pub fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|a| a == "inspect" || a == "lookup" || a == "economy" || a == "analyze")
    {
        let path = args
            .get(1)
            .ok_or("Usage: inspect FILE or lookup FILE [LEAGUE] [PAYMENT_CURRENCY]")?;
        let inspect_started = Instant::now();
        let item = Item::parse(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?;
        if args[0] == "analyze" {
            let config = Settings::load()?.map_check;
            println!(
                "{}",
                serde_json::to_string(&crate::inspection::report(&item, &config))
                    .map_err(|e| e.to_string())?
            );
            return Ok(());
        }
        if args[0] == "economy" {
            let league = args.get(2).cloned().unwrap_or(Settings::load()?.league);
            let quote = Economy::new()?.report(&league, &item, 1, &Arc::new(AtomicU64::new(1)));
            println!(
                "{}",
                serde_json::json!({"league":league,"item":item,"economy":quote})
            );
            return Ok(());
        }
        let profile = Settings::load()?;
        let league = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| profile.league.clone());
        let mut filters = filters::Filters {
            stats: vec![],
            ..profile.filters.clone()
        };
        let plan = filters::plan_for_league(&item, &filters, &league)?;
        filters.item = Some(plan);
        if let Some(currency) = args.get(3) {
            filters.bulk_currency = currency.clone();
        } else if item.trade_tag.as_deref() == Some("chaos") {
            filters.bulk_currency = "divine".into();
        }
        let query = filters::search_query(&item, &filters)?;
        if args[0] == "inspect" {
            let range = Settings::load()?.search_range;
            let modes = crate::presets::available(&item);
            let candidate_filters = if let Some(mode) = modes.first() {
                crate::presets::create(&item, range, *mode).filters
            } else {
                crate::calculated::candidates(&item, range)
            };
            let suggested = filters::Filters {
                stats: candidate_filters
                    .iter()
                    .filter(|c| !c.disabled)
                    .map(filters::StatFilter::from_candidate)
                    .collect(),
                ..filters.clone()
            };
            eprintln!(
                "inspect processing_us={}",
                inspect_started.elapsed().as_micros()
            );
            println!(
                "{}",
                serde_json::json!({"mercenary_preset":item.mercenary.as_ref().map(|m|m.preset()),"item":item,"query":query,"candidate_filters":candidate_filters,"item_filters":filters.item,"presets":modes.iter().map(|m|crate::presets::create(&item,range,*m)).collect::<Vec<_>>(),"suggested_query":filters::search_query(&item,&suggested)})
            );
            return Ok(());
        }
        let current = Arc::new(AtomicU64::new(1));
        let prices = Trade::new()?.search(&league, &query, 1, &current);
        let economy = Economy::new()?.quote(&league, &item, 1, &current);
        println!(
            "{}",
            serde_json::json!({"league":league,"item":item,"request":query,"trade":prices,"economy":economy})
        );
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--version") {
        println!("awakened-poe-trade-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--help" || a == "help") {
        println!("awakened-poe-trade-rs [daemon|probe|demo|settings|catalogue [gem|replica [QUERY]]|delve|delve-hide|stopwatches|images|check|copy|interact|hide|quit|status|league NAME|windows|inspect FILE|analyze FILE|inspect-copy|inspect-check|evaluate|prices|lookup FILE [LEAGUE] [CURRENCY]|economy FILE [LEAGUE]]\nStart daemon, copy an item with Ctrl+Alt+C in PoE, then check.\ncopy sends the copy gesture to the active XWayland PoE only.\ninteract enables pointer controls after first render. No automatic focus requests.\nSettings: {}",crate::settings::path().display());
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "status") {
        let client_path = socket_path()?.with_extension(format!("client-{}", std::process::id()));
        let client = UnixDatagram::bind(&client_path).map_err(|e| e.to_string())?;
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let result = (|| {
            client
                .send_to(b"status", socket_path()?)
                .map_err(|e| e.to_string())?;
            let mut buf = [0; 65536];
            let n = client.recv(&mut buf).map_err(|e| e.to_string())?;
            println!("{}", String::from_utf8_lossy(&buf[..n]));
            Ok(())
        })();
        let _ = std::fs::remove_file(client_path);
        return result;
    }
    if let Some(command) = args.first().filter(|s| {
        matches!(
            s.as_str(),
            "demo"
                | "check"
                | "copy"
                | "inspect-check"
                | "inspect-copy"
                | "evaluate"
                | "prices"
                | "hide"
                | "interact"
                | "quit"
                | "league"
                | "settings"
                | "catalogue"
                | "delve"
                | "delve-hide"
                | "stopwatches"
                | "images"
        )
    }) {
        UnixDatagram::unbound()
            .map_err(|e| e.to_string())?
            .send_to(
                if command == "league" || command == "catalogue" {
                    args.join(" ")
                } else {
                    command.to_string()
                }
                .as_bytes(),
                socket_path()?,
            )
            .map_err(|e| format!("start the daemon first: {e}"))?;
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "windows") {
        return wayland::track(|s| eprintln!("{s:?}"));
    }
    if args.first().is_some_and(|a| a != "daemon" && a != "probe") {
        return Err("Unknown command; see --help".into());
    }
    let settings = Settings::load()?;
    gtk::init().map_err(|e| e.to_string())?;
    if !gtk_layer_shell::is_supported() {
        return Err(
            "Native Wayland layer-shell unavailable; no compatible overlay backend on this session"
                .into(),
        );
    }
    if gtk_layer_shell::protocol_version() < 4 {
        return Err("Layer-shell v4 is required for nonexclusive OnDemand interaction".into());
    }
    eprintln!(
        "backend=wayland-layer-shell protocol={} desktop={:?} league={:?}",
        gtk_layer_shell::protocol_version(),
        std::env::var("XDG_CURRENT_DESKTOP"),
        settings.league
    );
    let path = socket_path()?;
    if path.exists() {
        if UnixDatagram::unbound()
            .map_err(|e| e.to_string())?
            .connect(&path)
            .is_ok()
        {
            return Err("daemon already running".into());
        }
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    let socket = UnixDatagram::bind(&path).map_err(|e| e.to_string())?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Awakened PoE Trade — Rust");
    window.set_widget_name("apt-native");
    window.set_size_request(
        (620.0 * settings.scale) as i32,
        (420.0 * settings.scale) as i32,
    );
    if let Some(visual) = GtkWindowExt::screen(&window).and_then(|s| s.rgba_visual()) {
        window.set_visual(Some(&visual));
    }
    window.init_layer_shell();
    window.set_namespace("awakened-poe-trade-rs");
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_layer_shell_margin(Edge::Top, 100);
    window.set_layer_shell_margin(Edge::Left, 50);
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::None);
    let delve = delve_grid::Grid::new();
    let css = gtk::CssProvider::new();
    set_style(&window, &css, settings.scale)?;
    gtk::StyleContext::add_provider_for_screen(
        &GtkWindowExt::screen(&window).unwrap(),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
    let (sender, receiver) = glib::MainContext::channel::<Event>(glib::Priority::DEFAULT);
    let order_sender = sender.clone();
    let close_sender = sender.clone();
    let widget_canvas = widget_canvas::Canvas::new(
        move || {
            let _ = order_sender.send(Event::WidgetOrder);
        },
        move || {
            let _ = close_sender.send(Event::Shortcut("hide"));
        },
    );
    if let Ok(c) = &widget_canvas {
        c.configure(&settings.widget_order, &settings.widget_keys());
    }
    let stopwatch_sender = sender.clone();
    let stopwatches = widget_canvas
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|canvas| {
            stopwatch_panel::Runtime::new(canvas.clone(), move |action| {
                let _ = stopwatch_sender.send(Event::Stopwatch(action));
            })
        });
    if let Ok(r) = &stopwatches {
        r.reconcile(&settings.stopwatches, settings.scale);
    }
    let manager_sender = sender.clone();
    let stopwatch_manager = stopwatch_manager::Manager::new(move |action| {
        let _ = manager_sender.send(Event::StopwatchManager(action));
    });
    stopwatch_manager.configure(&settings.stopwatches);
    let image_sender = sender.clone();
    let images = widget_canvas
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|canvas| {
            image_panel::Runtime::new(canvas.clone(), move |action| {
                let _ = image_sender.send(Event::Image(action));
            })
        });
    if let Ok(r) = &images {
        r.reconcile(&settings.image_strips, settings.scale);
    }
    let manager_sender = sender.clone();
    let image_manager = image_manager::Panel::new(move |action| {
        let _ = manager_sender.send(Event::ImageManager(action));
    });
    image_manager.configure(&settings.image_strips);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_border_width(20);
    let title = gtk::Label::new(Some("AWAKENED POE TRADE · RUST"));
    title.set_xalign(0.);
    title.set_line_wrap(true);
    let text = gtk::Label::new(None);
    text.set_xalign(0.);
    text.set_yalign(0.);
    text.set_line_wrap(true);
    text.set_selectable(false);
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scroll.set_min_content_height(200);
    scroll.add(&text);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let league = gtk::Entry::new();
    league.set_text(&settings.league);
    league.set_width_chars(18);
    league.set_placeholder_text(Some("Ligue"));
    let online = gtk::CheckButton::with_label("En ligne");
    online.set_active(settings.filters.online);
    row.pack_start(&gtk::Label::new(Some("Ligue")), false, false, 0);
    row.pack_start(&league, true, true, 0);
    row.pack_start(&online, false, false, 0);
    let range_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search_range = gtk::SpinButton::with_range(0., 100., 1.);
    search_range.set_value(settings.search_range);
    search_range.set_tooltip_text(Some("Tolérance des nouvelles bornes suggérées ; 0 = valeur exacte. Une nouvelle copie applique la plage choisie."));
    range_row.pack_start(&gtk::Label::new(Some("Plage suggérée ±")), false, false, 0);
    range_row.pack_start(&search_range, false, false, 0);
    range_row.pack_start(
        &gtk::Label::new(Some("% · prochaine copie / preset")),
        false,
        false,
        0,
    );
    let mod_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let bulk_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let currency = gtk::ComboBoxText::new();
    for &(id, label) in filters::PAYMENT_CURRENCIES {
        currency.append(Some(id), label);
    }
    currency.set_active_id(Some(&settings.filters.bulk_currency));
    currency.set_sensitive(false);
    bulk_row.pack_start(
        &gtk::Label::new(Some("Échange : paiement")),
        false,
        false,
        0,
    );
    bulk_row.pack_start(&currency, true, true, 0);
    let economy_panel = economy_panel::EconomyPanel::new();
    let mod_scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    mod_scroll.set_min_content_height(180);
    mod_scroll.set_max_content_height(300);
    mod_scroll.set_propagate_natural_height(true);
    mod_scroll.add(&mod_box);
    let mod_expander =
        gtk::Expander::new(Some("Mods reconnus — cochez un filtre, puis Rechercher"));
    mod_expander.add(&mod_scroll);
    let preset = gtk::ComboBoxText::new();
    preset.set_no_show_all(true);
    let current = Arc::new(AtomicU64::new(0));
    let edit_generation = current.clone();
    let item_sender = sender.clone();
    let item_filters = item_filters_panel::Panel::new(move || {
        let _ = item_sender.send(Event::FiltersChanged(
            edit_generation.load(Ordering::Acquire),
            "item".into(),
        ));
    });
    item_filters.container.set_no_show_all(true);
    preset.set_tooltip_text(Some(
        "Sélection des filtres selon le type d’objet ; un changement exige Rechercher.",
    ));
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::Button::with_label("Rechercher");
    let browser = gtk::Button::with_label("Ouvrir Trade");
    let close = gtk::Button::with_label("Fermer");
    let evaluate = gtk::Button::with_label("Évaluer");
    for b in [&search, &browser, &evaluate, &close] {
        actions.pack_start(b, true, true, 0);
    }
    let hint = gtk::Label::new(None);
    hint.set_line_wrap(true);
    hint.set_xalign(0.);
    let unidentified = unidentified_panel::Panel::new();
    let close_sender = sender.clone();
    unidentified.close.connect_clicked(move |_| {
        let _ = close_sender.send(Event::Shortcut("hide"));
    });
    let preferences = settings_panel::Panel::new();
    let catalogue_sender = sender.clone();
    let catalogue = item_search_panel::Panel::new(move |action| {
        let _ = catalogue_sender.send(Event::Catalogue(action));
    });
    let stack = gtk::Stack::new();
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    let price_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    price_content.pack_start(&scroll, true, true, 0);
    price_content.pack_start(&row, false, false, 0);
    price_content.pack_start(&range_row, false, false, 0);
    price_content.pack_start(&bulk_row, false, false, 0);
    price_content.pack_start(&economy_panel.container, false, false, 0);
    price_content.pack_start(&preset, false, false, 0);
    price_content.pack_start(&item_filters.container, false, false, 0);
    price_content.pack_start(&mod_expander, false, false, 0);
    price_content.pack_start(&actions, false, false, 0);
    let inspection_sender = sender.clone();
    let inspection_generation = current.clone();
    let inspection = inspection_panel::Panel::new(move |edit| {
        let _ = inspection_sender.send(Event::MapSettings(
            inspection_generation.load(Ordering::Acquire),
            edit,
        ));
    });
    stack.add_named(&image_manager.container, "images");
    image_manager.container.show_all();
    stack.add_named(&stopwatch_manager.container, "stopwatches");
    stopwatch_manager.container.show_all();
    stack.add_named(&catalogue.container, "catalogue");
    catalogue.container.show_all();
    stack.add_named(&preferences.container, "settings");
    preferences.container.show_all();
    stack.add_named(&unidentified.container, "identify");
    unidentified.container.show_all();
    stack.add_named(&price_content, "price");
    stack.add_named(&inspection.container, "inspection");
    // GtkStack ignores requests for an invisible child. Make both pages
    // available before selecting the first captured item's view.
    price_content.show_all();
    inspection.container.show_all();
    stack.set_visible_child_name("price");
    let header = gtk::Box::new(gtk::Orientation::Vertical, 6);
    header.pack_start(&title, false, false, 0);
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let image_open = gtk::Button::with_label("Images");
    image_open.set_sensitive(images.is_ok());
    let preferences_open = gtk::Button::with_label("Réglages");
    let catalogue_open = gtk::Button::with_label("Cibles Heist");
    let stopwatch_open = gtk::Button::with_label("Chronos");
    stopwatch_open.set_sensitive(stopwatches.is_ok());
    let delve_open = gtk::Button::with_label("Grille Delve");
    delve_open.set_sensitive(delve.is_ok());
    delve_open.set_tooltip_text(Some(
        "Ferme ce panneau et affiche une grille passive ; configurez son raccourci dans Réglages.",
    ));
    heading.pack_start(&image_open, false, false, 0);
    heading.pack_start(&stopwatch_open, false, false, 0);
    heading.pack_start(&delve_open, false, false, 0);
    heading.pack_start(&catalogue_open, false, false, 0);
    heading.pack_start(&preferences_open, false, false, 0);
    header.pack_start(&heading, false, false, 0);
    content.pack_start(&header, false, false, 0);
    content.pack_start(&stack, true, true, 0);
    content.pack_start(&hint, false, false, 0);
    window.add(&content);

    let game_active = Arc::new(AtomicBool::new(false));

    let (capture_worker, captures) = mpsc::sync_channel::<CaptureJob>(1);
    {
        let sender = sender.clone();
        let current = current.clone();
        let active = game_active.clone();
        std::thread::spawn(move || {
            while let Ok(job) = captures.recv() {
                if current.load(Ordering::Acquire) != job.generation {
                    continue;
                }
                let started = Instant::now();
                let result = (if job.copy {
                    clipboard::copy_from_game(&active)
                } else {
                    clipboard::read()
                })
                .and_then(|text| Item::parse(&text))
                .map(|item| (item, started.elapsed().as_millis()));
                if sender
                    .send(Event::Captured(job.generation, job.view, Box::new(result)))
                    .is_err()
                {
                    break;
                }
            }
        });
    }
    let (worker, jobs) = mpsc::sync_channel::<Job>(1);
    {
        let sender = sender.clone();
        let current = current.clone();
        std::thread::spawn(move || {
            let mut trade = Trade::new();
            while let Ok(job) = jobs.recv() {
                let result = trade.as_mut().map_err(|e| e.clone()).and_then(|trade| {
                    trade.search(&job.league, &job.query, job.generation, &current)
                });
                if sender.send(Event::Prices(job.generation, result)).is_err() {
                    break;
                }
            }
        });
    }
    let (economy_worker, economy_jobs) = mpsc::sync_channel::<EconomyJob>(1);
    {
        let sender = sender.clone();
        let current = current.clone();
        std::thread::spawn(move || {
            let mut economy = Economy::new();
            while let Ok(job) = economy_jobs.recv() {
                let event = match job {
                    EconomyJob::Price {
                        generation,
                        league,
                        item,
                    } => {
                        let result = economy
                            .as_mut()
                            .map_err(|e| e.clone())
                            .and_then(|e| e.report(&league, &item, generation, &current));
                        Event::Economy(generation, Box::new(result))
                    }
                    EconomyJob::Catalogue {
                        generation,
                        league,
                        choices,
                    } => {
                        let result = economy.as_mut().map_err(|e| e.clone()).and_then(|economy| {
                            choices
                                .into_iter()
                                .map(|choice| {
                                    let item = choice.trade_item()?;
                                    economy
                                        .report(&league, &item, generation, &current)
                                        .map(|r| (choice.id, r))
                                })
                                .collect::<Result<Vec<_>, String>>()
                        });
                        Event::CataloguePrices(generation, Box::new(result))
                    }
                };
                if sender.send(event).is_err() {
                    break;
                }
            }
        });
    }
    let (preferences_worker, preferences_jobs) = mpsc::sync_channel(1);
    let ui = Rc::new(Ui {
        widget_canvas,
        window: window.clone(),
        delve,
        stopwatches,
        stopwatch_manager,
        images,
        image_manager,
        pending_image_edit: Cell::new(None),
        widget_transaction: Cell::new(false),
        order_transaction: Cell::new(false),
        pending_interaction: Cell::new(None),
        pending_stopwatch_edit: Cell::new(None),
        started: Instant::now(),
        desktop: RefCell::new(Snapshot::default()),
        title,
        text,
        hint,
        league,
        online,
        search_range,
        currency,
        economy_panel,
        unidentified,
        catalogue,
        identity_epoch: Cell::new(0),
        economy_worker,
        economy_result: RefCell::new(None),
        mod_box,
        mod_scroll,
        mod_expander,
        stack,
        inspection,
        inspection_result: RefCell::new(None),
        view: Cell::new(View::Price),
        stat_rows: RefCell::new(Vec::new()),
        mercenary_panel: RefCell::new(None),
        preset,
        item_filters,
        events: sender.clone(),
        updating_preset: Cell::new(false),
        state: RefCell::new(Overlay::default()),
        settings: RefCell::new(settings),
        preferences,
        preferences_worker,
        preferences_pending: Cell::new(false),
        preferences_epoch: Cell::new(0),
        preferences_return: Cell::new(View::Price),
        backend_connection: Cell::new(0),
        backend_status: RefCell::new(Err("Initialisation des raccourcis…".into())),
        updating_settings: Cell::new(false),
        css,
        item: RefCell::new(None),
        current,
        game_active,
        worker,
        capture_worker,
        url: RefCell::new(None),
        shown: RefCell::new(None),
        result: RefCell::new(None),
        request: RefCell::new(None),
    });
    {
        let owner = ui.clone();
        preferences_open.connect_clicked(move |_| owner.open_settings());
        let owner = ui.clone();
        stopwatch_open.connect_clicked(move |_| owner.open_stopwatches());
        let owner = ui.clone();
        image_open.connect_clicked(move |_| owner.open_images());
        let owner = ui.clone();
        catalogue_open.connect_clicked(move |_| owner.open_catalogue());
        let owner = ui.clone();
        delve_open.connect_clicked(move |_| owner.toggle_delve());
        let owner = ui.clone();
        ui.preferences
            .apply
            .connect_clicked(move |_| owner.save_settings());
        let owner = ui.clone();
        ui.preferences
            .cancel
            .connect_clicked(move |_| owner.cancel_settings());
        let owner = ui.clone();
        ui.preferences
            .reset
            .connect_clicked(move |_| owner.preferences.defaults());
        for (index, button) in ui.preferences.capture_buttons().into_iter().enumerate() {
            let owner = ui.clone();
            button.connect_clicked(move |_| owner.preferences.arm_capture(index));
            let owner = ui.clone();
            button.connect_focus_out_event(move |_, _| {
                owner.preferences.cancel_capture();
                glib::Propagation::Proceed
            });
        }
    }
    {
        let ui = ui.clone();
        let league = ui.league.clone();
        league.connect_changed(move |_| {
            if let Some(item) = ui.item.borrow().as_ref() {
                ui.reset_item_filters(item);
                let generation = ui.state.borrow().generation;
                ui.filters_changed(generation, "league");
            }
        });
    }
    {
        let ui = ui.clone();
        let online = ui.online.clone();
        online.connect_toggled(move |c| {
            ui.item_filters.set_offline(!c.is_active());
            let generation = ui.state.borrow().generation;
            ui.filters_changed(generation, "online");
        });
    }
    {
        let ui = ui.clone();
        let preset = ui.preset.clone();
        preset.connect_changed(move |_| ui.preset_changed());
    }
    {
        let ui = ui.clone();
        close.connect_clicked(move |_| ui.hide(HideReason::Button));
    }
    {
        let ui = ui.clone();
        search.connect_clicked(move |_| ui.search());
    }
    {
        let ui = ui.clone();
        browser.connect_clicked(move |_| {
            if let Some(url) = ui.url.borrow().clone() {
                ui.hide(HideReason::ExternalLink);
                if let Err(e) = std::process::Command::new("xdg-open").arg(url).spawn() {
                    eprintln!("open Trade: {e}");
                }
            }
        });
    }
    {
        let ui = ui.clone();
        let open = ui.economy_panel.open.clone();
        open.connect_clicked(move |_| {
            if let Some(url) = ui.economy_panel.url() {
                ui.hide(HideReason::ExternalLink);
                if let Err(e) = std::process::Command::new("xdg-open").arg(url).spawn() {
                    eprintln!("open economy: {e}");
                }
            }
        });
    }
    {
        let ui = ui.clone();
        evaluate.connect_clicked(move |_| ui.change_view(View::Inspection));
    }
    {
        let panel = ui.inspection.clone();
        let ui = ui.clone();
        panel
            .price
            .connect_clicked(move |_| ui.change_view(View::Price));
    }
    {
        let panel = ui.inspection.clone();
        let ui = ui.clone();
        panel
            .close
            .connect_clicked(move |_| ui.hide(HideReason::Button));
    }
    for (button, kind) in [
        (ui.inspection.wiki.clone(), 0),
        (ui.inspection.poedb.clone(), 1),
        (ui.inspection.coe.clone(), 2),
    ] {
        let ui = ui.clone();
        button.connect_clicked(move |_| {
            let url = ui
                .inspection_result
                .borrow()
                .as_ref()
                .and_then(|r| match kind {
                    0 => Some(r.links.wiki.clone()),
                    1 => Some(r.links.poedb.clone()),
                    _ => r.links.craft_of_exile.clone(),
                });
            if let Some(url) = url {
                ui.hide(HideReason::ExternalLink);
                if let Err(error) = std::process::Command::new("xdg-open").arg(url).spawn() {
                    eprintln!("inspection open link: {error}");
                }
            }
        });
    }
    {
        let ui = ui.clone();
        window.connect_realize(move |w| {
        apply_input(w,false);
        if let Some(clock) = w.frame_clock() {
            let weak = Rc::downgrade(&ui);
            clock.connect_after_paint(move |_| { if let Some(ui) = weak.upgrade() {
                if ui.state.borrow().state == State::Preparing {
                    ui.state.borrow_mut().painted();
                    eprintln!("overlay state=Passive first_frame=painted display_ms={} geometry={}x{} activation_requested=false",ui.shown.borrow().as_ref().map(|t| t.elapsed().as_millis()).unwrap_or(0),ui.window.allocated_width(),ui.window.allocated_height());
                    if ui.pending_interaction.take()==Some(ui.state.borrow().generation) {ui.command("interact");}
                }
            } });
        }
    });
    }
    {
        let ui = ui.clone();
        window.connect_size_allocate(move |w, _| {
            apply_input(w, ui.state.borrow().state == State::Interactive)
        });
    }
    {
        let ui = ui.clone();
        window.connect_key_press_event(move |_, e| {
            if ui.view.get() == View::Settings && ui.preferences.capture_key(e) {
                return glib::Propagation::Stop;
            }
            if e.keyval() == gdk::keys::constants::Escape {
                ui.hide(HideReason::Escape);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    {
        let ui = ui.clone();
        window.connect_focus_in_event(move |_, event| {
            eprintln!(
                "overlay event=focus-in in={} elapsed_ms={} state={:?} generation={} gtk_active={}",
                event.is_in(),
                ui.started.elapsed().as_millis(),
                ui.state.borrow().state,
                ui.state.borrow().generation,
                ui.window.is_active()
            );
            ui.state.borrow_mut().focus_restored();
            if ui.window.is_visible() && ui.state.borrow().state == State::Interactive {
                ui.hide_delve(crate::delve::Reason::Interactive);
            }
            glib::Propagation::Proceed
        });
    }
    {
        let ui = ui.clone();
        window.connect_focus_out_event(move |_, event| {
            eprintln!("overlay event=focus-out in={} elapsed_ms={} state={:?} generation={} gtk_active={}",event.is_in(),ui.started.elapsed().as_millis(),ui.state.borrow().state,ui.state.borrow().generation,ui.window.is_active());
            ui.defer_focus_check();
            glib::Propagation::Proceed
        });
    }
    {
        let weak = Rc::downgrade(&ui);
        window.connect_is_active_notify(move |w| {
            if let Some(ui) = weak.upgrade() {
                if w.is_active() {
                    ui.state.borrow_mut().focus_restored();
                } else {
                    ui.defer_focus_check();
                }
            }
        });
    }
    {
        let weak = Rc::downgrade(&ui);
        window.connect_grab_notify(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.defer_focus_check();
            }
        });
    }
    {
        let ui = ui.clone();
        window.connect_delete_event(move |_, _| {
            ui.hide(HideReason::WindowClosed);
            glib::Propagation::Stop
        });
    }
    {
        let ui = ui.clone();
        receiver.attach(None, move |event| {
            ui.event(event);
            glib::ControlFlow::Continue
        });
    }
    {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let updates = sender.clone();
            if let Err(e) = wayland::track(move |s| {
                let _ = updates.send(Event::Windows(s));
            }) {
                let _ = sender.send(Event::TrackerError(e));
            }
        });
    }
    {
        let actions = sender.clone();
        let reports = sender.clone();
        let active = ui.game_active.clone();
        let initial = ui.settings.borrow().clone();
        std::thread::spawn(move || {
            crate::desktop::preferences::run(
                initial,
                active,
                preferences_jobs,
                move |command| {
                    let _ = actions.send(Event::Hotkey(command));
                },
                move |update| {
                    let _ = reports.send(Event::Preferences(Box::new(update)));
                },
            )
        });
    }
    let fd = socket.as_raw_fd();
    {
        let ui = ui.clone();
        glib::source::unix_fd_add_local(fd, glib::IOCondition::IN, move |_, _| {
            let mut bytes = [0; 4096];
            while let Ok((n, address)) = socket.recv_from(&mut bytes) {
                if &bytes[..n] == b"status" {
                    if let Some(path) = address.as_pathname() {
                        let state = ui.state.borrow();
                        let mut value = serde_json::json!({"state":format!("{:?}",state.state),"last_hide_reason":state.last_hide.map(|r|format!("{r:?}")),"desktop":*ui.desktop.borrow(),"mod_scroll":ui.mod_scroll.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.mod_scroll.allocated_width(),"height":ui.mod_scroll.allocated_height(),"value":ui.mod_scroll.vadjustment().value(),"upper":ui.mod_scroll.vadjustment().upper(),"page_size":ui.mod_scroll.vadjustment().page_size()})),"mercenary_controls":ui.mercenary_panel.borrow().as_ref().map(|p|p.diagnostics(&ui.window)),"stat_controls":ui.stat_rows.borrow().iter().map(|r|r.diagnostics(&ui.window)).collect::<Vec<_>>(),"preset_control":ui.preset.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.preset.allocated_width(),"height":ui.preset.allocated_height(),"popup":ui.preset.is_popup_shown()})),"mod_expander":ui.mod_expander.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.mod_expander.allocated_width(),"height":ui.mod_expander.allocated_height(),"expanded":ui.mod_expander.is_expanded()})),"generation":state.generation,"game_active":state.game_active,"rendered":state.rendered,"visible":ui.window.is_visible(),"width":ui.window.allocated_width(),"height":ui.window.allocated_height(),"scale":ui.window.scale_factor(),"view":format!("{:?}",ui.view.get()),"preset":ui.preset.active_id().as_deref(),"inspection":ui.inspection_result.borrow().as_ref().filter(|_|ui.view.get()==View::Inspection).map(|r|serde_json::json!({"map_like":r.map_like,"profile":ui.settings.borrow().map_check.profile,"stats":r.stats,"unknown":r.unknown,"outdated":r.outdated,"dps":r.dps,"dust":r.dust})),"app_scale":ui.settings.borrow().scale,"keyboard_mode":format!("{:?}",ui.window.keyboard_mode()),"owned_popup":owned_popup(&ui.window).is_some(),"origin":{"x":ui.window.layer_shell_margin(Edge::Left),"y":ui.window.layer_shell_margin(Edge::Top)},"overlay_focused":ui.window.is_active(),"payment_currency":ui.currency.active_id().as_deref(),"minimum_stock":ui.item_filters.read().ok().and_then(|p|p.stack_size.filter(|f|!f.disabled).map(|f|f.value)),"item_filters":ui.item_filters.read().ok(),"item":ui.item.borrow().as_ref().map(|i| serde_json::json!({"name":i.name,"base":i.base,"trade_tag":i.trade_tag,"stack":i.stack_size,"reference":i.reference,"namespace":i.namespace,"unidentified":i.unidentified,"virtual":i.raw=="VIRTUAL_ITEM"})),"economy_panel":ui.economy_panel.diagnostics(&ui.window),"economy":ui.economy_result.borrow().as_ref().filter(|(generation,_)| *generation==state.generation).map(|(_,r)| match r {Ok(r)=>serde_json::to_value(r).unwrap_or_default(),Err(e)=>serde_json::json!({"error":e})}),"trade_link":ui.url.borrow().clone(),"request":ui.request.borrow().as_ref().filter(|(generation,_)| *generation==state.generation).map(|(_,q)| q),"query":ui.result.borrow().as_ref().filter(|(generation,_)| *generation == state.generation).map(|(_,r)| match r { Ok(p) => serde_json::json!({"kind":p.kind,"total":p.total,"listings":p.listings.len(),"first_offer":p.listings.first().map(|l| serde_json::json!({"unit_price":l.amount,"currency":l.currency,"bulk":l.bulk})),"network_ms":p.network_ms,"cached":p.cached}), Err(e) => serde_json::json!({"error":e}) })});
                        value["delve"] = match &ui.delve {
                            Ok(grid) => grid.diagnostics(),
                            Err(error) => serde_json::json!({"available":false,"error":error}),
                        };
                        value["delve_button"]=serde_json::json!(delve_open.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":delve_open.allocated_width(),"height":delve_open.allocated_height()})));
                        value["widget_canvas"] = match &ui.widget_canvas {
                            Ok(c) => c.diagnostics(),
                            Err(e) => serde_json::json!({"available":false,"error":e}),
                        };
                        value["images"] = match &ui.images {
                            Ok(r) => r.diagnostics(ui.state.borrow().state == State::Interactive),
                            Err(e) => serde_json::json!({"available":false,"error":e}),
                        };
                        value["image_manager"] = ui
                            .image_manager
                            .diagnostics(&ui.window, ui.view.get() == View::Images);
                        value["image_button"]=serde_json::json!(image_open.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":image_open.allocated_width(),"height":image_open.allocated_height()})));
                        value["stopwatches"] = match &ui.stopwatches {
                            Ok(r) => r.snapshot(ui.state.borrow().state == State::Interactive),
                            Err(e) => serde_json::json!({"available":false,"error":e}),
                        };
                        value["stopwatch_manager"] = ui
                            .stopwatch_manager
                            .diagnostics(&ui.window, ui.view.get() == View::Stopwatches);
                        value["stopwatch_button"]=serde_json::json!(stopwatch_open.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":stopwatch_open.allocated_width(),"height":stopwatch_open.allocated_height()})));
                        value["catalogue"] = ui.catalogue.diagnostics(&ui.window);
                        value["catalogue_button"]=serde_json::json!(catalogue_open.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":catalogue_open.allocated_width(),"height":catalogue_open.allocated_height()})));
                        value["preferences"] = ui.preferences.diagnostics(&ui.window);
                        value["preferences_pending"] =
                            serde_json::json!(ui.preferences_pending.get());
                        {
                            let settings = ui.settings.borrow();
                            value["settings"] = serde_json::json!({"image_strip_count":settings.image_strips.len(),"stopwatch_count":settings.stopwatches.len(),"league":settings.league,"scale":settings.scale,"search_range":settings.search_range,"automatic_search":settings.automatic_search,"hotkeys":settings.hotkeys,"filters":{"online":settings.filters.online,"merchant_only":settings.filters.merchant_only,"collapse_listings":settings.filters.collapse_listings,"bulk_currency":settings.filters.bulk_currency,"default_currency":settings.filters.default_currency,"activate_stock_filter":settings.filters.activate_stock_filter,"stats_count":settings.filters.stats.len()},"map_check":{"profile":settings.map_check.profile,"showNewStats":settings.map_check.show_new_stats,"selected_stats_count":settings.map_check.selected_stats.len()}});
                        }
                        value["shortcut_backend"] = serde_json::json!({"connection":ui.backend_connection.get(),"status":*ui.backend_status.borrow()});
                        value["settings_button"]=serde_json::json!(preferences_open.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":preferences_open.allocated_width(),"height":preferences_open.allocated_height()})));
                        value["unidentified_resolver"] = ui.unidentified.diagnostics(&ui.window);
                        value["league_control"] = serde_json::json!(ui.league.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.league.allocated_width(),"height":ui.league.allocated_height(),"value":ui.league.text().as_str()})));
                        let _ = socket.send_to(status_payload(&value).as_bytes(), path);
                    }
                    continue;
                }
                ui.command(std::str::from_utf8(&bytes[..n]).unwrap_or(""));
            }
            glib::ControlFlow::Continue
        });
    }
    if args.first().is_some_and(|a| a == "probe") {
        let ui = ui.clone();
        glib::timeout_add_local_once(Duration::from_secs(2), move || ui.command("demo"));
        glib::timeout_add_local_once(Duration::from_secs(12), gtk::main_quit);
    }
    gtk::main();
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[cfg(test)]
mod diagnostic_tests {
    #[test]
    fn large_widget_diagnostics_still_fit_the_local_reply_buffer() {
        let value = serde_json::json!({"state":"Hidden","view":"Images","game_active":true,"visible":false,"stat_controls":vec!["x".repeat(300);400],"images":{"rows":vec![0;16]},"stopwatches":{"rows":vec![0;32]}});
        let encoded = super::status_payload(&value);
        assert!(encoded.len() < 65536);
        let reply: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(reply["state"], "Hidden");
        assert_eq!(reply["game_active"], true);
        assert_eq!(reply["diagnostics_truncated"], true);
        assert_eq!(reply["counts"]["image_strips"], 16);
        let small = serde_json::json!({"state":"Passive"});
        assert_eq!(super::status_payload(&small), small.to_string());
    }
}
