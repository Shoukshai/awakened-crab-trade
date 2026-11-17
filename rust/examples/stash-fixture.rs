//! GTK X11 search field on the private desktop; never starts on the host display.
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    io::{BufRead, Write},
    rc::Rc,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")?;
    if !runtime.starts_with("/tmp/apt-overlay-desktop-")
        || std::fs::read_to_string(format!("{runtime}/display"))?.trim()
            != std::env::var("DISPLAY")?
    {
        return Err("private XWayland fixture only".into());
    }
    std::env::set_var("GDK_BACKEND", "x11");
    gtk::init()?;
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Path of Exile");
    window.set_default_size(1000, 700);
    window.connect_realize(|_| {
        let found = std::process::Command::new("xdotool")
            .args([
                "search",
                "--all",
                "--pid",
                &std::process::id().to_string(),
                "--name",
                "^Path of Exile$",
            ])
            .output()
            .unwrap();
        assert!(found.status.success());
        let id = String::from_utf8(found.stdout).unwrap();
        let id = id.trim();
        assert!(!id.contains('\n'));
        assert!(std::process::Command::new("xdotool")
            .args([
                "set_window",
                "--classname",
                "steam_app_238960",
                "--class",
                "steam_app_238960",
                id
            ])
            .status()
            .unwrap()
            .success());
    });
    let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Private stash search field"));
    column.pack_start(&entry, false, false, 0);
    column.pack_start(
        &gtk::Label::new(Some("Private fake game — no chat")),
        false,
        false,
        0,
    );
    window.add(&column);
    let submitted = Rc::new(RefCell::new(Vec::<String>::new()));
    let keys = Rc::new(RefCell::new(Vec::<u32>::new()));
    let key_events = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
    let wheels = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
    window.add_events(gdk::EventMask::SCROLL_MASK | gdk::EventMask::SMOOTH_SCROLL_MASK);
    {
        let wheels = wheels.clone();
        window.connect_scroll_event(move |_,e|{wheels.borrow_mut().push(serde_json::json!({"direction":format!("{:?}",e.direction()),"state":e.state().bits()}));glib::Propagation::Proceed});
    }
    {
        let entry = entry.clone();
        let keys = keys.clone();
        let key_events = key_events.clone();
        window.connect_key_press_event(move |_, e| {
            keys.borrow_mut().push(*e.keyval());
            key_events
                .borrow_mut()
                .push(serde_json::json!({"key":*e.keyval(),"state":e.state().bits()}));
            if e.state()
                .contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::MOD1_MASK)
                && e.keyval() == gdk::keys::constants::c
            {
                let item = if std::env::var("APT_FIXTURE_ITEM").as_deref() == Ok("ring") {
                    include_str!("../src/fixtures/synthetic-calculated-ring.txt")
                } else {
                    include_str!("../src/fixtures/driftwood-wand.txt")
                };
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(item);
                return glib::Propagation::Stop;
            }
            if e.state().contains(gdk::ModifierType::CONTROL_MASK)
                && e.keyval() == gdk::keys::constants::f
            {
                entry.grab_focus();
                entry.select_region(0, -1);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    {
        let submitted = submitted.clone();
        entry.connect_activate(move |e| submitted.borrow_mut().push(e.text().into()));
    }
    #[allow(deprecated)] // Same bounded-lifetime GTK3 channel as the native UI.
    let (tx, rx) = glib::MainContext::channel::<String>(glib::Priority::DEFAULT);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    {
        let entry = entry.clone();
        let keys = keys.clone();
        rx.attach(None,move |line|{match line.as_str(){"input"=>{println!("{}",serde_json::json!({"submitted":*submitted.borrow(),"text":entry.text().as_str(),"keys":*keys.borrow(),"key_events":*key_events.borrow(),"wheels":*wheels.borrow()}));std::io::stdout().flush().unwrap();},"quit"=>{gtk::main_quit();return glib::ControlFlow::Break;},_=>panic!("unknown fixture command")};glib::ControlFlow::Continue});
    }
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        glib::Propagation::Stop
    });
    window.show_all();
    println!("{}", serde_json::json!({"ready":true}));
    std::io::stdout().flush()?;
    gtk::main();
    Ok(())
}
