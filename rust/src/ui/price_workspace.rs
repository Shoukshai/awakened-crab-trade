//! Reparent existing controls to switch layouts without rebuilding filter models.
use crate::appearance::Theme;
use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};
pub struct Workspace {
    pub container: gtk::Box,
    body: gtk::Box,
    left: gtk::Box,
    notebook: gtk::Notebook,
    filters: gtk::ScrolledWindow,
    offers: gtk::ScrolledWindow,
    actions: gtk::Box,
    markers: Vec<gtk::Label>,
    theme: Rc<Cell<Theme>>,
    scale: Rc<Cell<f64>>,
    currency: Cell<bool>,
    applied: Cell<bool>,
}
fn detach(widget: &impl IsA<gtk::Widget>) {
    if let Some(parent) = widget
        .parent()
        .and_then(|p| p.downcast::<gtk::Container>().ok())
    {
        parent.remove(widget);
    }
}
impl Workspace {
    pub fn new(
        filters: &gtk::ScrolledWindow,
        offers: &gtk::ScrolledWindow,
        actions: &gtk::Box,
    ) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let theme = Rc::new(Cell::new(Theme::Tide));
        let scale = Rc::new(Cell::new(1.0));
        let mode = theme.clone();
        let zoom = scale.clone();
        body.connect_size_allocate(move |body, a| {
            if mode.get() == Theme::Split {
                let wide = f64::from(a.width()) >= 560.0 * zoom.get();
                let orientation = if wide {
                    gtk::Orientation::Horizontal
                } else {
                    gtk::Orientation::Vertical
                };
                if body.orientation() != orientation {
                    body.set_orientation(orientation);
                    body.set_homogeneous(wide);
                }
            }
        });
        let left = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let notebook = gtk::Notebook::new();
        notebook.set_scrollable(true);
        notebook.style_context().add_class("workspace-tabs");
        let markers = ["02 · Filtres", "03 · Offres"]
            .into_iter()
            .map(|text| {
                let label = gtk::Label::new(Some(text));
                label.set_xalign(0.);
                label.style_context().add_class("section-marker");
                label.set_no_show_all(true);
                label
            })
            .collect();
        Self {
            container,
            body,
            left,
            notebook,
            filters: filters.clone(),
            offers: offers.clone(),
            actions: actions.clone(),
            markers,
            theme,
            scale,
            currency: Cell::new(false),
            applied: Cell::new(false),
        }
    }
    pub fn apply(&self, theme: Theme, scale: f64) {
        if self.applied.replace(true) && self.theme.get() == theme && self.scale.get() == scale {
            return;
        }
        self.theme.set(theme);
        self.scale.set(scale);
        for w in [
            self.filters.upcast_ref::<gtk::Widget>(),
            self.offers.upcast_ref(),
            self.actions.upcast_ref(),
            self.left.upcast_ref(),
            self.notebook.upcast_ref(),
            self.body.upcast_ref(),
        ] {
            detach(w);
        }
        for marker in &self.markers {
            detach(marker);
        }
        self.body.set_orientation(if theme == Theme::Split {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        });
        self.body.set_homogeneous(theme == Theme::Split);
        self.body.set_spacing(theme.spacing());
        self.left.set_spacing(theme.spacing());
        self.container.set_spacing(theme.spacing());
        let expanded = matches!(theme, Theme::Split | Theme::Focus);
        self.filters.set_vexpand(expanded);
        self.filters.set_propagate_natural_height(!expanded);
        self.filters
            .set_min_content_height(if expanded { 100 } else { 160 });
        self.offers.set_min_content_height(140);
        self.offers
            .set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        self.offers.set_hexpand(true);
        self.filters.set_hexpand(true);
        if theme == Theme::Focus {
            while self.notebook.n_pages() > 0 {
                self.notebook.remove_page(Some(0));
            }
            self.left.pack_start(&self.filters, true, true, 0);
            self.left.pack_start(&self.actions, false, false, 0);
            self.notebook
                .append_page(&self.left, Some(&gtk::Label::new(Some("Filtres"))));
            self.notebook
                .append_page(&self.offers, Some(&gtk::Label::new(Some("Offres"))));
            self.container.pack_start(&self.notebook, true, true, 0);
            self.notebook.set_current_page(Some(0));
        } else if theme == Theme::Split {
            self.left.pack_start(&self.filters, true, true, 0);
            self.left.pack_start(&self.actions, false, false, 0);
            self.body.pack_start(&self.left, true, true, 0);
            self.body.pack_start(&self.offers, true, true, 0);
            self.container.pack_start(&self.body, true, true, 0);
        } else {
            if theme == Theme::Blueprint {
                self.body.pack_start(&self.markers[0], false, false, 0);
            }
            self.body.pack_start(&self.filters, false, false, 0);
            if theme != Theme::Compact {
                self.body.pack_start(&self.actions, false, false, 0);
            }
            if theme == Theme::Blueprint {
                self.body.pack_start(&self.markers[1], false, false, 0);
            }
            self.body.pack_start(&self.offers, true, true, 0);
            self.container.pack_start(&self.body, true, true, 0);
            if theme == Theme::Compact {
                self.container.pack_start(&self.actions, false, false, 0);
            }
        }
        self.container.show_all();
        for marker in &self.markers {
            marker.set_visible(theme == Theme::Blueprint);
        }
        self.set_currency(self.currency.get());
    }
    pub fn set_currency(&self, currency: bool) {
        self.currency.set(currency);
        self.container.set_no_show_all(currency);
        self.container.set_visible(!currency);
    }
    pub fn copied(&self) {
        self.notebook.set_current_page(Some(0));
    }
    pub fn searched(&self) {
        if self.theme.get() == Theme::Focus {
            self.notebook.set_current_page(Some(1));
        }
    }
    pub fn diagnostics(&self, window: &gtk::Window) -> serde_json::Value {
        let rect = |w: &gtk::Widget| {
            w.translate_coordinates(window,0,0).map(|(x,y)|serde_json::json!({"x":x,"y":y,"width":w.allocated_width(),"height":w.allocated_height(),"mapped":w.is_mapped()}))
        };
        serde_json::json!({"orientation":format!("{:?}",self.body.orientation()),"layout":self.theme.get().id(),"page":self.notebook.current_page(),"tabs":(0..self.notebook.n_pages()).map(|i|self.notebook.nth_page(Some(i)).and_then(|c|self.notebook.tab_label(&c)).and_then(|w|rect(&w))).collect::<Vec<_>>(),"filters":rect(self.filters.upcast_ref()),"offers":rect(self.offers.upcast_ref()),"actions":rect(self.actions.upcast_ref()),"visible":self.container.is_visible()})
    }
}
