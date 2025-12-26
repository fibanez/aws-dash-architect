//! CloudTrail Events Viewer Window
//!
//! Displays CloudTrail events for AWS resources with search filtering.

#![warn(clippy::all, rust_2018_idioms)]

use super::window_focus::FocusableWindow;
use crate::app::data_plane::cloudtrail_events::{
    CloudTrailEvent, CloudTrailEventsClient, LookupResult,
};
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use chrono::{DateTime, Utc};
use eframe::egui;
use egui::{Color32, Context, RichText, Ui};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::sync::mpsc;
use std::sync::Arc;

/// Maximum number of events to display in the UI
const MAX_DISPLAY_EVENTS: usize = 1000;

/// Parameters for showing the CloudTrail Events window
#[derive(Clone)]
pub struct CloudTrailEventsShowParams {
    /// Resource type (for filtering)
    pub resource_type: String,

    /// Resource name (for display)
    pub resource_name: String,

    /// Resource ARN (optional, for filtering)
    pub resource_arn: Option<String>,

    /// AWS account ID
    pub account_id: String,

    /// AWS region
    pub region: String,
}

/// Result type for background loading
type LoadResult = Result<LookupResult, String>;

/// CloudTrail Events viewer window with async data loading
pub struct CloudTrailEventsWindow {
    /// Window open state
    pub open: bool,

    // Display parameters
    resource_type: String,
    resource_name: String,
    resource_arn: Option<String>,
    account_id: String,
    region: String,

    // State
    events: Vec<CloudTrailEvent>,
    search_filter: String,
    loading: bool,
    error_message: Option<String>,
    selected_event: Option<usize>,

    // Services
    client: Arc<CloudTrailEventsClient>,
    fuzzy_matcher: SkimMatcherV2,

    // Channel for receiving results from background thread
    receiver: mpsc::Receiver<LoadResult>,
    sender: mpsc::Sender<LoadResult>,
}

