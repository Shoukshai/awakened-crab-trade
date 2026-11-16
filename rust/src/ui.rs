use crate::{
    desktop::{
        clipboard,
        wayland::{self, Snapshot},
    },
    economy::{Economy, Quote},
    filters,
    item::Item,
    overlay::{DesktopState, HideReason, Overlay, State},
    settings::Settings,
    trade::{Prices, SearchKind, Trade},
};
mod economy_panel;
mod inspection_panel;
mod item_filters_panel;
mod stat_row;
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

type EconomyResult = Result<Option<Quote>, String>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Price,
    Inspection,
}
enum Event {
    Shortcut(&'static str),
    Windows(Snapshot),
    TrackerError(String),
    Captured(u64, View, Box<Result<(Item, u128), String>>),
    Prices(u64, Result<Prices, String>),
    Economy(u64, Result<Option<Quote>, String>),
    MapSettings(crate::inspection::Edit),
    FiltersChanged(u64, String),
}
struct EconomyJob {
    generation: u64,
    league: String,
    item: Item,
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
    window: gtk::Window,
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
    preset: gtk::ComboBoxText,
    item_filters: Rc<item_filters_panel::Panel>,
    events: glib::Sender<Event>,
    updating_preset: Cell<bool>,
    state: RefCell<Overlay>,
    settings: RefCell<Settings>,
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
        self.current
            .store(self.state.borrow().generation, Ordering::Release);
    }
    fn hide(&self, reason: HideReason) {
        let previous = self.state.borrow().state;
        let focused = self.window.is_active();
        self.state.borrow_mut().hide(reason);
        self.sync_generation();
        apply_input(&self.window, false);
        dismiss_owned_popup(&self.window);
        self.window.hide();
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
        self.window.queue_draw();
        self.hint
            .set_text("Passif · Maj+Espace pour interagir · Ctrl+Maj+D pour fermer");
        eprintln!(
            "overlay state=Preparing generation={} scale={}",
            self.state.borrow().generation,
            self.window.scale_factor()
        );
        true
    }
    fn command(&self, command: &str) {
        if let Some(league) = command.strip_prefix("league ") {
            let mut settings = self.settings.borrow().clone();
            settings.league = league.trim().to_string();
            match settings.save() {
                Ok(()) => {
                    self.league.set_text(&settings.league);
                    self.settings.replace(settings);
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
                self.economy_panel.show(&Ok(None), None);
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
                if self.state.borrow_mut().interact() {
                    apply_input(&self.window, true);
                    self.hint.set_text(
                        "Interactif · cliquez dans le panneau · Échap ou Fermer pour quitter",
                    );
                    eprintln!("overlay state=Interactive keyboard=OnDemand explicit=true");
                } else {
                    eprintln!("interact refused: first frame not ready or PoE inactive");
                }
            }
            "hide" => self.hide(HideReason::Explicit),
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
        if !self.window.is_visible() && !self.show() {
            return;
        }
        if view == View::Price {
            self.search();
        }
    }
    fn build_stat_rows(&self, item: &Item) {
        for child in self.mod_box.children() {
            self.mod_box.remove(&child);
        }
        self.stat_rows.borrow_mut().clear();
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
        self.mod_expander.set_label(Some(&format!(
            "Filtres · {} actifs",
            self.stat_rows
                .borrow()
                .iter()
                .filter(|r| r.selected_filter().is_some())
                .count()
                + self.settings.borrow().filters.stats.len()
        )));
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
        if self.view.get() != View::Price {
            return;
        }
        let Some(item) = self.item.borrow().clone() else {
            return;
        };
        // Even a locally rejected edit supersedes pending results. Otherwise an
        // older response could replace the validation error with stale prices.
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        let generation = self.state.borrow().generation;
        self.url.replace(None);
        self.request.replace(None);
        self.economy_panel.reset();
        self.economy_panel.show(&Ok(None), item.stack_size);
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
        let query = match filters::search_query(&item, &selected) {
            Ok(query) => query,
            Err(e) => {
                self.text.set_text(&e);
                return;
            }
        };
        self.request.replace(Some((generation, query.clone())));
        self.economy_panel.reset();
        if self
            .economy_worker
            .try_send(EconomyJob {
                generation,
                league: settings.league.clone(),
                item: item.clone(),
            })
            .is_err()
        {
            self.economy_panel.show(
                &Err("une requête indicative est déjà en cours ; réessayez".into()),
                item.stack_size,
            );
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
    fn event(&self, event: Event) {
        match event {
            Event::Shortcut(command) => self.command(command),
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
                self.game_active.store(active, Ordering::Release);
                self.desktop.replace(s);
                if let Some(reason) = reason {
                    self.hide(reason);
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
                        self.title.set_text(&item.description());
                        self.currency.set_sensitive(item.trade_tag.is_some());
                        if item.trade_tag.as_deref() == self.currency.active_id().as_deref() {
                            self.currency.set_active_id(Some(
                                if item.trade_tag.as_deref() == Some("chaos") {
                                    "divine"
                                } else {
                                    "chaos"
                                },
                            ));
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
                        eprintln!(
                            "filters prepared count={} ui_ms={}",
                            self.stat_rows.borrow().len(),
                            filters_started.elapsed().as_millis()
                        );
                        if self.show() && view == View::Price {
                            self.search();
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
                        self.economy_panel.show(&Ok(None), None);
                        self.title.set_text("Impossible de lire l'objet");
                        self.text.set_text(&e);
                        eprintln!("capture error={e}");
                        self.show();
                    }
                }
            }
            Event::FiltersChanged(generation, origin) => self.filters_changed(generation, &origin),
            Event::MapSettings(edit) => {
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
                        self.text.set_text(&e);
                        eprintln!("trade error generation={generation} error={e}");
                    }
                }
            }
            Event::Economy(generation, result) => {
                if !self.state.borrow().accepts(generation) {
                    eprintln!("discarded economy generation={generation}");
                    return;
                }
                if let Err(e) = &result {
                    eprintln!("economy error generation={generation} error={e}");
                }
                if let Ok(Some(q)) = &result {
                    eprintln!(
                        "economy result generation={generation} chaos={} age_s={} cached={}",
                        q.chaos, q.age_seconds, q.from_cache
                    );
                }
                self.economy_panel.show(
                    &result,
                    self.item.borrow().as_ref().and_then(|i| i.stack_size),
                );
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
            let quote = Economy::new()?.quote(&league, &item, 1, &Arc::new(AtomicU64::new(1)));
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
                serde_json::json!({"item":item,"query":query,"candidate_filters":candidate_filters,"item_filters":filters.item,"presets":modes.iter().map(|m|crate::presets::create(&item,range,*m)).collect::<Vec<_>>(),"suggested_query":filters::search_query(&item,&suggested)})
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
        println!("awakened-poe-trade-rs [daemon|probe|demo|check|copy|interact|hide|quit|status|league NAME|windows|inspect FILE|analyze FILE|inspect-copy|inspect-check|evaluate|prices|lookup FILE [LEAGUE] [CURRENCY]|economy FILE [LEAGUE]]\nStart daemon, copy an item with Ctrl+Alt+C in PoE, then check.\ncopy sends the copy gesture to the active XWayland PoE only.\ninteract enables pointer controls after first render. No automatic focus requests.\nSettings: {}",crate::settings::path().display());
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
    let css = gtk::CssProvider::new();
    css.load_from_data(format!("#apt-native {{ background-color: rgba(14,20,29,0.97); color: #eeeeef; border: 2px solid #b99a5b; border-radius: 10px; }} #apt-native label {{ font: {}px Sans; color: #eeeeef; }} #apt-native button {{ padding: 7px; }} #apt-native button, #apt-native entry, #apt-native spinbutton {{ color: #eeeeef; background-image: none; background-color: #263343; }} #apt-native button.map-danger {{ background-image: none; background-color: #7c2935; }} #apt-native button.map-warning {{ background-image: none; background-color: #865320; }} #apt-native button.map-good {{ background-image: none; background-color: #28603e; }} #apt-native .profile-active {{ border: 2px solid #d9b86b; }} #apt-native treeview {{ background-color: #182230; color: #eeeeef; }}",(16.0*settings.scale) as u32).as_bytes()).map_err(|e| e.to_string())?;
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
    let inspection = inspection_panel::Panel::new(move |edit| {
        let _ = inspection_sender.send(Event::MapSettings(edit));
    });
    stack.add_named(&price_content, "price");
    stack.add_named(&inspection.container, "inspection");
    // GtkStack ignores requests for an invisible child. Make both pages
    // available before selecting the first captured item's view.
    price_content.show_all();
    inspection.container.show_all();
    stack.set_visible_child_name("price");
    content.pack_start(&title, false, false, 0);
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
                let result = economy
                    .as_mut()
                    .map_err(|e| e.clone())
                    .and_then(|e| e.quote(&job.league, &job.item, job.generation, &current));
                if sender.send(Event::Economy(job.generation, result)).is_err() {
                    break;
                }
            }
        });
    }
    let ui = Rc::new(Ui {
        window: window.clone(),
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
        preset,
        item_filters,
        events: sender.clone(),
        updating_preset: Cell::new(false),
        state: RefCell::new(Overlay::default()),
        settings: RefCell::new(settings),
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
        let sender = sender.clone();
        let active = ui.game_active.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::desktop::xwayland_shortcuts::run(
                active,
                move |command| {
                    let _ = sender.send(Event::Shortcut(command));
                },
                || (),
            ) {
                eprintln!("shortcuts unavailable: {e}");
            }
        });
    }
    let fd = socket.as_raw_fd();
    {
        let ui = ui.clone();
        glib::source::unix_fd_add_local(fd, glib::IOCondition::IN, move |_, _| {
            let mut bytes = [0; 1024];
            while let Ok((n, address)) = socket.recv_from(&mut bytes) {
                if &bytes[..n] == b"status" {
                    if let Some(path) = address.as_pathname() {
                        let state = ui.state.borrow();
                        let value = serde_json::json!({"state":format!("{:?}",state.state),"last_hide_reason":state.last_hide.map(|r|format!("{r:?}")),"desktop":*ui.desktop.borrow(),"stat_controls":ui.stat_rows.borrow().iter().map(|r|r.diagnostics(&ui.window)).collect::<Vec<_>>(),"preset_control":ui.preset.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.preset.allocated_width(),"height":ui.preset.allocated_height(),"popup":ui.preset.is_popup_shown()})),"mod_expander":ui.mod_expander.translate_coordinates(&ui.window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":ui.mod_expander.allocated_width(),"height":ui.mod_expander.allocated_height(),"expanded":ui.mod_expander.is_expanded()})),"generation":state.generation,"game_active":state.game_active,"rendered":state.rendered,"visible":ui.window.is_visible(),"width":ui.window.allocated_width(),"height":ui.window.allocated_height(),"scale":ui.window.scale_factor(),"view":format!("{:?}",ui.view.get()),"preset":ui.preset.active_id().as_deref(),"inspection":ui.inspection_result.borrow().as_ref().filter(|_|ui.view.get()==View::Inspection).map(|r|serde_json::json!({"map_like":r.map_like,"profile":ui.settings.borrow().map_check.profile,"stats":r.stats,"unknown":r.unknown,"outdated":r.outdated,"dps":r.dps,"dust":r.dust})),"app_scale":ui.settings.borrow().scale,"keyboard_mode":format!("{:?}",ui.window.keyboard_mode()),"owned_popup":owned_popup(&ui.window).is_some(),"origin":{"x":ui.window.layer_shell_margin(Edge::Left),"y":ui.window.layer_shell_margin(Edge::Top)},"overlay_focused":ui.window.is_active(),"payment_currency":ui.currency.active_id().as_deref(),"minimum_stock":ui.item_filters.read().ok().and_then(|p|p.stack_size.filter(|f|!f.disabled).map(|f|f.value)),"item_filters":ui.item_filters.read().ok(),"item":ui.item.borrow().as_ref().map(|i| serde_json::json!({"name":i.name,"base":i.base,"trade_tag":i.trade_tag,"stack":i.stack_size})),"economy":ui.economy_result.borrow().as_ref().filter(|(generation,_)| *generation==state.generation).map(|(_,r)| match r {Ok(Some(q))=>serde_json::json!({"chaos":q.chaos,"age_seconds":q.age_seconds,"cached":q.from_cache,"category":q.category,"variant":q.variant}),Ok(None)=>serde_json::json!({"matched":false}),Err(e)=>serde_json::json!({"error":e})}),"request":ui.request.borrow().as_ref().filter(|(generation,_)| *generation==state.generation).map(|(_,q)| q),"query":ui.result.borrow().as_ref().filter(|(generation,_)| *generation == state.generation).map(|(_,r)| match r { Ok(p) => serde_json::json!({"kind":p.kind,"total":p.total,"listings":p.listings.len(),"first_offer":p.listings.first().map(|l| serde_json::json!({"unit_price":l.amount,"currency":l.currency,"bulk":l.bulk})),"network_ms":p.network_ms,"cached":p.cached}), Err(e) => serde_json::json!({"error":e}) })});
                        let _ = socket.send_to(value.to_string().as_bytes(), path);
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
