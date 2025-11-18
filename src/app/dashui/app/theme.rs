//! Theme management and UI dimension tracking

use super::{DashApp, ThemeChoice};
use eframe::egui;

impl DashApp {
    /// Apply the selected theme to the UI context
    pub(super) fn apply_theme(&self, ctx: &egui::Context) {
        // Apply the selected theme
        match self.theme {
            ThemeChoice::Latte => catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE),
            ThemeChoice::Frappe => catppuccin_egui::set_theme(ctx, catppuccin_egui::FRAPPE),
            ThemeChoice::Macchiato => catppuccin_egui::set_theme(ctx, catppuccin_egui::MACCHIATO),
            ThemeChoice::Mocha => catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA),
        }

        // Make window corners more square by setting global window style
        let mut style = (*ctx.style()).clone();
        style.visuals.window_corner_radius = egui::CornerRadius::same(2); // Set window corner radius to 2 for a more square look
        ctx.set_style(style);
    }

    /// Check for UI dimension changes like window resize or font scale change
    pub(super) fn check_ui_dimension_changes(&mut self, ctx: &egui::Context) {
        // Check for window size or font scale changes
        let current_screen_size = ctx.screen_rect().size();
        let current_pixels_per_point = ctx.pixels_per_point();

        // Detect window resize
        if self.previous_screen_size != Some(current_screen_size) {
            // Window size changed
            self.command_palette.on_window_resized();
            self.previous_screen_size = Some(current_screen_size);
        }

        // Detect font size change
        if self.previous_pixels_per_point != Some(current_pixels_per_point) {
            // Font scale changed
            self.command_palette.on_font_size_changed();
            self.previous_pixels_per_point = Some(current_pixels_per_point);
        }
    }
}