impl CloudTrailEventsWindow {
    /// Create new CloudTrail Events window
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            open: false,
            resource_type: String::new(),
            resource_name: String::new(),
            resource_arn: None,
            account_id: String::new(),
            region: String::new(),
            events: Vec::new(),
            search_filter: String::new(),
            loading: false,
            error_message: None,
            selected_event: None,
            client: Arc::new(CloudTrailEventsClient::new(credential_coordinator)),
            fuzzy_matcher: SkimMatcherV2::default(),
            receiver,
            sender,
        }
    }

    /// Open window for a specific resource
    pub fn open_for_resource(&mut self, params: CloudTrailEventsShowParams) {
        self.resource_type = params.resource_type;
        self.resource_name = params.resource_name;
        self.resource_arn = params.resource_arn;
        self.account_id = params.account_id;
        self.region = params.region;
        self.search_filter.clear();
        self.error_message = None;
        self.selected_event = None;
        self.open = true;

        // Load initial data
        self.refresh_events();
    }

    /// Refresh events from AWS
    fn refresh_events(&mut self) {
        self.loading = true;
        self.error_message = None;

        let client = Arc::clone(&self.client);
        let account_id = self.account_id.clone();
        let region = self.region.clone();
        let resource_type = self.resource_type.clone();
        let resource_arn = self.resource_arn.clone();
        let resource_name = self.resource_name.clone();
        let sender = self.sender.clone();

        // Create a new thread (since egui runs on a blocking thread) and run tokio inside it
        std::thread::spawn(move || {
            // Create a new tokio runtime for this thread
            let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

            // Run the async operation
            runtime.block_on(async move {
                // CloudTrail ResourceName filter expects resource ID (function name), NOT ARN
                // resource_name now contains the resource_id from tree.rs
                let resource_identifier = resource_name.as_str();

                log::info!(
                    "CloudTrail: Querying events with resource_type='{}', resource_identifier='{}' (ARN='{}'), account='{}', region='{}'",
                    resource_type,
                    resource_identifier,
                    resource_arn.as_deref().unwrap_or("<no-arn>"),
                    account_id,
                    region
                );

                let result = match client
                    .get_resource_events(&account_id, &region, &resource_type, Some(resource_identifier), 100)
                    .await
                {
                    Ok(result) => {
                        log::info!(
                            "Loaded {} CloudTrail events for resource type {}",
                            result.events.len(),
                            resource_type
                        );

                        // Log first event for debugging
                        if let Some(first_event) = result.events.first() {
                            log::info!(
                                "CloudTrail: First event - event_name='{}', username='{}', resources={:?}",
                                first_event.event_name,
                                first_event.username,
                                first_event.resources.iter()
                                    .map(|r| format!("{}:{}",
                                        r.resource_type.as_deref().unwrap_or("<no-type>"),
                                        r.resource_name.as_deref().unwrap_or("<no-name>")
                                    ))
                                    .collect::<Vec<_>>()
                            );
                        }

                        Ok(result)
                    }
                    Err(e) => {
                        log::error!("Failed to load CloudTrail events: {}", e);
                        Err(e.to_string())
                    }
                };

                let _ = sender.send(result);
            });
        });
    }

    /// Internal show implementation with optional focus
    fn show_internal(&mut self, ctx: &Context, bring_to_front: bool) {
        // Check for results from background thread
        if let Ok(result) = self.receiver.try_recv() {
            self.loading = false;
            match result {
                Ok(lookup_result) => {
                    let total_events = lookup_result.events.len();

                    // Client-side filtering: Match events that reference our specific resource
                    // CloudTrail may return events for multiple resources of the same type
                    let filtered_events: Vec<_> = lookup_result
                        .events
                        .into_iter()
                        .filter(|event| {
                            // Check if any resource in this event matches our target resource
                            event.resources.iter().any(|res| {
                                // Match by resource name (contains our resource_name)
                                // or by ARN (contains our ARN if available)
                                res.resource_name.as_ref().is_some_and(|name| {
                                    name.contains(&self.resource_name)
                                        || self
                                            .resource_arn
                                            .as_ref()
                                            .is_some_and(|arn| name.contains(arn))
                                })
                            })
                        })
                        .collect();

                    log::info!(
                        "CloudTrail: Client-side filtering: {} events matched out of {} total (resource_name='{}', arn='{}')",
                        filtered_events.len(),
                        total_events,
                        self.resource_name,
                        self.resource_arn.as_deref().unwrap_or("<no-arn>")
                    );

                    self.events = filtered_events;
                }
                Err(error) => {
                    self.error_message = Some(error);
                }
            }
        }

        // Store open state locally to avoid borrow checker issues
        let mut is_open = self.open;

        // Create window with unique ID per resource instance
        let mut window = egui::Window::new(format!("CloudTrail Events: {}", self.resource_name))
            .id(egui::Id::new((
                "cloudtrail_events_window",
                &self.resource_name,
                &self.account_id,
                &self.region,
            )))
            .open(&mut is_open)
            .default_width(1000.0)
            .default_height(700.0)
            .resizable(true)
            .collapsible(true);

        if bring_to_front {
            window = window.current_pos([100.0, 100.0]);
        }

        window.show(ctx, |ui| {
            self.render_ui(ui);
        });

        // Update open state from local variable
        self.open = is_open;
    }

    /// Render window UI
    fn render_ui(&mut self, ui: &mut Ui) {
        // Header with resource info
        ui.horizontal(|ui| {
            ui.label(RichText::new("Resource:").strong());
            ui.label(&self.resource_name);

            ui.separator();

            ui.label(RichText::new("Type:").strong());
            ui.label(&self.resource_type);

            ui.separator();

            ui.label(RichText::new("Account:").strong());
            ui.label(&self.account_id);

            ui.separator();

            ui.label(RichText::new("Region:").strong());
            ui.label(&self.region);
        });

        ui.separator();

        // Search and refresh controls
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_filter);

            if ui.button("Refresh").clicked() {
                self.refresh_events();
            }

            if ui.button("Clear").clicked() {
                self.search_filter.clear();
            }
        });

        ui.separator();

        // Loading indicator
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading CloudTrail events...");
            });
        }

        // Error message
        if let Some(error) = &self.error_message {
            ui.colored_label(Color32::RED, format!("Error: {}", error));
        }

        // Filter events based on search
        let filtered_events: Vec<_> = if self.search_filter.is_empty() {
            self.events.iter().enumerate().collect()
        } else {
            self.events
                .iter()
                .enumerate()
                .filter(|(_, event)| {
                    // Search across event name, username, event source
                    let search_text = format!(
                        "{} {} {}",
                        event.event_name, event.username, event.event_source
                    );
                    self.fuzzy_matcher
                        .fuzzy_match(&search_text, &self.search_filter)
                        .is_some()
                })
                .collect()
        };

        // Events display with scrolling
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 60.0)
            .show(ui, |ui| {
                for (_idx, event) in filtered_events.iter().take(MAX_DISPLAY_EVENTS) {
                    let has_error = event.error_code.is_some();

                    // Event header
                    ui.horizontal(|ui| {
                        // Timestamp
                        let timestamp = DateTime::from_timestamp(
                            event.event_time / 1000,
                            ((event.event_time % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());

                        ui.label(
                            RichText::new(timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
                                .color(Color32::GRAY)
                                .monospace(),
                        );

                        ui.separator();

                        // Event name (highlight errors in red)
                        let event_name_text = if has_error {
                            RichText::new(&event.event_name)
                                .color(Color32::from_rgb(255, 100, 100))
                                .strong()
                        } else {
                            RichText::new(&event.event_name).strong()
                        };
                        ui.label(event_name_text);

                        ui.separator();

                        // Username
                        ui.label(RichText::new("by").color(Color32::GRAY));
                        ui.label(&event.username);

                        // Error indicator
                        if has_error {
                            ui.separator();
                            if let Some(error_code) = &event.error_code {
                                ui.label(
                                    RichText::new(format!("Error: {}", error_code))
                                        .color(Color32::from_rgb(255, 100, 100)),
                                );
                            }
                        }
                    });

                    // Resources (if any)
                    if !event.resources.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Resources:").color(Color32::GRAY));
                            for resource in &event.resources {
                                if let Some(resource_name) = &resource.resource_name {
                                    ui.label(resource_name);
                                }
                                if let Some(resource_type) = &resource.resource_type {
                                    ui.label(
                                        RichText::new(format!("({})", resource_type))
                                            .color(Color32::GRAY),
                                    );
                                }
                            }
                        });
                    }

                    // Event details (always shown)
                    egui::Grid::new(format!("event_details_{}", event.event_id))
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Event ID:").strong());
                            ui.label(&event.event_id);
                            ui.end_row();

                            ui.label(RichText::new("Event Source:").strong());
                            ui.label(&event.event_source);
                            ui.end_row();

                            if let Some(access_key) = &event.access_key_id {
                                ui.label(RichText::new("Access Key:").strong());
                                ui.label(access_key);
                                ui.end_row();
                            }

                            if let Some(read_only) = &event.read_only {
                                ui.label(RichText::new("Read Only:").strong());
                                ui.label(read_only);
                                ui.end_row();
                            }

                            if let Some(error_msg) = &event.error_message {
                                ui.label(RichText::new("Error Message:").strong());
                                ui.label(
                                    RichText::new(error_msg)
                                        .color(Color32::from_rgb(255, 100, 100)),
                                );
                                ui.end_row();
                            }
                        });

                    // Full CloudTrail event JSON (if available)
                    if let Some(ct_event) = &event.cloud_trail_event {
                        ui.label(RichText::new("Full CloudTrail Event:").strong());

                        // Parse and pretty-print JSON
                        let formatted_json =
                            match serde_json::from_str::<serde_json::Value>(ct_event) {
                                Ok(json_value) => serde_json::to_string_pretty(&json_value)
                                    .unwrap_or_else(|_| ct_event.clone()),
                                Err(_) => ct_event.clone(), // If parsing fails, show original
                            };

                        // Use smaller, consistent height for JSON display (200px = ~5 lines)
                        // This prevents first event from being huge and keeps all events uniform
                        egui::ScrollArea::vertical()
                            .id_salt(format!("json_scroll_{}", event.event_id))
                            .max_height(200.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&formatted_json)
                                        .monospace()
                                        .color(Color32::GRAY),
                                );
                            });
                    }

                    // Separator between events
                    ui.separator();
                }

                // Warning if too many events
                if filtered_events.len() > MAX_DISPLAY_EVENTS {
                    ui.colored_label(
                        Color32::YELLOW,
                        format!(
                            "Showing first {} of {} events (refine your search)",
                            MAX_DISPLAY_EVENTS,
                            filtered_events.len()
                        ),
                    );
                }
            });

        // Footer with event count
        ui.separator();
        ui.horizontal(|ui| {
            let showing = filtered_events.len().min(MAX_DISPLAY_EVENTS);
            let total = self.events.len();

            if self.search_filter.is_empty() {
                ui.label(format!("Showing {} events", showing));
            } else {
                ui.label(format!(
                    "Showing {} of {} events (filtered from {} total)",
                    showing,
                    filtered_events.len(),
                    total
                ));
            }

            // Event type summary
            let failed_count = self
                .events
                .iter()
                .filter(|e| e.error_code.is_some())
                .count();
            if failed_count > 0 {
                ui.separator();
                ui.label(
                    RichText::new(format!("{} failed API calls", failed_count))
                        .color(Color32::from_rgb(255, 100, 100)),
                );
            }
        });
    }
}

impl CloudTrailEventsWindow {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub fn show(&mut self, ctx: &Context) {
        self.show_internal(ctx, false);
    }
}

impl FocusableWindow for CloudTrailEventsWindow {
    type ShowParams = CloudTrailEventsShowParams;

    fn window_id(&self) -> &'static str {
        "cloudtrail_events_window"
    }

    fn window_title(&self) -> String {
        format!("CloudTrail Events: {}", self.resource_name)
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn show_with_focus(&mut self, ctx: &Context, params: Self::ShowParams, bring_to_front: bool) {
        // First open for resource
        self.open_for_resource(params);

        // Then show with optional focus
        self.show_internal(ctx, bring_to_front);
    }
}
