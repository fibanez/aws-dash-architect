//! Property-based filtering and grouping system for AWS resources
//!
//! This module provides a comprehensive property-based filtering and grouping system
//! that complements the existing tag system. It allows users to filter and organize
//! resources by their JSON properties (e.g., instance.status, instance.type).
//!
//! # Core Concepts
//!
//! - **Property Discovery**: Properties are dynamically discovered from visible resources
//! - **Type Preservation**: Property values maintain their types (String, Number, Boolean, Date, etc.)
//! - **Dot Notation**: Nested objects are flattened with dot notation (e.g., "placement.availability_zone")
//! - **Type-Aware Filtering**: Filters work with appropriate operators for each type
//!
//! # Example
//!
//! ```rust,ignore
//! use property_system::{PropertyCatalog, PropertyFilter, PropertyFilterType};
//!
//! // Discover properties from resources
//! let catalog = PropertyCatalog::new();
//! catalog.rebuild(&resources);
//!
//! // Create a filter
//! let filter = PropertyFilter::new("instance.state.name", PropertyFilterType::Equals)
//!     .with_values(vec!["running"]);
//!
//! // Apply filter
//! let matches = filter.matches(&resource, &catalog);
//! ```

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;

// ============================================================================
// Property Value Types
// ============================================================================

/// Represents a property value with type preservation
///
/// This enum maintains the actual type of the property value to enable
/// type-aware filtering and sorting.
///
/// # Memory Layout
///
/// The enum is sized to accommodate the largest variant (~32 bytes total):
/// - String: 24 bytes (Arc<str>: pointer + len + cap)
/// - Number: 8 bytes
/// - Boolean: 1 byte
/// - Date: 12 bytes
/// - IpAddress: 24 bytes (String)
/// - Enum: 48 bytes (2 strings)
/// - Null: 0 bytes (discriminant only)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    /// String value with shared reference for memory efficiency
    String(Arc<str>),

    /// Numeric value (f64 for flexibility)
    Number(f64),

    /// Boolean value
    Boolean(bool),

    /// Date/time value
    Date(DateTime<Utc>),

    /// IP address or CIDR notation
    IpAddress(String),

    /// Enum value with code and user-friendly label
    Enum {
        /// Raw enum code (e.g., "16" for EC2 instance state)
        code: String,
        /// User-friendly label (e.g., "running")
        label: String,
    },

    /// Null/missing value
    Null,
}

impl PropertyValue {
    /// Get the value as a string reference if it's a String variant
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// Get the value as a number if it's a Number variant
    pub fn as_number(&self) -> Option<f64> {
        match self {
            PropertyValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get the value as a boolean if it's a Boolean variant
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            PropertyValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get the value as a date if it's a Date variant
    pub fn as_date(&self) -> Option<DateTime<Utc>> {
        match self {
            PropertyValue::Date(d) => Some(*d),
            _ => None,
        }
    }

    /// Get the value as an IP address if it's an IpAddress variant
    pub fn as_ip_address(&self) -> Option<&str> {
        match self {
            PropertyValue::IpAddress(ip) => Some(ip),
            _ => None,
        }
    }

    /// Get the enum code and label if it's an Enum variant
    pub fn as_enum(&self) -> Option<(&str, &str)> {
        match self {
            PropertyValue::Enum { code, label } => Some((code, label)),
            _ => None,
        }
    }

    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, PropertyValue::Null)
    }

    /// Get the type name of this property value
    pub fn type_name(&self) -> &'static str {
        match self {
            PropertyValue::String(_) => "String",
            PropertyValue::Number(_) => "Number",
            PropertyValue::Boolean(_) => "Boolean",
            PropertyValue::Date(_) => "Date",
            PropertyValue::IpAddress(_) => "IpAddress",
            PropertyValue::Enum { .. } => "Enum",
            PropertyValue::Null => "Null",
        }
    }

    /// Get the PropertyType for this value
    pub fn property_type(&self) -> PropertyType {
        match self {
            PropertyValue::String(_) => PropertyType::String,
            PropertyValue::Number(_) => PropertyType::Number,
            PropertyValue::Boolean(_) => PropertyType::Boolean,
            PropertyValue::Date(_) => PropertyType::Date,
            PropertyValue::IpAddress(_) => PropertyType::IpAddress,
            PropertyValue::Enum { .. } => PropertyType::Enum,
            PropertyValue::Null => PropertyType::String, // Treat null as string for typing purposes
        }
    }

    /// Convert the value to a display string
    ///
    /// This is used for grouping and UI display
    pub fn display_string(&self) -> String {
        match self {
            PropertyValue::String(s) => s.to_string(),
            PropertyValue::Number(n) => n.to_string(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Date(d) => d.to_rfc3339(),
            PropertyValue::IpAddress(ip) => ip.clone(),
            PropertyValue::Enum { label, .. } => label.clone(),
            PropertyValue::Null => "(not set)".to_string(),
        }
    }
}

// ============================================================================
// Property Type
// ============================================================================

/// Type classification for properties
///
/// This is used to determine which filter operators are available
/// and how to sort values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyType {
    /// String value
    String,

    /// Numeric value (integers and floats)
    Number,

    /// Boolean value (true/false)
    Boolean,

    /// Date/time value
    Date,

    /// IP address or CIDR notation
    IpAddress,

    /// Enumeration with predefined values
    Enum,

    /// Property has multiple types across resources
    Mixed,
}

