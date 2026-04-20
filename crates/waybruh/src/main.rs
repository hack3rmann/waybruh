slint::slint! {
    export component HelloWorld inherits Window {
        Text {
            text: "hello world";
            color: green;
        }
    }
}

fn main() {
    slint_backend_wayland::init().unwrap();
    HelloWorld::new().unwrap().run().unwrap();
}
