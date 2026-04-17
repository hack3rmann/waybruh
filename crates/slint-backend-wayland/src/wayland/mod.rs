use smithay_client_toolkit::{
    compositor::CompositorState,
    globals::GlobalData,
    output::{OutputData, OutputState},
    reexports::{
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            backend::ObjectId,
            globals::{GlobalListContents, registry_queue_init},
            protocol::{
                wl_compositor::WlCompositor, wl_output::WlOutput, wl_registry::WlRegistry,
                wl_shm::WlShm,
            },
        },
        protocols::xdg::xdg_output::zv1::client::{
            zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
        },
        protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1,
    },
    registry::RegistryState,
    shell::wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{Shm, ShmHandler},
};
use std::{ops::Deref, sync::Arc};

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

impl ShmHandler for ClientState {
    fn shm_state(&mut self) -> &mut Shm {
        todo!()
    }
}

pub struct WaylandInner {
    pub registry_state: RegistryState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub layer_shell: LayerShell,
    pub output_state: OutputState,
}

impl WaylandInner {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let connection = Connection::connect_to_env().unwrap();
        let (globals, event_queue) = registry_queue_init::<ClientState>(&connection).unwrap();
        let queue_handle = event_queue.handle();

        let registry_state = RegistryState::new(&globals);

        let compositor_state = CompositorState::bind(&globals, &queue_handle).unwrap();
        let shm_state = Shm::bind(&globals, &queue_handle).unwrap();

        let layer_shell = LayerShell::bind(&globals, &queue_handle).unwrap();

        let output_state = OutputState::new(&globals, &queue_handle);

        Self {
            registry_state,
            compositor_state,
            shm_state,
            layer_shell,
            output_state,
        }
    }
}

#[derive(Clone)]
pub struct Wayland(Arc<WaylandInner>);

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
