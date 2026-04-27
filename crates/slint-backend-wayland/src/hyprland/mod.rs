pub mod init;

use crate::hyprland::init::HyprlandSocketPaths;
use calloop::{EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory};
use rustix::io;
use std::os::fd::OwnedFd;
use thiserror::Error;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct HyprlandEvent {
    info: String,
}

pub struct HyprlandConnection {
    pub event_sock: OwnedFd,
    pub query_sock: OwnedFd,
}

impl HyprlandConnection {
    pub fn new() -> Option<Self> {
        let HyprlandSocketPaths {
            query: query_path,
            event: event_path,
        } = init::get_socket_paths()?;

        let query_sock = init::connect_socket(&query_path).ok()?;
        let event_sock = init::connect_socket(&event_path).ok()?;

        Some(Self {
            event_sock,
            query_sock,
        })
    }
}

#[derive(Debug, Error)]
pub enum Never {}

impl EventSource for HyprlandConnection {
    type Event = HyprlandEvent;
    type Metadata = ();
    type Ret = ();
    type Error = Never;

    fn process_events<F>(
        &mut self,
        _: Readiness,
        _: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let mut buf = vec![0; 4096];

        loop {
            let n_bytes = io::read(&self.event_sock, &mut buf).unwrap();

            if n_bytes == 0 {
                break;
            }

            let events = String::from_utf8_lossy(&buf[..n_bytes]);

            for event in events.split_terminator('\n') {
                callback(
                    HyprlandEvent {
                        info: event.to_owned(),
                    },
                    &mut (),
                );
            }
        }

        Ok(PostAction::Continue)
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        unsafe {
            poll.register(
                &self.event_sock,
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
            &self.event_sock,
            Interest::READ,
            Mode::Level,
            token_factory.token(),
        )
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        poll.unregister(&self.event_sock)
    }
}
