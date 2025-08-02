use crate::app::cfn_resources::PropertyDefinition;
use crate::app::projects::CloudFormationResource;
use eframe::egui::{self, Button, Color32, ComboBox, Grid, ScrollArea, Window};

/// Represents the reference picker window for selecting CloudFormation references
/// Supports intrinsic functions like !Ref, !GetAtt, !Sub, etc.
pub struct ReferencePickerWindow {
    /// Whether to show the window
    pub show: bool,

    /// The property name for which we're picking a reference
    pub property_name: String,

    /// The property definition with constraints
    pub property_definition: Option<PropertyDefinition>,

    /// Available resources that can be referenced
    pub available_resources: Vec<CloudFormationResource>,

    /// The currently selected intrinsic function
    selected_function: IntrinsicFunction,

    /// The currently selected resource ID for !Ref
    selected_resource_id: String,

    /// The currently selected attribute for !GetAtt
    selected_attribute: String,

    /// Custom expression for !Sub
    sub_expression: String,

    /// Error message if any
    pub error_message: Option<String>,

    /// Callback when a reference is selected
    pub on_reference_selected: Option<Box<dyn FnMut(String)>>,

    /// The last selected reference (for external access)
    pub last_selected_reference: Option<String>,

    /// Available attributes for the selected resource (for !GetAtt)
    available_attributes: Vec<String>,

    /// Whether the current selection is valid
    is_valid_selection: bool,
}

/// Supported CloudFormation intrinsic functions
#[derive(Debug, Clone, PartialEq)]
enum IntrinsicFunction {
    Ref,
    GetAtt,
    Sub,
    Join,
    Select,
    Split,
    Base64,
    FindInMap,
}

impl IntrinsicFunction {
    fn display_name(&self) -> &'static str {
        match self {
            IntrinsicFunction::Ref => "!Ref - Reference a resource",
            IntrinsicFunction::GetAtt => "!GetAtt - Get resource attribute",
            IntrinsicFunction::Sub => "!Sub - String substitution",
            IntrinsicFunction::Join => "!Join - Join strings",
            IntrinsicFunction::Select => "!Select - Select from list",
            IntrinsicFunction::Split => "!Split - Split string",
            IntrinsicFunction::Base64 => "!Base64 - Base64 encode",
            IntrinsicFunction::FindInMap => "!FindInMap - Find mapping value",
        }
    }
}

impl Default for ReferencePickerWindow {
    fn default() -> Self {
        Self {
            show: false,
            property_name: String::new(),
            property_definition: None,
            available_resources: Vec::new(),
            selected_function: IntrinsicFunction::Ref,
            selected_resource_id: String::new(),
            selected_attribute: String::new(),
            sub_expression: String::new(),
            error_message: None,
            on_reference_selected: None,
            last_selected_reference: None,
            available_attributes: Vec::new(),
            is_valid_selection: false,
        }
    }
}

impl ReferencePickerWindow {
    /// Create a new reference picker window
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the window for selecting a reference
    pub fn open(
        &mut self,
        property_name: String,
        property_definition: Option<PropertyDefinition>,
        available_resources: Vec<CloudFormationResource>,
        on_reference_selected: impl FnMut(String) + 'static,
    ) {
        self.property_name = property_name;
        self.property_definition = property_definition;
        self.available_resources = available_resources;
        self.selected_function = IntrinsicFunction::Ref;
        self.selected_resource_id = String::new();
        self.selected_attribute = String::new();
        self.sub_expression = String::new();
        self.error_message = None;
        self.on_reference_selected = Some(Box::new(on_reference_selected));
        self.available_attributes = Vec::new();
        self.is_valid_selection = false;
        self.show = true;
    }

    /// Show the reference picker window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut reference_selected = false;

        if !self.show {
            return reference_selected;
        }

        let title = format!("Pick Reference: {}", self.property_name);

        // Get screen dimensions for window sizing
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.5;
        let max_height = screen_rect.height() * 0.7;

        let mut close_window = false;
        let mut show_window = self.show;

