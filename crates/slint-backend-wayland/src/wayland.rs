use crate::{ChannelWrapper, scaling};
use calloop::LoopSignal;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use slint::{
    PhysicalPosition, PhysicalSize,
    platform::{PointerEventButton, WindowEvent},
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    delegate_layer, delegate_output, delegate_pointer, delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, EventQueue, Proxy, QueueHandle,
        backend::ObjectId,
        globals::registry_queue_init,
        protocol::{
            wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat, wl_surface::WlSurface,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor as WlrAnchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler,
            LayerSurface, LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, Region},
    delegate_compositor, delegate_shm,
    reexports::client::protocol::wl_output::Transform,
    shell::wlr_layer::Anchor,
};
use std::{
    collections::HashMap,
    ops::Deref,
    ptr::NonNull,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq)]
pub enum OutputEvent {
    Added(WlOutput),
    Removed(WlOutput),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WaylandEvent {
    Window {
        event: WindowEvent,
        surface_id: ObjectId,
    },
    Output(OutputEvent),
    SurfaceAdded {
        state: SurfaceState,
    },
    SurfaceResized {
        surface_id: ObjectId,
        size: PhysicalSize,
    },
    SurfaceRemoved {
        surface_id: ObjectId,
    },
}

pub type OutputId = ObjectId;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceState {
    pub size: PhysicalSize,
    pub surface: WlSurface,
    pub layer: LayerSurface,
}

pub struct ClientState {
    pub output_state: OutputState,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub shm: Shm,
    pub surface_state: HashMap<ObjectId, SurfaceState>,
    pub pointer: Option<WlPointer>,
    pub event_channel: ChannelWrapper<WaylandEvent>,
    pub exit_signal: Option<LoopSignal>,
}

impl ClientState {
    pub fn new(
        output_state: OutputState,
        registry_state: RegistryState,
        seat_state: SeatState,
        shm: Shm,
    ) -> Self {
        Self {
            output_state,
            registry_state,
            seat_state,
            shm,
            pointer: None,
            event_channel: ChannelWrapper::default(),
            exit_signal: None,
            surface_state: HashMap::new(),
        }
    }

    pub fn add_surface(&mut self, state: SurfaceState) {
        self.event_channel
            .send(WaylandEvent::SurfaceAdded {
                state: state.clone(),
            })
            .unwrap();

        self.surface_state.insert(state.surface.id(), state);
    }

    pub fn set_surface_size(&mut self, surface_id: ObjectId, size: PhysicalSize) {
        let logical_size = size.to_logical(scaling::get());

        let event = WindowEvent::Resized { size: logical_size };

        // FIXME(hack3rmann): remove WaylandEvent::SurfaceAdded
        self.event_channel
            .send(WaylandEvent::Window {
                surface_id: surface_id.clone(),
                event,
            })
            .unwrap();

        self.event_channel
            .send(WaylandEvent::SurfaceResized {
                surface_id: surface_id.clone(),
                size,
            })
            .unwrap();

        let Some(state) = self.surface_state.get_mut(&surface_id) else {
            return;
        };

        state.size = size;
    }

    pub fn get_surface_size(&self, surface_id: &ObjectId) -> Option<PhysicalSize> {
        self.surface_state.get(surface_id).map(|s| s.size)
    }

    pub fn remove_surface(&mut self, surface_id: &ObjectId) {
        self.event_channel
            .send(WaylandEvent::SurfaceRemoved {
                surface_id: surface_id.clone(),
            })
            .unwrap();

        self.surface_state.remove(surface_id);
    }
}

impl ProvidesRegistryState for ClientState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    fn runtime_add_global(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: u32,
        _: &str,
        _: u32,
    ) {
    }

    fn runtime_remove_global(&mut self, _: &Connection, _: &QueueHandle<Self>, _: u32, _: &str) {}
}

impl CompositorHandler for ClientState {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &WlSurface,
        new_factor: i32,
    ) {
        surface.set_buffer_scale(new_factor);
        surface.commit();
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &WlSurface,
        new_transform: Transform,
    ) {
        surface.set_buffer_transform(new_transform);
        surface.commit();
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }
}

delegate_compositor!(ClientState);

