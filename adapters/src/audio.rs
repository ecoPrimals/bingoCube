//! Audio adapter for BingoCube sonification.
//!
//! This adapter helps audio systems create soundscapes from BingoCube data.
//! It's OPTIONAL - BingoCube core doesn't need this.

use bingocube_core::{BingoCube, Color};
use std::collections::HashMap;

/// Audio attributes for a BingoCube cell
#[derive(Debug, Clone, PartialEq)]
pub struct CellAudio {
    /// Instrument for this cell
    pub instrument: Instrument,
    /// Pitch (MIDI note number, 0-127)
    pub pitch: u8,
    /// Volume (0.0 to 1.0)
    pub volume: f32,
    /// Pan (-1.0 left to 1.0 right)
    pub pan: f32,
    /// Duration (in milliseconds)
    pub duration_ms: u32,
}

/// Available instruments for sonification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instrument {
    /// Piano (general purpose)
    Piano,
    /// Strings (for continuous patterns)
    Strings,
    /// Bells (for highlights)
    Bells,
    /// Bass (for foundation)
    Bass,
    /// Percussion (for rhythm)
    Percussion,
}

/// Audio renderer for BingoCube
#[derive(Debug)]
pub struct BingoCubeAudioRenderer {
    /// The BingoCube to sonify
    bingocube: BingoCube,
    /// Master volume (0.0 to 1.0)
    master_volume: f32,
    /// Whether audio is enabled
    enabled: bool,
    /// Base MIDI note for the grid
    base_note: u8,
}

impl BingoCubeAudioRenderer {
    /// Creates a new audio renderer
    #[must_use]
    pub fn new(bingocube: BingoCube) -> Self {
        Self {
            bingocube,
            master_volume: 0.7,
            enabled: true,
            base_note: 60, // Middle C
        }
    }

    /// Sets the master volume
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Gets the master volume
    #[must_use]
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Sets whether audio is enabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Gets whether audio is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Maps a color (0-15) to an instrument
    #[must_use]
    pub fn color_to_instrument(color: Color) -> Instrument {
        match color % 5 {
            0 => Instrument::Bells,
            1 => Instrument::Strings,
            2 => Instrument::Piano,
            3 => Instrument::Percussion,
            4 => Instrument::Bass,
            _ => Instrument::Piano,
        }
    }

    /// Maps a color (0-15) to a pitch offset (0-11 semitones)
    #[must_use]
    pub fn color_to_pitch_offset(color: Color) -> u8 {
        color % 12 // Map to chromatic scale
    }

    /// Generates audio attributes for a cell at (row, col)
    #[must_use]
    pub fn generate_cell_audio(&self, row: usize, col: usize, color: Color) -> CellAudio {
        let grid_size = self.bingocube.config.grid_size;

        // Instrument based on color
        let instrument = Self::color_to_instrument(color);

        // Pitch: base note + octave (row) + color offset
        let octave_offset = (row as i16 - grid_size as i16 / 2) * 12 / grid_size as i16;
        let color_offset = Self::color_to_pitch_offset(color);
        let pitch = (i16::from(self.base_note) + octave_offset + i16::from(color_offset))
            .clamp(0, 127) as u8;

        // Volume: louder in center, quieter at edges
        let center_distance = ((row as f32 - grid_size as f32 / 2.0).powi(2)
            + (col as f32 - grid_size as f32 / 2.0).powi(2))
        .sqrt();
        let max_distance = (grid_size as f32 / 2.0) * 1.414; // Diagonal
        let volume = (1.0 - center_distance / max_distance) * self.master_volume;

        // Pan: left to right based on column
        let pan = (col as f32 / (grid_size as f32 - 1.0)) * 2.0 - 1.0;

        // Duration: longer for center cells
        let duration_ms = (100.0 + (1.0 - center_distance / max_distance) * 400.0) as u32;

        CellAudio {
            instrument,
            pitch,
            volume: volume.clamp(0.0, 1.0),
            pan: pan.clamp(-1.0, 1.0),
            duration_ms,
        }
    }

