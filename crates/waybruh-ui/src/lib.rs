pub mod instance_ext;
pub mod shell;

use slint_interpreter::{ComponentInstance, SetCallbackError, Value};
use thiserror::Error;
use waybruh_ui_macros::compile_exports_from;

pub use crate::{instance_ext::InstanceExt, shell::ShellExecute};

pub const RE_EXPORTS: &str = compile_exports_from!(["components/waybruh/globals.slint"]);

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
