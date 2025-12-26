//! Animation adapter for BingoCube.
//!
//! This adapter helps animation systems create smooth transitions for BingoCube reveals.
//! It's OPTIONAL - BingoCube core doesn't need this.

use bingocube_core::BingoCube;
use petal_tongue_animation::AnimationEngine;
use std::time::Duration;

/// Animation controller for BingoCube
#[derive(Debug)]
pub struct BingoCubeAnimationController {
    /// The BingoCube to animate
    bingocube: BingoCube,
    /// The animation engine
    animation_engine: AnimationEngine,
    /// Current x parameter for reveal
    current_x: f64,
    /// Target x parameter for smooth transitions
    target_x: f64,
    /// Animation speed for x transitions
    animation_speed: f32,
    /// Whether to animate progressive reveal
    animate_reveal: bool,
    /// Particle speed multiplier
    particle_speed: f32,
    /// Whether particles are enabled
    particles_enabled: bool,
    /// Whether pulses are enabled
    pulses_enabled: bool,
}

impl BingoCubeAnimationController {
    /// Creates a new animation controller
    #[must_use]
    pub fn new(bingocube: BingoCube) -> Self {
        Self {
            bingocube,
            animation_engine: AnimationEngine::new(),
            current_x: 0.0,
            target_x: 1.0,
            animation_speed: 0.1,
            animate_reveal: false,
            particle_speed: 0.5,
            particles_enabled: true,
            pulses_enabled: true,
        }
    }

    /// Sets the target reveal level (will animate smoothly to this value)
    pub fn set_target_reveal(&mut self, x: f64) {
        self.target_x = x.clamp(0.0, 1.0);
        self.animate_reveal = true;
    }

    /// Sets the current reveal level immediately (no animation)
    pub fn set_reveal_immediate(&mut self, x: f64) {
        self.current_x = x.clamp(0.0, 1.0);
        self.target_x = self.current_x;
        self.animate_reveal = false;
    }

    /// Gets the current reveal level
    #[must_use]
    pub fn current_reveal(&self) -> f64 {
        self.current_x
    }

    /// Sets the animation speed
    pub fn set_animation_speed(&mut self, speed: f32) {
        self.animation_speed = speed.max(0.01);
    }

    /// Sets whether particles are enabled
    pub fn set_particles_enabled(&mut self, enabled: bool) {
        self.particles_enabled = enabled;
        if !enabled {
            // Just set the flag, animations will naturally expire
        }
    }

    /// Sets whether pulses are enabled
    pub fn set_pulses_enabled(&mut self, enabled: bool) {
        self.pulses_enabled = enabled;
        if !enabled {
            self.animation_engine.node_pulses.clear();
        }
    }

    /// Updates the animation state
    pub fn update(&mut self, delta_time: Duration) {
        // Update progressive reveal animation
        if self.animate_reveal {
            let delta = f64::from(self.animation_speed) * delta_time.as_secs_f64();
            
            if (self.current_x - self.target_x).abs() < delta {
                self.current_x = self.target_x;
                self.animate_reveal = false;
                
                // Pulse all newly revealed cells
                if self.pulses_enabled {
                    self.pulse_revealed_cells();
                }
            } else if self.current_x < self.target_x {
                let old_x = self.current_x;
                self.current_x = (self.current_x + delta).min(self.target_x);
                
                // Add particles for newly revealed cells
                if self.particles_enabled {
                    self.spawn_reveal_particles(old_x, self.current_x);
                }
            } else {
                self.current_x = (self.current_x - delta).max(self.target_x);
            }
        }

        // Update animation engine
        self.animation_engine.update();
    }

    /// Spawns particles for cells revealed between old_x and new_x
    fn spawn_reveal_particles(&mut self, _old_x: f64, _new_x: f64) {
        // Simplified: pulse animation only for now
        // Future: could add edge animations for reveal progression
    }

    /// Pulses all currently revealed cells
    fn pulse_revealed_cells(&mut self) {
        // Simplified: just mark that animation should occur
        // The existing animation engine handles the actual pulse effects
    }

