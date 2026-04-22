use calloop::LoopSignal;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use slint::{LogicalPosition, PhysicalSize, platform::{PointerEventButton, WindowEvent}};
use smithay_client_toolkit::{
    compositor::{CompositorState, SurfaceData}, delegate_output, delegate_pointer, delegate_registry, delegate_seat, globals::GlobalData, output::{OutputHandler, OutputState}, reexports::{
        client::{
            Connection, Dispatch, EventQueue, Proxy, QueueHandle, backend::ObjectId, globals::registry_queue_init, protocol::{
                wl_compositor::WlCompositor, wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat, wl_shm::WlShm, wl_surface::WlSurface
            }
        },
        protocols_wlr::layer_shell::v1::client::{
            zwlr_layer_shell_v1::ZwlrLayerShellV1, zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
        },
    }, registry::{ProvidesRegistryState, RegistryState}, seat::{Capability, SeatHandler, SeatState, pointer::{PointerEvent, PointerEventKind, PointerHandler}}, shell::{WaylandSurface, wlr_layer::{
        Anchor as WlrAnchor,
        KeyboardInteractivity,
        Layer,
        LayerShell,
        LayerShellHandler,
        LayerSurface,
        LayerSurfaceConfigure,
        LayerSurfaceData,
    }}, shm::{Shm, ShmHandler}
};
use std::{collections::{HashMap, hash_map::Entry}, ops::Deref, ptr::NonNull, sync::{Arc, Mutex}};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event as LayerSurfaceEvent;
use crate::ChannelWrapper;

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
    SurfaceResized {
        surface_id: ObjectId,
        state: SurfaceState,
    },
    SurfaceRemoved {
        surface_id: ObjectId,
    },
}

pub type OutputId = ObjectId;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceState {
    pub size: Option<PhysicalSize>,
}

pub struct ClientState {
    pub output_state: OutputState,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
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
    ) -> Self {
        Self {
            output_state,
            registry_state,
            seat_state,
            pointer: None,
            event_channel: ChannelWrapper::default(),
            exit_signal: None,
            surface_state: HashMap::new(),
        }
    }

    pub fn set_surface_size(&mut self, surface_id: ObjectId, size: PhysicalSize) {
        self.event_channel
            .send(WaylandEvent::SurfaceResized {
                surface_id: surface_id.clone(),
                state: SurfaceState { size: Some(size) },
            })
            .unwrap();

        match self.surface_state.entry(surface_id) {
            Entry::Occupied(mut entry) => entry.get_mut().size = Some(size),
            Entry::Vacant(entry) => {
                entry.insert(SurfaceState { size: Some(size) });
            }
        }
    }

    pub fn get_surface_size(&self, surface_id: &ObjectId) -> Option<PhysicalSize> {
        self.surface_state.get(surface_id).and_then(|s| s.size)
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

impl Dispatch<WlCompositor, GlobalData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: <WlCompositor as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShm, GlobalData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: <WlShm as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, GlobalData> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: <ZwlrLayerShellV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl LayerShellHandler for ClientState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {}

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        _: LayerSurfaceConfigure,
        _: u32,
    ) {
    }
}

impl OutputHandler for ClientState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.event_channel
            .send(WaylandEvent::Output(OutputEvent::Added(output)))
            .unwrap();

        slint::invoke_from_event_loop(crate::start_window::show).unwrap();
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

impl Dispatch<WlSurface, SurfaceData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: <WlSurface as Proxy>::Event,
        _: &SurfaceData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, LayerSurfaceData> for ClientState {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        data: &LayerSurfaceData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let surface_id = data.layer_surface().unwrap().wl_surface().id();

        match event {
            // FIXME(hack3rmann): hardcoded height
            LayerSurfaceEvent::Configure {
                serial,
                width,
                height: _,
            } => {
                surface.set_exclusive_zone(40);
                surface.set_anchor(Anchor::Top);
                surface.ack_configure(serial);

                state.set_surface_size(surface_id, PhysicalSize { width, height: 40 });
            }
            LayerSurfaceEvent::Closed => state.remove_surface(&surface_id),
            _ => unimplemented!(),
        }
    }
}

impl ShmHandler for ClientState {
    fn shm_state(&mut self) -> &mut Shm {
        todo!()
    }
}

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
            let position = LogicalPosition::new(*x as f32, *y as f32);

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

pub struct WaylandInner {
    pub connection: Connection,
    pub event_queue: Mutex<Option<EventQueue<ClientState>>>,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
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
        let shm_state = Shm::bind(&globals, &qh).unwrap();
        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);

        let mut client_state = ClientState::new(output_state, registry_state, seat_state);

        event_queue.roundtrip(&mut client_state).unwrap();

        // NOTE(hack3rmann): interior mutability does not affect the hash here
        #[allow(clippy::mutable_key_type)]
        let windows = client_state
            .output_state
            .outputs()
            .map(|output| {
                let surface = compositor_state.create_surface::<ClientState>(&qh);
                let layer_surface = layer_shell.create_layer_surface::<ClientState>(
                    &qh,
                    surface.clone(),
                    Layer::Bottom,
                    Some("waybruh"),
                    Some(&output),
                );

                let (output_width, _) = client_state
                    .output_state
                    .info(&output)
                    .unwrap()
                    .logical_size
                    .unwrap();

                layer_surface.set_exclusive_zone(0);
                layer_surface.set_anchor(WlrAnchor::all());
                layer_surface.set_margin(0, 0, 0, 0);
                layer_surface.set_size(output_width as u32, 40);
                layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

                surface.commit();

                (
                    output.id(),
                    Arc::new(SurfaceBundle {
                        surface,
                        layer_surface,
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
            shm_state,
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