impl PropertyType {
    /// Get a display name for this property type
    pub fn display_name(&self) -> &'static str {
        match self {
            PropertyType::String => "String",
            PropertyType::Number => "Number",
            PropertyType::Boolean => "Boolean",
            PropertyType::Date => "Date",
            PropertyType::IpAddress => "IP Address",
            PropertyType::Enum => "Enum",
            PropertyType::Mixed => "Mixed",
        }
    }

    /// Get an icon for this property type (for UI)
    pub fn icon(&self) -> &'static str {
        match self {
            PropertyType::String => "📝",
            PropertyType::Number => "🔢",
            PropertyType::Boolean => "☑",
            PropertyType::Date => "📅",
            PropertyType::IpAddress => "🌐",
            PropertyType::Enum => "📋",
            PropertyType::Mixed => "❓",
        }
    }

    /// Get a color for this property type (for UI)
    pub fn color(&self) -> egui::Color32 {
        match self {
            PropertyType::String => egui::Color32::from_rgb(100, 150, 200),
            PropertyType::Number => egui::Color32::from_rgb(200, 100, 100),
            PropertyType::Boolean => egui::Color32::from_rgb(100, 200, 100),
            PropertyType::Date => egui::Color32::from_rgb(200, 150, 100),
            PropertyType::IpAddress => egui::Color32::from_rgb(150, 100, 200),
            PropertyType::Enum => egui::Color32::from_rgb(200, 200, 100),
            PropertyType::Mixed => egui::Color32::GRAY,
        }
    }
}

// ============================================================================
// Property Key
// ============================================================================

/// Represents a property key with metadata
///
/// This structure tracks information about a discovered property,
/// including its type, frequency, and common values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyKey {
    /// Full dot-notation path (e.g., "instance.placement.availability_zone")
    pub path: String,

    /// Detected type of this property
    pub value_type: PropertyType,

    /// How many resources have this property
    pub frequency: usize,

    /// Common values seen (for autocomplete/suggestions)
    pub common_values: Vec<String>,

    /// Value frequency map (value -> count)
    pub value_frequency: HashMap<String, usize>,
}

impl PropertyKey {
    /// Create a new property key with the given path
    pub fn new(path: String) -> Self {
        Self {
            path,
            value_type: PropertyType::String,
            frequency: 0,
            common_values: Vec::new(),
            value_frequency: HashMap::new(),
        }
    }

    /// Create a new property key with a specific type
    pub fn with_type(path: String, value_type: PropertyType) -> Self {
        Self {
            path,
            value_type,
            frequency: 0,
            common_values: Vec::new(),
            value_frequency: HashMap::new(),
        }
    }

    /// Update the frequency counter
    pub fn update_frequency(&mut self, delta: isize) {
        if delta < 0 {
            self.frequency = self.frequency.saturating_sub(delta.unsigned_abs());
        } else {
            self.frequency = self.frequency.saturating_add(delta as usize);
        }
    }

    /// Add a common value to the list
    ///
    /// Maintains a list of up to 10 most common values for autocomplete
    pub fn add_common_value(&mut self, value: String) {
        // Update frequency map
        *self.value_frequency.entry(value.clone()).or_insert(0) += 1;

        // Keep top 10 most common values
        if !self.common_values.contains(&value) {
            self.common_values.push(value);

            if self.common_values.len() > 10 {
                // Sort by frequency and keep top 10
                self.common_values.sort_by(|a, b| {
                    let freq_a = self.value_frequency.get(a).unwrap_or(&0);
                    let freq_b = self.value_frequency.get(b).unwrap_or(&0);
                    freq_b.cmp(freq_a) // Descending order
                });
                self.common_values.truncate(10);
            }
        }
    }

