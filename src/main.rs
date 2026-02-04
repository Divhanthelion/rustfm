mod app;
mod explorer;
mod search;
mod terminal;

use eframe::NativeOptions;

fn main() -> eframe::Result {
    // Initialize tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _enter = rt.enter();

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "File Explorer with Terminal",
        options,
        Box::new(|cc| Ok(Box::new(app::FileExplorerApp::new(cc)))),
    )
}
