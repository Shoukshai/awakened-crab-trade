//! Bind display properties only. Numeric entries, combo IDs, game item data and
//! filter models keep their original values when the interface language changes.
use gtk::prelude::*;
use std::{cell::RefCell, collections::HashMap};
thread_local! {
    static WATCHED: RefCell<HashMap<(usize, &'static str), glib::WeakRef<glib::Object>>> = RefCell::new(HashMap::new());
}
fn apply(object: &glib::Object, property: &'static str) {
    let value = object
        .property::<Option<String>>(property)
        .unwrap_or_default();
    let translated = crate::i18n::tr(&value);
    if value != translated {
        object.set_property(property, translated);
    }
}
fn bind(object: &impl IsA<glib::Object>, property: &'static str) {
    let object = object.as_ref();
    let key = (object.as_ptr() as usize, property);
    let fresh = WATCHED.with(|seen| {
        let mut seen = seen.borrow_mut();
        if seen.get(&key).and_then(glib::WeakRef::upgrade).is_some() {
            false
        } else {
            seen.insert(key, object.downgrade());
            true
        }
    });
    if fresh {
        object.connect_notify_local(Some(property), move |object, _| apply(object, property));
    }
    apply(object, property);
}
fn visit(widget: &gtk::Widget) {
    bind(widget, "tooltip-text");
    if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        bind(label, "label");
    }
    if let Some(entry) = widget.downcast_ref::<gtk::Entry>() {
        bind(entry, "placeholder-text");
    }
    if let Some(window) = widget.downcast_ref::<gtk::Window>() {
        bind(window, "title");
    }
    if let Some(combo) = widget.downcast_ref::<gtk::ComboBox>() {
        for cell in combo.cells() {
            if let Ok(cell) = cell.downcast::<gtk::CellRendererText>() {
                bind(&cell, "text");
            }
        }
        combo.queue_draw();
    }
    if let Some(tree) = widget.downcast_ref::<gtk::TreeView>() {
        for column in tree.columns() {
            bind(&column, "title");
            for cell in column.cells() {
                if let Ok(cell) = cell.downcast::<gtk::CellRendererText>() {
                    bind(&cell, "text");
                }
            }
        }
    }
    if let Some(area) = widget.downcast_ref::<gtk::DrawingArea>() {
        if let Some(accessible) = area.accessible() {
            if accessible
                .property::<Option<String>>("accessible-name")
                .is_some()
            {
                bind(&accessible, "accessible-name");
            }
        }
    }
    if let Some(container) = widget.downcast_ref::<gtk::Container>() {
        container.forall(visit);
    }
}
pub fn refresh(root: &impl IsA<gtk::Widget>) {
    WATCHED.with(|seen| {
        seen.borrow_mut()
            .retain(|_, object| object.upgrade().is_some())
    });
    visit(root.as_ref());
    let objects = WATCHED.with(|seen| {
        seen.borrow()
            .iter()
            .filter_map(|((_, prop), object)| object.upgrade().map(|object| (object, *prop)))
            .collect::<Vec<_>>()
    });
    for (object, prop) in objects {
        apply(&object, prop);
    }
}
