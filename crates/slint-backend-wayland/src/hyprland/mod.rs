pub mod init;

use crate::hyprland::init::HyprlandSocketPaths;
use calloop::{
    EventSource, Poll, PostAction, Readiness, Token, TokenFactory,
    channel::{Channel, Sender},
};
use rustix::io::{self, Errno};
use std::{
    os::fd::OwnedFd,
    str::FromStr,
    thread::{self, JoinHandle},
};
use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum HyprlandEvent {
    Workspace { name: String },
    WorkspaceV2 { id: String, name: String },
}

impl FromStr for HyprlandEvent {
    type Err = HyprlandEventParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, data) = s.split_once(">>").ok_or(HyprlandEventParseError)?;

        Ok(match name {
            "workspace" => Self::Workspace {
                name: data.to_owned(),
            },
            "workspacev2" => {
                let (id, name) = data.split_once(',').ok_or(HyprlandEventParseError)?;

                Self::WorkspaceV2 {
                    id: id.to_owned(),
                    name: name.to_owned(),
                }
            }
            _ => return Err(HyprlandEventParseError),
        })
    }
}

#[derive(Debug, Error)]
#[error("failed to parse Hyprland event")]
pub struct HyprlandEventParseError;

pub struct HyprlandConnection {
    pub event_sock: OwnedFd,
}

impl HyprlandConnection {
    pub fn new() -> Result<Self, HyprlandConnectionError> {
        let HyprlandSocketPaths {
            query: _,
            event: event_path,
        } = init::get_socket_paths().ok_or(HyprlandConnectionError::NoHyprland)?;

        let event_sock = init::connect_socket(&event_path)?;

        Ok(Self { event_sock })
    }
}

pub struct HyprlandEventSource {
    pub handle: JoinHandle<()>,
    pub channel: Channel<HyprlandEvent>,
}

impl HyprlandEventSource {
    pub fn new(conn: HyprlandConnection) -> Self {
        let (sender, channel) = calloop::channel::channel();
        let handle = thread::spawn(move || Self::dispatch_events(conn, sender));

        Self { handle, channel }
    }

    fn dispatch_events(conn: HyprlandConnection, sender: Sender<HyprlandEvent>) -> ! {
        let mut buf = [0_u8; 4096];

        loop {
            let n_bytes = io::read(&conn.event_sock, &mut buf).unwrap();

            let Ok(events) = str::from_utf8(&buf[..n_bytes]) else {
                continue;
            };

            for event in events.split_terminator('\n') {
                let Ok(event) = event.parse::<HyprlandEvent>() else {
                    continue;
                };

                sender.send(event).unwrap();
            }
        }
    }
}

impl EventSource for HyprlandEventSource {
    type Event = <Channel<HyprlandEvent> as EventSource>::Event;
    type Metadata = <Channel<HyprlandEvent> as EventSource>::Metadata;
    type Ret = <Channel<HyprlandEvent> as EventSource>::Ret;
    type Error = <Channel<HyprlandEvent> as EventSource>::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        <Channel<HyprlandEvent> as EventSource>::process_events(
            &mut self.channel,
            readiness,
            token,
            callback,
        )
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        <Channel<HyprlandEvent> as EventSource>::register(&mut self.channel, poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        <Channel<HyprlandEvent> as EventSource>::reregister(&mut self.channel, poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        <Channel<HyprlandEvent> as EventSource>::unregister(&mut self.channel, poll)
    }

    const NEEDS_EXTRA_LIFECYCLE_EVENTS: bool = false;

    fn before_sleep(&mut self) -> calloop::Result<Option<(Readiness, Token)>> {
        <Channel<HyprlandEvent> as EventSource>::before_sleep(&mut self.channel)
    }

    fn before_handle_events(&mut self, events: calloop::EventIterator<'_>) {
        <Channel<HyprlandEvent> as EventSource>::before_handle_events(&mut self.channel, events)
    }
}

#[derive(Clone, Debug, Error)]
pub enum HyprlandConnectionError {
    #[error("hyprland socket not found")]
    NoHyprland,
    #[error(transparent)]
    Errno(#[from] Errno),
}
