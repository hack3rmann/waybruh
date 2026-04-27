pub mod event;
pub mod init;

use crate::hyprland::{event::HyprlandEvent, init::HyprlandSocketPaths};
use calloop::{EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory};
use rustix::{
    io::Errno,
    net::{self, RecvFlags},
};
use std::{os::fd::OwnedFd, str::Utf8Error};
use thiserror::Error;

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
    conn: HyprlandConnection,
    buf: Vec<u8>,
}

impl HyprlandEventSource {
    pub fn new(conn: HyprlandConnection) -> Self {
        Self {
            conn,
            buf: vec![0; 4096],
        }
    }
}

#[derive(Debug, Error)]
pub enum HyprlandProcessEventError {
    #[error(transparent)]
    Recv(#[from] Errno),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
}

impl EventSource for HyprlandEventSource {
    type Event = HyprlandEvent;
    type Metadata = ();
    type Ret = ();
    type Error = HyprlandProcessEventError;

    fn process_events<F>(
        &mut self,
        _: Readiness,
        _: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let n_bytes = match net::recv(&self.conn.event_sock, &mut self.buf, RecvFlags::DONTWAIT) {
            Ok((n, _)) => n,
            Err(Errno::WOULDBLOCK) => return Ok(PostAction::Continue),
            Err(e) => return Err(HyprlandProcessEventError::Recv(e)),
        };

        // TODO(hack3rmann): handle n_bytes == 4096 case

        let events = str::from_utf8(&self.buf[..n_bytes])?;

        for event_str in events.split('\n') {
            let Ok(event) = event_str.parse::<HyprlandEvent>() else {
                continue;
            };

            callback(event, &mut ());
        }

        Ok(PostAction::Continue)
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        // Safety: calloop get to own the event socket therefore it isn't going to be dropped
        // before it's unregistered
        unsafe {
            poll.register(
                &self.conn.event_sock,
                Interest::WRITE,
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
            &self.conn.event_sock,
            Interest::WRITE,
            Mode::Level,
            token_factory.token(),
        )
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        poll.unregister(&self.conn.event_sock)
    }
}

#[derive(Clone, Debug, Error)]
pub enum HyprlandConnectionError {
    #[error("hyprland socket not found")]
    NoHyprland,
    #[error(transparent)]
    Errno(#[from] Errno),
}