impl LayerShellHandler for ClientState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer_surface: &LayerSurface) {
        let surface_id = layer_surface.wl_surface().id();
        self.remove_surface(&surface_id);
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer_surface: &LayerSurface,
        LayerSurfaceConfigure {
            new_size: (width, height),
            ..
        }: LayerSurfaceConfigure,
        _: u32,
    ) {
        let surface_id = layer_surface.wl_surface().id();

        layer_surface.set_anchor(Anchor::TOP);
        layer_surface.set_exclusive_zone(50);

        self.set_surface_size(surface_id, PhysicalSize { width, height });
    }
}

delegate_layer!(ClientState);

impl OutputHandler for ClientState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.event_channel
            .send(WaylandEvent::Output(OutputEvent::Added(output)))
            .unwrap();

        slint::invoke_from_event_loop(crate::instance::show).unwrap();
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.event_channel
            .send(WaylandEvent::Output(OutputEvent::Removed(output)))
            .unwrap();
    }
}

delegate_registry!(ClientState);
delegate_output!(ClientState);

impl ShmHandler for ClientState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_shm!(ClientState);

impl SeatHandler for ClientState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        cap: Capability,
    ) {
        let Capability::Pointer = cap else { return };

        self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        cap: Capability,
    ) {
        if let Capability::Pointer = cap {
            self.pointer = None;
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

fn button_wayland_to_slint(button: u32) -> PointerEventButton {
    use smithay_client_toolkit::seat::pointer::{
        BTN_BACK, BTN_FORWARD, BTN_LEFT, BTN_MIDDLE, BTN_RIGHT,
    };

    match button {
        BTN_LEFT => PointerEventButton::Left,
        BTN_RIGHT => PointerEventButton::Right,
        BTN_MIDDLE => PointerEventButton::Middle,
        BTN_FORWARD => PointerEventButton::Forward,
        BTN_BACK => PointerEventButton::Back,
        _ => PointerEventButton::Other,
    }
}

impl PointerHandler for ClientState {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlPointer,
        events: &[PointerEvent],
    ) {
        for PointerEvent {
            surface,
            position: (x, y),
            kind,
        } in events
        {
            let position = PhysicalPosition::new(*x as i32, *y as i32).to_logical(scaling::get());

            let event = match kind {
                // NOTE(hack3rmann): Enter does not have a matching WindowEvent on Slint's side
                PointerEventKind::Enter { serial: _ } => continue,
                PointerEventKind::Leave { serial: _ } => WindowEvent::PointerExited,
                PointerEventKind::Motion { time: _ } => WindowEvent::PointerMoved { position },
                PointerEventKind::Press {
                    time: _,
                    button,
                    serial: _,
                } => WindowEvent::PointerPressed {
                    position,
                    button: button_wayland_to_slint(*button),
                },
                PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => WindowEvent::PointerReleased {
                    position,
                    button: button_wayland_to_slint(*button),
                },
                // TODO(hack3rmann): handle finger scrolls better
                PointerEventKind::Axis {
                    time: _,
                    horizontal,
                    vertical,
                    source: _,
                } => WindowEvent::PointerScrolled {
                    position,
                    delta_x: horizontal.absolute as f32,
                    delta_y: vertical.absolute as f32,
                },
            };

            self.event_channel
                .send(WaylandEvent::Window {
                    event,
                    surface_id: surface.id(),
                })
                .unwrap();
        }
    }
}

delegate_seat!(ClientState);
delegate_pointer!(ClientState);

pub struct SurfaceBundle {
    pub surface: WlSurface,
    pub layer_surface: LayerSurface,
}

impl SurfaceBundle {
    pub fn raw_window_handle(&self) -> WaylandWindowHandle {
        let ptr = self.surface.id().as_ptr();

        WaylandWindowHandle::new(
            NonNull::new(ptr)
                .expect("*mut wl_surface expected to be non-null")
                .cast(),
        )
    }
}

impl HasWindowHandle for SurfaceBundle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Wayland(self.raw_window_handle())) })
    }
}

pub mod defaults {
    pub const EXCLUSIVE_ZONE: i32 = 50;
}

