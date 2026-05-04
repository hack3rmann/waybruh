use crate::{
    event_loop::WaylandPlatform,
    niri::{Niri, NiriConnection, NiriEvent, NiriEventSource, WindowId, WorkspaceId},
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
            NiriEvent::WorkspacesChanged { workspaces } => {
                niri.workspaces = workspaces
                    .into_iter()
                    .map(|w| (WorkspaceId(w.id), w))
                    .collect();

                global::set_workspaces(niri.workspaces.values());
            }
            NiriEvent::WorkspaceActivated { id, focused } => {
                let output = niri.workspaces[&WorkspaceId(id)].output.as_deref();

                if let Some(prev_id) = niri.workspaces.values().find_map(|w| {
                    (w.output.as_deref() == output && w.is_active).then_some(WorkspaceId(w.id))
                }) {
                    niri.workspaces.get_mut(&prev_id).unwrap().is_active = false;
                }

                niri.workspaces.get_mut(&WorkspaceId(id)).unwrap().is_active = true;

                if focused {
                    for workspace in niri.workspaces.values_mut() {
                        workspace.is_focused = false;
                    }

                    niri.workspaces
                        .get_mut(&WorkspaceId(id))
                        .unwrap()
                        .is_focused = true;
                }

                global::set_workspaces(niri.workspaces.values());
            }
            NiriEvent::WorkspaceUrgencyChanged { id, urgent } => {
                niri.workspaces.get_mut(&WorkspaceId(id)).unwrap().is_urgent = urgent;

                global::set_workspaces(niri.workspaces.values());
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
    use niri_ipc::Workspace;
    use slint::{ModelRc, SharedString, VecModel};
    use slint_interpreter::{Struct, Value};

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

    pub fn set_workspaces<'w>(workspaces: impl IntoIterator<Item = &'w Workspace>) {
        let mut workspaces = workspaces.into_iter().collect::<Vec<_>>();
        workspaces.sort_by_key(|w| w.idx);
        workspaces.sort_by_key(|w| w.output.as_deref());

        let value = workspaces
            .into_iter()
            .map(|w| {
                Value::Struct(Struct::from_iter([
                    (
                        "index".to_owned(),
                        Value::String(SharedString::from(w.idx.to_string())),
                    ),
                    (
                        "name".to_owned(),
                        Value::String(SharedString::from(w.name.as_deref().unwrap_or_default())),
                    ),
                    (
                        "output".to_owned(),
                        Value::String(SharedString::from(w.output.as_deref().unwrap_or_default())),
                    ),
                    ("is-active".to_owned(), Value::Bool(w.is_active)),
                    ("is-focused".to_owned(), Value::Bool(w.is_focused)),
                    ("is-urgent".to_owned(), Value::Bool(w.is_urgent)),
                ]))
            })
            .collect::<Vec<_>>();

        let value = Value::Model(ModelRc::new(VecModel::from(value)));

        instance::set_global_property("Niri", "workspaces", value).unwrap();
    }
}
