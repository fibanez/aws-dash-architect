//! Navigation State Management
//!
//! This module manages the global navigation state for the keyboard navigation system,
//! including mode tracking, key sequence processing, and command count parsing.

use super::keyboard_navigation::{KeyEventResult, NavigationCommand, NavigationMode};
use eframe::egui;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Maximum length of key sequence history
const MAX_KEY_SEQUENCE_LENGTH: usize = 10;

/// Timeout for key sequences (2 seconds)
const KEY_SEQUENCE_TIMEOUT: Duration = Duration::from_secs(2);

/// Global navigation state manager
#[derive(Debug)]
pub struct NavigationState {
    /// Current navigation mode
    current_mode: NavigationMode,
    /// Stack of previous modes for nested operations
    mode_stack: Vec<NavigationMode>,
    /// Current key sequence being built
    key_sequence: String,
    /// History of recent key presses
    key_history: VecDeque<KeyPress>,
    /// Current command count (e.g., "5" in "5j")
    command_count: Option<u32>,
    /// Time when current key sequence started
    sequence_start_time: Option<Instant>,
    /// Whether the next key should be treated as a literal input
    escape_next_key: bool,
    /// Currently active window ID for navigation
    active_window_id: Option<String>,
    /// Whether navigation is globally enabled
    navigation_enabled: bool,
}

/// Represents a single key press with timing information
#[derive(Debug, Clone)]
struct KeyPress {
    /// The key that was pressed
    #[allow(dead_code)]
    key: egui::Key,
    /// Any modifiers that were active
    #[allow(dead_code)]
    modifiers: egui::Modifiers,
    /// Character representation if applicable
    character: Option<char>,
    /// When this key was pressed
    timestamp: Instant,
}

impl NavigationState {
    /// Create a new navigation state
    pub fn new() -> Self {
        Self {
            current_mode: NavigationMode::Normal,
            mode_stack: Vec::new(),
            key_sequence: String::new(),
            key_history: VecDeque::new(),
            command_count: None,
            sequence_start_time: None,
            escape_next_key: false,
            active_window_id: None,
            navigation_enabled: true,
        }
    }

    /// Get the current navigation mode
    pub fn current_mode(&self) -> NavigationMode {
        self.current_mode
    }

    /// Set the navigation mode
    pub fn set_mode(&mut self, mode: NavigationMode) {
        if mode != self.current_mode {
            self.mode_stack.push(self.current_mode);
            self.current_mode = mode;
            self.clear_key_sequence();
        }
    }

    /// Enter a new mode, pushing the current mode onto the stack
    pub fn push_mode(&mut self, mode: NavigationMode) {
        self.mode_stack.push(self.current_mode);
        self.current_mode = mode;
        self.clear_key_sequence();
    }

    /// Return to the previous mode
    pub fn pop_mode(&mut self) -> NavigationMode {
        if let Some(previous_mode) = self.mode_stack.pop() {
            self.current_mode = previous_mode;
            self.clear_key_sequence();
        }
        self.current_mode
    }

    /// Get the current key sequence
    pub fn current_key_sequence(&self) -> &str {
        &self.key_sequence
    }

    /// Get the current command count
    pub fn current_command_count(&self) -> Option<u32> {
        self.command_count
    }

    /// Set the active window for navigation
    pub fn set_active_window(&mut self, window_id: Option<String>) {
        self.active_window_id = window_id;
    }

    /// Get the currently active window ID
    pub fn active_window_id(&self) -> Option<&String> {
        self.active_window_id.as_ref()
    }

    /// Enable or disable global navigation
    pub fn set_navigation_enabled(&mut self, enabled: bool) {
        self.navigation_enabled = enabled;
        if !enabled {
            self.clear_key_sequence();
        }
    }

    /// Check if navigation is globally enabled
    pub fn is_navigation_enabled(&self) -> bool {
        self.navigation_enabled
    }

