//! Key Mapping and Binding Registry
//!
//! This module provides configurable key binding management for the keyboard navigation system.
//! It supports loading keybindings from TOML configuration files and provides a registry
//! for mapping key sequences to navigation commands.

use super::keyboard_navigation::{ElementAction, NavigationCommand, NavigationMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A single key mapping from a key sequence to a navigation command
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyMapping {
    /// The key sequence that triggers this mapping (e.g., "j", "gg", "5j")
    pub sequence: String,
    /// The navigation command to execute
    pub command: String,
    /// Optional description for help display
    pub description: Option<String>,
    /// Which modes this mapping is active in
    pub modes: Vec<String>,
    /// Whether this mapping can be repeated with a count prefix
    pub repeatable: bool,
}

impl KeyMapping {
    /// Create a new key mapping
    pub fn new(sequence: String, command: String, modes: Vec<String>) -> Self {
        Self {
            sequence,
            command,
            description: None,
            modes,
            repeatable: false,
        }
    }

    /// Set the description for this mapping
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set whether this mapping is repeatable with count prefixes
    pub fn with_repeatable(mut self, repeatable: bool) -> Self {
        self.repeatable = repeatable;
        self
    }

    /// Check if this mapping is active in the given mode
    pub fn is_active_in_mode(&self, mode: NavigationMode) -> bool {
        let mode_str = match mode {
            NavigationMode::Normal => "normal",
            NavigationMode::Insert => "insert",
            NavigationMode::Hint => "hint",
            NavigationMode::Visual => "visual",
            NavigationMode::Command => "command",
        };

        self.modes.contains(&mode_str.to_string()) || self.modes.contains(&"all".to_string())
    }
}

/// Collection of key bindings organized by mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindingMap {
    /// Version of the key binding format
    pub version: String,
    /// Global settings
    pub settings: KeyBindingSettings,
    /// Key mappings organized by mode
    pub bindings: HashMap<String, Vec<KeyMapping>>,
    /// Window-specific overrides
    pub window_overrides: HashMap<String, Vec<KeyMapping>>,
}

/// Settings for key binding behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindingSettings {
    /// Timeout for multi-key sequences in milliseconds
    pub sequence_timeout_ms: u64,
    /// Whether to show key sequence in status bar
    pub show_key_sequence: bool,
    /// Whether to enable count prefixes (e.g., "5j")
    pub enable_count_prefixes: bool,
    /// Case sensitive key matching
    pub case_sensitive: bool,
}

impl Default for KeyBindingSettings {
    fn default() -> Self {
        Self {
            sequence_timeout_ms: 2000,
            show_key_sequence: true,
            enable_count_prefixes: true,
            case_sensitive: true,
        }
    }
}

impl Default for KeyBindingMap {
    fn default() -> Self {
        Self::create_default_bindings()
    }
}

