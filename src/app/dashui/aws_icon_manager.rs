// AWS Icon Management for egui Scene Graph
//
// This module provides texture loading and caching for AWS service icons
// used in the CloudFormation scene graph visualization.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::cfn_resource_icons::get_icon_for_resource;
use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Manager for AWS service icons with texture caching
pub struct AwsIconManager {
    /// Cache of loaded texture handles keyed by icon path
    texture_cache: HashMap<String, TextureHandle>,

    /// Fallback texture for unknown or missing icons
    fallback_texture: Option<TextureHandle>,

    /// Service-specific fallback textures
    service_fallbacks: HashMap<String, TextureHandle>,

    /// Whether the manager has been initialized
    initialized: bool,
}

impl AwsIconManager {
    /// Create a new AWS icon manager
    pub fn new() -> Self {
        Self {
            texture_cache: HashMap::new(),
            fallback_texture: None,
            service_fallbacks: HashMap::new(),
            initialized: false,
        }
    }

    /// Initialize the icon manager with egui context
    pub fn initialize(&mut self, ctx: &Context) {
        if self.initialized {
            return;
        }

        info!("Initializing AWS Icon Manager");

        // Create fallback texture for missing icons
        self.fallback_texture = Some(self.create_fallback_texture(ctx));

        // Create service-specific fallback textures
        self.create_service_fallbacks(ctx);

        self.initialized = true;
        info!("AWS Icon Manager initialized");
    }

    /// Get texture handle for a CloudFormation resource type
    pub fn get_texture_for_resource(
        &mut self,
        ctx: &Context,
        resource_type: &str,
    ) -> &TextureHandle {
        self.ensure_initialized(ctx);

        let icon_path = get_icon_for_resource(resource_type);
        self.get_texture_for_path(ctx, icon_path)
    }

    /// Get texture handle for a specific icon path
    pub fn get_texture_for_path(&mut self, ctx: &Context, icon_path: &str) -> &TextureHandle {
        self.ensure_initialized(ctx);

        // Check cache first
        if self.texture_cache.contains_key(icon_path) {
            return self.texture_cache.get(icon_path).unwrap();
        }

        // Try to load the texture
        if let Some(texture) = self.load_texture_from_path(ctx, icon_path) {
            self.texture_cache.insert(icon_path.to_string(), texture);
            return self.texture_cache.get(icon_path).unwrap();
        }

        // Return fallback if loading failed
        warn!(
            "Failed to load icon from path: {}, using fallback",
            icon_path
        );

        // Try to get a service-specific fallback first
        if let Some(service) = self.extract_service_from_path(icon_path) {
            if let Some(service_fallback) = self.service_fallbacks.get(&service) {
                return service_fallback;
            }
        }

        self.fallback_texture.as_ref().unwrap()
    }

    /// Get the fallback texture
    pub fn get_fallback_texture(&mut self, ctx: &Context) -> &TextureHandle {
        self.ensure_initialized(ctx);
        self.fallback_texture.as_ref().unwrap()
    }

    /// Clear the texture cache
    pub fn clear_cache(&mut self) {
        info!("Clearing AWS icon texture cache");
        self.texture_cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let cached_textures = self.texture_cache.len();
        let total_memory_estimate = cached_textures * 1024; // Rough estimate
        (cached_textures, total_memory_estimate)
    }

    /// Preload common AWS service icons for better performance
    pub fn preload_common_icons(&mut self, ctx: &Context) {
        info!("Preloading common AWS service icons");

        let common_resource_types = [
            "AWS::EC2::Instance",
            "AWS::Lambda::Function",
            "AWS::S3::Bucket",
            "AWS::IAM::Role",
            "AWS::RDS::DBInstance",
            "AWS::DynamoDB::Table",
            "AWS::SNS::Topic",
            "AWS::SQS::Queue",
            "AWS::ApiGateway::RestApi",
            "AWS::ECS::Service",
            "AWS::EKS::Cluster",
            "AWS::CloudFormation::Stack",
        ];

        for resource_type in &common_resource_types {
            let icon_path = get_icon_for_resource(resource_type);
            if !self.texture_cache.contains_key(icon_path) {
                if let Some(texture) = self.load_texture_from_path(ctx, icon_path) {
                    self.texture_cache.insert(icon_path.to_string(), texture);
                    debug!("Preloaded icon for {}", resource_type);
                }
            }
        }

        info!("Preloaded {} common icons", self.texture_cache.len());
    }

    /// Ensure the manager is initialized
    fn ensure_initialized(&mut self, ctx: &Context) {
        if !self.initialized {
            self.initialize(ctx);
        }
    }

    /// Load texture from file path
    fn load_texture_from_path(&self, ctx: &Context, path: &str) -> Option<TextureHandle> {
        debug!("Attempting to load texture from path: {}", path);

        // Try to load the image file
        match image::open(path) {
            Ok(img) => {
                let rgba_image = img.to_rgba8();
                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                let pixels = rgba_image.as_flat_samples();

                let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                let texture = ctx.load_texture(
                    format!("aws_icon_{}", path.replace(['/', '\\'], "_")),
                    color_image,
                    TextureOptions::default(),
                );

                debug!("Successfully loaded texture from: {}", path);
                Some(texture)
            }
            Err(e) => {
                warn!("Failed to load image from {}: {}", path, e);
                None
            }
        }
    }

