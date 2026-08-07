// SPDX-License-Identifier: AGPL-3.0-or-later
//! Visual adapter for BingoCube rendering.
//!
//! This adapter helps visualization systems (like petalTongue) render
//! BingoCube data using egui. It's OPTIONAL - BingoCube core doesn't need this.

use bingocube_core::{BingoCube, Color};
use egui::{Color32, Rect, Response, Sense, Ui, Vec2};

/// Default 16-color perceptually distinct palette.
#[must_use]
pub fn default_palette() -> [Color32; 16] {
    [
        Color32::from_rgb(100, 149, 237),
        Color32::from_rgb(60, 179, 113),
        Color32::from_rgb(255, 215, 0),
        Color32::from_rgb(220, 20, 60),
        Color32::from_rgb(138, 43, 226),
        Color32::from_rgb(255, 140, 0),
        Color32::from_rgb(46, 139, 87),
        Color32::from_rgb(199, 21, 133),
        Color32::from_rgb(0, 191, 255),
        Color32::from_rgb(154, 205, 50),
        Color32::from_rgb(255, 105, 180),
        Color32::from_rgb(64, 224, 208),
        Color32::from_rgb(255, 69, 0),
        Color32::from_rgb(186, 85, 211),
        Color32::from_rgb(50, 205, 50),
        Color32::from_rgb(255, 20, 147),
    ]
}

/// Visual rendering configuration.
#[derive(Debug, Clone)]
pub struct VisualConfig {
    /// Pixel size of each grid cell.
    pub cell_size: f32,
    /// Color palette indexed by `color % palette.len()`.
    pub palette: [Color32; 16],
    /// Fill color for unrevealed cells.
    pub unrevealed_cell_color: Color32,
    /// Grid line stroke color.
    pub grid_line_color: Color32,
    /// Grid line stroke width.
    pub grid_line_width: f32,
    /// Corner radius for cell fills and strokes.
    pub cell_corner_radius: f32,
    /// Inset applied before drawing each cell.
    pub cell_shrink: f32,
    /// Font size when `show_values` is enabled.
    pub value_font_size: f32,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            cell_size: 60.0,
            palette: default_palette(),
            unrevealed_cell_color: Color32::from_rgb(30, 30, 35),
            grid_line_color: Color32::from_rgb(60, 60, 65),
            grid_line_width: 1.0,
            cell_corner_radius: 4.0,
            cell_shrink: 2.0,
            value_font_size: 10.0,
        }
    }
}

impl VisualConfig {
    /// Set cell size in pixels (builder pattern).
    #[must_use]
    pub fn with_cell_size(mut self, size: f32) -> Self {
        self.cell_size = size.max(1.0);
        self
    }

    /// Replace the color palette (builder pattern).
    #[must_use]
    pub fn with_palette(mut self, palette: [Color32; 16]) -> Self {
        self.palette = palette;
        self
    }
}

/// Visual renderer for BingoCube
pub struct BingoCubeVisualRenderer {
    config: VisualConfig,

    /// Current reveal parameter (0.0-1.0)
    pub reveal_x: f64,

    /// Whether to animate reveal
    pub animate_reveal: bool,

    /// Target reveal for animation (None means animate to 1.0)
    target_reveal: Option<f64>,

    /// Animation speed (x per second)
    pub animation_speed: f64,

    /// Show grid lines
    pub show_grid_lines: bool,

    /// Show cell values (for debugging)
    pub show_values: bool,
}

