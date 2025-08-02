use crate::app::cfn_template::{CloudFormationTemplate, Parameter};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info};

/// Enhanced parameter information for UI display and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub parameter_type: String,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub allowed_pattern: Option<String>,
    pub constraint_description: Option<String>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub no_echo: bool,
    pub is_aws_specific: bool,
    pub aws_resource_type: Option<String>,
    pub is_sensitive: bool,
    pub validation_hints: Vec<String>,
}

/// Parameter input UI component type
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterInputType {
    Text,
    TextArea,
    Number,
    Select,
    MultiSelect,
    AwsResourcePicker,
    SecretInput,
    FileUpload,
}

/// Discovered parameter dependencies and relationships
#[derive(Debug, Clone)]
pub struct ParameterDependencies {
    pub parameter_name: String,
    pub depends_on: Vec<String>,
    pub referenced_by: Vec<String>,
    pub conditional_parameters: Vec<String>,
}

/// Parameter discovery and analysis system
pub struct ParameterDiscovery {
    template: Option<CloudFormationTemplate>,
    parameters: HashMap<String, ParameterInfo>,
    dependencies: HashMap<String, ParameterDependencies>,
}

impl ParameterDiscovery {
    pub fn new() -> Self {
        Self {
            template: None,
            parameters: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Load and analyze a CloudFormation template
    pub fn load_template(&mut self, template: CloudFormationTemplate) -> Result<()> {
        info!("Loading CloudFormation template for parameter discovery");

        // Clear previous state
        self.parameters.clear();
        self.dependencies.clear();

        // Analyze template parameters
        self.analyze_parameters(&template)?;

        // Analyze parameter dependencies
        self.analyze_dependencies(&template)?;

        self.template = Some(template);
        info!(
            "Template loaded successfully with {} parameters",
            self.parameters.len()
        );

        Ok(())
    }

    /// Get all discovered parameters
    pub fn get_parameters(&self) -> &HashMap<String, ParameterInfo> {
        &self.parameters
    }

    /// Get parameter dependencies
    pub fn get_dependencies(&self) -> &HashMap<String, ParameterDependencies> {
        &self.dependencies
    }

    /// Get parameters sorted by dependencies (dependencies first)
    pub fn get_parameters_sorted(&self) -> Vec<&ParameterInfo> {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Simple topological sort - dependencies first
        for param_name in self.parameters.keys() {
            self.visit_parameter_dependencies(param_name, &mut visited, &mut sorted);
        }

        sorted
    }

    /// Get the appropriate input type for a parameter
    pub fn get_input_type(&self, parameter_name: &str) -> ParameterInputType {
        if let Some(param) = self.parameters.get(parameter_name) {
            self.determine_input_type(param)
        } else {
            ParameterInputType::Text
        }
    }

    /// Validate a parameter value against its constraints
    pub fn validate_parameter_value(&self, parameter_name: &str, value: &str) -> Result<()> {
        if let Some(param) = self.parameters.get(parameter_name) {
            self.validate_value(param, value)
        } else {
            Ok(()) // Parameter not found, assume valid
        }
    }

    /// Analyze template parameters and create enhanced parameter info
    fn analyze_parameters(&mut self, template: &CloudFormationTemplate) -> Result<()> {
        for (name, param) in &template.parameters {
            debug!("Analyzing parameter: {}", name);

            let param_info = ParameterInfo {
                name: name.clone(),
                parameter_type: param.parameter_type.clone(),
                description: param.description.clone(),
                default_value: param.default.as_ref().map(|v| self.value_to_string(v)),
                allowed_values: param
                    .allowed_values
                    .as_ref()
                    .map(|vals| vals.iter().map(|v| self.value_to_string(v)).collect()),
                allowed_pattern: param.allowed_pattern.clone(),
                constraint_description: param.constraint_description.clone(),
                min_length: param.min_length,
                max_length: param.max_length,
                min_value: param.min_value,
                max_value: param.max_value,
                no_echo: param.no_echo.unwrap_or(false),
                is_aws_specific: self.is_aws_specific_type(&param.parameter_type),
                aws_resource_type: self.get_aws_resource_type(&param.parameter_type),
                is_sensitive: self.is_sensitive_parameter(name, param),
                validation_hints: self.generate_validation_hints(param),
            };

            self.parameters.insert(name.clone(), param_info);
        }

        Ok(())
    }

    /// Analyze parameter dependencies within the template
    fn analyze_dependencies(&mut self, template: &CloudFormationTemplate) -> Result<()> {
        // For now, implement basic dependency tracking
        // In the future, this could analyze Ref and GetAtt functions to find parameter usage

        for param_name in template.parameters.keys() {
            let dependencies = ParameterDependencies {
                parameter_name: param_name.clone(),
                depends_on: Vec::new(), // TODO: Analyze template for parameter references
                referenced_by: Vec::new(),
                conditional_parameters: Vec::new(),
            };

            self.dependencies.insert(param_name.clone(), dependencies);
        }

        Ok(())
    }

    /// Visit parameter dependencies for topological sort
    fn visit_parameter_dependencies<'a>(
        &'a self,
        param_name: &str,
        visited: &mut std::collections::HashSet<String>,
        sorted: &mut Vec<&'a ParameterInfo>,
    ) {
        if visited.contains(param_name) {
            return;
        }

        visited.insert(param_name.to_string());

        // Visit dependencies first
        if let Some(deps) = self.dependencies.get(param_name) {
            for dep in &deps.depends_on {
                self.visit_parameter_dependencies(dep, visited, sorted);
            }
        }

        // Add this parameter to sorted list
        if let Some(param) = self.parameters.get(param_name) {
            sorted.push(param);
        }
    }

    /// Determine if a parameter type is AWS-specific
    fn is_aws_specific_type(&self, param_type: &str) -> bool {
        param_type.starts_with("AWS::")
    }

    /// Extract AWS resource type from parameter type
    fn get_aws_resource_type(&self, param_type: &str) -> Option<String> {
        if param_type.starts_with("AWS::") {
            // Extract resource type from parameter type
            // e.g., "AWS::EC2::VPC::Id" -> "AWS::EC2::VPC"
            if let Some(id_pos) = param_type.rfind("::") {
                let resource_type = &param_type[..id_pos];
                if resource_type != param_type {
                    return Some(resource_type.to_string());
                }
            }
        }
        None
    }

    /// Determine if a parameter contains sensitive data
    fn is_sensitive_parameter(&self, name: &str, param: &Parameter) -> bool {
        // Check NoEcho flag
        if param.no_echo.unwrap_or(false) {
            return true;
        }

        // Check parameter name for common sensitive patterns
        let name_lower = name.to_lowercase();
        let sensitive_patterns = [
            "password",
            "secret",
            "key",
            "token",
            "credential",
            "auth",
            "api_key",
            "access_key",
            "private",
            "cert",
            "certificate",
        ];

        sensitive_patterns
            .iter()
            .any(|pattern| name_lower.contains(pattern))
    }

    /// Generate validation hints for a parameter
    fn generate_validation_hints(&self, param: &Parameter) -> Vec<String> {
        let mut hints = Vec::new();

        if let Some(pattern) = &param.allowed_pattern {
            hints.push(format!("Must match pattern: {}", pattern));
        }

        if let Some(min_len) = param.min_length {
            hints.push(format!("Minimum length: {} characters", min_len));
        }

        if let Some(max_len) = param.max_length {
            hints.push(format!("Maximum length: {} characters", max_len));
        }

        if let Some(min_val) = param.min_value {
            hints.push(format!("Minimum value: {}", min_val));
        }

        if let Some(max_val) = param.max_value {
            hints.push(format!("Maximum value: {}", max_val));
        }

        if let Some(allowed_values) = &param.allowed_values {
            if allowed_values.len() <= 10 {
                let values: Vec<String> = allowed_values
                    .iter()
                    .map(|v| self.value_to_string(v))
                    .collect();
                hints.push(format!("Allowed values: {}", values.join(", ")));
            } else {
                hints.push(format!(
                    "Must be one of {} predefined values",
                    allowed_values.len()
                ));
            }
        }

        hints
    }

    /// Determine the appropriate input type for a parameter
    fn determine_input_type(&self, param: &ParameterInfo) -> ParameterInputType {
        // Sensitive parameters get secure input
        if param.is_sensitive || param.no_echo {
            return ParameterInputType::SecretInput;
        }

        // AWS resource types get special picker
        if param.is_aws_specific && param.aws_resource_type.is_some() {
            return ParameterInputType::AwsResourcePicker;
        }

        // Parameters with allowed values get select dropdown
        if let Some(allowed_values) = &param.allowed_values {
            if allowed_values.len() <= 20 {
                return ParameterInputType::Select;
            } else {
                return ParameterInputType::MultiSelect;
            }
        }

        // Number types
        if param.parameter_type == "Number" || param.parameter_type.starts_with("List<Number>") {
            return ParameterInputType::Number;
        }

        // CommaDelimitedList gets text area
        if param.parameter_type == "CommaDelimitedList" {
            return ParameterInputType::TextArea;
        }

        // Large text fields get text area
        if let Some(max_len) = param.max_length {
            if max_len > 100 {
                return ParameterInputType::TextArea;
            }
        }

        // Default to text input
        ParameterInputType::Text
    }

    /// Validate a parameter value against its constraints
    fn validate_value(&self, param: &ParameterInfo, value: &str) -> Result<()> {
        // Check allowed values
        if let Some(allowed_values) = &param.allowed_values {
            if !allowed_values.contains(&value.to_string()) {
                return Err(anyhow::anyhow!(
                    "Value '{}' is not in allowed values: {}",
                    value,
                    allowed_values.join(", ")
                ));
            }
        }

        // Check pattern
        if let Some(pattern) = &param.allowed_pattern {
            let regex = regex::Regex::new(pattern)
                .with_context(|| format!("Invalid regex pattern: {}", pattern))?;
            if !regex.is_match(value) {
                return Err(anyhow::anyhow!(
                    "Value '{}' does not match required pattern: {}",
                    value,
                    pattern
                ));
            }
        }

        // Check length constraints
        if let Some(min_len) = param.min_length {
            if value.len() < min_len as usize {
                return Err(anyhow::anyhow!(
                    "Value must be at least {} characters long",
                    min_len
                ));
            }
        }

        if let Some(max_len) = param.max_length {
            if value.len() > max_len as usize {
                return Err(anyhow::anyhow!(
                    "Value must be no more than {} characters long",
                    max_len
                ));
            }
        }

        // Check numeric constraints
        if param.parameter_type == "Number" {
            let number: f64 = value
                .parse()
                .with_context(|| format!("Value '{}' is not a valid number", value))?;

            if let Some(min_val) = param.min_value {
                if number < min_val {
                    return Err(anyhow::anyhow!(
                        "Value {} is less than minimum value {}",
                        number,
                        min_val
                    ));
                }
            }

            if let Some(max_val) = param.max_value {
                if number > max_val {
                    return Err(anyhow::anyhow!(
                        "Value {} is greater than maximum value {}",
                        number,
                        max_val
                    ));
                }
            }
        }

        Ok(())
    }

    /// Convert a serde_json::Value to a string representation
    fn value_to_string(&self, value: &Value) -> String {
        Self::value_to_string_static(value)
    }

    /// Static helper for converting Value to string
    fn value_to_string_static(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let strings: Vec<String> = arr.iter().map(Self::value_to_string_static).collect();
                strings.join(",")
            }
            Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
            Value::Null => String::new(),
        }
    }
}

impl Default for ParameterDiscovery {
    fn default() -> Self {
        Self::new()
    }
}
