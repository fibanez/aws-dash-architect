//! Keyboard input, navigation, and hint mode handling

use super::{DashApp, FocusedWindow, PendingWidgetAction};
use crate::app::dashui::window_focus::FocusableWindow;
use crate::app::dashui::{ElementAction, KeyEventResult, NavigationCommand, NavigationMode};
use eframe::egui;
use tracing::info;

impl DashApp {
    /// Handle keyboard input for command palette and window closing
    pub(super) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        // Process navigation system first (only when UI doesn't want keyboard input)
        if !ctx.wants_keyboard_input() {
            self.handle_navigation_input(ctx);
        }

        // Legacy keybindings for backwards compatibility
        self.handle_legacy_keyboard_input(ctx);
    }

    /// Handle keyboard input through the navigation system
    pub(super) fn handle_navigation_input(&mut self, ctx: &egui::Context) {
        // Process all pending input events
        ctx.input(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        // Handle space bar for command palette (bypasses all Vimium navigation)
                        // This works in any mode: Normal, Insert, Hint, Visual, or Command
                        if *key == egui::Key::Space && modifiers.is_none() {
                            info!("Space bar pressed - opening command palette");
                            self.show_command_palette = true;
                            self.set_focused_window(FocusedWindow::CommandPalette);
                            continue; // Skip further processing for space bar
                        }

                        // Handle hint mode input first if active
                        if self.hint_mode.is_active() {
                            self.handle_hint_mode_input(event);
                        } else {
                            let result = self.navigation_state.process_key_event(event, ctx);
                            self.handle_navigation_result(result);
                        }
                    }
                    egui::Event::Text(_) => {
                        // Handle hint mode input first if active
                        if self.hint_mode.is_active() {
                            self.handle_hint_mode_input(event);
                        } else {
                            let result = self.navigation_state.process_key_event(event, ctx);
                            self.handle_navigation_result(result);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// Handle the result of navigation key processing
    pub(super) fn handle_navigation_result(&mut self, result: KeyEventResult) {
        match result {
            KeyEventResult::Handled => {
                // Navigation system handled the key, do nothing
            }
            KeyEventResult::PassThrough => {
                // Let the UI handle the key normally
            }
            KeyEventResult::ModeChanged(new_mode) => {
                // Mode changed, update UI feedback if needed
                info!("Navigation mode changed to: {:?}", new_mode);
            }
            KeyEventResult::Command(command) => {
                self.execute_navigation_command(command);
            }
            KeyEventResult::Cancel => {
                // Return to normal mode
                self.navigation_state.set_mode(NavigationMode::Normal);
            }
        }
    }

    /// Handle input when hint mode is active
    pub(super) fn handle_hint_mode_input(&mut self, event: &egui::Event) {
        // Check if we should skip this input (to prevent activation key double-processing)
        if self.skip_next_hint_input {
            self.skip_next_hint_input = false;
            info!("Skipping hint input to prevent activation key double-processing");
            return;
        }

        match event {
            egui::Event::Key {
                key, pressed: true, ..
            } => {
                match key {
                    egui::Key::Escape => {
                        // Exit hint mode
                        self.hint_mode.stop();
                        self.navigation_state.pop_mode();
                        info!("Exited hint mode");
                    }
                    egui::Key::Backspace => {
                        // Remove last filter character
                        self.hint_mode.remove_filter_char();
                    }
                    egui::Key::Enter => {
                        // Activate exact match if available
                        if let Some(element_id) = self.hint_mode.get_exact_match_element_id() {
                            self.activate_hint_element(&element_id);
                        }
                    }
                    _ => {
                        // Try to convert key to character for filtering
                        if let Some(ch) = self.key_to_char(*key) {
                            self.hint_mode.add_filter_char(ch);

                            // Check for exact match
                            if let Some(element_id) = self.hint_mode.get_exact_match_element_id() {
                                self.activate_hint_element(&element_id);
                            }
                        }
                    }
                }
            }
            egui::Event::Text(text) => {
                // Handle text input for hint filtering
                for ch in text.chars() {
                    self.hint_mode.add_filter_char(ch);
                }

                // Check for exact match after text input
                if let Some(element_id) = self.hint_mode.get_exact_match_element_id() {
                    self.activate_hint_element(&element_id);
                }
            }
            _ => {}
        }
    }

    /// Activate a hint element by its ID
    pub(super) fn activate_hint_element(&mut self, element_id: &str) {
        info!("Activating hint element: {}", element_id);

        // Get the resolved action from the hint marker, not the current mode action
        let action = if let Some(hint) = self.hint_mode.has_exact_match() {
            hint.action // Use resolved action from hint (Smart actions are already resolved here)
        } else {
            self.hint_mode.current_action() // Fallback to current action
        };

        info!(
            "Using resolved action: {:?} for element: {}",
            action, element_id
        );

        // Handle ResourceFormWindow-specific elements (legacy prefix-based routing)
        if element_id.starts_with("resource_form_") {
            self.handle_resource_form_element_activation(element_id, action);
        } else if element_id.starts_with("template_sections_") {
            self.handle_template_sections_element_activation(element_id, action);
        } else if self.is_resource_form_element(element_id) {
            // Handle ResourceFormWindow elements (new: real widget integration)
            info!(
                "Element routing debug: id='{}' identified as ResourceForm element",
                element_id
            );
            self.queue_resource_form_action(element_id, action);
        } else if self.is_property_type_form_element(element_id) {
            // Handle PropertyTypeFormWindow elements (new: real widget integration)
            info!(
                "Element routing debug: id='{}' identified as PropertyTypeFormWindow element",
                element_id
            );
            self.queue_property_type_form_action(element_id, action);
        } else {
            // Handle real widget integration - queue action for execution (TemplateSections and others)
            info!("Queueing action {:?} for element: {}", action, element_id);

            // Also handle immediate actions that don't require widget interaction
            match action {
                ElementAction::Copy => {
                    info!("Copying text from element: {}", element_id);
                    // Real implementation would copy actual element text to system clipboard
                    info!("Copied text from '{}' to clipboard", element_id);
                }
                _ => {
                    // Other actions will be handled when the widget is rendered
                }
            }
        }

        // Exit hint mode after activation
        self.hint_mode.stop();
        self.navigation_state.pop_mode();
    }

    /// Handle activation of ResourceFormWindow elements
    pub(super) fn handle_resource_form_element_activation(&mut self, element_id: &str, action: ElementAction) {
        info!(
            "Activating ResourceFormWindow element: {} with action: {:?}",
            element_id, action
        );

        // Resolve Smart action to specific action based on element type
        let resolved_action = if action == ElementAction::Smart {
            // Determine appropriate action based on element ID patterns
            if element_id.contains("_button") {
                ElementAction::Click
            } else if element_id.contains("_input")
                || element_id.contains("_id")
                || element_id.contains("_field")
            {
                ElementAction::Focus
            } else {
                ElementAction::Activate
            }
        } else {
            action
        };

        info!(
            "Using resolved action: {:?} for ResourceForm element: {}",
            resolved_action, element_id
        );

        // Parse element type from ID
        if element_id.contains("_save_button") {
            match resolved_action {
                ElementAction::Click | ElementAction::Activate => {
                    info!("ResourceForm: Save button activated");
                    // In a full implementation, this would trigger the save logic
                    // For now, just log the action
                }
                _ => {
                    info!(
                        "ResourceForm: Save button - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else if element_id.contains("_cancel_button") {
            match resolved_action {
                ElementAction::Click | ElementAction::Activate => {
                    info!("ResourceForm: Cancel button activated");
                    // Resource/template editor windows removed
                }
                _ => {
                    info!(
                        "ResourceForm: Cancel button - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else if element_id.contains("_resource_id") {
            match resolved_action {
                ElementAction::Focus => {
                    info!("ResourceForm: Focusing Resource ID field");
                    // In a full implementation, this would focus the text input
                }
                ElementAction::Copy => {
                    // Resource/template editor windows removed
                    // In a full implementation, this would copy to clipboard
                }
                _ => {
                    info!(
                        "ResourceForm: Resource ID field - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else {
            // Handle property fields
            match resolved_action {
                ElementAction::Focus => {
                    info!("ResourceForm: Focusing property field: {}", element_id);
                    // In a full implementation, this would focus the specific property field
                }
                ElementAction::Copy => {
                    info!("ResourceForm: Copying property value from: {}", element_id);
                    // In a full implementation, this would copy the property value
                }
                _ => {
                    info!(
                        "ResourceForm: Property field {} - action {:?} not supported",
                        element_id, resolved_action
                    );
                }
            }
        }
    }

    /// Handle activation of TemplateSectionsWindow elements
    pub(super) fn handle_template_sections_element_activation(
        &mut self,
        element_id: &str,
        action: ElementAction,
    ) {
        info!(
            "Activating TemplateSectionsWindow element: {} with action: {:?}",
            element_id, action
        );

        // Resolve Smart action to specific action based on element type
        let resolved_action = if action == ElementAction::Smart {
            // Determine appropriate action based on element ID patterns
            if element_id.contains("_button")
                || element_id.contains("_resource_")
                || element_id.contains("_section_")
            {
                ElementAction::Click
            } else if element_id.contains("_filter")
                || element_id.contains("_input")
                || element_id.contains("_field")
            {
                ElementAction::Focus
            } else {
                ElementAction::Activate
            }
        } else {
            action
        };

        info!(
            "Using resolved action: {:?} for TemplateSections element: {}",
            resolved_action, element_id
        );

        // Parse element type from ID and queue the appropriate action
        if element_id.contains("_edit_resource_") {
            // Extract resource ID from element_id (format: "template_sections_edit_resource_{resource_id}")
            if let Some(resource_id) = element_id.strip_prefix("template_sections_edit_resource_") {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!(
                            "TemplateSections: Edit resource button activated for: {}",
                            resource_id
                        );
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ClickButton(format!(
                                "edit_resource_{}",
                                resource_id
                            )));
                    }
                    _ => {
                        info!(
                            "TemplateSections: Edit resource button - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_json_resource_") {
            // Extract resource ID from element_id
            if let Some(resource_id) = element_id.strip_prefix("template_sections_json_resource_") {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!(
                            "TemplateSections: JSON resource button activated for: {}",
                            resource_id
                        );
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ClickButton(format!(
                                "json_resource_{}",
                                resource_id
                            )));
                    }
                    _ => {
                        info!(
                            "TemplateSections: JSON resource button - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_delete_resource_") {
            // Extract resource ID from element_id
            if let Some(resource_id) = element_id.strip_prefix("template_sections_delete_resource_")
            {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!(
                            "TemplateSections: Delete resource button activated for: {}",
                            resource_id
                        );
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ClickButton(format!(
                                "delete_resource_{}",
                                resource_id
                            )));
                    }
                    _ => {
                        info!(
                            "TemplateSections: Delete resource button - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_section_") {
            // Handle section tab activation
            if let Some(section_name) = element_id.strip_prefix("template_sections_section_") {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!("TemplateSections: Section tab activated: {}", section_name);
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ActivateSection(
                                section_name.to_string(),
                            ));
                    }
                    _ => {
                        info!(
                            "TemplateSections: Section tab - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_resource_filter") {
            match resolved_action {
                ElementAction::Focus => {
                    info!("TemplateSections: Focusing resource filter field");
                    self.pending_widget_actions
                        .push(PendingWidgetAction::FocusTextInput(
                            "resource_filter".to_string(),
                        ));
                }
                ElementAction::Copy => {
                    // Resource/template editor windows removed
                    // In a full implementation, this would copy to clipboard
                }
                _ => {
                    info!(
                        "TemplateSections: Resource filter - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else {
            info!(
                "TemplateSections: Unknown element activated: {}",
                element_id
            );
        }
    }

    /// Helper to detect if an element belongs to ResourceFormWindow
    pub(super) fn is_resource_form_element(&self, _element_id: &str) -> bool {
        // Resource/template editor windows removed
        false
    }

    /// Helper to detect if an element belongs to PropertyTypeFormWindow
    pub(super) fn is_property_type_form_element(&self, element_id: &str) -> bool {
        // PropertyTypeFormWindow elements include:
        // - property_form_cancel_button, property_form_apply_button (buttons)
        // - property_input_{prop_name} (text inputs)
        element_id == "property_form_cancel_button"
            || element_id == "property_form_apply_button"
            || element_id.starts_with("property_input_")
    }

    /// Queue action on ResourceFormWindow
    pub(super) fn queue_resource_form_action(&mut self, _element_id: &str, _action: ElementAction) {
        // Resource/template editor windows removed
    }

    /// Queue action on PropertyTypeFormWindow (via ResourceFormWindow)
    pub(super) fn queue_property_type_form_action(&mut self, _element_id: &str, _action: ElementAction) {
        // Resource/template editor windows removed
    }

    /// Process pending widget actions queued from hint activation
    pub(super) fn process_pending_widget_actions(&mut self) {
        let actions = std::mem::take(&mut self.pending_widget_actions);

        for action in actions {
            match action {
                PendingWidgetAction::ClickButton(button_id) => {
                    info!("Processing pending click action for button: {}", button_id);
                    // This will be handled by the actual UI rendering when it creates the button
                    // For now, we store it for the template sections window to check
                    if button_id.starts_with("edit_resource_") {
                        // Extract resource ID and trigger edit
                        if let Some(resource_id) = button_id.strip_prefix("edit_resource_") {
                            info!("Triggering edit for resource: {}", resource_id);
                            // TODO: Actually trigger the edit action in template sections window
                        }
                    }
                }
                PendingWidgetAction::FocusTextInput(input_id) => {
                    info!(
                        "Processing pending focus action for text input: {}",
                        input_id
                    );
                    // This will be handled when the UI renders the text input
                }
                PendingWidgetAction::SelectListItem(item_id) => {
                    info!(
                        "Processing pending select action for list item: {}",
                        item_id
                    );
                    // This will be handled when the UI renders the list
                }
                PendingWidgetAction::ActivateSection(section_name) => {
                    info!("Processing pending section activation: {}", section_name);
                    // TODO: Actually change the selected section in template sections window
                }
            }
        }
    }

    /// Convert a key to character for hint filtering
    pub(super) fn key_to_char(&self, key: egui::Key) -> Option<char> {
        match key {
            egui::Key::A => Some('a'),
            egui::Key::B => Some('b'),
            egui::Key::C => Some('c'),
            egui::Key::D => Some('d'),
            egui::Key::E => Some('e'),
            egui::Key::F => Some('f'),
            egui::Key::G => Some('g'),
            egui::Key::H => Some('h'),
            egui::Key::I => Some('i'),
            egui::Key::J => Some('j'),
            egui::Key::K => Some('k'),
            egui::Key::L => Some('l'),
            egui::Key::M => Some('m'),
            egui::Key::N => Some('n'),
            egui::Key::O => Some('o'),
            egui::Key::P => Some('p'),
            egui::Key::Q => Some('q'),
            egui::Key::R => Some('r'),
            egui::Key::S => Some('s'),
            egui::Key::T => Some('t'),
            egui::Key::U => Some('u'),
            egui::Key::V => Some('v'),
            egui::Key::W => Some('w'),
            egui::Key::X => Some('x'),
            egui::Key::Y => Some('y'),
            egui::Key::Z => Some('z'),
            _ => None,
        }
    }

    /// Execute a navigation command
    pub(super) fn execute_navigation_command(&mut self, command: NavigationCommand) {
        match command {
            NavigationCommand::ScrollVertical(amount) => {
                self.handle_scroll_command(amount, false);
            }
            NavigationCommand::ScrollHorizontal(amount) => {
                self.handle_scroll_command(amount, true);
            }
            NavigationCommand::NextWindow => {
                self.focus_next_window();
            }
            NavigationCommand::PreviousWindow => {
                self.focus_previous_window();
            }
            NavigationCommand::CloseWindow => {
                self.close_focused_window();
            }
            NavigationCommand::WindowByIndex(index) => {
                self.focus_window_by_index(index);
            }
            NavigationCommand::LastWindow => {
                self.focus_last_window();
            }
            NavigationCommand::MoveToTop => {
                // Scroll to top (large negative scroll)
                self.handle_scroll_command(-1000, false);
            }
            NavigationCommand::MoveToBottom => {
                // Scroll to bottom (large positive scroll)
                self.handle_scroll_command(1000, false);
            }
            NavigationCommand::OpenCommandPalette => {
                self.show_command_palette = true;
                self.set_focused_window(FocusedWindow::CommandPalette);
            }
            NavigationCommand::EnterHintMode(action) => {
                // Enter hint mode with specified action
                self.navigation_state.push_mode(NavigationMode::Hint);

                // Get collected elements from the widget manager
                let collected_elements = self
                    .widget_manager
                    .collector()
                    .get_enabled_elements()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                tracing::info!(
                    "EnterHintMode: Widget manager provided {} elements",
                    collected_elements.len()
                );

                // Collect elements from all open windows
                let elements = collected_elements;

                // Add elements from other windows
                if self.help_window.is_open() {
                    tracing::info!("EnterHintMode: HelpWindow is open - but collect_navigable_elements is not implemented yet");
                    // TODO: Add collect_navigable_elements to HelpWindow when implemented
                } else {
                    tracing::info!("EnterHintMode: HelpWindow is not open");
                }

                if self.log_window.is_open() {
                    tracing::info!("EnterHintMode: LogWindow is open - but collect_navigable_elements is not implemented yet");
                    // TODO: Add collect_navigable_elements to LogWindow when implemented
                } else {
                    tracing::info!("EnterHintMode: LogWindow is not open");
                }

                if self.show_command_palette {
                    tracing::info!("EnterHintMode: CommandPalette is open - but collect_navigable_elements is not implemented yet");
                    // TODO: Add collect_navigable_elements to CommandPalette when implemented
                } else {
                    tracing::info!("EnterHintMode: CommandPalette is not open");
                }

                // Debug validation logs for hint mode element collection
                #[cfg(debug_assertions)]
                {
                    tracing::debug!("Hint mode: Collected {} navigable elements from all sources", elements.len());

                    // Validation logging for hint mode
                    if elements.len() >= 80 {
                        tracing::info!("Hint mode activated with {} elements (exceeds 80+ target)", elements.len());
                    } else if !elements.is_empty() {
                        tracing::info!("Hint mode activated with {} elements (below 80+ target)", elements.len());
                    } else {
                        tracing::warn!("Hint mode activated with 0 elements - no widgets to navigate");
                    }
                }

                // Start hint mode - logging happens in hint_mode.start()
                self.hint_mode.start(action, &elements);
                // Set flag to skip the next hint input (prevents activation key double-processing)
                self.skip_next_hint_input = true;
            }
            NavigationCommand::NextElement => {
                // Navigate to next focusable element
                info!("Next element");
            }
            NavigationCommand::PreviousElement => {
                // Navigate to previous focusable element
                info!("Previous element");
            }
            NavigationCommand::ActivateElement => {
                // Activate currently focused element
                info!("Activate element");
            }
            NavigationCommand::FocusSearchField => {
                // Focus search field in current window
                info!("Focus search field");
                // This is typically handled by individual windows
            }
        }
    }

    /// Process pending scroll requests from navigation commands
    pub(super) fn process_pending_scroll_requests(&mut self, _ctx: &egui::Context) {
        if let Some(scroll_amount) = self.pending_scroll_request.take() {
            // For now, we'll store the scroll request and apply it during window rendering
            // egui doesn't allow injecting scroll events directly, so we'll need to
            // coordinate with individual windows to handle scrolling
            info!("Processing scroll request: {} pixels", scroll_amount);

            // Store scroll request for windows to consume
            self.apply_scroll_to_focused_window(scroll_amount);
        }
    }

    /// Apply scroll to the currently focused window
    pub(super) fn apply_scroll_to_focused_window(&mut self, scroll_amount: f32) {
        // Based on the currently focused window, apply scrolling
        match self.currently_focused_window {
            Some(FocusedWindow::Help) => {
                info!("Scrolling Help window by {} pixels", scroll_amount);
                // TODO: Add scroll state to HelpWindow
            }
            Some(FocusedWindow::Log) => {
                info!("Scrolling Log window by {} pixels", scroll_amount);
                // TODO: Add scroll state to LogWindow
            }
            _ => {
                info!("No focused window or unsupported window for scrolling");
            }
        }
    }

    /// Handle scroll commands by sending scroll events to egui
    pub(super) fn handle_scroll_command(&mut self, amount: i32, horizontal: bool) {
        // Store scroll command to be applied on next frame
        // egui handles scrolling through input events, so we'll simulate scroll wheel events
        if horizontal {
            info!("Horizontal scroll: {}", amount);
            // For now, just log horizontal scrolling
            // TODO: Implement horizontal scrolling when needed
        } else {
            // Vertical scrolling - simulate mouse wheel
            let scroll_amount = amount as f32 * 20.0; // Convert to pixels
            info!("Vertical scroll: {} pixels", scroll_amount);

            // Store the scroll request for the next frame
            // egui will process this during the next update cycle
            self.pending_scroll_request = Some(scroll_amount);
        }
    }

    /// Focus the next window in the window order
    pub(super) fn focus_next_window(&mut self) {
        // Implement window cycling logic
        if let Some(current) = self.currently_focused_window {
            // For now, just cycle through a few common windows
            match current {
                FocusedWindow::Help => {
                    // Resource/template editor windows removed
                }
                _ => {
                    // Resource/template editor windows removed
                }
            }
        } else {
            // Resource/template editor windows removed
        }
    }

    /// Focus the previous window in the window order
    pub(super) fn focus_previous_window(&mut self) {
        // Implement reverse window cycling logic
        if let Some(current) = self.currently_focused_window {
            match current {
                FocusedWindow::Help => {
                    // Resource/template editor windows removed
                }
                _ => {
                    self.help_window.open = true;
                    self.set_focused_window(FocusedWindow::Help);
                }
            }
        } else {
            self.help_window.open = true;
            self.set_focused_window(FocusedWindow::Help);
        }
    }

    /// Focus a window by its index (1-9)
    pub(super) fn focus_window_by_index(&mut self, index: u8) {
        let window = match index {
            6 => {
                self.help_window.open = true;
                FocusedWindow::Help
            }
            7 => {
                self.log_window.open = true;
                FocusedWindow::Log
            }
            8 => {
                // Chat window removed
                FocusedWindow::Chat
            }
            9 => {
                self.credentials_debug_window.open = true;
                FocusedWindow::CredentialsDebug
            }
            _ => return,
        };

        self.set_focused_window(window);
    }

    /// Focus the last active window
    pub(super) fn focus_last_window(&mut self) {
        if let Some(last_window) = self.window_focus_order.last().copied() {
            self.set_focused_window(last_window);
        }
    }

    /// Legacy keyboard input handling for backwards compatibility
    pub(super) fn handle_legacy_keyboard_input(&mut self, ctx: &egui::Context) {
        // Space to open command palette (when navigation is in insert mode or disabled)
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !ctx.wants_keyboard_input() {
            match self.navigation_state.current_mode() {
                NavigationMode::Insert | NavigationMode::Command => {
                    self.show_command_palette = true;
                    self.set_focused_window(FocusedWindow::CommandPalette);
                }
                _ => {
                    // In normal/hint/visual modes, space is handled by navigation system
                }
            }
        }

        // F1 to open chat window - REMOVED (chat window deleted)

        // Ctrl+G to open CloudFormation graph window
        // Project management removed

        // Windows+C to close windows (legacy code kept for compatibility)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
            // Resource/template editor windows removed
        }
    }
}
