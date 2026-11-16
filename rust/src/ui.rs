use crate::{
    desktop::{
        clipboard,
        wayland::{self, Snapshot},
    },
    filters,
    item::Item,
    overlay::{Overlay, State},
    settings::Settings,
    trade::{Prices, Trade},
};
use gtk::{gdk, prelude::*};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::RefCell,
    os::unix::{io::AsRawFd, net::UnixDatagram},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};

enum Event {
    Shortcut(&'static str),
    Windows(Snapshot),
    TrackerError(String),
    Captured(u64, Result<(Item, u128), String>),
    Prices(u64, Result<Prices, String>),
}
enum Job {
    Capture {
        generation: u64,
        copy: bool,
    },
    Search {
        generation: u64,
        league: String,
        query: serde_json::Value,
    },
}
struct Ui {
    window: gtk::Window,
    title: gtk::Label,
    text: gtk::Label,
    hint: gtk::Label,
    league: gtk::Entry,
    ilvl: gtk::SpinButton,
    online: gtk::CheckButton,
    mod_box: gtk::Box,
    stat_rows: RefCell<Vec<(gtk::CheckButton, gtk::SpinButton, String)>>,
    state: RefCell<Overlay>,
    settings: RefCell<Settings>,
    item: RefCell<Option<Item>>,
    current: Arc<AtomicU64>,
    game_active: Arc<AtomicBool>,
    worker: SyncSender<Job>,
    url: RefCell<Option<String>>,
    shown: RefCell<Option<Instant>>,
    result: RefCell<Option<(u64, Result<Prices, String>)>>,
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
impl Ui {
    fn sync_generation(&self) {
        self.current
            .store(self.state.borrow().generation, Ordering::Release);
    }
    fn hide(&self) {
        self.state.borrow_mut().hide();
        self.sync_generation();
        apply_input(&self.window, false);
        self.window.hide();
        eprintln!("overlay state=Hidden input=empty keyboard=None");
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
            .set_text("Passif · le jeu garde les entrées · commande interact pour les contrôles");
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
                self.title.set_text("AWAKENED POE TRADE · RUST");
                self.text.set_text("Panneau natif de validation\n\nClavier et clics restent au jeu.\nAucun prix simulé n'est affiché.");
                self.show();
            }
            "check" | "copy" => {
                if !self.state.borrow().game_active {
                    eprintln!("capture refused: PoE is not active");
                    return;
                }
                self.hide();
                let generation = self.state.borrow().generation;
                if let Err(e) = self.worker.try_send(Job::Capture {
                    generation,
                    copy: command == "copy",
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
            "hide" => self.hide(),
            "quit" => {
                self.hide();
                gtk::main_quit();
            }
            _ => eprintln!("unknown command: {command}"),
        }
    }
    fn search(&self) {
        let Some(item) = self.item.borrow().clone() else {
            return;
        };
        let mut settings = self.settings.borrow().clone();
        settings.league = self.league.text().trim().to_string();
        settings.filters.online = self.online.is_active();
        settings.filters.min_item_level =
            (self.ilvl.value_as_int() > 0).then_some(self.ilvl.value_as_int() as u32);
        if let Err(e) = settings.save() {
            self.text.set_text(&e);
            return;
        }
        self.settings.replace(settings.clone());
        let mut selected = settings.filters.clone();
        selected.stats.extend(
            self.stat_rows
                .borrow()
                .iter()
                .filter(|(check, _, _)| check.is_active())
                .map(|(_, value, id)| filters::StatFilter {
                    id: id.clone(),
                    min: Some(value.value()),
                    max: None,
                }),
        );
        let query = filters::query(&item, &selected);
        self.state.borrow_mut().generation += 1;
        self.sync_generation();
        let generation = self.state.borrow().generation;
        self.url.replace(None);
        self.text.set_text(&format!("Recherche dans {}…\n\nComparaison par base et filtres sélectionnés.\nLes mods sont sélectionnés explicitement dans le panneau.",settings.league));
        match self.worker.try_send(Job::Search {
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
                self.state.borrow_mut().game_active = active;
                self.game_active.store(active, Ordering::Release);
                if (s.other_active()
                    || !s.game_present()
                    || (!active && self.state.borrow().state != State::Interactive))
                    && self.state.borrow().state != State::Hidden
                {
                    self.hide();
                }
            }
            Event::TrackerError(e) => {
                self.game_active.store(false, Ordering::Release);
                self.state.borrow_mut().game_active = false;
                self.hide();
                self.state.borrow_mut().fail();
                eprintln!("tracker error={e}");
            }
            Event::Captured(generation, result) => {
                if generation != self.state.borrow().generation || !self.state.borrow().game_active
                {
                    eprintln!("discarded capture generation={generation}");
                    return;
                }
                match result {
                    Ok((item, elapsed)) => {
                        eprintln!(
                            "capture parsed processing_ms={elapsed} clipboard_before_overlay=true"
                        );
                        self.title.set_text(&item.description());
                        for child in self.mod_box.children() {
                            self.mod_box.remove(&child);
                        }
                        self.stat_rows.borrow_mut().clear();
                        for modifier in &item.modifiers {
                            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                            let check = gtk::CheckButton::new();
                            let label = gtk::Label::new(Some(&modifier.text));
                            label.set_line_wrap(true);
                            label.set_max_width_chars(45);
                            label.set_xalign(0.);
                            check.add(&label);
                            check.set_tooltip_text(Some(&modifier.id));
                            let value = gtk::SpinButton::with_range(-100000., 100000., 1.);
                            value.set_digits(1);
                            value.set_value(modifier.value);
                            value.set_tooltip_text(Some("Valeur minimum à rechercher"));
                            row.pack_start(&check, true, true, 0);
                            row.pack_start(&value, false, false, 0);
                            self.mod_box.pack_start(&row, false, false, 0);
                            self.stat_rows
                                .borrow_mut()
                                .push((check, value, modifier.id.clone()));
                        }
                        if item.modifiers.is_empty() {
                            self.mod_box.pack_start(&gtk::Label::new(Some("Aucun mod simple reconnu. Les mods complexes restent à porter.")),false,false,0);
                        }
                        self.item.replace(Some(item));
                        if self.show() {
                            self.search();
                        }
                    }
                    Err(e) => {
                        self.item.replace(None);
                        self.title.set_text("Impossible de lire l'objet");
                        self.text.set_text(&e);
                        eprintln!("capture error={e}");
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
                        let rows = p
                            .listings
                            .iter()
                            .map(|l| format!("{:>8} {:<16} {}", l.amount, l.currency, l.seller))
                            .collect::<Vec<_>>()
                            .join("\n");
                        self.text.set_text(&format!("{} résultats · {} offres affichées\n{}\n\n{}\n\nComparaison par base et filtres sélectionnés.\nVérifiez les mods et la ligue avant d'estimer un rare.",p.total,p.listings.len(),if p.cached {"Cache (< 60 s)".into()} else {format!("Réseau : {} ms",p.network_ms)},if rows.is_empty() {"Aucune offre avec prix."} else {&rows}));
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
        }
    }
}
#[allow(deprecated)] // GTK3's matching glib 0.18 channel; wakes the main loop without polling.
pub fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "help") {
        println!("awakened-poe-trade-rs [daemon|probe|demo|check|copy|interact|hide|quit|windows]\nStart daemon, copy an item with Ctrl+Alt+C in PoE, then check.\ncopy sends the copy gesture to the active XWayland PoE only.\ninteract enables pointer controls after first render. No automatic focus requests.\nSettings: {}",crate::settings::path().display());
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
            let mut buf = [0; 4096];
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
            "demo" | "check" | "copy" | "hide" | "interact" | "quit" | "league"
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
    css.load_from_data(format!("#apt-native {{ background-color: rgba(14,20,29,0.97); color: #eeeeef; border: 2px solid #b99a5b; border-radius: 10px; }} #apt-native label {{ font: {}px Sans; color: #eeeeef; }} #apt-native button {{ padding: 7px; }} #apt-native button, #apt-native entry, #apt-native spinbutton {{ color: #eeeeef; background-image: none; background-color: #263343; }}",(16.0*settings.scale) as u32).as_bytes()).map_err(|e| e.to_string())?;
    gtk::StyleContext::add_provider_for_screen(
        &GtkWindowExt::screen(&window).unwrap(),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
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
    let ilvl = gtk::SpinButton::with_range(0., 100., 1.);
    ilvl.set_value(settings.filters.min_item_level.unwrap_or(0) as f64);
    ilvl.set_tooltip_text(Some("Niveau d'objet minimum ; 0 = aucun"));
    let online = gtk::CheckButton::with_label("En ligne");
    online.set_active(settings.filters.online);
    row.pack_start(&gtk::Label::new(Some("Ligue")), false, false, 0);
    row.pack_start(&league, true, true, 0);
    row.pack_start(&gtk::Label::new(Some("ilvl ≥")), false, false, 0);
    row.pack_start(&ilvl, false, false, 0);
    row.pack_start(&online, false, false, 0);
    let mod_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let mod_scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    mod_scroll.set_min_content_height(90);
    mod_scroll.set_max_content_height(180);
    mod_scroll.add(&mod_box);
    let mod_expander =
        gtk::Expander::new(Some("Mods reconnus — cochez un filtre, puis Rechercher"));
    mod_expander.add(&mod_scroll);
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
    content.pack_start(&title, false, false, 0);
    content.pack_start(&scroll, true, true, 0);
    content.pack_start(&row, false, false, 0);
    content.pack_start(&mod_expander, false, false, 0);
    content.pack_start(&actions, false, false, 0);
    content.pack_start(&hint, false, false, 0);
    window.add(&content);
    let current = Arc::new(AtomicU64::new(0));
    let game_active = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = glib::MainContext::channel::<Event>(glib::Priority::DEFAULT);
    let (worker, jobs) = mpsc::sync_channel(1);
    {
        let sender = sender.clone();
        let current = current.clone();
        let game_active = game_active.clone();
        std::thread::spawn(move || {
            let mut trade = Trade::new();
            while let Ok(job) = jobs.recv() {
                match job {
                    Job::Capture { generation, copy } => {
                        if current.load(Ordering::Acquire) != generation {
                            continue;
                        }
                        let start = Instant::now();
                        let result = (if copy {
                            clipboard::copy_from_game(&game_active)
                        } else {
                            clipboard::read()
                        })
                        .and_then(|s| Item::parse(&s))
                        .map(|item| (item, start.elapsed().as_millis()));
                        if sender.send(Event::Captured(generation, result)).is_err() {
                            break;
                        }
                    }
                    Job::Search {
                        generation,
                        league,
                        query,
                    } => {
                        let result = trade
                            .as_mut()
                            .map_err(|e| e.clone())
                            .and_then(|trade| trade.search(&league, &query, generation, &current));
                        if sender.send(Event::Prices(generation, result)).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }
    let ui = Rc::new(Ui {
        window: window.clone(),
        title,
        text,
        hint,
        league,
        ilvl,
        online,
        mod_box,
        stat_rows: RefCell::new(Vec::new()),
        state: RefCell::new(Overlay::default()),
        settings: RefCell::new(settings),
        item: RefCell::new(None),
        current,
        game_active,
        worker,
        url: RefCell::new(None),
        shown: RefCell::new(None),
        result: RefCell::new(None),
    });
    {
        let ui = ui.clone();
        close.connect_clicked(move |_| ui.hide());
    }
    {
        let ui = ui.clone();
        search.connect_clicked(move |_| ui.search());
    }
    {
        let ui = ui.clone();
        browser.connect_clicked(move |_| {
            if let Some(url) = ui.url.borrow().clone() {
                ui.hide();
                if let Err(e) = std::process::Command::new("xdg-open").arg(url).spawn() {
                    eprintln!("open Trade: {e}");
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
                ui.hide();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    {
        let ui = ui.clone();
        window.connect_focus_out_event(move |_, _| {
            if ui.state.borrow().state == State::Interactive {
                ui.hide();
            }
            glib::Propagation::Proceed
        });
    }
    {
        let ui = ui.clone();
        window.connect_delete_event(move |_, _| {
            ui.hide();
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
            if let Err(e) = crate::desktop::xwayland_shortcuts::run(active, move |command| {
                let _ = sender.send(Event::Shortcut(command));
            }) {
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
                        let value = serde_json::json!({"state":format!("{:?}",state.state),"generation":state.generation,"game_active":state.game_active,"rendered":state.rendered,"visible":ui.window.is_visible(),"width":ui.window.allocated_width(),"height":ui.window.allocated_height(),"scale":ui.window.scale_factor(),"query":ui.result.borrow().as_ref().filter(|(generation,_)| *generation == state.generation).map(|(_,r)| match r { Ok(p) => serde_json::json!({"total":p.total,"listings":p.listings.len(),"network_ms":p.network_ms,"cached":p.cached}), Err(e) => serde_json::json!({"error":e}) })});
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
