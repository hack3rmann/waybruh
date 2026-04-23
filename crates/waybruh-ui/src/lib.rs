use slint_interpreter::{ComponentInstance, SetCallbackError, Value};
use thiserror::Error;

pub use crate::{instance_ext::InstanceExt, shell::ShellExecute};

pub mod instance_ext;
pub mod shell;

pub const RE_EXPORTS: &str = r#"
export { Shell } from "waybruh/globals.slint";
"#;

pub fn populate_instance(instance: &ComponentInstance) -> Result<(), InitError> {
    instance.add_global_callback::<ShellExecute>()?;
    Ok(())
}

#[derive(Error, Debug)]
pub enum InitError {
    #[error(transparent)]
    SetCallback(#[from] SetCallbackError),
}

pub trait GlobalCallback: 'static {
    const GLOBAL_NAME: &str;
    const CALLBACK_NAME: &str;

    fn execute(params: &[Value]) -> Value;
}
