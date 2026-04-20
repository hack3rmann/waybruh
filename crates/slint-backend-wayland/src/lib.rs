pub mod event_loop;
pub mod wayland;

use crate::{
    event_loop::{Event, EventLoopHandle, Quit},
    wayland::Wayland,
};
use calloop::{
    EventLoop, LoopSignal,
    channel::{Channel, Event as ChannelEvent, Sender},
};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use slint::{
    PhysicalSize, PlatformError, Window,
    platform::{EventLoopProxy, Platform, Renderer, SetPlatformError, WindowAdapter},
};
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use std::{
    rc::{Rc, Weak},
    sync::Mutex,
    time::Duration,
};

pub fn init() -> Result<(), SetPlatformError> {
    slint::platform::set_platform(Box::new(WaylandPlatform::default()))
}

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

#[derive(Default)]
pub struct WaylandPlatform {
    wayland: Wayland,
    event_channel: ChannelWrapper<Event>,
    quit_channel: ChannelWrapper<Quit>,
}

impl Platform for WaylandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let output = self.wayland.output_state.outputs().next().unwrap();

        Ok(SlintWindowAdapter::new(
            self.wayland.clone(),
            output,
            &SkiaSharedContext::default(),
        ))
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        let mut event_loop = EventLoop::<LoopSignal>::try_new().unwrap();
        let handle = event_loop.handle();

        let event_receiver = self
            .event_channel
            .take_receiver()
            .expect("event receiver should not be taken");

        let quit_receiver = self
            .quit_channel
            .take_receiver()
            .expect("event receiver should not be taken");

        handle
            .insert_source(event_receiver, |event, _, _| match event {
                ChannelEvent::Msg(callback) => callback(),
                ChannelEvent::Closed => {}
            })
            .unwrap();

        handle
            .insert_source(quit_receiver, |_, _, signal| signal.stop())
            .unwrap();

        let mut shared_data = event_loop.get_signal();

        event_loop
            .run(Duration::from_millis(200), &mut shared_data, |_| {})
            .map_err(|_| PlatformError::NoEventLoopProvider)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(EventLoopHandle::new(
            self.event_channel.sender.clone(),
            self.quit_channel.sender.clone(),
        )))
    }
}

pub struct SlintWindowAdapter {
    window: Window,
    renderer: SkiaRenderer,
    wayland: Wayland,
    output: WlOutput,
}

impl SlintWindowAdapter {
    pub fn new(wayland: Wayland, output: WlOutput, skia_context: &SkiaSharedContext) -> Rc<Self> {
        let renderer = SkiaRenderer::default_vulkan(skia_context);

        Rc::new_cyclic(move |weak: &Weak<Self>| Self {
            window: Window::new(Weak::clone(weak) as Weak<dyn WindowAdapter>),
            renderer,
            wayland,
            output,
        })
    }
}

impl WindowAdapter for SlintWindowAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.wayland
            .output_state
            .info(&self.output)
            .map(|i| i.physical_size)
            .map(|(w, h)| PhysicalSize::new(u32::try_from(w).unwrap(), u32::try_from(h).unwrap()))
            .unwrap_or_default()
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}
