use eframe::egui::{self, Color32, RichText};
use std::collections::HashMap;

/// Represents information about an open window
#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub is_visible: bool,
    pub window_type: WindowType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowType {
    ResourceForm,
    PropertyTypeForm,
    ValueEditor,
    ReferenceePicker,
    ResourceGraph,
    CloudFormationScene,
    ResourceDetails,
    LogWindow,
    HelpWindow,
    Chat,
    ResourceTypes,
    TemplateSection,
    ResourceJsonEditor,
    Other(String),
}

impl WindowType {
    pub fn icon(&self) -> &'static str {
        match self {
            WindowType::ResourceForm => "📝",
            WindowType::PropertyTypeForm => "🔧",
            WindowType::ValueEditor => "✏️",
            WindowType::ReferenceePicker => "🔗",
            WindowType::ResourceGraph => "🕸️",
            WindowType::CloudFormationScene => "🎭",
            WindowType::ResourceDetails => "📊",
            WindowType::LogWindow => "📜",
            WindowType::HelpWindow => "❓",
            WindowType::Chat => "💬",
            WindowType::ResourceTypes => "📚",
            WindowType::TemplateSection => "📋",
            WindowType::ResourceJsonEditor => "🗂️",
            WindowType::Other(_) => "🪟",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            WindowType::ResourceForm => Color32::from_rgb(40, 140, 60),
            WindowType::PropertyTypeForm => Color32::from_rgb(60, 180, 200),
            WindowType::ValueEditor => Color32::from_rgb(100, 170, 255),
            WindowType::ReferenceePicker => Color32::from_rgb(255, 190, 70),
            WindowType::ResourceGraph => Color32::from_rgb(120, 200, 80),
            WindowType::CloudFormationScene => Color32::from_rgb(180, 140, 220),
            WindowType::ResourceDetails => Color32::from_rgb(140, 200, 170),
            WindowType::LogWindow => Color32::from_rgb(200, 160, 100),
            WindowType::HelpWindow => Color32::from_rgb(240, 130, 130),
            WindowType::Chat => Color32::from_rgb(200, 150, 255),
            WindowType::ResourceTypes => Color32::from_rgb(150, 200, 255),
            WindowType::TemplateSection => Color32::from_rgb(255, 180, 150),
            WindowType::ResourceJsonEditor => Color32::from_rgb(180, 255, 180),
            WindowType::Other(_) => Color32::from_rgb(180, 180, 180),
        }
    }
}

/// Window selector that shows a list of open windows
#[derive(Default)]
pub struct WindowSelector {
    pub show: bool,
    windows: HashMap<String, WindowInfo>,
}

impl WindowSelector {
    pub fn new() -> Self {
        Self {
            show: false,
            windows: HashMap::new(),
        }
    }

    /// Register a window as open
    pub fn register_window(&mut self, id: String, title: String, window_type: WindowType) {
        self.windows.insert(
            id.clone(),
            WindowInfo {
                id,
                title,
                is_visible: true,
                window_type,
            },
        );
    }

    /// Mark a window as closed
    pub fn unregister_window(&mut self, id: &str) {
        self.windows.remove(id);
    }

    /// Update window visibility status
    pub fn update_window_visibility(&mut self, id: &str, is_visible: bool) {
        if let Some(window_info) = self.windows.get_mut(id) {
            window_info.is_visible = is_visible;
        }
    }

    /// Show the window selector as a menu button
    pub fn show_menu(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut selected_window = None;

        ui.menu_button(RichText::new("🪟").size(16.0), |ui| {
            ui.set_min_width(250.0); // Make menu wider for better readability

            if self.windows.is_empty() {
                ui.label(RichText::new("No windows open").weak());
            } else {
                // Group windows by visibility
                let mut visible_windows = Vec::new();
                let mut hidden_windows = Vec::new();

                for (window_id, window_info) in &self.windows {
                    if window_info.is_visible {
                        visible_windows.push((window_id, window_info));
                    } else {
                        hidden_windows.push((window_id, window_info));
                    }
                }

                // Show visible windows first
                if !visible_windows.is_empty() {
                    ui.label(RichText::new("Visible Windows").strong());
                    ui.separator();

                    for (window_id, window_info) in &visible_windows {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(window_info.window_type.icon()).size(16.0));

                            if ui.button(&window_info.title).clicked() {
                                selected_window = Some((*window_id).clone());
                            }
                        });
                    }
                }

                // Show hidden windows if any
                if !hidden_windows.is_empty() {
                    if !visible_windows.is_empty() {
                        ui.add_space(8.0);
                    }
                    ui.label(RichText::new("Hidden Windows").weak());
                    ui.separator();

                    for (window_id, window_info) in &hidden_windows {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(window_info.window_type.icon())
                                    .size(16.0)
                                    .weak(),
                            );

                            if ui
                                .button(RichText::new(&window_info.title).weak())
                                .clicked()
                            {
                                selected_window = Some((*window_id).clone());
                            }
                        });
                    }
                }
            }
        });

        selected_window
    }

    /// Open the window selector
    pub fn open(&mut self) {
        self.show = true;
    }

    /// Get the list of registered windows for debugging
    pub fn get_windows(&self) -> &HashMap<String, WindowInfo> {
        &self.windows
    }
}
