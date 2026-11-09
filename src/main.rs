// Rust fractal viewer with GUI, zoom/pan, trippy color effects, low-res preview, debounced rendering, and double-click zoom

use eframe::{egui, App};
use egui::{ColorImage, TextureOptions};
use image::{ImageBuffer, Rgb};
use num_complex::Complex;
use rayon::prelude::*;
use std::time::{Duration, Instant};

mod gpu;

#[derive(Copy, Clone, PartialEq)]
pub enum FractalType {
    Mandelbrot,
    Julia,
    BurningShip,
}

#[derive(Copy, Clone)]
struct FractalParams {
    fractal_type: FractalType,
    max_iterations: u32,
    color_offset: u8,
    julia_re: f64,
    julia_im: f64,
}

impl Default for FractalParams {
    fn default() -> Self {
        Self {
            fractal_type: FractalType::Mandelbrot,
            max_iterations: 100,
            color_offset: 0,
            julia_re: -0.8,
            julia_im: 0.156,
        }
    }
}

fn iterate_fractal(cx: f64, cy: f64, params: &FractalParams) -> u32 {
    let (c, mut z) = match params.fractal_type {
        FractalType::Mandelbrot => (Complex::new(cx, cy), Complex::new(0.0, 0.0)),
        FractalType::Julia => (
            Complex::new(params.julia_re, params.julia_im),
            Complex::new(cx, cy),
        ),
        FractalType::BurningShip => (Complex::new(cx, cy), Complex::new(0.0, 0.0)),
    };
    let mut i = 0u32;
    while i < params.max_iterations && z.norm_sqr() <= 4.0 {
        match params.fractal_type {
            FractalType::BurningShip => {
                z = Complex::new(z.re.abs(), z.im.abs());
                z = z * z + c;
            }
            _ => {
                z = z * z + c;
            }
        }
        i += 1;
    }
    i
}

fn color_from_iterations(i: u32, max_iterations: u32, color_offset: u8) -> Rgb<u8> {
    let t = i as f32 / max_iterations as f32;
    let r = ((9.0 * (1.0 - t) * t * t * t) * 255.0) as u8;
    let g = ((15.0 * (1.0 - t) * (1.0 - t) * t * t) * 255.0) as u8;
    let b = ((8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t) * 255.0) as u8;

    Rgb([
        r.wrapping_add(color_offset),
        g.wrapping_add(color_offset),
        b.wrapping_add(color_offset),
    ])
}

pub struct FractalApp {
    zoom: f64,
    center_x: f64,
    center_y: f64,
    width: usize,
    height: usize,
    texture_handle: Option<egui::TextureHandle>,
    params: FractalParams,
    last_drag_pos: Option<egui::Pos2>,
    last_interaction: Instant,
    is_interacting: bool,
    render_low_res: bool,
    /// True when `gpu::init` found a wgpu backend; `ui` then uses the GPU paint callback.
    gpu_active: bool,
    /// Bottom edge of the control bar in points (updated per frame; replaces the +32.0 guess).
    panel_bottom: f32,
}

impl Default for FractalApp {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            center_x: -0.5,
            center_y: 0.0,
            width: 800,
            height: 600,
            texture_handle: None,
            params: FractalParams::default(),
            last_drag_pos: None,
            last_interaction: Instant::now(),
            is_interacting: false,
            render_low_res: false,
            gpu_active: false,
            panel_bottom: 32.0,
        }
    }
}

impl FractalApp {
    /// Create the app: probe the wgpu backend (Phase 4A) and remember whether the
    /// GPU paint callback (Phase 4C) is usable; otherwise `ui` falls back to the CPU path.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let gpu_active = match gpu::init(cc) {
            Ok(()) => {
                eprintln!("GPU backend active");
                true
            }
            Err(e) => {
                eprintln!("GPU backend unavailable, using CPU renderer: {e:?}");
                false
            }
        };
        Self {
            gpu_active,
            ..Self::default()
        }
    }
}

