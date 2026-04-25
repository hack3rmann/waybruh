use crate::{
    ChannelWrapper,
    event_loop::{PlatformSharedState, SlintEvent},
    instance, scaling,
};
use calloop::{LoopSignal, channel::Sender};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use slint::{
    PhysicalPosition, PhysicalSize,
    platform::{PointerEventButton, WindowEvent},
};
use slint_interpreter::Value;
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
    registry_handlers,
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
}

pub type OutputId = ObjectId;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceState {
    pub size: PhysicalSize,
    pub surface: WlSurface,
    pub layer: LayerSurface,
}

impl SurfaceState {
    pub fn set_exclusive_zone(&self, compositor: &CompositorState, zone: i32) {
        let region = Region::new(compositor).unwrap();

        region.add(0, 0, 2520, zone);

        self.layer.set_input_region(Some(region.wl_region()));
        self.layer.set_exclusive_zone(zone);
    }

    pub fn raw_window_handle(&self) -> WaylandWindowHandle {
        let ptr = self.surface.id().as_ptr();

        WaylandWindowHandle::new(
            NonNull::new(ptr)
                .expect("*mut wl_surface expected to be non-null")
                .cast(),
        )
    }
}

impl HasWindowHandle for SurfaceState {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Wayland(self.raw_window_handle())) })
    }
}

pub struct ClientState {
    pub shared_state: Arc<PlatformSharedState>,
    pub slint_channel: Sender<SlintEvent>,
    pub output_state: OutputState,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub shm: Shm,
    pub pointer: Option<WlPointer>,
    pub event_channel: ChannelWrapper<WaylandEvent>,
    pub exit_signal: Option<LoopSignal>,
    pub compositor_state: CompositorState,
    pub layer_shell: LayerShell,
    pub need_roundtrip: bool,
}

impl ClientState {
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

        self.shared_state.set_size(&surface_id, size);

        let mut states = self.shared_state.surface_states.write().unwrap();

        let Some(state) = states.get_mut(&surface_id) else {
            return;
        };

        state.size = size;
    }

    pub fn get_surface_size(&self, surface_id: &ObjectId) -> Option<PhysicalSize> {
        let states = self.shared_state.surface_states.read().unwrap();
        states.get(surface_id).map(|s| s.size)
    }

    pub fn remove_surface(&mut self, surface_id: &ObjectId) {
        let mut states = self.shared_state.surface_states.write().unwrap();
        states.remove(surface_id);
    }

    fn destroy_output(&mut self, output: &WlOutput) {
        {
            let mut windows = self.shared_state.windows.write().unwrap();
            windows.remove(&output.id());
        }

        {
            let mut pending = self.shared_state.pending_outputs.write().unwrap();
            let Some(index) = pending
                .iter()
                .enumerate()
                .find_map(|(i, o)| (o.id() == output.id()).then_some(i))
            else {
                return;
            };

            pending.swap_remove(index);
        }
    }

    fn create_output(&mut self, qh: &QueueHandle<Self>, output: WlOutput) {
        let output_id = output.id();

        {
            let mut outputs = self.shared_state.pending_outputs.write().unwrap();
            outputs.push(output.clone());
        }

        // TODO(hack3rmann): move this to WaylandEvent::Fn
        self.slint_channel
            .send(SlintEvent::Fn(Box::new(move || instance::show(output_id))))
            .unwrap();

        self.create_surface(qh, &output);
    }

    fn create_surface(&mut self, qh: &QueueHandle<Self>, output: &WlOutput) {
        let surface = self.compositor_state.create_surface::<ClientState>(qh);
        let layer = self.layer_shell.create_layer_surface::<ClientState>(
            qh,
            surface.clone(),
            Layer::Top,
            Some("waybruh"),
            Some(output),
        );

        let (output_width, output_height) = self
            .output_state
            .info(output)
            .unwrap()
            .logical_size
            .unwrap();

        let size = PhysicalSize {
            width: output_width as u32,
            height: output_height as u32,
        };

        let input_region = Region::new(&self.compositor_state).unwrap();

        let zone = defaults::get_zone();

        input_region.add(0, 0, output_width, zone);

        layer.set_input_region(Some(input_region.wl_region()));
        layer.set_exclusive_zone(zone);
        layer.set_anchor(WlrAnchor::TOP);
        layer.set_margin(0, 0, 0, 0);
        layer.set_size(size.width, size.height);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);

        surface.commit();

        let state = SurfaceState {
            size,
            surface: surface.clone(),
            layer: layer.clone(),
        };

        {
            let mut states = self.shared_state.surface_states.write().unwrap();
            states.insert(state.surface.id(), state.clone());
        }

        {
            let mut windows = self.shared_state.windows.write().unwrap();
            windows.insert(output.id(), Arc::new(state));
        }
    }
}

