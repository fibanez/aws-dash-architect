// CloudFormation Graph Visualization using egui Scene + Native Widgets
//
// This module provides an interactive graph visualization for CloudFormation
// resources, parameters, outputs, and conditions using egui's Scene container
// and native widgets for enhanced zoom/pan and interaction capabilities.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::dashui::window_focus::{FocusableWindow, SimpleShowParams};
use crate::app::dashui::{AwsIconManager, NodeWindowManager};
use crate::app::projects::{CloudFormationResource, Project};
use egui::{Color32, Pos2, Rect, Ui, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, trace, warn};

// Constants for coordinate persistence in CloudFormation template metadata
pub const SCENE_METADATA_KEY: &str = "AwsDashScene";
pub const POSITION_KEY: &str = "position";

/// Main CloudFormation Scene Graph Window using egui Scene container
pub struct CloudFormationSceneGraph {
    /// Whether the window is currently shown
    pub show: bool,

    /// Scene rectangle for the egui Scene container
    pub scene_rect: Rect,

    /// All nodes positioned in the scene
    pub nodes: HashMap<String, SceneNode>,

    /// Currently selected node IDs
    pub selected_nodes: Vec<String>,

    /// Window title
    pub title: String,

    /// Layout dirty flag to trigger re-layout
    pub layout_dirty: bool,

    /// AWS icon manager for texture loading and caching
    pub icon_manager: AwsIconManager,

    /// Current zoom level for font scaling
    pub current_zoom: f32,

    /// Base font size before zoom adjustments
    pub base_font_size: f32,

    /// Window manager for node windows
    pub node_window_manager: NodeWindowManager,

    /// Whether drag mode is enabled for moving nodes
    pub drag_mode_enabled: bool,
}

/// Represents a positioned node in the scene
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneNode {
    /// Position in scene coordinates
    pub position: Pos2,

    /// Node size
    pub size: Vec2,

    /// The CloudFormation element this node represents
    pub node_type: CloudFormationNodeType,

    /// List of resource IDs this node depends on
    pub dependencies: Vec<String>,

    /// List of resource IDs that depend on this node
    pub dependents: Vec<String>,

    /// Whether this node is currently selected
    pub selected: bool,

    /// Whether this node is currently hovered
    pub hovered: bool,

    /// Whether this node was being dragged in the previous frame (for detecting drag end)
    #[serde(skip)]
    pub was_dragging: bool,
}

/// Types of CloudFormation elements that can be visualized
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CloudFormationNodeType {
    /// AWS CloudFormation Resource
    Resource {
        logical_id: String,
        resource_type: String,
        aws_service: String,
        icon_path: String,
        properties_count: usize,
        description: Option<String>,
    },

    /// CloudFormation Parameter
    Parameter {
        name: String,
        param_type: String,
        default_value: Option<String>,
        description: Option<String>,
    },

    /// CloudFormation Output
    Output {
        name: String,
        value: String,
        description: Option<String>,
    },

    /// CloudFormation Condition
    Condition { name: String, condition: String },
}

impl CloudFormationNodeType {
    /// Get the display name for this node
    pub fn display_name(&self) -> String {
        match self {
            CloudFormationNodeType::Resource { logical_id, .. } => logical_id.clone(),
            CloudFormationNodeType::Parameter { name, .. } => name.clone(),
            CloudFormationNodeType::Output { name, .. } => name.clone(),
            CloudFormationNodeType::Condition { name, .. } => name.clone(),
        }
    }

    /// Get the type description for this node
    pub fn type_description(&self) -> String {
        match self {
            CloudFormationNodeType::Resource { resource_type, .. } => resource_type.clone(),
            CloudFormationNodeType::Parameter { param_type, .. } => {
                format!("Parameter ({})", param_type)
            }
            CloudFormationNodeType::Output { .. } => "Output".to_string(),
            CloudFormationNodeType::Condition { .. } => "Condition".to_string(),
        }
    }

    /// Extract AWS service name from resource type
    pub fn aws_service(&self) -> String {
        match self {
            CloudFormationNodeType::Resource { aws_service, .. } => aws_service.clone(),
            _ => "CloudFormation".to_string(),
        }
    }
}

impl SceneNode {
    /// Create a new scene node from a CloudFormation resource
    pub fn from_resource(resource: &CloudFormationResource, position: Pos2) -> Self {
        use crate::app::cfn_resource_icons::get_icon_for_resource;

        let aws_service = Self::extract_service(&resource.resource_type);
        let icon_path = get_icon_for_resource(&resource.resource_type).to_string();

        // Use the position passed in (already loaded from DAG or calculated)
        // No need to reload from metadata here since create_from_project handles position loading

        Self {
            position,
            size: Vec2::new(180.0, 120.0), // Increased height to show all content properly
            node_type: CloudFormationNodeType::Resource {
                logical_id: resource.resource_id.clone(),
                resource_type: resource.resource_type.clone(),
                aws_service,
                icon_path,
                properties_count: resource.properties.len(),
                description: None,
            },
            dependencies: Vec::new(),
            dependents: Vec::new(),
            selected: false,
            hovered: false,
            was_dragging: false,
        }
    }

    /// Extract AWS service name from resource type (e.g., "AWS::EC2::Instance" -> "EC2")
    fn extract_service(resource_type: &str) -> String {
        resource_type
            .split("::")
            .nth(1)
            .unwrap_or("Unknown")
            .to_string()
    }

