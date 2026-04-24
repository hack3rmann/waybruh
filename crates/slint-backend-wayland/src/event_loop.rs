use crate::{
    channel::ChannelWrapper,
    scaling,
    wayland::{ClientState, OutputEvent, SurfaceState, Wayland, WaylandEvent},
    window::SlintWindowAdapter,
};
use calloop::{
    EventLoop,
    channel::{Event as ChannelEvent, Sender},
};
use i_slint_renderer_skia::SkiaSharedContext;
use slint::{
    EventLoopError, PhysicalSize, PlatformError, WindowSize,
    platform::{EventLoopProxy, LayoutConstraints, Platform, WindowAdapter, WindowEvent},
};
use smithay_client_toolkit::reexports::{
    calloop_wayland_source::WaylandSource,
    client::{
        Proxy as _,
        backend::ObjectId,
        protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    },
};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    fmt::{self, Debug},
    rc::Rc,
    sync::{Arc, LazyLock, Mutex, RwLock},
    time::Duration,
};

#[derive(Default)]
pub struct PlatformSharedState {
    pub surface_states: RwLock<HashMap<ObjectId, SurfaceState>>,
    pub pending_outputs: RwLock<Vec<WlOutput>>,
}

impl PlatformSharedState {
    pub fn window_size(&self, surface_id: &ObjectId) -> Option<PhysicalSize> {
        let state = self.surface_states.read().unwrap();
        state.get(surface_id).map(|s| s.size)
    }

    pub fn set_size(&self, surface_id: &ObjectId, size: PhysicalSize) {
        let mut states = self.surface_states.write().unwrap();

        if let Some(state) = states.get_mut(surface_id) {
            state.size = size;
        }
    }
}

#[derive(Default)]
pub struct WaylandPlatform {
    wayland: LazyLock<Wayland>,
    slint_event_channel: ChannelWrapper<SlintEvent>,
    adapters: Mutex<Vec<Rc<SlintWindowAdapter>>>,
    shared_state: Arc<PlatformSharedState>,
    is_event_loop_initialized: Cell<bool>,
    pending_rendering: Mutex<HashSet<ObjectId>>,
}

impl WaylandPlatform {
    pub fn get_output_surface_id(&self, output_id: &ObjectId) -> Option<ObjectId> {
        self.get_output_surface(output_id).map(|s| s.id())
    }

    pub fn get_output_surface(&self, output_id: &ObjectId) -> Option<&WlSurface> {
        self.wayland.windows.get(output_id).map(|w| &w.surface)
    }

    pub fn window_size(&self, output_id: &ObjectId) -> Option<PhysicalSize> {
        let surface_id = self.get_output_surface_id(output_id)?;
        self.shared_state.window_size(&surface_id)
    }

    pub fn handle_wayland_event(&self, event: WaylandEvent) {
        match event {
            WaylandEvent::Window { event, surface_id } => {
                let adapters = self.adapters.lock().unwrap();
                let current_adapters = adapters.iter().filter(|adapter| {
                    let Some(id) = self.get_output_surface_id(&adapter.output.id()) else {
                        return false;
                    };

                    id == surface_id
                });

                for adapter in current_adapters {
                    adapter.window().dispatch_event(event.clone());
                }
            }
            WaylandEvent::Output(OutputEvent::Added(output)) => {
                let mut outputs = self.shared_state.pending_outputs.write().unwrap();
                outputs.push(output);
            }
            WaylandEvent::Output(OutputEvent::Removed(output)) => {
                let mut outputs = self.shared_state.pending_outputs.write().unwrap();

                let Some(index) = outputs
                    .iter()
                    .enumerate()
                    .find_map(|(i, o)| (o.id() == output.id()).then_some(i))
                else {
                    return;
                };

                outputs.swap_remove(index);
            }
            WaylandEvent::SurfaceAdded { state } => {
                let mut states = self.shared_state.surface_states.write().unwrap();
                states.insert(state.surface.id(), state);
            }
            WaylandEvent::SurfaceResized { surface_id, size } => {
                self.shared_state.set_size(&surface_id, size);
            }
            WaylandEvent::SurfaceRemoved { surface_id } => {
                let mut states = self.shared_state.surface_states.write().unwrap();
                states.remove(&surface_id);
            }
        }
    }
}

impl Platform for WaylandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        if !self.is_event_loop_initialized.get() {
            let state = self.wayland.client_state.lock().unwrap();
            let state = state
                .as_ref()
                .expect("client state should not be taken before event loop initialization");

            let receiver = state.event_channel.receiver();
            let receiver = receiver.as_ref().expect(
                "wayland event receiver should not be taken before event loop initialization",
            );

