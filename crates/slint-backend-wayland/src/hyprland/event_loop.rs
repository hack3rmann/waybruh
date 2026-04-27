use crate::{
    event_loop::WaylandPlatform,
    hyprland::{
        HyprlandConnection, HyprlandEventSource,
        event::{Fullscreen, HyprlandEvent},
    },
    instance,
    wayland::ClientState,
};
use calloop::LoopHandle;
use slint_interpreter::Value;

impl WaylandPlatform {
    pub fn add_hyprland_source<'h, 's: 'h>(&'s self, handle: &LoopHandle<'h, ClientState>) {
        if let Ok(hyprland_conn) = HyprlandConnection::new() {
            let hyprland_source = HyprlandEventSource::new(hyprland_conn);

            handle
                .insert_source(hyprland_source, |event, _, _| {
                    self.handle_hyprland_event(event)
                })
                .unwrap();
        }
    }

    pub fn handle_hyprland_event(&self, event: HyprlandEvent) {
        use slint::SharedString;

        match event {
            HyprlandEvent::ActiveWindow {
                window_title,
                window_class,
            } => {
                instance::set_global_property(
                    "Hyprland",
                    "active-window-title",
                    Value::String(SharedString::from(window_title)),
                )
                .unwrap();

                instance::set_global_property(
                    "Hyprland",
                    "active-window-class",
                    Value::String(SharedString::from(window_class)),
                )
                .unwrap();
            }
            HyprlandEvent::ActiveWindowV2 { window_address } => {
                instance::set_global_property(
                    "Hyprland",
                    "active-window-address",
                    Value::String(SharedString::from(window_address)),
                )
                .unwrap();
            }
            HyprlandEvent::Workspace { name } => {
                instance::set_global_property(
                    "Hyprland",
                    "active-workspace",
                    Value::String(SharedString::new()),
                )
                .unwrap();

                instance::set_global_property(
                    "Hyprland",
                    "active-workspace-name",
                    Value::String(SharedString::from(name)),
                )
                .unwrap();
            }
            HyprlandEvent::WorkspaceV2 { id, name } => {
                instance::set_global_property(
                    "Hyprland",
                    "active-workspace-id",
                    Value::String(SharedString::from(id)),
                )
                .unwrap();

                instance::set_global_property(
                    "Hyprland",
                    "active-workspace-name",
                    Value::String(SharedString::from(name)),
                )
                .unwrap();
            }
            HyprlandEvent::Fullscreen(mode) => {
                let is_fullscreen = match mode {
                    Fullscreen::Exit => false,
                    Fullscreen::Enter => true,
                };

                instance::set_global_property(
                    "Hyprland",
                    "entered-fullscreen",
                    Value::Bool(is_fullscreen),
                )
                .unwrap();
            }
            event => {
                dbg!(event);
            }
        }
    }
}
