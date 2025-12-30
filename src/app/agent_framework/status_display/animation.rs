//! Processing Animations
//!
//! Provides animated visual feedback during agent processing using egui's Painter API.
//!
//! ## Animation Styles
//!
//! - **OrbitalDots**: Three dots orbiting in a circle (for thinking phases)
//! - **WaveBars**: Five bars with wave-like height animation (for tool execution)
//!
//! ## Phase Mapping
//!
//! | Phase           | Animation      | Speed   |
//! |-----------------|----------------|---------|
//! | Thinking        | OrbitalDots    | Normal  |
//! | ExecutingTool   | WaveBars       | Fast    |
//! | AnalyzingResults| OrbitalDots    | Slow    |

#![warn(clippy::all, rust_2018_idioms)]

use std::f32::consts::TAU;
use std::time::Instant;

use egui::{Color32, Pos2, Rect, Ui, Vec2};

use super::ProcessingPhase;

/// Animation style for processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationStyle {
    /// Three dots orbiting in a circle
    OrbitalDots,
    /// Five bars with wave-like height animation
    WaveBars,
}

impl AnimationStyle {
    /// Get the appropriate animation style for a processing phase
    pub fn for_phase(phase: &ProcessingPhase) -> Self {
        match phase {
            ProcessingPhase::Thinking => AnimationStyle::OrbitalDots,
            ProcessingPhase::ExecutingTool(_) => AnimationStyle::WaveBars,
            ProcessingPhase::AnalyzingResults => AnimationStyle::OrbitalDots,
            ProcessingPhase::Idle => AnimationStyle::OrbitalDots, // Fallback
        }
    }
}

/// Animated processing indicator
pub struct ProcessingAnimation {
    /// When the animation started
    start_time: Instant,
    /// Current animation style
    style: AnimationStyle,
    /// Animation speed multiplier (1.0 = normal)
    speed: f32,
    /// Base color for the animation
    color: Color32,
}

impl Default for ProcessingAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingAnimation {
    /// Default animation dimensions
    const DOT_RADIUS: f32 = 3.0;
    const ORBIT_RADIUS: f32 = 10.0;
    const BAR_WIDTH: f32 = 4.0;
    const BAR_BASE_HEIGHT: f32 = 8.0;
    const BAR_AMPLITUDE: f32 = 8.0;
    const BAR_SPACING: f32 = 6.0;
    const NUM_DOTS: usize = 3;
    const NUM_BARS: usize = 5;

