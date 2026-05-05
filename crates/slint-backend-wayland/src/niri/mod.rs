pub mod diff;
pub mod event_loop;
pub mod init;

use calloop::{EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory};
use niri_ipc::{Request, Window, Workspace};
use rustix::{
    io::{self, Errno},
    net::{self, RecvFlags, SendFlags},
};
use slint::{ModelRc, VecModel};
use slint_interpreter::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::OwnedFd,
    rc::{Rc, Weak},
    str::Utf8Error,
};
use thiserror::Error;

pub use niri_ipc::{self, Action as NiriAction, Event as NiriEvent, Request as NiriRequest};

use crate::instance;

#[derive(Debug)]
pub struct NiriConnection {
    event: OwnedFd,
    request: OwnedFd,
}

impl NiriConnection {
    pub fn new() -> Result<Self, NiriConnectionError> {
        let path = init::socket_path().ok_or(NiriConnectionError::NoNiri)?;

        let event = init::connect(path)?;
        let request = init::connect(path)?;

        let mut event_stream_request = serde_json::to_string(&Request::EventStream).unwrap();
        event_stream_request.push('\n');

        io::write(&event, event_stream_request.as_bytes())?;

        Ok(Self { event, request })
    }
}

#[derive(Error, Debug)]
pub enum NiriConnectionError {
    #[error(transparent)]
    Errno(#[from] Errno),
    #[error("NIRI_SOCKET is not defined")]
    NoNiri,
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowId(pub u64);

pub struct Niri {
    request_sock: OwnedFd,
    pub windows: HashMap<WindowId, Window>,
    pub focused_window: Option<WindowId>,
    pub keyboard_layouts: Vec<String>,
    pub current_keyboard_layout_index: usize,
    pub workspaces: Vec<Workspace>,
    pub workspaces_model: Rc<VecModel<Value>>,
}

thread_local! {
    pub static NIRI_INSTANCE: RefCell<Weak<RefCell<Niri>>> = const { RefCell::new(Weak::new()) };
}

pub fn instance() -> Option<Rc<RefCell<Niri>>> {
    NIRI_INSTANCE.with(|i| i.borrow().upgrade())
}

impl Niri {
    pub fn new(sock: OwnedFd) -> Self {
        let workspaces_model = Rc::<VecModel<Value>>::default();
        instance::set_global_property(
            "Niri",
            "workspaces",
            Value::Model(ModelRc::new(Rc::clone(&workspaces_model))),
        )
        .unwrap();

        Self {
            request_sock: sock,
            windows: HashMap::default(),
            focused_window: None,
            keyboard_layouts: vec![],
            current_keyboard_layout_index: 0,
            workspaces: vec![],
            workspaces_model,
        }
    }

    pub fn send(&self, request: NiriRequest) {
        let mut buf = serde_json::to_string(&request).unwrap();
        buf.push('\n');

        let size = net::send(&self.request_sock, buf.as_bytes(), SendFlags::DONTWAIT).unwrap();
        assert_eq!(size, buf.len());
    }
}

pub struct NiriEventSource {
    event: OwnedFd,
    niri: Rc<RefCell<Niri>>,
    buf: Vec<u8>,
}

impl NiriEventSource {
    pub fn new(conn: NiriConnection) -> Self {
        let niri = Rc::new(RefCell::new(Niri::new(conn.request)));

        NIRI_INSTANCE.with(|i| {
            *i.borrow_mut() = Rc::downgrade(&niri);
        });

        Self {
            event: conn.event,
            niri,
            buf: vec![0; 4096],
        }
    }
}

#[derive(Debug, Error)]
pub enum NiriProcessEventsError {
    #[error(transparent)]
    Recv(Errno),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
}

impl EventSource for NiriEventSource {
    type Event = NiriEvent;
    type Metadata = Niri;
    type Ret = ();
    type Error = NiriProcessEventsError;

    fn process_events<F>(
        &mut self,
        _: Readiness,
        _: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let n_bytes = match net::recv(&self.event, &mut self.buf, RecvFlags::DONTWAIT) {
            Ok((n, _)) => n,
            Err(Errno::WOULDBLOCK) => return Ok(PostAction::Continue),
            Err(errno) => return Err(NiriProcessEventsError::Recv(errno)),
        };

        let events_string = str::from_utf8(&self.buf[..n_bytes])?;
        let mut niri = self.niri.borrow_mut();

        for event_str in events_string.split_terminator('\n') {
            let Ok(event) = serde_json::from_str::<NiriEvent>(event_str) else {
                continue;
            };

            callback(event, &mut niri);
        }

        niri.flush_events();

        Ok(PostAction::Continue)
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        unsafe {
            poll.register(
                &self.event,
                Interest::READ,
                Mode::Level,
                token_factory.token(),
            )
        }
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        poll.reregister(
            &self.event,
            Interest::READ,
            Mode::Level,
            token_factory.token(),
        )
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        poll.unregister(&self.event)
    }
}
