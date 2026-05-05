pub mod command;
pub mod date;
pub mod instance_ext;
pub mod niri;
pub mod shell;
pub mod string;
pub mod system;

use crate::{command::Command, date::Date, niri::Niri, shell::Shell, string::StringGlobal};
use slint_interpreter::{ComponentInstance, SetCallbackError, Value};
use thiserror::Error;
use waybruh_ui_macros::compile_exports_from;

pub use crate::{instance_ext::InstanceExt, shell::ShellExecute};

pub const RE_EXPORTS: &str = compile_exports_from!([
    "components/waybruh/globals.slint",
    "components/waybruh/bar.slint",
]);

pub fn populate_instance(instance: &ComponentInstance) -> Result<(), InitError> {
    Shell::build(instance)?;
    Command::build(instance)?;
    Date::build(instance)?;
    StringGlobal::build(instance)?;
    Niri::build(instance)?;

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

pub trait Global: 'static {
    fn build(instance: &ComponentInstance) -> Result<(), InitError>;
}
