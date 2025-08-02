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
            ui.horizontal(|ui| {
                ui.label(RichText::new("F1").strong());
                ui.label("- Help Assistant");
            });

            ui.add_space(15.0);

            // IAM Identity Center setup section
            ui.heading("AWS IAM Identity Center Setup");
            ui.add_space(5.0);

            ui.label("To use the AWS Dash login functionality, you need to set up a Permission Set in IAM Identity Center:");
            ui.add_space(10.0);

            ui.add_space(10.0);
            ui.label(RichText::new("Get started by:").strong());
            ui.label("1. Creating a new project (Space > Project > New)");
            ui.label("2. Login to AWS (Space > Login)");
            ui.label("3. Adding resources to your project");
            ui.label("4. Configuring resource properties");
            ui.label("5. Deploying CloudFormation");

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
