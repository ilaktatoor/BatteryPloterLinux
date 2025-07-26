use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, PlotBounds, Polygon};
use eframe::egui::Color32;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fs;
use std::io::{BufRead, BufReader};
use chrono::{Local, Timelike}; // For getting system time
use battery::Manager;

struct BatteryApp {
    data: Arc<Mutex<Vec<(f64, f64)>>>, // (Actual time in hours, Battery percentage)
    period_secs: Arc<Mutex<u64>>, // Periodo de recolección en segundos
    battery_info: Arc<Mutex<Option<BatteryInfo>>>,
}

#[derive(Default, Clone)]
struct BatteryInfo {
    model: String,
    state: String,
    cycle_count: String,
    energy_full: String,
    energy_full_design: String,
    energy: String,
    voltage: String,
}

impl Default for BatteryApp {
    fn default() -> Self {
        let data = Arc::new(Mutex::new(Vec::new()));
        let period_secs = Arc::new(Mutex::new(60));
        let battery_info = Arc::new(Mutex::new(None));

        // Hilo para leer el archivo CSV periódicamente
        let data_clone = Arc::clone(&data);
        let period_clone = Arc::clone(&period_secs);
        let info_clone = Arc::clone(&battery_info);
        thread::spawn(move || {
            let data_file = "/tmp/battery_data.csv";
            let mut last_len = 0;
            loop {
                if let Ok(file) = fs::File::open(data_file) {
                    let reader = BufReader::new(file);
                    let mut new_data = Vec::new();
                    let mut last_info = BatteryInfo::default();
                    for (i, line) in reader.lines().enumerate() {
                        if i == 0 { continue; } // skip header
                        if let Ok(l) = line {
                            let parts: Vec<&str> = l.split(',').collect();
                            if parts.len() >= 11 {
                                let hour: f64 = parts[6].parse().unwrap_or(0.0);
                                let minute: f64 = parts[7].parse().unwrap_or(0.0);
                                let percentage: f64 = parts[8].parse().unwrap_or(0.0);
                                new_data.push((hour + minute/60.0, percentage));
                                last_info = BatteryInfo {
                                    model: parts[9].to_string(),
                                    state: parts[10].to_string(),
                                    cycle_count: parts.get(11).unwrap_or(&"").to_string(),
                                    energy_full: parts.get(12).unwrap_or(&"").to_string(),
                                    energy_full_design: parts.get(13).unwrap_or(&"").to_string(),
                                    energy: parts.get(14).unwrap_or(&"").to_string(),
                                    voltage: parts.get(15).unwrap_or(&"").to_string(),
                                };
                            }
                        }
                    }
                    let mut data = data_clone.lock().unwrap();
                    *data = new_data;
                    let mut info = info_clone.lock().unwrap();
                    *info = Some(last_info);
                }
                let period = *period_clone.lock().unwrap();
                thread::sleep(Duration::from_secs(period));
            }
        });

        Self { data, period_secs, battery_info }
    }
}

impl eframe::App for BatteryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().frame(egui::Frame::none().fill(Color32::from_rgb(24, 24, 32))).show(ctx, |ui| {
            ui.heading(egui::RichText::new("Battery Life Tracker").color(Color32::LIGHT_GRAY));

            // Configuración del periodo de recolección
            let mut period = *self.period_secs.lock().unwrap();
            ui.horizontal(|ui| {
                let label = if period < 60 {
                    format!("Periodo de recolección: {} min", period / 60)
                } else {
                    let hrs = period as f64 / 3600.0;
                    if hrs < 1.0 {
                        format!("Periodo de recolección: {} min", period / 60)
                    } else {
                        format!("Periodo de recolección: {:.1} hrs", hrs)
                    }
                };
                ui.label(egui::RichText::new(label).color(Color32::GRAY));
                if ui.add(egui::Slider::new(&mut period, 60..=3600).step_by(60.0)).changed() {
                    *self.period_secs.lock().unwrap() = period;
                }
            });

            // Mostrar información de la batería
            if let Some(info) = self.battery_info.lock().unwrap().clone() {
                ui.separator();
                ui.label(egui::RichText::new(format!("Marca/Modelo: {}", info.model)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("Estado: {}", info.state)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("Ciclos de carga: {}", info.cycle_count)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("Energía máxima: {} Wh", info.energy_full)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("Energía de diseño: {} Wh", info.energy_full_design)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("Energía actual: {} Wh", info.energy)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("Voltaje: {} V", info.voltage)).color(Color32::GRAY));
                ui.separator();
            }

            let data = self.data.lock().unwrap();
            let points_vec: Vec<[f64; 2]> = data.iter().map(|(x, y)| [*x, *y]).collect();
            let points: PlotPoints = points_vec.clone().into();
            // Para el área bajo la curva, agregamos los puntos de la base
            let mut area_points: Vec<[f64; 2]> = points_vec.clone();
            if let Some(first) = area_points.first() {
                area_points.insert(0, [first[0], 0.0]);
            }
            if let Some(last) = area_points.last() {
                area_points.push([last[0], 0.0]);
            }
            // Azul translúcido para el área
            let area = Polygon::new(area_points).fill_color(Color32::from_rgba_premultiplied(120, 150, 255, 120));
            // Línea azul claro
            let line = Line::new(points).color(Color32::from_rgb(120, 150, 255)).width(3.0);
            Plot::new("battery_plot")
                .data_aspect(1.0)
                .include_x(0.0)
                .include_y(100.0)
                .label_formatter(|name, value| {
                    format!("{}: ({:.2} hr, {:.2}%)", name, value.x, value.y)
                })
                .show(ui, |plot_ui| {
                    plot_ui.polygon(area);
                    plot_ui.line(line);
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [24.0, 100.0]));
                    // Etiquetas de porcentaje a la derecha
                    for y in (0..=100).step_by(25) {
                        let label = format!("{}%", y);
                        plot_ui.text(egui_plot::Text::new([24.5, y as f64].into(), egui::RichText::new(label).color(Color32::GRAY)));
                    }
                    // Etiquetas de hora en el eje X
                    let n = points_vec.len();
                    if n > 0 {
                        let first_h = points_vec.first().unwrap()[0];
                        let last_h = points_vec.last().unwrap()[0];
                        plot_ui.text(egui_plot::Text::new([first_h, -8.0].into(), egui::RichText::new(format_time(first_h)).color(Color32::GRAY)));
                        plot_ui.text(egui_plot::Text::new([last_h, -8.0].into(), egui::RichText::new("Ahora").color(Color32::GRAY)));
                        // Si hay más de 2 puntos, poner una etiqueta intermedia
                        if n > 2 {
                            let mid = points_vec[n/2][0];
                            plot_ui.text(egui_plot::Text::new([mid, -8.0].into(), egui::RichText::new(format_time(mid)).color(Color32::GRAY)));
                        }
                    }
                });
        });

        ctx.request_repaint_after(Duration::from_secs(5)); // Repaint UI every 5 seconds
    }
}

// Formatea la hora decimal a "HH:MM"
fn format_time(hour: f64) -> String {
    let h = hour.floor() as u32;
    let m = ((hour - h as f64) * 60.0).round() as u32;
    format!("{:02}:{:02}", h, m)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Battery Life Tracker", options, Box::new(|_cc| Box::new(BatteryApp::default())))
}