    /// Get the background color for this node based on its type and state
    pub fn get_background_color(&self) -> Color32 {
        let base_color = match &self.node_type {
            CloudFormationNodeType::Resource { aws_service, .. } => {
                get_aws_service_color(aws_service)
            }
            CloudFormationNodeType::Parameter { .. } => Color32::from_rgb(100, 150, 200),
            CloudFormationNodeType::Output { .. } => Color32::from_rgb(150, 200, 100),
            CloudFormationNodeType::Condition { .. } => Color32::from_rgb(200, 150, 100),
        };

        if self.selected {
            // Brighten selected nodes
            Color32::from_rgb(
                (base_color.r() as u16 + 40).min(255) as u8,
                (base_color.g() as u16 + 40).min(255) as u8,
                (base_color.b() as u16 + 40).min(255) as u8,
            )
        } else if self.hovered {
            // Slightly brighten hovered nodes
            Color32::from_rgb(
                (base_color.r() as u16 + 20).min(255) as u8,
                (base_color.g() as u16 + 20).min(255) as u8,
                (base_color.b() as u16 + 20).min(255) as u8,
            )
        } else {
            // Darken for background fill
            Color32::from_rgb(
                (base_color.r() as f32 * 0.3) as u8,
                (base_color.g() as f32 * 0.3) as u8,
                (base_color.b() as f32 * 0.3) as u8,
            )
        }
    }

    /// Get the border stroke for this node based on its state
    pub fn get_border_stroke(&self) -> egui::Stroke {
        let color = match &self.node_type {
            CloudFormationNodeType::Resource { aws_service, .. } => {
                get_aws_service_color(aws_service)
            }
            CloudFormationNodeType::Parameter { .. } => Color32::from_rgb(100, 150, 200),
            CloudFormationNodeType::Output { .. } => Color32::from_rgb(150, 200, 100),
            CloudFormationNodeType::Condition { .. } => Color32::from_rgb(200, 150, 100),
        };

        let width = if self.selected { 3.0 } else { 2.0 };
        egui::Stroke::new(width, color)
    }
}

impl CloudFormationSceneGraph {
    /// Load position from CloudFormation template metadata for a specific resource
    fn load_position_from_template_metadata(project: &Project, resource_id: &str) -> Option<Pos2> {
        // Try to get position from CloudFormation template metadata
        if let Some(template) = &project.cfn_template {
            if let Some(cfn_resource) = template.resources.get(resource_id) {
                if let Some(metadata) = &cfn_resource.metadata {
                    if let Some(scene_data) = metadata.get(SCENE_METADATA_KEY) {
                        if let Some(position_data) = scene_data.get(POSITION_KEY) {
                            if let (Some(x), Some(y)) =
                                (position_data.get("x"), position_data.get("y"))
                            {
                                if let (Some(x_val), Some(y_val)) = (x.as_f64(), y.as_f64()) {
                                    info!(
                                        "🎯 TEMPLATE_METADATA: Found position for {} in template: ({:.1}, {:.1})",
                                        resource_id, x_val, y_val
                                    );
                                    return Some(Pos2::new(x_val as f32, y_val as f32));
                                }
                            }
                        }
                    }
                }
            }
        }

        info!(
            "❌ TEMPLATE_METADATA: No position found for {} in template metadata",
            resource_id
        );
        None
    }

    /// Create a new CloudFormation scene graph
    pub fn new() -> Self {
        Self {
            show: false,
            scene_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            nodes: HashMap::new(),
            selected_nodes: Vec::new(),
            title: "CloudFormation Resource Graph (Scene)".to_string(),
            layout_dirty: false,
            icon_manager: AwsIconManager::new(),
            current_zoom: 1.0,
            base_font_size: 14.0,
            node_window_manager: NodeWindowManager::new(),
            drag_mode_enabled: false,
        }
    }

    /// Show the scene graph window (maximized and non-movable)
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.show {
            return;
        }

        // Log once per frame to track graph activity
        trace!(
            "🎬 SCENE_GRAPH_ACTIVE: Rendering frame with {} nodes",
            self.nodes.len()
        );

        info!(
            "🎯 SHOW: Showing CloudFormation scene graph window with {} nodes",
            self.nodes.len()
        );

        // Log node positions at start of show method
        for (node_id, node) in &self.nodes {
            info!(
                "🎯 SHOW: Node {} at position ({:.1}, {:.1}) before rendering",
                node_id, node.position.x, node.position.y
            );
        }

        // Get the full viewport size
        let viewport_rect = ctx.screen_rect();

        // Adjust position to account for top menu (approximately 40px height)
        let menu_height = 40.0;
        let adjusted_pos = egui::Pos2::new(viewport_rect.min.x, viewport_rect.min.y + menu_height);
        let adjusted_size =
            egui::Vec2::new(viewport_rect.width(), viewport_rect.height() - menu_height);

        let _window_result = egui::Window::new(&self.title)
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .title_bar(true)
            .fixed_size(adjusted_size)
            .fixed_pos(adjusted_pos)
            .show(ctx, |ui| {
                // Top toolbar (compact)
                let close_requested = self.render_toolbar(ui);
                if close_requested {
                    self.show = false;
                }

                ui.separator();

                // Main scene area (fills remaining space)
                let available_rect = ui.available_rect_before_wrap();
                self.render_scene(ui, available_rect);
            });

