use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use slint::PhysicalSize;
use smithay_client_toolkit::{
    compositor::{CompositorState, SurfaceData},
    globals::GlobalData,
    output::{OutputData, OutputState},
    reexports::{
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            backend::ObjectId,
            globals::{GlobalListContents, registry_queue_init},
            protocol::{
                wl_compositor::WlCompositor, wl_output::WlOutput, wl_registry::WlRegistry,
                wl_shm::WlShm, wl_surface::WlSurface,
            },
        },
        protocols::xdg::xdg_output::zv1::client::{
            zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
        },
        protocols_wlr::layer_shell::v1::client::{
            zwlr_layer_shell_v1::ZwlrLayerShellV1, zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
        },
    },
    registry::RegistryState,
    shell::wlr_layer::{
        Anchor as WlrAnchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure, LayerSurfaceData
    },
    shm::{Shm, ShmHandler},
};
use std::{collections::HashMap, ops::Deref, ptr::NonNull, sync::Arc};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event as LayerSurfaceEvent;

pub type OutputId = ObjectId;

pub struct ClientState;

impl Dispatch<WlRegistry, GlobalListContents> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
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

impl Dispatch<WlOutput, OutputData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        _: <WlOutput as Proxy>::Event,
        _: &OutputData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZxdgOutputV1, OutputData> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZxdgOutputV1,
        _: <ZxdgOutputV1 as Proxy>::Event,
        _: &OutputData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZxdgOutputManagerV1, GlobalData> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZxdgOutputManagerV1,
        _: <ZxdgOutputManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

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
        _: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _: &LayerSurfaceData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            LayerSurfaceEvent::Configure {
                serial,
                width,
                height,
            } => {
                surface.set_size(width, height);
                surface.set_exclusive_zone(40);
                surface.set_anchor(Anchor::Top);
                surface.ack_configure(serial);
            }
            LayerSurfaceEvent::Closed => todo!(),
            _ => todo!(),
        }
    }
}

impl ShmHandler for ClientState {
    fn shm_state(&mut self) -> &mut Shm {
        todo!()
    }
}

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
    pub registry_state: RegistryState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub layer_shell: LayerShell,
    pub output_state: OutputState,
    pub windows: HashMap<ObjectId, Arc<SurfaceBundle>>,
}

impl WaylandInner {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let connection = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) = registry_queue_init::<ClientState>(&connection).unwrap();
        let qh = event_queue.handle();

        let mut state = ClientState;

        let registry_state = RegistryState::new(&globals);

        let compositor_state = CompositorState::bind(&globals, &qh).unwrap();
        let shm_state = Shm::bind(&globals, &qh).unwrap();

        let layer_shell = LayerShell::bind(&globals, &qh).unwrap();

        let output_state = OutputState::new(&globals, &qh);

        // NOTE(hack3rmann): interior mutability does not affect the hash here
        #[allow(clippy::mutable_key_type)]
        let windows = output_state
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

                layer_surface.set_exclusive_zone(0);
                layer_surface.set_anchor(WlrAnchor::all());
                layer_surface.set_margin(0, 0, 0, 0);
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

        event_queue.blocking_dispatch(&mut state).unwrap();

        Self {
            connection,
            registry_state,
            compositor_state,
            shm_state,
            layer_shell,
            output_state,
            windows,
        }
    }

    pub fn output_size(&self, output: &WlOutput) -> Option<PhysicalSize> {
        self.output_state
            .info(output)
            .map(|i| i.physical_size)
            .map(|(w, h)| PhysicalSize::new(u32::try_from(w).unwrap(), u32::try_from(h).unwrap()))
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
