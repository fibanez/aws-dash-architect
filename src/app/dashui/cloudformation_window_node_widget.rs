// CloudFormation Window Node Widget Implementation
//
// This module provides egui::Window-based rendering for CloudFormation nodes
// in the scene graph, using native Window widgets to achieve crisp text rendering
// at all zoom levels, following the pattern of the CloudFormation Resources window.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::dashui::{
    cloudformation_scene_graph::{CloudFormationNodeType, SceneNode},
    AwsIconManager,
};
use egui::{Color32, Image, Response, Ui, Vec2};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Window-based CloudFormation node widget using egui::Window for crisp native text rendering
pub struct CloudFormationWindowNodeWidget;

/// Manager for all node windows in the scene
pub struct NodeWindowManager {
    /// Track which nodes have windows open
    pub open_node_windows: HashMap<String, bool>,

    /// Track window interactions for node selection
    pub window_interactions: HashMap<String, Response>,
}

impl NodeWindowManager {
    pub fn new() -> Self {
        Self {
            open_node_windows: HashMap::new(),
            window_interactions: HashMap::new(),
        }
    }

    /// Render all node windows in the scene
    pub fn render_node_windows(
        &mut self,
        ctx: &egui::Context,
        nodes: &mut HashMap<String, SceneNode>,
        icon_manager: &mut AwsIconManager,
    ) {
        // Clear previous interactions
        self.window_interactions.clear();

        debug!(
            "NodeWindowManager rendering {} nodes as windows",
            nodes.len()
        );

        // Render each node as a window
        for (node_id, node) in nodes.iter_mut() {
            // Ensure the window is marked as open
            self.open_node_windows.insert(node_id.clone(), true);

            debug!(
                "Rendering window for node: {} at position {:?}",
                node_id, node.position
            );

            // Render the node window
            CloudFormationWindowNodeWidget::render_node_window(
                ctx,
                node_id,
                node,
                icon_manager,
                &mut self.window_interactions,
            );
        }
    }

    /// Handle window interactions for node selection and dragging
    pub fn handle_interactions(&self, nodes: &mut HashMap<String, SceneNode>) {
        // Find clicked node first
        let clicked_node_id = self
            .window_interactions
            .iter()
            .find(|(_, response)| response.clicked())
            .map(|(node_id, _)| node_id.clone());

        // Update all nodes based on interactions
        for (node_id, node) in nodes.iter_mut() {
            if let Some(response) = self.window_interactions.get(node_id) {
                // Handle selection
                node.selected = clicked_node_id.as_ref() == Some(node_id);
                // Update hover state
                node.hovered = response.hovered();
            }
        }

        // Log selection
        if let Some(selected_id) = clicked_node_id {
            debug!("Node window clicked: {}", selected_id);
        }
    }
}

impl Default for NodeWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudFormationWindowNodeWidget {
    /// Render a single node as an egui::Window for crisp text at all zoom levels
    pub fn render_node_window(
        ctx: &egui::Context,
        node_id: &str,
        node: &mut SceneNode,
        icon_manager: &mut AwsIconManager,
        window_interactions: &mut HashMap<String, Response>,
    ) {
        let window_title = format!("Node: {}", node.node_type.display_name());

        // Calculate appropriate window size based on node content
        let window_size = Self::calculate_window_size(node);

        // Position the window at the node's scene position
        let window_pos = node.position;

        let window_response = egui::Window::new(&window_title)
            .id(egui::Id::new(format!("cf_node_{}", node_id))) // Unique ID per node
            .title_bar(false) // Remove title bar to look like a node
            .resizable(false) // Fixed size like a node
            .movable(true) // Allow dragging
            .fixed_size(window_size)
            .fixed_pos(window_pos)
            .frame(Self::create_window_frame(node)) // Custom styling
            .show(ctx, |ui| {
                // Render node content using native widgets
                Self::render_window_content(node, ui, icon_manager);
            });

        // Store the window response for interaction handling
        if let Some(window_response) = window_response {
            let response = window_response.response;

            debug!(
                "Window {} successfully created and shown at position {:?}",
                node_id, window_pos
            );

            // Update node position if window was dragged
            let new_pos = response.interact_rect;
            if (new_pos.min - window_pos).length() > 1.0 {
                debug!(
                    "Node {} position changed from {:?} to {:?} (delta: {:?})",
                    node_id,
                    window_pos,
                    new_pos.min,
                    new_pos.min - window_pos
                );
            }
            node.position = new_pos.min;

            // Store response for interaction handling
            window_interactions.insert(node_id.to_string(), response);
        } else {
            warn!(
                "Failed to create window for node: {} at position {:?}",
                node_id, window_pos
            );
        }
    }

