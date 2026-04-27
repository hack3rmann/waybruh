use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fullscreen {
    Exit = 0,
    Enter = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToggleGroupState {
    Off = 0,
    On = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IgnoreGroupLockState {
    Off = 0,
    On = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LockGroupsState {
    Off = 0,
    On = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MinimizedState {
    Unminimized = 0,
    Minimized = 1,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HyprlandEvent {
    Workspace {
        name: String,
    },
    WorkspaceV2 {
        id: String,
        name: String,
    },
    FocusedMon {
        mon: String,
        workspace: String,
    },
    FocusedMonV2 {
        mon: String,
        workspace_id: String,
    },
    ActiveWindow {
        window_class: String,
        window_title: String,
    },
    ActiveWindowV2 {
        window_address: String,
    },
    Fullscreen(Fullscreen),
    MonitorRemoved {
        monitor_name: String,
    },
    MonitorRemovedV2 {
        monitor_id: String,
        monitor_name: String,
        monitor_description: String,
    },
    MonitorAdded {
        monitor_name: String,
    },
    MonitorAddedV2 {
        monitor_id: String,
        monitor_name: String,
        monitor_description: String,
    },
    CreateWorkspace {
        name: String,
    },
    CreateWorkspaceV2 {
        id: String,
        name: String,
    },
    DestroyWorkspace {
        name: String,
    },
    DestroyWorkspaceV2 {
        id: String,
        name: String,
    },
    MoveWorkspace {
        workspace: String,
        mon: String,
    },
    MoveWorkspaceV2 {
        workspace_id: String,
        workspace: String,
        mon: String,
    },
    RenameWorkspace {
        workspace_id: String,
        new_name: String,
    },
    ActiveSpecial {
        workspace: String,
        mon: String,
    },
    ActiveSpecialV2 {
        workspace_id: String,
        workspace: String,
        mon: String,
    },
    ActiveLayout {
        keyboard_name: String,
        layout_name: String,
    },
    OpenWindow {
        window_address: String,
        workspace: String,
        window_class: String,
        window_title: String,
    },
    CloseWindow {
        window_address: String,
    },
    Kill {
        window_address: String,
    },
    MoveWindow {
        window_address: String,
        workspace: String,
    },
    MoveWindowV2 {
        window_address: String,
        workspace_id: String,
        workspace: String,
    },
    OpenLayer {
        namespace: String,
    },
    CloseLayer {
        namespace: String,
    },
    Submap {
        submap_name: String,
    },
    ChangeFloatingMode {
        window_address: String,
        floating: String,
    },
    Urgent {
        window_address: String,
    },
    Screencast {
        state: String,
        owner: String,
    },
    ScreencastV2 {
        state: String,
        owner: String,
        name: String,
    },
    WindowTitle {
        window_address: String,
    },
    WindowTitleV2 {
        window_address: String,
        window_title: String,
    },
    ToggleGroup {
        state: ToggleGroupState,
        window_addresses: Vec<String>,
    },
    MoveIntoGroup {
        window_address: String,
    },
    MoveOutOfGroup {
        window_address: String,
    },
    IgnoreGroupLock {
        state: IgnoreGroupLockState,
    },
    LockGroups {
        state: LockGroupsState,
    },
    ConfigReloaded,
    Pin {
        window_address: String,
        pin_state: String,
    },
    Minimized {
        window_address: String,
        state: MinimizedState,
    },
    Bell {
        window_address: String,
    },
}

impl FromStr for HyprlandEvent {
    type Err = HyprlandEventParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, data) = s.split_once(">>").ok_or(HyprlandEventParseError)?;

        Ok(match name {
            "workspace" => Self::Workspace {
                name: data.to_owned(),
            },
            "workspacev2" => {
                let (id, name) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::WorkspaceV2 {
                    id: id.to_owned(),
                    name: name.to_owned(),
                }
            }
            "focusedmon" => {
                let (mon, workspace) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::FocusedMon {
                    mon: mon.to_owned(),
                    workspace: workspace.to_owned(),
                }
            }
            "focusedmonv2" => {
                let (mon, workspace_id) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::FocusedMonV2 {
                    mon: mon.to_owned(),
                    workspace_id: workspace_id.to_owned(),
                }
            }
            "activewindow" => {
                let (window_class, window_title) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::ActiveWindow {
                    window_class: window_class.to_owned(),
                    window_title: window_title.to_owned(),
                }
            }
            "activewindowv2" => Self::ActiveWindowV2 {
                window_address: data.to_owned(),
            },
            "fullscreen" => {
                let fullscreen = match data {
                    "0" => Fullscreen::Exit,
                    "1" => Fullscreen::Enter,
                    _ => return Err(HyprlandEventParseError),
                };

                Self::Fullscreen(fullscreen)
            }
            "monitorremoved" => Self::MonitorRemoved {
                monitor_name: data.to_owned(),
            },
            "monitorremovedv2" => {
                let mut it = data.splitn(3, ',');
                let monitor_id = it.next().ok_or(HyprlandEventParseError)?;
                let monitor_name = it.next().ok_or(HyprlandEventParseError)?;
                let monitor_description = it.next().ok_or(HyprlandEventParseError)?;

                Self::MonitorRemovedV2 {
                    monitor_id: monitor_id.to_owned(),
                    monitor_name: monitor_name.to_owned(),
                    monitor_description: monitor_description.to_owned(),
                }
            }
            "monitoradded" => Self::MonitorAdded {
                monitor_name: data.to_owned(),
            },
            "monitoraddedv2" => {
                let mut it = data.splitn(3, ',');
                let monitor_id = it.next().ok_or(HyprlandEventParseError)?;
                let monitor_name = it.next().ok_or(HyprlandEventParseError)?;
                let monitor_description = it.next().ok_or(HyprlandEventParseError)?;

                Self::MonitorAddedV2 {
                    monitor_id: monitor_id.to_owned(),
                    monitor_name: monitor_name.to_owned(),
                    monitor_description: monitor_description.to_owned(),
                }
            }
            "createworkspace" => Self::CreateWorkspace {
                name: data.to_owned(),
            },
            "createworkspacev2" => {
                let (id, name) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::CreateWorkspaceV2 {
                    id: id.to_owned(),
                    name: name.to_owned(),
                }
            }
            "destroyworkspace" => Self::DestroyWorkspace {
                name: data.to_owned(),
            },
            "destroyworkspacev2" => {
                let (id, name) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::DestroyWorkspaceV2 {
                    id: id.to_owned(),
                    name: name.to_owned(),
                }
            }
            "moveworkspace" => {
                let (workspace, mon) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::MoveWorkspace {
                    workspace: workspace.to_owned(),
                    mon: mon.to_owned(),
                }
            }
            "moveworkspacev2" => {
                let mut it = data.splitn(3, ',');
                let workspace_id = it.next().ok_or(HyprlandEventParseError)?;
                let workspace = it.next().ok_or(HyprlandEventParseError)?;
                let mon = it.next().ok_or(HyprlandEventParseError)?;

                Self::MoveWorkspaceV2 {
                    workspace_id: workspace_id.to_owned(),
                    workspace: workspace.to_owned(),
                    mon: mon.to_owned(),
                }
            }
            "renameworkspace" => {
                let (workspace_id, new_name) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::RenameWorkspace {
                    workspace_id: workspace_id.to_owned(),
                    new_name: new_name.to_owned(),
                }
            }
            "activespecial" => {
                let (workspace, mon) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::ActiveSpecial {
                    workspace: workspace.to_owned(),
                    mon: mon.to_owned(),
                }
            }
            "activespecialv2" => {
                let mut it = data.splitn(3, ',');
                let workspace_id = it.next().ok_or(HyprlandEventParseError)?;
                let workspace = it.next().ok_or(HyprlandEventParseError)?;
                let mon = it.next().ok_or(HyprlandEventParseError)?;

                Self::ActiveSpecialV2 {
                    workspace_id: workspace_id.to_owned(),
                    workspace: workspace.to_owned(),
                    mon: mon.to_owned(),
                }
            }
            "activelayout" => {
                let (keyboard_name, layout_name) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::ActiveLayout {
                    keyboard_name: keyboard_name.to_owned(),
                    layout_name: layout_name.to_owned(),
                }
            }
            "openwindow" => {
                let mut it = data.splitn(4, ',');
                let window_address = it.next().ok_or(HyprlandEventParseError)?;
                let workspace = it.next().ok_or(HyprlandEventParseError)?;
                let window_class = it.next().ok_or(HyprlandEventParseError)?;
                let window_title = it.next().ok_or(HyprlandEventParseError)?;

                Self::OpenWindow {
                    window_address: window_address.to_owned(),
                    workspace: workspace.to_owned(),
                    window_class: window_class.to_owned(),
                    window_title: window_title.to_owned(),
                }
            }
            "closewindow" => Self::CloseWindow {
                window_address: data.to_owned(),
            },
            "kill" => Self::Kill {
                window_address: data.to_owned(),
            },
            "movewindow" => {
                let (window_address, workspace) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::MoveWindow {
                    window_address: window_address.to_owned(),
                    workspace: workspace.to_owned(),
                }
            }
            "movewindowv2" => {
                let mut it = data.splitn(3, ',');
                let window_address = it.next().ok_or(HyprlandEventParseError)?;
                let workspace_id = it.next().ok_or(HyprlandEventParseError)?;
                let workspace = it.next().ok_or(HyprlandEventParseError)?;

                Self::MoveWindowV2 {
                    window_address: window_address.to_owned(),
                    workspace_id: workspace_id.to_owned(),
                    workspace: workspace.to_owned(),
                }
            }
            "openlayer" => Self::OpenLayer {
                namespace: data.to_owned(),
            },
            "closelayer" => Self::CloseLayer {
                namespace: data.to_owned(),
            },
            "submap" => Self::Submap {
                submap_name: data.to_owned(),
            },
            "changefloatingmode" => {
                let (window_address, floating) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::ChangeFloatingMode {
                    window_address: window_address.to_owned(),
                    floating: floating.to_owned(),
                }
            }
            "urgent" => Self::Urgent {
                window_address: data.to_owned(),
            },
            "screencast" => {
                let (state, owner) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::Screencast {
                    state: state.to_owned(),
                    owner: owner.to_owned(),
                }
            }
            "screencastv2" => {
                let mut it = data.splitn(3, ',');
                let state = it.next().ok_or(HyprlandEventParseError)?;
                let owner = it.next().ok_or(HyprlandEventParseError)?;
                let name = it.next().ok_or(HyprlandEventParseError)?;

                Self::ScreencastV2 {
                    state: state.to_owned(),
                    owner: owner.to_owned(),
                    name: name.to_owned(),
                }
            }
            "windowtitle" => Self::WindowTitle {
                window_address: data.to_owned(),
            },
            "windowtitlev2" => {
                let (window_address, window_title) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::WindowTitleV2 {
                    window_address: window_address.to_owned(),
                    window_title: window_title.to_owned(),
                }
            }
            "togglegroup" => {
                let (state, window_addresses_string) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                let state = match state {
                    "0" => ToggleGroupState::Off,
                    "1" => ToggleGroupState::On,
                    _ => return Err(HyprlandEventParseError),
                };

                let window_addresses = window_addresses_string
                    .split(',')
                    .map(str::to_owned)
                    .collect();

                Self::ToggleGroup {
                    state,
                    window_addresses,
                }
            }
            "moveintogroup" => Self::MoveIntoGroup {
                window_address: data.to_owned(),
            },
            "moveoutofgroup" => Self::MoveOutOfGroup {
                window_address: data.to_owned(),
            },
            "ignoregrouplock" => {
                let state = match data {
                    "0" => IgnoreGroupLockState::Off,
                    "1" => IgnoreGroupLockState::On,
                    _ => return Err(HyprlandEventParseError),
                };

                Self::IgnoreGroupLock { state }
            }
            "lockgroups" => {
                let state = match data {
                    "0" => LockGroupsState::Off,
                    "1" => LockGroupsState::On,
                    _ => return Err(HyprlandEventParseError),
                };

                Self::LockGroups { state }
            }
            "configreloaded" => Self::ConfigReloaded,
            "pin" => {
                let (window_address, pin_state) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::Pin {
                    window_address: window_address.to_owned(),
                    pin_state: pin_state.to_owned(),
                }
            }
            "minimized" => {
                let (window_address, state) =
                    data.split_once(',').ok_or(HyprlandEventParseError)?;

                let state = match state {
                    "0" => MinimizedState::Unminimized,
                    "1" => MinimizedState::Minimized,
                    _ => return Err(HyprlandEventParseError),
                };

                Self::Minimized {
                    window_address: window_address.to_owned(),
                    state,
                }
            }
            "bell" => Self::Bell {
                window_address: data.to_owned(),
            },
            _ => return Err(HyprlandEventParseError),
        })
    }
}

#[derive(Debug, Error)]
#[error("failed to parse Hyprland event")]
pub struct HyprlandEventParseError;
