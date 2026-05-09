use crate::{Global, GlobalCallback, InitError, InstanceExt};
use roman_numerals::{FromRoman, ToRoman};
use slint::SharedString;
use slint_interpreter::{ComponentInstance, Value};

pub struct StringGlobal;

impl Global for StringGlobal {
    fn build(instance: &ComponentInstance) -> Result<(), InitError> {
        instance
            .add_global_callback::<StringIndianToRoman>()?
            .add_global_callback::<StringRomanToIndian>()?
            .add_global_callback::<StringUnquote>()?
            .add_global_callback::<StringStartsWith>()?
            .add_global_callback::<StringEndsWith>()?
            .add_global_callback::<StringReplace>()?
            .add_global_callback::<StringTrimStart>()?
            .add_global_callback::<StringTrimEnd>()?
            .add_global_callback::<StringTrim>()?
            .add_global_callback::<StringContains>()?;

        Ok(())
    }
}

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

pub struct StringUnquote;

impl GlobalCallback for StringUnquote {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "unquote";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(param)] = params else {
            panic!("expected a single param");
        };

        if param.starts_with('"') && param.ends_with('"') && param.len() >= 2 {
            let unquoted = &param[1..param.len() - 1];
            Value::String(SharedString::from(unquoted))
        } else {
            Value::String(param.clone())
        }
    }
}

pub struct StringStartsWith;

impl GlobalCallback for StringStartsWith {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "starts-with";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(source), Value::String(pattern)] = params else {
            panic!("expected two parameters of type string");
        };

        Value::Bool(source.starts_with(pattern.as_str()))
    }
}

pub struct StringEndsWith;

impl GlobalCallback for StringEndsWith {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "ends-with";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(source), Value::String(pattern)] = params else {
            panic!("expected two parameters of type string");
        };

        Value::Bool(source.ends_with(pattern.as_str()))
    }
}

pub struct StringReplace;

impl GlobalCallback for StringReplace {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "replace";

    fn execute(params: &[Value]) -> Value {
        let [
            Value::String(source),
            Value::String(pattern),
            Value::String(replacement),
        ] = params
        else {
            panic!("expected 3 parameters of type string");
        };

        Value::String(SharedString::from(
            source.replace(pattern.as_str(), replacement.as_str()),
        ))
    }
}

pub struct StringTrimStart;

impl GlobalCallback for StringTrimStart {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "trim-start";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(source)] = params else {
            panic!("expected a parameter of type string");
        };

        Value::String(SharedString::from(source.trim_start()))
    }
}

pub struct StringTrimEnd;

impl GlobalCallback for StringTrimEnd {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "trim-end";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(source)] = params else {
            panic!("expected a parameter of type string");
        };

        Value::String(SharedString::from(source.trim_end()))
    }
}

pub struct StringTrim;

impl GlobalCallback for StringTrim {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "trim";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(source)] = params else {
            panic!("expected a parameter of type string");
        };

        Value::String(SharedString::from(source.trim()))
    }
}

pub struct StringContains;

impl GlobalCallback for StringContains {
    const GLOBAL_NAME: &str = "String";
    const CALLBACK_NAME: &str = "contains";

    fn execute(params: &[Value]) -> Value {
        let [Value::String(source), Value::String(pattern)] = params else {
            panic!("expected two parameters of type string");
        };

        Value::Bool(source.contains(pattern.as_str()))
    }
}
