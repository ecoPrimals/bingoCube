// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interactive `BingoCube` demo (egui).

pub mod interactive;

use eframe::egui;

/// Launch the interactive `BingoCube` visualization demo.
///
/// # Errors
///
/// Returns an error when the native window cannot be created or the event loop fails.
pub fn run_demo() -> eframe::Result<()> {
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