        // Note: Nodes are now rendered inside the Scene container, not as independent windows
    }

    /// Render the toolbar with controls
    fn render_toolbar(&mut self, ui: &mut Ui) -> bool {
        let mut close_requested = false;

        ui.horizontal(|ui| {
            // Close button
            if ui.button("❌ Close").clicked() {
                close_requested = true;
            }

            ui.separator();

            // Layout controls
            if ui.button("🔄 Auto Layout").clicked() {
                self.apply_auto_layout();
            }

            if ui.button("🔍 Fit to View").clicked() {
                self.fit_to_view();
            }

            if ui.button("🎯 Center").clicked() {
                self.center_view();
            }

            if ui.button("🔍 Reset View").clicked() {
                // Reset the scene rectangle to default
                self.scene_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
            }

            ui.separator();

            // Scene info - the Scene container handles zoom internally
            ui.label(format!("Nodes: {}", self.nodes.len()));
            ui.label(format!("Selected: {}", self.selected_nodes.len()));
            ui.label(format!(
                "Scene: {:.0}x{:.0}",
                self.scene_rect.width(),
                self.scene_rect.height()
            ));

            ui.separator();

            // Show current interaction mode
            let shift_held = ui.input(|i| i.modifiers.shift);
            if shift_held {
                ui.colored_label(
                    Color32::from_rgb(100, 200, 100),
                    "📍 Node Move Mode (Shift+Drag)",
                );
            } else {
                ui.label("🖐️ Pan Mode (Drag to pan)");
            }

            // Help text for Scene container controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.small("🖱️ Wheel: Zoom | Drag: Pan | Shift+Drag: Move Nodes | ❌ Close window");
            });
        });

        close_requested
    }

    /// Render the main scene area with proper egui::Scene container
    fn render_scene(&mut self, ui: &mut Ui, _available_rect: Rect) {
        // Use proper egui::Scene container for pan/zoom functionality
        let nodes_len = self.nodes.len();
        let mut nodes = std::mem::take(&mut self.nodes);
        let mut icon_manager = std::mem::take(&mut self.icon_manager);

        // Store scene_rect to avoid borrow checker issues
        let current_scene_rect = self.scene_rect;

        egui::containers::Scene::new()
            .zoom_range(0.1..=5.0) // Allow zoom from 10% to 500%
            .max_inner_size(egui::Vec2::new(5000.0, 5000.0)) // Large canvas for nodes
            .show(ui, &mut self.scene_rect, |scene_ui| {
                info!(
                    "🎯 RENDER: Rendering {} nodes inside Scene container",
                    nodes_len
                );
                info!(
                    "🎯 SCENE_RECT: Current scene_rect: {:?}",
                    current_scene_rect
                );

                // Log all node positions before rendering
                for (node_id, node) in &nodes {
                    info!(
                        "🎯 RENDER: Node {} at position ({:.1}, {:.1})",
                        node_id, node.position.x, node.position.y
                    );
                }

                // Render all nodes with proper interaction priority
                Self::render_nodes_with_dragging_priority(scene_ui, &mut nodes, &mut icon_manager);
            });

        // Put everything back
        self.nodes = nodes;
        self.icon_manager = icon_manager;

        // Draw scene info overlay outside the scene
        self.draw_info_overlay_in_scene(ui);
    }

    /// Render nodes with proper interaction priority for dragging (static method)
    fn render_nodes_with_dragging_priority(
        ui: &mut Ui,
        nodes: &mut HashMap<String, SceneNode>,
        icon_manager: &mut AwsIconManager,
    ) {
        // Check if Shift key is held (for visual feedback only)
        let shift_held = ui.input(|i| i.modifiers.shift);

        // Track if any node is being dragged
        let mut _any_node_dragging = false;

        for (node_id, node) in nodes.iter_mut() {
            // Position the node at its scene coordinates
            let node_rect = Rect::from_min_size(node.position, node.size);

            // LOG: Actual egui coordinates being used for rendering
            info!(
                "🎯 EGUI_DRAW: Node {} - node.position=({:.1}, {:.1}), node_rect={:?}",
                node_id, node.position.x, node.position.y, node_rect
            );

            // STEP 1: Allocate interaction area
            // Always allow dragging - the Scene container handles pan/zoom separately
            let sense = egui::Sense::click_and_drag();

            let response = ui.allocate_rect(node_rect, sense);

            // LOG: Response rect from egui allocation
            info!(
                "🎯 EGUI_RESPONSE: Node {} - response.rect={:?}, sense={:?}",
                node_id, response.rect, sense
            );

            // STEP 2: Handle interactions BEFORE rendering (priority over scene)
            let is_dragging = response.dragged_by(egui::PointerButton::Primary);
            let is_hovered = response.hovered();
            let is_clicked = response.clicked();
            let drag_delta = response.drag_delta();
            let drag_started = response.drag_started();
            let drag_released = response.drag_stopped();

            // DEBUG: Log all interaction states for this node
            if is_hovered
                || is_clicked
                || is_dragging
                || drag_started
                || drag_released
                || drag_delta.length() > 0.0
            {
                info!("🔍 DRAG_DEBUG: Node {} - hovered:{}, clicked:{}, dragging:{}, drag_started:{}, drag_released:{}, drag_delta:{:?}",
                      node_id, is_hovered, is_clicked, is_dragging, drag_started, drag_released, drag_delta);
            }

            if is_dragging {
                let old_pos = node.position;
                node.position += drag_delta;
                _any_node_dragging = true;
                // Log every drag movement to track position changes
                info!("✅ DRAG_ACTIVE: Node {} dragged from ({:.1}, {:.1}) to ({:.1}, {:.1}) (delta: {:?})",
                      node_id, old_pos.x, old_pos.y, node.position.x, node.position.y, drag_delta);
                trace!(
                    "📍 POSITION_UPDATE: Node {} now at exact position ({}, {})",
                    node_id,
                    node.position.x,
                    node.position.y
                );
            }

            // Detect drag end (was dragging, but not anymore)
            if node.was_dragging && !is_dragging {
                info!(
                    "🎯 DRAG_END: Node {} drag ended at final position ({:.1}, {:.1})",
                    node_id, node.position.x, node.position.y
                );
                info!(
                    "💾 DRAG_END: Node {} position will be saved to project",
                    node_id
                );
                // Log exact final position for verification
                info!(
                    "📍 FINAL_POSITION: Node {} final coordinates: x={}, y={}",
                    node_id, node.position.x, node.position.y
                );
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                info!("Node {} clicked", node_id);
                node.selected = !node.selected;
            }

            // Update states
            node.hovered = response.hovered();
            node.was_dragging = is_dragging;

            // STEP 3: Render visual content without additional interaction
            ui.scope_builder(egui::UiBuilder::new().max_rect(node_rect), |ui| {
                // LOG: UI coordinates for visual rendering
                info!("🎯 EGUI_RENDER_UI: Node {} - ui.max_rect()={:?}, ui.available_rect_before_wrap()={:?}",
                      node_id, ui.max_rect(), ui.available_rect_before_wrap());

                // Don't set clip rect to allow content to render fully
                // ui.set_clip_rect(node_rect); // Removed to prevent clipping
                Self::render_styled_node_with_icons_static(ui, node_id, node, icon_manager, shift_held);
            });
        }
    }

    /// Render node using styled egui widgets with colors, borders, and icons (static method)
    fn render_styled_node_with_icons_static(
        ui: &mut egui::Ui,
        node_id: &str,
        node: &SceneNode,
        icon_manager: &mut AwsIconManager,
        shift_held: bool,
    ) -> egui::Response {
        // LOG: Final frame rendering coordinates
        info!(
            "🎯 EGUI_FRAME: Node {} - ui.available_rect_before_wrap()={:?}",
            node_id,
            ui.available_rect_before_wrap()
        );

        // Create styled frame with colors and borders
        let (background_color, mut border_color, mut border_width) = Self::get_node_styling(node);

        // If shift is held, add visual indication that nodes are draggable
        if shift_held {
            border_color = border_color.linear_multiply(1.3); // Brighter border
            border_width += 0.5; // Slightly thicker border
        }

        let frame = egui::Frame::new()
            .fill(background_color)
            .stroke(egui::Stroke::new(border_width, border_color))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(8));

        let frame_response = frame.show(ui, |ui| {
            // LOG: Inside frame coordinates
            info!(
                "🎯 EGUI_FRAME_INNER: Node {} - frame inner ui.available_rect_before_wrap()={:?}",
                node_id,
                ui.available_rect_before_wrap()
            );
            match &node.node_type {
                CloudFormationNodeType::Resource {
                    logical_id,
                    resource_type,
                    aws_service,
                    properties_count,
                    ..
                } => {
                    // Header with icon and service badge
                    ui.horizontal(|ui| {
                        // CloudFormation resource icon - load actual texture
                        let texture_handle =
                            icon_manager.get_texture_for_resource(ui.ctx(), resource_type);
                        ui.add(
                            egui::Image::from_texture(texture_handle)
                                .max_size(egui::Vec2::new(16.0, 16.0)),
                        );

                        // Service badge with color
                        Self::render_colored_service_badge(ui, aws_service);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Status indicator
                            let status_color = if node.selected {
                                egui::Color32::from_rgb(100, 255, 100)
                            } else {
                                egui::Color32::from_rgb(100, 200, 100)
                            };
                            ui.colored_label(status_color, "●");
                        });
                    });

                    ui.add_space(4.0);

                    // Resource title with black bold text
                    ui.colored_label(
                        egui::Color32::BLACK,
                        egui::RichText::new(logical_id).strong().size(14.0),
                    );

                    ui.add_space(2.0);

                    // Resource type as subtitle
                    let subtitle_color = Self::get_subtitle_color_for_node(node);
                    ui.colored_label(subtitle_color, Self::format_resource_type(resource_type));

                    ui.add_space(4.0);

                    // Properties info with button
                    ui.horizontal(|ui| {
                        let detail_color = Self::get_detail_color_for_node(node);
                        ui.colored_label(
                            detail_color,
                            format!("⚙ {} properties", properties_count),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Edit").clicked() {
                                // Handle click
                            }
                        });
                    });
                }
                CloudFormationNodeType::Parameter {
                    name, param_type, ..
                } => {
                    // Header with parameter icon
                    ui.horizontal(|ui| {
                        ui.label("📝");
                        Self::render_colored_type_badge(
                            ui,
                            param_type,
                            egui::Color32::from_rgb(100, 150, 200),
                        );
                    });

                    ui.add_space(4.0);

                    // Parameter name with colored text
                    let title_color = Self::get_title_color_for_node(node);
                    ui.colored_label(title_color, egui::RichText::new(name).strong().size(14.0));

                    ui.add_space(2.0);

                    // Type description
                    let subtitle_color = Self::get_subtitle_color_for_node(node);
                    ui.colored_label(subtitle_color, "CloudFormation Parameter");
                }
                CloudFormationNodeType::Output { name, .. } => {
                    // Header with output icon
                    ui.horizontal(|ui| {
                        ui.label("📤");
                        Self::render_colored_type_badge(
                            ui,
                            "OUT",
                            egui::Color32::from_rgb(150, 200, 100),
                        );
                    });

                    ui.add_space(4.0);

                    // Output name with colored text
                    let title_color = Self::get_title_color_for_node(node);
                    ui.colored_label(title_color, egui::RichText::new(name).strong().size(14.0));

                    ui.add_space(2.0);

                    // Type description
                    let subtitle_color = Self::get_subtitle_color_for_node(node);
                    ui.colored_label(subtitle_color, "CloudFormation Output");
                }
                CloudFormationNodeType::Condition { name, .. } => {
                    // Header with condition icon
                    ui.horizontal(|ui| {
                        ui.label("❓");
                        Self::render_colored_type_badge(
                            ui,
                            "IF",
                            egui::Color32::from_rgb(200, 150, 100),
                        );
                    });

                    ui.add_space(4.0);

                    // Condition name with colored text
                    let title_color = Self::get_title_color_for_node(node);
                    ui.colored_label(title_color, egui::RichText::new(name).strong().size(14.0));

                    ui.add_space(2.0);

                    // Type description
                    let subtitle_color = Self::get_subtitle_color_for_node(node);
                    ui.colored_label(subtitle_color, "CloudFormation Condition");
                }
            }
        });

        // LOG: Final frame response coordinates
        info!(
            "🎯 EGUI_FRAME_RESPONSE: Node {} - frame_response.rect={:?}",
            node_id, frame_response.response.rect
        );

        frame_response.response
    }

    /// Get node styling based on type and state
    fn get_node_styling(node: &SceneNode) -> (egui::Color32, egui::Color32, f32) {
        let base_color = match &node.node_type {
            CloudFormationNodeType::Resource { aws_service, .. } => {
                get_aws_service_color(aws_service)
            }
            CloudFormationNodeType::Parameter { .. } => egui::Color32::from_rgb(100, 150, 200),
            CloudFormationNodeType::Output { .. } => egui::Color32::from_rgb(150, 200, 100),
            CloudFormationNodeType::Condition { .. } => egui::Color32::from_rgb(200, 150, 100),
        };

        let (background_color, border_color, border_width) = if node.selected {
            // Selected node: bright background, thick border
            (
                egui::Color32::from_rgba_premultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    60,
                ),
                base_color,
                3.0,
            )
        } else if node.hovered {
            // Hovered node: medium background, medium border
            (
                egui::Color32::from_rgba_premultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    40,
                ),
                base_color,
                2.5,
            )
        } else {
            // Normal node: subtle background, thin border
            (
                egui::Color32::from_rgba_premultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    20,
                ),
                base_color,
                2.0,
            )
        };

        (background_color, border_color, border_width)
    }

    /// Get title color based on node state
    fn get_title_color_for_node(node: &SceneNode) -> egui::Color32 {
        if node.selected {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_gray(240)
        }
    }

    /// Get subtitle color based on node state
    fn get_subtitle_color_for_node(node: &SceneNode) -> egui::Color32 {
        if node.selected {
            egui::Color32::from_gray(220)
        } else {
            egui::Color32::from_gray(180)
        }
    }

    /// Get detail text color based on node state
    fn get_detail_color_for_node(node: &SceneNode) -> egui::Color32 {
        if node.selected {
            egui::Color32::from_gray(200)
        } else {
            egui::Color32::from_gray(140)
        }
    }

    /// Render colored service badge
    fn render_colored_service_badge(ui: &mut egui::Ui, service: &str) {
        let badge_color = Self::get_service_badge_color(service);

        egui::Frame::new()
            .fill(badge_color)
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(4, 2))
            .show(ui, |ui| {
                ui.colored_label(
                    egui::Color32::WHITE,
                    egui::RichText::new(service).size(9.0).strong(),
                );
            });
    }

    /// Render colored type badge
    fn render_colored_type_badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
        egui::Frame::new()
            .fill(color)
            .corner_radius(3.0)
            .inner_margin(egui::Margin::symmetric(3, 1))
            .show(ui, |ui| {
                ui.colored_label(
                    egui::Color32::WHITE,
                    egui::RichText::new(text).size(8.0).strong(),
                );
            });
    }

    /// Get badge color for AWS service using professional color scheme
    fn get_service_badge_color(service: &str) -> egui::Color32 {
        // Use the same professional colors as the main service color mapping
        get_aws_service_color(service)
    }

    /// Format resource type for display (remove AWS:: prefix and shorten)
    fn format_resource_type(resource_type: &str) -> String {
        resource_type
            .strip_prefix("AWS::")
            .unwrap_or(resource_type)
            .replace("::", " → ")
    }

    /// Draw info overlay within the scene showing Scene container information
    fn draw_info_overlay_in_scene(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let scene_info = format!(
            "Scene: {:.0}x{:.0} | Nodes: {}",
            rect.width(),
            rect.height(),
            self.nodes.len()
        );

        let text_pos = Pos2::new(rect.max.x - 200.0, rect.max.y - 30.0);
        let text_color = egui::Color32::LIGHT_GRAY;

        ui.painter().text(
            text_pos,
            egui::Align2::RIGHT_BOTTOM,
            scene_info,
            egui::FontId::monospace(12.0),
            text_color,
        );
    }

    /// Create the scene graph from a project
    pub fn create_from_project(&mut self, project: &Project) {
        let resources = project.get_resources();
        info!(
            "🎨 CREATE_FROM_PROJECT: Creating scene graph from project: {} with {} CloudFormation resources",
            project.name,
            resources.len()
        );
        info!(
            "📂 PROJECT_PATH: Loading from {}",
            project
                .local_folder
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<no folder>".to_string())
        );

        self.nodes.clear();
        self.selected_nodes.clear();

        // Log each resource we're processing
        for (i, resource) in resources.iter().enumerate() {
            info!(
                "Resource {}: {} ({})",
                i + 1,
                resource.resource_id,
                resource.resource_type
            );
        }

        // Create nodes with proper position loading hierarchy
        let mut node_positions = HashMap::new();
        let mut default_x = 50.0;
        let mut default_y = 50.0;
        let node_width = 200.0; // Width of our nodes (180 + margin)
        let node_height = 140.0; // Height of our nodes (120 + margin)
        let max_x = 1200.0; // Stay within reasonable screen bounds (wider for new nodes)
        let max_y = 700.0; // Stay within reasonable screen bounds

        // Build DAG from project to get positions
        let dag = project.build_dag_from_resources();
        info!(
            "📊 POSITION_LOAD: Built DAG with {} saved positions",
            dag.get_node_positions().len()
        );
        for (res_id, (x, y)) in dag.get_node_positions() {
            info!(
                "📊 POSITION_LOAD: DAG has {} at ({:.1}, {:.1})",
                res_id, x, y
            );
        }

        for resource in &resources {
            // FIXED Position loading hierarchy: Template metadata > DAG positions > Default grid position
            // This ensures that saved positions from template always take precedence over DAG defaults
            let position = if let Some(metadata_pos) =
                Self::load_position_from_template_metadata(project, &resource.resource_id)
            {
                info!(
                    "🎯 POSITION_LOAD: Loading {} from template metadata: ({:.1}, {:.1})",
                    resource.resource_id, metadata_pos.x, metadata_pos.y
                );
                metadata_pos
            } else if let Some((dag_x, dag_y)) = dag.get_node_positions().get(&resource.resource_id)
            {
                info!(
                    "📊 POSITION_LOAD: Loading {} from DAG: ({:.1}, {:.1})",
                    resource.resource_id, dag_x, dag_y
                );
                Pos2::new(*dag_x, *dag_y)
            } else {
                info!(
                    "⚪ POSITION_LOAD: No DAG position for {}, using default grid: ({:.1}, {:.1})",
                    resource.resource_id, default_x, default_y
                );
                Pos2::new(default_x, default_y)
            };

            let node = SceneNode::from_resource(resource, position);

            info!(
                "✅ POSITION_LOAD: Created node for {} at final position ({:.1}, {:.1})",
                resource.resource_id, position.x, position.y
            );

            self.nodes.insert(resource.resource_id.clone(), node);
            node_positions.insert(resource.resource_id.clone(), position);

            // Only advance default grid position for resources that used the default
            // This ensures saved positions don't affect the grid layout for new resources
            if position.x == default_x && position.y == default_y {
                default_x += node_width;
                if default_x > max_x {
                    default_x = 50.0;
                    default_y += node_height;

                    if default_y > max_y {
                        info!("Many resources detected - nodes may extend beyond initial view. Use Auto Layout button to reorganize.");
                    }
                }
            }
        }

        // Add dependencies from built DAG
        info!("Processing dependencies from built DAG");
        for (resource_id, node) in &mut self.nodes {
            let deps = dag.get_dependencies(resource_id);
            if !deps.is_empty() {
                info!("Resource {} depends on: {:?}", resource_id, deps);
            }
            node.dependencies = deps;
        }

        self.layout_dirty = false;

        info!("Scene graph created with {} nodes total", self.nodes.len());

        // CRITICAL FIX: Auto-fit viewport to show all positioned nodes
        info!("🎯 VIEWPORT_FIX: Auto-fitting scene to show all positioned nodes");
        self.fit_to_view();
    }

    /// Apply automatic layout to the nodes - spreads them across full screen without overlapping
    fn apply_auto_layout(&mut self) {
        info!("🚫 AUTO_LAYOUT: Auto layout disabled temporarily to preserve saved positions");
        return; // TEMPORARILY DISABLED to preserve saved positions

        #[allow(unreachable_code)]
        {
            if self.nodes.is_empty() {
                return;
            }

            // Calculate optimal grid dimensions to fill the screen
            let node_count = self.nodes.len();
            let cols = (node_count as f32).sqrt().ceil() as usize;
            let rows = (node_count as f32 / cols as f32).ceil() as usize;

            // Use large canvas area for spreading nodes
            let canvas_width = 2000.0;
            let canvas_height = 1500.0;

            // Calculate spacing to use most of the screen
            let node_width = 180.0; // Width for our wider nodes (168 + margin)
            let node_height = 80.0; // Height for our nodes (60 + margin)

            let spacing_x = if cols > 1 {
                (canvas_width - node_width) / (cols - 1) as f32
            } else {
                canvas_width / 2.0
            };

            let spacing_y = if rows > 1 {
                (canvas_height - node_height) / (rows - 1) as f32
            } else {
                canvas_height / 2.0
            };

            // Ensure minimum spacing to prevent overlap
            let min_spacing = 200.0; // Wider spacing for wider nodes
            let final_spacing_x = spacing_x.max(min_spacing);
            let final_spacing_y = spacing_y.max(100.0); // Adjusted for new node height

            // Position nodes in grid layout
            let mut col = 0;
            let mut row = 0;

            for node in self.nodes.values_mut() {
                let x = 50.0 + col as f32 * final_spacing_x;
                let y = 50.0 + row as f32 * final_spacing_y;

                node.position = Pos2::new(x, y);

                col += 1;
                if col >= cols {
                    col = 0;
                    row += 1;
                }
            }

            info!(
                "Auto layout applied to {} nodes in {}x{} grid with spacing ({:.0}, {:.0})",
                self.nodes.len(),
                cols,
                rows,
                final_spacing_x,
                final_spacing_y
            );
        } // End of unreachable code block
    }

    /// Fit all nodes to the view using Scene container
    fn fit_to_view(&mut self) {
        info!("Fitting scene graph to view using Scene container");

        if self.nodes.is_empty() {
            self.scene_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
            return;
        }

        // Calculate bounding box of all nodes
        let mut min_pos = Pos2::new(f32::INFINITY, f32::INFINITY);
        let mut max_pos = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);

        for node in self.nodes.values() {
            let node_min = node.position;
            let node_max = node.position + node.size;

            min_pos.x = min_pos.x.min(node_min.x);
            min_pos.y = min_pos.y.min(node_min.y);
            max_pos.x = max_pos.x.max(node_max.x);
            max_pos.y = max_pos.y.max(node_max.y);
        }

        // Add some padding
        let padding = 50.0;
        min_pos -= Vec2::splat(padding);
        max_pos += Vec2::splat(padding);

        // Update scene rectangle to encompass all nodes with padding
        self.scene_rect = Rect::from_two_pos(min_pos, max_pos);

        info!("Fit to view: scene_rect={:?}", self.scene_rect);
    }

    /// Center the view on all nodes using Scene container
    fn center_view(&mut self) {
        info!("Centering scene graph view using Scene container");

        if self.nodes.is_empty() {
            self.scene_rect = Rect::from_center_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
            return;
        }

        // Calculate center of all nodes
        let mut center = Vec2::ZERO;
        for node in self.nodes.values() {
            center += node.position.to_vec2();
        }
        center /= self.nodes.len() as f32;

        // Center the scene rectangle on this point
        let current_size = self.scene_rect.size();
        self.scene_rect = Rect::from_center_size(center.to_pos2(), current_size);

        info!("Centered view on ({:.1}, {:.1})", center.x, center.y);
    }

    /// Toggle window visibility
    pub fn toggle(&mut self) {
        self.show = !self.show;
    }

    /// Set window visibility
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
        // Reset font scaling when closing
        if !show {
            // Note: We can't call reset here as we don't have access to ctx
            // This should be called from the parent when the window closes
        }
    }

    /// Sync node coordinates directly to CloudFormation template metadata
    /// Only updates position data, preserves all existing metadata
    pub fn sync_coordinates_to_project(&self, project: &mut crate::app::projects::Project) {
        let mut changed_count = 0;

        // Update positions directly in CloudFormation template metadata
        if let Some(template) = &mut project.cfn_template {
            for (node_id, node) in &self.nodes {
                if let Some(template_resource) = template.resources.get_mut(node_id) {
                    // Create or update the metadata
                    if template_resource.metadata.is_none() {
                        template_resource.metadata =
                            Some(serde_json::Value::Object(serde_json::Map::new()));
                    }

                    if let Some(metadata_obj) =
                        template_resource.metadata.as_mut().unwrap().as_object_mut()
                    {
                        // Create AwsDashScene metadata if it doesn't exist
                        if !metadata_obj.contains_key(SCENE_METADATA_KEY) {
                            metadata_obj
                                .insert(SCENE_METADATA_KEY.to_string(), serde_json::json!({}));
                        }

                        // Update position
                        if let Some(scene_metadata) = metadata_obj.get_mut(SCENE_METADATA_KEY) {
                            if let Some(scene_obj) = scene_metadata.as_object_mut() {
                                let position_data = serde_json::json!({
                                    "x": node.position.x,
                                    "y": node.position.y
                                });

                                scene_obj.insert(POSITION_KEY.to_string(), position_data);
                                changed_count += 1;

                                info!(
                                    "🔄 POSITION_SYNC: Updated {} position in template metadata: ({:.1}, {:.1})",
                                    node_id, node.position.x, node.position.y
                                );
                            }
                        }
                    }
                }
            }
        }

        if changed_count > 0 {
            info!(
                "✅ POSITION_SYNC: {} node positions changed, syncing to template",
                changed_count
            );

            // Save project to persist position changes to CloudFormation template
            info!(
                "💾 POSITION_SYNC: Saving {} position changes to CloudFormation template",
                changed_count
            );
            if let Err(e) = project.save_all_resources() {
                error!(
                    "❌ POSITION_SYNC: Failed to save resources after coordinate sync: {}",
                    e
                );
            } else {
                info!("✅ POSITION_SYNC: {} position changes successfully persisted to CloudFormation template", changed_count);
                info!("✅ FILE_SAVED: Position changes written to disk");
            }
        }
    }
}

