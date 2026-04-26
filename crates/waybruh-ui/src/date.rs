use crate::GlobalCallback;
use datetime::{LocalDateTime, fmt::DateFormat};
use locale::Time as LocaleTime;
use slint::SharedString;
use slint_interpreter::Value;

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

        let Ok(fmt) = DateFormat::parse(format) else {
            eprintln!("failed to parse date format");
            return Value::String(SharedString::from("error"));
        };

        let locale = LocaleTime::load_user_locale().unwrap_or_else(|_| LocaleTime::english());
        let now = LocalDateTime::now();

        let time = fmt.format(&now, &locale);

        Value::String(SharedString::from(time))
    }
}
