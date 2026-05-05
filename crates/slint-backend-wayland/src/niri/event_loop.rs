use crate::{
    event_loop::WaylandPlatform,
    niri::{
        Niri, NiriConnection, NiriEvent, NiriEventSource, WindowId,
        event_loop::global::SlintWorkspace,
    },
    wayland::ClientState,
};
use calloop::LoopHandle;
use niri_ipc::{KeyboardLayouts, Workspace};
use slint::Model;

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
            NiriEvent::WorkspacesChanged { mut workspaces } => {
                workspaces.sort_by_key(|w| w.idx);
                workspaces.sort_by(|lhs, rhs| lhs.output.cmp(&rhs.output));

                global::update_workspaces(&niri.workspaces, &workspaces, &niri.workspaces_model);

                niri.workspaces = workspaces;
            }
            NiriEvent::WorkspaceActivated { id, focused } => {
                let output = niri
                    .workspaces
                    .iter()
                    .find_map(|w| (w.id == id).then(|| w.output.clone()))
                    .expect("niri behaves sound");

                let update = |i: usize, workspace: &Workspace| {
                    let value = SlintWorkspace::from_niri(workspace).to_slint();
                    niri.workspaces_model.set_row_data(i, value);
                };

                for (i, workspace) in niri.workspaces.iter_mut().enumerate() {
                    if workspace.is_active && workspace.output == output {
                        workspace.is_active = false;
                        update(i, workspace);
                    }

                    if workspace.id == id {
                        workspace.is_active = true;
                        update(i, workspace);
                    }
                }

                if focused {
                    for (i, workspace) in niri.workspaces.iter_mut().enumerate() {
                        if workspace.is_focused {
                            workspace.is_focused = false;
                            update(i, workspace);
                        }

                        if workspace.id == id {
                            workspace.is_focused = true;
                            update(i, workspace);
                        }
                    }
                }
            }
            NiriEvent::WorkspaceUrgencyChanged { id, urgent } => {
                for (i, workspace) in niri.workspaces.iter_mut().enumerate() {
                    if workspace.id == id {
                        workspace.is_urgent = urgent;

                        let value = SlintWorkspace::from_niri(workspace).to_slint();
                        niri.workspaces_model.set_row_data(i, value);
                    }
                }
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
    use crate::{
        instance,
        niri::diff::{self, Edit},
    };
    use niri_ipc::Workspace;
    use slint::{Model, SharedString, VecModel};
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

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SlintWorkspace<'s> {
        pub id: u64,
        pub index: usize,
        pub name: Option<&'s str>,
        pub output: Option<&'s str>,
        pub is_active: bool,
        pub is_focused: bool,
        pub is_urgent: bool,
    }

    impl<'s> SlintWorkspace<'s> {
        pub fn from_niri(niri: &'s Workspace) -> Self {
            Self {
                id: niri.id,
                index: niri.idx as usize,
                name: niri.name.as_deref(),
                output: niri.output.as_deref(),
                is_active: niri.is_active,
                is_focused: niri.is_focused,
                is_urgent: niri.is_urgent,
            }
        }

        pub fn to_slint(self) -> Value {
            Value::Struct(Struct::from_iter([
                (
                    "id".to_owned(),
                    Value::String(SharedString::from(self.id.to_string())),
                ),
                (
                    "index".to_owned(),
                    Value::String(SharedString::from(self.index.to_string())),
                ),
                (
                    "name".to_owned(),
                    Value::String(SharedString::from(self.name.unwrap_or_default())),
                ),
                (
                    "output".to_owned(),
                    Value::String(SharedString::from(self.output.unwrap_or_default())),
                ),
                ("is-active".to_owned(), Value::Bool(self.is_active)),
                ("is-focused".to_owned(), Value::Bool(self.is_focused)),
                ("is-urgent".to_owned(), Value::Bool(self.is_urgent)),
            ]))
        }
    }

    /// # Note
    ///
    /// Both `old_workspaces` and `new_workspaces` must be sorted first by id then by output
    pub fn update_workspaces(
        old_workspaces: &[Workspace],
        new_workspaces: &[Workspace],
        model: &VecModel<Value>,
    ) {
        let diff = diff::difference_by(old_workspaces, new_workspaces, |a, b| {
            SlintWorkspace::from_niri(a) == SlintWorkspace::from_niri(b)
        });

        for (i, &diff) in diff.iter().enumerate() {
            match diff {
                Edit::Equal(_) => continue,
                Edit::Insert(workspace) => {
                    let value = SlintWorkspace::from_niri(workspace).to_slint();
                    model.insert(i, value);
                }
                Edit::Remove(_) => {
                    model.remove(i);
                }
                Edit::Replace(workspace) => {
                    let value = SlintWorkspace::from_niri(workspace).to_slint();
                    model.set_row_data(i, value);
                }
            }
        }
    }
}