impl App for FractalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let input = ui.ctx().input(|i| i.clone());
        let panel_top = self.panel_bottom;
        let mut in_viewport = false;
        if let Some(pos) = input.pointer.interact_pos() {
            in_viewport = pos.y > panel_top;
        }

        // Detect interaction
        let mut interaction_happened = false;

        // Touch input: pinch to zoom (using egui's MultiTouchInfo)
        if let Some(touch) = input.multi_touch() {
            // Use zoom_delta_2d for pinch zoom (f32)
            let zoom_vec = touch.zoom_delta_2d;
            // If zoom_vec is not the default (1.0, 1.0), apply zoom
            if in_viewport && zoom_vec != egui::Vec2::splat(1.0) {
                // Use the y component for zooming (conventionally, pinch zoom is on y)
                self.zoom *= zoom_vec.y as f64;
                interaction_happened = true;
            }
        }

        // Double-click zoom
        if let Some(pos) = input.pointer.interact_pos() {
            if pos.y > panel_top
                && input.pointer.any_click()
                && input
                    .pointer
                    .button_double_clicked(egui::PointerButton::Primary)
            {
                let norm_x = (pos.x / self.width as f32 - 0.5) * 2.0;
                let norm_y = (pos.y / self.height as f32 - 0.5) * 2.0;
                let dx = norm_x as f64 / self.zoom;
                let dy = norm_y as f64 / self.zoom;
                self.center_x += dx;
                self.center_y += dy;

                let zoom_factor = if input.modifiers.shift { 0.5 } else { 2.0 };
                self.zoom *= zoom_factor;
                interaction_happened = true;
            }
        }

        // Mouse wheel zoom
        if in_viewport && input.smooth_scroll_delta.y != 0.0 {
            let zoom_amount = 1.1_f64.powf(input.smooth_scroll_delta.y as f64 / 100.0);
            self.zoom *= zoom_amount;
            interaction_happened = true;
        }

        // Mouse drag to pan
        if in_viewport && input.pointer.primary_down() {
            if let Some(pos) = input.pointer.interact_pos() {
                if let Some(last_pos) = self.last_drag_pos {
                    let delta = pos - last_pos;
                    self.center_x -= delta.x as f64 / (self.zoom * self.width as f64 / 2.0);
                    self.center_y -= delta.y as f64 / (self.zoom * self.height as f64 / 2.0);
                    interaction_happened = true;
                }
                self.last_drag_pos = Some(pos);
            }
        } else {
            self.last_drag_pos = None;
        }

        // Handle interaction timing and resolution switching
        if interaction_happened {
            self.last_interaction = Instant::now();
            self.is_interacting = true;
            self.render_low_res = true;
            self.texture_handle = None;
        } else if self.is_interacting && self.last_interaction.elapsed() > Duration::from_millis(25)
        {
            self.is_interacting = false;
            self.render_low_res = false;
            self.texture_handle = None;
        }

        let top_bar = egui::Panel::top("controls").show(ui, |ui| {
            // Outer wrap lets whole label+control groups drop to a new row on narrow
            // windows; inner `ui.horizontal` groups keep each label glued to its control.
            ui.horizontal_wrapped(|ui| {
                if ui.button("💾 Save PNG").clicked() {
                    save_fractal(
                        self.center_x,
                        self.center_y,
                        self.zoom,
                        self.params,
                        self.width,
                        self.height,
                    );
                }

                if ui.button("🔄 Reset").clicked() {
                    self.params = FractalParams::default();
                    self.center_x = -0.5;
                    self.center_y = 0.0;
                    self.zoom = 1.0;
                    self.texture_handle = None;
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("🎨 Color Offset:");
                    if ui
                        .add(egui::Slider::new(&mut self.params.color_offset, 0..=255))
                        .changed()
                    {
                        self.texture_handle = None;
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("🔢 Fractal Type:");
                    egui::ComboBox::from_id_salt("fractal_type")
                        .selected_text(match self.params.fractal_type {
                            FractalType::Mandelbrot => "Mandelbrot",
                            FractalType::Julia => "Julia",
                            FractalType::BurningShip => "Burning Ship",
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    matches!(self.params.fractal_type, FractalType::Mandelbrot),
                                    "Mandelbrot",
                                )
                                .clicked()
                            {
                                self.params.fractal_type = FractalType::Mandelbrot;
                                self.texture_handle = None;
                            }
                            if ui
                                .selectable_label(
                                    matches!(self.params.fractal_type, FractalType::Julia),
                                    "Julia",
                                )
                                .clicked()
                            {
                                self.params.fractal_type = FractalType::Julia;
                                self.texture_handle = None;
                            }
                            if ui
                                .selectable_label(
                                    matches!(self.params.fractal_type, FractalType::BurningShip),
                                    "Burning Ship",
                                )
                                .clicked()
                            {
                                self.params.fractal_type = FractalType::BurningShip;
                                self.texture_handle = None;
                            }
                        });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("🔁 Iterations:");
                    if ui
                        .add(egui::Slider::new(
                            &mut self.params.max_iterations,
                            10..=1000,
                        ))
                        .changed()
                    {
                        self.texture_handle = None;
                    }
                });

                if self.params.fractal_type == FractalType::Julia {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("➰ Julia Re:");
                        if ui
                            .add(egui::Slider::new(&mut self.params.julia_re, -1.5..=1.5))
                            .changed()
                        {
                            self.texture_handle = None;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("➰ Julia Im:");
                        if ui
                            .add(egui::Slider::new(&mut self.params.julia_im, -1.5..=1.5))
                            .changed()
                        {
                            self.texture_handle = None;
                        }
                    });
                }
            });
        });
        self.panel_bottom = top_bar.response.rect.max.y;

        egui::CentralPanel::default().show(ui, |ui| {
            let available = ui.available_size();
            self.width = available.x.max(1.0) as usize;
            self.height = available.y.max(1.0) as usize;

            if self.gpu_active {
                // Phase 4C: hand the frame to the GPU paint callback. The rect is in
                // points; GpuParams carries the physical-pixel transform so the shader
                // can map fragCoord back onto the complex plane.
                let ppu = ui.ctx().pixels_per_point();
                let rect = ui.available_rect_before_wrap();
                let fractal_type = match self.params.fractal_type {
                    FractalType::Mandelbrot => 1,
                    FractalType::Julia => 0,
                    FractalType::BurningShip => 2,
                };
                let params = gpu::GpuParams {
                    fractal_type,
                    max_iterations: self.params.max_iterations,
                    color_offset: self.params.color_offset as u32,
                    julia_re: self.params.julia_re as f32,
                    julia_im: self.params.julia_im as f32,
                    center_x: self.center_x as f32,
                    center_y: self.center_y as f32,
                    zoom: self.zoom as f32,
                    origin_x: rect.min.x * ppu,
                    origin_y: rect.min.y * ppu,
                    width: rect.width() * ppu,
                    height: rect.height() * ppu,
                };
                let callback = eframe::egui_wgpu::Callback::new_paint_callback(
                    rect,
                    gpu::GpuFractal { params },
                );
                // `new_paint_callback` already returns a finished
                // `epaint::PaintCallback` whose type-erased `callback` field
                // is `Arc<egui_wgpu::Callback>`. Wrap it a second time and
                // the renderer's `downcast_ref::<Callback>()` (egui-wgpu
                // `Renderer::update_buffers`/`render`) silently fails and
                // neither `prepare` nor `paint` ever runs -> black screen.
                ui.painter().add(egui::Shape::Callback(callback));
                return;
            }

            // CPU fallback path (also the reference path used by Save PNG).
            let render_width = if self.render_low_res {
                self.width / 2
            } else {
                self.width
            };
            let render_height = if self.render_low_res {
                self.height / 2
            } else {
                self.height
            };

            if self.texture_handle.is_none() {
                self.texture_handle = Some(render_fractal_to_texture_by_type(
                    ui.ctx(),
                    self.center_x,
                    self.center_y,
                    self.zoom,
                    self.params,
                    render_width,
                    render_height,
                ));
            }

            if let Some(tex) = &self.texture_handle {
                let tex_size = egui::vec2(self.width as f32, self.height as f32);
                ui.add(egui::Image::from_texture(tex).fit_to_exact_size(tex_size));
            } else {
                ui.label("No texture loaded");
            }
        });
    }

    // --- End of App trait impl ---
}

