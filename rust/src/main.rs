mod desktop;
mod filters;
mod item;
mod modifiers;
mod overlay;
mod settings;
mod trade;
mod ui;

fn main() {
    if let Err(e) = ui::run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
