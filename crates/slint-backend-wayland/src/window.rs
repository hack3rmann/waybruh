use crate::wayland::Wayland;
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use slint::{
    PhysicalSize, Window,
    platform::{Renderer, WindowAdapter},
};
use smithay_client_toolkit::reexports::client::{Proxy as _, protocol::wl_output::WlOutput};
use std::rc::{Rc, Weak};

pub struct SlintWindowAdapter {
    pub window: Window,
    pub renderer: SkiaRenderer,
    pub wayland: Wayland,
    pub output: WlOutput,
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
