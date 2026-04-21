slint::slint! {
    export component HelloWorld inherits Window {
        Text {
            text: "hello world";
            color: green;
        }
    }
}

fn show_window() {
    HelloWorld::new().unwrap().show().unwrap();
}

fn main() {
    slint_backend_wayland::init().unwrap();
    slint_backend_wayland::start_window::set(show_window);

    slint::run_event_loop().unwrap();
}