impl KeyBindingMap {
    /// Create default key bindings similar to Vimium
    pub fn create_default_bindings() -> Self {
        let mut bindings = HashMap::new();

        // Normal mode bindings
        let normal_bindings = vec![
            // Scrolling
            KeyMapping::new(
                "j".to_string(),
                "scroll_down".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Scroll down".to_string())
            .with_repeatable(true),
            KeyMapping::new(
                "k".to_string(),
                "scroll_up".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Scroll up".to_string())
            .with_repeatable(true),
            KeyMapping::new(
                "h".to_string(),
                "scroll_left".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Scroll left".to_string())
            .with_repeatable(true),
            KeyMapping::new(
                "l".to_string(),
                "scroll_right".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Scroll right".to_string())
            .with_repeatable(true),
            KeyMapping::new(
                "d".to_string(),
                "scroll_half_page_down".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Scroll half page down".to_string())
            .with_repeatable(true),
            KeyMapping::new(
                "u".to_string(),
                "scroll_half_page_up".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Scroll half page up".to_string())
            .with_repeatable(true),
            KeyMapping::new(
                "gg".to_string(),
                "move_to_top".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Move to top".to_string()),
            KeyMapping::new(
                "G".to_string(),
                "move_to_bottom".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Move to bottom".to_string()),
            // Window navigation
            KeyMapping::new(
                "gt".to_string(),
                "next_window".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Next window".to_string()),
            KeyMapping::new(
                "gT".to_string(),
                "previous_window".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Previous window".to_string()),
            KeyMapping::new(
                "x".to_string(),
                "close_window".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Close window".to_string()),
            // Hint mode - universal hinting with smart actions
            KeyMapping::new(
                "f".to_string(),
                "hint_universal".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Show hints for all elements (smart actions)".to_string()),
            // Mode switching
            KeyMapping::new(
                "i".to_string(),
                "insert_mode".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Enter insert mode".to_string()),
            KeyMapping::new(
                "v".to_string(),
                "visual_mode".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Enter visual mode".to_string()),
            KeyMapping::new(
                ":".to_string(),
                "command_mode".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Enter command mode".to_string()),
            // Window selection by number
            KeyMapping::new(
                "1".to_string(),
                "window_1".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 1".to_string()),
            KeyMapping::new(
                "2".to_string(),
                "window_2".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 2".to_string()),
            KeyMapping::new(
                "3".to_string(),
                "window_3".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 3".to_string()),
            KeyMapping::new(
                "4".to_string(),
                "window_4".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 4".to_string()),
            KeyMapping::new(
                "5".to_string(),
                "window_5".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 5".to_string()),
            KeyMapping::new(
                "6".to_string(),
                "window_6".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 6".to_string()),
            KeyMapping::new(
                "7".to_string(),
                "window_7".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 7".to_string()),
            KeyMapping::new(
                "8".to_string(),
                "window_8".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 8".to_string()),
            KeyMapping::new(
                "9".to_string(),
                "window_9".to_string(),
                vec!["normal".to_string()],
            )
            .with_description("Go to window 9".to_string()),
        ];

        bindings.insert("normal".to_string(), normal_bindings);

        // Insert mode bindings
        let insert_bindings = vec![KeyMapping::new(
            "Escape".to_string(),
            "normal_mode".to_string(),
            vec!["insert".to_string()],
        )
        .with_description("Return to normal mode".to_string())];
        bindings.insert("insert".to_string(), insert_bindings);

        // Hint mode bindings
        let hint_bindings = vec![
            KeyMapping::new(
                "Escape".to_string(),
                "normal_mode".to_string(),
                vec!["hint".to_string()],
            )
            .with_description("Cancel hint mode".to_string()),
            KeyMapping::new(
                "Enter".to_string(),
                "activate_hint".to_string(),
                vec!["hint".to_string()],
            )
            .with_description("Activate selected hint".to_string()),
        ];
        bindings.insert("hint".to_string(), hint_bindings);

        // Visual mode bindings
        let visual_bindings = vec![KeyMapping::new(
            "Escape".to_string(),
            "normal_mode".to_string(),
            vec!["visual".to_string()],
        )
        .with_description("Return to normal mode".to_string())];
        bindings.insert("visual".to_string(), visual_bindings);

        // Command mode bindings
        let command_bindings = vec![
            KeyMapping::new(
                "Escape".to_string(),
                "normal_mode".to_string(),
                vec!["command".to_string()],
            )
            .with_description("Cancel command mode".to_string()),
            KeyMapping::new(
                "Enter".to_string(),
                "execute_command".to_string(),
                vec!["command".to_string()],
            )
            .with_description("Execute command".to_string()),
        ];
        bindings.insert("command".to_string(), command_bindings);

        Self {
            version: "1.0".to_string(),
            settings: KeyBindingSettings::default(),
            bindings,
            window_overrides: HashMap::new(),
        }
    }

    /// Load key bindings from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let bindings: KeyBindingMap = toml::from_str(&content)?;
        Ok(bindings)
    }

    /// Save key bindings to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get bindings for a specific mode
    pub fn get_bindings_for_mode(&self, mode: NavigationMode) -> Vec<&KeyMapping> {
        let mode_str = match mode {
            NavigationMode::Normal => "normal",
            NavigationMode::Insert => "insert",
            NavigationMode::Hint => "hint",
            NavigationMode::Visual => "visual",
            NavigationMode::Command => "command",
        };

        self.bindings
            .get(mode_str)
            .map(|bindings| bindings.iter().collect())
            .unwrap_or_default()
    }

    /// Get window-specific overrides for a window
    pub fn get_window_overrides(&self, window_id: &str) -> Vec<&KeyMapping> {
        self.window_overrides
            .get(window_id)
            .map(|bindings| bindings.iter().collect())
            .unwrap_or_default()
    }

    /// Add a new key binding
    pub fn add_binding(&mut self, mode: NavigationMode, mapping: KeyMapping) {
        let mode_str = match mode {
            NavigationMode::Normal => "normal",
            NavigationMode::Insert => "insert",
            NavigationMode::Hint => "hint",
            NavigationMode::Visual => "visual",
            NavigationMode::Command => "command",
        };

        self.bindings
            .entry(mode_str.to_string())
            .or_default()
            .push(mapping);
    }

    /// Remove a key binding by sequence
    pub fn remove_binding(&mut self, mode: NavigationMode, sequence: &str) -> bool {
        let mode_str = match mode {
            NavigationMode::Normal => "normal",
            NavigationMode::Insert => "insert",
            NavigationMode::Hint => "hint",
            NavigationMode::Visual => "visual",
            NavigationMode::Command => "command",
        };

        if let Some(bindings) = self.bindings.get_mut(mode_str) {
            let initial_len = bindings.len();
            bindings.retain(|binding| binding.sequence != sequence);
            bindings.len() != initial_len
        } else {
            false
        }
    }
}

/// Registry for managing key mappings and command resolution
#[derive(Debug)]
pub struct KeyMappingRegistry {
    /// Current key binding configuration
    bindings: KeyBindingMap,
    /// Cache for fast command lookup
    command_cache: HashMap<String, NavigationCommand>,
}

impl KeyMappingRegistry {
    /// Create a new registry with default bindings
    pub fn new() -> Self {
        Self::with_bindings(KeyBindingMap::default())
    }

    /// Create a registry with custom bindings
    pub fn with_bindings(bindings: KeyBindingMap) -> Self {
        let mut registry = Self {
            bindings,
            command_cache: HashMap::new(),
        };
        registry.rebuild_cache();
        registry
    }

    /// Load bindings from a TOML configuration file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let bindings = KeyBindingMap::load_from_file(path)?;
        Ok(Self::with_bindings(bindings))
    }

    /// Get the current key binding settings
    pub fn settings(&self) -> &KeyBindingSettings {
        &self.bindings.settings
    }

    /// Look up a navigation command for a key sequence in a specific mode
    pub fn lookup_command(
        &self,
        sequence: &str,
        mode: NavigationMode,
        window_id: Option<&str>,
    ) -> Option<NavigationCommand> {
        // Check window-specific overrides first
        if let Some(window_id) = window_id {
            for mapping in self.bindings.get_window_overrides(window_id) {
                if mapping.sequence == sequence && mapping.is_active_in_mode(mode) {
                    return self.parse_command_string(&mapping.command);
                }
            }
        }

        // Check mode-specific bindings
        for mapping in self.bindings.get_bindings_for_mode(mode) {
            if mapping.sequence == sequence {
                return self.parse_command_string(&mapping.command);
            }
        }

        None
    }

    /// Get all available bindings for a mode (for help display)
    pub fn get_bindings_for_mode(&self, mode: NavigationMode) -> Vec<&KeyMapping> {
        self.bindings.get_bindings_for_mode(mode)
    }

    /// Check if a sequence is a prefix of any valid binding
    pub fn is_sequence_prefix(&self, sequence: &str, mode: NavigationMode) -> bool {
        for mapping in self.bindings.get_bindings_for_mode(mode) {
            if mapping.sequence.starts_with(sequence) && mapping.sequence != sequence {
                return true;
            }
        }
        false
    }

    /// Parse a command string into a NavigationCommand
    fn parse_command_string(&self, command: &str) -> Option<NavigationCommand> {
        // Use cache if available
        if let Some(cached_command) = self.command_cache.get(command) {
            return Some(cached_command.clone());
        }

        // Parse command string
        match command {
            "scroll_down" => Some(NavigationCommand::ScrollVertical(1)),
            "scroll_up" => Some(NavigationCommand::ScrollVertical(-1)),
            "scroll_left" => Some(NavigationCommand::ScrollHorizontal(-1)),
            "scroll_right" => Some(NavigationCommand::ScrollHorizontal(1)),
            "scroll_half_page_down" => Some(NavigationCommand::ScrollVertical(10)),
            "scroll_half_page_up" => Some(NavigationCommand::ScrollVertical(-10)),
            "move_to_top" => Some(NavigationCommand::MoveToTop),
            "move_to_bottom" => Some(NavigationCommand::MoveToBottom),
            "next_window" => Some(NavigationCommand::NextWindow),
            "previous_window" => Some(NavigationCommand::PreviousWindow),
            "close_window" => Some(NavigationCommand::CloseWindow),
            "hint_universal" => Some(NavigationCommand::EnterHintMode(ElementAction::Smart)),
            "activate_hint" => Some(NavigationCommand::ActivateElement),
            "next_element" => Some(NavigationCommand::NextElement),
            "previous_element" => Some(NavigationCommand::PreviousElement),
            "window_1" => Some(NavigationCommand::WindowByIndex(1)),
            "window_2" => Some(NavigationCommand::WindowByIndex(2)),
            "window_3" => Some(NavigationCommand::WindowByIndex(3)),
            "window_4" => Some(NavigationCommand::WindowByIndex(4)),
            "window_5" => Some(NavigationCommand::WindowByIndex(5)),
            "window_6" => Some(NavigationCommand::WindowByIndex(6)),
            "window_7" => Some(NavigationCommand::WindowByIndex(7)),
            "window_8" => Some(NavigationCommand::WindowByIndex(8)),
            "window_9" => Some(NavigationCommand::WindowByIndex(9)),
            _ => None,
        }
    }

    /// Rebuild the command cache
    fn rebuild_cache(&mut self) {
        self.command_cache.clear();

        for bindings in self.bindings.bindings.values() {
            for mapping in bindings {
                if let Some(command) = self.parse_command_string(&mapping.command) {
                    self.command_cache.insert(mapping.command.clone(), command);
                }
            }
        }
    }

    /// Update the key bindings and rebuild cache
    pub fn update_bindings(&mut self, bindings: KeyBindingMap) {
        self.bindings = bindings;
        self.rebuild_cache();
    }

    /// Add a custom binding
    pub fn add_binding(&mut self, mode: NavigationMode, mapping: KeyMapping) {
        self.bindings.add_binding(mode, mapping);
        self.rebuild_cache();
    }

    /// Remove a binding
    pub fn remove_binding(&mut self, mode: NavigationMode, sequence: &str) -> bool {
        let removed = self.bindings.remove_binding(mode, sequence);
        if removed {
            self.rebuild_cache();
        }
        removed
    }
}

impl Default for KeyMappingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_mapping_creation() {
        let mapping = KeyMapping::new(
            "j".to_string(),
            "scroll_down".to_string(),
            vec!["normal".to_string()],
        )
        .with_description("Scroll down".to_string())
        .with_repeatable(true);

        assert_eq!(mapping.sequence, "j");
        assert_eq!(mapping.command, "scroll_down");
        assert_eq!(mapping.description, Some("Scroll down".to_string()));
        assert!(mapping.repeatable);
        assert!(mapping.is_active_in_mode(NavigationMode::Normal));
        assert!(!mapping.is_active_in_mode(NavigationMode::Insert));
    }

    #[test]
    fn test_key_mapping_mode_check() {
        let all_modes_mapping = KeyMapping::new(
            "escape".to_string(),
            "cancel".to_string(),
            vec!["all".to_string()],
        );

        assert!(all_modes_mapping.is_active_in_mode(NavigationMode::Normal));
        assert!(all_modes_mapping.is_active_in_mode(NavigationMode::Insert));
        assert!(all_modes_mapping.is_active_in_mode(NavigationMode::Hint));
    }

    #[test]
    fn test_default_bindings_creation() {
        let bindings = KeyBindingMap::create_default_bindings();

        assert!(!bindings.bindings.is_empty());
        assert!(bindings.bindings.contains_key("normal"));
        assert!(bindings.bindings.contains_key("insert"));

        let normal_bindings = &bindings.bindings["normal"];
        assert!(!normal_bindings.is_empty());

        // Check for specific expected bindings
        let has_j_binding = normal_bindings.iter().any(|b| b.sequence == "j");
        assert!(has_j_binding);
    }

    #[test]
    fn test_key_mapping_registry() {
        let registry = KeyMappingRegistry::new();

        // Test command lookup
        let command = registry.lookup_command("j", NavigationMode::Normal, None);
        assert!(command.is_some());

        // Test prefix checking
        assert!(registry.is_sequence_prefix("g", NavigationMode::Normal));
        assert!(!registry.is_sequence_prefix("xyz", NavigationMode::Normal));
    }

    #[test]
    fn test_command_parsing() {
        let registry = KeyMappingRegistry::new();

        let scroll_down = registry.parse_command_string("scroll_down");
        assert_eq!(scroll_down, Some(NavigationCommand::ScrollVertical(1)));

        let invalid_command = registry.parse_command_string("invalid_command");
        assert_eq!(invalid_command, None);
    }

    #[test]
    fn test_binding_modification() {
        let mut registry = KeyMappingRegistry::new();

        // Add a custom binding
        let custom_mapping = KeyMapping::new(
            "custom".to_string(),
            "scroll_down".to_string(),
            vec!["normal".to_string()],
        );

        registry.add_binding(NavigationMode::Normal, custom_mapping);

        // Test that the new binding works
        let command = registry.lookup_command("custom", NavigationMode::Normal, None);
        assert!(command.is_some());

        // Remove the binding
        let removed = registry.remove_binding(NavigationMode::Normal, "custom");
        assert!(removed);

        // Test that the binding is gone
        let command = registry.lookup_command("custom", NavigationMode::Normal, None);
        assert!(command.is_none());
    }

    #[test]
    fn test_window_override_lookup() {
        let mut bindings = KeyBindingMap::create_default_bindings();

        // Add window-specific override
        let window_mapping = KeyMapping::new(
            "j".to_string(),
            "next_element".to_string(),
            vec!["normal".to_string()],
        );

        bindings
            .window_overrides
            .insert("test_window".to_string(), vec![window_mapping]);

        let registry = KeyMappingRegistry::with_bindings(bindings);

        // Test window-specific lookup
        let command = registry.lookup_command("j", NavigationMode::Normal, Some("test_window"));
        assert_eq!(command, Some(NavigationCommand::NextElement));

        // Test fallback to general binding for different window
        let command = registry.lookup_command("j", NavigationMode::Normal, Some("other_window"));
        assert_eq!(command, Some(NavigationCommand::ScrollVertical(1)));
    }

    #[test]
    fn test_serialization() {
        let bindings = KeyBindingMap::create_default_bindings();

        // Test TOML serialization
        let toml_string = toml::to_string(&bindings);
        assert!(toml_string.is_ok());

        // Test deserialization
        let toml_content = toml_string.unwrap();
        let deserialized: Result<KeyBindingMap, _> = toml::from_str(&toml_content);
        assert!(deserialized.is_ok());
    }
}
