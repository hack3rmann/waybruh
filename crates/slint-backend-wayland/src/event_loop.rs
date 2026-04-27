use crate::{
    channel::ChannelWrapper,
    hyprland::{HyprlandConnection, HyprlandEventSource},
    instance, scaling,
    system::{self, SystemEvent},
    wayland::{ClientState, SurfaceState, Wayland, WaylandEvent},
    window::SlintWindowAdapter,
};
use calloop::{
    EventLoop, LoopHandle,
    channel::{Event as ChannelEvent, Sender},
    timer::{TimeoutAction, Timer},
};
use i_slint_renderer_skia::SkiaSharedContext;
use slint::{
    EventLoopError, PhysicalSize, PlatformError, WindowSize,
    platform::{EventLoopProxy, LayoutConstraints, Platform, WindowAdapter, WindowEvent},
};
use slint_interpreter::Value;
use smithay_client_toolkit::{
    reexports::{
        calloop_wayland_source::WaylandSource,
        client::{
            Proxy as _,
            backend::ObjectId,
            protocol::{wl_output::WlOutput, wl_surface::WlSurface},
        },
    },
    shell::WaylandSurface,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Debug},
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

#[derive(Default)]
pub struct PlatformSharedState {
    pub windows: RwLock<HashMap<ObjectId, Arc<SurfaceState>>>,
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

pub struct WaylandPlatform {
    wayland: Wayland,
    slint_event_channel: ChannelWrapper<SlintEvent>,
    adapters: Mutex<Vec<Rc<SlintWindowAdapter>>>,
    shared_state: Arc<PlatformSharedState>,
    pending_rendering: Mutex<HashSet<ObjectId>>,
}

impl Default for WaylandPlatform {
    fn default() -> Self {
        let shared_state = Arc::<PlatformSharedState>::default();
        let slint_event_channel = ChannelWrapper::default();

        Self {
            wayland: Wayland::new(slint_event_channel.make_sender(), Arc::clone(&shared_state)),
            slint_event_channel,
            adapters: Mutex::default(),
            shared_state,
            pending_rendering: Mutex::default(),
        }
    }
}

impl WaylandPlatform {
    pub fn get_output_surface_id(&self, output_id: &ObjectId) -> Option<ObjectId> {
        self.get_output_surface(output_id).map(|s| s.id())
    }

    pub fn get_output_surface(&self, output_id: &ObjectId) -> Option<WlSurface> {
        let windows = self.shared_state.windows.read().unwrap();
        windows.get(output_id).map(|w| &w.surface).cloned()
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
        }
    }

    pub fn handle_system_event(&self, event: SystemEvent, state: &mut ClientState) {
        match event {
            SystemEvent::ExclusiveZoneChanged(zone) => {
                let states = self.shared_state.surface_states.read().unwrap();

                // TODO(hack3rmann): match the exclusive zone to the source surface
                for surface_state in states.values() {
                    surface_state.set_exclusive_zone(&state.compositor_state, zone);
                    surface_state.layer.commit();
                }
            }
        }
    }

    pub fn handle_slint_event(&self, event: SlintEvent, state: &mut ClientState) {
        match event {
            SlintEvent::Fn(SlintFnEvent(callback)) => callback(),
            SlintEvent::Quit => {
                if let Some(signal) = &state.exit_signal {
                    signal.stop();
                }
            }
            SlintEvent::RedrawRequested { surface_id } => {
                let mut pending = self.pending_rendering.lock().unwrap();
                pending.insert(surface_id);
            }
            SlintEvent::SetWindowSize { surface_id, size } => {
                let states = self.shared_state.surface_states.read().unwrap();

                let Some(state) = states.get(&surface_id) else {
                    return;
                };

                let size = match size {
                    WindowSize::Physical(physical_size) => physical_size,
                    WindowSize::Logical(logical_size) => logical_size.to_physical(scaling::get()),
                };

                state.layer.set_size(size.width, size.height);
                state.layer.commit();
            }
            SlintEvent::UpdateWindowLayoutConstraints { .. } => {}
        }
    }

    pub fn insert_event_sources<'h, 's: 'h>(&'s self, handle: &LoopHandle<'h, ClientState>) {
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

        let system_source = system::get()
            .channel()
            .take_receiver()
            .expect("event should not be taken");

        // TODO(hack3rmann): handle different refresh rates
        let frame_time = Duration::from_secs_f32(1.0 / 60.0);

        #[cfg(feature = "hyprland-ipc")]
        if let Ok(hyprland_conn) = HyprlandConnection::new() {
            let hyprland_source = HyprlandEventSource::new(hyprland_conn);

            handle
                .insert_source(hyprland_source, |event, _, _| match event {
                    ChannelEvent::Msg(event) => drop(dbg!(event)),
                    ChannelEvent::Closed => {}
                })
                .unwrap();
        }

        handle
            .insert_source(Timer::from_duration(frame_time), move |_, _, _| {
                TimeoutAction::ToDuration(frame_time)
            })
            .unwrap();

        handle
            .insert_source(system_source, |event, (), state| match event {
                ChannelEvent::Msg(event) => self.handle_system_event(event, state),
                ChannelEvent::Closed => {}
            })
            .unwrap();

        handle
            .insert_source(wayland_source, |(), queue, state| {
                if state.need_roundtrip {
                    state.need_roundtrip = false;
                    queue.roundtrip(state)
                } else {
                    queue.dispatch_pending(state)
                }
            })
            .unwrap();

        // NOTE(hack3rmann): wayland events must be processed before SlintEvents to initialize the
        // backend properly
        handle
            .insert_source(window_receiver, |event, (), _| match event {
                ChannelEvent::Msg(event) => self.handle_wayland_event(event),
                ChannelEvent::Closed => {}
            })
            .unwrap();

        handle
            .insert_source(event_receiver, |event, (), state| match event {
                ChannelEvent::Msg(event) => self.handle_slint_event(event, state),
                ChannelEvent::Closed => {}
            })
            .unwrap();
    }

    pub fn run_initial_setup(&self, state: &mut ClientState) {
        for output in state.output_state.outputs() {
            instance::show(output.id());
        }

        if let Ok(Value::Number(zone)) = instance::get_property("exclusive-zone") {
            let zone = (zone * scaling::get() as f64).round() as i32;
            self.handle_system_event(SystemEvent::ExclusiveZoneChanged(zone), state);
        }
    }

    pub fn run_loop_iteration(&self, _state: &mut ClientState) {
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
    }
}

impl Platform for WaylandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
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
            self.slint_event_channel.make_sender(),
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

        self.insert_event_sources(&handle);

        let mut client_state = {
            let mut client_state = self.wayland.client_state.lock().unwrap();
            client_state.take().unwrap()
        };

        self.run_initial_setup(&mut client_state);

        const TIMEOUT: Duration = Duration::from_millis(10_000);

        event_loop
            .run(TIMEOUT, &mut client_state, |state| {
                self.run_loop_iteration(state);
            })
            .map_err(|_| PlatformError::NoEventLoopProvider)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(EventLoopHandle::new(
            self.slint_event_channel.make_sender(),
        )))
    }
}

pub struct SlintFnEvent(pub Box<dyn FnOnce() + Send>);

impl Debug for SlintFnEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SlintFnEvent").finish_non_exhaustive()
    }
}

#[derive(Debug)]
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

    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), EventLoopError> {
        self.sender
            .send(SlintEvent::Fn(SlintFnEvent(event)))
            .map_err(|_| EventLoopError::NoEventLoopProvider)
    }
}
