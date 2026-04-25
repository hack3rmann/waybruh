use clap::Parser;
use slint_interpreter::{Compiler, ComponentDefinition};
use std::path::PathBuf;
use tokio::fs;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    slint_backend_wayland::init().unwrap();

    let compiler = Compiler::default();
    let args = Args::parse();

    let definition = prepare_main_component(&compiler, args.path, &args.entry).await;

    slint_backend_wayland::instance::set_show_hook(|i| waybruh_ui::populate_instance(i).unwrap());
    slint_backend_wayland::instance::set_definition(definition);

    slint::run_event_loop().unwrap();
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
) -> ComponentDefinition {
    let mut source_code = fs::read_to_string(&path).await.unwrap();

    source_code.push_str(waybruh_ui::RE_EXPORTS);

    let result = compiler.build_from_source(source_code, path.clone()).await;

    let Some(definition) = result.component(entry) else {
        result.print_diagnostics();
        panic!("failed to find BruhBar component in {}", path.display(),);
    };

    definition
}
