// Rust fractal viewer with GUI, zoom/pan, trippy color effects, low-res preview, debounced rendering, and double-click zoom

use eframe::{egui, App};
use egui::{ColorImage, TextureOptions};
use image::{ImageBuffer, Rgb};
use num_complex::Complex;
use rayon::prelude::*;
use std::time::{Duration, Instant};

#[derive(Copy, Clone, PartialEq)]
pub enum FractalType {
    Mandelbrot,
    Julia,
    BurningShip,
}

pub struct FractalApp {
    zoom: f64,
    center_x: f64,
    center_y: f64,
    width: usize,
    height: usize,
    texture_handle: Option<egui::TextureHandle>,
    color_offset: u8,
    last_drag_pos: Option<egui::Pos2>,
    last_interaction: Instant,
    is_interacting: bool,
    render_low_res: bool,
    fractal_type: FractalType,
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
            color_offset: 0,
            last_drag_pos: None,
            last_interaction: Instant::now(),
            is_interacting: false,
            render_low_res: false,
            fractal_type: FractalType::Mandelbrot,
        }
    }
}

#[allow(non_snake_case)]
impl App for FractalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let input = ctx.input(|i| i.clone());
        let mut in_viewport = false;
        match input.pointer.interact_pos() {
            Some(pos) => {
                let panel_top = ctx.screen_rect().top() + 32.0;
                in_viewport = pos.y > panel_top;
            }
            None => {}
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
        match input.pointer.interact_pos() {
            Some(pos) => {
                let panel_top = ctx.screen_rect().top() + 32.0; // rough top bar height
                if pos.y > panel_top && input.pointer.any_click() && input.pointer.button_double_clicked(egui::PointerButton::Primary) {
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
            None => {}
        }

        // Mouse wheel zoom
        if in_viewport && input.raw_scroll_delta.y != 0.0 {
            let zoom_amount = 1.1_f64.powf(input.raw_scroll_delta.y as f64 / 100.0);
            self.zoom *= zoom_amount;
            interaction_happened = true;
        }

        // Mouse drag to pan
        if in_viewport && input.pointer.primary_down() {
            match input.pointer.interact_pos() {
                Some(pos) => {
                    match self.last_drag_pos {
                        Some(last_pos) => {
                            let delta = pos - last_pos;
                            self.center_x -= delta.x as f64 / (self.zoom * self.width as f64 / 2.0);
                            self.center_y -= delta.y as f64 / (self.zoom * self.height as f64 / 2.0);
                            interaction_happened = true;
                        }
                        None => {}
                    }
                    self.last_drag_pos = Some(pos);
                }
                None => {}
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

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("💾 Save PNG").clicked() {
                    save_fractal(
                        self.center_x,
                        self.center_y,
                        self.zoom,
                        self.width,
                        self.height,
                        self.color_offset,
                    );
                }
                ui.label("🎨 Color Offset:");
                if ui.add(egui::Slider::new(&mut self.color_offset, 0..=255)).changed() {
                    self.texture_handle = None;
                }

                ui.separator();
                ui.label("🧠 Fractal Type:");
                egui::ComboBox::from_id_source("fractal_type")
                    .selected_text(match self.fractal_type {
                        FractalType::Mandelbrot => "Mandelbrot",
                        FractalType::Julia => "Julia",
                        FractalType::BurningShip => "Burning Ship",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(matches!(self.fractal_type, FractalType::Mandelbrot), "Mandelbrot").clicked() {
                            self.fractal_type = FractalType::Mandelbrot;
                            self.texture_handle = None;
                        }
                        if ui.selectable_label(matches!(self.fractal_type, FractalType::Julia), "Julia").clicked() {
                            self.fractal_type = FractalType::Julia;
                            self.texture_handle = None;
                        }
                        if ui.selectable_label(matches!(self.fractal_type, FractalType::BurningShip), "Burning Ship").clicked() {
                            self.fractal_type = FractalType::BurningShip;
                            self.texture_handle = None;
                        }
                    });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            
// Use the same CPU rendering path for all fractal types
            let available = ui.available_size();
            self.width = available.x.max(1.0) as usize;
            self.height = available.y.max(1.0) as usize;

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
                self.texture_handle = Some(render_fractal_to_texture_by_type(self.fractal_type, 
                    ctx,
                    self.center_x,
                    self.center_y,
                    self.zoom,
                    render_width,
                    render_height,
                    self.color_offset,
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
    fractal_type: FractalType,
    ctx: &egui::Context,
    center_x: f64,
    center_y: f64,
    zoom: f64,
    width: usize,
    height: usize,
    color_offset: u8,
) -> egui::TextureHandle {
    let max_iterations = 100;
    let mut imgbuf = ImageBuffer::new(width as u32, height as u32);

    imgbuf.enumerate_pixels_mut().par_bridge().for_each(|(x, y, pixel)| {
        let cx = (x as f64 - width as f64 / 2.0) / (0.5 * zoom * width as f64) + center_x;
        let cy = (y as f64 - height as f64 / 2.0) / (0.5 * zoom * height as f64) + center_y;
        let (c, mut z) = match fractal_type {
        FractalType::Mandelbrot => (Complex::new(cx, cy), Complex::new(0.0, 0.0)),
        FractalType::Julia => (Complex::new(cx, cy), Complex::new(-0.8, 0.156)),
        FractalType::BurningShip => (Complex::new(cx, cy), Complex::new(0.0, 0.0)),
    };
        let mut i = 0;
        while i < max_iterations && z.norm_sqr() <= 4.0 {
            match fractal_type {
                FractalType::BurningShip => {
                    z = Complex::new((z.re as f64).abs(), (z.im as f64).abs());
                    z = z * z + c;
                }
                _ => {
                    z = z * z + c;
                }
            }
            i += 1;
        }

        let t = i as f32 / max_iterations as f32;
        let r = ((9.0 * (1.0 - t) * t * t * t) * 255.0) as u8;
        let g = ((15.0 * (1.0 - t) * (1.0 - t) * t * t) * 255.0) as u8;
        let b = ((8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t) * 255.0) as u8;

        *pixel = Rgb([
            r.wrapping_add(color_offset),
            g.wrapping_add(color_offset),
            b.wrapping_add(color_offset),
        ]);
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
    width: usize,
    height: usize,
    color_offset: u8,
) {
    let max_iterations = 100;
    let mut imgbuf = ImageBuffer::new(width as u32, height as u32);

    imgbuf.enumerate_pixels_mut().par_bridge().for_each(|(x, y, pixel)| {
        let cx = (x as f64 - width as f64 / 2.0) / (0.5 * zoom * width as f64) + center_x;
        let cy = (y as f64 - height as f64 / 2.0) / (0.5 * zoom * height as f64) + center_y;
        let c = Complex::new(cx, cy);
        let mut z = Complex::new(0.0, 0.0);
        let mut i = 0;
        while i < max_iterations && z.norm_sqr() <= 4.0 {
            z = z * z + c;
            i += 1;
        }

        let t = i as f32 / max_iterations as f32;
        let r = ((9.0 * (1.0 - t) * t * t * t) * 255.0) as u8;
        let g = ((15.0 * (1.0 - t) * (1.0 - t) * t * t) * 255.0) as u8;
        let b = ((8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t) * 255.0) as u8;

        *pixel = Rgb([
            r.wrapping_add(color_offset),
            g.wrapping_add(color_offset),
            b.wrapping_add(color_offset),
        ]);
    });

    let filename = format!(
        "mandelbrot_{:.2}_{:.2}_z{:.2}_c{}.png",
        center_x, center_y, zoom, color_offset
    );
    imgbuf.save(&filename).expect("Failed to save image");
    println!("Saved image to: {}", filename);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Fractal Explorer",
        options,
        Box::new(|_cc| Box::new(FractalApp::default())),
    )
}
