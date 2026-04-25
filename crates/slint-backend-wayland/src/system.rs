use crate::channel::ChannelWrapper;
use lazy_static::lazy_static;

#[derive(Clone, Debug, PartialEq)]
pub enum SystemEvent {
    ExclusiveZoneChanged(i32),
}

#[derive(Default)]
pub struct System {
    channel: ChannelWrapper<SystemEvent>,
}

impl System {
    pub fn channel(&self) -> &ChannelWrapper<SystemEvent> {
        &self.channel
    }
}

lazy_static! {
    static ref SYSTEM: System = System::default();
}

pub fn get() -> &'static System {
    &SYSTEM
}

pub fn set_exclusive_zone(exclusive_zone: i32) {
    get()
        .channel()
        .send(SystemEvent::ExclusiveZoneChanged(exclusive_zone))
        .unwrap();
}
