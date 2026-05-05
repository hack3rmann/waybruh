use crate::{Global, GlobalCallback, InitError, InstanceExt};
use chrono::Local;
use slint::SharedString;
use slint_interpreter::{ComponentInstance, Value};

pub struct Date;

impl Global for Date {
    fn build(instance: &ComponentInstance) -> Result<(), InitError> {
        instance.add_global_callback::<DateCurrentTime>()?;
        Ok(())
    }
}

pub struct DateCurrentTime;

impl GlobalCallback for DateCurrentTime {
    const GLOBAL_NAME: &str = "Date";
    const CALLBACK_NAME: &str = "current-time";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(format) = param else {
            panic!("value_type expected to be string");
        };

        let now = Local::now();
        let time = now.format(format).to_string();

        Value::String(SharedString::from(time))
    }
}