    /// Get a user-friendly display name for this property
    ///
    /// Converts dot notation to readable names:
    /// - "instance.state.name" -> "State Name"
    /// - "placement.availability_zone" -> "Availability Zone"
    pub fn display_name(&self) -> String {
        // Take the last component after the last dot
        let last_part = self.path.split('.').next_back().unwrap_or(&self.path);

        // Convert snake_case to Title Case
        last_part
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// ============================================================================
// Property Catalog
// ============================================================================

/// Central catalog for tracking discovered properties across visible resources
///
/// The catalog is rebuilt dynamically when visible resources change (e.g., due to
/// filtering). This ensures that only relevant properties are shown in the UI.
///
/// # Example
///
/// ```rust,ignore
/// let mut catalog = PropertyCatalog::new();
/// catalog.rebuild(&resources);
///
/// // Get all discovered property keys
/// let keys = catalog.get_keys_sorted();
///
/// // Get property value for a specific resource
/// if let Some(value) = catalog.get_property("i-001", "instance.state.name") {
///     println!("Instance state: {}", value.display_string());
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct PropertyCatalog {
    /// All discovered property keys, keyed by path
    keys: HashMap<String, PropertyKey>,

    /// Property values for each resource (resource_id -> property_path -> value)
    values: HashMap<String, HashMap<String, PropertyValue>>,

    /// Index: which resources have which properties (property_path -> Set<resource_id>)
    property_index: HashMap<String, HashSet<String>>,

    /// Last rebuild timestamp (for invalidation)
    last_updated: DateTime<Utc>,
}

impl PropertyCatalog {
    /// Create a new empty property catalog
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            values: HashMap::new(),
            property_index: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    /// Get all property keys (unordered)
    pub fn keys(&self) -> impl Iterator<Item = &PropertyKey> {
        self.keys.values()
    }

    /// Get all property keys sorted alphabetically by path
    pub fn get_keys_sorted(&self) -> Vec<&PropertyKey> {
        let mut keys: Vec<&PropertyKey> = self.keys.values().collect();
        keys.sort_by(|a, b| a.path.cmp(&b.path));
        keys
    }

    /// Get property keys filtered by prefix (for autocomplete)
    pub fn get_keys_matching(&self, prefix: &str) -> Vec<&PropertyKey> {
        let mut keys: Vec<&PropertyKey> = self
            .keys
            .values()
            .filter(|k| k.path.to_lowercase().contains(&prefix.to_lowercase()))
            .collect();
        keys.sort_by(|a, b| a.path.cmp(&b.path));
        keys
    }

    /// Get a property value for a specific resource
    pub fn get_property(&self, resource_id: &str, property_path: &str) -> Option<&PropertyValue> {
        self.values
            .get(resource_id)
            .and_then(|props| props.get(property_path))
    }

    /// Get a property value or Null if not present
    pub fn get_property_or_null(&self, resource_id: &str, property_path: &str) -> PropertyValue {
        self.get_property(resource_id, property_path)
            .cloned()
            .unwrap_or(PropertyValue::Null)
    }

    /// Get all resources that have a specific property
    pub fn get_resources_with_property(&self, property_path: &str) -> Option<&HashSet<String>> {
        self.property_index.get(property_path)
    }

    /// Mark the catalog as needing a rebuild
    pub fn invalidate(&mut self) {
        self.last_updated = Utc::now();
    }

    /// Get the last update timestamp
    pub fn last_updated(&self) -> DateTime<Utc> {
        self.last_updated
    }

    /// Add a resource to the catalog
    ///
    /// This extracts properties from the resource and updates the catalog metadata.
    pub fn add_resource(&mut self, resource_id: &str, properties: HashMap<String, PropertyValue>) {
        for (path, value) in &properties {
            // Update property key metadata
            let key = self
                .keys
                .entry(path.clone())
                .or_insert_with(|| PropertyKey::new(path.clone()));

            // Update frequency
            key.frequency += 1;

            // Update type (detect type from first value, mark Mixed if mismatch)
            let value_type = value.property_type();
            if key.frequency == 1 {
                key.value_type = value_type;
            } else if key.value_type != value_type && value_type != PropertyType::String {
                // Don't mark as Mixed if null (treated as string)
                if !value.is_null() {
                    key.value_type = PropertyType::Mixed;
                }
            }

            // Track common values
            let display = value.display_string();
            if display != "(not set)" {
                key.add_common_value(display);
            }

            // Update property index
            self.property_index
                .entry(path.clone())
                .or_default()
                .insert(resource_id.to_string());
        }

        // Store resource properties
        self.values.insert(resource_id.to_string(), properties);
    }

    /// Remove a resource from the catalog
    pub fn remove_resource(&mut self, resource_id: &str) {
        if let Some(properties) = self.values.remove(resource_id) {
            for path in properties.keys() {
                // Update property key metadata
                if let Some(key) = self.keys.get_mut(path) {
                    key.update_frequency(-1);
                }

                // Remove from property index
                if let Some(resource_set) = self.property_index.get_mut(path) {
                    resource_set.remove(resource_id);
                }
            }
        }
    }

    /// Rebuild the catalog from a list of resources
    ///
    /// This clears the catalog and rebuilds it from scratch based on the
    /// provided resources. This is called when visible resources change
    /// due to filtering.
    pub fn rebuild(&mut self, resources: &[crate::app::resource_explorer::state::ResourceEntry]) {
        // Clear existing data
        self.keys.clear();
        self.values.clear();
        self.property_index.clear();

        // Extract and add properties from each resource
        for resource in resources {
            let properties = Self::extract_properties(resource);
            self.add_resource(&resource.resource_id, properties);
        }

        self.last_updated = Utc::now();

        tracing::debug!(
            "Property catalog rebuilt: {} properties from {} resources",
            self.keys.len(),
            resources.len()
        );
    }

    // ========================================================================
    // Property Extraction
    // ========================================================================

    /// Extract all properties from a resource's JSON representation
    ///
    /// This converts the ResourceEntry to JSON and recursively flattens
    /// it using dot notation.
    fn extract_properties(
        resource: &crate::app::resource_explorer::state::ResourceEntry,
    ) -> HashMap<String, PropertyValue> {
        let mut properties = HashMap::new();

        // Convert resource to serde_json::Value
        match serde_json::to_value(resource) {
            Ok(json_value) => {
                // Recursively flatten with dot notation
                Self::flatten_json("", &json_value, &mut properties);
            }
            Err(e) => {
                tracing::warn!("Failed to convert resource to JSON: {}", e);
            }
        }

        properties
    }

    /// Recursively flatten JSON object with dot notation
    ///
    /// This converts nested JSON structures into flat key-value pairs:
    /// - `{"instance": {"state": {"name": "running"}}}` → `instance.state.name = "running"`
    /// - Arrays are flattened by extracting all values (see flatten_array)
    fn flatten_json(
        prefix: &str,
        value: &serde_json::Value,
        output: &mut HashMap<String, PropertyValue>,
    ) {
        use serde_json::Value;

        match value {
            Value::Object(map) => {
                // Recursively flatten nested objects
                for (key, val) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten_json(&path, val, output);
                }
            }
            Value::Array(arr) => {
                // Flatten array values
                Self::flatten_array(prefix, arr, output);
            }
            _ => {
                // Leaf value - detect type and store
                if !prefix.is_empty() {
                    if let Some(prop_value) = Self::json_to_property_value(value) {
                        output.insert(prefix.to_string(), prop_value);
                    }
                }
            }
        }
    }

    // ========================================================================
    // Type Detection
    // ========================================================================

    /// Convert a JSON value to a PropertyValue with type detection
    ///
    /// This detects the appropriate PropertyValue variant based on the JSON value
    /// and its content. Special handling for:
    /// - Dates (ISO 8601 / RFC 3339 format)
    /// - IP addresses and CIDR notation
    /// - AWS enum codes
    fn json_to_property_value(value: &serde_json::Value) -> Option<PropertyValue> {
        use serde_json::Value;

        match value {
            Value::String(s) => {
                // Detect special string types
                if s.is_empty() {
                    return Some(PropertyValue::Null);
                }

                // Check for date/time (ISO 8601 / RFC 3339)
                if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                    return Some(PropertyValue::Date(dt.with_timezone(&Utc)));
                }

                // Check for IP address or CIDR
                if Self::is_ip_or_cidr(s) {
                    return Some(PropertyValue::IpAddress(s.clone()));
                }

                // Check for enum (AWS state codes, etc.)
                // For now, treat all strings as regular strings
                // Enum detection can be enhanced based on property path in the future
                Some(PropertyValue::String(s.clone().into()))
            }
            Value::Number(n) => {
                // Convert to f64
                n.as_f64().map(PropertyValue::Number)
            }
            Value::Bool(b) => Some(PropertyValue::Boolean(*b)),
            Value::Null => Some(PropertyValue::Null),
            Value::Array(_) | Value::Object(_) => {
                // Skip complex types (handled by flattening)
                None
            }
        }
    }