    /// Create a fallback texture for missing icons
    fn create_fallback_texture(&self, ctx: &Context) -> TextureHandle {
        debug!("Creating fallback texture for missing icons");

        // Create a simple 16x16 icon with AWS orange color and a question mark
        let size = 16;
        let mut pixels = Vec::with_capacity(size * size * 4);

        for y in 0..size {
            for x in 0..size {
                let is_border = x == 0 || x == size - 1 || y == 0 || y == size - 1;
                let is_question_mark = self.is_question_mark_pixel(x, y, size);

                if is_border {
                    // Orange border
                    pixels.extend_from_slice(&[255, 153, 0, 255]);
                } else if is_question_mark {
                    // White question mark
                    pixels.extend_from_slice(&[255, 255, 255, 255]);
                } else {
                    // Transparent background
                    pixels.extend_from_slice(&[255, 153, 0, 100]);
                }
            }
        }

        let color_image = ColorImage::from_rgba_unmultiplied([size, size], &pixels);

        ctx.load_texture(
            "aws_fallback_icon".to_string(),
            color_image,
            TextureOptions::default(),
        )
    }

    /// Check if a pixel should be part of the question mark pattern
    fn is_question_mark_pixel(&self, x: usize, y: usize, size: usize) -> bool {
        // Simple question mark pattern for 16x16
        if size != 16 {
            return false;
        }

        // Question mark pattern (rough approximation)
        match (x, y) {
            // Top arc
            (5..=10, 3) | (4, 4) | (11, 4) | (4, 5) | (11, 5) => true,
            // Middle section
            (10, 6) | (9, 7) | (8, 8) => true,
            // Dot
            (8, 11) => true,
            _ => false,
        }
    }

    /// Create service-specific fallback textures
    fn create_service_fallbacks(&mut self, ctx: &Context) {
        debug!("Creating service-specific fallback textures");

        let services = [
            ("EC2", [255, 153, 0]),       // Orange
            ("Lambda", [255, 159, 0]),    // Orange
            ("S3", [142, 215, 73]),       // Green
            ("RDS", [76, 137, 255]),      // Blue
            ("DynamoDB", [76, 137, 255]), // Blue
            ("IAM", [255, 71, 115]),      // Red
            ("VPC", [147, 104, 255]),     // Purple
            ("SNS", [255, 87, 34]),       // Deep Orange
            ("SQS", [255, 87, 34]),       // Deep Orange
        ];

        for (service, color) in &services {
            let texture = self.create_service_fallback_texture(ctx, service, *color);
            self.service_fallbacks.insert(service.to_string(), texture);
        }

        debug!(
            "Created {} service-specific fallbacks",
            self.service_fallbacks.len()
        );
    }

    /// Create a service-specific fallback texture
    fn create_service_fallback_texture(
        &self,
        ctx: &Context,
        service: &str,
        color: [u8; 3],
    ) -> TextureHandle {
        let size = 16;
        let mut pixels = Vec::with_capacity(size * size * 4);

        // Create a simple colored square with service name initial
        for y in 0..size {
            for x in 0..size {
                let is_border = x == 0 || x == size - 1 || y == 0 || y == size - 1;
                let is_center = (6..=9).contains(&x) && (6..=9).contains(&y);

                if is_border {
                    // Colored border
                    pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
                } else if is_center {
                    // White center area (could add service initial here)
                    pixels.extend_from_slice(&[255, 255, 255, 255]);
                } else {
                    // Semi-transparent background with service color
                    pixels.extend_from_slice(&[color[0], color[1], color[2], 100]);
                }
            }
        }

        let color_image = ColorImage::from_rgba_unmultiplied([size, size], &pixels);

        ctx.load_texture(
            format!("aws_fallback_{}", service.to_lowercase()),
            color_image,
            TextureOptions::default(),
        )
    }

    /// Extract service name from icon path or resource type
    fn extract_service_from_path(&self, path: &str) -> Option<String> {
        // Try to extract from path that might contain resource types like "AWS::EC2::Instance"
        if path.contains("::") {
            return path.split("::").nth(1).map(|s| s.to_string());
        }

        // Try to extract from file path
        if path.contains("Architecture-Service-Icons") {
            // Pattern like "Arch_Amazon-EC2_16.png" or "Arch_AWS-Lambda_16.png"
            if let Some(filename) = path.split('/').next_back() {
                if let Some(service_part) = filename.split('_').nth(1) {
                    // Remove "Amazon-" or "AWS-" prefix and file extension
                    let clean_service = service_part
                        .replace("Amazon-", "")
                        .replace("AWS-", "")
                        .replace(".png", "");
                    return Some(clean_service);
                }
            }
        }

        None
    }
}

impl Default for AwsIconManager {
    fn default() -> Self {
        Self::new()
    }
}
