//! Enhanced Font Configuration for AWS Dash
//!
//! This module provides enhanced emoji and multilingual font support using Noto fonts.
//! It configures font fallback chains to support more emojis and characters beyond the
//! default egui font capabilities.

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;
use tracing::info;

/// Configure enhanced fonts with emoji fallback support
/// 
/// This function adds Noto Sans and Noto Color Emoji fonts as fallbacks to the existing fonts:
/// - Preserves original egui font appearance for English text
/// - Better emoji coverage as fallback (supporting Unicode 16.0 as of 2024)
/// - Multilingual character support as fallback
/// - Font fallback chains for missing glyphs only
/// 
/// Should be called once during application initialization for best performance.
pub fn configure_enhanced_fonts(ctx: &egui::Context) {
    info!("🎨 Adding enhanced emoji fallback fonts while preserving original font appearance");
    
    let mut fonts = FontDefinitions::default();
    
    // Add Noto Color Emoji font data
    fonts.font_data.insert(
        "noto_emoji".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/NotoColorEmoji.ttf")))
    );
    
    // Add Noto Sans font data
    fonts.font_data.insert(
        "noto_sans".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/NotoSans-Regular.ttf")))
    );
    
    // Configure Proportional font family with priority order
    // Keep original fonts first, add emoji fonts as fallbacks
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("noto_emoji".to_owned()); // Add emoji font as fallback
    
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("noto_sans".to_owned()); // Add text font as last fallback
    
    // Configure Monospace font family with same priority order
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("noto_emoji".to_owned()); // Add emoji font as fallback
    
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("noto_sans".to_owned()); // Add text font as last fallback
    
    // Apply the enhanced font configuration
    ctx.set_fonts(fonts);
    
    info!("✅ Enhanced emoji fallback fonts configured - original fonts preserved, emojis available as fallback");
}

/// Test emoji rendering capabilities
/// 
/// This function can be used to test if the enhanced fonts are working properly
/// by trying to render various emoji categories.
pub fn test_emoji_support() -> Vec<String> {
    vec![
        // Basic emojis
        "😀 😃 😄 😁 😆 😅 😂 🤣".to_string(),
        // Activity emojis for our log analysis
        "🔍 🔎 📊 📈 📉 📋 📝 📖".to_string(),
        // Tool and process emojis
        "🚀 🛠️ 🔧 🔨 ⚙️ 🎯 ✨".to_string(),
        // Status emojis
        "✅ ❌ ⚠️ ℹ️ 💡 🔥 ⭐".to_string(),
        // Completion emojis
        "🏁 🏆 🎉 🎊 🎁 🎪".to_string(),
        // Technical symbols
        "⚡ 🖥️ 💻 📱 🌐 🔒 🔓".to_string(),
    ]
}

/// Get enhanced icon mappings for log analysis events
/// 
/// Returns a comprehensive set of emojis that can be used for different
/// log analysis activities and events.
pub fn get_log_analysis_icons() -> LogAnalysisIcons {
    LogAnalysisIcons {
        query_start: "🔍",
        model_start: "🚀",
        tool_start: "🔧",
        tool_complete_success: "✅",
        tool_complete_failure: "❌",
        discovery: "🔎",
        retrieval: "📖",
        analysis: "⚡",
        completion: "🏁",
        error: "❌",
        warning: "⚠️",
        info: "ℹ️",
        logs: "📄",
        events: "📋",
        patterns: "🎯",
        insights: "💡",
        metrics: "📊",
        timeline: "📈",
    }
}

/// Icon mappings for log analysis events
pub struct LogAnalysisIcons {
    pub query_start: &'static str,
    pub model_start: &'static str,
    pub tool_start: &'static str,
    pub tool_complete_success: &'static str,
    pub tool_complete_failure: &'static str,
    pub discovery: &'static str,
    pub retrieval: &'static str,
    pub analysis: &'static str,
    pub completion: &'static str,
    pub error: &'static str,
    pub warning: &'static str,
    pub info: &'static str,
    pub logs: &'static str,
    pub events: &'static str,
    pub patterns: &'static str,
    pub insights: &'static str,
    pub metrics: &'static str,
    pub timeline: &'static str,
}