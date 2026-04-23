use crate::GlobalCallback;
use slint_interpreter::{SharedString, Value};
use std::process::{Command, Stdio};

pub struct ShellExecute;

impl GlobalCallback for ShellExecute {
    const GLOBAL_NAME: &str = "Shell";
    const CALLBACK_NAME: &str = "execute";

    fn execute(params: &[Value]) -> Value {
        let [param] = params else {
            panic!("expected a single param");
        };

        let Value::String(param) = param else {
            panic!("value_type expected to be string");
        };

        let child = match Command::new("sh")
            .args(["-c", param])
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

        Value::String(SharedString::from(stdout.as_ref()))
    }
}
