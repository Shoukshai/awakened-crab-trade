//! Win32 window placement and input policy for the shared GTK interface.
use glib::translate::ToGlibPtr;
use gtk::{gdk, prelude::*};
use std::{cell::RefCell, collections::HashMap};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
#[link(name = "gdk-3")]
extern "C" {
    fn gdk_win32_window_get_handle(window: *mut gdk::ffi::GdkWindow) -> HWND;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Overlay,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardMode {
    None,
    OnDemand,
}
#[derive(Clone)]
struct State {
    monitor: Option<gdk::Monitor>,
    right: bool,
    left: i32,
    top: i32,
    bottom: i32,
    mode: KeyboardMode,
    input: Option<bool>,
    backdrop: bool,
    last: Option<(i32, i32, i32, i32, u32)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            monitor: None,
            right: false,
            left: 0,
            top: 0,
            bottom: 0,
            mode: KeyboardMode::None,
            input: None,
            backdrop: false,
            last: None,
        }
    }
}
thread_local! {static WINDOWS:RefCell<HashMap<usize,State>>=RefCell::new(HashMap::new());}
fn id(w: &gtk::Window) -> usize {
    w.as_ptr() as usize
}
fn change(w: &gtk::Window, update: impl FnOnce(&mut State)) {
    WINDOWS.with(|m| update(m.borrow_mut().entry(id(w)).or_default()));
    apply(w);
}
pub fn handle(w: &gtk::Window) -> Option<HWND> {
    let window = w.window()?;
    let raw = unsafe { gdk_win32_window_get_handle(window.to_glib_none().0) };
    (!raw.is_null()).then_some(raw)
}
fn apply(w: &gtk::Window) {
    let Some(state) = WINDOWS.with(|m| m.borrow().get(&id(w)).cloned()) else {
        return;
    };
    let Some(monitor) = state.monitor else {
        return;
    };
    let g = monitor.geometry();
    let factor = monitor.scale_factor().max(1);
    let width = w.allocated_width().max(w.width_request()).max(1);
    let height = (g.height() - state.top - state.bottom).max(1);
    if w.allocated_height() != height {
        w.resize(width, height);
    }
    let Some(raw) = handle(w) else {
        return;
    };
    let x = if state.right {
        g.x() + g.width() - width
    } else {
        g.x() + state.left
    };
    let y = g.y() + state.top;
    // Keep GTK/GDK placement in sync; native-only moves are overwritten on map.
    if w.position() != (x, y) {
        w.move_(x, y);
    }
    let transparent = state
        .input
        .map(|input| !input)
        .unwrap_or(state.mode == KeyboardMode::None);
    let mut flags = WS_EX_TOOLWINDOW;
    if transparent {
        flags |= WS_EX_TRANSPARENT;
    }
    if state.mode == KeyboardMode::None || state.backdrop {
        flags |= WS_EX_NOACTIVATE;
    }
    if let Some(surface) = w.window() {
        surface.set_pass_through(transparent);
    }
    let rect = (
        x * factor,
        y * factor,
        width * factor,
        height * factor,
        flags,
    );
    let managed = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_APPWINDOW | WS_EX_TRANSPARENT;
    let old = unsafe { GetWindowLongPtrW(raw, GWL_EXSTYLE) as u32 };
    if state.last == Some(rect) && old & managed == flags {
        return;
    }
    // Store before invoking Win32: size messages can re-enter GTK allocation.
    WINDOWS.with(|m| {
        if let Some(state) = m.borrow_mut().get_mut(&id(w)) {
            state.last = Some(rect);
        }
    });
    unsafe {
        SetWindowLongPtrW(raw, GWL_EXSTYLE, ((old & !managed) | flags) as isize);

        SetWindowPos(
            raw,
            HWND_TOPMOST,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}
pub fn set_input_enabled(w: &gtk::Window, enabled: bool) {
    change(w, |s| s.input = Some(enabled));
}
pub fn monitor_for_game(display: &gdk::Display, game: Option<u32>) -> Option<gdk::Monitor> {
    let window = game
        .map(crate::desktop::native::hwnd)
        .or_else(crate::desktop::native::focused_target);
    if let Some(window) = window {
        let native = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
        let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if unsafe { GetMonitorInfoW(native, &mut info) } != 0 {
            let best = (0..display.n_monitors())
                .filter_map(|n| display.monitor(n))
                .max_by_key(|monitor| {
                    let g = monitor.geometry();
                    let scale = monitor.scale_factor().max(1);
                    let width = (g.x() * scale + g.width() * scale).min(info.rcMonitor.right)
                        - (g.x() * scale).max(info.rcMonitor.left);
                    let height = (g.y() * scale + g.height() * scale).min(info.rcMonitor.bottom)
                        - (g.y() * scale).max(info.rcMonitor.top);
                    i64::from(width.max(0)) * i64::from(height.max(0))
                });
            if best.is_some() {
                return best;
            }
        }
    }
    display.primary_monitor().or_else(|| display.monitor(0))
}
pub trait LayerShell {
    fn init_layer_shell(&self);
    fn set_namespace(&self, name: &str);
    fn set_layer(&self, layer: Layer);
    fn set_anchor(&self, edge: Edge, anchor: bool);
    fn set_exclusive_zone(&self, zone: i32);
    fn set_monitor(&self, monitor: &gdk::Monitor);
    fn set_keyboard_mode(&self, mode: KeyboardMode);
    fn keyboard_mode(&self) -> KeyboardMode;
    fn set_layer_shell_margin(&self, edge: Edge, margin: i32);
    fn layer_shell_margin(&self, edge: Edge) -> i32;
}
impl LayerShell for gtk::Window {
    fn init_layer_shell(&self) {
        WINDOWS.with(|m| {
            m.borrow_mut().insert(id(self), State::default());
        });
        self.set_accept_focus(false);
        self.set_focus_on_map(false);
        self.set_skip_taskbar_hint(true);
        self.set_skip_pager_hint(true);
        self.set_keep_above(true);
        self.connect_realize(apply);
        self.connect_map(apply);
        self.connect_size_allocate(|w, _| apply(w));
        self.connect_destroy(|w| {
            WINDOWS.with(|m| {
                m.borrow_mut().remove(&id(w));
            });
        });
    }
    fn set_namespace(&self, name: &str) {
        change(self, |s| s.backdrop = name.ends_with("-backdrop"));
    }
    fn set_layer(&self, _: Layer) {}
    fn set_anchor(&self, edge: Edge, anchor: bool) {
        if edge == Edge::Right {
            change(self, |s| s.right = anchor);
        }
    }
    fn set_exclusive_zone(&self, _: i32) {}
    fn set_monitor(&self, monitor: &gdk::Monitor) {
        change(self, |s| s.monitor = Some(monitor.clone()));
    }
    fn set_keyboard_mode(&self, mode: KeyboardMode) {
        let previous = self.keyboard_mode();
        self.set_accept_focus(mode == KeyboardMode::OnDemand);
        change(self, |s| s.mode = mode);
        // Explicit interaction follows first paint. Only activate over the game
        // or our own window; a late capture must never take another app's focus.
        if previous != mode
            && mode == KeyboardMode::OnDemand
            && self.is_visible()
            && (crate::desktop::native::focused_target().is_some()
                || crate::desktop::native::owned_active())
        {
            if let Some(surface) = self.window() {
                surface.focus(gtk::current_event_time());
            }
        }
    }
    fn keyboard_mode(&self) -> KeyboardMode {
        WINDOWS.with(|m| {
            m.borrow()
                .get(&id(self))
                .map(|s| s.mode)
                .unwrap_or(KeyboardMode::None)
        })
    }
    fn set_layer_shell_margin(&self, edge: Edge, margin: i32) {
        change(self, |s| match edge {
            Edge::Left => s.left = margin,
            Edge::Top => s.top = margin,
            Edge::Bottom => s.bottom = margin,
            Edge::Right => {}
        });
    }
    fn layer_shell_margin(&self, edge: Edge) -> i32 {
        WINDOWS.with(|m| {
            m.borrow()
                .get(&id(self))
                .map(|s| match edge {
                    Edge::Left => s.left,
                    Edge::Top => s.top,
                    Edge::Bottom => s.bottom,
                    Edge::Right => 0,
                })
                .unwrap_or(0)
        })
    }
}

/// Reassert topmost once per explicit open, even if a cloud client is topmost.
/// No polling or activation of an unrelated foreground application.
pub fn raise(w: &gtk::Window) {
    if !(crate::desktop::native::focused_target().is_some()
        || crate::desktop::native::owned_active())
    {
        return;
    }
    if let Some(raw) = handle(w) {
        unsafe {
            SetWindowPos(
                raw,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}
