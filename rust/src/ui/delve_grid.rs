//! Independent layer-shell surface: always input-empty and keyboard None.
use crate::delve::{Geometry, Policy, Reason, BORDER_RGB};
use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub struct Grid {
    window: gtk::Window,
    policy: RefCell<Policy>,
    geometry: RefCell<Option<Geometry>>,
    pending_paint: Cell<Option<u64>>,
    draws: Cell<u64>,
    frames: Cell<u64>,
    error: RefCell<Option<String>>,
    exclusive_suppressed: Cell<bool>,
}
fn empty_input(window: &gtk::Window) {
    window.set_keyboard_mode(KeyboardMode::None);
    if let Some(w) = window.window() {
        w.input_shape_combine_region(&gtk::cairo::Region::create(), 0, 0);
    }
}
fn draw(context: &gtk::cairo::Context, g: &Geometry) -> Result<(), String> {
    context.save().map_err(|e| e.to_string())?;
    let result = (|| {
        context.set_operator(gtk::cairo::Operator::Source);
        context.set_source_rgba(0., 0., 0., 0.);
        context.paint()?;
        context.set_operator(gtk::cairo::Operator::Over);
        context.rectangle(
            g.left,
            f64::from(g.top),
            f64::from(g.width),
            f64::from(g.height),
        );
        context.clip();
        context.set_source_rgba(1., 1., 1., 0.1);
        context.paint()?;
        context.set_source_rgb(
            f64::from(BORDER_RGB[0]) / 255.,
            f64::from(BORDER_RGB[1]) / 255.,
            f64::from(BORDER_RGB[2]) / 255.,
        );
        context.set_line_width(1.);
        if g.cell > 1 {
            for row in 0..g.rows {
                for column in 0..g.columns {
                    context.rectangle(
                        g.grid_left + column as f64 * f64::from(g.cell) + 0.5,
                        g.grid_top + row as f64 * f64::from(g.cell) + 0.5,
                        f64::from(g.cell - 1),
                        f64::from(g.cell - 1),
                    );
                }
            }
            context.stroke()?;
        }
        Ok::<_, gtk::cairo::Error>(())
    })();
    let restored = context.restore();
    result.and(restored).map_err(|e| e.to_string())
}
impl Grid {
    pub fn new() -> Result<Rc<Self>, String> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Awakened Crab Trade — Grille Delve");
        window.set_widget_name("apt-delve");
        let visual = GtkWindowExt::screen(&window)
            .and_then(|s| s.rgba_visual())
            .ok_or("Une surface RGBA est nécessaire à la grille Delve.")?;
        window.set_visual(Some(&visual));
        window.set_app_paintable(true);
        window.set_decorated(false);
        window.set_accept_focus(false);
        window.set_focus_on_map(false);
        window.init_layer_shell();
        window.set_namespace("awakened-poe-trade-rs-delve");
        window.set_layer(Layer::Overlay);
        window.set_exclusive_zone(-1);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
        }
        empty_input(&window);
        let grid = Rc::new(Self {
            window,
            policy: RefCell::new(Policy::default()),
            geometry: RefCell::new(None),
            pending_paint: Cell::new(None),
            draws: Cell::new(0),
            frames: Cell::new(0),
            error: RefCell::new(None),
            exclusive_suppressed: Cell::new(false),
        });
        let weak = Rc::downgrade(&grid);
        grid.window.connect_realize(move |w| {
            empty_input(w);
            if let Some(clock) = w.frame_clock() {
                let weak = weak.clone();
                clock.connect_after_paint(move |_| {
                    if let Some(g) = weak.upgrade() {
                        if let Some(token) = g.pending_paint.take() {
                            if g.window.is_visible() && g.policy.borrow_mut().painted(token) {
                                g.frames.set(g.frames.get() + 1);
                                eprintln!(
                                    "delve frame={} generation={token} keyboard=None input=empty",
                                    g.frames.get()
                                );
                            }
                        }
                    }
                });
            }
        });
        grid.window.connect_size_allocate(|w, _| empty_input(w));
        grid.window.connect_map_event(|w, _| {
            empty_input(w);
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&grid);
        grid.window.connect_draw(move |w, context| {
            let Some(g) = weak.upgrade() else {
                return glib::Propagation::Stop;
            };
            if !w.is_visible() || !g.policy.borrow().requested() {
                return glib::Propagation::Stop;
            }
            let token = g.policy.borrow().generation;
            let geometry = Geometry::new(w.allocated_width(), w.allocated_height());
            match geometry.and_then(|geo| {
                draw(context, &geo)?;
                Ok(geo)
            }) {
                Ok(geo) => {
                    g.geometry.replace(Some(geo));
                    g.draws.set(g.draws.get() + 1);
                    g.pending_paint.set(Some(token));
                }
                Err(error) => {
                    eprintln!("delve draw error={error}");
                    g.error.replace(Some(error));
                    g.policy.borrow_mut().fail();
                    empty_input(w);
                    w.hide();
                }
            }
            glib::Propagation::Stop
        });
        let weak = Rc::downgrade(&grid);
        grid.window.connect_focus_in_event(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.hide(Reason::Interactive);
                eprintln!("delve unexpected focus: surface hidden");
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&grid);
        grid.window.connect_delete_event(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.hide(Reason::Explicit);
            }
            glib::Propagation::Stop
        });
        Ok(grid)
    }
    pub fn requested(&self) -> bool {
        self.policy.borrow().requested()
    }
    pub fn toggle(&self, allowed: bool, wait_for_game: bool) {
        let requested = self.policy.borrow_mut().toggle(allowed, wait_for_game);
        if requested {
            self.error.replace(None);
        } else {
            self.pending_paint.set(None);
            empty_input(&self.window);
            self.window.hide();
        }
        eprintln!(
            "delve toggle requested={requested} allowed={allowed} pending_game={wait_for_game}"
        );
    }
    pub fn update(
        &self,
        present: bool,
        active: bool,
        other: bool,
        interactive: bool,
        exclusive: bool,
    ) {
        let show = self
            .policy
            .borrow_mut()
            .context(present, active, other, interactive, exclusive);
        self.exclusive_suppressed
            .set(exclusive && self.policy.borrow().requested());
        if show && !self.window.is_visible() {
            empty_input(&self.window);
            self.window.show_all();
            self.window.queue_draw();
        } else if !show && self.window.is_visible() {
            self.pending_paint.set(None);
            empty_input(&self.window);
            self.window.hide();
        }
    }
    pub fn hide(&self, reason: Reason) {
        self.exclusive_suppressed.set(false);
        self.policy.borrow_mut().hide(reason);
        self.pending_paint.set(None);
        empty_input(&self.window);
        self.window.hide();
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"available":true,"policy":*self.policy.borrow(),"visible":self.window.is_visible(),"mapped":self.window.is_mapped(),"keyboard_mode":format!("{:?}",self.window.keyboard_mode()),"input_empty":true,"exclusive_suppressed":self.exclusive_suppressed.get(),"focused":self.window.is_active(),"geometry":*self.geometry.borrow(),"draws":self.draws.get(),"frames":self.frames.get(),"buffer_scale":self.window.scale_factor(),"error":*self.error.borrow()})
    }
}
