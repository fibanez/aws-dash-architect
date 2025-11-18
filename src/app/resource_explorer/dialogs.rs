use super::state::*;
use crate::app::aws_identity::AwsAccount;
use egui::{Context, Window};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::collections::HashMap;

#[derive(Default)]
pub struct FuzzySearchDialog {
    pub search_term: String,
    pub selected_index: Option<usize>,
    pub selected_resource_types: HashMap<String, bool>, // Track checkbox selections by resource_type
    pub selected_accounts: HashMap<String, bool>,       // Track checkbox selections by account_id
    pub selected_regions: HashMap<String, bool>,        // Track checkbox selections by region_code
    matcher: SkimMatcherV2,
}

impl FuzzySearchDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all selected resource types
    pub fn clear_all_resource_types(&mut self) {
        self.selected_resource_types.clear();
    }

    /// Get count of selected resource types
    pub fn selected_resource_types_count(&self) -> usize {
        self.selected_resource_types
            .values()
            .filter(|&&selected| selected)
            .count()
    }

    /// Get list of selected resource types
    pub fn get_selected_resource_types(
        &self,
        available_types: &[ResourceTypeSelection],
    ) -> Vec<ResourceTypeSelection> {
        available_types
            .iter()
            .filter(|resource_type| {
                self.selected_resource_types
                    .get(&resource_type.resource_type)
                    .copied()
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Clear all selected accounts
    pub fn clear_all_accounts(&mut self) {
        self.selected_accounts.clear();
    }

    /// Get count of selected accounts
    pub fn selected_accounts_count(&self) -> usize {
        self.selected_accounts
            .values()
            .filter(|&&selected| selected)
            .count()
    }

    /// Get list of selected accounts
    pub fn get_selected_accounts(
        &self,
        available_accounts: &[AwsAccount],
    ) -> Vec<AccountSelection> {
        available_accounts
            .iter()
            .filter(|account| {
                self.selected_accounts
                    .get(&account.account_id)
                    .copied()
                    .unwrap_or(false)
            })
            .map(|account| {
                AccountSelection::new(
                    account.account_id.clone(),
                    self.format_account_display_name(account),
                )
            })
            .collect()
    }

    /// Clear all selected regions
    pub fn clear_all_regions(&mut self) {
        self.selected_regions.clear();
    }

    /// Get count of selected regions
    pub fn selected_regions_count(&self) -> usize {
        self.selected_regions
            .values()
            .filter(|&&selected| selected)
            .count()
    }

    /// Get list of selected regions
    pub fn get_selected_regions(&self, available_regions: &[String]) -> Vec<RegionSelection> {
        available_regions
            .iter()
            .filter(|region| self.selected_regions.get(*region).copied().unwrap_or(false))
            .map(|region| {
                RegionSelection::new(region.clone(), self.format_region_display_name(region))
            })
            .collect()
    }

    pub fn show_account_dialog(
        &mut self,
        ctx: &Context,
        is_open: &mut bool,
        available_accounts: &[AwsAccount],
    ) -> Option<Vec<AccountSelection>> {
        if !*is_open {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        Window::new("Add AWS Accounts")
            .default_size([500.0, 400.0])
            .resizable(false)
            .collapsible(false)
            .open(is_open)
            .show(ctx, |ui| {
                ui.label("Search and select AWS accounts:");

                // Search input
                let search_response = ui.text_edit_singleline(&mut self.search_term);
                if search_response.changed() {
                    self.selected_index = None; // Reset navigation when search changes
                }

                ui.separator();

                // Selection info and clear button
                ui.horizontal(|ui| {
                    let selected_count = self.selected_accounts_count();
                    if selected_count > 0 {
                        ui.label(format!("{} accounts selected", selected_count));
                        ui.separator();
                        if ui.button("Clear All").clicked() {
                            self.clear_all_accounts();
                        }
                    } else {
                        ui.label("No accounts selected");
                    }
                });

                ui.separator();

                // Filter accounts based on search
                let filtered_accounts = self.filter_accounts(available_accounts);

                if filtered_accounts.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No matching accounts found");
                    });
                } else {
                    // Show filtered results in a scrollable area with checkboxes and grid layout
                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show(ui, |ui| {
                            egui::Grid::new("account_grid")
                                .num_columns(3)
                                .spacing([10.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for account in &filtered_accounts {
                                        // Checkbox column
                                        let mut is_checked = self
                                            .selected_accounts
                                            .get(&account.account_id)
                                            .copied()
                                            .unwrap_or(false);

                                        // Apply highlight color to checkbox for better visibility
                                        ui.scope(|ui| {
                                            let mut style = (*ui.ctx().style()).clone();
                                            style.visuals.selection.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            style.visuals.widgets.active.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            style.visuals.widgets.hovered.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            ui.ctx().set_style(style);

                                            if ui.checkbox(&mut is_checked, "").changed() {
                                                self.selected_accounts
                                                    .insert(account.account_id.clone(), is_checked);
                                            }
                                        });

                                        // Account name column (left-aligned, consistent width)
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_min_width(200.0);
                                                ui.label(&account.account_name);
                                            },
                                        );

                                        // Account ID column
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.label(&account.account_id);
                                            },
                                        );

                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let selected_count = self.selected_accounts_count();
                        let can_add = selected_count > 0;

                        let button_text = match selected_count.cmp(&1) {
                            std::cmp::Ordering::Greater => {
                                format!("Add {} Accounts", selected_count)
                            }
                            std::cmp::Ordering::Equal => "Add Account".to_string(),
                            std::cmp::Ordering::Less => "Add Accounts".to_string(),
                        };

                        if ui
                            .add_enabled(can_add, egui::Button::new(button_text))
                            .clicked()
                        {
                            let selected = self.get_selected_accounts(available_accounts);
                            if !selected.is_empty() {
                                result = Some(selected);
                                should_close = true;
                            }
                        }
                    });
                });

                // Handle keyboard navigation
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }

                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let selected = self.get_selected_accounts(available_accounts);
                    if !selected.is_empty() {
                        result = Some(selected);
                        should_close = true;
                    }
                }
            });

        if should_close {
            *is_open = false;
            self.reset();
        }

        result
    }

    pub fn show_region_dialog(
        &mut self,
        ctx: &Context,
        is_open: &mut bool,
        available_regions: &[String],
    ) -> Option<Vec<RegionSelection>> {
        if !*is_open {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        Window::new("Add AWS Regions")
            .default_size([500.0, 400.0])
            .resizable(false)
            .collapsible(false)
            .open(is_open)
            .show(ctx, |ui| {
                ui.label("Search and select AWS regions:");

                // Search input
                let search_response = ui.text_edit_singleline(&mut self.search_term);
                if search_response.changed() {
                    self.selected_index = None; // Reset navigation when search changes
                }

                ui.separator();

                // Selection info and clear button
                ui.horizontal(|ui| {
                    let selected_count = self.selected_regions_count();
                    if selected_count > 0 {
                        ui.label(format!("{} regions selected", selected_count));
                        ui.separator();
                        if ui.button("Clear All").clicked() {
                            self.clear_all_regions();
                        }
                    } else {
                        ui.label("No regions selected");
                    }
                });

                ui.separator();

                // Filter regions based on search
                let filtered_regions = self.filter_regions(available_regions);

                if filtered_regions.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No matching regions found");
                    });
                } else {
                    // Show filtered results in a scrollable area with checkboxes and grid layout
                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show(ui, |ui| {
                            egui::Grid::new("region_grid")
                                .num_columns(3)
                                .spacing([10.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for region in &filtered_regions {
                                        // Checkbox column
                                        let mut is_checked = self
                                            .selected_regions
                                            .get(&region.region_code)
                                            .copied()
                                            .unwrap_or(false);

                                        // Apply highlight color to checkbox for better visibility
                                        ui.scope(|ui| {
                                            let mut style = (*ui.ctx().style()).clone();
                                            style.visuals.selection.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            style.visuals.widgets.active.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            style.visuals.widgets.hovered.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            ui.ctx().set_style(style);

                                            if ui.checkbox(&mut is_checked, "").changed() {
                                                self.selected_regions
                                                    .insert(region.region_code.clone(), is_checked);
                                            }
                                        });

                                        // Region name column (left-aligned, consistent width)
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_min_width(150.0);
                                                ui.label(&region.display_name);
                                            },
                                        );

                                        // Region code column
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.label(&region.region_code);
                                            },
                                        );

                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let selected_count = self.selected_regions_count();
                        let can_add = selected_count > 0;

                        let button_text = match selected_count.cmp(&1) {
                            std::cmp::Ordering::Greater => {
                                format!("Add {} Regions", selected_count)
                            }
                            std::cmp::Ordering::Equal => "Add Region".to_string(),
                            std::cmp::Ordering::Less => "Add Regions".to_string(),
                        };

                        if ui
                            .add_enabled(can_add, egui::Button::new(button_text))
                            .clicked()
                        {
                            let selected = self.get_selected_regions(available_regions);
                            if !selected.is_empty() {
                                result = Some(selected);
                                should_close = true;
                            }
                        }
                    });
                });

                // Handle keyboard navigation
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }

                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let selected = self.get_selected_regions(available_regions);
                    if !selected.is_empty() {
                        result = Some(selected);
                        should_close = true;
                    }
                }
            });

        if should_close {
            *is_open = false;
            self.reset();
        }

        result
    }

    pub fn show_resource_type_dialog(
        &mut self,
        ctx: &Context,
        is_open: &mut bool,
        available_resource_types: &[ResourceTypeSelection],
    ) -> Option<Vec<ResourceTypeSelection>> {
        if !*is_open {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        Window::new("Add AWS Resource Types")
            .default_size([600.0, 500.0])
            .resizable(false)
            .collapsible(false)
            .open(is_open)
            .show(ctx, |ui| {
                ui.label("Search and select AWS resource types:");

                // Search input
                let search_response = ui.text_edit_singleline(&mut self.search_term);
                if search_response.changed() {
                    self.selected_index = None; // Reset navigation when search changes
                }

                ui.separator();

                // Selection info and clear button
                ui.horizontal(|ui| {
                    let selected_count = self.selected_resource_types_count();
                    if selected_count > 0 {
                        ui.label(format!("{} resource types selected", selected_count));
                        ui.separator();
                        if ui.button("Clear All").clicked() {
                            self.clear_all_resource_types();
                        }
                    } else {
                        ui.label("No resource types selected");
                    }
                });

                ui.separator();

                // Filter resource types based on search
                let filtered_types = self.filter_resource_types(available_resource_types);

                if filtered_types.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No matching resource types found");
                    });
                } else {
                    // Show filtered results in a scrollable area with checkboxes and grid layout
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            egui::Grid::new("resource_type_grid")
                                .num_columns(3)
                                .spacing([10.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for resource_type in &filtered_types {
                                        // Checkbox column
                                        let mut is_checked = self
                                            .selected_resource_types
                                            .get(&resource_type.resource_type)
                                            .copied()
                                            .unwrap_or(false);

                                        // Apply highlight color to checkbox for better visibility
                                        ui.scope(|ui| {
                                            let mut style = (*ui.ctx().style()).clone();
                                            style.visuals.selection.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            style.visuals.widgets.active.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            style.visuals.widgets.hovered.bg_fill =
                                                ui.visuals().selection.bg_fill;
                                            ui.ctx().set_style(style);

                                            if ui.checkbox(&mut is_checked, "").changed() {
                                                self.selected_resource_types.insert(
                                                    resource_type.resource_type.clone(),
                                                    is_checked,
                                                );
                                            }
                                        });

                                        // Service name column (left-aligned, consistent width)
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_min_width(80.0);
                                                ui.label(&resource_type.service_name);
                                            },
                                        );

                                        // Resource display name column
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.label(&resource_type.display_name);
                                            },
                                        );

                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let selected_count = self.selected_resource_types_count();
                        let can_add = selected_count > 0;

                        let button_text = match selected_count.cmp(&1) {
                            std::cmp::Ordering::Greater => {
                                format!("Add {} Resource Types", selected_count)
                            }
                            std::cmp::Ordering::Equal => "Add Resource Type".to_string(),
                            std::cmp::Ordering::Less => "Add Resource Types".to_string(),
                        };

                        if ui
                            .add_enabled(can_add, egui::Button::new(button_text))
                            .clicked()
                        {
                            let selected =
                                self.get_selected_resource_types(available_resource_types);
                            if !selected.is_empty() {
                                result = Some(selected);
                                should_close = true;
                            }
                        }
                    });
                });

                // Handle keyboard navigation
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }

                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let selected = self.get_selected_resource_types(available_resource_types);
                    if !selected.is_empty() {
                        result = Some(selected);
                        should_close = true;
                    }
                }
            });

        if should_close {
            *is_open = false;
            self.reset();
        }

        result
    }

    fn filter_accounts(&self, accounts: &[AwsAccount]) -> Vec<AwsAccount> {
        if self.search_term.is_empty() {
            // Sort by account name when no search term
            let mut sorted_accounts = accounts.to_vec();
            sorted_accounts.sort_by(|a, b| a.account_name.cmp(&b.account_name));
            return sorted_accounts;
        }

        let mut scored_accounts: Vec<(AwsAccount, i64)> = accounts
            .iter()
            .filter_map(|account| {
                // Create searchable string with account name, email, and ID
                let searchable_string = self.create_searchable_account_string(account);

                self.matcher
                    .fuzzy_match(&searchable_string, &self.search_term)
                    .map(|score| (account.clone(), score))
            })
            .collect();

        scored_accounts.sort_by(|a, b| b.1.cmp(&a.1));
        scored_accounts
            .into_iter()
            .map(|(account, _)| account)
            .collect()
    }

    /// Create a searchable string containing account name, email, and ID
    fn create_searchable_account_string(&self, account: &AwsAccount) -> String {
        let email_part = account
            .account_email
            .as_ref()
            .map(|email| format!(" {}", email))
            .unwrap_or_default();

        format!(
            "{} {} {}{}",
            account.account_name,
            account.account_id,
            account.account_id, // Include ID twice for better matching
            email_part
        )
    }

    fn filter_regions(&self, regions: &[String]) -> Vec<RegionSelection> {
        let region_selections: Vec<RegionSelection> = regions
            .iter()
            .map(|region| {
                RegionSelection::new(region.clone(), self.format_region_display_name(region))
            })
            .collect();

        if self.search_term.is_empty() {
            return region_selections;
        }

        let mut scored_regions: Vec<(RegionSelection, i64)> = region_selections
            .into_iter()
            .filter_map(|region| {
                self.matcher
                    .fuzzy_match(&region.display_name, &self.search_term)
                    .map(|score| (region, score))
            })
            .collect();

        scored_regions.sort_by(|a, b| b.1.cmp(&a.1));
        scored_regions
            .into_iter()
            .map(|(region, _)| region)
            .collect()
    }

    fn filter_resource_types(
        &self,
        resource_types: &[ResourceTypeSelection],
    ) -> Vec<ResourceTypeSelection> {
        if self.search_term.is_empty() {
            // Sort by service name first, then by display name
            let mut sorted_types = resource_types.to_vec();
            sorted_types.sort_by(|a, b| {
                a.service_name
                    .cmp(&b.service_name)
                    .then_with(|| a.display_name.cmp(&b.display_name))
            });
            return sorted_types;
        }

        let mut scored_types: Vec<(ResourceTypeSelection, i64)> = resource_types
            .iter()
            .filter_map(|rt| {
                let search_text = format!(
                    "{} {} {}",
                    rt.display_name, rt.resource_type, rt.service_name
                );
                self.matcher
                    .fuzzy_match(&search_text, &self.search_term)
                    .map(|score| (rt.clone(), score))
            })
            .collect();

        scored_types.sort_by(|a, b| b.1.cmp(&a.1));
        scored_types.into_iter().map(|(rt, _)| rt).collect()
    }

    fn format_account_display_name(&self, account: &AwsAccount) -> String {
        format!("{} - {}", account.account_name, account.account_id)
    }

    fn format_region_display_name(&self, region_code: &str) -> String {
        // Map region codes to friendly names
        match region_code {
            "us-east-1" => "US East (N. Virginia)".to_string(),
            "us-east-2" => "US East (Ohio)".to_string(),
            "us-west-1" => "US West (N. California)".to_string(),
            "us-west-2" => "US West (Oregon)".to_string(),
            "eu-west-1" => "Europe (Ireland)".to_string(),
            "eu-west-2" => "Europe (London)".to_string(),
            "eu-west-3" => "Europe (Paris)".to_string(),
            "eu-central-1" => "Europe (Frankfurt)".to_string(),
            "ap-southeast-1" => "Asia Pacific (Singapore)".to_string(),
            "ap-southeast-2" => "Asia Pacific (Sydney)".to_string(),
            "ap-northeast-1" => "Asia Pacific (Tokyo)".to_string(),
            "ap-northeast-2" => "Asia Pacific (Seoul)".to_string(),
            "ap-south-1" => "Asia Pacific (Mumbai)".to_string(),
            "sa-east-1" => "South America (São Paulo)".to_string(),
            "ca-central-1" => "Canada (Central)".to_string(),
            _ => region_code.to_string(),
        }
    }

    fn reset(&mut self) {
        self.search_term.clear();
        self.selected_index = None;
        self.selected_resource_types.clear();
        self.selected_accounts.clear();
        self.selected_regions.clear();
    }
}

