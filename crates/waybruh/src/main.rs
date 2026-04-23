use clap::Parser;
use slint_backend_wayland::start_window;
use slint_interpreter::{Compiler, ComponentHandle, ComponentInstance};
use std::{env, path::PathBuf};
use tokio::fs;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    slint_backend_wayland::init().unwrap();

    let waybruh_ui_path = env::var_os("WAYBRUH_UI_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut working_dir = env::current_dir().unwrap();
            working_dir.push("crates/waybruh-ui/ui");
            working_dir
        });

    let mut compiler = Compiler::default();
    compiler.set_include_paths(vec![waybruh_ui_path]);

    let args = Args::parse();

    let instance = prepare_main_component(&compiler, args.path, &args.entry).await;

    start_window::set(move || instance.show().unwrap());

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
