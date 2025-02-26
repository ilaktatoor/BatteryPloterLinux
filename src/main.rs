use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, PlotBounds};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use chrono::{Local, Timelike}; // For getting system time
use battery::Manager;

struct BatteryApp {
    data: Arc<Mutex<Vec<(f64, f64)>>>, // (Actual time in hours, Battery percentage)
}

impl Default for BatteryApp {
    fn default() -> Self {
        let data = Arc::new(Mutex::new(Vec::new()));

        // Spawn a separate thread to update battery percentage every 60s
        let data_clone = Arc::clone(&data);
        thread::spawn(move || {
            let manager = Manager::new().ok();
            loop {
                if let Some(mgr) = &manager {
                    if let Ok(mut batteries) = mgr.batteries() {
                        if let Some(Ok(battery)) = batteries.next() {
                            let now = Local::now();
                            let current_time = now.hour() as f64 + now.minute() as f64 / 60.0; // Convert time to float (HH.MM)

                            let percentage = f64::from(battery.state_of_charge().value) * 100.0;

                            let mut data = data_clone.lock().unwrap();
                            data.push((current_time, percentage));
                        }
                    }
                }
                thread::sleep(Duration::from_secs(60)); // Sleep only in the worker thread
            }
        });

        Self { data }
    }
}

impl eframe::App for BatteryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Battery Life Tracker");
            let data = self.data.lock().unwrap();
            let points: PlotPoints = data.iter().map(|(x, y)| [*x, *y]).collect();
            let line = Line::new(points);
            Plot::new("battery_plot")
                .data_aspect(1.0)
                .include_x(0.0)
                .include_y(100.0)
                .label_formatter(|name, value| {
                    format!("{}: ({:.2} hr, {:.2}%)", name, value.x, value.y)
                })
                .show(ui, |plot_ui| {
                    plot_ui.line(line);
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [24.0, 100.0]));
                    plot_ui.text(egui_plot::Text::new([12.0, -5.0].into(), "Hour"));
                    plot_ui.text(egui_plot::Text::new([-2.0, 50.0].into(), "Bat %"));
                });
        });

        ctx.request_repaint_after(Duration::from_secs(5)); // Repaint UI every 5 seconds
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Battery Life Tracker", options, Box::new(|_cc| Box::new(BatteryApp::default())))
}

