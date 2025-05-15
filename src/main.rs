use eframe::{egui, epi};
use image::{ImageBuffer, Rgb};
use num_complex::Complex;

pub struct FractalApp {
    zoom: f64,
    center_x: f64,
    center_y: f64,
    width: usize,
    height: usize,
}

impl Default for FractalApp {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            center_x: -0.5,
            center_y: 0.0,
            width: 800,
            height: 600,
        }
    }
}

impl epi::App for FractalApp {
    fn name(&self) -> &str {
        "Fractal Explorer"
    }

    fn update(&mut self, ctx: &egui::Context, _: &mut epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🌀 Fractal Explorer (ASCII GUI Vibes)");
            ui.label(format!("Center: ({:.4}, {:.4}) Zoom: {:.2}", self.center_x, self.center_y, self.zoom));

            if ui.button("⬅️").clicked() {
                self.center_x -= 0.1 / self.zoom;
            }
            if ui.button("➡️").clicked() {
                self.center_x += 0.1 / self.zoom;
            }
            if ui.button("⬆️").clicked() {
                self.center_y -= 0.1 / self.zoom;
            }
            if ui.button("⬇️").clicked() {
                self.center_y += 0.1 / self.zoom;
            }
            if ui.button("Zoom In 🔍").clicked() {
                self.zoom *= 2.0;
            }
            if ui.button("Zoom Out 🧯").clicked() {
                self.zoom /= 2.0;
            }

            if ui.button("🖼 Save PNG").clicked() {
                save_fractal(self.center_x, self.center_y, self.zoom, self.width, self.height);
            }
        });
    }
}

fn save_fractal(center_x: f64, center_y: f64, zoom: f64, width: usize, height: usize) {
    let max_iterations = 100;
    let mut imgbuf = ImageBuffer::new(width as u32, height as u32);

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let cx = (x as f64 - width as f64 / 2.0) / (0.5 * zoom * width as f64) + center_x;
        let cy = (y as f64 - height as f64 / 2.0) / (0.5 * zoom * height as f64) + center_y;
        let c = Complex::new(cx, cy);
        let mut z = Complex::new(0.0, 0.0);
        let mut i = 0;
        while i < max_iterations && z.norm() <= 2.0 {
            z = z * z + c;
            i += 1;
        }

        let color = if i < max_iterations {
            (255 - (i * 255 / max_iterations)) as u8
        } else {
            0
        };
        *pixel = Rgb([color, color, color]);
    }

    imgbuf.save("mandelbrot.png").expect("Failed to save image");
    println!("Saved mandelbrot.png");
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Fractal Explorer",
        options,
        Box::new(|_cc| Box::new(FractalApp::default())),
    )
}