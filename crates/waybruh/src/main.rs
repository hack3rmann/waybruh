use clap::Parser;
use slint_interpreter::{Compiler, ComponentInstance};
use std::path::PathBuf;
use tokio::fs;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dbg!("main: before_init");
    slint_backend_wayland::init().unwrap();
    dbg!("main: after_init");

    let compiler = Compiler::default();
    let args = Args::parse();

    dbg!("before prepare_main_component");
    let instance = prepare_main_component(&compiler, args.path, &args.entry).await;
    dbg!("after prepare_main_component");

    slint_backend_wayland::instance::set(instance);

    dbg!(std::thread::current().id());

    _ = dbg!(slint_backend_wayland::instance::get_property(
        "exclusive-zone"
    ));

    dbg!("before run_event_loop");
    slint::run_event_loop().unwrap();
    dbg!("after run_event_loop");
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to `.slint` file containing BruhBar component that must inherit from Window
    path: PathBuf,

    /// Entry component
    #[arg(short, long, default_value_t = String::from("BruhBar"))]
    entry: String,
}

async fn prepare_main_component(
    compiler: &Compiler,
    path: PathBuf,
    entry: &str,
) -> ComponentInstance {
    let mut source_code = fs::read_to_string(&path).await.unwrap();

    source_code.push_str(waybruh_ui::RE_EXPORTS);

    let result = compiler.build_from_source(source_code, path.clone()).await;

    let Some(definition) = result.component(entry) else {
        result.print_diagnostics();
        panic!("failed to find BruhBar component in {}", path.display(),);
    };

    let instance = definition.create().unwrap();

    waybruh_ui::populate_instance(&instance).unwrap();

    instance
}
