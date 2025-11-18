use super::window_focus::FocusableWindow;
use eframe::egui;
use egui::{Context, RichText, Ui};

#[derive(Default)]
pub struct HelpWindow {
    pub open: bool,
}

impl HelpWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &Context) {
        self.show_with_offset(ctx, egui::Vec2::ZERO);
    }

    pub fn show_with_focus(&mut self, ctx: &Context, bring_to_front: bool) {
        self.show_with_offset_and_focus(ctx, egui::Vec2::ZERO, bring_to_front);
    }

    pub fn show_with_offset(&mut self, ctx: &Context, offset: egui::Vec2) {
        self.show_with_offset_and_focus(ctx, offset, false);
    }

    pub fn show_with_offset_and_focus(
        &mut self,
        ctx: &Context,
        offset: egui::Vec2,
        bring_to_front: bool,
    ) {
        if !self.open {
            return;
        }

        let central_panel_size = ctx.available_rect().size();
        let window_width = central_panel_size.x.min(600.0);
        let window_height = central_panel_size.y.min(500.0);

        let mut window = egui::Window::new("Help")
            .fixed_size([window_width, window_height])
            .anchor(egui::Align2::CENTER_CENTER, offset)
            .resizable(false)
            .collapsible(false);

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            self.ui_content(ui);
        });
    }

    fn ui_content(&self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(5.0);

            // Keyboard shortcuts section
            ui.heading("Keyboard Shortcuts");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Space").strong());
                ui.label("- Open command palette");
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Escape").strong());
                ui.label("- Close current window");
            });

            ui.add_space(15.0);

            // Command Palette section
            ui.heading("Command Palette");
            ui.add_space(5.0);

            ui.label("Press Space to open the command palette, then:");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("L").strong());
                ui.label("- Login to AWS Identity Center");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("E").strong());
                ui.label("- Open AWS Resource Explorer");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("M").strong());
                ui.label("- Open Agent Manager (AI Assistants)");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Q").strong());
                ui.label("- Quit application");
            });

            ui.add_space(15.0);

            // IAM Identity Center setup section
            ui.heading("AWS IAM Identity Center Setup");
            ui.add_space(5.0);

            ui.label("To use AWS Dash, you need to set up IAM Identity Center:");
            ui.add_space(10.0);

            ui.label("1. Login to AWS (Space > Login)");
            ui.label("2. Explore AWS resources across your accounts (Space > AWS Explorer)");
            ui.label("3. Use the AI assistant for infrastructure operations (Space > Agent Manager)");

            ui.add_space(20.0);
        });
    }
}

impl FocusableWindow for HelpWindow {
    type ShowParams = super::window_focus::SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "help_window"
    }

    fn window_title(&self) -> String {
        "Help".to_string()
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        _params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // Call the existing show_with_focus method
        HelpWindow::show_with_focus(self, ctx, bring_to_front);
    }
}
