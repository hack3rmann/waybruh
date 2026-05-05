use crate::GlobalCallback;
use slint_backend_wayland::niri::{self, NiriAction, NiriRequest, niri_ipc::WorkspaceReferenceArg};
use slint_interpreter::Value;

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