    /// Check if a string is a valid IP address or CIDR notation
    fn is_ip_or_cidr(s: &str) -> bool {
        use std::net::IpAddr;

        // Check if it's a plain IP address
        if s.parse::<IpAddr>().is_ok() {
            return true;
        }

        // Check if it's CIDR notation (IP/prefix)
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            if let Ok(prefix_len) = parts[1].parse::<u8>() {
                if parts[0].parse::<IpAddr>().is_ok() {
                    // Validate prefix length
                    if parts[0].contains(':') {
                        // IPv6: prefix must be <= 128
                        return prefix_len <= 128;
                    } else {
                        // IPv4: prefix must be <= 32
                        return prefix_len <= 32;
                    }
                }
            }
        }

        false
    }

    // ========================================================================
    // Array Handling
    // ========================================================================

    /// Flatten array values for searching
    ///
    /// Arrays are flattened by:
    /// 1. Extracting all primitive values (strings, numbers, booleans)
    /// 2. Recursively flattening nested objects within arrays
    /// 3. Combining primitive values into a comma-separated string for "contains" matching
    ///
    /// Example:
    /// - `["sg-123", "sg-456"]` → property value `"sg-123,sg-456"`
    /// - Filtering with "contains sg-123" will match
    /// - Filtering with "in [sg-456, sg-789]" will match
    fn flatten_array(
        prefix: &str,
        arr: &[serde_json::Value],
        output: &mut HashMap<String, PropertyValue>,
    ) {
        use serde_json::Value;

        let mut primitive_values = Vec::new();

        for item in arr {
            match item {
                Value::String(s) => {
                    if !s.is_empty() {
                        primitive_values.push(s.clone());
                    }
                }
                Value::Number(n) => {
                    primitive_values.push(n.to_string());
                }
                Value::Bool(b) => {
                    primitive_values.push(b.to_string());
                }
                Value::Object(_) => {
                    // Recursively flatten nested objects in array
                    // Don't add array index to path - treat all array elements equally
                    Self::flatten_json(prefix, item, output);
                }
                Value::Array(_) => {
                    // Nested arrays - flatten recursively
                    if let Value::Array(nested) = item {
                        Self::flatten_array(prefix, nested, output);
                    }
                }
                Value::Null => {
                    // Skip null values in arrays
                }
            }
        }

        // Store primitive values as comma-separated string
        if !primitive_values.is_empty() {
            let combined = primitive_values.join(",");
            output.insert(prefix.to_string(), PropertyValue::String(combined.into()));
        }
    }
}

// ============================================================================
// Property Filtering (M2)
// ============================================================================

/// Type of property filter operation
///
/// This includes all TagFilterType operations plus property-specific types:
/// - GreaterThan/LessThan for numeric/date comparisons
/// - InSubnet for IP/CIDR network matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyFilterType {
    /// Property value equals specific value(s)
    Equals,
    /// Property value does not equal specific value(s)
    NotEquals,
    /// Property value contains substring
    Contains,
    /// Property value does not contain substring
    NotContains,
    /// Property value starts with prefix
    StartsWith,
    /// Property value ends with suffix
    EndsWith,
    /// Property value matches regex pattern
    Regex,
    /// Property exists on resource (any value)
    Exists,
    /// Property does not exist on resource
    NotExists,
    /// Property value is in list of values
    In,
    /// Property value is not in list of values
    NotIn,
    /// Property value is greater than threshold (numeric/date)
    GreaterThan,
    /// Property value is less than threshold (numeric/date)
    LessThan,
    /// IP address is in subnet/CIDR range
    InSubnet,
}

