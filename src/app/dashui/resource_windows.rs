use crate::app::cfn_resources::PropertyDefinitionMap;

pub struct ResourceTypesWindow;
pub struct ResourceDetailsWindow;

/// Struct to represent a property type window
pub struct PropertyTypeWindow {
    /// The property type name
    pub property_type: String,
    /// Whether this window should be displayed
    pub show: bool,
    /// Properties of the property type
    pub properties: Option<PropertyDefinitionMap>,
}