pub struct WaylandInner {
    pub connection: Connection,
    pub event_queue: Mutex<Option<EventQueue<ClientState>>>,
    pub compositor_state: CompositorState,
    pub layer_shell: LayerShell,
    pub windows: HashMap<ObjectId, Arc<SurfaceBundle>>,
    // TODO(hack3rmann): move the state out of this
    pub client_state: Mutex<Option<ClientState>>,
    pub startup_outputs: Mutex<Vec<WlOutput>>,
}

impl WaylandInner {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let connection = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) = registry_queue_init::<ClientState>(&connection).unwrap();
        let qh = event_queue.handle();

        let registry_state = RegistryState::new(&globals);

        let layer_shell = LayerShell::bind(&globals, &qh).unwrap();
        let compositor_state = CompositorState::bind(&globals, &qh).unwrap();
        let shm = Shm::bind(&globals, &qh).unwrap();
        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);

        dbg!(std::thread::current().id());

        let mut client_state = ClientState::new(output_state, registry_state, seat_state, shm);

        event_queue.roundtrip(&mut client_state).unwrap();

        // NOTE(hack3rmann): interior mutability does not affect the hash here
        #[allow(clippy::mutable_key_type)]
        let windows = client_state
            .output_state
            .outputs()
            .map(|output| {
                let surface = compositor_state.create_surface::<ClientState>(&qh);
                let layer = layer_shell.create_layer_surface::<ClientState>(
                    &qh,
                    surface.clone(),
                    Layer::Top,
                    Some("waybruh"),
                    Some(&output),
                );

                let (output_width, output_height) = client_state
                    .output_state
                    .info(&output)
                    .unwrap()
                    .logical_size
                    .unwrap();

                let size = PhysicalSize {
                    width: output_width as u32,
                    height: output_height as u32,
                };

                let input_region = Region::new(&compositor_state).unwrap();

                input_region.add(0, 0, output_width, defaults::EXCLUSIVE_ZONE);

                layer.set_input_region(Some(input_region.wl_region()));
                layer.set_exclusive_zone(defaults::EXCLUSIVE_ZONE);
                layer.set_anchor(WlrAnchor::TOP);
                layer.set_margin(0, 0, 0, 0);
                layer.set_size(size.width, size.height);
                layer.set_keyboard_interactivity(KeyboardInteractivity::None);

                surface.commit();

                client_state.add_surface(SurfaceState {
                    size,
                    surface: surface.clone(),
                    layer: layer.clone(),
                });

                (
                    output.id(),
                    Arc::new(SurfaceBundle {
                        surface,
                        layer_surface: layer,
                    }),
                )
            })
            .collect();

        let outputs = client_state.output_state.outputs().collect();

        event_queue.roundtrip(&mut client_state).unwrap();

        Self {
            connection,
            event_queue: Mutex::new(Some(event_queue)),
            compositor_state,
            layer_shell,
            windows,
            client_state: Mutex::new(Some(client_state)),
            startup_outputs: Mutex::new(outputs),
        }
    }

    pub fn raw_display_handle(&self) -> WaylandDisplayHandle {
        let ptr = self.connection.backend().display_ptr();

        WaylandDisplayHandle::new(
            NonNull::new(ptr)
                .expect("*mut wl_display expected to be non-null")
                .cast(),
        )
    }

    pub fn window_handle(
        &self,
        output_id: &ObjectId,
    ) -> Option<Arc<dyn HasWindowHandle + Send + Sync>> {
        self.windows
            .get(output_id)
            .map(|o| Arc::clone(o) as Arc<dyn HasWindowHandle + Send + Sync>)
    }
}

impl HasDisplayHandle for WaylandInner {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(self.raw_display_handle()))
        })
    }
}

#[derive(Clone)]
pub struct Wayland(Arc<WaylandInner>);

impl Wayland {
    pub fn display_handle(&self) -> Arc<dyn HasDisplayHandle + Send + Sync> {
        Arc::clone(&self.0) as Arc<dyn HasDisplayHandle + Send + Sync>
    }
}

impl Default for Wayland {
    fn default() -> Self {
        Self(Arc::new(WaylandInner::new()))
    }
}

impl Deref for Wayland {
    type Target = WaylandInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
