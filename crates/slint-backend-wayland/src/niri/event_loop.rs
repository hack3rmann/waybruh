use crate::{
    event_loop::WaylandPlatform,
    niri::{Niri, NiriConnection, NiriEvent, NiriEventSource, WindowId},
    wayland::ClientState,
};
use calloop::LoopHandle;
use niri_ipc::KeyboardLayouts;

impl WaylandPlatform {
    pub fn add_niri_source<'h, 's: 'h>(&'s self, handle: &LoopHandle<'h, ClientState>) {
        if let Ok(conn) = NiriConnection::new() {
            let hyprland_source = NiriEventSource::new(conn);

            handle
                .insert_source(hyprland_source, |event, request, _| {
                    self.handle_niri_event(event, request)
                })
                .unwrap();
        }
    }

    pub fn handle_niri_event(&self, event: NiriEvent, niri: &mut Niri) {
        dbg!(&event);

        match event {
            NiriEvent::WindowsChanged { windows } => {
                if let Some(focused_id) = windows
                    .iter()
                    .find_map(|w| w.is_focused.then_some(WindowId(w.id)))
                {
                    niri.focused_window = Some(focused_id);
                }

                let windows = windows.into_iter().map(|w| (WindowId(w.id), w));
                niri.windows.extend(windows);
            }
            NiriEvent::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    niri.focused_window = Some(WindowId(window.id));
                }

                niri.windows.insert(WindowId(window.id), window);
            }
            NiriEvent::WindowClosed { id } => {
                if let Some(focused_id) = niri.focused_window
                    && focused_id == WindowId(id)
                {
                    niri.focused_window = None;
                }

                niri.windows.remove(&WindowId(id));
            }
            NiriEvent::WindowFocusChanged { id } => {
                niri.focused_window = id.map(WindowId);
            }
            NiriEvent::KeyboardLayoutsChanged {
                keyboard_layouts: KeyboardLayouts { names, current_idx },
            } => {
                niri.keyboard_layouts = names;
                niri.current_keyboard_layout_index = current_idx as usize;

                let layout = niri.keyboard_layouts[niri.current_keyboard_layout_index].as_str();
                global::set_active_keyboard_layout(layout);
            }
            NiriEvent::KeyboardLayoutSwitched { idx } => {
                niri.current_keyboard_layout_index = idx as usize;

                let layout = niri.keyboard_layouts[niri.current_keyboard_layout_index].as_str();
                global::set_active_keyboard_layout(layout);
            }
            NiriEvent::OverviewOpenedOrClosed { is_open } => {
                global::set_overview_opened(is_open);
            }
            _ => {}
        }
    }
}

impl Niri {
    pub fn flush_events(&self) {
        if let Some(focused_id) = self.focused_window
            && let Some(window) = self.windows.get(&focused_id)
        {
            let title = window.title.as_deref().unwrap_or_default();
            let class = window.app_id.as_deref().unwrap_or_default();

            global::set_active_window_title(title);
            global::set_active_window_class(class);
            global::set_active_window_pid(window.pid.unwrap_or(-1))
        } else {
            global::set_active_window_title("");
            global::set_active_window_class("");
            global::set_active_window_pid(-1);
        }
    }
}

mod global {
    use crate::instance;
    use slint::SharedString;
    use slint_interpreter::Value;

    pub fn set_active_keyboard_layout(name: &str) {
        instance::set_global_property(
            "Niri",
            "active-keyboard-layout",
            Value::String(SharedString::from(name)),
        )
        .unwrap();

        instance::set_global_property(
            "ActiveCompositor",
            "active-keyboard-layout",
            Value::String(SharedString::from(name)),
        )
        .unwrap();
    }

    pub fn set_overview_opened(value: bool) {
        instance::set_global_property("Niri", "overview-opened", Value::Bool(value)).unwrap();
    }

    pub fn set_active_window_title(value: &str) {
        instance::set_global_property(
            "Niri",
            "active-window-title",
            Value::String(SharedString::from(value)),
        )
        .unwrap();

        instance::set_global_property(
            "ActiveCompositor",
            "active-window-title",
            Value::String(SharedString::from(value)),
        )
        .unwrap();
    }

    pub fn set_active_window_class(value: &str) {
        instance::set_global_property(
            "Niri",
            "active-window-class",
            Value::String(SharedString::from(value)),
        )
        .unwrap();

        instance::set_global_property(
            "ActiveCompositor",
            "active-window-class",
            Value::String(SharedString::from(value)),
        )
        .unwrap();
    }

    pub fn set_active_window_pid(value: i32) {
        instance::set_global_property(
            "Niri",
            "active-window-pid",
            Value::String(SharedString::from(value.to_string())),
        )
        .unwrap();
    }
}
