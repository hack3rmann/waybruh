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
use smithay_client_toolkit::reexports::client::{Proxy as _, protocol::wl_output::WlOutput};
use std::{
    rc::{Rc, Weak},
    sync::{LazyLock, Mutex},
    time::Duration,
};

pub fn init() -> Result<(), SetPlatformError> {
    slint::platform::set_platform(Box::new(WaylandPlatform::default()))
}

pub mod start_window {
    use std::cell::RefCell;

    thread_local! {
        static SHOW_START_WINDOW: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
    }

    pub fn set(show: impl Fn() + 'static) {
        SHOW_START_WINDOW.with_borrow_mut(|window| {
            *window = Some(Box::new(show));
        });
    }

    pub fn show() {
        SHOW_START_WINDOW.with_borrow(|show| {
            if let Some(show) = show {
                show()
            }
        });
    }
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
    wayland: LazyLock<Wayland>,
    event_channel: ChannelWrapper<Event>,
    quit_channel: ChannelWrapper<Quit>,
    adapters: Mutex<Vec<Rc<SlintWindowAdapter>>>,
}

impl Platform for WaylandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let output = {
            let mut state = self.wayland.client_state.lock().unwrap();
            state.pending_outputs.pop().expect("no outputs left")
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
        let mut event_loop = EventLoop::<LoopSignal>::try_new().unwrap();
        let handle = event_loop.handle();

        let event_receiver = self
            .event_channel
            .take_receiver()
            .expect("event receiver should not be taken");

        let quit_receiver = self
            .quit_channel
            .take_receiver()
            .expect("quit receiver should not be taken");

        let window_receiver = {
            let state = self.wayland.client_state.lock().unwrap();

            state
                .event_channel
                .take_receiver()
                .expect("wayland event receiver should not be taken")
        };

        handle
            .insert_source(event_receiver, |event, _, _| match event {
                ChannelEvent::Msg(callback) => callback(),
                ChannelEvent::Closed => {}
            })
            .unwrap();

        handle
            .insert_source(quit_receiver, |_, _, signal| signal.stop())
            .unwrap();

        handle
            .insert_source(window_receiver, |event, _, _| {
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

        let mut shared_data = event_loop.get_signal();

        event_loop
            .run(Duration::from_millis(20), &mut shared_data, |_| {
                slint::platform::update_timers_and_animations();

                let adapters = self.adapters.lock().unwrap();

                for adapter in adapters.iter() {
                    adapter.renderer.render().unwrap();
                }

                let mut event_queue = self.wayland.event_queue.lock().unwrap();
                let mut client_state = self.wayland.client_state.lock().unwrap();

                event_queue.roundtrip(&mut client_state).unwrap();
            })
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
        let renderer = SkiaRenderer::default_wgpu_28(skia_context);

        let display_handle = wayland.display_handle();
        let window_handle = wayland.window_handle(&output.id()).unwrap();
        let size = wayland.window_size(&output.id()).unwrap();

        renderer
            .set_window_handle(window_handle, display_handle, size, None)
            .unwrap();

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
        self.wayland.window_size(&self.output.id()).unwrap()
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}
