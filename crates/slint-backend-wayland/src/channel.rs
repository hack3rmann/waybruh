use calloop::channel::{Channel, Sender};
use std::sync::{Mutex, MutexGuard, mpsc::SendError};

pub struct ChannelWrapper<T> {
    sender: Sender<T>,
    receiver: Mutex<Option<Channel<T>>>,
}

impl<T> ChannelWrapper<T> {
    pub fn take_receiver(&self) -> Option<Channel<T>> {
        let mut receiver = self.receiver.lock().ok()?;
        receiver.take()
    }

    pub fn make_sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    pub fn sender(&self) -> &Sender<T> {
        &self.sender
    }

    pub fn receiver(&self) -> MutexGuard<'_, Option<Channel<T>>> {
        self.receiver.lock().unwrap()
    }

    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.sender.send(value)
    }
}

impl<T> Default for ChannelWrapper<T> {
    fn default() -> Self {
        let (sender, receiver) = calloop::channel::channel();

        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }
}