    /// Process a key event and update navigation state
    pub fn process_key_event(
        &mut self,
        event: &egui::Event,
        ctx: &egui::Context,
    ) -> KeyEventResult {
        if !self.navigation_enabled {
            return KeyEventResult::PassThrough;
        }

        // Handle escape next key mode
        if self.escape_next_key {
            self.escape_next_key = false;
            return KeyEventResult::PassThrough;
        }

        // Clean up old key sequences
        self.cleanup_old_sequences();

        match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => self.handle_key_press(*key, *modifiers, ctx),
            egui::Event::Text(text) => self.handle_text_input(text),
            _ => KeyEventResult::PassThrough,
        }
    }

    /// Handle a key press event
    fn handle_key_press(
        &mut self,
        key: egui::Key,
        modifiers: egui::Modifiers,
        ctx: &egui::Context,
    ) -> KeyEventResult {
        // Record the key press
        let key_press = KeyPress {
            key,
            modifiers,
            character: self.key_to_char(key, modifiers),
            timestamp: Instant::now(),
        };

        self.add_key_to_history(key_press.clone());

        // Handle mode-specific key processing
        match self.current_mode {
            NavigationMode::Normal => self.handle_normal_mode_key(key, modifiers, ctx),
            NavigationMode::Insert => self.handle_insert_mode_key(key, modifiers),
            NavigationMode::Hint => self.handle_hint_mode_key(key, modifiers),
            NavigationMode::Visual => self.handle_visual_mode_key(key, modifiers),
            NavigationMode::Command => self.handle_command_mode_key(key, modifiers),
        }
    }

    /// Handle text input events
    fn handle_text_input(&mut self, text: &str) -> KeyEventResult {
        match self.current_mode {
            NavigationMode::Normal => {
                // In normal mode, text input might be part of key sequences
                self.add_to_key_sequence(text);
                self.try_parse_command()
            }
            NavigationMode::Insert => {
                // Pass through text input in insert mode
                KeyEventResult::PassThrough
            }
            NavigationMode::Hint => {
                // In hint mode, text input filters hints
                self.add_to_key_sequence(text);
                KeyEventResult::Command(NavigationCommand::NextElement) // Placeholder
            }
            NavigationMode::Command => {
                // Command mode handles text for command entry
                KeyEventResult::PassThrough
            }
            NavigationMode::Visual => {
                // Visual mode may use text for search
                KeyEventResult::PassThrough
            }
        }
    }

    /// Handle keys in normal mode
    fn handle_normal_mode_key(
        &mut self,
        key: egui::Key,
        modifiers: egui::Modifiers,
        _ctx: &egui::Context,
    ) -> KeyEventResult {
        // Handle escape - clear current operations
        if key == egui::Key::Escape {
            self.clear_key_sequence();
            return KeyEventResult::Handled;
        }

        // Handle mode switching keys
        match key {
            egui::Key::I if modifiers.is_none() => {
                return KeyEventResult::ModeChanged(NavigationMode::Insert);
            }
            egui::Key::V if modifiers.is_none() => {
                return KeyEventResult::ModeChanged(NavigationMode::Visual);
            }
            egui::Key::Colon if modifiers.is_none() => {
                return KeyEventResult::ModeChanged(NavigationMode::Command);
            }
            _ => {}
        }

        // Add key to sequence and try to parse
        if let Some(ch) = self.key_to_char(key, modifiers) {
            self.add_to_key_sequence(&ch.to_string());
        }

        self.try_parse_command()
    }

    /// Handle keys in insert mode
    fn handle_insert_mode_key(
        &mut self,
        key: egui::Key,
        _modifiers: egui::Modifiers,
    ) -> KeyEventResult {
        match key {
            egui::Key::Escape => KeyEventResult::ModeChanged(NavigationMode::Normal),
            _ => KeyEventResult::PassThrough,
        }
    }

    /// Handle keys in hint mode
    fn handle_hint_mode_key(
        &mut self,
        key: egui::Key,
        _modifiers: egui::Modifiers,
    ) -> KeyEventResult {
        match key {
            egui::Key::Escape => KeyEventResult::ModeChanged(NavigationMode::Normal),
            egui::Key::Enter => KeyEventResult::Command(NavigationCommand::ActivateElement),
            _ => {
                // Filter hints based on key input
                if let Some(ch) = self.key_to_char(key, egui::Modifiers::NONE) {
                    self.add_to_key_sequence(&ch.to_string());
                }
                KeyEventResult::Handled
            }
        }
    }

    /// Handle keys in visual mode
    fn handle_visual_mode_key(
        &mut self,
        key: egui::Key,
        _modifiers: egui::Modifiers,
    ) -> KeyEventResult {
        match key {
            egui::Key::Escape => KeyEventResult::ModeChanged(NavigationMode::Normal),
            _ => KeyEventResult::PassThrough,
        }
    }

    /// Handle keys in command mode
    fn handle_command_mode_key(
        &mut self,
        key: egui::Key,
        _modifiers: egui::Modifiers,
    ) -> KeyEventResult {
        match key {
            egui::Key::Escape => KeyEventResult::ModeChanged(NavigationMode::Normal),
            egui::Key::Enter => {
                // Execute command
                let result = KeyEventResult::Command(NavigationCommand::OpenCommandPalette);
                self.clear_key_sequence();
                result
            }
            _ => KeyEventResult::PassThrough,
        }
    }

    /// Try to parse the current key sequence as a command
    fn try_parse_command(&mut self) -> KeyEventResult {
        let sequence = self.key_sequence.clone();

        // Parse number prefix for command count
        if let Some(command_with_count) = self.extract_command_count(&sequence) {
            if let Some(command) = self.parse_navigation_command(&command_with_count) {
                self.clear_key_sequence();
                return KeyEventResult::Command(command);
            }
        }

        // Check for single-key commands
        match sequence.as_str() {
            "j" => {
                self.clear_key_sequence();
                let count = self.take_command_count().unwrap_or(1) as i32;
                KeyEventResult::Command(NavigationCommand::ScrollVertical(count))
            }
            "k" => {
                self.clear_key_sequence();
                let count = self.take_command_count().unwrap_or(1) as i32;
                KeyEventResult::Command(NavigationCommand::ScrollVertical(-count))
            }
            "h" => {
                self.clear_key_sequence();
                let count = self.take_command_count().unwrap_or(1) as i32;
                KeyEventResult::Command(NavigationCommand::ScrollHorizontal(-count))
            }
            "l" => {
                self.clear_key_sequence();
                let count = self.take_command_count().unwrap_or(1) as i32;
                KeyEventResult::Command(NavigationCommand::ScrollHorizontal(count))
            }
            "gg" => {
                self.clear_key_sequence();
                KeyEventResult::Command(NavigationCommand::MoveToTop)
            }
            "G" => {
                self.clear_key_sequence();
                KeyEventResult::Command(NavigationCommand::MoveToBottom)
            }
            "x" => {
                self.clear_key_sequence();
                KeyEventResult::Command(NavigationCommand::CloseWindow)
            }
            "gt" => {
                self.clear_key_sequence();
                KeyEventResult::Command(NavigationCommand::NextWindow)
            }
            "gT" => {
                self.clear_key_sequence();
                KeyEventResult::Command(NavigationCommand::PreviousWindow)
            }
            "f" => {
                self.clear_key_sequence();
                KeyEventResult::Command(NavigationCommand::EnterHintMode(
                    super::keyboard_navigation::ElementAction::Smart,
                ))
            }
            // Single digit window selection
            s if s.len() == 1 && s.chars().next().unwrap().is_ascii_digit() => {
                if let Ok(index) = s.parse::<u8>() {
                    if (1..=9).contains(&index) {
                        self.clear_key_sequence();
                        return KeyEventResult::Command(NavigationCommand::WindowByIndex(index));
                    }
                }
                KeyEventResult::Handled
            }
            _ => {
                // Continue building sequence or timeout
                if sequence.len() > 3 || self.is_sequence_expired() {
                    self.clear_key_sequence();
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Handled
                }
            }
        }
    }

    /// Extract command count from key sequence
    fn extract_command_count(&mut self, sequence: &str) -> Option<String> {
        let mut chars = sequence.chars();
        let mut count_str = String::new();

        // Extract leading digits
        while let Some(ch) = chars.next() {
            if ch.is_ascii_digit() {
                count_str.push(ch);
            } else {
                // Found non-digit, parse count and return remaining command
                if !count_str.is_empty() {
                    if let Ok(count) = count_str.parse::<u32>() {
                        self.command_count = Some(count);
                        // Include the current non-digit character plus remaining characters
                        let mut remaining = String::new();
                        remaining.push(ch);
                        remaining.push_str(&chars.collect::<String>());
                        return Some(remaining);
                    }
                }
                // No count found, return original sequence
                return Some(sequence.to_string());
            }
        }

        // All digits, not a complete command yet
        None
    }

    /// Parse a navigation command from the command string
    fn parse_navigation_command(&self, command: &str) -> Option<NavigationCommand> {
        match command {
            "d" => Some(NavigationCommand::ScrollVertical(
                self.command_count.unwrap_or(1) as i32 * 10,
            )),
            "u" => Some(NavigationCommand::ScrollVertical(
                -(self.command_count.unwrap_or(1) as i32 * 10),
            )),
            _ => None,
        }
    }

    /// Add a key press to the history
    fn add_key_to_history(&mut self, key_press: KeyPress) {
        self.key_history.push_back(key_press);

        // Keep history size manageable
        while self.key_history.len() > MAX_KEY_SEQUENCE_LENGTH {
            self.key_history.pop_front();
        }
    }

    /// Add text to the current key sequence
    fn add_to_key_sequence(&mut self, text: &str) {
        if self.sequence_start_time.is_none() {
            self.sequence_start_time = Some(Instant::now());
        }
        self.key_sequence.push_str(text);
    }

    /// Clear the current key sequence and reset state
    fn clear_key_sequence(&mut self) {
        self.key_sequence.clear();
        self.command_count = None;
        self.sequence_start_time = None;
    }

    /// Take the current command count, consuming it
    fn take_command_count(&mut self) -> Option<u32> {
        self.command_count.take()
    }

    /// Check if the current key sequence has expired
    fn is_sequence_expired(&self) -> bool {
        if let Some(start_time) = self.sequence_start_time {
            start_time.elapsed() > KEY_SEQUENCE_TIMEOUT
        } else {
            false
        }
    }

    /// Clean up expired key sequences
    fn cleanup_old_sequences(&mut self) {
        if self.is_sequence_expired() {
            self.clear_key_sequence();
        }

        // Clean up old key history
        let cutoff_time = Instant::now() - KEY_SEQUENCE_TIMEOUT;
        while let Some(key_press) = self.key_history.front() {
            if key_press.timestamp < cutoff_time {
                self.key_history.pop_front();
            } else {
                break;
            }
        }
    }

    /// Convert a key press to a character representation
    fn key_to_char(&self, key: egui::Key, modifiers: egui::Modifiers) -> Option<char> {
        match key {
            egui::Key::A => Some(if modifiers.shift { 'A' } else { 'a' }),
            egui::Key::B => Some(if modifiers.shift { 'B' } else { 'b' }),
            egui::Key::C => Some(if modifiers.shift { 'C' } else { 'c' }),
            egui::Key::D => Some(if modifiers.shift { 'D' } else { 'd' }),
            egui::Key::E => Some(if modifiers.shift { 'E' } else { 'e' }),
            egui::Key::F => Some(if modifiers.shift { 'F' } else { 'f' }),
            egui::Key::G => Some(if modifiers.shift { 'G' } else { 'g' }),
            egui::Key::H => Some(if modifiers.shift { 'H' } else { 'h' }),
            egui::Key::I => Some(if modifiers.shift { 'I' } else { 'i' }),
            egui::Key::J => Some(if modifiers.shift { 'J' } else { 'j' }),
            egui::Key::K => Some(if modifiers.shift { 'K' } else { 'k' }),
            egui::Key::L => Some(if modifiers.shift { 'L' } else { 'l' }),
            egui::Key::M => Some(if modifiers.shift { 'M' } else { 'm' }),
            egui::Key::N => Some(if modifiers.shift { 'N' } else { 'n' }),
            egui::Key::O => Some(if modifiers.shift { 'O' } else { 'o' }),
            egui::Key::P => Some(if modifiers.shift { 'P' } else { 'p' }),
            egui::Key::Q => Some(if modifiers.shift { 'Q' } else { 'q' }),
            egui::Key::R => Some(if modifiers.shift { 'R' } else { 'r' }),
            egui::Key::S => Some(if modifiers.shift { 'S' } else { 's' }),
            egui::Key::T => Some(if modifiers.shift { 'T' } else { 't' }),
            egui::Key::U => Some(if modifiers.shift { 'U' } else { 'u' }),
            egui::Key::V => Some(if modifiers.shift { 'V' } else { 'v' }),
            egui::Key::W => Some(if modifiers.shift { 'W' } else { 'w' }),
            egui::Key::X => Some(if modifiers.shift { 'X' } else { 'x' }),
            egui::Key::Y => Some(if modifiers.shift { 'Y' } else { 'y' }),
            egui::Key::Z => Some(if modifiers.shift { 'Z' } else { 'z' }),
            egui::Key::Num0 => Some('0'),
            egui::Key::Num1 => Some('1'),
            egui::Key::Num2 => Some('2'),
            egui::Key::Num3 => Some('3'),
            egui::Key::Num4 => Some('4'),
            egui::Key::Num5 => Some('5'),
            egui::Key::Num6 => Some('6'),
            egui::Key::Num7 => Some('7'),
            egui::Key::Num8 => Some('8'),
            egui::Key::Num9 => Some('9'),
            egui::Key::Colon => Some(':'),
            _ => None,
        }
    }

    /// Get recent key history as a string for debugging
    pub fn get_key_history_string(&self) -> String {
        self.key_history
            .iter()
            .filter_map(|kp| kp.character)
            .collect()
    }
}

