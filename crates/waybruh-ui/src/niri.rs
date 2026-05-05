use crate::{Global, GlobalCallback, InitError, InstanceExt};
use slint::Model;
use slint_backend_wayland::niri::{
    self, NiriAction, NiriRequest,
    niri_ipc::{LayoutSwitchTarget, WorkspaceReferenceArg},
};
use slint_interpreter::{ComponentInstance, Value};

pub struct Niri;

impl Global for Niri {
    fn build(instance: &ComponentInstance) -> Result<(), InitError> {
        instance.add_global_callback::<NiriSpawn>()?;
        instance.add_global_callback::<NiriSpawnSh>()?;
        instance.add_global_callback::<NiriFocusWorkspace>()?;
        instance.add_global_callback::<NiriSwitchLayout>()?;
        instance.add_global_callback::<NiriToggleOverview>()?;
        instance.add_global_callback::<NiriOpenOverview>()?;
        instance.add_global_callback::<NiriCloseOverview>()?;

        Ok(())
    }
}

pub struct NiriFocusWorkspace;

impl GlobalCallback for NiriFocusWorkspace {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "focus-workspace";

    fn execute(params: &[Value]) -> Value {
        let [p1, p2] = params else {
            panic!("expected 2 parameters");
        };

        let Value::String(ty) = p1 else {
            panic!("value_type expected to be string");
        };

        let Value::String(ident) = p2 else {
            panic!("value_type expected to be string");
        };

        let reference = match ty.as_str() {
            "id" => {
                let Ok(id) = ident.parse::<u64>() else {
                    return Value::Void;
                };
                WorkspaceReferenceArg::Id(id)
            }
            "idx" => {
                let Ok(index) = ident.parse::<u8>() else {
                    return Value::Void;
                };
                WorkspaceReferenceArg::Index(index)
            }
            "name" => WorkspaceReferenceArg::Name(ident.to_string()),
            _ => return Value::Void,
        };

        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();
        niri.send(NiriRequest::Action(NiriAction::FocusWorkspace {
            reference,
        }));

        Value::Void
    }
}

pub struct NiriSpawn;

impl GlobalCallback for NiriSpawn {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "spawn";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected 2 parameters");
        };

        let Value::Model(model) = param else {
            panic!("value_type expected to be string");
        };

        let command = model
            .iter()
            .flat_map(|v| match v {
                Value::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect();

        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();

        niri.send(NiriRequest::Action(NiriAction::Spawn { command }));

        Value::Void
    }
}

pub struct NiriSpawnSh;

impl GlobalCallback for NiriSpawnSh {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "spawn-sh";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected 2 parameters");
        };

        let Value::String(command) = param else {
            panic!("value_type expected to be string");
        };

        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();

        niri.send(NiriRequest::Action(NiriAction::SpawnSh {
            command: command.to_string(),
        }));

        Value::Void
    }
}

pub struct NiriSwitchLayout;

impl GlobalCallback for NiriSwitchLayout {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "switch-layout";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected 2 parameters");
        };

        let Value::String(target) = param else {
            panic!("value_type expected to be string");
        };

        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();

        let layout = if let Ok(index) = target.parse::<u8>() {
            LayoutSwitchTarget::Index(index)
        } else {
            match target.as_str() {
                "next" => LayoutSwitchTarget::Next,
                "prev" => LayoutSwitchTarget::Prev,
                _ => return Value::Void,
            }
        };

        niri.send(NiriRequest::Action(NiriAction::SwitchLayout { layout }));

        Value::Void
    }
}

pub struct NiriToggleOverview;

impl GlobalCallback for NiriToggleOverview {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "toggle-overview";

    fn execute(_: &[Value]) -> Value {
        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();

        niri.send(NiriRequest::Action(NiriAction::ToggleOverview {}));

        Value::Void
    }
}

pub struct NiriOpenOverview;

impl GlobalCallback for NiriOpenOverview {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "open-overview";

    fn execute(_: &[Value]) -> Value {
        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();

        niri.send(NiriRequest::Action(NiriAction::OpenOverview {}));

        Value::Void
    }
}

pub struct NiriCloseOverview;

impl GlobalCallback for NiriCloseOverview {
    const GLOBAL_NAME: &str = "Niri";
    const CALLBACK_NAME: &str = "close-overview";

    fn execute(_: &[Value]) -> Value {
        let Some(niri) = niri::instance() else {
            return Value::Void;
        };

        let niri = niri.borrow();

        niri.send(NiriRequest::Action(NiriAction::CloseOverview {}));

        Value::Void
    }
}