fn render_fractal_to_texture_by_type(
    ctx: &egui::Context,
    center_x: f64,
    center_y: f64,
    zoom: f64,
    params: FractalParams,
    width: usize,
    height: usize,
) -> egui::TextureHandle {
    let mut imgbuf = ImageBuffer::new(width as u32, height as u32);

    imgbuf
        .enumerate_pixels_mut()
        .par_bridge()
        .for_each(|(x, y, pixel)| {
            let cx = (x as f64 - width as f64 / 2.0) / (0.5 * zoom * width as f64) + center_x;
            let cy = (y as f64 - height as f64 / 2.0) / (0.5 * zoom * height as f64) + center_y;
            let i = iterate_fractal(cx, cy, &params);
            *pixel = color_from_iterations(i, params.max_iterations, params.color_offset);
        });

    let size = [width, height];
    let pixels: Vec<_> = imgbuf
        .pixels()
        .flat_map(|p| vec![p[0], p[1], p[2], 255])
        .collect();

    let image = ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture("fractal", image, TextureOptions::NEAREST)
}

fn save_fractal(
    center_x: f64,
    center_y: f64,
    zoom: f64,
    params: FractalParams,
    width: usize,
    height: usize,
) {
    let mut imgbuf = ImageBuffer::new(width as u32, height as u32);

    imgbuf
        .enumerate_pixels_mut()
        .par_bridge()
        .for_each(|(x, y, pixel)| {
            let cx = (x as f64 - width as f64 / 2.0) / (0.5 * zoom * width as f64) + center_x;
            let cy = (y as f64 - height as f64 / 2.0) / (0.5 * zoom * height as f64) + center_y;
            let i = iterate_fractal(cx, cy, &params);
            *pixel = color_from_iterations(i, params.max_iterations, params.color_offset);
        });

    let name = match params.fractal_type {
        FractalType::Mandelbrot => "mandelbrot",
        FractalType::Julia => "julia",
        FractalType::BurningShip => "burningship",
    };
    let filename = format!(
        "{}_{:.2}_{:.2}_z{:.2}_c{}.png",
        name, center_x, center_y, zoom, params.color_offset
    );
    match imgbuf.save(&filename) {
        Ok(()) => println!("Saved image to: {}", filename),
        Err(e) => println!("Failed to save image to {}: {}", filename, e),
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(800.0, 600.0))
            .with_min_inner_size(egui::vec2(480.0, 320.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Fractal Explorer",
        options,
        Box::new(|cc| Ok(Box::new(FractalApp::new(cc)))),
    )
}
