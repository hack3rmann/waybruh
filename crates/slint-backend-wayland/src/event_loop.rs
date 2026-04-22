use crate::{
    channel::ChannelWrapper,
    wayland::{ClientState, Wayland},
    window::SlintWindowAdapter,
};
use calloop::{
    EventLoop,
    channel::{Event as ChannelEvent, Sender},
};
use i_slint_renderer_skia::SkiaSharedContext;
use slint::{
    EventLoopError, PlatformError,
    platform::{EventLoopProxy, Platform, WindowAdapter},
};
use smithay_client_toolkit::reexports::{
    calloop_wayland_source::WaylandSource, client::Proxy as _,
};
use std::{
    rc::Rc,
    sync::{LazyLock, Mutex},
    time::Duration,
};

#[derive(Default)]
pub struct WaylandPlatform {
    wayland: LazyLock<Wayland>,
    slint_event_channel: ChannelWrapper<SlintEvent>,
    adapters: Mutex<Vec<Rc<SlintWindowAdapter>>>,
}

impl Platform for WaylandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let output = {
            let mut outputs = self.wayland.pending_outputs.write().unwrap();
            outputs.pop().expect("no outputs left")
        };

        let adapter =
            SlintWindowAdapter::new(self.wayland.clone(), output, &SkiaSharedContext::default());

        {
            let mut adapters = self.adapters.lock().unwrap();
            adapters.push(adapter.clone());
        }

        Ok(adapter)
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        let mut event_loop = EventLoop::<ClientState>::try_new().unwrap();
        let handle = event_loop.handle();

        let event_receiver = self
            .slint_event_channel
            .take_receiver()
            .expect("event receiver should not be taken");

        let window_receiver = {
            let state = self.wayland.client_state.lock().unwrap();
            let state = state.as_ref().unwrap();

            state
                .event_channel
                .take_receiver()
                .expect("wayland event receiver should not be taken")
        };

        let event_queue = {
            let mut lock = self.wayland.event_queue.lock().unwrap();
            lock.take().expect("event should not be taken")
        };

        let wayland_source = WaylandSource::new(self.wayland.connection.clone(), event_queue);

        handle
            .insert_source(wayland_source, |(), queue, state| {
                {
                    let mut surface_state = self.wayland.surface_state.write().unwrap();
                    surface_state.clone_from(&state.surface_state);
                }

                {
                    // FIXME(hack3rmann): sync pending_outputs in a correct way
                    let mut outputs = self.wayland.pending_outputs.write().unwrap();
                    outputs.clone_from(&state.pending_outputs);
                }

                queue.dispatch_pending(state)
            })
            .unwrap();

        handle
            .insert_source(event_receiver, |event, (), state| match event {
                ChannelEvent::Msg(SlintEvent::Fn(callback)) => callback(),
                ChannelEvent::Msg(SlintEvent::Quit) => {
                    if let Some(signal) = &state.exit_signal {
                        signal.stop();
                    }
                }
                ChannelEvent::Closed => {}
            })
            .unwrap();

        handle
            .insert_source(window_receiver, |event, (), _| {
                let ChannelEvent::Msg((event, surface_id)) = event else {
                    return;
                };

                let adapters = self.adapters.lock().unwrap();

                for adapter in adapters.iter() {
                    let Some(id) = adapter.wayland.get_output_surface_id(&adapter.output.id())
                    else {
                        continue;
                    };

                    if id != surface_id {
                        continue;
                    }

                    adapter.window().dispatch_event(event.clone());
                }
            })
            .unwrap();

        let mut client_state = {
            let mut client_state = self.wayland.client_state.lock().unwrap();
            client_state.take().unwrap()
        };

        event_loop
            .run(Duration::from_millis(1000), &mut client_state, |_| {
                slint::platform::update_timers_and_animations();

                let adapters = self.adapters.lock().unwrap();

                for adapter in adapters.iter() {
                    adapter.renderer.render().unwrap();
                }
            })
            .map_err(|_| PlatformError::NoEventLoopProvider)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(EventLoopHandle::new(
            self.slint_event_channel.sender.clone(),
        )))
    }
}

pub type SlintFnEvent = Box<dyn FnOnce() + Send>;

pub enum SlintEvent {
    Fn(SlintFnEvent),
    Quit,
}

#[derive(Clone)]
pub struct EventLoopHandle {
    sender: Sender<SlintEvent>,
}

impl EventLoopHandle {
    pub const fn new(sender: Sender<SlintEvent>) -> Self {
        Self { sender }
    }
}

impl EventLoopProxy for EventLoopHandle {
    fn quit_event_loop(&self) -> Result<(), EventLoopError> {
        self.sender
            .send(SlintEvent::Quit)
            .map_err(|_| EventLoopError::NoEventLoopProvider)
    }

    fn invoke_from_event_loop(&self, event: SlintFnEvent) -> Result<(), EventLoopError> {
        self.sender
            .send(SlintEvent::Fn(event))
            .map_err(|_| EventLoopError::NoEventLoopProvider)
    }
}
