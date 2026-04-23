pub mod command;
pub mod instance_ext;
pub mod shell;
pub mod string;

use slint_interpreter::{ComponentInstance, SetCallbackError, Value};
use thiserror::Error;
use waybruh_ui_macros::compile_exports_from;

use crate::{
    command::CommandExecute,
    string::{StringTrim, StringTrimEnd, StringTrimStart},
};
pub use crate::{instance_ext::InstanceExt, shell::ShellExecute};

pub const RE_EXPORTS: &str = compile_exports_from!(["components/waybruh/globals.slint"]);

pub fn populate_instance(instance: &ComponentInstance) -> Result<(), InitError> {
    instance.add_global_callback::<ShellExecute>()?;
    instance.add_global_callback::<CommandExecute>()?;
    instance.add_global_callback::<StringTrim>()?;
    instance.add_global_callback::<StringTrimStart>()?;
    instance.add_global_callback::<StringTrimEnd>()?;
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
