use std::cell::RefCell;

thread_local! {
    static SHOW_START_WINDOW: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
}

pub fn set(show: impl Fn() + 'static) {
    SHOW_START_WINDOW.with_borrow_mut(|window| {
        *window = Some(Box::new(show));
    });
}

pub fn show() {
    SHOW_START_WINDOW.with_borrow(|show| {
        if let Some(show) = show {
            show()
        }
    });
}
