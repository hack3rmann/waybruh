use clap::Parser;
use slint_backend_wayland::start_window;
use slint_interpreter::{Compiler, ComponentHandle};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to `.slint` file containing BruhBar component that must inherit from Window
    path: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    slint_backend_wayland::init().unwrap();

    let args = Args::parse();

    let compiler = Compiler::default();
    let result = compiler.build_from_path(&args.path).await;

    let Some(definition) = result.component("BruhBar") else {
        panic!("failed to find BruhBar in {}", args.path.display());
    };

    let instance = definition.create().unwrap();

    start_window::set(move || {
        instance.show().unwrap();
    });

    slint::run_event_loop().unwrap();
}