    /// Create a new animation
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            style: AnimationStyle::OrbitalDots,
            speed: 1.0,
            color: Color32::from_rgb(100, 149, 237), // Cornflower blue
        }
    }

    /// Set animation style
    pub fn with_style(mut self, style: AnimationStyle) -> Self {
        self.style = style;
        self
    }

    /// Set animation speed (1.0 = normal, 2.0 = twice as fast)
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Set animation color
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Update animation for the given processing phase
    pub fn set_phase(&mut self, phase: &ProcessingPhase) {
        let new_style = AnimationStyle::for_phase(phase);

        // Update speed based on phase
        self.speed = match phase {
            ProcessingPhase::Thinking => 1.0,
            ProcessingPhase::ExecutingTool(_) => 1.5,
            ProcessingPhase::AnalyzingResults => 0.7,
            ProcessingPhase::Idle => 0.5,
        };

        // Reset animation if style changed
        if new_style != self.style {
            self.style = new_style;
            self.start_time = Instant::now();
        }
    }

    /// Get the current animation phase (0.0 to TAU)
    fn phase(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        (elapsed * self.speed * TAU / 2.0) % TAU
    }

    /// Calculate pulsing color (subtle brightness variation)
    fn pulsing_color(&self) -> Color32 {
        let phase = self.phase();
        let pulse = (phase.sin() + 1.0) / 2.0; // 0.0 to 1.0
        let brightness = 0.7 + pulse * 0.3; // 0.7 to 1.0

        Color32::from_rgb(
            (self.color.r() as f32 * brightness) as u8,
            (self.color.g() as f32 * brightness) as u8,
            (self.color.b() as f32 * brightness) as u8,
        )
    }

    /// Render orbital dots animation
    fn render_orbital_dots(&self, ui: &mut Ui, center: Pos2) {
        let painter = ui.painter();
        let phase = self.phase();
        let color = self.pulsing_color();

        for i in 0..Self::NUM_DOTS {
            let angle = phase + (i as f32 * TAU / Self::NUM_DOTS as f32);
            let offset = Vec2::angled(angle) * Self::ORBIT_RADIUS;
            let pos = center + offset;

            // Dots fade based on position in orbit (leader is brightest)
            let alpha = 1.0 - (i as f32 * 0.25);
            let dot_color = Color32::from_rgba_unmultiplied(
                color.r(),
                color.g(),
                color.b(),
                (color.a() as f32 * alpha) as u8,
            );

            painter.circle_filled(pos, Self::DOT_RADIUS, dot_color);
        }
    }

    /// Render wave bars animation
    fn render_wave_bars(&self, ui: &mut Ui, center: Pos2) {
        let painter = ui.painter();
        let phase = self.phase();
        let color = self.pulsing_color();

        // Calculate total width to center the bars
        let total_width =
            Self::NUM_BARS as f32 * Self::BAR_WIDTH + (Self::NUM_BARS - 1) as f32 * Self::BAR_SPACING;
        let start_x = center.x - total_width / 2.0;

        for i in 0..Self::NUM_BARS {
            // Phase offset creates wave effect
            let bar_phase = phase + (i as f32 * 0.5);
            let height = Self::BAR_BASE_HEIGHT + bar_phase.sin() * Self::BAR_AMPLITUDE;

            let x = start_x + i as f32 * (Self::BAR_WIDTH + Self::BAR_SPACING);
            let y = center.y - height / 2.0;

            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(Self::BAR_WIDTH, height));

            // Bars have slight color variation based on position
            let hue_shift = (i as f32 / Self::NUM_BARS as f32) * 0.1;
            let bar_color = Color32::from_rgb(
                ((color.r() as f32 * (1.0 - hue_shift)) as u8).max(50),
                color.g(),
                ((color.b() as f32 * (1.0 + hue_shift)).min(255.0)) as u8,
            );

            painter.rect_filled(rect, 2.0, bar_color);
        }
    }

    /// Render the animation at the current position
    ///
    /// Returns the size of the rendered animation for layout purposes
    pub fn show(&self, ui: &mut Ui) -> Vec2 {
        let size = match self.style {
            AnimationStyle::OrbitalDots => {
                Vec2::splat(Self::ORBIT_RADIUS * 2.0 + Self::DOT_RADIUS * 2.0)
            }
            AnimationStyle::WaveBars => {
                let width = Self::NUM_BARS as f32 * Self::BAR_WIDTH
                    + (Self::NUM_BARS - 1) as f32 * Self::BAR_SPACING;
                let height = Self::BAR_BASE_HEIGHT + Self::BAR_AMPLITUDE * 2.0;
                Vec2::new(width, height)
            }
        };

        // Allocate space and get center position
        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
        let center = rect.center();

        // Render based on style
        match self.style {
            AnimationStyle::OrbitalDots => self.render_orbital_dots(ui, center),
            AnimationStyle::WaveBars => self.render_wave_bars(ui, center),
        }

        // Request repaint for animation
        ui.ctx().request_repaint();

        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_style_for_phase() {
        assert_eq!(
            AnimationStyle::for_phase(&ProcessingPhase::Thinking),
            AnimationStyle::OrbitalDots
        );
        assert_eq!(
            AnimationStyle::for_phase(&ProcessingPhase::ExecutingTool("test".into())),
            AnimationStyle::WaveBars
        );
        assert_eq!(
            AnimationStyle::for_phase(&ProcessingPhase::AnalyzingResults),
            AnimationStyle::OrbitalDots
        );
    }

    #[test]
    fn test_animation_creation() {
        let anim = ProcessingAnimation::new();
        assert_eq!(anim.style, AnimationStyle::OrbitalDots);
        assert!((anim.speed - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_animation_builder() {
        let anim = ProcessingAnimation::new()
            .with_style(AnimationStyle::WaveBars)
            .with_speed(2.0)
            .with_color(Color32::RED);

        assert_eq!(anim.style, AnimationStyle::WaveBars);
        assert!((anim.speed - 2.0).abs() < 0.01);
        assert_eq!(anim.color, Color32::RED);
    }

    #[test]
    fn test_phase_updates_speed() {
        let mut anim = ProcessingAnimation::new();

        anim.set_phase(&ProcessingPhase::Thinking);
        assert!((anim.speed - 1.0).abs() < 0.01);

        anim.set_phase(&ProcessingPhase::ExecutingTool("test".into()));
        assert!((anim.speed - 1.5).abs() < 0.01);

        anim.set_phase(&ProcessingPhase::AnalyzingResults);
        assert!((anim.speed - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_phase_calculation() {
        let anim = ProcessingAnimation::new();
        let phase = anim.phase();

        // Phase should be between 0 and TAU
        assert!(phase >= 0.0);
        assert!(phase < TAU);
    }

    #[test]
    fn test_pulsing_color() {
        let anim = ProcessingAnimation::new().with_color(Color32::from_rgb(100, 100, 100));

        let color = anim.pulsing_color();
        // Color should be some variation of the base color
        assert!(color.r() >= 70 && color.r() <= 100);
        assert!(color.g() >= 70 && color.g() <= 100);
        assert!(color.b() >= 70 && color.b() <= 100);
    }
}
