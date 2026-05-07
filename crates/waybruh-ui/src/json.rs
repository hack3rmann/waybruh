use crate::{Global, GlobalCallback, InitError, InstanceExt};
use serde_json::Value as JsonValue;
use serde_json_path::JsonPath;
use slint::SharedString;
use slint_interpreter::{ComponentInstance, Value};

pub struct Json;

impl Global for Json {
    fn build(instance: &ComponentInstance) -> Result<(), InitError> {
        instance.add_global_callback::<JsonQuery>()?;
        Ok(())
    }
}

pub struct JsonQuery;

impl GlobalCallback for JsonQuery {
    const GLOBAL_NAME: &str = "Json";
    const CALLBACK_NAME: &str = "query";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(query_str), Value::String(json_str)] = params else {
            panic!("expected a single param");
        };

        let json = match serde_json::from_str::<JsonValue>(json_str) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to parse json: {error}");
                return Value::String(SharedString::new());
            }
        };

        let path = match JsonPath::parse(query_str) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("failed to parse json query: {error}");
                return Value::String(SharedString::new());
            }
        };

        match path.query(&json).all().as_slice() {
            [] => Value::String(SharedString::new()),
            [single] => {
                let result = single.to_string();
                Value::String(SharedString::from(result))
            }
            array => {
                let result =
                    JsonValue::Array(array.iter().map(|&v| v.clone()).collect()).to_string();

                Value::String(SharedString::from(result))
            }
        }
    }
}
