//! Example showing how to use wl_monitor_detector library to get monitor information

use wl_monitor_detector::{MonitorDetector, MonitorEvent};

fn main() {
    let (detector, receiver) =
        MonitorDetector::new().expect("Failed to create monitor detector");

    std::thread::spawn(move || {
        if let Err(e) = detector.run() {
            eprintln!("Detector error: {}", e);
        }
    });

    while let Ok(event) = receiver.recv() {
        match event {
            MonitorEvent::InitialState(monitor) => {
                println!("\n=== Monitor: {} ===", monitor.name);
                println!("ID: {}", monitor.id);
                println!("Enabled: {}", monitor.enabled);
                println!(
                    "Position: ({}, {})",
                    monitor.position.x, monitor.position.y
                );
                println!(
                    "Active: {}x{} @ {}Hz",
                    monitor.resolution.width,
                    monitor.resolution.height,
                    monitor.refresh_rate
                );
                println!("Available modes:");
                for mode in &monitor.modes {
                    println!(
                        "  - {}x{} @ {}Hz",
                        mode.resolution.width,
                        mode.resolution.height,
                        mode.refresh_rate
                    );
                }
            }
        }
    }
}
