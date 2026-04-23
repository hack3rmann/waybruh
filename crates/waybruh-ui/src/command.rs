use crate::GlobalCallback;
use slint::Model;
use slint_interpreter::{SharedString, Struct, Value};
use std::process::{Command, Stdio};

pub struct CommandExecute;

impl GlobalCallback for CommandExecute {
    const GLOBAL_NAME: &str = "Command";
    const CALLBACK_NAME: &str = "execute";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::Model(model) = param else {
            panic!("value_type expected to be string");
        };

        let mut params = model.iter();

        let Some(program) = params.next() else {
            return Value::Struct(Struct::from_iter([
                ("stdout".to_owned(), Value::String(SharedString::new())),
                ("stderr".to_owned(), Value::String(SharedString::new())),
                ("exit_code".to_owned(), Value::Number(0.0)),
            ]));
        };

        let Value::String(program) = program else {
            panic!("program expected to be string");
        };

        let arguments = params.map(|a| match a {
            Value::String(string) => string,
            _ => panic!("arguments expected to be strings"),
        });

        let child = match Command::new(program)
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                // TODO(hack3rmann): use tracing instead
                eprintln!("failed to run child process: {err}");
                return Value::String(SharedString::new());
            }
        };

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(err) => {
                // TODO(hack3rmann): use tracing instead
                eprintln!("failed to run wait for output: {err}");
                return Value::String(SharedString::new());
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        Value::Struct(Struct::from_iter([
            (
                "stdout".to_owned(),
                Value::String(SharedString::from(stdout.as_ref())),
            ),
            (
                "stderr".to_owned(),
                Value::String(SharedString::from(stderr.as_ref())),
            ),
            ("exit_code".to_owned(), Value::Number(exit_code as f64)),
        ]))
    }
}
