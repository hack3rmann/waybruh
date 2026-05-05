use slint::ComponentHandle;
use slint_interpreter::{
    ComponentDefinition, ComponentInstance, GetPropertyError, SetPropertyError, Value,
};
use smithay_client_toolkit::reexports::client::backend::ObjectId;
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error;

#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalEntry {
    global: String,
    name: String,
}

type ShowHook = Box<dyn Fn(&ComponentInstance)>;

thread_local! {
    static COMPONENT_DEFINITION: RefCell<Option<ComponentDefinition>>
        = const { RefCell::new(None) };

    static COMPONENT_INSTANCES: RefCell<HashMap<ObjectId, ComponentInstance>>
        = RefCell::default();

    static SHOW_HOOK: RefCell<Option<ShowHook>> = const { RefCell::new(None) };

    static GLOBAL_PROPERTIES: RefCell<HashMap<GlobalEntry, Value>> = RefCell::default();
}

fn create_instance() -> Option<ComponentInstance> {
    COMPONENT_DEFINITION.with_borrow(|d| d.as_ref().and_then(|d| d.create().ok()))
}

fn execute_show_hook(instance: &ComponentInstance) {
    SHOW_HOOK.with_borrow(|h| {
        if let Some(hook) = h {
            hook(instance);
        }
    });
}

fn populate_global_properties(instance: &ComponentInstance) -> Result<(), SetPropertyError> {
    GLOBAL_PROPERTIES.with_borrow(|g| {
        for (GlobalEntry { global, name }, value) in g {
            instance.set_global_property(global, name, value.clone())?;
        }

        Ok(())
    })
}

pub fn set_definition(definition: ComponentDefinition) {
    COMPONENT_DEFINITION.with_borrow_mut(|d| {
        *d = Some(definition);
    });
}

pub fn set_show_hook(hook: impl Fn(&ComponentInstance) + 'static) {
    SHOW_HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(hook));
    })
}

pub fn show(output_id: ObjectId) {
    COMPONENT_INSTANCES.with_borrow_mut(|i| {
        let instance = i
            .entry(output_id)
            .or_insert_with(|| create_instance().unwrap());

        execute_show_hook(instance);
        populate_global_properties(instance).unwrap();

        instance.show().unwrap();
    });
}

pub fn remove(output_id: &ObjectId) {
    COMPONENT_INSTANCES.with_borrow_mut(|i| {
        i.remove(output_id);
    });
}

// FIXME(hack3rmann): per-output property set
pub fn set_property(name: &str, value: Value) -> Result<(), InstanceSetPropertyError> {
    COMPONENT_INSTANCES.with_borrow(|i| {
        for instance in i.values() {
            instance.set_property(name, value.clone())?;
        }

        Ok(())
    })
}

// FIXME(hack3rmann): per-output property get
pub fn get_property(name: &str) -> Result<Value, InstanceGetPropertyError> {
    COMPONENT_INSTANCES.with_borrow(|i| match i.values().next() {
        Some(instance) => Ok(instance.get_property(name)?),
        None => Err(InstanceGetPropertyError::NoInstance),
    })
}

pub fn set_global_property(
    global: &str,
    name: &str,
    value: Value,
) -> Result<(), InstanceSetPropertyError> {
    let result = COMPONENT_INSTANCES.with_borrow(|i| {
        for instance in i.values() {
            instance.set_global_property(global, name, value.clone())?;
        }

        Ok(())
    });

    if result.is_ok() {
        GLOBAL_PROPERTIES.with_borrow_mut(|g| {
            g.insert(
                GlobalEntry {
                    global: global.to_owned(),
                    name: name.to_owned(),
                },
                value,
            );
        });
    }

    result
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
