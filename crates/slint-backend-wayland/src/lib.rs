pub mod wayland;

use crate::wayland::Wayland;
use core::time::Duration;
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use lazy_static::lazy_static;
use slint::{
    PhysicalSize, PlatformError, Window,
    platform::{Platform, Renderer, WindowAdapter},
};
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use std::{
    rc::{Rc, Weak},
    time::Instant,
};

pub struct WaylandBackend {
    wayland: Wayland,
}

lazy_static! {
    pub static ref INITIAL_INSTANT: Instant = Instant::now();
}

impl Platform for WaylandBackend {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let output = self.wayland.output_state.outputs().next().unwrap();

        Ok(SlintWindowAdapter::new(
            self.wayland.clone(),
            output,
            &SkiaSharedContext::default(),
        ))
    }

    fn duration_since_start(&self) -> Duration {
        INITIAL_INSTANT.elapsed()
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
        let renderer = SkiaRenderer::default_vulkan(skia_context);

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
        self.wayland
            .output_state
            .info(&self.output)
            .map(|i| i.physical_size)
            .map(|(w, h)| PhysicalSize::new(u32::try_from(w).unwrap(), u32::try_from(h).unwrap()))
            .unwrap_or_default()
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}