impl Default for BingoCubeVisualRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl BingoCubeVisualRenderer {
    /// Create a new `BingoCubeVisualRenderer`
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(VisualConfig::default())
    }

    /// Create a renderer with custom visual configuration (builder pattern).
    #[must_use]
    pub fn with_config(config: VisualConfig) -> Self {
        Self {
            config,
            reveal_x: 1.0,
            animate_reveal: false,
            target_reveal: None,
            animation_speed: 0.2,
            show_grid_lines: true,
            show_values: false,
        }
    }

    /// Returns a reference to the visual configuration.
    #[must_use]
    pub fn config(&self) -> &VisualConfig {
        &self.config
    }

    /// Create a new renderer with a specific reveal level (builder pattern)
    #[must_use]
    pub fn with_reveal(mut self, x: f64) -> Self {
        self.reveal_x = x.clamp(0.0, 1.0);
        self
    }

    /// Create a new renderer with animation enabled (builder pattern)
    #[must_use]
    pub fn with_animation(mut self, speed: f64) -> Self {
        self.animate_reveal = true;
        self.animation_speed = speed;
        self
    }

    /// Create a new renderer with grid lines disabled (builder pattern)
    #[must_use]
    pub fn without_grid_lines(mut self) -> Self {
        self.show_grid_lines = false;
        self
    }

    /// Create a new renderer with values shown (builder pattern)
    #[must_use]
    pub fn with_values(mut self) -> Self {
        self.show_values = true;
        self
    }

    /// Set reveal parameter with validation
    ///
    /// # Returns
    /// Returns `&mut Self` for method chaining
    pub fn set_reveal(&mut self, x: f64) -> &mut Self {
        self.reveal_x = x.clamp(0.0, 1.0);
        self
    }

    /// Get current reveal parameter
    #[must_use]
    pub fn get_reveal(&self) -> f64 {
        self.reveal_x
    }

    /// Set animation speed
    ///
    /// # Returns
    /// Returns `&mut Self` for method chaining
    pub fn set_animation_speed(&mut self, speed: f64) -> &mut Self {
        self.animation_speed = speed.max(0.0);
        self
    }

    /// Start or stop animation
    ///
    /// # Returns
    /// Returns `&mut Self` for method chaining
    pub fn set_animate(&mut self, animate: bool) -> &mut Self {
        self.animate_reveal = animate;
        self
    }

    /// Check if currently animating
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.animate_reveal
    }

    /// Render a `BingoCube` to the UI
    ///
    /// Returns the response for interaction
    pub fn render(&mut self, ui: &mut Ui, cube: &BingoCube) -> Response {
        // Update animation
        if self.animate_reveal {
            let delta = self.animation_speed * ui.input(|i| i.stable_dt as f64);
            let target = self.target_reveal.unwrap_or(1.0);

            if (target - self.reveal_x).abs() < delta {
                // Reached target
                self.reveal_x = target;
                self.animate_reveal = false;
                self.target_reveal = None;
            } else if target > self.reveal_x {
                // Animate forward
                self.reveal_x += delta;
            } else {
                // Animate backward
                self.reveal_x -= delta;
            }

            self.reveal_x = self.reveal_x.clamp(0.0, 1.0);
        }

        let subcube = cube
            .subcube(self.reveal_x)
            .map_or_else(|_| cube.subcube(1.0).ok(), Some);

        let Some(subcube) = subcube else {
            let (response, _painter) = ui.allocate_painter(Vec2::ZERO, Sense::hover());
            return response;
        };

        let size = cube.config.grid_size;
        let cell_size = self.config.cell_size;
        let grid_size = Vec2::splat(cell_size * size as f32);

        let (response, painter) = ui.allocate_painter(grid_size, Sense::hover());
        let rect = response.rect;

        for i in 0..size {
            for j in 0..size {
                let cell_rect = Rect::from_min_size(
                    rect.min + Vec2::new(j as f32 * cell_size, i as f32 * cell_size),
                    Vec2::splat(cell_size),
                );
                let draw_rect = cell_rect.shrink(self.config.cell_shrink);
                let corner = self.config.cell_corner_radius;

                if subcube.is_revealed(i, j) {
                    if let Some(color) = subcube.get_color(i, j) {
                        let cell_color = self.color_index_to_color32(color);
                        painter.rect_filled(draw_rect, corner, cell_color);
                    }
                } else {
                    painter.rect_filled(draw_rect, corner, self.config.unrevealed_cell_color);
                }

                if self.show_grid_lines {
                    painter.rect_stroke(
                        draw_rect,
                        corner,
                        (self.config.grid_line_width, self.config.grid_line_color),
                    );
                }

                if self.show_values {
                    if let Some(color) = cube.get_color(i, j) {
                        let text = format!("{color}");
                        painter.text(
                            cell_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::monospace(self.config.value_font_size),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }

        response
    }

    /// Start reveal animation to a target value
    pub fn animate_to(&mut self, target_x: f64) -> &mut Self {
        self.target_reveal = Some(target_x.clamp(0.0, 1.0));
        self.animate_reveal = true;
        self
    }

    /// Reset to no reveal
    pub fn reset(&mut self) -> &mut Self {
        self.reveal_x = 0.0;
        self.animate_reveal = false;
        self.target_reveal = None;
        self
    }

    fn color_index_to_color32(&self, color: Color) -> Color32 {
        let index = (color as usize) % self.config.palette.len();
        self.config.palette[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingocube_core::Config;
    use egui::{Pos2, Vec2};

    #[test]
    fn test_renderer_creation() {
        let renderer = BingoCubeVisualRenderer::new();
        assert_eq!(renderer.reveal_x, 1.0);
        assert!(!renderer.animate_reveal);
        assert!(renderer.show_grid_lines);
        assert!(!renderer.show_values);
        assert_eq!(renderer.get_reveal(), 1.0);
    }

    #[test]
    fn test_builder_chain_and_setters() {
        let mut r = BingoCubeVisualRenderer::new()
            .with_reveal(1.5)
            .with_animation(0.5)
            .without_grid_lines()
            .with_values();
        assert_eq!(r.reveal_x, 1.0);
        assert!(r.animate_reveal);
        assert_eq!(r.animation_speed, 0.5);
        assert!(!r.show_grid_lines);
        assert!(r.show_values);

        r.set_reveal(-0.5)
            .set_animation_speed(-1.0)
            .set_animate(false);
        assert_eq!(r.reveal_x, 0.0);
        assert_eq!(r.animation_speed, 0.0);
        assert!(!r.is_animating());

        r.animate_to(0.75);
        assert!(r.is_animating());

        r.reset();
        assert_eq!(r.reveal_x, 0.0);
        assert!(!r.is_animating());
        assert!(r.target_reveal.is_none());
    }

    #[test]
    fn test_color_mapping() {
        let renderer = BingoCubeVisualRenderer::new();
        let color1 = renderer.color_index_to_color32(0);
        let color2 = renderer.color_index_to_color32(0);
        assert_eq!(color1, color2);

        let color_a = renderer.color_index_to_color32(0);
        let color_b = renderer.color_index_to_color32(1);
        assert_ne!(color_a, color_b);
    }

    #[test]
    fn test_color_palette_modulo_and_distinctness() {
        let renderer = BingoCubeVisualRenderer::new();
        assert_eq!(
            renderer.color_index_to_color32(3),
            renderer.color_index_to_color32(19)
        );

        let mut seen = std::collections::HashSet::new();
        for i in 0..16_u8 {
            seen.insert(renderer.color_index_to_color32(i));
        }
        assert_eq!(
            seen.len(),
            16,
            "each palette index 0..16 should map to a distinct Color32"
        );
    }

    #[test]
    fn test_visual_config_defaults() {
        let config = VisualConfig::default();
        assert_eq!(config.cell_size, 60.0);
        assert_eq!(config.palette, default_palette());
    }

    #[test]
    fn test_visual_config_builder() {
        let config = VisualConfig::default().with_cell_size(80.0);
        assert_eq!(config.cell_size, 80.0);
    }

    #[test]
    fn test_grid_layout_math_matches_render() {
        let cell_size = 60.0_f32;
        let config = Config::default();
        let size = config.grid_size;
        let grid_size = Vec2::splat(cell_size * size as f32);
        assert_eq!(grid_size.x, 300.0);
        assert_eq!(grid_size.y, 300.0);

        let rect_min = Pos2::ZERO;
        for i in 0..size {
            for j in 0..size {
                let min = rect_min + Vec2::new(j as f32 * cell_size, i as f32 * cell_size);
                let cell = Rect::from_min_size(min, Vec2::splat(cell_size));
                assert_eq!(cell.width(), cell_size);
                assert_eq!(cell.height(), cell_size);
                let shrunk = cell.shrink(2.0);
                assert_eq!(shrunk.width(), cell_size - 4.0);
            }
        }
    }

    #[test]
    fn test_subcube_to_color_mapping_pipeline() {
        let config = Config::default();
        let cube = BingoCube::from_seed(b"visual_cov", config).expect("cube");
        let sub = cube.subcube(0.4).expect("subcube");
        let size = cube.config.grid_size;
        for i in 0..size {
            for j in 0..size {
                if sub.is_revealed(i, j) {
                    let c = sub.get_color(i, j).expect("color when revealed");
                    let mapped = BingoCubeVisualRenderer::new().color_index_to_color32(c);
                    assert_ne!(mapped, Color32::TRANSPARENT);
                }
            }
        }
    }
}