        Window::new(title)
            .open(&mut show_window)
            .min_width(450.0)
            .min_height(400.0)
            .max_width(max_width)
            .max_height(max_height)
            .resizable(true)
            .default_pos(screen_rect.center())
            .show(ctx, |ui| {
                // Error message display
                if let Some(error) = &self.error_message {
                    ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                    ui.separator();
                }

                // Property information
                ui.horizontal(|ui| {
                    ui.label("Property:");
                    ui.strong(&self.property_name);
                });

                if let Some(def) = &self.property_definition {
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        if let Some(primitive) = &def.primitive_type {
                            ui.label(primitive);
                        } else if let Some(type_name) = &def.type_name {
                            ui.label(type_name);
                        } else {
                            ui.label("Unknown");
                        }

                        if def.required {
                            ui.colored_label(Color32::RED, "*Required");
                        }
                    });
                }

                ui.separator();

                // Function selection
                ui.horizontal(|ui| {
                    ui.label("Intrinsic Function:");
                });

                ComboBox::from_id_salt("intrinsic_function_selector")
                    .selected_text(self.selected_function.display_name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::Ref,
                            IntrinsicFunction::Ref.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::GetAtt,
                            IntrinsicFunction::GetAtt.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::Sub,
                            IntrinsicFunction::Sub.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::Join,
                            IntrinsicFunction::Join.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::Select,
                            IntrinsicFunction::Select.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::Split,
                            IntrinsicFunction::Split.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::Base64,
                            IntrinsicFunction::Base64.display_name(),
                        );
                        ui.selectable_value(
                            &mut self.selected_function,
                            IntrinsicFunction::FindInMap,
                            IntrinsicFunction::FindInMap.display_name(),
                        );
                    });

                ui.separator();

                // Function-specific configuration
                ScrollArea::vertical()
                    .max_height(max_height - 200.0)
                    .show(ui, |ui| {
                        self.show_function_configuration(ui);
                    });

                ui.separator();

                // Preview of the generated reference
                self.show_reference_preview(ui);

                ui.separator();

                // Validation and buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close_window = true;
                    }

                    // Select button - enabled only if valid selection
                    let select_enabled = self.is_valid_selection;
                    let select_button =
                        ui.add_enabled(select_enabled, Button::new("Select Reference"));

                    if select_button.clicked() {
                        if let Some(reference) = self.generate_reference() {
                            // Store the selected reference for external access
                            self.last_selected_reference = Some(reference.clone());

                            if let Some(on_selected) = &mut self.on_reference_selected {
                                on_selected(reference);
                            }
                            reference_selected = true;
                            close_window = true;
                        }
                    }

                    // Validation info
                    if !self.is_valid_selection {
                        ui.colored_label(Color32::RED, "Complete configuration to select");
                    }
                });
            });

        // Update show state
        self.show = show_window;

        // Close window if requested
        if close_window {
            self.show = false;
        }

        // Update validation status after UI changes
        self.update_validation_status();

        reference_selected
    }

    /// Get and clear the last selected reference
    pub fn take_selected_reference(&mut self) -> Option<String> {
        self.last_selected_reference.take()
    }

    /// Show the configuration UI for the selected intrinsic function
    fn show_function_configuration(&mut self, ui: &mut egui::Ui) {
        match self.selected_function {
            IntrinsicFunction::Ref => self.show_ref_configuration(ui),
            IntrinsicFunction::GetAtt => self.show_getatt_configuration(ui),
            IntrinsicFunction::Sub => self.show_sub_configuration(ui),
            IntrinsicFunction::Join => self.show_join_configuration(ui),
            IntrinsicFunction::Select => self.show_select_configuration(ui),
            IntrinsicFunction::Split => self.show_split_configuration(ui),
            IntrinsicFunction::Base64 => self.show_base64_configuration(ui),
            IntrinsicFunction::FindInMap => self.show_findinmap_configuration(ui),
        }
    }

    /// Show configuration for !Ref function
    fn show_ref_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("Select Resource to Reference:");

        ComboBox::from_id_salt("ref_resource_selector")
            .selected_text(if self.selected_resource_id.is_empty() {
                "Select resource..."
            } else {
                &self.selected_resource_id
            })
            .show_ui(ui, |ui| {
                for resource in &self.available_resources {
                    ui.selectable_value(
                        &mut self.selected_resource_id,
                        resource.resource_id.clone(),
                        &resource.resource_id,
                    );
                }
            });

        // Show resource information if selected
        if !self.selected_resource_id.is_empty() {
            if let Some(resource) = self
                .available_resources
                .iter()
                .find(|r| r.resource_id == self.selected_resource_id)
            {
                ui.separator();
                Grid::new("ref_resource_info")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Resource Type:");
                        ui.label(&resource.resource_type);
                        ui.end_row();

                        // Note: Resource description not available in current CloudFormationResource struct
                    });
            }
        }
    }

    /// Show configuration for !GetAtt function
    fn show_getatt_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("Select Resource:");

        ComboBox::from_id_salt("getatt_resource_selector")
            .selected_text(if self.selected_resource_id.is_empty() {
                "Select resource..."
            } else {
                &self.selected_resource_id
            })
            .show_ui(ui, |ui| {
                let old_selection = self.selected_resource_id.clone();
                let mut resource_type_to_update = None;

                for resource in &self.available_resources {
                    ui.selectable_value(
                        &mut self.selected_resource_id,
                        resource.resource_id.clone(),
                        &resource.resource_id,
                    );

                    // Check if resource changed
                    if self.selected_resource_id != old_selection
                        && self.selected_resource_id == resource.resource_id
                    {
                        resource_type_to_update = Some(resource.resource_type.clone());
                    }
                }

                // Update available attributes outside the loop to avoid borrow checker issues
                if let Some(resource_type) = resource_type_to_update {
                    self.update_available_attributes(&resource_type);
                    self.selected_attribute = String::new();
                }
            });

        // Show attribute selection if resource is selected
        if !self.selected_resource_id.is_empty() && !self.available_attributes.is_empty() {
            ui.separator();
            ui.label("Select Attribute:");

            ComboBox::from_id_salt("getatt_attribute_selector")
                .selected_text(if self.selected_attribute.is_empty() {
                    "Select attribute..."
                } else {
                    &self.selected_attribute
                })
                .show_ui(ui, |ui| {
                    for attribute in &self.available_attributes {
                        ui.selectable_value(
                            &mut self.selected_attribute,
                            attribute.clone(),
                            attribute,
                        );
                    }
                });
        }
    }

    /// Show configuration for !Sub function
    fn show_sub_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("Substitution Expression:");
        ui.text_edit_multiline(&mut self.sub_expression);

        ui.small("Use ${ResourceLogicalId} to reference resources");
        ui.small("Use ${AWS::Region}, ${AWS::AccountId}, etc. for pseudo parameters");

        // Show available resources for reference
        if !self.available_resources.is_empty() {
            ui.separator();
            ui.label("Available Resources:");
            ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                for resource in &self.available_resources {
                    ui.horizontal(|ui| {
                        if ui.small_button("Insert").clicked() {
                            let insertion = format!("${{{}}}", resource.resource_id);
                            self.sub_expression.push_str(&insertion);
                        }
                        ui.small(&resource.resource_id);
                        ui.small(format!("({})", resource.resource_type));
                    });
                }
            });
        }
    }

    /// Show configuration for !Join function
    fn show_join_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("!Join function configuration:");
        ui.small("This is a simplified configuration. For complex !Join expressions, use the JSON editor.");

        ui.horizontal(|ui| {
            ui.label("Delimiter:");
            ui.text_edit_singleline(&mut self.sub_expression);
        });

        ui.label("Note: Use the JSON editor for full !Join configuration with value arrays.");
    }

    /// Show configuration for !Select function
    fn show_select_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("!Select function configuration:");
        ui.small("Use the JSON editor for complete !Select configuration.");
        ui.small("Format: !Select [index, [value1, value2, value3]]");
    }

    /// Show configuration for !Split function
    fn show_split_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("!Split function configuration:");
        ui.small("Use the JSON editor for complete !Split configuration.");
        ui.small("Format: !Split [delimiter, source_string]");
    }

    /// Show configuration for !Base64 function
    fn show_base64_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("Value to encode:");
        ui.text_edit_multiline(&mut self.sub_expression);
        ui.small("This value will be Base64 encoded by CloudFormation");
    }

    /// Show configuration for !FindInMap function
    fn show_findinmap_configuration(&mut self, ui: &mut egui::Ui) {
        ui.label("!FindInMap function configuration:");
        ui.small("Use the JSON editor for complete !FindInMap configuration.");
        ui.small("Format: !FindInMap [MapName, TopLevelKey, SecondLevelKey]");
    }

    /// Show a preview of the reference that will be generated
    fn show_reference_preview(&self, ui: &mut egui::Ui) {
        ui.label("Generated Reference Preview:");

        if let Some(reference) = self.generate_reference() {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::GREEN, "✅");
                ui.code(&reference);
            });
        } else {
            ui.colored_label(Color32::GRAY, "Complete configuration to see preview");
        }
    }

    /// Generate the CloudFormation reference string based on current selection
    fn generate_reference(&self) -> Option<String> {
        match self.selected_function {
            IntrinsicFunction::Ref => {
                if !self.selected_resource_id.is_empty() {
                    Some(format!("{{\"Ref\": \"{}\"}}", self.selected_resource_id))
                } else {
                    None
                }
            }
            IntrinsicFunction::GetAtt => {
                if !self.selected_resource_id.is_empty() && !self.selected_attribute.is_empty() {
                    Some(format!(
                        "{{\"Fn::GetAtt\": [\"{}\", \"{}\"]}}",
                        self.selected_resource_id, self.selected_attribute
                    ))
                } else {
                    None
                }
            }
            IntrinsicFunction::Sub => {
                if !self.sub_expression.trim().is_empty() {
                    Some(format!("{{\"Fn::Sub\": \"{}\"}}", self.sub_expression))
                } else {
                    None
                }
            }
            IntrinsicFunction::Base64 => {
                if !self.sub_expression.trim().is_empty() {
                    Some(format!("{{\"Fn::Base64\": \"{}\"}}", self.sub_expression))
                } else {
                    None
                }
            }
            // For complex functions, return a placeholder that indicates JSON editor is needed
            IntrinsicFunction::Join
            | IntrinsicFunction::Select
            | IntrinsicFunction::Split
            | IntrinsicFunction::FindInMap => Some(format!(
                "{{\"Fn::{}\": \"Configure in JSON editor\"}}",
                match self.selected_function {
                    IntrinsicFunction::Join => "Join",
                    IntrinsicFunction::Select => "Select",
                    IntrinsicFunction::Split => "Split",
                    IntrinsicFunction::FindInMap => "FindInMap",
                    _ => unreachable!(),
                }
            )),
        }
    }

    /// Update the validation status based on current configuration
    fn update_validation_status(&mut self) {
        self.is_valid_selection = match self.selected_function {
            IntrinsicFunction::Ref => !self.selected_resource_id.is_empty(),
            IntrinsicFunction::GetAtt => {
                !self.selected_resource_id.is_empty() && !self.selected_attribute.is_empty()
            }
            IntrinsicFunction::Sub | IntrinsicFunction::Base64 => {
                !self.sub_expression.trim().is_empty()
            }
            // Complex functions are always considered valid for JSON editor fallback
            IntrinsicFunction::Join
            | IntrinsicFunction::Select
            | IntrinsicFunction::Split
            | IntrinsicFunction::FindInMap => true,
        };

        // Clear error message if selection becomes valid
        if self.is_valid_selection {
            self.error_message = None;
        }
    }

    /// Update available attributes based on the selected resource type
    fn update_available_attributes(&mut self, resource_type: &str) {
        // This is a simplified attribute mapping. In a real implementation,
        // you would load this from CloudFormation resource type specifications
        self.available_attributes = match resource_type {
            "AWS::S3::Bucket" => vec![
                "Arn".to_string(),
                "DomainName".to_string(),
                "DualStackDomainName".to_string(),
                "RegionalDomainName".to_string(),
                "WebsiteURL".to_string(),
            ],
            "AWS::EC2::Instance" => vec![
                "AvailabilityZone".to_string(),
                "PrivateDnsName".to_string(),
                "PrivateIp".to_string(),
                "PublicDnsName".to_string(),
                "PublicIp".to_string(),
            ],
            "AWS::RDS::DBInstance" => vec![
                "Endpoint.Address".to_string(),
                "Endpoint.Port".to_string(),
                "DBInstanceEndpointAddress".to_string(),
                "DBInstanceEndpointPort".to_string(),
            ],
            "AWS::Lambda::Function" => vec!["Arn".to_string(), "Version".to_string()],
            "AWS::IAM::Role" => vec!["Arn".to_string(), "RoleId".to_string()],
            "AWS::EC2::VPC" => vec![
                "CidrBlock".to_string(),
                "CidrBlockAssociations".to_string(),
                "DefaultNetworkAcl".to_string(),
                "DefaultSecurityGroup".to_string(),
                "Ipv6CidrBlocks".to_string(),
            ],
            "AWS::EC2::Subnet" => vec![
                "AvailabilityZone".to_string(),
                "Ipv6CidrBlocks".to_string(),
                "NetworkAclAssociationId".to_string(),
                "VpcId".to_string(),
            ],
            _ => {
                // Generic attributes that are commonly available
                vec!["Arn".to_string()]
            }
        };
    }
}
