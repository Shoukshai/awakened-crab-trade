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
mod backdrop;
mod economy_panel;
mod item_filters_panel;
mod listings_panel;
mod mercenary_panel;
mod settings_panel;
mod stat_row;
mod unidentified_panel;
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
    Settings,
}
enum Event {
    Leagues(Result<crate::leagues::Selection, String>),
    GameGeometry(u32, crate::price_layout::Rect),
    Hotkey(crate::hotkeys::Action),
    Windows(Snapshot),
    TrackerError(String),
    Captured(u64, bool, Box<Result<(Item, u128), String>>),
    Prices(u64, Result<Prices, String>),
    Economy(u64, Box<EconomyResult>),
    Preferences(Box<crate::desktop::preferences::Update>),
    FiltersChanged(u64, String),
    IdentifyUnique(u64, String),
}
enum EconomyJob {
    Price {
        generation: u64,
        league: String,
        item: Box<Item>,
    },
}
struct CaptureJob {
    generation: u64,
    copy: bool,
    interactive: bool,
    restore: bool,
}
struct Job {
    generation: u64,
    league: String,
    query: serde_json::Value,
}
struct Ui {
    backdrop: Rc<backdrop::Backdrop>,
    window: gtk::Window,
    game_bounds: Cell<Option<crate::price_layout::Rect>>,
    game_window: Cell<Option<u32>>,
    panel_bounds: Cell<Option<crate::price_layout::Rect>>,
    pending_interaction: Cell<Option<u64>>,
    started: Instant,
    frames: Cell<u64>,
    desktop: RefCell<Snapshot>,
    title: gtk::Label,
    text: gtk::Label,
    search_button: gtk::Button,
    listings: listings_panel::Panel,
    hint: gtk::Label,
    league: gtk::Entry,
    league_label: gtk::Label,
    league_worker: SyncSender<()>,
    league_ready: Cell<bool>,
    league_error: RefCell<Option<String>>,
    online: gtk::CheckButton,
    search_range: gtk::SpinButton,
    currency: gtk::ComboBoxText,
    bulk_row: gtk::Box,
    economy_panel: economy_panel::EconomyPanel,
    unidentified: unidentified_panel::Panel,
    identity_epoch: Cell<u64>,
    economy_worker: SyncSender<EconomyJob>,
    economy_result: RefCell<Option<(u64, EconomyResult)>>,
    mod_box: gtk::Box,
    mod_scroll: gtk::ScrolledWindow,
    mod_expander: gtk::Expander,
    stack: gtk::Stack,
    view: Cell<View>,
    stat_rows: RefCell<Vec<stat_row::StatRow>>,
    mercenary_panel: RefCell<Option<mercenary_panel::Panel>>,
    preset: gtk::ComboBoxText,
    item_filters: Rc<item_filters_panel::Panel>,
    events: glib::Sender<Event>,
    state: RefCell<Overlay>,
    settings: RefCell<Settings>,
    updating_preset: Cell<bool>,
    preferences: settings_panel::Panel,
    preferences_worker: SyncSender<crate::desktop::preferences::Request>,
    preferences_pending: Cell<bool>,
    preferences_epoch: Cell<u64>,
    backend_connection: Cell<u64>,
    backend_status: RefCell<Result<(), String>>,
    updating_settings: Cell<bool>,
    css: gtk::CssProvider,
    item: RefCell<Option<Item>>,
    current: Arc<AtomicU64>,
    game_active: Arc<AtomicBool>,
    worker: SyncSender<Job>,
    capture_worker: SyncSender<CaptureJob>,
    capture_pending: Cell<Option<u64>>,
    url: RefCell<Option<String>>,
    shown: RefCell<Option<Instant>>,
    result: RefCell<Option<(u64, Result<Prices, String>)>>,
    request: RefCell<Option<(u64, serde_json::Value)>>,
}
fn set_style(window: &gtk::Window, css: &gtk::CssProvider, scale: f64) -> Result<(), String> {
    window.set_size_request((480. * scale) as i32, 0);
    css.load_from_data(format!("#apt-native {{ background-color: rgba(14,20,29,0.97); color: #eeeeef; border: 2px solid #b99a5b; border-radius: 0px; }} #apt-native label {{ font: {}px Sans; color: #eeeeef; }} #apt-native button {{ padding: 7px; }} #apt-native button, #apt-native entry, #apt-native spinbutton {{ color: #eeeeef; background-image: none; background-color: #263343; }} #apt-native button.map-danger {{ background-image: none; background-color: #7c2935; }} #apt-native button.map-warning {{ background-image: none; background-color: #865320; }} #apt-native button.map-good {{ background-image: none; background-color: #28603e; }} #apt-native .profile-active {{ border: 2px solid #d9b86b; }} #apt-native .mod-row {{ border: 1px solid #59606b; border-radius: 4px; background-color: #1b2430; }} #apt-native .mod-row.selected {{ border-color: #c7a65f; background-color: #373321; }} #apt-native .trade-row {{ border: 1px solid #4a515e; border-radius: 3px; background-color: #1b2430; }} #apt-native togglebutton:checked, #apt-native button:checked {{ border-color: #c7a65f; background-color: #373321; }} #apt-native .trade-row:hover {{ border-color: #b99a5b; }} #apt-offer-details, tooltip {{ background-color: #151b24; border: 1px solid #b99a5b; padding: 10px; }} #apt-offer-details label, tooltip label {{ color: #eeeeef; font: 14px Sans; }} #apt-native treeview {{ background-color: #182230; color: #eeeeef; }} #apt-native notebook, #apt-native notebook > stack, #apt-native notebook > header, #apt-native viewport {{ background-color: #182230; color: #eeeeef; }} #apt-native notebook > header > tabs > tab {{ background-color: #263343; }}",(16.0*scale) as u32).as_bytes()).map_err(|e| e.to_string())?;
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
    serde_json::json!({"diagnostics_truncated":true,"state":value["state"],"view":value["view"],"game_active":value["game_active"],"visible":value["visible"],"generation":value["generation"],"query":value["query"]}).to_string()
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
    fn refresh_league(&self) {
        self.league_error.replace(None);
        let s = self.settings.borrow();
        if s.automatic_league {
            self.league_ready.set(false);
            self.search_button.set_sensitive(false);
            self.league_label.set_text("Détection de la ligue…");
            let _ = self.league_worker.try_send(());
        } else {
            self.league_ready.set(true);
            self.search_button.set_sensitive(true);
            self.league_label.set_text(&s.league);
        }
    }

    fn position_panel(&self, actual_width: Option<i32>) {
        use crate::price_layout::Rect;
        if let Some((id, bounds)) = crate::desktop::game_geometry::focused() {
            self.game_bounds.set(Some(bounds));
            self.game_window.set(Some(id));
        }
        let Some(monitor) =
            gtk::gdk::Display::default().and_then(|d| d.primary_monitor().or_else(|| d.monitor(0)))
        else {
            return;
        };
        let g = monitor.geometry();
        let output = Rect {
            x: g.x(),
            y: g.y(),
            width: g.width(),
            height: g.height(),
        };
        let factor = monitor.scale_factor().max(1);
        let game = self
            .game_bounds
            .get()
            .map(|b| Rect {
                x: b.x / factor,
                y: b.y / factor,
                width: b.width / factor,
                height: b.height / factor,
            })
            .unwrap_or(output);
        let width = actual_width.unwrap_or((480. * self.settings.borrow().scale) as i32);
        let p = crate::price_layout::beside_inventory(output, game, width);
        if self.panel_bounds.get() == Some(p) {
            return;
        }
        self.panel_bounds.set(Some(p));
        self.backdrop.position(
            &monitor,
            Rect {
                x: p.x - output.x,
                y: p.y - output.y,
                ..p
            },
        );
        self.mod_scroll
            .set_max_content_height((output.height * 42 / 100).clamp(220, 650));
        self.window.set_monitor(&monitor);
        self.window
            .set_layer_shell_margin(Edge::Left, p.x - output.x);
        self.window.set_layer_shell_margin(Edge::Top, 0);
        self.window.set_layer_shell_margin(Edge::Bottom, 0);
        eprintln!("price panel geometry={p:?} game={game:?}");
    }

    fn defer_focus_check(self: &Rc<Self>) {
        let Some(token) = self.state.borrow_mut().focus_lost() else {
            return;
        };
        let weak = Rc::downgrade(self);
        // GTK menus temporarily move keyboard focus to another native surface.
        // Observe settled state; never request activation or focus to compensate.
        glib::timeout_add_local_once(Duration::from_millis(30), move || {
            if let Some(ui) = weak.upgrade() {
                let active = ui.window.is_active();
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
    fn sync_generation(&self) {
        let generation = self.state.borrow().generation;
        if self.state.borrow().state == State::Preparing && self.pending_interaction.get().is_some()
        {
            self.pending_interaction.set(Some(generation));
        }
        if self.capture_pending.get().is_some_and(|g| g != generation) {
            self.capture_pending.set(None);
        }
        self.current.store(generation, Ordering::Release);
    }
    fn hide(&self, reason: HideReason) {
        self.pending_interaction.set(None);
        self.state.borrow_mut().hide(reason);
        self.sync_generation();
        apply_input(&self.window, false);
        dismiss_owned_popup(&self.window);
        self.backdrop.hide();
        self.window.hide();
        eprintln!(
            "overlay state=Hidden reason={reason:?} generation={} input=empty keyboard=None",
            self.state.borrow().generation
        );
    }
    fn show(&self) -> bool {
        let generation = self.state.borrow_mut().show();
        if generation.is_none() {
            eprintln!("show refused: PoE is not active");
            return false;
        }
        self.sync_generation();
        self.shown.replace(Some(Instant::now()));
        self.position_panel(None);
        apply_input(&self.window, false);
        self.backdrop.show();
        self.window.show_all();
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
        self.listings.clear();
        self.request.replace(None);
        self.url.replace(None);
        self.economy_result.replace(None);
        self.economy_panel.not_requested();
        self.economy_panel.container.hide();
        self.state.borrow().generation
    }
    fn open_settings(&self) {
        if self.preferences_pending.get()
            || (self.view.get() == View::Settings && self.window.is_visible())
        {
            return;
        }
        self.preferences_epoch.set(self.preferences_epoch.get() + 1);
        self.invalidate_prices();
        self.preferences.show(&self.settings.borrow());
        self.view.set(View::Settings);
        self.title.set_text("Réglages du price checker");
        self.stack.set_visible_child_name("settings");
        if !self.window.is_visible() {
            self.show();
        }
    }
    fn cancel_settings(&self) {
        if self.preferences_pending.get() {
            return;
        }
        self.view.set(View::Price);
        if let Some(item) = self.item.borrow().as_ref() {
            self.title.set_text(&item.description());
            self.stack
                .set_visible_child_name(if item.needs_unique_identity() {
                    "identify"
                } else {
                    "price"
                });
            self.text
                .set_text("Réglages annulés. Cliquez Rechercher pour consulter les prix.");
        } else {
            self.hide(HideReason::Button);
        }
    }
    fn save_settings(&self) {
        if self.view.get() != View::Settings
            || self.preferences_pending.get()
            || self.state.borrow().state != State::Interactive
        {
            return;
        }
        let candidate = match self.preferences.read() {
            Ok(s) => s,
            Err(e) => {
                self.preferences.error(&e);
                return;
            }
        };
        self.preferences_pending.set(true);
        self.preferences.busy(true);
        self.stack.set_sensitive(false);
        if let Err(e) = self
            .preferences_worker
            .try_send(crate::desktop::preferences::Request {
                epoch: self.preferences_epoch.get(),
                settings: candidate,
            })
        {
            self.preferences_pending.set(false);
            self.stack.set_sensitive(true);
            self.preferences.error(&e.to_string());
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
                let _ = self.backend_status.replace(result);
            }
            Update::Failed { epoch, error } => {
                self.preferences_pending.set(false);
                self.stack.set_sensitive(true);
                if epoch == self.preferences_epoch.get() {
                    self.preferences.error(&error);
                }
            }
            Update::Applied {
                epoch,
                settings,
                connection,
                available,
            } => {
                self.preferences_pending.set(false);
                self.stack.set_sensitive(true);
                self.backend_connection.set(connection);
                let _ = self.backend_status.replace(if available {
                    Ok(())
                } else {
                    Err("Connexion aux raccourcis perdue".into())
                });
                self.settings.replace(*settings.clone());
                self.refresh_league();
                self.invalidate_prices();
                self.updating_settings.set(true);
                self.league.set_text(&settings.league);
                self.online.set_active(settings.filters.online);
                self.search_range.set_value(settings.search_range);
                self.currency
                    .set_active_id(Some(&settings.filters.bulk_currency));
                self.updating_settings.set(false);
                if let Err(e) = set_style(&self.window, &self.css, settings.scale) {
                    eprintln!("style: {e}");
                }
                eprintln!("settings committed epoch={epoch} automatic_http=false");
                if self.view.get() == View::Settings && epoch == self.preferences_epoch.get() {
                    self.hide(HideReason::Button);
                }
            }
        }
    }
    fn price_after_copy(&self) {
        // Rare equipment needs explicit stat selection before a meaningful comparison.
        let manual_rare = self.item.borrow().as_ref().is_some_and(|i| {
            i.rarity == crate::item::Rarity::Rare
                && !matches!(
                    i.category.as_deref(),
                    Some("map" | "heistcontract" | "heistblueprint")
                )
        });
        if self.settings.borrow().automatic_search && !manual_rare {
            self.search();
        } else {
            self.text
                .set_text("Sélectionnez les stats et leurs bornes, puis cliquez Rechercher.");
            self.economy_panel.not_requested();
            self.economy_panel.container.hide();
        }
    }
    fn command(&self, command: &str) {
        if let Some(league) = command.strip_prefix("league ") {
            if self.preferences_pending.get() {
                return;
            }
            let mut s = self.settings.borrow().clone();
            s.league = league.trim().into();
            match s.save() {
                Ok(()) => {
                    self.league.set_text(&s.league);
                    self.settings.replace(s);
                }
                Err(e) => eprintln!("settings: {e}"),
            };
            return;
        }
        match command {
            "check" | "copy" | "locked-check" | "locked-copy" => {
                if !self.state.borrow().game_active {
                    return;
                }
                self.hide(HideReason::Capture);
                let generation = self.state.borrow().generation;
                if self
                    .capture_worker
                    .try_send(CaptureJob {
                        generation,
                        copy: command.ends_with("copy"),
                        interactive: command.starts_with("locked-"),
                        restore: self.settings.borrow().restore_clipboard,
                    })
                    .is_ok()
                {
                    self.capture_pending.set(Some(generation));
                }
            }
            "interact" => {
                if self.state.borrow_mut().interact() {
                    apply_input(&self.window, self.window.is_visible());
                    self.hint
                        .set_text("Modifiez les filtres puis Rechercher · Échap pour fermer");
                }
            }
            "settings" => self.open_settings(),
            "hide" => self.hide(HideReason::Explicit),
            "quit" => {
                self.hide(HideReason::Quit);
                gtk::main_quit();
            }
            "demo" => {
                self.view.set(View::Price);
                self.stack.set_visible_child_name("price");
                self.title.set_text("Price checker");
                self.text
                    .set_text("Copiez un objet depuis PoE avec Ctrl+D.");
                self.show();
            }
            _ => eprintln!("Unsupported price-checker command"),
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
        self.listings.clear();
        self.request.replace(None);
        self.url.replace(None);
        self.economy_result.replace(None);
        self.economy_panel.reset();
        self.economy_panel.container.hide();
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
        if item.trade_tag.is_some() {
            for child in self.bulk_row.children() {
                child.show_all();
            }
            self.bulk_row.show();
        } else {
            self.bulk_row.hide();
        }
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
        self.stack.set_visible_child_name("price");
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
            .set_min_content_height(if candidates.len() > 2 { 240 } else { 100 });
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
        self.listings.clear();
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
        self.listings.clear();
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
        if !self.league_ready.get() {
            self.text.set_text(if self.league_error.borrow().is_some() {
                "La ligue actuelle n’a pas pu être détectée. Choisissez une ligue dans Réglages."
            } else {
                "Détection de la ligue actuelle en cours. Vous pouvez déjà choisir les stats."
            });
            return;
        }
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
        // Commit text still being edited before incrementing the request generation.
        // Any value-change event queued by GTK then belongs to the previous generation.
        for row in self.stat_rows.borrow().iter() {
            row.commit();
        }
        self.item_filters.commit();
        // Even a locally rejected edit supersedes pending results. Otherwise an
        // older response could replace the validation error with stale prices.
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        let generation = self.state.borrow().generation;
        self.listings.clear();
        self.url.replace(None);
        self.request.replace(None);
        self.listings.clear();
        self.economy_panel.reset();
        self.economy_panel.container.hide();
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
        self.economy_panel.container.hide();
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
            Event::GameGeometry(id, bounds) => {
                if self.game_window.get() == Some(id) {
                    self.game_bounds.set(Some(bounds));
                    if self.window.is_visible() {
                        self.position_panel(Some(self.window.allocated_width()));
                    }
                }
            }
            Event::Leagues(result) => {
                if !self.settings.borrow().automatic_league {
                    return;
                }
                let was_waiting = !self.league_ready.get();
                match result {
                    Ok(selection) => {
                        self.league_error.replace(None);
                        self.league_ready.set(true);
                        self.search_button.set_sensitive(true);
                        self.league_label.set_text(&selection.id);
                        self.league_label
                            .set_tooltip_text(Some("Ligue actuelle, sélection automatique"));
                        self.league.set_text(&selection.id);
                        self.settings.borrow_mut().league = selection.id.clone();
                        self.preferences.detected_league(&selection.id);
                        eprintln!(
                            "league automatic=true id={:?} cached={}",
                            selection.id, selection.from_cache
                        );
                        if was_waiting
                            && self.window.is_visible()
                            && self.view.get() == View::Price
                            && self.item.borrow().is_some()
                        {
                            self.price_after_copy();
                        }
                    }
                    Err(error) => {
                        self.league_error.replace(Some(error.clone()));
                        self.league_label.set_text("Ligue indisponible — Réglages");
                        self.league_label.set_tooltip_text(Some(&error));
                        self.text.set_text("La ligue actuelle n’a pas pu être détectée. Choisissez une ligue dans Réglages.");
                        eprintln!("league automatic failed: {error}");
                    }
                }
            }
            Event::Hotkey(crate::hotkeys::Action::Command(command)) => self.command(command),
            Event::Hotkey(_) => {}
            Event::FiltersChanged(generation, origin) => self.filters_changed(generation, &origin),
            Event::IdentifyUnique(epoch, reference) => self.identify_unique(epoch, &reference),
            Event::Preferences(update) => self.preferences_update(*update),
            Event::Windows(snapshot) => {
                let active = snapshot.game_active();
                if !active && self.capture_pending.take().is_some() {
                    self.state.borrow_mut().generation += 1;
                    self.sync_generation();
                }
                let reason = self.state.borrow_mut().desktop_changed(DesktopState {
                    game_exists: snapshot.game_exists(),
                    game_visible: snapshot.game_present(),
                    game_active: active,
                    other_active: snapshot.other_active(),
                });
                if snapshot.other_active() || !snapshot.game_present() {
                    self.pending_interaction.set(None);
                }
                self.game_active.store(active, Ordering::Release);
                self.desktop.replace(snapshot);
                if let Some(reason) = reason {
                    self.hide(reason);
                }
            }
            Event::TrackerError(error) => {
                self.game_active.store(false, Ordering::Release);
                self.state.borrow_mut().game_active = false;
                self.hide(HideReason::TrackerFailure);
                self.state.borrow_mut().fail();
                eprintln!("tracker: {error}");
            }
            Event::Captured(generation, interactive, result) => {
                if self.capture_pending.get() == Some(generation) {
                    self.capture_pending.set(None);
                }
                if generation != self.state.borrow().generation || !self.state.borrow().game_active
                {
                    return;
                }
                match *result {
                    Ok((item, elapsed)) => {
                        eprintln!("capture processing_ms={elapsed} clipboard_before_overlay=true");
                        self.prepare_item(item, View::Price, generation);
                        if self.show() {
                            self.price_after_copy();
                            if interactive {
                                self.pending_interaction
                                    .set(Some(self.state.borrow().generation));
                            }
                        }
                    }
                    Err(error) => {
                        self.view.set(View::Price);
                        self.stack.set_visible_child_name("price");
                        self.title.set_text("Copie de l’objet impossible");
                        self.text.set_text(&error);
                        self.item.replace(None);
                        self.item_filters.container.hide();
                        self.mod_expander.hide();
                        self.show();
                    }
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
                        self.listings.show(&p);
                        self.text.set_text(&format!(
                            "{} · {} résultats · {} offres{}",
                            if p.kind == SearchKind::Bulk {
                                "Échange en gros"
                            } else {
                                "Recherche Trade"
                            },
                            p.total,
                            p.listings.len(),
                            if p.cached { " · cache" } else { "" }
                        ));
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
                if result.as_ref().is_ok_and(|r| {
                    r.quote.is_some()
                        || r.related.is_some()
                        || r.stack.is_some()
                        || r.warning.is_some()
                }) {
                    self.economy_panel.container.show();
                } else {
                    self.economy_panel.container.hide();
                }
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
        println!("awakened-poe-trade-rs [daemon|check|copy|locked-check|locked-copy|settings|interact|hide|quit|status|league NAME|inspect FILE|lookup FILE [LEAGUE]]\nCtrl+D: price check. Ctrl+Alt+D: interactive price check after first paint.");
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
                | "locked-check"
                | "locked-copy"
                | "hide"
                | "interact"
                | "quit"
                | "league"
                | "settings"
        )
    }) {
        UnixDatagram::unbound()
            .map_err(|e| e.to_string())?
            .send_to(
                if command == "league" {
                    args.join(" ")
                } else {
                    command.to_string()
                }
                .as_bytes(),
                socket_path()?,
            )
            .map_err(|e| format!("start daemon first: {e}"))?;
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
    window.set_size_request((480.0 * settings.scale) as i32, 0);
    if let Some(visual) = GtkWindowExt::screen(&window).and_then(|s| s.rgba_visual()) {
        window.set_visual(Some(&visual));
    }
    window.init_layer_shell();
    window.set_namespace("awakened-poe-trade-rs");
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_layer_shell_margin(Edge::Top, 0);
    window.set_layer_shell_margin(Edge::Left, 0);
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::None);
    let css = gtk::CssProvider::new();
    set_style(&window, &css, settings.scale)?;
    gtk::StyleContext::add_provider_for_screen(
        &GtkWindowExt::screen(&window).unwrap(),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
    let (sender, receiver) = glib::MainContext::channel::<Event>(glib::Priority::DEFAULT);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_border_width(20);
    let title = gtk::Label::new(Some("AWAKENED POE TRADE · RUST"));
    title.set_xalign(0.);
    title.set_line_wrap(true);
    title.set_max_width_chars(28);
    let text = gtk::Label::new(None);
    text.set_xalign(0.);
    text.set_yalign(0.);
    text.set_line_wrap(true);
    text.set_selectable(false);
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scroll.set_min_content_height(160);
    scroll.set_vexpand(true);
    scroll.set_propagate_natural_height(true);
    let listings = listings_panel::Panel::new();
    let result_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    result_box.pack_start(&text, false, false, 0);
    result_box.pack_start(&listings.container, false, false, 0);
    scroll.add(&result_box);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let league = gtk::Entry::new();
    league.set_text(&settings.league);
    league.set_width_chars(18);
    league.set_placeholder_text(Some("Ligue"));
    let online = gtk::CheckButton::with_label("En ligne");
    online.set_active(settings.filters.online);
    let league_label = gtk::Label::new(Some("Détection de la ligue…"));
    league_label.set_xalign(0.);
    row.pack_start(&league_label, true, true, 0);
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
    bulk_row.set_no_show_all(true);
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
    economy_panel.container.set_no_show_all(true);
    for child in economy_panel.container.children() {
        child.show_all();
    }
    let mod_scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    mod_scroll.set_min_content_height(180);
    mod_scroll.set_vexpand(false);
    mod_scroll.set_max_content_height(380);
    mod_scroll.set_propagate_natural_height(true);
    let filter_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    mod_scroll.add(&filter_content);
    let mod_expander =
        gtk::Expander::new(Some("Mods reconnus — cochez un filtre, puis Rechercher"));
    mod_expander.add(&mod_box);
    mod_expander.set_expanded(true);
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
    for b in [&search, &browser, &close] {
        actions.pack_start(b, true, true, 0);
    }
    let hint = gtk::Label::new(None);
    hint.set_line_wrap(true);
    hint.set_xalign(0.);
    let unidentified = unidentified_panel::Panel::new();
    let close_sender = sender.clone();
    unidentified.close.connect_clicked(move |_| {
        let _ = close_sender.send(Event::Hotkey(crate::hotkeys::Action::Command("hide")));
    });
    let preferences = settings_panel::Panel::new();
    let stack = gtk::Stack::new();
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    let price_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    price_content.pack_start(&row, false, false, 0);
    price_content.pack_start(&bulk_row, false, false, 0);
    price_content.pack_start(&economy_panel.container, false, false, 0);
    filter_content.pack_start(&item_filters.container, false, false, 0);
    filter_content.pack_start(&preset, false, false, 0);
    filter_content.pack_start(&mod_expander, false, false, 0);
    price_content.pack_start(&mod_scroll, false, false, 0);
    price_content.pack_start(&actions, false, false, 0);
    price_content.pack_start(&scroll, true, true, 0);
    stack.add_named(&preferences.container, "settings");
    preferences.container.show_all();
    stack.add_named(&unidentified.container, "identify");
    unidentified.container.show_all();
    stack.add_named(&price_content, "price");
    price_content.show_all();
    stack.set_visible_child_name("price");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    title.set_hexpand(true);
    let preferences_open = gtk::Button::with_label("Réglages");
    header.pack_start(&title, true, true, 0);
    header.pack_start(&preferences_open, false, false, 0);
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
                let focus = job.interactive;
                let result = (if job.copy {
                    clipboard::copy_from_game(
                        &crate::desktop::game_input::Context {
                            active: &active,
                            generation: &current,
                            token: job.generation,
                        },
                        job.restore,
                    )
                } else {
                    clipboard::read()
                })
                .and_then(|text| Item::parse(&text))
                .map(|item| (item, started.elapsed().as_millis()));
                if sender
                    .send(Event::Captured(job.generation, focus, Box::new(result)))
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
                };
                if sender.send(event).is_err() {
                    break;
                }
            }
        });
    }
    let (league_worker, league_jobs) = mpsc::sync_channel::<()>(1);
    {
        let sender = sender.clone();
        std::thread::spawn(move || {
            while league_jobs.recv().is_ok() {
                if sender
                    .send(Event::Leagues(crate::leagues::resolve()))
                    .is_err()
                {
                    break;
                }
            }
        });
    }
    let (preferences_worker, preferences_jobs) = mpsc::sync_channel(1);
    let close_sender = sender.clone();
    let backdrop = backdrop::Backdrop::new(move || {
        let _ = close_sender.send(Event::Hotkey(crate::hotkeys::Action::Command("hide")));
    });
    let ui = Rc::new(Ui {
        backdrop,
        window: window.clone(),
        game_bounds: Cell::new(None),
        game_window: Cell::new(None),
        panel_bounds: Cell::new(None),
        pending_interaction: Cell::new(None),
        started: Instant::now(),
        frames: Cell::new(0),
        desktop: RefCell::new(Snapshot::default()),
        title,
        text,
        search_button: search.clone(),
        listings,
        hint,
        league,
        league_label,
        league_worker,
        league_ready: Cell::new(false),
        league_error: RefCell::new(None),
        online,
        search_range,
        currency,
        bulk_row,
        economy_panel,
        unidentified,
        identity_epoch: Cell::new(0),
        economy_worker,
        economy_result: RefCell::new(None),
        mod_box,
        mod_scroll,
        mod_expander,
        stack,
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
        backend_connection: Cell::new(0),
        backend_status: RefCell::new(Err("Initialisation des raccourcis…".into())),
        updating_settings: Cell::new(false),
        css,
        item: RefCell::new(None),
        current,
        game_active,
        worker,
        capture_worker,
        capture_pending: Cell::new(None),
        url: RefCell::new(None),
        shown: RefCell::new(None),
        result: RefCell::new(None),
        request: RefCell::new(None),
    });
    {
        let events = sender.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::desktop::game_geometry::watch(move |id, bounds| {
                let _ = events.send(Event::GameGeometry(id, bounds));
            }) {
                eprintln!("geometry observer stopped: {e}");
            }
        });
    }
    ui.refresh_league();
    {
        let owner = ui.clone();
        preferences_open.connect_clicked(move |_| owner.open_settings());
    }
    {
        let owner = ui.clone();
        ui.preferences
            .apply
            .connect_clicked(move |_| owner.save_settings());
    }
    {
        let owner = ui.clone();
        ui.preferences
            .cancel
            .connect_clicked(move |_| owner.cancel_settings());
    }
    {
        let owner = ui.clone();
        ui.preferences
            .reset
            .connect_clicked(move |_| owner.preferences.defaults());
    }
    for (index, button) in ui.preferences.capture_buttons().into_iter().enumerate() {
        let owner = ui.clone();
        button.connect_clicked(move |_| owner.preferences.arm_capture(index));
        let owner = ui.clone();
        button.connect_focus_out_event(move |_, _| {
            owner.preferences.cancel_capture();
            glib::Propagation::Proceed
        });
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
        window.connect_realize(move |w| {
        apply_input(w,false);
        if let Some(clock) = w.frame_clock() {
            let weak = Rc::downgrade(&ui);
            clock.connect_after_paint(move |_| { if let Some(ui) = weak.upgrade() {
                ui.frames.set(ui.frames.get()+1);
                ui.backdrop.main_painted();
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
            apply_input(w, ui.state.borrow().state == State::Interactive);
            ui.position_panel(Some(w.allocated_width()));
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
                        let mut value = serde_json::json!({"state":format!("{:?}",state.state),"last_hide_reason":state.last_hide.map(|r|format!("{r:?}")),"desktop":*ui.desktop.borrow(),"mod_scroll":ui.mod_scroll.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.mod_scroll.allocated_width(),"height":ui.mod_scroll.allocated_height(),"value":ui.mod_scroll.vadjustment().value(),"upper":ui.mod_scroll.vadjustment().upper(),"page_size":ui.mod_scroll.vadjustment().page_size()})),"mercenary_controls":ui.mercenary_panel.borrow().as_ref().map(|p|p.diagnostics(&ui.window)),"stat_controls":ui.stat_rows.borrow().iter().map(|r|r.diagnostics(&ui.window)).collect::<Vec<_>>(),"preset_control":ui.preset.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.preset.allocated_width(),"height":ui.preset.allocated_height(),"popup":ui.preset.is_popup_shown()})),"mod_expander":ui.mod_expander.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.mod_expander.allocated_width(),"height":ui.mod_expander.allocated_height(),"expanded":ui.mod_expander.is_expanded()})),"generation":state.generation,"game_active":state.game_active,"rendered":state.rendered,"visible":ui.window.is_visible(),"width":ui.window.allocated_width(),"height":ui.window.allocated_height(),"scale":ui.window.scale_factor(),"view":format!("{:?}",ui.view.get()),"preset":ui.preset.active_id().as_deref(),"app_scale":ui.settings.borrow().scale,"keyboard_mode":format!("{:?}",ui.window.keyboard_mode()),"owned_popup":owned_popup(&ui.window).is_some(),"origin":{"x":ui.window.layer_shell_margin(Edge::Left),"y":ui.window.layer_shell_margin(Edge::Top)},"overlay_focused":ui.window.is_active(),"payment_currency":ui.currency.active_id().as_deref(),"minimum_stock":ui.item_filters.read().ok().and_then(|p|p.stack_size.filter(|f|!f.disabled).map(|f|f.value)),"item_filters":ui.item_filters.read().ok(),"item":ui.item.borrow().as_ref().map(|i| serde_json::json!({"name":i.name,"base":i.base,"trade_tag":i.trade_tag,"stack":i.stack_size,"reference":i.reference,"namespace":i.namespace,"unidentified":i.unidentified,"virtual":i.raw=="VIRTUAL_ITEM"})),"economy_panel":ui.economy_panel.diagnostics(&ui.window),"economy":ui.economy_result.borrow().as_ref().filter(|(generation,_)| *generation==state.generation).map(|(_,r)| match r {Ok(r)=>serde_json::to_value(r).unwrap_or_default(),Err(e)=>serde_json::json!({"error":e})}),"trade_link":ui.url.borrow().clone(),"request":ui.request.borrow().as_ref().filter(|(generation,_)| *generation==state.generation).map(|(_,q)| q),"query":ui.result.borrow().as_ref().filter(|(generation,_)| *generation == state.generation).map(|(_,r)| match r { Ok(p) => serde_json::json!({"kind":p.kind,"total":p.total,"listings":p.listings.len(),"first_offer":p.listings.first().map(|l| serde_json::json!({"unit_price":l.amount,"currency":l.currency,"bulk":l.bulk})),"network_ms":p.network_ms,"cached":p.cached}), Err(e) => serde_json::json!({"error":e}) })});
                        value["preferences"] = ui.preferences.diagnostics(&ui.window);
                        value["preferences_pending"] =
                            serde_json::json!(ui.preferences_pending.get());
                        value["settings"] =
                            serde_json::to_value(&*ui.settings.borrow()).unwrap_or_default();
                        value["shortcut_backend"] = serde_json::json!({"connection":ui.backend_connection.get(),"status":*ui.backend_status.borrow()});
                        value["settings_button"]=serde_json::json!(preferences_open.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":preferences_open.allocated_width(),"height":preferences_open.allocated_height()})));
                        value["unidentified_resolver"] = ui.unidentified.diagnostics(&ui.window);
                        value["league_control"] = serde_json::json!(ui.league.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.league.allocated_width(),"height":ui.league.allocated_height(),"value":ui.league.text().as_str()})));
                        value["capture_pending"] = serde_json::json!(ui.capture_pending.get());
                        value["frames"] = serde_json::json!(ui.frames.get());
                        value["backdrop"] = ui.backdrop.diagnostics();
                        value["item_filter_controls"] = ui.item_filters.diagnostics(&ui.window);
                        value["listings"] = ui.listings.diagnostics(&ui.window);
                        value["league_selection"] = serde_json::json!({"automatic":ui.settings.borrow().automatic_league,"ready":ui.league_ready.get(),"label":ui.league_label.text().as_str()});
                        value["panel_bounds"] = serde_json::json!(ui.panel_bounds.get());
                        value["search_button"]=serde_json::json!(search.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":search.allocated_width(),"height":search.allocated_height(),"mapped":search.is_mapped()})));
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
