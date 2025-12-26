use egui::{self, Align2, Context, FontId, Id, Pos2, Rect, RichText, Vec2};

// Define an enum for main command palette actions
pub enum CommandAction {
    Login,
    AWSExplorer,  // AWS resource explorer
    AgentManager, // Agent Manager for managing multiple agents
    Quit,
}

// Command structure for the command palette
struct CommandEntry {
    key: egui::Key,
    key_char: char,
    label: &'static str,
    color: egui::Color32,
    description: &'static str,
}

#[derive(Default)]
pub struct CommandPalette {
    pub show: bool,
    palette_dimensions: Option<PaletteDimensions>,
    needs_recalculation: bool,
}

// Store the calculated dimensions to avoid recalculation on every frame
#[derive(Clone)]
struct PaletteDimensions {
    window_width: f32,
    window_height: f32,
    window_pos: Pos2,
    column_width: f32,
    column_spacing: f32,
    left_margin: f32,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            show: false,
            palette_dimensions: None,
            needs_recalculation: true,
        }
    }

    // Calculate dimensions based on screen size
    fn calculate_dimensions(&mut self, ctx: &Context) {
        let screen_rect = ctx.screen_rect();

        // Use almost full screen width (90% with margins)
        let window_width = screen_rect.width() * 0.9; // 90% of screen width
        let window_height = screen_rect.height() * 0.25; // 1/4 of screen height

        // Position at bottom of screen, centered horizontally
        let window_pos = Pos2::new(
            screen_rect.center().x - (window_width / 2.0),
            screen_rect.max.y - window_height - 20.0, // 20px margin from bottom
        );

        // Calculate column and spacing based on window width
        let column_width = (window_width * 0.35).min(400.0); // 35% of width, max 400px
        let column_spacing = window_width * 0.1; // 10% of width for spacing
        let left_margin = (window_width - (2.0 * column_width + column_spacing)) / 2.0;

        self.palette_dimensions = Some(PaletteDimensions {
            window_width,
            window_height,
            window_pos,
            column_width,
            column_spacing,
            left_margin,
        });

        self.needs_recalculation = false;
    }

    // Respond to events that should trigger recalculation
    pub fn on_window_resized(&mut self) {
        self.needs_recalculation = true;
    }

    pub fn on_font_size_changed(&mut self) {
        self.needs_recalculation = true;
    }

    // Helper to draw a styled command button
    fn draw_command_button(
        &self,
        ui: &mut egui::Ui,
        cmd: &CommandEntry,
        clicked: &mut bool,
        key_pressed: bool,
    ) {
        ui.horizontal(|ui| {
            // Key in circle with color
            let circle_size = Vec2::new(32.0, 32.0);
            let (rect, response) = ui.allocate_exact_size(circle_size, egui::Sense::click());

            if ui.is_rect_visible(rect) {
                let visuals = ui.style().interact(&response);
                let circle_stroke = egui::Stroke::new(1.5, visuals.fg_stroke.color);

                // Draw colored circle
                ui.painter().circle(
                    rect.center(),
                    rect.width() / 2.0,
                    cmd.color.linear_multiply(0.8), // Slightly darker
                    circle_stroke,
                );

                // Draw key character centered in circle
                let text = cmd.key_char.to_string();

                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    text,
                    FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );
            }

            ui.add_space(8.0);

            // Command information with arrow
            ui.vertical(|ui| {
                // Label with color
                ui.label(
                    RichText::new(cmd.label)
                        .size(16.0)
                        .color(cmd.color)
                        .strong(),
                );
                // Description in smaller text
                ui.label(RichText::new(cmd.description).size(13.0).weak());
            });

            if response.clicked() || key_pressed {
                *clicked = true;
            }
        });
    }

    pub fn show(&mut self, ctx: &Context) -> Option<CommandAction> {
        self.show_with_offset(ctx, Vec2::ZERO)
    }

    pub fn show_with_offset(&mut self, ctx: &Context, offset: Vec2) -> Option<CommandAction> {
        if !self.show {
            return None;
        }

        let mut result = None;

        // Calculate dimensions when showing for the first time or after resize/font change
        if self.needs_recalculation || self.palette_dimensions.is_none() {
            self.calculate_dimensions(ctx);
        }

        // Clone the dimensions for use in closures
        let mut dimensions = self.palette_dimensions.as_ref().unwrap().clone();
        // Apply offset to window position
        dimensions.window_pos += offset;

        // Command entries with colors and descriptions
        let commands = [
            CommandEntry {
                key: egui::Key::L,
                key_char: 'L',
                label: "Login AWS",
                color: egui::Color32::from_rgb(255, 190, 70), // Orange
                description: "Login to AWS Identity Center",
            },
            CommandEntry {
                key: egui::Key::E,
                key_char: 'E',
                label: "AWS Explorer",
                color: egui::Color32::from_rgb(255, 140, 70), // Orange-Red
                description: "Explore AWS resources across accounts",
            },
            CommandEntry {
                key: egui::Key::M,
                key_char: 'M',
                label: "Agent Manager",
                color: egui::Color32::from_rgb(100, 180, 220), // Light Blue
                description: "Manage multiple AI agents",
            },
            CommandEntry {
                key: egui::Key::Q,
                key_char: 'Q',
                label: "Quit",
                color: egui::Color32::from_rgb(240, 130, 130), // Red
                description: "Exit the application",
            },
        ];

        // Create window with calculated dimensions
        let window_size = Vec2::new(dimensions.window_width, dimensions.window_height);

        egui::Area::new(Id::new("command_palette"))
            .fixed_pos(dimensions.window_pos)
            .movable(false)
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(ui.style().visuals.extreme_bg_color)
                    .stroke(egui::Stroke::new(
                        1.5,
                        ui.style().visuals.widgets.active.bg_fill,
                    ))
                    .inner_margin(egui::Margin {
                        left: 25,
                        right: 25,
                        top: 20,
                        bottom: 20,
                    })
                    .corner_radius(8.0);

                frame.show(ui, |ui| {
                    ui.set_min_size(window_size);

                    // Skip the title, just add a small top space
                    ui.add_space(10.0);

                    // Two column layout with calculated positions
                    ui.horizontal(|ui| {
                        // Add left margin
                        ui.add_space(dimensions.left_margin);

                        // First column
                        ui.vertical(|ui| {
                            ui.set_width(dimensions.column_width);
                            // Calculate ceiling division
                            #[allow(clippy::manual_div_ceil)]
                            let mid = (commands.len() + 1) / 2;

                            for cmd in commands.iter().take(mid) {
                                let mut clicked = false;
                                let key_pressed = ctx.input(|input| input.key_pressed(cmd.key));

                                self.draw_command_button(ui, cmd, &mut clicked, key_pressed);

                                // Handle actions for first column
                                if clicked || key_pressed {
                                    self.show = false;
                                    match cmd.key {
                                        egui::Key::L => result = Some(CommandAction::Login),
                                        egui::Key::E => result = Some(CommandAction::AWSExplorer),
                                        egui::Key::M => result = Some(CommandAction::AgentManager),
                                        egui::Key::Q => result = Some(CommandAction::Quit),
                                        _ => {}
                                    }
                                }

                                ui.add_space(20.0);
                            }
                        });

                        // Add calculated spacing between columns
                        ui.add_space(dimensions.column_spacing);

                        // Second column
                        ui.vertical(|ui| {
                            ui.set_width(dimensions.column_width);
                            // Calculate ceiling division
                            #[allow(clippy::manual_div_ceil)]
                            let mid = (commands.len() + 1) / 2;

                            for cmd in commands.iter().skip(mid) {
                                let mut clicked = false;
                                let key_pressed = ctx.input(|input| input.key_pressed(cmd.key));

                                self.draw_command_button(ui, cmd, &mut clicked, key_pressed);

                                // Handle action for second column
                                if clicked || key_pressed {
                                    self.show = false;
                                    match cmd.key {
                                        egui::Key::L => result = Some(CommandAction::Login),
                                        egui::Key::E => result = Some(CommandAction::AWSExplorer),
                                        egui::Key::M => result = Some(CommandAction::AgentManager),
                                        egui::Key::Q => result = Some(CommandAction::Quit),
                                        _ => {}
                                    }
                                }

                                ui.add_space(20.0);
                            }
                        });
                    });
                });
            });

        // Close palette if clicking outside
        if ctx.input(|i| i.pointer.any_click()) {
            let mouse_pos = ctx.input(|i| i.pointer.interact_pos());
            if let Some(pos) = mouse_pos {
                let rect = Rect::from_min_size(dimensions.window_pos, window_size);
                if !rect.contains(pos) {
                    self.show = false;
                }
            }
        }

        result
    }
}