    /// Generates a soundscape description for the current reveal level
    #[must_use]
    pub fn generate_soundscape(&self, x: f64) -> HashMap<(usize, usize), CellAudio> {
        if !self.enabled {
            return HashMap::new();
        }

        let subcube = match self.bingocube.subcube(x) {
            Ok(sc) => sc,
            Err(_) => return HashMap::new(),
        };

        let mut soundscape = HashMap::new();
        for ((row, col), color) in &subcube.revealed {
            let audio = self.generate_cell_audio(*row, *col, *color);
            soundscape.insert((*row, *col), audio);
        }

        soundscape
    }

    /// Gets a textual description of the soundscape
    #[must_use]
    pub fn describe_soundscape(&self, x: f64) -> String {
        let soundscape = self.generate_soundscape(x);

        if soundscape.is_empty() {
            return "Silence. No cells revealed.".to_string();
        }

        let mut instrument_counts = HashMap::new();
        for audio in soundscape.values() {
            *instrument_counts.entry(audio.instrument).or_insert(0) += 1;
        }

        let total_cells = soundscape.len();
        let mut desc = format!("Soundscape with {} cells:\n", total_cells);

        for (instrument, count) in &instrument_counts {
            desc.push_str(&format!("  • {:?}: {} cells\n", instrument, count));
        }

        desc.push_str(&format!(
            "\nReveal: {:.0}% ({}/{})",
            x * 100.0,
            total_cells,
            self.bingocube.config.grid_size * self.bingocube.config.grid_size
        ));

        desc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingocube_core::Config;

    #[test]
    fn test_audio_renderer_creation() {
        let config = Config::default();
        let bingocube =
            BingoCube::from_seed(b"test_seed", config).expect("Failed to create BingoCube");
        let renderer = BingoCubeAudioRenderer::new(bingocube);

        assert_eq!(renderer.master_volume(), 0.7);
        assert!(renderer.is_enabled());
    }

    #[test]
    fn test_color_to_instrument() {
        assert_eq!(
            BingoCubeAudioRenderer::color_to_instrument(0),
            Instrument::Bells
        );
        assert_eq!(
            BingoCubeAudioRenderer::color_to_instrument(1),
            Instrument::Strings
        );
    }

    #[test]
    fn test_color_to_pitch_offset() {
        assert_eq!(BingoCubeAudioRenderer::color_to_pitch_offset(0), 0);
        assert_eq!(BingoCubeAudioRenderer::color_to_pitch_offset(4), 4);
        assert_eq!(BingoCubeAudioRenderer::color_to_pitch_offset(9), 9);
    }

    #[test]
    fn test_generate_cell_audio() {
        let config = Config::default();
        let bingocube =
            BingoCube::from_seed(b"test_seed", config).expect("Failed to create BingoCube");
        let renderer = BingoCubeAudioRenderer::new(bingocube);

        let audio = renderer.generate_cell_audio(2, 2, 0);

        assert!(audio.pitch <= 127);
        assert!(audio.volume >= 0.0 && audio.volume <= 1.0);
        assert!(audio.pan >= -1.0 && audio.pan <= 1.0);
        assert!(audio.duration_ms > 0);
    }

    #[test]
    fn test_generate_soundscape() {
        let config = Config::default();
        let bingocube =
            BingoCube::from_seed(b"test_seed", config).expect("Failed to create BingoCube");
        let renderer = BingoCubeAudioRenderer::new(bingocube);

        let soundscape = renderer.generate_soundscape(0.5);
        assert!(!soundscape.is_empty());

        let soundscape_empty = renderer.generate_soundscape(0.0);
        assert!(soundscape_empty.is_empty());
    }

    #[test]
    fn test_describe_soundscape() {
        let config = Config::default();
        let bingocube =
            BingoCube::from_seed(b"test_seed", config).expect("Failed to create BingoCube");
        let renderer = BingoCubeAudioRenderer::new(bingocube);

        let desc = renderer.describe_soundscape(0.5);
        assert!(desc.contains("Soundscape"));
        assert!(desc.contains("cells"));

        let desc_empty = renderer.describe_soundscape(0.0);
        assert!(desc_empty.contains("Silence"));
    }
}
