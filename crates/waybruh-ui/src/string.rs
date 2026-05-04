use crate::GlobalCallback;
use roman_numerals::{FromRoman, ToRoman};
use slint::SharedString;
use slint_interpreter::Value;

pub struct StringIndianToRoman;

impl GlobalCallback for StringIndianToRoman {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "indian-to-roman";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(indian) = param else {
            panic!("value expected to be string");
        };

        let Ok(integer) = indian.parse::<u128>() else {
            return Value::String(indian.clone());
        };

        Value::String(SharedString::from(integer.to_roman()))
    }
}

pub struct StringRomanToIndian;

impl GlobalCallback for StringRomanToIndian {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "roman-to-indian";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(roman) = param else {
            panic!("value expected to be string");
        };

        let Some(integer) = u128::from_roman(roman) else {
            return Value::String(roman.clone());
        };

        Value::String(SharedString::from(integer.to_string()))
    }
}
