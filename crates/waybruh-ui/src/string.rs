use slint::SharedString;
use slint_interpreter::Value;

use crate::GlobalCallback;

pub struct StringTrim;

impl GlobalCallback for StringTrim {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "trim";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(param) = param else {
            panic!("value_type expected to be string");
        };

        Value::String(SharedString::from(param.trim()))
    }
}

pub struct StringTrimStart;

impl GlobalCallback for StringTrimStart {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "trim-start";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(param) = param else {
            panic!("value_type expected to be string");
        };

        Value::String(SharedString::from(param.trim_start()))
    }
}

pub struct StringTrimEnd;

impl GlobalCallback for StringTrimEnd {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "trim-end";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(param) = param else {
            panic!("value_type expected to be string");
        };

        Value::String(SharedString::from(param.trim_end()))
    }
}
