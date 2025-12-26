//! BingoCube demo application entry point

use eframe::egui;
use tracing_subscriber;

mod interactive;

fn main() -> eframe::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    tracing::info!("Starting BingoCube demo application");
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("BingoCube Visualization - Standalone Demo"),
        ..Default::default()
    };
    
    eframe::run_native(
        "BingoCube Demo",
        options,
        Box::new(|cc| Ok(Box::new(interactive::BingoCubeDemo::new(cc)))),
    )
}
