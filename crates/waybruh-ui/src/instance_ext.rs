use crate::GlobalCallback;
use slint_interpreter::{ComponentInstance, SetCallbackError};

pub trait InstanceExt {
    fn add_global_callback<C: GlobalCallback>(&self) -> Result<(), SetCallbackError>;
}

impl InstanceExt for ComponentInstance {
    fn add_global_callback<C: GlobalCallback>(&self) -> Result<(), SetCallbackError> {
        self.set_global_callback(C::GLOBAL_NAME, C::CALLBACK_NAME, C::execute)
    }
}
