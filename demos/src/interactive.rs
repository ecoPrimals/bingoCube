//! BingoCube interactive demonstration

use bingocube_core::{BingoCube, Config};
use bingocube_adapters::visual::BingoCubeVisualRenderer;

/// BingoCube demo application
pub struct BingoCubeDemo {
    /// Current BingoCube
    cube: BingoCube,
    
    /// Visual renderer
    renderer: BingoCubeVisualRenderer,
    
    /// Current seed input
    seed_input: String,
    
    /// Configuration
    config: Config,
    
    /// Show board A
    show_board_a: bool,
    
    /// Show board B
    show_board_b: bool,
}

impl BingoCubeDemo {
    /// Create a new demo application
    #[must_use]
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::default();
        let seed_input = String::from("alice_identity");
        let cube = BingoCube::from_seed(seed_input.as_bytes(), config.clone())
            .expect("failed to generate cube");
        
        let mut renderer = BingoCubeVisualRenderer::new();
        renderer.reveal_x = 1.0; // Start with full reveal
        
        Self {
            cube,
            renderer,
            seed_input,
            config,
            show_board_a: false,
            show_board_b: false,
        }
    }
    
    /// Regenerate cube from current seed
    fn regenerate_cube(&mut self) {
        match BingoCube::from_seed(self.seed_input.as_bytes(), self.config.clone()) {
            Ok(cube) => self.cube = cube,
            Err(e) => tracing::error!("Failed to generate cube: {}", e),
        }
    }
}

impl eframe::App for BingoCubeDemo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set dark theme
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
        style.visuals.window_fill = egui::Color32::from_rgb(25, 25, 30);
        style.visuals.panel_fill = egui::Color32::from_rgb(30, 30, 35);
        ctx.set_style(style);
        
        // Top panel
        egui::TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(20, 20, 25))
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.heading(
                        egui::RichText::new("🎲 BingoCube Visualization")
                            .size(20.0)
                            .color(egui::Color32::from_rgb(100, 149, 237)),
                    );
                    ui.label(
                        egui::RichText::new("Multi-Modal Cryptographic Commitment")
                            .size(14.0)
                            .color(egui::Color32::GRAY),
                    );
                });
            });
        
        // Left panel - Controls
        egui::SidePanel::left("controls")
            .default_width(300.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 30, 35))
                    .inner_margin(12.0),
            )
            .show(ctx, |ui| {
                ui.heading(egui::RichText::new("⚙️ Controls").size(18.0));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Seed input
                ui.label(egui::RichText::new("Seed Input").strong());
                let response = ui.text_edit_singleline(&mut self.seed_input);
                if response.changed() {
                    self.regenerate_cube();
                }
                
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("alice_identity").clicked() {
                        self.seed_input = "alice_identity".to_string();
                        self.regenerate_cube();
                    }
                    if ui.button("bob_identity").clicked() {
                        self.seed_input = "bob_identity".to_string();
                        self.regenerate_cube();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("document_v1").clicked() {
                        self.seed_input = "document_v1".to_string();
                        self.regenerate_cube();
                    }
                    if ui.button("peer_12345").clicked() {
                        self.seed_input = "peer_12345".to_string();
                        self.regenerate_cube();
                    }
                });
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                
                // Reveal parameter
                ui.label(egui::RichText::new("Progressive Reveal").strong());
                ui.add(
                    egui::Slider::new(&mut self.renderer.reveal_x, 0.0..=1.0)
                        .text("x parameter")
                        .show_value(true),
                );
                
                let revealed_count = (self.renderer.reveal_x * 25.0).ceil() as usize;
                ui.label(format!("Revealed: {}/25 cells ({:.0}%)", 
                    revealed_count, 
                    self.renderer.reveal_x * 100.0
                ));
                
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset (0%)").clicked() {
                        self.renderer.reset();
                    }
                    if ui.button("Animate").clicked() {
                        self.renderer.animate_to(1.0);
                    }
                });
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                
                // Visual options
                ui.label(egui::RichText::new("Visual Options").strong());
                ui.checkbox(&mut self.renderer.show_grid_lines, "Show Grid Lines");
                ui.checkbox(&mut self.renderer.show_values, "Show Color Values");
                ui.checkbox(&mut self.show_board_a, "Show Board A");
                ui.checkbox(&mut self.show_board_b, "Show Board B");
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                
                // Animation controls
                ui.label(egui::RichText::new("Animation").strong());
                ui.checkbox(&mut self.renderer.animate_reveal, "Auto-Animate");
                ui.add(
                    egui::Slider::new(&mut self.renderer.animation_speed, 0.05..=1.0)
                        .text("Speed"),
                );
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                
                // Info
                ui.label(egui::RichText::new("ℹ️ Info").strong());
                ui.label(format!("Grid Size: {}×{}", self.config.grid_size, self.config.grid_size));
                ui.label(format!("Universe: 0-{}", self.config.universe_size - 1));
                ui.label(format!("Palette: {} colors", self.config.palette_size));
                ui.label(format!("Entropy: ~385 bits"));
                ui.label(format!("Forgery (x=0.5): ~2^(-50)"));
            });
        
        // Central panel - BingoCube visualization
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(25, 25, 30)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    
                    // Main BingoCube
                    ui.heading(egui::RichText::new("Color Grid (Cross-Matrix)").size(16.0));
                    ui.add_space(10.0);
                    self.renderer.render(ui, &self.cube);
                    
                    ui.add_space(20.0);
                    
                    // Board A and B (if enabled)
                    if self.show_board_a || self.show_board_b {
                        ui.horizontal(|ui| {
                            if self.show_board_a {
                                ui.vertical(|ui| {
                                    ui.heading(egui::RichText::new("Board A").size(14.0));
                                    ui.add_space(5.0);
                                    render_board(ui, &self.cube.board_a);
                                });
                            }
                            
                            if self.show_board_b {
                                ui.vertical(|ui| {
                                    ui.heading(egui::RichText::new("Board B").size(14.0));
                                    ui.add_space(5.0);
                                    render_board(ui, &self.cube.board_b);
                                });
                            }
                        });
                    }
                });
                
                // Request repaint for animation
                if self.renderer.animate_reveal {
                    ctx.request_repaint();
                }
            });
    }
}

