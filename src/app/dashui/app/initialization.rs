//! App initialization and font configuration

use super::super::DashApp;
use crate::app::agent_framework::skills::initialize_skill_system;
use crate::app::fonts;
use eframe::egui;
use tracing::{info, warn};

impl DashApp {
    /// Create a new DashApp instance from creation context
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        // Apply the saved theme
        app.apply_theme(&cc.egui_ctx);

        // Start repository synchronization in background
        app.start_repository_sync();

        // Initialize skill system (independent of AWS login)
        app.initialize_skills();

        app
    }

    /// Initialize the agent skill system at application startup
    pub(super) fn initialize_skills(&mut self) {
        match initialize_skill_system() {
            Ok(count) => {
                info!("✅ Skill system initialized: {} skills discovered", count);
            }
            Err(e) => {
                warn!("⚠️ Failed to initialize skill system: {}", e);
            }
        }
    }

    /// Repository sync feature removed
    pub(super) fn start_repository_sync(&mut self) {
        // Guard repository system removed
    }

    /// Configure enhanced fonts with emoji support
    pub(super) fn configure_fonts(&mut self, ctx: &egui::Context) {
        // Configure enhanced fonts only once for performance
        if !self.fonts_configured {
            info!("🎨 Initializing enhanced fonts with emoji support");
            fonts::configure_enhanced_fonts(ctx);
            self.fonts_configured = true;
        }

        // Continue with basic font size configuration (disabled to prevent Scene zoom conflicts)
        // The Scene container should handle its own font scaling independently
        let base_font_size = 14.0; // Use consistent base size
        self.configure_font_definitions(ctx, base_font_size);
    }

    /// Configure font definitions for optimal text rendering at all zoom levels
    pub(super) fn configure_font_definitions(&self, ctx: &egui::Context, _base_font_size: f32) {
        // Get current font definitions
        ctx.fonts(|_fonts| {
            // Configure font rasterization settings for crisp text
            // Note: egui uses ab_glyph for font rasterization
            // The font atlas is automatically managed, but we can influence quality
            // by ensuring consistent sizing and avoiding sub-pixel positioning issues
        });
    }
}