impl PropertyFilterType {
    /// Get a human-readable display name for the filter type
    pub fn display_name(&self) -> &'static str {
        match self {
            PropertyFilterType::Equals => "Equals",
            PropertyFilterType::NotEquals => "Not Equals",
            PropertyFilterType::Contains => "Contains",
            PropertyFilterType::NotContains => "Not Contains",
            PropertyFilterType::StartsWith => "Starts With",
            PropertyFilterType::EndsWith => "Ends With",
            PropertyFilterType::Regex => "Matches Regex",
            PropertyFilterType::Exists => "Exists",
            PropertyFilterType::NotExists => "Does Not Exist",
            PropertyFilterType::In => "Is In",
            PropertyFilterType::NotIn => "Is Not In",
            PropertyFilterType::GreaterThan => "Greater Than",
            PropertyFilterType::LessThan => "Less Than",
            PropertyFilterType::InSubnet => "In Subnet",
        }
    }

    /// Get an icon for this filter type
    pub fn icon(&self) -> &'static str {
        match self {
            PropertyFilterType::Equals => "=",
            PropertyFilterType::NotEquals => "≠",
            PropertyFilterType::Contains => "⊃",
            PropertyFilterType::NotContains => "⊅",
            PropertyFilterType::StartsWith => "⋯←",
            PropertyFilterType::EndsWith => "→⋯",
            PropertyFilterType::Regex => ".*",
            PropertyFilterType::Exists => "✓",
            PropertyFilterType::NotExists => "✗",
            PropertyFilterType::In => "∈",
            PropertyFilterType::NotIn => "∉",
            PropertyFilterType::GreaterThan => ">",
            PropertyFilterType::LessThan => "<",
            PropertyFilterType::InSubnet => "🌐",
        }
    }

    /// Check if this filter type requires a value
    pub fn requires_value(&self) -> bool {
        !matches!(
            self,
            PropertyFilterType::Exists | PropertyFilterType::NotExists
        )
    }

    /// Check if this filter type supports multiple values
    pub fn supports_multiple_values(&self) -> bool {
        matches!(self, PropertyFilterType::In | PropertyFilterType::NotIn)
    }

    /// Check if this filter type is type-aware (requires specific property types)
    pub fn is_type_aware(&self) -> bool {
        matches!(
            self,
            PropertyFilterType::GreaterThan
                | PropertyFilterType::LessThan
                | PropertyFilterType::InSubnet
        )
    }

    /// Get the compatible property types for this filter
    pub fn compatible_types(&self) -> Option<Vec<PropertyType>> {
        match self {
            PropertyFilterType::GreaterThan | PropertyFilterType::LessThan => {
                Some(vec![PropertyType::Number, PropertyType::Date])
            }
            PropertyFilterType::InSubnet => Some(vec![PropertyType::IpAddress]),
            _ => None, // All other filters work on any type (coerced to string)
        }
    }

    /// Get all available filter types
    pub fn all() -> Vec<PropertyFilterType> {
        vec![
            PropertyFilterType::Equals,
            PropertyFilterType::NotEquals,
            PropertyFilterType::Contains,
            PropertyFilterType::NotContains,
            PropertyFilterType::StartsWith,
            PropertyFilterType::EndsWith,
            PropertyFilterType::Regex,
            PropertyFilterType::Exists,
            PropertyFilterType::NotExists,
            PropertyFilterType::In,
            PropertyFilterType::NotIn,
            PropertyFilterType::GreaterThan,
            PropertyFilterType::LessThan,
            PropertyFilterType::InSubnet,
        ]
    }
}

/// A single property filter
///
/// Filters resources based on property values with type-aware comparison.
/// Similar to TagFilter but works on resource JSON properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyFilter {
    /// Property path to filter on (e.g., "instance.state.name")
    pub property_path: String,
    /// Type of filter operation
    pub filter_type: PropertyFilterType,
    /// Values to compare against (for Equals, In, etc.)
    /// Empty for Exists/NotExists filters
    pub values: Vec<String>,
    /// Regex pattern (for Regex filter type)
    pub pattern: Option<String>,
    /// Expected property type (for type-aware filters like GreaterThan)
    pub expected_type: Option<PropertyType>,
}

impl PropertyFilter {
    /// Create a new property filter
    pub fn new(property_path: String, filter_type: PropertyFilterType) -> Self {
        Self {
            property_path,
            filter_type,
            values: Vec::new(),
            pattern: None,
            expected_type: None,
        }
    }

    /// Create a filter with a single value
    pub fn with_value(
        property_path: String,
        filter_type: PropertyFilterType,
        value: String,
    ) -> Self {
        Self {
            property_path,
            filter_type,
            values: vec![value],
            pattern: None,
            expected_type: None,
        }
    }

    /// Create a filter with multiple values
    pub fn with_values(
        property_path: String,
        filter_type: PropertyFilterType,
        values: Vec<String>,
    ) -> Self {
        Self {
            property_path,
            filter_type,
            values,
            pattern: None,
            expected_type: None,
        }
    }

    /// Create a regex filter
    pub fn with_regex(property_path: String, pattern: String) -> Self {
        Self {
            property_path,
            filter_type: PropertyFilterType::Regex,
            values: Vec::new(),
            pattern: Some(pattern),
            expected_type: None,
        }
    }

    /// Create an exists filter
    pub fn exists(property_path: String) -> Self {
        Self::new(property_path, PropertyFilterType::Exists)
    }

    /// Create a not-exists filter
    pub fn not_exists(property_path: String) -> Self {
        Self::new(property_path, PropertyFilterType::NotExists)
    }

    /// Create a type-aware filter (GreaterThan, LessThan, InSubnet)
    pub fn with_type(
        property_path: String,
        filter_type: PropertyFilterType,
        value: String,
        expected_type: PropertyType,
    ) -> Self {
        Self {
            property_path,
            filter_type,
            values: vec![value],
            pattern: None,
            expected_type: Some(expected_type),
        }
    }