impl ProvidesRegistryState for ClientState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
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
        layer_surface.set_exclusive_zone(defaults::get_zone());

        self.set_surface_size(surface_id, PhysicalSize { width, height });
    }
}

delegate_layer!(ClientState);

impl OutputHandler for ClientState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {
        self.create_output(qh, output.clone());
        self.need_roundtrip = true;
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        instance::remove(&output.id());
        self.destroy_output(&output);
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
    use super::*;

    pub const EXCLUSIVE_ZONE: i32 = 25;

    pub fn get_zone() -> i32 {
        let zone = match instance::get_property("exclusive-zone") {
            Ok(Value::Number(zone)) => zone as i32,
            _ => defaults::EXCLUSIVE_ZONE,
        };

        (zone as f32 * scaling::get()).round() as i32
    }
}

pub struct WaylandInner {
    pub connection: Connection,
    pub event_queue: Mutex<Option<EventQueue<ClientState>>>,
    // TODO(hack3rmann): move the state out of this
    pub client_state: Mutex<Option<ClientState>>,
    pub shared_state: Arc<PlatformSharedState>,
}

impl WaylandInner {
    pub fn new(slint_channel: Sender<SlintEvent>, shared_state: Arc<PlatformSharedState>) -> Self {
        let connection = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) = registry_queue_init::<ClientState>(&connection).unwrap();
        let qh = event_queue.handle();

        let registry_state = RegistryState::new(&globals);

        let layer_shell = LayerShell::bind(&globals, &qh).unwrap();
        let compositor_state = CompositorState::bind(&globals, &qh).unwrap();
        let shm = Shm::bind(&globals, &qh).unwrap();
        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);

        let mut state = ClientState {
            slint_channel,
            output_state,
            registry_state,
            seat_state,
            shm,
            compositor_state,
            layer_shell,
            shared_state: Arc::clone(&shared_state),
            pointer: None,
            event_channel: ChannelWrapper::default(),
            exit_signal: None,
            need_roundtrip: false,
        };

        const N_ROUNDTRIPTS: usize = 2;

        for _ in 0..N_ROUNDTRIPTS {
            event_queue.roundtrip(&mut state).unwrap();
            event_queue.roundtrip(&mut state).unwrap();
        }

        Self {
            connection,
            event_queue: Mutex::new(Some(event_queue)),
            client_state: Mutex::new(Some(state)),
            shared_state,
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
        let windows = self.shared_state.windows.read().unwrap();

        windows
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
    pub fn new(sender: Sender<SlintEvent>, state: Arc<PlatformSharedState>) -> Self {
        Self(Arc::new(WaylandInner::new(sender, state)))
    }
}

impl Wayland {
    pub fn display_handle(&self) -> Arc<dyn HasDisplayHandle + Send + Sync> {
        Arc::clone(&self.0) as Arc<dyn HasDisplayHandle + Send + Sync>
    }
}

impl Deref for Wayland {
    type Target = WaylandInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