    /// Calculate appropriate window size with fixed dimensions (no text estimation)
    fn calculate_window_size(node: &SceneNode) -> Vec2 {
        match &node.node_type {
            CloudFormationNodeType::Resource { .. } => {
                Vec2::new(200.0, 140.0) // Fixed size for resources
            }
            CloudFormationNodeType::Parameter { .. } => {
                Vec2::new(180.0, 130.0) // Fixed size for parameters
            }
            CloudFormationNodeType::Output { .. } => {
                Vec2::new(200.0, 150.0) // Fixed size for outputs
            }
            CloudFormationNodeType::Condition { .. } => {
                Vec2::new(200.0, 150.0) // Fixed size for conditions
            }
        }
    }

    /// Create custom window frame styling to match node appearance
    fn create_window_frame(node: &SceneNode) -> egui::Frame {
        let base_color = node.get_background_color();
        let border_stroke = node.get_border_stroke();

        let (fill_color, stroke) = if node.selected {
            (base_color, egui::Stroke::new(3.0, border_stroke.color))
        } else if node.hovered {
            (
                Color32::from_rgb(
                    (base_color.r() as u16 + 15).min(255) as u8,
                    (base_color.g() as u16 + 15).min(255) as u8,
                    (base_color.b() as u16 + 15).min(255) as u8,
                ),
                egui::Stroke::new(2.5, border_stroke.color),
            )
        } else {
            (base_color, border_stroke)
        };

        let mut frame = egui::Frame::new()
            .fill(fill_color)
            .stroke(stroke)
            .corner_radius(10.0)
            .inner_margin(8.0);

        // Add shadow for selected nodes
        if node.selected {
            frame = frame.shadow(egui::epaint::Shadow {
                color: Color32::from_black_alpha(100),
                offset: [2, 2],
                blur: 8,
                spread: 0,
            });
        }

        frame
    }

    /// Render window content using native egui widgets for crisp text
    fn render_window_content(node: &SceneNode, ui: &mut Ui, icon_manager: &mut AwsIconManager) {
        match &node.node_type {
            CloudFormationNodeType::Resource { .. } => {
                Self::render_resource_window_content(node, ui, icon_manager);
            }
            CloudFormationNodeType::Parameter { .. } => {
                Self::render_parameter_window_content(node, ui);
            }
            CloudFormationNodeType::Output { .. } => {
                Self::render_output_window_content(node, ui);
            }
            CloudFormationNodeType::Condition { .. } => {
                Self::render_condition_window_content(node, ui);
            }
        }
    }

