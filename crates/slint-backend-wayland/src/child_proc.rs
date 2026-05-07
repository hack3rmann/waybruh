use calloop::{EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory};
use rustix::{
    fs::{self, OFlags},
    io::{self, Errno},
};
use slint::SharedString;
use std::{
    ffi::OsStr,
    os::fd::AsFd,
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};
use thiserror::Error;

fn make_nonblocking(fd: impl AsFd) -> Result<(), Errno> {
    let fd = fd.as_fd();

    let flags = fs::fcntl_getfl(fd)?;
    fs::fcntl_setfl(fd, flags | OFlags::NONBLOCK)?;

    Ok(())
}

pub struct ChildProcessSource {
    child: Child,
    stdout: ChildStdout,
    stderr: ChildStderr,
    buf: Vec<u8>,
}

impl ChildProcessSource {
    pub fn new(command: &[impl AsRef<OsStr>]) -> Self {
        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        make_nonblocking(&stdout).unwrap();
        make_nonblocking(&stderr).unwrap();

        Self {
            child,
            stdout,
            stderr,
            buf: vec![0; 4096],
        }
    }
}

impl Drop for ChildProcessSource {
    fn drop(&mut self) {
        self.child.wait().unwrap();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChildProcessEvent {
    StdoutLine(SharedString),
    StderrLine(SharedString),
}

#[derive(Debug, Error)]
pub enum ChildProcessProcessEventsError {}

impl EventSource for ChildProcessSource {
    type Event = ChildProcessEvent;
    type Metadata = ();
    type Ret = ();
    type Error = ChildProcessProcessEventsError;

    fn process_events<F>(
        &mut self,
        _: Readiness,
        _: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        loop {
            let n_bytes = match io::read(&self.stdout, &mut self.buf) {
                Ok(n) => n,
                Err(Errno::WOULDBLOCK) => break,
                Err(error) => panic!("failed to read from child's stdout: {error}"),
            };

            let lines = String::from_utf8_lossy(&self.buf[..n_bytes]);

            for line in lines.split_terminator('\n') {
                callback(ChildProcessEvent::StdoutLine(line.into()), &mut ());
            }
        }

        loop {
            let n_bytes = match io::read(&self.stderr, &mut self.buf) {
                Ok(n) => n,
                Err(Errno::WOULDBLOCK) => break,
                Err(error) => panic!("failed to read from child's stdout: {error}"),
            };

            let lines = String::from_utf8_lossy(&self.buf[..n_bytes]);

            for line in lines.split_terminator('\n') {
                callback(ChildProcessEvent::StderrLine(line.into()), &mut ());
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
                &self.stdout,
                Interest::READ,
                Mode::Level,
                token_factory.token(),
            )?;
            poll.register(
                &self.stderr,
                Interest::READ,
                Mode::Level,
                token_factory.token(),
            )?;
        }

        Ok(())
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        poll.reregister(
            &self.stdout,
            Interest::READ,
            Mode::Level,
            token_factory.token(),
        )?;
        poll.reregister(
            &self.stderr,
            Interest::READ,
            Mode::Level,
            token_factory.token(),
        )?;
        Ok(())
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        poll.unregister(&self.stdout)?;
        poll.unregister(&self.stderr)?;
        Ok(())
    }
}
