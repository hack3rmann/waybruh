use crate::{event_loop::PlatformSharedState, wayland::Wayland};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use slint::{
    PhysicalSize, Window,
    platform::{Renderer, WindowAdapter},
};
use smithay_client_toolkit::reexports::client::{
    Proxy as _,
    protocol::{wl_output::WlOutput, wl_surface::WlSurface},
};
use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

pub struct SlintWindowAdapter {
    pub window: Window,
    pub renderer: SkiaRenderer,
    pub wayland: Wayland,
    pub output: WlOutput,
    pub surface: WlSurface,
    pub state: Arc<PlatformSharedState>,
}

impl SlintWindowAdapter {
    pub fn new(
        state: Arc<PlatformSharedState>,
        wayland: Wayland,
        surface: WlSurface,
        output: WlOutput,
        skia_context: &SkiaSharedContext,
    ) -> Rc<Self> {
        let renderer = SkiaRenderer::default_wgpu_28(skia_context);

        let display_handle = wayland.display_handle();
        let window_handle = wayland.window_handle(&output.id()).unwrap();
        let size = state.window_size(&surface.id()).unwrap();

        renderer
            .set_window_handle(window_handle, display_handle, size, None)
            .unwrap();

        Rc::new_cyclic(move |weak: &Weak<Self>| Self {
            window: Window::new(Weak::clone(weak) as Weak<dyn WindowAdapter>),
            state,
            renderer,
            wayland,
            output,
            surface,
        })
    }
}

impl WindowAdapter for SlintWindowAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.state.window_size(&self.surface.id()).unwrap()
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}
