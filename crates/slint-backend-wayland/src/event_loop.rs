use calloop::channel::Sender;
use slint::{EventLoopError, platform::EventLoopProxy};

pub type Event = Box<dyn FnOnce() + Send>;

pub struct Quit;

pub struct EventLoopHandle {
    event_sender: Sender<Event>,
    quit_sender: Sender<Quit>,
}

impl EventLoopHandle {
    pub fn new(event_sender: Sender<Event>, quit_sender: Sender<Quit>) -> Self {
        Self {
            event_sender,
            quit_sender,
        }
    }
}

impl EventLoopProxy for EventLoopHandle {
    fn quit_event_loop(&self) -> Result<(), EventLoopError> {
        self.quit_sender
            .send(Quit)
            .map_err(|_| EventLoopError::NoEventLoopProvider)
    }

    fn invoke_from_event_loop(&self, event: Event) -> Result<(), EventLoopError> {
        self.event_sender
            .send(event)
            .map_err(|_| EventLoopError::NoEventLoopProvider)
    }
}
