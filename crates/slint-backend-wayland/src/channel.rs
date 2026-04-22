use calloop::channel::{Channel, Sender};
use std::sync::Mutex;

pub struct ChannelWrapper<T> {
    pub sender: Sender<T>,
    pub receiver: Mutex<Option<Channel<T>>>,
}

impl<T> ChannelWrapper<T> {
    pub fn take_receiver(&self) -> Option<Channel<T>> {
        let mut receiver = self.receiver.lock().ok()?;
        receiver.take()
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