    /// Get a human-readable description of this filter
    pub fn description(&self) -> String {
        let path_display = self.property_path.replace('.', " → ");
        match self.filter_type {
            PropertyFilterType::Exists => format!("{} exists", path_display),
            PropertyFilterType::NotExists => format!("{} does not exist", path_display),
            PropertyFilterType::Regex => {
                if let Some(pattern) = &self.pattern {
                    format!("{} matches /{}/", path_display, pattern)
                } else {
                    format!("{} matches regex", path_display)
                }
            }
            PropertyFilterType::In | PropertyFilterType::NotIn => {
                let op = if self.filter_type == PropertyFilterType::In {
                    "in"
                } else {
                    "not in"
                };
                format!("{} {} [{}]", path_display, op, self.values.join(", "))
            }
            _ => {
                let value = self.values.first().map(|v| v.as_str()).unwrap_or("");
                format!(
                    "{} {} {}",
                    path_display,
                    self.filter_type.display_name().to_lowercase(),
                    value
                )
            }
        }
    }

    /// Check if this filter is valid
    pub fn is_valid(&self) -> bool {
        // Check if required value is present
        if self.filter_type.requires_value() && self.values.is_empty() && self.pattern.is_none() {
            return false;
        }

        // Check if type-aware filters have expected type
        if self.filter_type.is_type_aware() && self.expected_type.is_none() {
            return false;
        }

        // Check if expected type is compatible
        if let Some(expected_type) = self.expected_type {
            if let Some(compatible_types) = self.filter_type.compatible_types() {
                if !compatible_types.contains(&expected_type) {
                    return false;
                }
            }
        }

        true
    }

    // ========================================================================
    // Filter Matching
    // ========================================================================

    /// Check if a property value matches this filter
    ///
    /// This performs type-aware comparison based on the filter type and property value type.
    pub fn matches(&self, value: &PropertyValue) -> bool {
        match self.filter_type {
            PropertyFilterType::Exists => !value.is_null(),
            PropertyFilterType::NotExists => value.is_null(),

            PropertyFilterType::Equals => {
                if value.is_null() {
                    return false;
                }
                self.values
                    .iter()
                    .any(|v| self.compare_values(value, v, CompareOp::Equal))
            }

            PropertyFilterType::NotEquals => {
                if value.is_null() {
                    return true; // Null is not equal to anything
                }
                !self
                    .values
                    .iter()
                    .any(|v| self.compare_values(value, v, CompareOp::Equal))
            }

            PropertyFilterType::Contains => {
                if value.is_null() {
                    return false;
                }
                let value_str = value.display_string();
                self.values.iter().any(|v| value_str.contains(v))
            }

            PropertyFilterType::NotContains => {
                if value.is_null() {
                    return true;
                }
                let value_str = value.display_string();
                !self.values.iter().any(|v| value_str.contains(v))
            }

            PropertyFilterType::StartsWith => {
                if value.is_null() {
                    return false;
                }
                let value_str = value.display_string();
                self.values.iter().any(|v| value_str.starts_with(v))
            }

            PropertyFilterType::EndsWith => {
                if value.is_null() {
                    return false;
                }
                let value_str = value.display_string();
                self.values.iter().any(|v| value_str.ends_with(v))
            }

            PropertyFilterType::Regex => {
                if value.is_null() {
                    return false;
                }
                if let Some(pattern) = &self.pattern {
                    if let Ok(re) = Regex::new(pattern) {
                        let value_str = value.display_string();
                        return re.is_match(&value_str);
                    }
                }
                false
            }

            PropertyFilterType::In => {
                if value.is_null() {
                    return false;
                }
                self.values
                    .iter()
                    .any(|v| self.compare_values(value, v, CompareOp::Equal))
            }

            PropertyFilterType::NotIn => {
                if value.is_null() {
                    return true;
                }
                !self
                    .values
                    .iter()
                    .any(|v| self.compare_values(value, v, CompareOp::Equal))
            }

            PropertyFilterType::GreaterThan => {
                if value.is_null() || self.values.is_empty() {
                    return false;
                }
                self.compare_values(value, &self.values[0], CompareOp::GreaterThan)
            }

            PropertyFilterType::LessThan => {
                if value.is_null() || self.values.is_empty() {
                    return false;
                }
                self.compare_values(value, &self.values[0], CompareOp::LessThan)
            }

            PropertyFilterType::InSubnet => {
                if value.is_null() || self.values.is_empty() {
                    return false;
                }
                // Future enhancement: network operators
                self.compare_subnet(value, &self.values[0])
            }
        }
    }

