//! GPU fractal rendering via an egui-wgpu paint callback (Phase 4).
//!
//! Architecture: [`GpuFractal`] is registered per frame as an
//! `egui_wgpu::Callback` paint callback. All GPU objects (shader, pipeline,
//! uniform buffer, bind group) are created once by [`init`] and cached in the
//! renderer's [`CallbackResources`] TypeMap. `prepare` pushes the current
//! frame's [`GpuParams`] into the uniform buffer; `paint` draws a fullscreen
//! triangle whose fragment shader mirrors the CPU iteration/palette math.

use eframe::egui_wgpu::{self, CallbackResources, ScreenDescriptor};

/// Per-frame fractal state, pushed to the GPU.
/// `#[repr(C)]` — LAYOUT IS FROZEN (Phase 4 contract); the WGSL uniform
/// struct must match field-for-field in order, name and type.
#[allow(dead_code)] // wired into main.rs in Phase 4C
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParams {
    /// 0 = Mandelbrot, 1 = Julia, 2 = BurningShip
    pub fractal_type: u32,
    pub max_iterations: u32,
    /// `u8` widened to `u32`
    pub color_offset: u32,
    pub julia_re: f32,
    pub julia_im: f32,
    pub center_re: f32,
    pub center_im: f32,
    pub zoom: f32,
    /// Viewport top-left in PHYSICAL pixels of the swapchain
    pub origin_x: f32,
    pub origin_y: f32,
    /// Fractal viewport size in physical pixels
    pub width: u32,
    pub height: u32,
}

/// The paint callback. Holds only the current frame's params; all GPU objects
/// live in [`CallbackResources`] (TypeMap), created once by [`init`].
#[allow(dead_code)] // wired into main.rs in Phase 4C
#[derive(Copy, Clone, Default)]
pub struct GpuFractal {
    pub params: GpuParams,
}

#[allow(dead_code)] // wired into main.rs in Phase 4C
impl GpuFractal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_params(&mut self, params: GpuParams) {
        self.params = params;
    }
}

impl egui_wgpu::CallbackTrait for GpuFractal {
    /// Called per frame before egui's render pass: push
    /// [`GpuFractal::params`] into the cached uniform buffer.
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        // Part B: queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&self.params));
        Vec::new()
    }

    /// Called inside egui's render pass: draw the fullscreen triangle.
    fn paint(
        &self,
        _info: egui::epaint::PaintCallbackInfo,
        _render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &CallbackResources,
    ) {
        // Part B: set_pipeline / set_bind_group / draw(0..3, 0..1).
    }
}

/// One-time GPU init: build the shader module, pipeline layout, bind group
/// layout, pipeline, uniform buffer and bind group from
/// `cc.wgpu_render_state`, and insert them into
/// `render_state.renderer.write().callback_resources` — the same TypeMap later
/// passed to `prepare`/`paint`.
///
/// Contract: called exactly once from `FractalApp::new`. Returns `Ok(())` only
/// when the wgpu backend is active AND init succeeded; `Err` otherwise (the
/// app stays on the CPU renderer).
pub fn init(cc: &eframe::CreationContext) -> eframe::Result<()> {
    if cc.wgpu_render_state.is_some() {
        // Part B: build + cache GPU resources here.
        Ok(())
    } else {
        Err(eframe::Error::AppCreation(
            std::io::Error::other("wgpu backend not active").into(),
        ))
    }
}