            while let Ok(event) = receiver.try_recv() {
                self.handle_wayland_event(event);
            }
        }

        let output = {
            let mut outputs = self.shared_state.pending_outputs.write().unwrap();
            outputs.pop().expect("no outputs left")
        };

        let surface = self.get_output_surface(&output.id()).unwrap().clone();
        let surface_id = surface.id();

        let adapter = SlintWindowAdapter::new(
            Arc::clone(&self.shared_state),
            self.wayland.clone(),
            surface,
            output,
            self.slint_event_channel.sender(),
            &SkiaSharedContext::default(),
        );

        {
            let mut adapters = self.adapters.lock().unwrap();
            adapters.push(adapter.clone());
        }

        let scale_factor = scaling::get();

        let size = {
            let states = self.shared_state.surface_states.read().unwrap();
            states[&surface_id].size.to_logical(scale_factor)
        };

        self.handle_wayland_event(WaylandEvent::Window {
            event: WindowEvent::ScaleFactorChanged { scale_factor },
            surface_id: surface_id.clone(),
        });

        self.handle_wayland_event(WaylandEvent::Window {
            event: WindowEvent::Resized { size },
            surface_id,
        });

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
                ChannelEvent::Msg(SlintEvent::RedrawRequested { surface_id }) => {
                    let mut pending = self.pending_rendering.lock().unwrap();
                    pending.insert(surface_id);
                }
                ChannelEvent::Msg(SlintEvent::SetWindowSize { surface_id, size }) => {
                    let Some(state) = state.surface_state.get(&surface_id) else {
                        return;
                    };

                    let size = match size {
                        WindowSize::Physical(physical_size) => physical_size,
                        WindowSize::Logical(logical_size) => {
                            logical_size.to_physical(scaling::get())
                        }
                    };

                    state.layer.set_size(size.width, size.height);
                }
                ChannelEvent::Msg(SlintEvent::UpdateWindowLayoutConstraints {
                    surface_id,
                    contraints,
                }) => {
                    let Some(state) = state.surface_state.get(&surface_id) else {
                        return;
                    };

                    let size = PhysicalSize {
                        width: state.size.width,
                        ..contraints.preferred.to_physical(scaling::get())
                    };

                    state.layer.set_size(size.width, size.height);
                }
                ChannelEvent::Closed => {}
            })
            .unwrap();

        handle
            .insert_source(window_receiver, |event, (), _| match event {
                ChannelEvent::Msg(event) => self.handle_wayland_event(event),
                ChannelEvent::Closed => {}
            })
            .unwrap();

        self.is_event_loop_initialized.set(true);

        let mut client_state = {
            let mut client_state = self.wayland.client_state.lock().unwrap();
            client_state.take().unwrap()
        };

        const TIMEOUT: Duration = Duration::from_millis(10_000);

        event_loop
            .run(TIMEOUT, &mut client_state, |_| {
                slint::platform::update_timers_and_animations();

                let mut pending = self.pending_rendering.lock().unwrap();

                if pending.is_empty() {
                    return;
                }

                let adapters = self.adapters.lock().unwrap();

                for adapter in adapters
                    .iter()
                    .filter(|a| pending.contains(&a.surface.id()))
                {
                    adapter.renderer.render().unwrap();
                }

                pending.clear();
            })
            .map_err(|_| PlatformError::NoEventLoopProvider)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(EventLoopHandle::new(
            self.slint_event_channel.sender(),
        )))
    }
}

pub type SlintFnEvent = Box<dyn FnOnce() + Send>;

pub enum SlintEvent {
    Fn(SlintFnEvent),
    Quit,
    RedrawRequested {
        surface_id: ObjectId,
    },
    SetWindowSize {
        surface_id: ObjectId,
        size: WindowSize,
    },
    UpdateWindowLayoutConstraints {
        surface_id: ObjectId,
        contraints: LayoutConstraints,
    },
}

impl Debug for SlintEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlintEvent::Fn(_) => f.write_str("SlintEvent::Fn(..)"),
            SlintEvent::Quit => f.write_str("SlintEvent::Quit"),
            SlintEvent::RedrawRequested { surface_id } => write!(
                f,
                "SlintEvent::RedrawRequested {{ surface_id: {surface_id:?} }}",
            ),
            SlintEvent::SetWindowSize { surface_id, size } => write!(
                f,
                "SlintEvent::SetWindowSize {{ surface_id: {surface_id:?}, size: {size:?} }}",
            ),
            SlintEvent::UpdateWindowLayoutConstraints {
                surface_id,
                contraints,
            } => write!(
                f,
                "SlintEvent::SetWindowSize {{ surface_id: {surface_id:?}, contraints: {contraints:?} }}",
            ),
        }
    }
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