// Default available options for testing
pub fn get_default_accounts() -> Vec<String> {
    // Return empty list instead of fake accounts
    // Real accounts should come from AWS Identity Center
    Vec::new()
}

pub fn get_default_regions() -> Vec<String> {
    vec![
        "us-east-1".to_string(),
        "us-east-2".to_string(),
        "us-west-1".to_string(),
        "us-west-2".to_string(),
        "eu-west-1".to_string(),
        "eu-west-2".to_string(),
        "eu-central-1".to_string(),
        "ap-southeast-1".to_string(),
        "ap-southeast-2".to_string(),
        "ap-northeast-1".to_string(),
    ]
}

pub fn get_default_resource_types() -> Vec<ResourceTypeSelection> {
    vec![
        // EC2 Resources
        ResourceTypeSelection::new(
            "AWS::EC2::Instance".to_string(),
            "EC2 Instance".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::SecurityGroup".to_string(),
            "Security Group".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::VPC".to_string(),
            "VPC".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::Volume".to_string(),
            "EBS Volume".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::Snapshot".to_string(),
            "EBS Snapshot".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::Image".to_string(),
            "AMI (Amazon Machine Image)".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::Subnet".to_string(),
            "Subnet".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::RouteTable".to_string(),
            "Route Table".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::NatGateway".to_string(),
            "NAT Gateway".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::NetworkInterface".to_string(),
            "Network Interface".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::VPCEndpoint".to_string(),
            "VPC Endpoint".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::NetworkAcl".to_string(),
            "Network ACL".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::KeyPair".to_string(),
            "Key Pair".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::InternetGateway".to_string(),
            "Internet Gateway".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::TransitGateway".to_string(),
            "Transit Gateway".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::VPCPeeringConnection".to_string(),
            "VPC Peering Connection".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::FlowLog".to_string(),
            "VPC Flow Log".to_string(),
            "EC2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EC2::VolumeAttachment".to_string(),
            "EBS Volume Attachment".to_string(),
            "EC2".to_string(),
        ),
        // Fargate Resources
        ResourceTypeSelection::new(
            "AWS::ECS::FargateService".to_string(),
            "ECS Fargate Service".to_string(),
            "ECS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ECS::FargateTask".to_string(),
            "ECS Fargate Task".to_string(),
            "ECS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::EKS::FargateProfile".to_string(),
            "EKS Fargate Profile".to_string(),
            "EKS".to_string(),
        ),
        // IAM Resources
        ResourceTypeSelection::new(
            "AWS::IAM::Role".to_string(),
            "IAM Role".to_string(),
            "IAM".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::IAM::User".to_string(),
            "IAM User".to_string(),
            "IAM".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::IAM::Policy".to_string(),
            "IAM Policy".to_string(),
            "IAM".to_string(),
        ),
        // S3 Resources
        ResourceTypeSelection::new(
            "AWS::S3::Bucket".to_string(),
            "S3 Bucket".to_string(),
            "S3".to_string(),
        ),
        // CloudFormation Resources
        ResourceTypeSelection::new(
            "AWS::CloudFormation::Stack".to_string(),
            "CloudFormation Stack".to_string(),
            "CloudFormation".to_string(),
        ),
        // Certificate Manager Resources
        ResourceTypeSelection::new(
            "AWS::CertificateManager::Certificate".to_string(),
            "SSL/TLS Certificate".to_string(),
            "Certificate Manager".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ACMPCA::CertificateAuthority".to_string(),
            "Private Certificate Authority".to_string(),
            "Certificate Manager".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::WAFv2::WebACL".to_string(),
            "Web Application Firewall".to_string(),
            "WAF & Shield".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::GuardDuty::Detector".to_string(),
            "Threat Detection Service".to_string(),
            "GuardDuty".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::SecurityHub::Hub".to_string(),
            "Security Hub Service".to_string(),
            "Security Hub".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Detective::Graph".to_string(),
            "Detective Security Investigation".to_string(),
            "Detective".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::AccessAnalyzer::Analyzer".to_string(),
            "IAM Access Analyzer".to_string(),
            "Access Analyzer".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CloudFront::Distribution".to_string(),
            "Content Delivery Network".to_string(),
            "CloudFront".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ElastiCache::CacheCluster".to_string(),
            "Cache Cluster".to_string(),
            "ElastiCache".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ElastiCache::ReplicationGroup".to_string(),
            "Redis Replication Group".to_string(),
            "ElastiCache".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ElastiCache::ParameterGroup".to_string(),
            "Cache Parameter Group".to_string(),
            "ElastiCache".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Neptune::DBCluster".to_string(),
            "Graph Database Cluster".to_string(),
            "Neptune".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Neptune::DBInstance".to_string(),
            "Graph Database Instance".to_string(),
            "Neptune".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::OpenSearchService::Domain".to_string(),
            "Search and Analytics Engine".to_string(),
            "OpenSearch".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Cognito::UserPool".to_string(),
            "User Pool".to_string(),
            "Cognito".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Cognito::IdentityPool".to_string(),
            "Identity Pool".to_string(),
            "Cognito".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Cognito::UserPoolClient".to_string(),
            "User Pool Client".to_string(),
            "Cognito".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Batch::JobQueue".to_string(),
            "Batch Job Queue".to_string(),
            "Batch".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Batch::ComputeEnvironment".to_string(),
            "Batch Compute Environment".to_string(),
            "Batch".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::KinesisFirehose::DeliveryStream".to_string(),
            "Kinesis Data Firehose Delivery Stream".to_string(),
            "Kinesis Data Firehose".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::MSK::Cluster".to_string(),
            "MSK Kafka Cluster".to_string(),
            "MSK".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::QuickSight::DataSource".to_string(),
            "QuickSight Data Source".to_string(),
            "QuickSight".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::QuickSight::Dashboard".to_string(),
            "QuickSight Dashboard".to_string(),
            "QuickSight".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::QuickSight::DataSet".to_string(),
            "QuickSight Data Set".to_string(),
            "QuickSight".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Macie::Session".to_string(),
            "Macie Session".to_string(),
            "Security".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Inspector::Configuration".to_string(),
            "Inspector Configuration".to_string(),
            "Security".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::KMS::Key".to_string(),
            "KMS Encryption Key".to_string(),
            "KMS".to_string(),
        ),
        // Auto Scaling Resources
        ResourceTypeSelection::new(
            "AWS::AutoScaling::AutoScalingGroup".to_string(),
            "Auto Scaling Group".to_string(),
            "Auto Scaling".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::AutoScaling::ScalingPolicy".to_string(),
            "Auto Scaling Policy".to_string(),
            "Auto Scaling".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::SecretsManager::Secret".to_string(),
            "Secrets Manager Secret".to_string(),
            "Secrets Manager".to_string(),
        ),
        // Step Functions Resources
        ResourceTypeSelection::new(
            "AWS::StepFunctions::StateMachine".to_string(),
            "Step Functions State Machine".to_string(),
            "Step Functions".to_string(),
        ),
        // X-Ray Resources
        ResourceTypeSelection::new(
            "AWS::XRay::SamplingRule".to_string(),
            "X-Ray Sampling Rule".to_string(),
            "X-Ray".to_string(),
        ),
        // Shield Resources (DDoS Protection)
        ResourceTypeSelection::new(
            "AWS::Shield::Protection".to_string(),
            "Shield Protection".to_string(),
            "Shield".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Shield::Subscription".to_string(),
            "Shield Advanced Subscription".to_string(),
            "Shield".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Timestream::Database".to_string(),
            "Timestream Database".to_string(),
            "Database".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::DocumentDB::Cluster".to_string(),
            "DocumentDB Cluster".to_string(),
            "Database".to_string(),
        ),
        // RDS Resources
        ResourceTypeSelection::new(
            "AWS::RDS::DBInstance".to_string(),
            "RDS DB Instance".to_string(),
            "RDS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::RDS::DBCluster".to_string(),
            "RDS DB Cluster".to_string(),
            "RDS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::RDS::DBSnapshot".to_string(),
            "RDS DB Snapshot".to_string(),
            "RDS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::RDS::DBParameterGroup".to_string(),
            "RDS DB Parameter Group".to_string(),
            "RDS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::RDS::DBSubnetGroup".to_string(),
            "RDS DB Subnet Group".to_string(),
            "RDS".to_string(),
        ),
        // Lambda Resources
        ResourceTypeSelection::new(
            "AWS::Lambda::Function".to_string(),
            "Lambda Function".to_string(),
            "Lambda".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Lambda::LayerVersion".to_string(),
            "Lambda Layer".to_string(),
            "Lambda".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Lambda::EventSourceMapping".to_string(),
            "Lambda Event Source Mapping".to_string(),
            "Lambda".to_string(),
        ),
        // DynamoDB Resources
        ResourceTypeSelection::new(
            "AWS::DynamoDB::Table".to_string(),
            "DynamoDB Table".to_string(),
            "DynamoDB".to_string(),
        ),
        // CloudWatch Resources
        ResourceTypeSelection::new(
            "AWS::CloudWatch::Alarm".to_string(),
            "CloudWatch Alarm".to_string(),
            "CloudWatch".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CloudWatch::Dashboard".to_string(),
            "CloudWatch Dashboard".to_string(),
            "CloudWatch".to_string(),
        ),
        // API Gateway Resources
        ResourceTypeSelection::new(
            "AWS::ApiGateway::RestApi".to_string(),
            "API Gateway REST API".to_string(),
            "API Gateway".to_string(),
        ),
        // SNS Resources
        ResourceTypeSelection::new(
            "AWS::SNS::Topic".to_string(),
            "SNS Topic".to_string(),
            "SNS".to_string(),
        ),
        // SQS Resources
        ResourceTypeSelection::new(
            "AWS::SQS::Queue".to_string(),
            "SQS Queue".to_string(),
            "SQS".to_string(),
        ),
        // ECS Resources
        ResourceTypeSelection::new(
            "AWS::ECS::Cluster".to_string(),
            "ECS Cluster".to_string(),
            "ECS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ECS::Service".to_string(),
            "ECS Service".to_string(),
            "ECS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ECS::Task".to_string(),
            "ECS Task".to_string(),
            "ECS".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ECS::TaskDefinition".to_string(),
            "ECS Task Definition".to_string(),
            "ECS".to_string(),
        ),
        // EKS Resources
        ResourceTypeSelection::new(
            "AWS::EKS::Cluster".to_string(),
            "EKS Cluster".to_string(),
            "EKS".to_string(),
        ),
        // Load Balancer Resources
        ResourceTypeSelection::new(
            "AWS::ElasticLoadBalancing::LoadBalancer".to_string(),
            "Classic Load Balancer".to_string(),
            "ELB".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ElasticLoadBalancingV2::LoadBalancer".to_string(),
            "Application/Network Load Balancer".to_string(),
            "ELBv2".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ElasticLoadBalancingV2::TargetGroup".to_string(),
            "Target Group".to_string(),
            "ELBv2".to_string(),
        ),
        // CloudWatch Logs Resources
        ResourceTypeSelection::new(
            "AWS::Logs::LogGroup".to_string(),
            "CloudWatch Log Group".to_string(),
            "CloudWatch".to_string(),
        ),
        // API Gateway v2 Resources
        ResourceTypeSelection::new(
            "AWS::ApiGatewayV2::Api".to_string(),
            "API Gateway v2 HTTP API".to_string(),
            "API Gateway".to_string(),
        ),
        // Kinesis Resources
        ResourceTypeSelection::new(
            "AWS::Kinesis::Stream".to_string(),
            "Kinesis Data Stream".to_string(),
            "Kinesis".to_string(),
        ),
        // SageMaker Resources
        ResourceTypeSelection::new(
            "AWS::SageMaker::Endpoint".to_string(),
            "SageMaker Endpoint".to_string(),
            "SageMaker".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::SageMaker::TrainingJob".to_string(),
            "SageMaker Training Job".to_string(),
            "SageMaker".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::SageMaker::Model".to_string(),
            "SageMaker Model".to_string(),
            "SageMaker".to_string(),
        ),
        // Redshift Resources
        ResourceTypeSelection::new(
            "AWS::Redshift::Cluster".to_string(),
            "Redshift Cluster".to_string(),
            "Redshift".to_string(),
        ),
        // Glue Resources
        ResourceTypeSelection::new(
            "AWS::Glue::Job".to_string(),
            "Glue ETL Job".to_string(),
            "Glue".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::LakeFormation::DataLakeSettings".to_string(),
            "Lake Formation Data Lake Settings".to_string(),
            "Lake Formation".to_string(),
        ),
        // Athena Resources
        ResourceTypeSelection::new(
            "AWS::Athena::WorkGroup".to_string(),
            "Athena Workgroup".to_string(),
            "Athena".to_string(),
        ),
        // Bedrock Resources
        ResourceTypeSelection::new(
            "AWS::Bedrock::Model".to_string(),
            "Bedrock Foundation Model".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::InferenceProfile".to_string(),
            "Bedrock Inference Profile".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::Guardrail".to_string(),
            "Bedrock Guardrail".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::ProvisionedModelThroughput".to_string(),
            "Bedrock Provisioned Model Throughput".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::Agent".to_string(),
            "Bedrock Agent".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::KnowledgeBase".to_string(),
            "Bedrock Knowledge Base".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::CustomModel".to_string(),
            "Bedrock Custom Model".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::ImportedModel".to_string(),
            "Bedrock Imported Model".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::EvaluationJob".to_string(),
            "Bedrock Evaluation Job".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::ModelInvocationJob".to_string(),
            "Bedrock Model Invocation Job".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::Prompt".to_string(),
            "Bedrock Prompt".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::Flow".to_string(),
            "Bedrock Flow".to_string(),
            "Bedrock".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Bedrock::ModelCustomizationJob".to_string(),
            "Bedrock Model Customization Job".to_string(),
            "Bedrock".to_string(),
        ),
        // BedrockAgentCore - Control Plane Resources
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::AgentRuntime".to_string(),
            "Bedrock AgentCore Runtime".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::AgentRuntimeEndpoint".to_string(),
            "Bedrock AgentCore Runtime Endpoint".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::Memory".to_string(),
            "Bedrock AgentCore Memory".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::Gateway".to_string(),
            "Bedrock AgentCore Gateway".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::Browser".to_string(),
            "Bedrock AgentCore Browser".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        // BedrockAgentCore - Additional Control Plane Resources
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::CodeInterpreter".to_string(),
            "Bedrock AgentCore Code Interpreter".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::ApiKeyCredentialProvider".to_string(),
            "Bedrock AgentCore API Key Credential Provider".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::OAuth2CredentialProvider".to_string(),
            "Bedrock AgentCore OAuth2 Credential Provider".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::BedrockAgentCore::WorkloadIdentity".to_string(),
            "Bedrock AgentCore Workload Identity".to_string(),
            "Bedrock AgentCore".to_string(),
        ),
        // Route53 Resources
        ResourceTypeSelection::new(
            "AWS::Route53::HostedZone".to_string(),
            "Route53 Hosted Zone".to_string(),
            "Route53".to_string(),
        ),
        // EFS Resources
        ResourceTypeSelection::new(
            "AWS::EFS::FileSystem".to_string(),
            "EFS File System".to_string(),
            "EFS".to_string(),
        ),
        // Transfer Family Resources
        ResourceTypeSelection::new(
            "AWS::Transfer::Server".to_string(),
            "Transfer Family Server".to_string(),
            "Transfer Family".to_string(),
        ),
        // DataSync Resources
        ResourceTypeSelection::new(
            "AWS::DataSync::Task".to_string(),
            "DataSync Task".to_string(),
            "DataSync".to_string(),
        ),
        // FSx Resources
        ResourceTypeSelection::new(
            "AWS::FSx::FileSystem".to_string(),
            "FSx File System".to_string(),
            "FSx".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::FSx::Backup".to_string(),
            "FSx Backup".to_string(),
            "FSx".to_string(),
        ),
        // WorkSpaces Resources
        ResourceTypeSelection::new(
            "AWS::WorkSpaces::Workspace".to_string(),
            "WorkSpaces Workspace".to_string(),
            "WorkSpaces".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::WorkSpaces::Directory".to_string(),
            "WorkSpaces Directory".to_string(),
            "WorkSpaces".to_string(),
        ),
        // App Runner Resources
        ResourceTypeSelection::new(
            "AWS::AppRunner::Service".to_string(),
            "App Runner Service".to_string(),
            "App Runner".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::AppRunner::Connection".to_string(),
            "App Runner Connection".to_string(),
            "App Runner".to_string(),
        ),
        // CloudTrail Resources
        ResourceTypeSelection::new(
            "AWS::CloudTrail::Trail".to_string(),
            "CloudTrail Trail".to_string(),
            "CloudTrail".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CloudTrail::Event".to_string(),
            "CloudTrail Event".to_string(),
            "CloudTrail".to_string(),
        ),
        // Config Resources
        ResourceTypeSelection::new(
            "AWS::Config::ConfigurationRecorder".to_string(),
            "Config Configuration Recorder".to_string(),
            "Config".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Config::ConfigRule".to_string(),
            "Config Rule".to_string(),
            "Config".to_string(),
        ),
        // Data Preparation Resources
        ResourceTypeSelection::new(
            "AWS::DataBrew::Job".to_string(),
            "DataBrew Job".to_string(),
            "DataBrew".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::DataBrew::Dataset".to_string(),
            "DataBrew Dataset".to_string(),
            "DataBrew".to_string(),
        ),
        // Code Artifact Resources
        ResourceTypeSelection::new(
            "AWS::CodeArtifact::Domain".to_string(),
            "CodeArtifact Domain".to_string(),
            "CodeArtifact".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CodeArtifact::Repository".to_string(),
            "CodeArtifact Repository".to_string(),
            "CodeArtifact".to_string(),
        ),
        // CodeDeploy Resources
        ResourceTypeSelection::new(
            "AWS::CodeDeploy::Application".to_string(),
            "CodeDeploy Application".to_string(),
            "CodeDeploy".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CodeDeploy::DeploymentGroup".to_string(),
            "CodeDeploy Deployment Group".to_string(),
            "CodeDeploy".to_string(),
        ),
        // AppConfig Resources
        ResourceTypeSelection::new(
            "AWS::AppConfig::Application".to_string(),
            "AppConfig Application".to_string(),
            "AppConfig".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::AppConfig::Environment".to_string(),
            "AppConfig Environment".to_string(),
            "AppConfig".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::AppConfig::ConfigurationProfile".to_string(),
            "AppConfig Configuration Profile".to_string(),
            "AppConfig".to_string(),
        ),
        // CloudTrail Lake Resources
        ResourceTypeSelection::new(
            "AWS::CloudTrail::EventDataStore".to_string(),
            "CloudTrail Event Data Store".to_string(),
            "CloudTrail".to_string(),
        ),
        // Systems Manager Resources
        ResourceTypeSelection::new(
            "AWS::SSM::Parameter".to_string(),
            "Systems Manager Parameter".to_string(),
            "SSM".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::SSM::Document".to_string(),
            "Systems Manager Document".to_string(),
            "SSM".to_string(),
        ),
        // AWS Backup Resources
        ResourceTypeSelection::new(
            "AWS::Backup::BackupPlan".to_string(),
            "Backup Plan".to_string(),
            "Backup".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Backup::BackupVault".to_string(),
            "Backup Vault".to_string(),
            "Backup".to_string(),
        ),
        // AWS Organizations Resources
        ResourceTypeSelection::new(
            "AWS::Organizations::Account".to_string(),
            "Organizations Account".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::DelegatedAdministrator".to_string(),
            "Delegated Administrator".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::Handshake".to_string(),
            "Organization Handshake".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::CreateAccountStatus".to_string(),
            "Account Creation Status".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::AwsServiceAccess".to_string(),
            "Service Access".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::Organization".to_string(),
            "Organization".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::OrganizationalUnit".to_string(),
            "Organizational Unit".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::Policy".to_string(),
            "Service Control Policy".to_string(),
            "Organizations".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Organizations::Root".to_string(),
            "Organizations Root".to_string(),
            "Organizations".to_string(),
        ),
        // EventBridge Resources
        ResourceTypeSelection::new(
            "AWS::Events::EventBus".to_string(),
            "EventBridge Event Bus".to_string(),
            "EventBridge".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Events::Rule".to_string(),
            "EventBridge Rule".to_string(),
            "EventBridge".to_string(),
        ),
        // AppSync Resources
        ResourceTypeSelection::new(
            "AWS::AppSync::GraphQLApi".to_string(),
            "AppSync GraphQL API".to_string(),
            "AppSync".to_string(),
        ),
        // Amazon MQ Resources
        ResourceTypeSelection::new(
            "AWS::AmazonMQ::Broker".to_string(),
            "Amazon MQ Broker".to_string(),
            "MQ".to_string(),
        ),
        // Developer Tools Resources
        ResourceTypeSelection::new(
            "AWS::CodePipeline::Pipeline".to_string(),
            "CodePipeline Pipeline".to_string(),
            "CodePipeline".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CodeBuild::Project".to_string(),
            "CodeBuild Project".to_string(),
            "CodeBuild".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::CodeCommit::Repository".to_string(),
            "CodeCommit Repository".to_string(),
            "CodeCommit".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::ECR::Repository".to_string(),
            "ECR Container Registry".to_string(),
            "ECR".to_string(),
        ),
        // IoT and Edge Services Resources
        ResourceTypeSelection::new(
            "AWS::IoT::Thing".to_string(),
            "IoT Thing".to_string(),
            "IoT Core".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::GreengrassV2::ComponentVersion".to_string(),
            "Greengrass Component Version".to_string(),
            "Greengrass".to_string(),
        ),
        // Core Infrastructure Services
        ResourceTypeSelection::new(
            "AWS::GlobalAccelerator::Accelerator".to_string(),
            "Global Accelerator".to_string(),
            "GlobalAccelerator".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Connect::Instance".to_string(),
            "Connect Instance".to_string(),
            "Connect".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Amplify::App".to_string(),
            "Amplify App".to_string(),
            "Amplify".to_string(),
        ),
        // AI/ML Services
        ResourceTypeSelection::new(
            "AWS::Lex::Bot".to_string(),
            "Lex Bot".to_string(),
            "Lex".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Rekognition::Collection".to_string(),
            "Rekognition Collection".to_string(),
            "Rekognition".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Rekognition::StreamProcessor".to_string(),
            "Rekognition Stream Processor".to_string(),
            "Rekognition".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Polly::Voice".to_string(),
            "Polly Voice".to_string(),
            "Polly".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Polly::Lexicon".to_string(),
            "Polly Lexicon".to_string(),
            "Polly".to_string(),
        ),
        ResourceTypeSelection::new(
            "AWS::Polly::SynthesisTask".to_string(),
            "Polly Synthesis Task".to_string(),
            "Polly".to_string(),
        ),
    ]
}
