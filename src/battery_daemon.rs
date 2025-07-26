use chrono::{Local, Timelike};
use battery::Manager;
use std::fs::{OpenOptions};
use std::io::Write;
use std::thread;
use std::time::Duration;

fn main() {
    let interval_secs = 60; // Cambia este valor para el periodo de recolección
    let data_file = "/tmp/battery_data.csv";
    
    // Escribir encabezado si el archivo no existe
    if !std::path::Path::new(data_file).exists() {
        let mut file = OpenOptions::new().create(true).append(true).open(data_file).unwrap();
        writeln!(file, "timestamp,hour,minute,percentage,model,state,cycle_count,energy_full,energy_full_design,energy,voltage").unwrap();
    }

    let manager = Manager::new().unwrap();
    loop {
        if let Ok(mut batteries) = manager.batteries() {
            if let Some(Ok(battery)) = batteries.next() {
                let now = Local::now();
                let percentage = f64::from(battery.state_of_charge().value) * 100.0;
                let model = battery.model().unwrap_or("").replace(',', " ");
                let state = format!("{:?}", battery.state());
                let cycle_count = battery.cycle_count().map(|c| c.to_string()).unwrap_or("".to_string());
                let energy_full = battery.energy_full().value;
                let energy_full_design = battery.energy_full_design().value;
                let energy = battery.energy().value;
                let voltage = battery.voltage().value;
                let mut file = OpenOptions::new().create(true).append(true).open(data_file).unwrap();
                writeln!(file, "{}-{}-{} {}:{}:{},{},{},{:.2},{},{:.2},{:.2},{:.2},{:.2}",
                    now.year(), now.month(), now.day(), now.hour(), now.minute(), now.second(),
                    now.hour(), now.minute(), percentage, model, state, cycle_count, energy_full, energy_full_design, energy, voltage
                ).unwrap();
            }
        }
        thread::sleep(Duration::from_secs(interval_secs));
    }
}