    /// Compare two values with type awareness
    fn compare_values(
        &self,
        property_value: &PropertyValue,
        filter_value: &str,
        op: CompareOp,
    ) -> bool {
        match property_value {
            PropertyValue::String(s) => match op {
                CompareOp::Equal => s.as_ref() == filter_value,
                CompareOp::GreaterThan => s.as_ref() > filter_value,
                CompareOp::LessThan => s.as_ref() < filter_value,
            },

            PropertyValue::Number(n) => {
                if let Ok(filter_num) = filter_value.parse::<f64>() {
                    match op {
                        CompareOp::Equal => (*n - filter_num).abs() < f64::EPSILON,
                        CompareOp::GreaterThan => *n > filter_num,
                        CompareOp::LessThan => *n < filter_num,
                    }
                } else {
                    false
                }
            }

            PropertyValue::Boolean(b) => {
                if let Ok(filter_bool) = filter_value.parse::<bool>() {
                    match op {
                        CompareOp::Equal => *b == filter_bool,
                        _ => false, // GT/LT don't make sense for booleans
                    }
                } else {
                    false
                }
            }

            PropertyValue::Date(dt) => {
                if let Ok(filter_dt) = DateTime::parse_from_rfc3339(filter_value) {
                    let filter_utc = filter_dt.with_timezone(&Utc);
                    match op {
                        CompareOp::Equal => *dt == filter_utc,
                        CompareOp::GreaterThan => *dt > filter_utc,
                        CompareOp::LessThan => *dt < filter_utc,
                    }
                } else {
                    false
                }
            }

            PropertyValue::IpAddress(ip) => {
                match op {
                    CompareOp::Equal => ip == filter_value,
                    _ => false, // GT/LT don't make sense for IPs
                }
            }

            PropertyValue::Enum { code, label } => match op {
                CompareOp::Equal => code == filter_value || label == filter_value,
                _ => false,
            },

            PropertyValue::Null => false,
        }
    }

    // ========================================================================
    // Network Operators
    // ========================================================================

    /// Compare IP address against subnet/CIDR
    ///
    /// Supports both IPv4 and IPv6 addresses and CIDR notation.
    /// Examples: "10.0.0.5" in "10.0.0.0/24", "2001:db8::1" in "2001:db8::/32"
    fn compare_subnet(&self, value: &PropertyValue, subnet: &str) -> bool {
        let ip_str = match value {
            PropertyValue::IpAddress(ip) => ip.as_str(),
            PropertyValue::String(s) => s.as_ref(),
            _ => return false,
        };

        // Parse the IP address
        let ip_addr = match ip_str.parse::<IpAddr>() {
            Ok(addr) => addr,
            Err(_) => return false,
        };

        // Parse subnet/CIDR
        if let Some((network_str, prefix_str)) = subnet.split_once('/') {
            // CIDR notation
            let network_addr = match network_str.parse::<IpAddr>() {
                Ok(addr) => addr,
                Err(_) => return false,
            };

            let prefix_len = match prefix_str.parse::<u8>() {
                Ok(len) => len,
                Err(_) => return false,
            };

            // Check if IP types match (v4 vs v6)
            match (ip_addr, network_addr) {
                (IpAddr::V4(ip), IpAddr::V4(network)) => {
                    Self::ipv4_in_subnet(ip, network, prefix_len)
                }
                (IpAddr::V6(ip), IpAddr::V6(network)) => {
                    Self::ipv6_in_subnet(ip, network, prefix_len)
                }
                _ => false, // Mismatched IP versions
            }
        } else {
            // Plain IP address (exact match)
            match subnet.parse::<IpAddr>() {
                Ok(subnet_addr) => ip_addr == subnet_addr,
                Err(_) => false,
            }
        }
    }

    /// Check if IPv4 address is in subnet
    fn ipv4_in_subnet(ip: std::net::Ipv4Addr, network: std::net::Ipv4Addr, prefix_len: u8) -> bool {
        if prefix_len > 32 {
            return false;
        }

        let ip_bits = u32::from(ip);
        let network_bits = u32::from(network);

        if prefix_len == 0 {
            return true; // 0.0.0.0/0 matches all
        }

        let mask = !0u32 << (32 - prefix_len);
        (ip_bits & mask) == (network_bits & mask)
    }

    /// Check if IPv6 address is in subnet
    fn ipv6_in_subnet(ip: std::net::Ipv6Addr, network: std::net::Ipv6Addr, prefix_len: u8) -> bool {
        if prefix_len > 128 {
            return false;
        }

        let ip_bits = u128::from(ip);
        let network_bits = u128::from(network);

        if prefix_len == 0 {
            return true; // ::/0 matches all
        }

        let mask = !0u128 << (128 - prefix_len);
        (ip_bits & mask) == (network_bits & mask)
    }
}

/// Comparison operation for type-aware value comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompareOp {
    Equal,
    GreaterThan,
    LessThan,
}

/// A group of property filters combined with boolean logic
///
/// Allows building complex filter expressions with AND/OR logic and nested groups.
/// Similar to TagFilterGroup but works with PropertyFilter instances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyFilterGroup {
    /// Boolean operator for combining filters (AND/OR)
    pub operator: crate::app::resource_explorer::state::BooleanOperator,
    /// Individual property filters in this group
    pub filters: Vec<PropertyFilter>,
    /// Nested sub-groups for complex boolean logic
    pub sub_groups: Vec<PropertyFilterGroup>,
}

impl Default for PropertyFilterGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyFilterGroup {
    /// Create a new empty filter group with AND operator
    pub fn new() -> Self {
        Self {
            operator: crate::app::resource_explorer::state::BooleanOperator::And,
            filters: Vec::new(),
            sub_groups: Vec::new(),
        }
    }

    /// Create a new filter group with OR operator
    pub fn new_or() -> Self {
        Self {
            operator: crate::app::resource_explorer::state::BooleanOperator::Or,
            filters: Vec::new(),
            sub_groups: Vec::new(),
        }
    }

    /// Create a filter group with specific operator
    pub fn with_operator(operator: crate::app::resource_explorer::state::BooleanOperator) -> Self {
        Self {
            operator,
            filters: Vec::new(),
            sub_groups: Vec::new(),
        }
    }

    /// Add a filter to this group
    pub fn add_filter(&mut self, filter: PropertyFilter) {
        self.filters.push(filter);
    }