impl Default for CloudFormationSceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// AWS service color mapping using professional, tested color combinations
/// Based on enterprise dashboard design principles and accessibility guidelines
pub fn get_aws_service_color(service: &str) -> Color32 {
    match service {
        // Compute Services - Professional Orange (AWS Official Orange family)
        "EC2" => Color32::from_rgb(255, 153, 0), // AWS Orange - vibrant and energetic
        "Lambda" => Color32::from_rgb(230, 126, 34), // Warm Orange - reliable and modern
        "ECS" => Color32::from_rgb(211, 84, 0),  // Deep Orange - professional strength
        "EKS" => Color32::from_rgb(230, 115, 0), // Rich Orange - enterprise ready
        "Batch" => Color32::from_rgb(184, 94, 0), // Burnt Orange - sophisticated

        // Storage Services - Professional Blue-Green (Trust and reliability)
        "S3" => Color32::from_rgb(46, 125, 50), // Forest Green - stability and growth
        "EFS" => Color32::from_rgb(67, 160, 71), // Success Green - dependable storage
        "EBS" => Color32::from_rgb(27, 94, 32), // Deep Green - enterprise reliability
        "FSx" => Color32::from_rgb(76, 175, 80), // Material Green - modern storage

        // Database Services - Professional Blue (AWS Navy family)
        "RDS" => Color32::from_rgb(37, 47, 62), // AWS Navy - authoritative and trustworthy
        "DynamoDB" => Color32::from_rgb(63, 81, 181), // Indigo - professional database
        "ElastiCache" => Color32::from_rgb(33, 150, 243), // Material Blue - high performance
        "DocumentDB" => Color32::from_rgb(25, 118, 210), // Deep Blue - enterprise grade
        "Neptune" => Color32::from_rgb(48, 63, 159), // Rich Indigo - sophisticated

        // Networking Services - Professional Purple (Connectivity and flow)
        "VPC" => Color32::from_rgb(103, 58, 183), // Deep Purple - foundational network
        "CloudFront" => Color32::from_rgb(156, 39, 176), // Magenta - global distribution
        "Route53" => Color32::from_rgb(142, 36, 170), // Rich Purple - DNS authority
        "ELB" | "ElasticLoadBalancing" => Color32::from_rgb(126, 87, 194), // Balanced Purple
        "ALB" => Color32::from_rgb(149, 117, 205), // Light Purple - application layer
        "NLB" => Color32::from_rgb(94, 53, 177),  // Strong Purple - network layer
        "APIGateway" => Color32::from_rgb(171, 71, 188), // Bright Purple - API management

        // Security Services - Professional Red (Security and protection)
        "IAM" => Color32::from_rgb(198, 40, 40), // Strong Red - identity security
        "KMS" => Color32::from_rgb(183, 28, 28), // Deep Red - encryption strength
        "SecretsManager" => Color32::from_rgb(211, 47, 47), // Reliable Red - secret protection
        "SSM" => Color32::from_rgb(229, 57, 53), // Bright Red - system management
        "Cognito" => Color32::from_rgb(244, 67, 54), // Material Red - user authentication

        // Analytics Services - Professional Amber (Insight and intelligence)
        "Kinesis" => Color32::from_rgb(255, 160, 0), // Rich Amber - real-time processing
        "EMR" => Color32::from_rgb(255, 143, 0),     // Deep Amber - big data processing
        "Athena" => Color32::from_rgb(255, 193, 7),  // Gold - query intelligence
        "Glue" => Color32::from_rgb(255, 179, 0),    // Warm Gold - data integration
        "QuickSight" => Color32::from_rgb(255, 167, 38), // Professional Amber - visualization

        // Application Integration - Professional Teal (Integration and flow)
        "SNS" => Color32::from_rgb(0, 150, 136), // Teal - notification service
        "SQS" => Color32::from_rgb(0, 121, 107), // Deep Teal - message queuing
        "EventBridge" => Color32::from_rgb(0, 172, 149), // Bright Teal - event management
        "AppSync" => Color32::from_rgb(38, 166, 154), // Material Teal - app integration

        // Developer Tools - Professional Blue-Grey (Development and deployment)
        "CodeCommit" => Color32::from_rgb(84, 110, 122), // Blue Grey - source control
        "CodeBuild" => Color32::from_rgb(96, 125, 139),  // Medium Blue Grey - build automation
        "CodeDeploy" => Color32::from_rgb(69, 90, 100),  // Dark Blue Grey - deployment
        "CodePipeline" => Color32::from_rgb(120, 144, 156), // Light Blue Grey - CI/CD

        // Machine Learning - Professional Indigo (AI and intelligence)
        "SageMaker" => Color32::from_rgb(92, 107, 192), // Soft Indigo - ML platform
        "Rekognition" => Color32::from_rgb(121, 85, 199), // Purple Indigo - image recognition
        "Comprehend" => Color32::from_rgb(159, 168, 218), // Light Indigo - text analysis

        // Monitoring - Professional Grey (Observability and oversight)
        "CloudWatch" => Color32::from_rgb(97, 97, 97), // Professional Grey - monitoring
        "CloudTrail" => Color32::from_rgb(117, 117, 117), // Medium Grey - auditing
        "XRay" => Color32::from_rgb(158, 158, 158),    // Light Grey - tracing

        // Default for unknown services - Neutral professional grey
        _ => Color32::from_rgb(120, 144, 156), // Professional Blue-Grey
    }
}

impl FocusableWindow for CloudFormationSceneGraph {
    type ShowParams = SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "cloudformation_scene"
    }

    fn window_title(&self) -> String {
        "CloudFormation Graph".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        _params: Self::ShowParams,
        _bring_to_front: bool,
    ) {
        // The CloudFormation Scene Graph uses the full screen mode and egui Scene container,
        // so it doesn't use a traditional window with Order::Foreground.
        // The bring_to_front parameter is not applicable for this type of window.
        // We'll just delegate to the existing show method.
        self.show(ctx);
    }
}
