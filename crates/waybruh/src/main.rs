use clap::Parser;
use slint::SharedString;
use slint_backend_wayland::start_window;
use slint_interpreter::{Compiler, ComponentHandle, Value};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use tokio::fs;

pub struct Shell;

impl Shell {
    pub fn execute(params: &[Value]) -> Value {
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to `.slint` file containing BruhBar component that must inherit from Window
    path: PathBuf,
}

fn add_reexports(source: &mut String) {
    // TODO(hack3rmann): do re-exports with a proc macro
    source.push_str("\nexport { Shell } from \"waybruh-globals.slint\";");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    slint_backend_wayland::init().unwrap();

    let args = Args::parse();

    let compiler = Compiler::default();

    let mut source_code = fs::read_to_string(&args.path).await.unwrap();

    add_reexports(&mut source_code);

    let result = compiler
        .build_from_source(source_code, args.path.clone())
        .await;

    let Some(definition) = result.component("BruhBar") else {
        result.print_diagnostics();
        panic!(
            "failed to find BruhBar component in {}",
            args.path.display(),
        );
    };

    let instance = definition.create().unwrap();

    instance
        .set_global_callback("Shell", "execute", Shell::execute)
        .unwrap();

    start_window::set(move || {
        instance.show().unwrap();
    });

    slint::run_event_loop().unwrap();
}
