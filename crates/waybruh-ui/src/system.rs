use crate::GlobalCallback;
use slint_backend_wayland::{scaling, system};
use slint_interpreter::Value;

pub struct SystemExclusiveZoneChanged;

impl GlobalCallback for SystemExclusiveZoneChanged {
    const GLOBAL_NAME: &str = "System";
    const CALLBACK_NAME: &str = "exclusive-zone-changed";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        // Exclusive zone in logical size
        let &Value::Number(exclusive_zone) = param else {
            panic!("value_type expected to be string");
        };

        let exclusive_zone = (exclusive_zone * scaling::get() as f64).round() as i32;

        system::set_exclusive_zone(exclusive_zone);

        Value::Void
    }
}