/// Render a board as a grid of numbers
fn render_board(ui: &mut egui::Ui, board: &bingocube_core::Board) {
    use egui::*;
    
    let cell_size = 50.0;
    let size = board.size;
    let grid_size = Vec2::splat(cell_size * size as f32);
    
    let (response, painter) = ui.allocate_painter(grid_size, Sense::hover());
    let rect = response.rect;
    
    for i in 0..size {
        for j in 0..size {
            let cell_rect = Rect::from_min_size(
                rect.min + Vec2::new(j as f32 * cell_size, i as f32 * cell_size),
                Vec2::splat(cell_size),
            );
            
            // Background
            painter.rect_filled(
                cell_rect.shrink(2.0),
                4.0,
                Color32::from_rgb(40, 40, 45),
            );
            
            // Border
            painter.rect_stroke(
                cell_rect.shrink(2.0),
                4.0,
                (1.0, Color32::from_rgb(60, 60, 65)),
            );
            
            // Value
            if let Some(val) = board.get(i, j) {
                let text = format!("{val}");
                painter.text(
                    cell_rect.center(),
                    Align2::CENTER_CENTER,
                    text,
                    FontId::monospace(14.0),
                    Color32::from_rgb(200, 200, 200),
                );
            } else {
                // Free cell
                painter.text(
                    cell_rect.center(),
                    Align2::CENTER_CENTER,
                    "✱",
                    FontId::proportional(20.0),
                    Color32::from_rgb(150, 150, 150),
                );
            }
        }
    }
}

