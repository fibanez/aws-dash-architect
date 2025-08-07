//! Compliance Program Selector Component
//!
//! This component provides a tag-based interface for selecting compliance programs,
//! similar to how resource explorer allows adding tags. It supports search, filtering
//! by category, and displays selected programs as colored tags.

use crate::app::cfn_guard::{ComplianceCategory, ComplianceProgram};
use crate::app::compliance_discovery::{ComplianceDiscovery, AvailableComplianceProgram};
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// State for compliance program selection UI
#[derive(Default)]
pub struct ComplianceProgramSelector {
    /// Currently selected programs by ID
    pub selected_programs: HashSet<String>,
    /// Full list of selected CompliancePrograms (for external use)
    pub selected_compliance_programs: Vec<ComplianceProgram>,
    /// Search query for filtering
    search_query: String,
    /// Selected category filter (None = all categories)
    selected_category: Option<ComplianceCategory>,
    /// Available programs loaded from discovery
    available_programs: Vec<AvailableComplianceProgram>,
    /// Error loading programs
    loading_error: Option<String>,
    /// Whether programs are being loaded
    is_loading: bool,
    /// Whether we've attempted to load programs yet
    has_attempted_load: bool,
    /// ComplianceDiscovery instance for loading programs
    compliance_discovery: Option<Arc<Mutex<ComplianceDiscovery>>>,
    /// Whether the add programs popup is open
    show_add_popup: bool,
    /// Cached color assignments for categories
    category_colors: HashMap<ComplianceCategory, Color32>,
}

impl ComplianceProgramSelector {
    pub fn new() -> Self {
        let mut selector = Self::default();
        selector.init_category_colors();
        selector
    }

    /// Initialize color assignments for different compliance categories
    fn init_category_colors(&mut self) {
        self.category_colors.insert(ComplianceCategory::Government, Color32::from_rgb(70, 130, 180));      // Steel blue
        self.category_colors.insert(ComplianceCategory::Industry, Color32::from_rgb(60, 179, 113));        // Medium sea green  
        self.category_colors.insert(ComplianceCategory::International, Color32::from_rgb(186, 85, 211));   // Medium orchid
        self.category_colors.insert(ComplianceCategory::Framework, Color32::from_rgb(255, 140, 0));        // Dark orange
        self.category_colors.insert(ComplianceCategory::Custom("".to_string()), Color32::from_rgb(128, 128, 128)); // Gray
    }

    /// Set the ComplianceDiscovery instance for loading programs
    pub fn set_compliance_discovery(&mut self, discovery: Arc<Mutex<ComplianceDiscovery>>) {
        self.compliance_discovery = Some(discovery);
    }

    /// Load available compliance programs from discovery service
    pub async fn load_programs(&mut self, discovery: Arc<Mutex<ComplianceDiscovery>>) {
        self.is_loading = true;
        self.loading_error = None;
        
        match discovery.lock() {
            Ok(mut disc) => {
                match disc.discover_available_programs().await {
                    Ok(programs) => {
                        self.available_programs = programs;
                        self.is_loading = false;
                        tracing::info!("Loaded {} compliance programs", self.available_programs.len());
                    }
                    Err(e) => {
                        self.loading_error = Some(format!("Failed to discover compliance programs: {}", e));
                        self.is_loading = false;
                        tracing::error!("Failed to load compliance programs: {}", e);
                    }
                }
            }
            Err(e) => {
                self.loading_error = Some(format!("Unable to access compliance discovery: {}", e));
                self.is_loading = false;
            }
        }
    }

    /// Set the currently selected programs (for initialization from existing project)
    pub fn set_selected_programs(&mut self, programs: Vec<ComplianceProgram>) {
        self.selected_programs.clear();
        for program in &programs {
            self.selected_programs.insert(program.id.clone());
        }
        self.selected_compliance_programs = programs;
    }

    /// Get the currently selected programs as ComplianceProgram structs
    pub fn get_selected_programs(&self) -> Vec<ComplianceProgram> {
        self.selected_compliance_programs.clone()
    }

