use egui::Color32;
use random_color::{Luminosity, RandomColor};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Color generator for AWS accounts and regions with deterministic seeded colors
///
/// This module provides consistent, attractive color generation for AWS accounts and regions
/// using seeded random color generation. Each account ID and region code will always
/// generate the same color, ensuring visual consistency across sessions.
///
/// Colors are generated using the random_color crate (Rust equivalent of JavaScript's randomColor)
/// with specific luminosity and saturation settings optimized for UI tag backgrounds.
#[derive(Debug)]
pub struct AwsColorGenerator {
    /// Cache of account ID -> Color32 mappings for performance
    account_colors: Arc<RwLock<HashMap<String, Color32>>>,
    /// Cache of region code -> Color32 mappings for performance
    region_colors: Arc<RwLock<HashMap<String, Color32>>>,
}

impl AwsColorGenerator {
    /// Create a new color generator
    pub fn new() -> Self {
        Self {
            account_colors: Arc::new(RwLock::new(HashMap::new())),
            region_colors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a deterministic color for an AWS account ID
    ///
    /// Uses the account ID as a seed to ensure the same account always gets the same color.
    /// Colors are optimized for tag backgrounds with good contrast and visual appeal.
    ///
    /// # Parameters
    /// * `account_id` - The AWS account ID (e.g., "123456789012")
    ///
    /// # Returns
    /// An egui::Color32 suitable for use as a tag background color
    pub fn get_account_color(&self, account_id: &str) -> Color32 {
        // Check cache first
        if let Ok(cache) = self.account_colors.read() {
            if let Some(&color) = cache.get(account_id) {
                return color;
            }
        }

        // Generate new color
        let color = self.generate_account_color(account_id);

        // Cache the result
        if let Ok(mut cache) = self.account_colors.write() {
            cache.insert(account_id.to_string(), color);
        }

        color
    }

    /// Generate a deterministic color for an AWS region
    ///
    /// Uses the region code as a seed to ensure the same region always gets the same color.
    /// Colors are optimized for tag backgrounds with good contrast and visual appeal.
    ///
    /// # Parameters
    /// * `region_code` - The AWS region code (e.g., "us-east-1", "eu-west-2")
    ///
    /// # Returns
    /// An egui::Color32 suitable for use as a tag background color
    pub fn get_region_color(&self, region_code: &str) -> Color32 {
        // Check cache first
        if let Ok(cache) = self.region_colors.read() {
            if let Some(&color) = cache.get(region_code) {
                return color;
            }
        }

        // Generate new color
        let color = self.generate_region_color(region_code);

        // Cache the result
        if let Ok(mut cache) = self.region_colors.write() {
            cache.insert(region_code.to_string(), color);
        }

        color
    }

    /// Generate an account-specific color using seeded random generation
    fn generate_account_color(&self, account_id: &str) -> Color32 {
        let mut random_color = RandomColor::new();

        // Seed with account ID for deterministic results
        random_color.seed(format!("account_{}", account_id));

        // Use light luminosity for pastel-like colors that work well as tag backgrounds
        random_color.luminosity(Luminosity::Light);

        // Generate the color and convert to egui::Color32
        let hex_color = random_color.to_hex();
        self.hex_to_color32(&hex_color)
    }

    /// Generate a region-specific color using seeded random generation
    fn generate_region_color(&self, region_code: &str) -> Color32 {
        // Special case for Global region - use a distinctive gold color
        if region_code == "Global" {
            return Color32::from_rgb(255, 215, 0); // Gold color for global services
        }
        
        let mut random_color = RandomColor::new();

        // Seed with region code for deterministic results
        random_color.seed(format!("region_{}", region_code));

        // Use light luminosity for pastel-like colors that work well as tag backgrounds
        random_color.luminosity(Luminosity::Light);

        // Generate the color and convert to egui::Color32
        let hex_color = random_color.to_hex();
        self.hex_to_color32(&hex_color)
    }

    /// Generate a resource type specific color using seeded random generation
    ///
    /// Uses a hybrid approach for maximum color contrast:
    /// 1. Extracts the resource name (e.g., "Instance" from "AWS::EC2::Instance")
    /// 2. Maps resource name to one of 8 distinct hue families via hash
    /// 3. Alternates luminosity (Bright/Light) based on hash for additional contrast
    ///
    /// This ensures visually distinct colors for different resource types even within
    /// the same AWS service (e.g., Instance, SecurityGroup, VPC all get different hues).
    pub fn get_resource_type_color(&self, resource_type: &str) -> Color32 {
        use random_color::Color;

        // Extract just the resource name (last part after ::)
        // "AWS::EC2::Instance" -> "Instance"
        // "AWS::EC2::SecurityGroup" -> "SecurityGroup"
        let resource_name = resource_type.split("::").last().unwrap_or(resource_type);

        // Create a simple hash of the resource name for hue selection
        let hash = self.hash_string(resource_name);

        // Map hash to one of 8 distinct hue families for maximum color separation
        let hue = match hash % 8 {
            0 => Color::Red,
            1 => Color::Orange,
            2 => Color::Yellow,
            3 => Color::Green,
            4 => Color::Blue,
            5 => Color::Purple,
            6 => Color::Pink,
            _ => Color::Monochrome,
        };

        // Alternate luminosity based on hash parity for additional visual distinction
        let luminosity = if hash % 2 == 0 {
            Luminosity::Bright
        } else {
            Luminosity::Light
        };

        let mut random_color = RandomColor::new();

        // Seed with resource name for deterministic results
        random_color.seed(format!("resource_type_{}", resource_name));
        random_color.hue(hue);
        random_color.luminosity(luminosity);

        // Generate the color and convert to egui::Color32
        let hex_color = random_color.to_hex();
        self.hex_to_color32(&hex_color)
    }

    /// Simple hash function for string-to-number mapping
    fn hash_string(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Generate a tag key specific color using seeded random generation
    ///
    /// Uses the tag key as a seed to ensure the same tag key always gets the same color.
    /// This provides visual consistency for tag badges across resources.
    ///
    /// # Parameters
    /// * `tag_key` - The tag key (e.g., "Environment", "Team", "Project")
    ///
    /// # Returns
    /// An egui::Color32 suitable for use as a tag badge background color
    pub fn get_tag_key_color(&self, tag_key: &str) -> Color32 {
        let mut random_color = RandomColor::new();

        // Seed with tag key for deterministic results
        random_color.seed(format!("tag_key_{}", tag_key));

        // Use light luminosity for pastel-like colors that work well as tag backgrounds
        random_color.luminosity(Luminosity::Light);

        // Generate the color and convert to egui::Color32
        let hex_color = random_color.to_hex();
        self.hex_to_color32(&hex_color)
    }

    /// Convert hex color string to egui::Color32
    ///
    /// # Parameters
    /// * `hex` - Hex color string (e.g., "#FF5733")
    ///
    /// # Returns
    /// egui::Color32 representation of the hex color
    fn hex_to_color32(&self, hex: &str) -> Color32 {
        // Remove '#' if present
        let hex = hex.trim_start_matches('#');

        // Parse hex string to RGB values
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color32::from_rgb(r, g, b);
            }
        }

        // Fallback to a default color if parsing fails
        Color32::from_rgb(128, 128, 128)
    }

    /// Get contrasting text color for a given background color
    ///
    /// Determines whether to use dark or light text based on the background color's luminosity.
    /// Useful for ensuring text readability on colored tag backgrounds.
    ///
    /// # Parameters
    /// * `background_color` - The background color to get contrasting text for
    ///
    /// # Returns
    /// egui::Color32 for text that contrasts well with the background
    pub fn get_contrasting_text_color(&self, background_color: Color32) -> Color32 {
        // Calculate luminosity using standard formula
        let r = background_color.r() as f32 / 255.0;
        let g = background_color.g() as f32 / 255.0;
        let b = background_color.b() as f32 / 255.0;

        // Convert to linear RGB
        let r_linear = if r <= 0.03928 {
            r / 12.92
        } else {
            ((r + 0.055) / 1.055).powf(2.4)
        };
        let g_linear = if g <= 0.03928 {
            g / 12.92
        } else {
            ((g + 0.055) / 1.055).powf(2.4)
        };
        let b_linear = if b <= 0.03928 {
            b / 12.92
        } else {
            ((b + 0.055) / 1.055).powf(2.4)
        };

        // Calculate relative luminosity
        let luminosity = 0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear;

        // Use dark text for light backgrounds, light text for dark backgrounds
        if luminosity > 0.5 {
            Color32::from_rgb(33, 37, 41) // Dark text
        } else {
            Color32::from_rgb(248, 249, 250) // Light text
        }
    }

    /// Clear the color cache
    ///
    /// Useful for testing or if you want to regenerate colors with different settings
    pub fn clear_cache(&self) {
        if let Ok(mut account_cache) = self.account_colors.write() {
            account_cache.clear();
        }
        if let Ok(mut region_cache) = self.region_colors.write() {
            region_cache.clear();
        }
    }

    /// Get cache statistics for monitoring
    pub fn get_cache_stats(&self) -> ColorCacheStats {
        let account_count = self
            .account_colors
            .read()
            .map(|cache| cache.len())
            .unwrap_or(0);

        let region_count = self
            .region_colors
            .read()
            .map(|cache| cache.len())
            .unwrap_or(0);

        ColorCacheStats {
            account_colors_cached: account_count,
            region_colors_cached: region_count,
        }
    }
}

impl Default for AwsColorGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AwsColorGenerator {
    fn clone(&self) -> Self {
        Self {
            account_colors: Arc::clone(&self.account_colors),
            region_colors: Arc::clone(&self.region_colors),
        }
    }
}

/// Statistics about cached colors
#[derive(Debug, Clone)]
pub struct ColorCacheStats {
    pub account_colors_cached: usize,
    pub region_colors_cached: usize,
}

/// Global color generator instance
use once_cell::sync::Lazy;

static GLOBAL_COLOR_GENERATOR: Lazy<AwsColorGenerator> = Lazy::new(AwsColorGenerator::new);

/// Get a deterministic color for an AWS account ID
///
/// Convenience function that uses the global color generator instance.
///
/// # Parameters
/// * `account_id` - The AWS account ID
///
/// # Returns
/// A consistent Color32 for the given account ID
pub fn assign_account_color(account_id: &str) -> Color32 {
    GLOBAL_COLOR_GENERATOR.get_account_color(account_id)
}

/// Get a deterministic color for an AWS region
///
/// Convenience function that uses the global color generator instance.
///
/// # Parameters
/// * `region_code` - The AWS region code
///
/// # Returns
/// A consistent Color32 for the given region code
pub fn assign_region_color(region_code: &str) -> Color32 {
    GLOBAL_COLOR_GENERATOR.get_region_color(region_code)
}

/// Get contrasting text color for a background color
///
/// Convenience function that uses the global color generator instance.
///
/// # Parameters
/// * `background_color` - The background color
///
/// # Returns
/// A contrasting text color for good readability
pub fn get_contrasting_text_color(background_color: Color32) -> Color32 {
    GLOBAL_COLOR_GENERATOR.get_contrasting_text_color(background_color)
}

/// Get a deterministic color for an AWS resource type
///
/// Convenience function that uses the global color generator instance.
///
/// # Parameters
/// * `resource_type` - The AWS resource type (e.g., "AWS::EC2::Instance")
///
/// # Returns
/// A consistent Color32 for the given resource type
pub fn assign_resource_type_color(resource_type: &str) -> Color32 {
    GLOBAL_COLOR_GENERATOR.get_resource_type_color(resource_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_account_colors() {
        let generator = AwsColorGenerator::new();
        let account_id = "123456789012";

        // Same account ID should always produce the same color
        let color1 = generator.get_account_color(account_id);
        let color2 = generator.get_account_color(account_id);

        assert_eq!(color1, color2);
    }

    #[test]
    fn test_deterministic_region_colors() {
        let generator = AwsColorGenerator::new();
        let region = "us-east-1";

        // Same region should always produce the same color
        let color1 = generator.get_region_color(region);
        let color2 = generator.get_region_color(region);

        assert_eq!(color1, color2);
    }

    #[test]
    fn test_different_accounts_different_colors() {
        let generator = AwsColorGenerator::new();

        let account1_color = generator.get_account_color("123456789012");
        let account2_color = generator.get_account_color("987654321098");

        // Different accounts should produce different colors (very likely)
        assert_ne!(account1_color, account2_color);
    }

    #[test]
    fn test_different_regions_different_colors() {
        let generator = AwsColorGenerator::new();

        let region1_color = generator.get_region_color("us-east-1");
        let region2_color = generator.get_region_color("eu-west-2");

        // Different regions should produce different colors (very likely)
        assert_ne!(region1_color, region2_color);
    }

    #[test]
    fn test_hex_to_color32() {
        let generator = AwsColorGenerator::new();

        let color = generator.hex_to_color32("#FF5733");
        assert_eq!(color, Color32::from_rgb(255, 87, 51));

        let color_no_hash = generator.hex_to_color32("FF5733");
        assert_eq!(color_no_hash, Color32::from_rgb(255, 87, 51));
    }

    #[test]
    fn test_contrasting_text_colors() {
        let generator = AwsColorGenerator::new();

        // Light background should get dark text
        let light_bg = Color32::from_rgb(240, 240, 240);
        let text_for_light = generator.get_contrasting_text_color(light_bg);
        assert_eq!(text_for_light, Color32::from_rgb(33, 37, 41));

        // Dark background should get light text
        let dark_bg = Color32::from_rgb(40, 40, 40);
        let text_for_dark = generator.get_contrasting_text_color(dark_bg);
        assert_eq!(text_for_dark, Color32::from_rgb(248, 249, 250));
    }

    #[test]
    fn test_convenience_functions() {
        let account_id = "123456789012";
        let region = "us-east-1";

        // Test convenience functions work
        let account_color = assign_account_color(account_id);
        let region_color = assign_region_color(region);
        let text_color = get_contrasting_text_color(account_color);

        // Should be valid colors (not the fallback gray)
        assert_ne!(account_color, Color32::from_rgb(128, 128, 128));
        assert_ne!(region_color, Color32::from_rgb(128, 128, 128));

        // Text color should be either light or dark
        assert!(
            text_color == Color32::from_rgb(33, 37, 41)
                || text_color == Color32::from_rgb(248, 249, 250)
        );
    }
}
