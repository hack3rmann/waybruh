use slint::ComponentHandle;
use slint_interpreter::{ComponentInstance, GetPropertyError, SetPropertyError, Value};
use std::cell::RefCell;
use thiserror::Error;

thread_local! {
    static COMPONENT_INSTANCE: RefCell<Option<ComponentInstance>> = const { RefCell::new(None) };
}

pub fn set(instance: ComponentInstance) {
    COMPONENT_INSTANCE.with_borrow_mut(|i| {
        *i = Some(instance);
    });
}

pub fn show() {
    COMPONENT_INSTANCE.with_borrow(|i| {
        if let Some(instance) = i {
            instance.show().unwrap();
        }
    });
}

pub fn set_property(name: &str, value: Value) -> Result<(), InstanceSetPropertyError> {
    COMPONENT_INSTANCE.with_borrow(|i| match i {
        Some(instance) => Ok(instance.set_property(name, value)?),
        None => Err(InstanceSetPropertyError::NoInstance),
    })
}

pub fn get_property(name: &str) -> Result<Value, InstanceGetPropertyError> {
    COMPONENT_INSTANCE.with_borrow(|i| match i {
        Some(instance) => Ok(instance.get_property(name)?),
        None => Err(InstanceGetPropertyError::NoInstance),
    })
}

#[derive(Error, Debug)]
pub enum InstanceGetPropertyError {
    #[error(transparent)]
    Slint(#[from] GetPropertyError),
    #[error("no instance being set")]
    NoInstance,
}

#[derive(Error, Debug)]
pub enum InstanceSetPropertyError {
    #[error(transparent)]
    Slint(#[from] SetPropertyError),
    #[error("no instance being set")]
    NoInstance,
}
