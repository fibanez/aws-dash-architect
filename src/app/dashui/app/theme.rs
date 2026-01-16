//! Theme management and UI dimension tracking

use super::{DashApp, ThemeChoice};
use crate::app::agent_framework::{set_app_theme, AppTheme};
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

        // Sync theme to agent framework for page builder prompts
        let app_theme = match self.theme {
            ThemeChoice::Latte => AppTheme::Latte,
            ThemeChoice::Frappe => AppTheme::Frappe,
            ThemeChoice::Macchiato => AppTheme::Macchiato,
            ThemeChoice::Mocha => AppTheme::Mocha,
        };
        set_app_theme(app_theme);

        // Make window corners more square and adjust heading size
        let mut style = (*ctx.style()).clone();
        style.visuals.window_corner_radius = egui::CornerRadius::same(2); // Set window corner radius to 2 for a more square look

        // Make window title bars ~23.5% smaller (0.85 * 0.90 = 0.765)
        // Title bar height = max(font_height, interact_size.y) + inner_margin
        // Button size = icon_width (capped at title bar height)
        // Sources:
        //   - Title bar height calculation: https://docs.rs/egui/latest/src/egui/containers/window.rs.html
        //   - Spacing struct: https://docs.rs/egui/latest/egui/style/struct.Spacing.html
        let scale = 0.765;

        // Scale the heading font
        if let Some(heading_font) = style.text_styles.get(&egui::TextStyle::Heading).cloned() {
            let smaller_size = heading_font.size * scale;
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(smaller_size, heading_font.family),
            );
        }

        // Scale the minimum interaction height (affects title bar minimum height)
        style.spacing.interact_size.y *= scale;

        // Scale the icon width (affects close/collapse button size)
        style.spacing.icon_width *= scale;

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