    /// Add multiple filters to this group
    pub fn add_filters(&mut self, filters: Vec<PropertyFilter>) {
        self.filters.extend(filters);
    }

    /// Add a sub-group to this group
    pub fn add_sub_group(&mut self, sub_group: PropertyFilterGroup) {
        self.sub_groups.push(sub_group);
    }

    /// Remove a filter at the given index
    pub fn remove_filter(&mut self, index: usize) {
        if index < self.filters.len() {
            self.filters.remove(index);
        }
    }

    /// Remove a sub-group at the given index
    pub fn remove_sub_group(&mut self, index: usize) {
        if index < self.sub_groups.len() {
            self.sub_groups.remove(index);
        }
    }

    /// Clear all filters and sub-groups
    pub fn clear(&mut self) {
        self.filters.clear();
        self.sub_groups.clear();
    }

    /// Check if this group is empty (no filters or sub-groups)
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty() && self.sub_groups.is_empty()
    }

    /// Get the total number of filters (including nested)
    pub fn total_filter_count(&self) -> usize {
        let direct_count = self.filters.len();
        let nested_count: usize = self.sub_groups.iter().map(|g| g.total_filter_count()).sum();
        direct_count + nested_count
    }

    /// Get the maximum nesting depth
    pub fn max_depth(&self) -> usize {
        if self.sub_groups.is_empty() {
            1
        } else {
            1 + self
                .sub_groups
                .iter()
                .map(|g| g.max_depth())
                .max()
                .unwrap_or(0)
        }
    }

    /// Check if all filters in this group are valid
    pub fn is_valid(&self) -> bool {
        // Check direct filters
        if !self.filters.iter().all(|f| f.is_valid()) {
            return false;
        }

        // Check sub-groups
        if !self.sub_groups.iter().all(|g| g.is_valid()) {
            return false;
        }

        true
    }

    /// Get a human-readable description of this filter group
    pub fn description(&self) -> String {
        if self.is_empty() {
            return "No filters".to_string();
        }

        let mut parts = Vec::new();

        // Add filter descriptions
        for filter in &self.filters {
            parts.push(filter.description());
        }

        // Add sub-group descriptions
        for sub_group in &self.sub_groups {
            let sub_desc = sub_group.description();
            parts.push(format!("({})", sub_desc));
        }

        // Join with operator
        let op = match self.operator {
            crate::app::resource_explorer::state::BooleanOperator::And => " AND ",
            crate::app::resource_explorer::state::BooleanOperator::Or => " OR ",
        };

        parts.join(op)
    }

    /// Convert to a simplified single-line summary
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "No filters".to_string();
        }

        let total = self.total_filter_count();
        let depth = self.max_depth();
        let op_name = match self.operator {
            crate::app::resource_explorer::state::BooleanOperator::And => "AND",
            crate::app::resource_explorer::state::BooleanOperator::Or => "OR",
        };

        if depth == 1 {
            format!("{} filter(s) with {}", total, op_name)
        } else {
            format!("{} filter(s) with {} (depth {})", total, op_name, depth)
        }
    }

    // ========================================================================
    // Filter Evaluation
    // ========================================================================

    /// Evaluate this filter group against a resource
    ///
    /// Returns true if the resource matches the filter criteria based on the boolean operator.
    pub fn matches(&self, resource_id: &str, catalog: &PropertyCatalog) -> bool {
        // Empty group matches everything
        if self.is_empty() {
            return true;
        }

        match self.operator {
            crate::app::resource_explorer::state::BooleanOperator::And => {
                // All filters must match
                let filters_match = self
                    .filters
                    .iter()
                    .all(|filter| self.evaluate_filter(filter, resource_id, catalog));

                let sub_groups_match = self
                    .sub_groups
                    .iter()
                    .all(|group| group.matches(resource_id, catalog));

                filters_match && sub_groups_match
            }
            crate::app::resource_explorer::state::BooleanOperator::Or => {
                // At least one filter or sub-group must match
                let any_filter_matches = self
                    .filters
                    .iter()
                    .any(|filter| self.evaluate_filter(filter, resource_id, catalog));

                let any_sub_group_matches = self
                    .sub_groups
                    .iter()
                    .any(|group| group.matches(resource_id, catalog));

                any_filter_matches || any_sub_group_matches
            }
        }
    }

    /// Evaluate a single filter against a resource
    fn evaluate_filter(
        &self,
        filter: &PropertyFilter,
        resource_id: &str,
        catalog: &PropertyCatalog,
    ) -> bool {
        // Get the property value from the catalog
        let value = catalog.get_property_or_null(resource_id, &filter.property_path);

        // Check if filter matches the value
        filter.matches(&value)
    }

    /// Filter a list of resources, returning only those that match
    pub fn filter_resources<'a>(
        &self,
        resources: &'a [crate::app::resource_explorer::state::ResourceEntry],
        catalog: &PropertyCatalog,
    ) -> Vec<&'a crate::app::resource_explorer::state::ResourceEntry> {
        if self.is_empty() {
            return resources.iter().collect();
        }

        resources
            .iter()
            .filter(|resource| self.matches(&resource.resource_id, catalog))
            .collect()
    }

    /// Count how many resources match this filter group
    pub fn count_matches(
        &self,
        resources: &[crate::app::resource_explorer::state::ResourceEntry],
        catalog: &PropertyCatalog,
    ) -> usize {
        if self.is_empty() {
            return resources.len();
        }

        resources
            .iter()
            .filter(|resource| self.matches(&resource.resource_id, catalog))
            .count()
    }
}
