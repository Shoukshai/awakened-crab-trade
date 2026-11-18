//! OCR is reviewed before it becomes a Trade item; edits stay local.
use gtk::prelude::*;
pub struct Panel {
    pub container: gtk::Box,
    pub apply: gtk::Button,
    text: gtk::TextView,
    status: gtk::Label,
    unknown: gtk::TextView,
    missing: gtk::Expander,
    diagnostic: gtk::Expander,
    diagnostic_text: gtk::Label,
    copy_diagnostic: gtk::Button,
}
impl Panel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let status = gtk::Label::new(None);
        status.set_xalign(0.);
        status.set_line_wrap(true);
        container.pack_start(&status, false, false, 0);
        let diagnostic = gtk::Expander::new(Some("Diagnostic OCR"));
        diagnostic.set_no_show_all(true);
        let diagnostic_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let diagnostic_text = gtk::Label::new(None);
        diagnostic_text.set_xalign(0.);
        diagnostic_text.set_line_wrap(true);
        diagnostic_text.set_line_wrap_mode(gtk::pango::WrapMode::WordChar);
        diagnostic_text.set_selectable(true);
        diagnostic_box.pack_start(&diagnostic_text, false, false, 0);
        let copy_diagnostic = gtk::Button::with_label("Copier le diagnostic");
        let copied_text = diagnostic_text.clone();
        copy_diagnostic.connect_clicked(move |_| {
            gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(&copied_text.text());
        });
        diagnostic_box.pack_start(&copy_diagnostic, false, false, 0);
        diagnostic_box.show_all();
        diagnostic.add(&diagnostic_box);
        container.pack_start(&diagnostic, false, false, 0);
        let missing = gtk::Expander::new(Some("Lignes non reconnues :"));
        missing.set_no_show_all(true);
        let unknown = gtk::TextView::new();
        unknown.set_editable(false);
        unknown.set_monospace(true);
        unknown.set_wrap_mode(gtk::WrapMode::WordChar);
        let unknown_scroll =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        unknown_scroll.set_min_content_height(96);
        unknown_scroll.set_max_content_height(160);
        unknown_scroll.add(&unknown);
        unknown_scroll.show_all();
        missing.add(&unknown_scroll);
        container.pack_start(&missing, false, false, 0);
        let text = gtk::TextView::new();
        text.set_monospace(true);
        text.set_wrap_mode(gtk::WrapMode::WordChar);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.add(&text);
        container.pack_start(&scroll, true, true, 0);
        let apply = gtk::Button::with_label("Utiliser ces stats");
        apply.style_context().add_class("primary");
        container.pack_start(&apply, false, false, 0);
        Self {
            container,
            apply,
            text,
            status,
            unknown,
            missing,
            diagnostic,
            diagnostic_text,
            copy_diagnostic,
        }
    }
    pub fn show(&self, result: Result<crate::ocr::Draft, String>) {
        match result {
            Ok(d) => {
                self.diagnostic
                    .set_label(Some(&crate::i18n::tr("Diagnostic OCR")));
                self.copy_diagnostic
                    .set_label(&crate::i18n::tr("Copier le diagnostic"));
                self.diagnostic_text
                    .set_text(d.diagnostic.as_deref().unwrap_or_default());
                self.diagnostic.set_expanded(d.diagnostic.is_some());
                self.diagnostic.set_visible(d.diagnostic.is_some());
                self.text.buffer().unwrap().set_text(&d.text);
                self.unknown
                    .buffer()
                    .unwrap()
                    .set_text(&d.unknown.join("\n"));
                self.missing.set_label(Some(&format!(
                    "{} {}",
                    crate::i18n::tr("Lignes non reconnues :"),
                    d.unknown.len()
                )));
                self.missing.set_expanded(false);
                self.missing.set_visible(!d.unknown.is_empty());
                self.status.set_text(&format!("{} — {}",d.engine,crate::i18n::tr("Vérifiez la base, les valeurs et les mods. L’ilvl reste vide s’il n’est pas visible.")));
                self.apply.set_sensitive(true);
            }
            Err(error) => {
                self.diagnostic.hide();
                self.diagnostic_text.set_text("");
                self.text.buffer().unwrap().set_text("");
                self.status.set_text(&error);
                self.missing.set_expanded(false);
                self.missing.hide();
                self.unknown.buffer().unwrap().set_text("");
                self.apply.set_sensitive(false);
            }
        }
        self.container.show_all();
    }
    pub fn read(&self) -> Result<String, String> {
        let b = self.text.buffer().ok_or("OCR text unavailable")?;
        Ok(b.text(&b.start_iter(), &b.end_iter(), false)
            .unwrap_or_default()
            .to_string())
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        serde_json::json!({"mapped":self.container.is_mapped(),"text":self.read().unwrap_or_default(),"status":self.status.text().as_str(),"diagnostic":self.diagnostic_text.text().as_str(),"diagnostic_visible":self.diagnostic.is_visible(),"copy_diagnostic":self.copy_diagnostic.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":self.copy_diagnostic.allocated_width(),"height":self.copy_diagnostic.allocated_height()})),"apply":self.apply.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":self.apply.allocated_width(),"height":self.apply.allocated_height(),"enabled":self.apply.is_sensitive()}))})
    }
    pub fn error(&self, error: &str) {
        self.status.set_text(error);
    }
}
