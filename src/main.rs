#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod process;
mod console;
mod worker;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_min_inner_size([600.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Remote Console Attach Tool",
        options,
        Box::new(|cc| Ok(Box::new(ui::RemoteConApp::new(cc)))),
    )
}
