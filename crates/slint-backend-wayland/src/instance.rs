use slint::ComponentHandle;
use slint_interpreter::{ComponentInstance, GetPropertyError, SetPropertyError, Value};
use std::cell::RefCell;

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

pub fn set_property(name: &str, value: Value) -> Result<(), SetPropertyError> {
    COMPONENT_INSTANCE.with_borrow(|i| {
        i.as_ref()
            .map(|i| i.set_property(name, value))
            .unwrap_or(Ok(()))
    })
}

pub fn get_property(name: &str) -> Result<Value, GetPropertyError> {
    COMPONENT_INSTANCE.with_borrow(|i| i.as_ref().map(|i| i.get_property(name)).unwrap())
}
