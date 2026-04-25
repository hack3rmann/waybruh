pub mod channel;
pub mod event_loop;
pub mod instance;
pub mod scaling;
pub mod system;
pub mod wayland;
pub mod window;

use crate::{channel::ChannelWrapper, event_loop::WaylandPlatform};
use slint::platform::SetPlatformError;

pub fn init() -> Result<(), SetPlatformError> {
    slint::platform::set_platform(Box::new(WaylandPlatform::default()))
}