impl Default for NavigationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_state_creation() {
        let state = NavigationState::new();
        assert_eq!(state.current_mode(), NavigationMode::Normal);
        assert_eq!(state.current_key_sequence(), "");
        assert_eq!(state.current_command_count(), None);
        assert!(state.is_navigation_enabled());
    }

    #[test]
    fn test_mode_switching() {
        let mut state = NavigationState::new();

        // Test setting mode
        state.set_mode(NavigationMode::Insert);
        assert_eq!(state.current_mode(), NavigationMode::Insert);

        // Test push/pop mode
        state.push_mode(NavigationMode::Hint);
        assert_eq!(state.current_mode(), NavigationMode::Hint);

        let previous = state.pop_mode();
        assert_eq!(previous, NavigationMode::Insert);
        assert_eq!(state.current_mode(), NavigationMode::Insert);
    }

    #[test]
    fn test_key_sequence_building() {
        let mut state = NavigationState::new();

        state.add_to_key_sequence("g");
        assert_eq!(state.current_key_sequence(), "g");

        state.add_to_key_sequence("g");
        assert_eq!(state.current_key_sequence(), "gg");

        state.clear_key_sequence();
        assert_eq!(state.current_key_sequence(), "");
    }

    #[test]
    fn test_command_count_extraction() {
        let mut state = NavigationState::new();

        // Test extracting count from "5j"
        let result = state.extract_command_count("5j");
        assert_eq!(result, Some("j".to_string()));
        assert_eq!(state.command_count, Some(5));

        // Reset for next test
        state.command_count = None;

        // Test command without count
        let result = state.extract_command_count("j");
        assert_eq!(result, Some("j".to_string()));
        assert_eq!(state.command_count, None);

        // Test just digits (incomplete command)
        let result = state.extract_command_count("123");
        assert_eq!(result, None);
    }

    #[test]
    fn test_key_to_char_conversion() {
        let state = NavigationState::new();

        // Test regular keys
        assert_eq!(
            state.key_to_char(egui::Key::A, egui::Modifiers::NONE),
            Some('a')
        );
        assert_eq!(
            state.key_to_char(egui::Key::A, egui::Modifiers::SHIFT),
            Some('A')
        );
        assert_eq!(
            state.key_to_char(egui::Key::Num5, egui::Modifiers::NONE),
            Some('5')
        );
        assert_eq!(
            state.key_to_char(egui::Key::Colon, egui::Modifiers::NONE),
            Some(':')
        );

        // Test keys that don't map to characters
        assert_eq!(
            state.key_to_char(egui::Key::Tab, egui::Modifiers::NONE),
            None
        );
        assert_eq!(
            state.key_to_char(egui::Key::Enter, egui::Modifiers::NONE),
            None
        );
    }

    #[test]
    fn test_navigation_enable_disable() {
        let mut state = NavigationState::new();

        assert!(state.is_navigation_enabled());

        state.set_navigation_enabled(false);
        assert!(!state.is_navigation_enabled());

        state.set_navigation_enabled(true);
        assert!(state.is_navigation_enabled());
    }

    #[test]
    fn test_active_window_tracking() {
        let mut state = NavigationState::new();

        assert_eq!(state.active_window_id(), None);

        state.set_active_window(Some("test_window".to_string()));
        assert_eq!(state.active_window_id(), Some(&"test_window".to_string()));

        state.set_active_window(None);
        assert_eq!(state.active_window_id(), None);
    }

    #[test]
    fn test_command_count_take() {
        let mut state = NavigationState::new();

        state.command_count = Some(5);
        assert_eq!(state.take_command_count(), Some(5));
        assert_eq!(state.command_count, None);

        // Taking again should return None
        assert_eq!(state.take_command_count(), None);
    }

    #[test]
    fn test_key_history() {
        let mut state = NavigationState::new();

        let key_press = KeyPress {
            key: egui::Key::A,
            modifiers: egui::Modifiers::NONE,
            character: Some('a'),
            timestamp: Instant::now(),
        };

        state.add_key_to_history(key_press);
        assert_eq!(state.key_history.len(), 1);

        // Test history string generation
        let history_string = state.get_key_history_string();
        assert_eq!(history_string, "a");
    }

    #[test]
    fn test_sequence_timeout() {
        let mut state = NavigationState::new();

        // Fresh state should not be expired
        assert!(!state.is_sequence_expired());

        // Set start time to past
        state.sequence_start_time = Some(Instant::now() - Duration::from_secs(5));
        assert!(state.is_sequence_expired());
    }
}