    /// Add a program to the selection
    fn add_program(&mut self, program: &AvailableComplianceProgram) {
        if !self.selected_programs.contains(&program.name) {
            self.selected_programs.insert(program.name.clone());
            self.selected_compliance_programs.push(program.to_compliance_program());
        }
    }

    /// Remove a program from the selection
    fn remove_program(&mut self, program_id: &str) {
        if self.selected_programs.remove(program_id) {
            self.selected_compliance_programs.retain(|p| p.id != program_id);
        }
    }

    /// Get color for a compliance program based on its category
    fn get_program_color(&self, program: &AvailableComplianceProgram) -> Color32 {
        let category = match program.category.as_str() {
            "Government" => ComplianceCategory::Government,
            "Industry" => ComplianceCategory::Industry,
            "International" => ComplianceCategory::International,
            "Framework" => ComplianceCategory::Framework,
            _ => ComplianceCategory::Custom(program.category.clone()),
        };
        
        self.category_colors.get(&category)
            .copied()
            .unwrap_or(Color32::from_rgb(128, 128, 128))
    }

    /// Filter available programs based on search and category
    fn filter_programs(&self) -> Vec<&AvailableComplianceProgram> {
        let query_lower = self.search_query.to_lowercase();
        
        self.available_programs
            .iter()
            .filter(|program| {
                // Category filter
                if let Some(ref selected_cat) = self.selected_category {
                    let program_category = match program.category.as_str() {
                        "Government" => ComplianceCategory::Government,
                        "Industry" => ComplianceCategory::Industry,
                        "International" => ComplianceCategory::International,
                        "Framework" => ComplianceCategory::Framework,
                        _ => ComplianceCategory::Custom(program.category.clone()),
                    };
                    
                    if std::mem::discriminant(selected_cat) != std::mem::discriminant(&program_category) {
                        return false;
                    }
                }
                
                // Search filter
                if !query_lower.is_empty() {
                    let matches_name = program.display_name.to_lowercase().contains(&query_lower);
                    let matches_description = program.description.to_lowercase().contains(&query_lower);
                    let matches_tags = program.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));
                    
                    if !matches_name && !matches_description && !matches_tags {
                        return false;
                    }
                }
                
                true
            })
            .collect()
    }

    /// Try to load programs synchronously if ComplianceDiscovery is available
    fn try_load_programs(&mut self) {
        if let Some(ref discovery) = self.compliance_discovery.clone() {
            self.is_loading = true;
            self.has_attempted_load = true;
            self.loading_error = None;
            
            match discovery.lock() {
                Ok(mut disc) => {
                    // First check if the repository is available
                    if !self.is_repository_ready(&disc) {
                        self.loading_error = Some(
                            "Guard rules repository is still being downloaded. Please wait for the sync to complete and try again.".to_string()
                        );
                        self.is_loading = false;
                        return;
                    }
                    
                    // Debug: Log repository status
                    tracing::info!("Repository ready check passed, attempting discovery...");
                    
                    // Use a blocking approach for simplicity in UI code
                    // This will be fast for cached results, slower for first-time downloads
                    match self.sync_discover_programs(&mut disc) {
                        Ok(programs) => {
                            self.available_programs = programs;
                            self.is_loading = false;
                            tracing::info!("Loaded {} compliance programs", self.available_programs.len());
                        }
                        Err(e) => {
                            // Provide more user-friendly error messages
                            let user_friendly_error = if e.contains("repository not available") {
                                "Guard rules repository is not available. The repository may still be syncing or there was an error during download. Please try restarting the application.".to_string()
                            } else if e.contains("No compliance programs found") {
                                "No compliance programs found in the repository. This might indicate a repository structure issue.".to_string()
                            } else {
                                format!("Failed to load compliance programs: {}", e)
                            };
                            
                            self.loading_error = Some(user_friendly_error);
                            self.is_loading = false;
                            tracing::error!("Failed to load compliance programs: {}", e);
                        }
                    }
                }
                Err(e) => {
                    self.loading_error = Some(format!("Failed to access compliance discovery: {}", e));
                    self.is_loading = false;
                    tracing::error!("Failed to lock compliance discovery: {}", e);
                }
            }
        } else {
            self.loading_error = Some("Compliance discovery service not initialized".to_string());
            self.is_loading = false;
        }
    }
    
    /// Check if the repository is ready for compliance discovery
    fn is_repository_ready(&self, _discovery: &ComplianceDiscovery) -> bool {
        // Access the repository manager to check if repository is cloned
        use crate::app::guard_repository_manager::GuardRepositoryManager;
        
        match GuardRepositoryManager::new() {
            Ok(manager) => manager.is_repository_cloned(),
            Err(_) => false,
        }
    }
    
    /// Get detailed repository status information for user feedback
    fn get_repository_status_info(&self) -> Option<String> {
        use crate::app::guard_repository_manager::GuardRepositoryManager;
        
        match GuardRepositoryManager::new() {
            Ok(manager) => {
                if !manager.is_repository_cloned() {
                    Some("💡 The Guard rules repository is not yet available. It may still be downloading in the background.".to_string())
                } else {
                    // Repository exists but discovery failed - could be JSON parsing issue
                    Some("💡 Repository is available but compliance program discovery failed. This may be a temporary issue.".to_string())
                }
            },
            Err(_) => {
                Some("💡 Unable to check repository status. There may be a configuration issue.".to_string())
            }
        }
    }

    /// Synchronous wrapper for discovering compliance programs
    fn sync_discover_programs(&mut self, discovery: &mut ComplianceDiscovery) -> Result<Vec<AvailableComplianceProgram>, String> {
        // The issue is that when creating a new tokio runtime, the working directory context changes
        // causing "No such file or directory" errors. Let's use a direct synchronous approach instead.
        
        tracing::info!("Starting sync discovery of compliance programs...");
        
        // Instead of trying to run the async method, let's call the sync repository parsing directly
        // This avoids the tokio runtime context issues entirely
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.sync_parse_local_repository(discovery)
        })) {
            Ok(result) => result,
            Err(_) => {
                tracing::error!("Discovery panicked during execution");
                Err("Discovery failed due to internal error".to_string())
            }
        }
    }
    
    /// Synchronous version of parse_local_repository to avoid tokio runtime issues
    fn sync_parse_local_repository(&self, discovery: &ComplianceDiscovery) -> Result<Vec<AvailableComplianceProgram>, String> {
        let mappings_dir = discovery.get_mappings_path();
        
        tracing::info!("Looking for mappings in: {:?}", mappings_dir);

        if !mappings_dir.exists() {
            return Err(format!("Mappings directory not found in cloned repository: {:?}", mappings_dir));
        }
        
        let mut programs = Vec::new();
        let mut processed_count = 0;
        let mut error_count = 0;
        
        // Read all files in the mappings directory
        match std::fs::read_dir(&mappings_dir) {
            Ok(entries) => {
                for entry_result in entries {
                    match entry_result {
                        Ok(entry) => {
                            let path = entry.path();
                            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                tracing::debug!("Checking file: {}", file_name);
                                
                                if file_name.ends_with(".guard") || file_name.ends_with(".json") {
                                    processed_count += 1;
                                    tracing::info!("Processing mapping file: {}", file_name);
                                    
                                    // Parse mapping file synchronously
                                    match self.sync_create_compliance_program_from_local_file(&path) {
                                        Ok(program) => {
                                            tracing::info!("Successfully parsed program: {} ({})", program.display_name, program.name);
                                            programs.push(program);
                                        }
                                        Err(e) => {
                                            error_count += 1;
                                            tracing::warn!("Failed to process mapping file {:?}: {}. Skipping.", path, e);
                                        }
                                    }
                                } else {
                                    tracing::debug!("Skipping non-mapping file: {}", file_name);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to read directory entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to read mappings directory: {}", e));
            }
        }
        
        tracing::info!("Discovery summary: processed {} files, {} errors, {} programs found", 
            processed_count, error_count, programs.len());

        if programs.is_empty() {
            return Err("No compliance programs found in repository mappings directory".to_string());
        }

        Ok(programs)
    }
    
    /// Synchronous version of create_compliance_program_from_local_file 
    fn sync_create_compliance_program_from_local_file(&self, file_path: &std::path::Path) -> Result<AvailableComplianceProgram, String> {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("Invalid file path: {:?}", file_path))?;

        // First try to extract metadata from JSON content if it's a JSON file
        if file_name.ends_with(".json") {
            match self.sync_parse_json_mapping_file(file_path) {
                Ok(program) => return Ok(program),
                Err(e) => {
                    tracing::warn!("Failed to parse JSON mapping file {}: {}", file_name, e);
                    // Fall through to filename parsing as backup
                }
            }
        }

        // Fallback: Parse the mapping file name to extract program information  
        let (program_id, display_name, description, category) = self.sync_parse_mapping_filename(file_name)
            .ok_or_else(|| format!("Unable to parse mapping filename: {}", file_name))?;
        
        // For the fallback, we'll use a default rule count since we can't easily count rules synchronously
        let rule_count = 10; // Default estimate
        
        let tags = vec![category.clone(), "Guard".to_string()];
        
        Ok(AvailableComplianceProgram {
            name: program_id,
            display_name,
            description,
            github_path: format!("mappings/{}", file_name),
            estimated_rule_count: rule_count,
            category,
            tags,
        })
    }
    
    /// Synchronous version of parse_json_mapping_file
    fn sync_parse_json_mapping_file(&self, file_path: &std::path::Path) -> Result<AvailableComplianceProgram, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        let json_data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        // Extract metadata from JSON structure
        let rule_set_name = json_data.get("ruleSetName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing ruleSetName in JSON mapping file".to_string())?;
            
        let description = json_data.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description available");
            
        let version = json_data.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0");
            
        // Count the number of mappings (rules)
        let rule_count = json_data.get("mappings")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        
        // Generate program details
        let program_id = rule_set_name.to_string(); // Use ruleSetName as unique ID
        let display_name = self.sync_format_description_for_display(description);
        let category = self.sync_categorize_compliance_program(rule_set_name);
        
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
            
        let tags = vec![category.clone(), "Guard".to_string()];
        
        Ok(AvailableComplianceProgram {
            name: program_id,
            display_name,
            description: format!("{} (v{})", description, version),
            github_path: format!("mappings/{}", file_name),
            estimated_rule_count: rule_count,
            category,
            tags,
        })
    }
    
    // Helper methods for synchronous processing
    
    fn sync_format_description_for_display(&self, description: &str) -> String {
        // Remove "AWS Guard rule set for" prefix from description if present
        let cleaned_description = description
            .strip_prefix("AWS Guard rule set for ")
            .unwrap_or(description);
        
        // Return the cleaned description as the display name
        cleaned_description.to_string()
    }
    
    
    fn sync_categorize_compliance_program(&self, rule_set_name: &str) -> String {
        let name_lower = rule_set_name.to_lowercase();
        
        if name_lower.contains("fedramp") || name_lower.contains("nist") || name_lower.contains("cisa") {
            "Government".to_string()
        } else if name_lower.contains("pci") || name_lower.contains("hipaa") || name_lower.contains("sox") {
            "Industry".to_string()
        } else if name_lower.contains("iso") || name_lower.contains("cis") {
            "International".to_string()
        } else if name_lower.contains("cmmc") || name_lower.contains("framework") {
            "Framework".to_string()
        } else {
            "Custom".to_string()
        }
    }
    
    fn sync_parse_mapping_filename(&self, filename: &str) -> Option<(String, String, String, String)> {
        // Remove "rule_set_" prefix and file extension suffix
        let name_part = filename.strip_prefix("rule_set_")?;
        let name_part = name_part.strip_suffix(".guard")
            .or_else(|| name_part.strip_suffix(".json"))?;
        
        let program_id = name_part.to_lowercase().replace("-", "_");
        let display_name = name_part.replace("_", " ").replace("-", " ");
        let description = format!("Compliance program for {}", display_name);
        let category = "Compliance".to_string();
        
        Some((program_id, display_name, description, category))
    }

    /// Render the main selector UI
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Header with selected programs count
            ui.horizontal(|ui| {
                ui.label(RichText::new("Compliance Programs:").strong());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Add programs button
                    let add_button = egui::Button::new("+ Add Programs")
                        .fill(Color32::from_rgb(70, 130, 180));
                    
                    if ui.add(add_button).clicked() {
                        self.show_add_popup = true;
                        // Trigger automatic loading if no programs are available
                        if self.available_programs.is_empty() && !self.has_attempted_load && !self.is_loading {
                            self.try_load_programs();
                        }
                    }
                    
                    // Selected count
                    if !self.selected_programs.is_empty() {
                        ui.label(
                            RichText::new(format!("{} selected", self.selected_programs.len()))
                                .color(Color32::LIGHT_GRAY)
                        );
                    }
                });
            });

            ui.separator();

            // Show selected programs as tags
            if self.selected_programs.is_empty() {
                ui.label(
                    RichText::new("No compliance programs selected")
                        .weak()
                        .italics()
                );
            } else {
                self.show_selected_tags(ui);
            }

            // Add programs popup
            if self.show_add_popup {
                self.show_add_popup(ui);
            }

            // Error display
            if let Some(ref error) = self.loading_error {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠️").color(Color32::from_rgb(220, 100, 50)));
                    ui.label(
                        RichText::new(error)
                            .color(Color32::from_rgb(220, 100, 50))
                    );
                });
            }
        });
    }

    /// Show selected programs as colored tags
    fn show_selected_tags(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut to_remove = Vec::new();
            
            for program in &self.selected_compliance_programs {
                let color = match program.category {
                    ComplianceCategory::Government => Color32::from_rgb(70, 130, 180),
                    ComplianceCategory::Industry => Color32::from_rgb(60, 179, 113),
                    ComplianceCategory::International => Color32::from_rgb(186, 85, 211),
                    ComplianceCategory::Framework => Color32::from_rgb(255, 140, 0),
                    ComplianceCategory::Custom(_) => Color32::from_rgb(128, 128, 128),
                };

                // Create tag with remove button
                ui.horizontal(|ui| {
                    let tag_response = ui.add(
                        egui::Label::new(
                            RichText::new(&program.display_name)
                                .color(Color32::WHITE)
                                .size(11.0)
                        )
                        .sense(egui::Sense::hover())
                    );
                    
                    // Add background to the tag
                    let rect = tag_response.rect.expand2(Vec2::new(6.0, 3.0));
                    ui.painter().rect_filled(
                        rect,
                        4.0,
                        color,
                    );
                    
                    // Add remove button
                    let remove_btn = ui.add_sized(
                        [16.0, 16.0],
                        egui::Button::new(RichText::new("×").size(10.0))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                    );
                    
                    if remove_btn.clicked() {
                        to_remove.push(program.id.clone());
                    }

                    // Add tooltip with details
                    if tag_response.hovered() {
                        tag_response.on_hover_ui(|ui| {
                            ui.label(format!("Program: {}", program.display_name));
                            ui.label(format!("Category: {}", program.category.display_name()));
                            ui.label(format!("Rules: {}", program.rule_count));
                            ui.label(&program.description);
                        });
                    }
                });
            }

            // Remove programs that were clicked for removal
            for program_id in to_remove {
                self.remove_program(&program_id);
            }
        });
    }

    /// Show the add programs popup window
    fn show_add_popup(&mut self, ui: &mut egui::Ui) {
        let mut popup_open = self.show_add_popup;
        
        egui::Window::new("Add Compliance Programs")
            .open(&mut popup_open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ui.ctx(), |ui| {
            ui.set_min_width(400.0);
            ui.set_max_width(600.0);
            ui.set_max_height(400.0);
            
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Add Compliance Programs").strong());
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            self.show_add_popup = false;
                        }
                    });
                });
                
                ui.separator();

                // Search bar
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                });

                // Category filter
                ui.horizontal(|ui| {
                    ui.label("Category:");
                    
                    let current_text = match &self.selected_category {
                        Some(cat) => cat.display_name(),
                        None => "All Categories",
                    };
                    
                    egui::ComboBox::from_id_salt("category_filter")
                        .selected_text(current_text)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.selected_category.is_none(), "All Categories").clicked() {
                                self.selected_category = None;
                            }
                            
                            for category in [
                                ComplianceCategory::Government,
                                ComplianceCategory::Industry, 
                                ComplianceCategory::International,
                                ComplianceCategory::Framework,
                            ] {
                                let is_selected = self.selected_category.as_ref()
                                    .map(|c| std::mem::discriminant(c) == std::mem::discriminant(&category))
                                    .unwrap_or(false);
                                
                                if ui.selectable_label(is_selected, category.display_name()).clicked() {
                                    self.selected_category = Some(category);
                                }
                            }
                        });
                });

                ui.separator();

                // Loading state
                if self.is_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading compliance programs...");
                    });
                    return;
                }

                // Error state
                if let Some(ref error) = self.loading_error {
                    let error_message = error.clone();
                    let mut retry_requested = false;
                    
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("⚠️ Failed to load compliance programs")
                                .color(Color32::from_rgb(220, 100, 50))
                                .strong()
                        );
                        ui.label(
                            RichText::new(&error_message)
                                .color(Color32::LIGHT_GRAY)
                                .size(11.0)
                        );
                        
                        // Show additional context about repository status
                        if let Some(repo_status) = self.get_repository_status_info() {
                            ui.label(
                                RichText::new(&repo_status)
                                    .color(Color32::LIGHT_BLUE)
                                    .size(10.0)
                                    .italics()
                            );
                        }
                        
                        ui.separator();
                    });
                    
                    if ui.button("🔄 Retry").clicked() {
                        retry_requested = true;
                    }
                    
                    if retry_requested {
                        // Reset the attempt flag to allow retry
                        self.has_attempted_load = false;
                        self.loading_error = None;
                        self.try_load_programs();
                    }
                    return;
                }

                // Empty state when no programs loaded yet
                if self.available_programs.is_empty() && !self.has_attempted_load {
                    let mut load_requested = false;
                    
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Click 'Load Programs' to discover compliance programs from GitHub")
                                .weak()
                                .italics()
                        );
                    });
                    
                    if ui.button("🌐 Load Programs").clicked() {
                        load_requested = true;
                    }
                    
                    if load_requested {
                        self.try_load_programs();
                    }
                    return;
                }

                // Program list
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        let filtered_programs = self.filter_programs();
                        
                        if filtered_programs.is_empty() {
                            ui.label(
                                RichText::new("No compliance programs match your search")
                                    .weak()
                                    .italics()
                            );
                        } else {
                            // Collect program modifications first
                            let mut programs_to_add = Vec::new();
                            let mut programs_to_remove = Vec::new();
                            
                            for program in &filtered_programs {
                                let is_selected = self.selected_programs.contains(&program.name);
                                let color = self.get_program_color(program);
                                
                                ui.horizontal(|ui| {
                                    // Category color indicator
                                    let color_rect = ui.allocate_response(Vec2::new(12.0, 12.0), egui::Sense::hover());
                                    ui.painter().rect_filled(
                                        color_rect.rect,
                                        2.0,
                                        color,
                                    );

                                    // Checkbox for selection with full program name as label
                                    let mut selected = is_selected;
                                    if ui.checkbox(&mut selected, &program.display_name).changed() {
                                        if selected {
                                            programs_to_add.push((*program).clone());
                                        } else {
                                            programs_to_remove.push(program.name.clone());
                                        }
                                    }
                                });
                                
                                ui.separator();
                            }
                            
                            // Apply modifications after iteration
                            for program in programs_to_add {
                                self.add_program(&program);
                            }
                            for program_name in programs_to_remove {
                                self.remove_program(&program_name);
                            }
                        }
                    });
            });
        });
        
        // Update the popup state
        self.show_add_popup = popup_open;
    }

}