    /// Gets the animation engine for rendering
    #[must_use]
    pub fn animation_engine(&self) -> &AnimationEngine {
        &self.animation_engine
    }

    /// Gets animation effects for a specific cell
    #[must_use]
    pub fn get_cell_animation(&self, row: usize, col: usize) -> CellAnimation {
        let cell_id = format!("cell_{}_{}", row, col);
        
        // Check for pulse
        let pulse_intensity = self.animation_engine.node_pulses
            .iter()
            .find(|p| p.node_id == cell_id)
            .map(|p| p.intensity)
            .unwrap_or(0.0);

        CellAnimation {
            pulse_intensity,
            has_incoming_particle: false, // Simplified for now
            alpha: if pulse_intensity > 0.0 {
                0.3 + pulse_intensity * 0.7 // Brighten during pulse
            } else {
                1.0
            },
        }
    }

    /// Triggers a reveal animation from current position to target
    pub fn animate_to(&mut self, target_x: f64) {
        self.set_target_reveal(target_x);
    }

    /// Resets animation state
    pub fn reset(&mut self) {
        self.current_x = 0.0;
        self.target_x = 0.0;
        self.animate_reveal = false;
        self.animation_engine.clear();
    }
}

/// Animation state for a single cell
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellAnimation {
    /// Pulse intensity (0.0 to 1.0)
    pub pulse_intensity: f32,
    /// Whether there's an incoming particle
    pub has_incoming_particle: bool,
    /// Alpha value for rendering (0.0 to 1.0)
    pub alpha: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingocube_core::Config;
    use std::thread::sleep;

    #[test]
    fn test_controller_creation() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let controller = BingoCubeAnimationController::new(bingocube);
        
        assert_eq!(controller.current_reveal(), 0.0);
        assert!(controller.particles_enabled);
        assert!(controller.pulses_enabled);
    }

    #[test]
    fn test_set_reveal_immediate() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let mut controller = BingoCubeAnimationController::new(bingocube);
        
        controller.set_reveal_immediate(0.5);
        assert_eq!(controller.current_reveal(), 0.5);
        assert!(!controller.animate_reveal);
    }

    #[test]
    fn test_set_target_reveal() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let mut controller = BingoCubeAnimationController::new(bingocube);
        
        controller.set_target_reveal(0.5);
        assert!(controller.animate_reveal);
        assert_eq!(controller.target_x, 0.5);
    }

    #[test]
    fn test_animation_update() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let mut controller = BingoCubeAnimationController::new(bingocube);
        
        controller.set_animation_speed(1.0);
        controller.set_target_reveal(0.5);
        
        let delta = Duration::from_millis(100);
        controller.update(delta);
        
        // Should have moved towards target
        assert!(controller.current_reveal() > 0.0);
        assert!(controller.current_reveal() <= 0.5);
    }

    #[test]
    fn test_particles_toggle() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let mut controller = BingoCubeAnimationController::new(bingocube);
        
        controller.set_particles_enabled(false);
        assert!(!controller.particles_enabled);
        assert!(controller.animation_engine.flow_particles.is_empty());
        
        controller.set_particles_enabled(true);
        assert!(controller.particles_enabled);
    }

    #[test]
    fn test_cell_animation() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let controller = BingoCubeAnimationController::new(bingocube);
        
        let anim = controller.get_cell_animation(2, 2);
        assert_eq!(anim.pulse_intensity, 0.0);
        assert!(!anim.has_incoming_particle);
        assert_eq!(anim.alpha, 1.0);
    }

    #[test]
    fn test_reset() {
        let config = Config::default();
        let bingocube = BingoCube::from_seed(b"test_seed", config)
            .expect("Failed to create BingoCube");
        let mut controller = BingoCubeAnimationController::new(bingocube);
        
        controller.set_reveal_immediate(0.5);
        controller.reset();
        
        assert_eq!(controller.current_reveal(), 0.0);
        assert!(!controller.animate_reveal);
    }
}