    /// Render resource node content in window using native widgets
    fn render_resource_window_content(
        node: &SceneNode,
        ui: &mut Ui,
        icon_manager: &mut AwsIconManager,
    ) {
        if let CloudFormationNodeType::Resource {
            logical_id,
            resource_type,
            aws_service,
            properties_count,
            description,
            ..
        } = &node.node_type
        {
            ui.vertical(|ui| {
                // Header with icon and service badge
                ui.horizontal(|ui| {
                    // AWS service icon
                    let texture = icon_manager.get_texture_for_resource(ui.ctx(), resource_type);
                    ui.add(Image::from_texture(texture).max_size(Vec2::new(20.0, 20.0)));

                    // Service badge
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        Self::render_service_badge_in_window(ui, aws_service);
                    });
                });

                ui.add_space(4.0);

                // Resource logical ID (main title) - NATIVE TEXT RENDERING
                ui.vertical_centered(|ui| {
                    let title_color = if node.selected {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(240)
                    };
                    ui.colored_label(
                        title_color,
                        egui::RichText::new(logical_id).strong().size(14.0),
                    );
                });

                ui.add_space(2.0);

                // Resource type (subtitle) - NATIVE TEXT RENDERING
                ui.vertical_centered(|ui| {
                    let subtitle_color = if node.selected {
                        Color32::from_gray(220)
                    } else {
                        Color32::from_gray(180)
                    };
                    ui.colored_label(
                        subtitle_color,
                        egui::RichText::new(Self::format_resource_type(resource_type)).size(11.0),
                    );
                });

                // Properties info - NATIVE TEXT RENDERING
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let detail_color = if node.selected {
                        Color32::from_gray(200)
                    } else {
                        Color32::from_gray(140)
                    };

                    ui.colored_label(
                        detail_color,
                        egui::RichText::new(format!("⚙ {} properties", properties_count))
                            .size(10.0),
                    );

                    // Status indicator
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status_color = if node.selected {
                            Color32::from_rgb(100, 255, 100)
                        } else {
                            Color32::from_rgb(100, 200, 100)
                        };
                        ui.colored_label(status_color, egui::RichText::new("●").size(8.0));
                    });
                });

                // Description (if available) - NATIVE TEXT RENDERING
                if let Some(desc) = description {
                    if !desc.is_empty() {
                        ui.add_space(4.0);
                        ui.vertical_centered(|ui| {
                            ui.colored_label(
                                Color32::from_gray(120),
                                egui::RichText::new(Self::truncate_text(desc, 40))
                                    .size(9.0)
                                    .italics(),
                            );
                        });
                    }
                }
            });
        }
    }

    /// Render service badge in window using native widgets
    fn render_service_badge_in_window(ui: &mut Ui, service: &str) {
        let badge_color = Self::get_service_badge_color(service);

        egui::Frame::new()
            .fill(badge_color)
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(4, 2))
            .show(ui, |ui| {
                ui.colored_label(
                    Color32::WHITE,
                    egui::RichText::new(service).size(9.0).strong(),
                );
            });
    }

    /// Render parameter node content in window
    fn render_parameter_window_content(node: &SceneNode, ui: &mut Ui) {
        if let CloudFormationNodeType::Parameter {
            name,
            param_type,
            default_value,
            ..
        } = &node.node_type
        {
            ui.vertical(|ui| {
                // Header with parameter icon and type badge
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📝").size(18.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Parameter type badge
                        egui::Frame::new()
                            .fill(Color32::from_rgb(100, 150, 200))
                            .corner_radius(3.0)
                            .inner_margin(egui::Margin::symmetric(3, 1))
                            .show(ui, |ui| {
                                ui.colored_label(
                                    Color32::WHITE,
                                    egui::RichText::new(param_type).size(8.0).strong(),
                                );
                            });
                    });
                });

                ui.add_space(4.0);

                // Parameter name - NATIVE TEXT
                ui.vertical_centered(|ui| {
                    let title_color = if node.selected {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(240)
                    };
                    ui.colored_label(title_color, egui::RichText::new(name).strong().size(14.0));
                });

                ui.add_space(2.0);

                // Type description - NATIVE TEXT
                ui.vertical_centered(|ui| {
                    let subtitle_color = if node.selected {
                        Color32::from_gray(220)
                    } else {
                        Color32::from_gray(180)
                    };
                    ui.colored_label(
                        subtitle_color,
                        egui::RichText::new("CloudFormation Parameter").size(10.0),
                    );
                });

                // Default value or required indicator - NATIVE TEXT
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if let Some(default) = default_value {
                        let detail_color = if node.selected {
                            Color32::from_gray(200)
                        } else {
                            Color32::from_gray(140)
                        };
                        ui.colored_label(
                            detail_color,
                            egui::RichText::new(format!(
                                "💾 Default: {}",
                                Self::truncate_text(default, 15)
                            ))
                            .size(10.0),
                        );
                    } else {
                        ui.colored_label(
                            Color32::from_rgb(255, 193, 7),
                            egui::RichText::new("⚠ Required").size(10.0),
                        );
                    }
                });
            });
        }
    }

    /// Render output node content in window
    fn render_output_window_content(node: &SceneNode, ui: &mut Ui) {
        if let CloudFormationNodeType::Output { name, value, .. } = &node.node_type {
            ui.vertical(|ui| {
                // Header with output icon and badge
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📤").size(18.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Output badge
                        egui::Frame::new()
                            .fill(Color32::from_rgb(150, 200, 100))
                            .corner_radius(3.0)
                            .inner_margin(egui::Margin::symmetric(3, 1))
                            .show(ui, |ui| {
                                ui.colored_label(
                                    Color32::WHITE,
                                    egui::RichText::new("OUT").size(8.0).strong(),
                                );
                            });
                    });
                });

                ui.add_space(4.0);

                // Output name - NATIVE TEXT
                ui.vertical_centered(|ui| {
                    let title_color = if node.selected {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(240)
                    };
                    ui.colored_label(title_color, egui::RichText::new(name).strong().size(14.0));
                });

                ui.add_space(2.0);

                // Type description - NATIVE TEXT
                ui.vertical_centered(|ui| {
                    let subtitle_color = if node.selected {
                        Color32::from_gray(220)
                    } else {
                        Color32::from_gray(180)
                    };
                    ui.colored_label(
                        subtitle_color,
                        egui::RichText::new("CloudFormation Output").size(10.0),
                    );
                });

                // Value preview - NATIVE TEXT
                ui.add_space(4.0);
                ui.label(egui::RichText::new("🔗 Value:").size(9.0));

                ui.add_space(2.0);

                // Value content with background
                let value_bg_color = if node.selected {
                    Color32::from_rgba_premultiplied(255, 255, 255, 20)
                } else {
                    Color32::from_rgba_premultiplied(255, 255, 255, 10)
                };

                egui::Frame::new()
                    .fill(value_bg_color)
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        let value_color = if node.selected {
                            Color32::from_gray(220)
                        } else {
                            Color32::from_gray(160)
                        };
                        ui.colored_label(
                            value_color,
                            egui::RichText::new(Self::truncate_text(value, 30))
                                .size(9.0)
                                .family(egui::FontFamily::Monospace),
                        );
                    });
            });
        }
    }

    /// Render condition node content in window
    fn render_condition_window_content(node: &SceneNode, ui: &mut Ui) {
        if let CloudFormationNodeType::Condition { name, condition } = &node.node_type {
            ui.vertical(|ui| {
                // Header with condition icon and badge
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("❓").size(18.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Condition badge
                        egui::Frame::new()
                            .fill(Color32::from_rgb(200, 150, 100))
                            .corner_radius(3.0)
                            .inner_margin(egui::Margin::symmetric(3, 1))
                            .show(ui, |ui| {
                                ui.colored_label(
                                    Color32::WHITE,
                                    egui::RichText::new("IF").size(8.0).strong(),
                                );
                            });
                    });
                });

                ui.add_space(4.0);

                // Condition name - NATIVE TEXT
                ui.vertical_centered(|ui| {
                    let title_color = if node.selected {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(240)
                    };
                    ui.colored_label(title_color, egui::RichText::new(name).strong().size(14.0));
                });

                ui.add_space(2.0);

                // Type description - NATIVE TEXT
                ui.vertical_centered(|ui| {
                    let subtitle_color = if node.selected {
                        Color32::from_gray(220)
                    } else {
                        Color32::from_gray(180)
                    };
                    ui.colored_label(
                        subtitle_color,
                        egui::RichText::new("CloudFormation Condition").size(10.0),
                    );
                });

                // Logic preview - NATIVE TEXT
                ui.add_space(4.0);
                ui.label(egui::RichText::new("🔧 Logic:").size(9.0));

                ui.add_space(2.0);

                // Logic content with background
                let logic_bg_color = if node.selected {
                    Color32::from_rgba_premultiplied(255, 255, 255, 20)
                } else {
                    Color32::from_rgba_premultiplied(255, 255, 255, 10)
                };

                egui::Frame::new()
                    .fill(logic_bg_color)
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        let logic_color = if node.selected {
                            Color32::from_gray(220)
                        } else {
                            Color32::from_gray(160)
                        };
                        ui.colored_label(
                            logic_color,
                            egui::RichText::new(Self::format_condition_logic(condition))
                                .size(9.0)
                                .family(egui::FontFamily::Monospace),
                        );
                    });
            });
        }
    }

    /// Get badge color for AWS service (preserved from original implementation)
    fn get_service_badge_color(service: &str) -> Color32 {
        match service {
            "EC2" | "Lambda" | "ECS" | "EKS" => Color32::from_rgb(230, 126, 34), // Orange
            "S3" | "EFS" | "EBS" => Color32::from_rgb(46, 204, 113),             // Green
            "RDS" | "DynamoDB" | "ElastiCache" => Color32::from_rgb(52, 152, 219), // Blue
            "VPC" | "CloudFront" | "Route53" => Color32::from_rgb(155, 89, 182), // Purple
            "IAM" | "KMS" | "SecretsManager" => Color32::from_rgb(231, 76, 60),  // Red
            "SNS" | "SQS" | "EventBridge" => Color32::from_rgb(243, 156, 18),    // Yellow
            _ => Color32::from_gray(120),                                        // Default gray
        }
    }

    /// Format resource type for display (remove AWS:: prefix and shorten)
    fn format_resource_type(resource_type: &str) -> String {
        resource_type
            .strip_prefix("AWS::")
            .unwrap_or(resource_type)
            .replace("::", " → ")
    }

    /// Truncate text to fit in available space
    fn truncate_text(text: &str, max_chars: usize) -> String {
        if text.len() <= max_chars {
            text.to_string()
        } else {
            format!("{}…", &text[..max_chars.saturating_sub(1)])
        }
    }

    /// Format condition logic for better readability
    fn format_condition_logic(condition: &str) -> String {
        // Simplify common CloudFormation condition patterns
        let simplified = condition
            .replace("!Equals", "==")
            .replace("!Not", "!")
            .replace("!And", "&&")
            .replace("!Or", "||")
            .replace("!Ref", "Ref:")
            .replace("[", " [")
            .replace("]", "] ")
            .replace(",", ", ");

        Self::truncate_text(simplified.trim(), 35)
    }
}
