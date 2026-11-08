//! GPU fractal rendering via an egui-wgpu paint callback (Phase 4).
//!
//! [`init`] builds a fullscreen-triangle render pipeline from
//! `cc.wgpu_render_state` and caches it, along with its uniform buffer and
//! bind group, as a [`GpuResources`] value in the egui-wgpu renderer's
//! `callback_resources` TypeMap. Each frame, `main.rs` constructs a fresh
//! [`GpuFractal`] carrying that frame's [`GpuParams`] and registers it as an
//! `egui_wgpu::Callback`; [`GpuFractal::prepare`] writes those params into
//! the cached uniform buffer and [`GpuFractal::paint`] draws the fullscreen
//! triangle whose fragment shader mirrors the CPU `iterate_fractal` /
//! `color_from_iterations` math in `main.rs`.

use eframe::egui_wgpu::{self, CallbackResources, ScreenDescriptor};

/// Per-frame fractal state, pushed to the GPU via a uniform buffer.
/// `#[repr(C)]` — LAYOUT IS FROZEN (Phase 4 contract); the WGSL uniform
/// struct below must match field-for-field in order, name and type. All
/// fields are 4 bytes wide (`u32`/`f32`) and sequentially packed, so the
/// struct is 48 bytes with no padding on either side.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParams {
    /// 0 = Julia, 1 = Mandelbrot, 2 = BurningShip (matches `main.rs`'s wiring,
    /// not `FractalType`'s declaration order)
    pub fractal_type: u32,
    pub max_iterations: u32,
    /// `u8` widened to `u32`
    pub color_offset: u32,
    pub julia_re: f32,
    pub julia_im: f32,
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
    /// Viewport top-left in PHYSICAL pixels of the swapchain
    pub origin_x: f32,
    pub origin_y: f32,
    /// Fractal viewport size in physical pixels
    pub width: f32,
    pub height: f32,
}

/// The paint callback. Holds only the current frame's params; all GPU objects
/// live in [`CallbackResources`] (TypeMap), created once by [`init`].
#[derive(Copy, Clone, Default)]
pub struct GpuFractal {
    pub params: GpuParams,
}

impl egui_wgpu::CallbackTrait for GpuFractal {
    /// Called per frame before egui's render pass: push
    /// [`GpuFractal::params`] into the cached uniform buffer.
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(resources) = callback_resources.get_mut::<GpuResources>() {
            queue.write_buffer(
                &resources.uniform_buffer,
                0,
                bytemuck::bytes_of(&self.params),
            );
        }
        Vec::new()
    }

    /// Called inside egui's render pass: draw the fullscreen triangle.
    fn paint(
        &self,
        _info: egui::epaint::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let Some(resources) = callback_resources.get::<GpuResources>() else {
            // init() never ran (or failed) for this renderer; nothing to draw.
            return;
        };
        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_bind_group(0, &resources.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

/// Persistent GPU objects, created once by [`init`] and cached in the
/// renderer's `callback_resources` TypeMap for [`GpuFractal::prepare`] and
/// [`GpuFractal::paint`] to use every frame.
struct GpuResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

/// WGSL source for the fractal shader: a vertex stage that emits a
/// fullscreen triangle (no vertex buffers) and a fragment stage that mirrors
/// `iterate_fractal`/`color_from_iterations` in `main.rs`.
const SHADER_SRC: &str = r#"
struct FractalParams {
    fractal_type: u32,
    max_iterations: u32,
    color_offset: u32,
    julia_re: f32,
    julia_im: f32,
    center_x: f32,
    center_y: f32,
    zoom: f32,
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
};

@group(0) @binding(0)
var<uniform> params: FractalParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // wgpu's frag_coord is pixel-center: (origin + i + 0.5). Recover the
    // viewport-local pixel index `i`/`j` that main.rs's CPU renderer uses
    // directly (as an integer, no +0.5) so the two mappings line up exactly.
    let px = frag_coord.x - params.origin_x - 0.5;
    let py = frag_coord.y - params.origin_y - 0.5;

    let cx = (px - 0.5 * params.width) / (0.5 * params.zoom * params.width) + params.center_x;
    let cy = (py - 0.5 * params.height) / (0.5 * params.zoom * params.height) + params.center_y;

    var z: vec2<f32>;
    var c: vec2<f32>;
    if (params.fractal_type == 0u) {
        // Julia: c is the fixed Julia constant, z starts at the pixel's complex coordinate.
        c = vec2<f32>(params.julia_re, params.julia_im);
        z = vec2<f32>(cx, cy);
    } else {
        // Mandelbrot and Burning Ship: c is the pixel's complex coordinate, z starts at 0.
        c = vec2<f32>(cx, cy);
        z = vec2<f32>(0.0, 0.0);
    }

    // Defensive: the UI slider never goes below 10, but guard the division
    // below so a 0 value can't produce NaN instead of a sane fallback color.
    if (params.max_iterations == 0u) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var i: u32 = 0u;
    while (i < params.max_iterations && (z.x * z.x + z.y * z.y) <= 4.0) {
        if (params.fractal_type == 2u) {
            // Burning Ship: take the absolute value of z BEFORE squaring.
            z = vec2<f32>(abs(z.x), abs(z.y));
        }
        z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        i = i + 1u;
    }

    let t = f32(i) / f32(params.max_iterations);
    let one_minus_t = 1.0 - t;

    let r = 9.0 * one_minus_t * t * t * t * 255.0;
    let g = 15.0 * one_minus_t * one_minus_t * t * t * 255.0;
    let b = 8.5 * one_minus_t * one_minus_t * one_minus_t * t * 255.0;

    // Matches `color_from_iterations` in main.rs: a saturating float->u8 cast
    // (clamp to [0, 255] then truncate) followed by `wrapping_add(color_offset)`,
    // i.e. mod-256 wraparound, reproduced here with u32 arithmetic.
    let r_u8 = (u32(clamp(r, 0.0, 255.0)) + params.color_offset) % 256u;
    let g_u8 = (u32(clamp(g, 0.0, 255.0)) + params.color_offset) % 256u;
    let b_u8 = (u32(clamp(b, 0.0, 255.0)) + params.color_offset) % 256u;

    return vec4<f32>(f32(r_u8) / 255.0, f32(g_u8) / 255.0, f32(b_u8) / 255.0, 1.0);
}
"#;

/// One-time GPU init: build the shader module, pipeline layout, bind group
/// layout, pipeline, uniform buffer and bind group from
/// `cc.wgpu_render_state`, and insert them into
/// `render_state.renderer.write().callback_resources` — the same TypeMap later
/// passed to `prepare`/`paint`.
///
/// Contract: called exactly once from `FractalApp::new`. Returns `Ok(())` only
/// when the wgpu backend is active AND init succeeded; `Err` otherwise (the
/// app stays on the CPU renderer).
pub fn init(cc: &eframe::CreationContext<'_>) -> Result<(), String> {
    let render_state = cc
        .wgpu_render_state
        .as_ref()
        .ok_or_else(|| "wgpu backend not active".to_string())?;

    let device = &render_state.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fractal-shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fractal-bind-group-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fractal-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fractal-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: render_state.target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fractal-uniform-buffer"),
        size: std::mem::size_of::<GpuParams>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fractal-bind-group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    render_state
        .renderer
        .write()
        .callback_resources
        .insert(GpuResources {
            pipeline,
            bind_group,
            uniform_buffer,
        });

    Ok(())
}
