//! CloudFormation template representation and manipulation.
//!
//! This module provides the core data structures and operations for working with
//! AWS CloudFormation templates. You can use this to parse, validate, and manipulate
//! CloudFormation templates in both JSON and YAML formats.
//!
//! # Core Components
//!
//! - [`CloudFormationTemplate`] - The main template structure containing all sections
//! - [`Resource`] - Individual AWS resources with properties and dependencies
//! - [`Parameter`] - Input parameters for template customization
//! - [`Output`] - Values returned after stack creation
//! - [`DependsOn`] - Resource dependency declarations
//!
//! # Key Features
//!
//! - **Multi-format support**: Parse and serialize JSON and YAML templates
//! - **Dependency validation**: Detect circular dependencies and invalid references
//! - **Template verification**: Compare templates for structural differences
//! - **Legacy compatibility**: Convert from older resource formats
//!
//! # Integration
//!
//! This module integrates with:
//! - [`crate::app::cfn_dag`] for dependency graph visualization
//! - [`crate::app::cfn_resources`] for resource type management
//! - [`crate::app::projects`] for project-level template organization
//!
//! # Examples
//!
//! Load and validate a CloudFormation template:
//! ```rust
//! use std::path::Path;
//! use aws_dash::app::cfn_template::CloudFormationTemplate;
//!
//! // Load template from file
//! let template = CloudFormationTemplate::from_file(Path::new("template.yaml"))?;
//!
//! // Validate dependencies
//! let errors = template.validate_dependencies();
//! if !errors.is_empty() {
//!     println!("Validation errors: {:?}", errors);
//! }
//!
//! // Check for circular dependencies
//! let cycles = template.detect_circular_dependencies();
//! if !cycles.is_empty() {
//!     println!("Circular dependencies detected: {:?}", cycles);
//! }
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::app::cfn_guard::{GuardValidator, GuardValidation};

/// Represents resource dependencies in CloudFormation templates.
///
/// CloudFormation allows dependencies to be specified as either a single resource name
/// or an array of resource names. This enum handles both formats seamlessly during
/// serialization and deserialization.
///
/// # Usage
///
/// You can create dependencies in several ways:
/// - Single dependency: `DependsOn::Single("MyResource".to_string())`
/// - Multiple dependencies: `DependsOn::Multiple(vec!["Resource1".to_string(), "Resource2".to_string()])`
/// - Convert to vector: `depends_on.to_vec()` for uniform processing
///
/// # CloudFormation Compatibility
///
/// This type automatically serializes to the correct CloudFormation format:
/// - Single dependency becomes a string in the template
/// - Multiple dependencies become an array in the template
#[derive(Debug, Clone, PartialEq)]
pub enum DependsOn {
    /// A single resource dependency
    Single(String),
    /// Multiple resource dependencies
    Multiple(Vec<String>),
}

impl Default for DependsOn {
    fn default() -> Self {
        DependsOn::Multiple(Vec::new())
    }
}

impl DependsOn {
    /// Convert dependencies to a vector for uniform processing.
    ///
    /// This method provides a consistent way to work with dependencies regardless
    /// of whether they were specified as a single string or array in the template.
    /// You can use this when iterating over all dependencies or when building
    /// dependency graphs.
    ///
    /// # Returns
    ///
    /// A vector containing all dependency resource names.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aws_dash::app::cfn_template::DependsOn;
    ///
    /// let single = DependsOn::Single("Database".to_string());
    /// assert_eq!(single.to_vec(), vec!["Database"]);
    ///
    /// let multiple = DependsOn::Multiple(vec!["Database".to_string(), "Cache".to_string()]);
    /// assert_eq!(multiple.to_vec(), vec!["Database", "Cache"]);
    /// ```
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            DependsOn::Single(s) => vec![s.clone()],
            DependsOn::Multiple(v) => v.clone(),
        }
    }
}

impl Serialize for DependsOn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DependsOn::Single(s) => s.serialize(serializer),
            DependsOn::Multiple(v) => v.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for DependsOn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) => Ok(DependsOn::Single(s)),
            Value::Array(arr) => {
                let strings: Result<Vec<String>, _> = arr
                    .into_iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .ok_or_else(|| serde::de::Error::custom("Expected string in array"))
                    })
                    .collect();
                Ok(DependsOn::Multiple(strings?))
            }
            _ => Err(serde::de::Error::custom("Expected string or array")),
        }
    }
}

/// A complete AWS CloudFormation template with all standard sections.
///
/// This is the main structure for representing CloudFormation templates in memory.
/// You can use this to load templates from files, modify them programmatically,
/// and save them back to disk. The structure supports all standard CloudFormation
/// sections and handles both JSON and YAML formats automatically.
///
/// # Template Sections
///
/// The template includes all standard CloudFormation sections:
/// - **AWSTemplateFormatVersion**: Template format version (usually "2010-09-09")
/// - **Description**: Human-readable template description
/// - **Transform**: SAM or other transforms to apply
/// - **Parameters**: Input parameters for customization
/// - **Mappings**: Static lookup tables
/// - **Conditions**: Conditional logic for resource creation
/// - **Resources**: AWS resources to create (the core of any template)
/// - **Outputs**: Values to return after stack creation
/// - **Metadata**: Additional template metadata
/// - **Rules**: Template validation rules
///
/// # Validation Features
///
/// The template provides several validation methods:
/// - [`validate_dependencies`] - Check for invalid resource references
/// - [`detect_circular_dependencies`] - Find circular dependency loops
/// - [`verify_against`] - Compare against another template for differences
///
/// # File Operations
///
/// Load and save templates in multiple formats:
/// - [`from_file`] - Load from JSON or YAML files
/// - [`to_file`] - Save to JSON or YAML based on file extension
///
/// [`validate_dependencies`]: Self::validate_dependencies
/// [`detect_circular_dependencies`]: Self::detect_circular_dependencies
/// [`verify_against`]: Self::verify_against
/// [`from_file`]: Self::from_file
/// [`to_file`]: Self::to_file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct CloudFormationTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_template_format_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, Parameter>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mappings: HashMap<String, Value>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub conditions: HashMap<String, Value>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub resources: HashMap<String, Resource>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub outputs: HashMap<String, Output>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub rules: HashMap<String, Rule>,
}

/// A CloudFormation template parameter for user input.
///
/// Parameters allow users to customize templates when creating stacks. Each parameter
/// has a type and optional constraints that CloudFormation validates during stack
/// operations. You can use parameters to make templates reusable across different
/// environments or configurations.
///
/// # Parameter Types
///
/// CloudFormation supports several parameter types:
/// - `String` - Text values
/// - `Number` - Numeric values
/// - `List<Number>` - Comma-separated numeric values
/// - `CommaDelimitedList` - Comma-separated string values
/// - AWS-specific types like `AWS::EC2::KeyPair::KeyName`
///
/// # Validation
///
/// Parameters can include various validation constraints:
/// - Value ranges with `min_value` and `max_value`
/// - String length limits with `min_length` and `max_length`
/// - Allowed values with `allowed_values`
/// - Pattern matching with `allowed_pattern`
///
/// # Security
///
/// Set `no_echo` to `true` for sensitive parameters like passwords to prevent
/// them from being displayed in the CloudFormation console.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Parameter {
    #[serde(rename = "Type")]
    pub parameter_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_pattern: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_echo: Option<bool>,
}

/// An AWS resource definition within a CloudFormation template.
///
/// Resources are the core components of CloudFormation templates, representing
/// the AWS services and infrastructure you want to create. Each resource has a
/// type (like `AWS::S3::Bucket`) and properties that configure the resource.
///
/// # Resource Attributes
///
/// Beyond the basic type and properties, resources support several optional attributes:
/// - **DependsOn**: Explicit dependencies on other resources
/// - **Condition**: Conditional creation based on template conditions
/// - **Metadata**: Additional information attached to the resource
/// - **DeletionPolicy**: What happens when the resource is deleted
/// - **UpdateReplacePolicy**: Behavior during stack updates
/// - **CreationPolicy**: Signals CloudFormation waits for during creation
/// - **UpdatePolicy**: How updates are handled for specific resource types
///
/// # Dependencies
///
/// Resources can depend on other resources in two ways:
/// 1. **Explicit dependencies**: Using the `depends_on` attribute
/// 2. **Implicit dependencies**: Through `Ref` or `Fn::GetAtt` functions in properties
///
/// The template validation methods can detect both types and identify circular dependencies.
///
/// # Properties
///
/// Resource properties are stored as a flexible `HashMap<String, Value>` to accommodate
/// the wide variety of AWS resource types and their different property schemas.
/// The actual properties depend on the specific resource type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Resource {
    #[serde(rename = "Type")]
    pub resource_type: String,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, Value>,

    // Universal resource attributes that can be added to any resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<DependsOn>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_policy: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_replace_policy: Option<String>,

    // Conditional resource attributes that can be added to specific resource types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_policy: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_policy: Option<Value>,
}

/// A CloudFormation template output that returns values after stack creation.
///
/// Outputs provide a way to return information from your CloudFormation stacks.
/// You can use outputs to return values like resource IDs, endpoints, or other
/// computed values that other stacks or applications might need.
///
/// # Export Feature
///
/// Outputs can be exported with a name, making them available to other stacks
/// in the same region through cross-stack references. Use the `export` field
/// to specify an export name.
///
/// # Conditional Outputs
///
/// Like resources, outputs can be conditional. Set the `condition` field to
/// reference a condition defined in the template's Conditions section.
///
/// # Common Use Cases
///
/// - Return resource ARNs or IDs for use in other stacks
/// - Export VPC or subnet IDs for network sharing
/// - Provide application endpoints or URLs
/// - Return generated values like passwords or keys
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Output {
    pub value: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<Export>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// Export configuration for a CloudFormation output.
///
/// When an output includes an export, it becomes available to other stacks
/// in the same AWS region through the `Fn::ImportValue` intrinsic function.
/// The export name must be unique within the region.
///
/// # Cross-Stack References
///
/// Exported outputs enable loose coupling between stacks by allowing one
/// stack to reference values from another without hard-coding resource names
/// or IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Export {
    pub name: Value,
}

/// A CloudFormation template rule for parameter validation.
///
/// Rules provide additional validation logic for template parameters beyond
/// the basic constraints. You can create rules that validate parameter
/// combinations or apply complex business logic during stack operations.
///
/// # Rule Structure
///
/// Each rule can include:
/// - An optional condition that determines when the rule applies
/// - One or more assertions that must be true for the rule to pass
///
/// Rules are evaluated during stack creation and updates, and CloudFormation
/// will reject the operation if any rule fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Rule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_condition: Option<Value>,

    pub assertions: Vec<Assertion>,
}

/// A validation assertion within a CloudFormation rule.
///
/// Each assertion contains a condition that must evaluate to true and a
/// description that explains what the assertion validates. If the assertion
/// fails, CloudFormation displays the description as part of the error message.
///
/// # Assertion Logic
///
/// The assertion field typically contains CloudFormation intrinsic functions
/// that evaluate parameter values and return true or false. Common patterns
/// include checking parameter combinations or validating against external constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Assertion {
    #[serde(rename = "Assert")]
    pub assertion: Value,

    pub assertion_description: String,
}

impl CloudFormationTemplate {
    /// Load a CloudFormation template from a JSON or YAML file.
    ///
    /// This method automatically detects the file format based on the file extension
    /// and content. It supports both `.json` and `.yaml`/`.yml` files. If the extension
    /// is ambiguous, it attempts to detect the format by examining the file content.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CloudFormation template file
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed template or an error if parsing fails.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The file cannot be read
    /// - The file content is not valid JSON or YAML
    /// - The file structure doesn't match the CloudFormation template schema
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    ///
    /// // Load a YAML template
    /// let template = CloudFormationTemplate::from_file(Path::new("infrastructure.yaml"))?;
    ///
    /// // Load a JSON template
    /// let template = CloudFormationTemplate::from_file(Path::new("stack.json"))?;
    /// ```
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;

        // Determine the file type and parse accordingly
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("json") => serde_json::from_str::<CloudFormationTemplate>(&content)
                .map_err(|e| anyhow!("Failed to parse JSON: {}", e)),
            Some("yaml") | Some("yml") => serde_yaml::from_str::<CloudFormationTemplate>(&content)
                .map_err(|e| anyhow!("Failed to parse YAML: {}", e)),
            _ => {
                // Try to detect format from content
                if content.trim_start().starts_with("{") {
                    // Likely JSON
                    serde_json::from_str::<CloudFormationTemplate>(&content)
                        .map_err(|e| anyhow!("Failed to parse as JSON: {}", e))
                } else {
                    // Try YAML
                    serde_yaml::from_str::<CloudFormationTemplate>(&content)
                        .map_err(|e| anyhow!("Failed to parse as YAML: {}", e))
                }
            }
        }
    }

    /// Parse a CloudFormation template from a JSON string.
    ///
    /// This method parses a JSON string representation of a CloudFormation template
    /// and returns a structured CloudFormationTemplate object.
    ///
    /// # Arguments
    ///
    /// * `json_content` - JSON string containing the CloudFormation template
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed template or an error if parsing fails.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The JSON string is malformed
    /// - The JSON structure doesn't match the CloudFormation template schema
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    ///
    /// let json_str = r#"{
    ///     "AWSTemplateFormatVersion": "2010-09-09",
    ///     "Resources": {
    ///         "MyBucket": {
    ///             "Type": "AWS::S3::Bucket"
    ///         }
    ///     }
    /// }"#;
    ///
    /// let template = CloudFormationTemplate::from_json(json_str)?;
    /// ```
    pub fn from_json(json_content: &str) -> Result<Self> {
        serde_json::from_str::<CloudFormationTemplate>(json_content)
            .map_err(|e| anyhow!("Failed to parse JSON: {}", e))
    }

    /// Save the template to a JSON or YAML file.
    ///
    /// The output format is determined by the file extension. YAML format is used
    /// for `.yaml` and `.yml` extensions, while JSON format is used for all other
    /// extensions (including `.json`).
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the template should be saved
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The file cannot be written
    /// - Serialization to the target format fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    ///
    /// let template = CloudFormationTemplate::default();
    ///
    /// // Save as YAML
    /// template.to_file(Path::new("output.yaml"))?;
    ///
    /// // Save as JSON
    /// template.to_file(Path::new("output.json"))?;
    /// ```
    pub fn to_file(&self, path: &Path) -> Result<()> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        let content = match extension.as_deref() {
            Some("yaml") | Some("yml") => serde_yaml::to_string(self)?,
            _ => {
                // Default to JSON
                serde_json::to_string_pretty(self)?
            }
        };

        fs::write(path, content)?;
        Ok(())
    }

    /// Convert legacy project resources to a standard CloudFormation template.
    ///
    /// This method provides compatibility with older resource formats used in the
    /// application's project system. It creates a new template with the standard
    /// CloudFormation format version and converts each legacy resource to the
    /// standard resource format.
    ///
    /// # Arguments
    ///
    /// * `resources` - Vector of legacy resources from the project system
    ///
    /// # Returns
    ///
    /// A new CloudFormation template containing the converted resources.
    ///
    /// # Template Structure
    ///
    /// The generated template includes:
    /// - AWSTemplateFormatVersion set to "2010-09-09"
    /// - Resources section with all converted resources
    /// - Empty sections for other template components
    ///
    /// # Migration Support
    ///
    /// This method supports migrating from the application's internal resource
    /// format to standard CloudFormation templates, enabling better integration
    /// with AWS tooling and broader CloudFormation ecosystem.
    pub fn from_legacy_resources(
        resources: Vec<crate::app::projects::CloudFormationResource>,
    ) -> Self {
        let mut template = Self {
            aws_template_format_version: Some("2010-09-09".to_string()),
            ..Default::default()
        };

        for resource in resources {
            let cfn_resource = Resource {
                resource_type: resource.resource_type.clone(),
                properties: resource.properties,
                depends_on: None,
                condition: None,
                metadata: None,
                deletion_policy: None,
                update_replace_policy: None,
                creation_policy: None,
                update_policy: None,
            };

            template
                .resources
                .insert(resource.resource_id, cfn_resource);
        }

        template
    }

    /// Compare this template against a source template to find structural differences.
    ///
    /// This method performs a comprehensive comparison between templates, checking
    /// all sections and reporting any discrepancies. You can use this to verify
    /// that template transformations or imports preserved all necessary content.
    ///
    /// # Arguments
    ///
    /// * `source` - The reference template to compare against
    ///
    /// # Returns
    ///
    /// A vector of strings describing each discrepancy found. An empty vector
    /// indicates the templates are structurally equivalent.
    ///
    /// # Comparison Scope
    ///
    /// The method compares:
    /// - Template metadata (version, description, transforms)
    /// - All template sections (parameters, mappings, conditions, etc.)
    /// - Resource types and properties
    /// - Resource attributes (dependencies, policies, etc.)
    /// - Output configurations and exports
    ///
    /// # Use Cases
    ///
    /// - Validate template import/export operations
    /// - Verify template transformations preserve content
    /// - Check template compatibility between versions
    /// - Audit template changes during development
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    ///
    /// let original = CloudFormationTemplate::from_file(Path::new("original.yaml"))?;
    /// let converted = CloudFormationTemplate::from_file(Path::new("converted.json"))?;
    ///
    /// let differences = converted.verify_against(&original);
    /// if !differences.is_empty() {
    ///     println!("Found {} differences:", differences.len());
    ///     for diff in differences {
    ///         println!("  - {}", diff);
    ///     }
    /// }
    /// ```
    pub fn verify_against(&self, source: &CloudFormationTemplate) -> Vec<String> {
        let mut discrepancies = Vec::new();

        // Check AWSTemplateFormatVersion
        if source.aws_template_format_version.is_some()
            && self.aws_template_format_version != source.aws_template_format_version
        {
            discrepancies.push(format!(
                "AWSTemplateFormatVersion mismatch: expected {:?}, found {:?}",
                source.aws_template_format_version, self.aws_template_format_version
            ));
        }

        // Check Description
        if source.description.is_some() && self.description != source.description {
            discrepancies.push(format!(
                "Description mismatch: expected {:?}, found {:?}",
                source.description, self.description
            ));
        }

        // Check Transform
        if source.transform.is_some() && self.transform != source.transform {
            discrepancies.push(format!(
                "Transform mismatch: expected {:?}, found {:?}",
                source.transform, self.transform
            ));
        }

        // Check Parameters
        for (key, param) in &source.parameters {
            if !self.parameters.contains_key(key) {
                discrepancies.push(format!("Missing parameter: {}", key));
            } else {
                let self_param_json = serde_json::to_value(&self.parameters[key]).ok();
                let source_param_json = serde_json::to_value(param).ok();
                if self_param_json != source_param_json {
                    discrepancies.push(format!("Parameter '{}' content mismatch", key));
                }
            }
        }

        // Check Mappings
        for (key, mapping) in &source.mappings {
            if !self.mappings.contains_key(key) {
                discrepancies.push(format!("Missing mapping: {}", key));
            } else if &self.mappings[key] != mapping {
                discrepancies.push(format!("Mapping '{}' content mismatch", key));
            }
        }

        // Check Conditions
        for (key, condition) in &source.conditions {
            if !self.conditions.contains_key(key) {
                discrepancies.push(format!("Missing condition: {}", key));
            } else if &self.conditions[key] != condition {
                discrepancies.push(format!("Condition '{}' content mismatch", key));
            }
        }

        // Check Resources
        for (key, resource) in &source.resources {
            if !self.resources.contains_key(key) {
                discrepancies.push(format!("Missing resource: {}", key));
            } else {
                let self_res = &self.resources[key];
                if self_res.resource_type != resource.resource_type {
                    discrepancies.push(format!(
                        "Resource '{}' type mismatch: expected {}, found {}",
                        key, resource.resource_type, self_res.resource_type
                    ));
                }

                // Compare properties
                for (prop_key, prop_value) in &resource.properties {
                    if !self_res.properties.contains_key(prop_key) {
                        discrepancies
                            .push(format!("Resource '{}' missing property: {}", key, prop_key));
                    } else if &self_res.properties[prop_key] != prop_value {
                        discrepancies.push(format!(
                            "Resource '{}' property '{}' mismatch",
                            key, prop_key
                        ));
                    }
                }

                // Check other resource attributes
                if self_res.depends_on != resource.depends_on {
                    discrepancies.push(format!("Resource '{}' DependsOn mismatch", key));
                }
                if self_res.condition != resource.condition {
                    discrepancies.push(format!("Resource '{}' Condition mismatch", key));
                }
                if self_res.metadata != resource.metadata {
                    discrepancies.push(format!("Resource '{}' Metadata mismatch", key));
                }
                if self_res.deletion_policy != resource.deletion_policy {
                    discrepancies.push(format!("Resource '{}' DeletionPolicy mismatch", key));
                }
                if self_res.update_replace_policy != resource.update_replace_policy {
                    discrepancies.push(format!("Resource '{}' UpdateReplacePolicy mismatch", key));
                }
                if self_res.creation_policy != resource.creation_policy {
                    discrepancies.push(format!("Resource '{}' CreationPolicy mismatch", key));
                }
                if self_res.update_policy != resource.update_policy {
                    discrepancies.push(format!("Resource '{}' UpdatePolicy mismatch", key));
                }
            }
        }

        // Check Outputs
        for (key, output) in &source.outputs {
            if !self.outputs.contains_key(key) {
                discrepancies.push(format!("Missing output: {}", key));
            } else {
                let self_output_json = serde_json::to_value(&self.outputs[key]).ok();
                let source_output_json = serde_json::to_value(output).ok();
                if self_output_json != source_output_json {
                    discrepancies.push(format!("Output '{}' content mismatch", key));
                }
            }
        }

        // Check Metadata
        for (key, metadata) in &source.metadata {
            if !self.metadata.contains_key(key) {
                discrepancies.push(format!("Missing metadata: {}", key));
            } else if &self.metadata[key] != metadata {
                discrepancies.push(format!("Metadata '{}' content mismatch", key));
            }
        }

        // Check Rules
        for (key, rule) in &source.rules {
            if !self.rules.contains_key(key) {
                discrepancies.push(format!("Missing rule: {}", key));
            } else {
                let self_rule_json = serde_json::to_value(&self.rules[key]).ok();
                let source_rule_json = serde_json::to_value(rule).ok();
                if self_rule_json != source_rule_json {
                    discrepancies.push(format!("Rule '{}' content mismatch", key));
                }
            }
        }

        discrepancies
    }

    /// Validate all resource dependencies and references in the template.
    ///
    /// This method performs comprehensive dependency validation, checking both
    /// explicit dependencies (DependsOn) and implicit dependencies (Ref, Fn::GetAtt).
    /// It ensures all referenced resources exist and validates condition references.
    ///
    /// # Returns
    ///
    /// A vector of validation error messages. An empty vector indicates all
    /// dependencies are valid.
    ///
    /// # Validation Checks
    ///
    /// The method validates:
    /// - **DependsOn references**: All referenced resources exist
    /// - **Self-dependencies**: Resources don't depend on themselves
    /// - **Condition references**: All referenced conditions are defined
    /// - **Intrinsic functions**: Ref and Fn::GetAtt point to valid resources
    /// - **Parameter references**: Ref functions reference existing parameters
    /// - **AWS pseudo parameters**: Built-in AWS references are recognized
    ///
    /// # Supported Intrinsic Functions
    ///
    /// - `Ref` - References to parameters, resources, or AWS pseudo parameters
    /// - `Fn::GetAtt` - Attribute references to resources
    ///
    /// # AWS Pseudo Parameters
    ///
    /// Recognizes standard AWS pseudo parameters like:
    /// - `AWS::AccountId`, `AWS::Region`, `AWS::StackName`
    /// - `AWS::Partition`, `AWS::URLSuffix`, `AWS::NoValue`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    ///
    /// let template = CloudFormationTemplate::from_file(Path::new("template.yaml"))?;
    /// let errors = template.validate_dependencies();
    ///
    /// if errors.is_empty() {
    ///     println!("All dependencies are valid!");
    /// } else {
    ///     println!("Found {} dependency errors:", errors.len());
    ///     for error in errors {
    ///         println!("  - {}", error);
    ///     }
    /// }
    /// ```
    pub fn validate_dependencies(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Collect all resource names for reference validation
        let resource_names: std::collections::HashSet<String> =
            self.resources.keys().cloned().collect();

        for (resource_name, resource) in &self.resources {
            // Validate DependsOn references
            if let Some(depends_on) = &resource.depends_on {
                for dependency in depends_on.to_vec() {
                    if !resource_names.contains(&dependency) {
                        errors.push(format!(
                            "Resource '{}' has DependsOn reference to non-existent resource '{}'",
                            resource_name, dependency
                        ));
                    }
                    if dependency == *resource_name {
                        errors.push(format!(
                            "Resource '{}' cannot depend on itself",
                            resource_name
                        ));
                    }
                }
            }

            // Validate condition references
            if let Some(condition_name) = &resource.condition {
                if !self.conditions.contains_key(condition_name) {
                    errors.push(format!(
                        "Resource '{}' references non-existent condition '{}'",
                        resource_name, condition_name
                    ));
                }
            }

            // Validate intrinsic function references in properties
            errors.extend(self.validate_property_references(
                resource_name,
                &resource.properties,
                &resource_names,
            ));
        }

        // Validate output references
        for (output_name, output) in &self.outputs {
            errors.extend(self.validate_value_references(
                &format!("Output '{}'", output_name),
                &output.value,
                &resource_names,
            ));

            if let Some(condition_name) = &output.condition {
                if !self.conditions.contains_key(condition_name) {
                    errors.push(format!(
                        "Output '{}' references non-existent condition '{}'",
                        output_name, condition_name
                    ));
                }
            }
        }

        errors
    }

    /// Validate references within a resource's properties.
    ///
    /// This helper method recursively examines all property values to find and
    /// validate intrinsic function references. It's used internally by the main
    /// dependency validation logic.
    ///
    /// # Arguments
    ///
    /// * `resource_name` - Name of the resource being validated (for error context)
    /// * `properties` - The resource's properties map
    /// * `resource_names` - Set of all valid resource names in the template
    ///
    /// # Returns
    ///
    /// A vector of validation error messages for invalid references found.
    fn validate_property_references(
        &self,
        resource_name: &str,
        properties: &HashMap<String, Value>,
        resource_names: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        for (prop_name, prop_value) in properties {
            let context = format!("Resource '{}' property '{}'", resource_name, prop_name);
            errors.extend(self.validate_value_references(&context, prop_value, resource_names));
        }

        errors
    }

    /// Recursively validate references within a JSON value structure.
    ///
    /// This method traverses JSON objects and arrays to find intrinsic functions
    /// like Ref and Fn::GetAtt, validating that they reference existing resources,
    /// parameters, or AWS pseudo parameters.
    ///
    /// # Arguments
    ///
    /// * `context` - Description of where this value appears (for error messages)
    /// * `value` - The JSON value to validate
    /// * `resource_names` - Set of all valid resource names in the template
    ///
    /// # Returns
    ///
    /// A vector of validation error messages for any invalid references found.
    ///
    /// # Intrinsic Function Support
    ///
    /// Validates these CloudFormation intrinsic functions:
    /// - `{"Ref": "ResourceName"}` - References to resources, parameters, or pseudo parameters
    /// - `{"Fn::GetAtt": ["ResourceName", "AttributeName"]}` - Resource attribute references
    fn validate_value_references(
        &self,
        context: &str,
        value: &Value,
        resource_names: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        match value {
            Value::Object(obj) => {
                // Check for Ref function
                if let Some(ref_value) = obj.get("Ref") {
                    if let Some(ref_str) = ref_value.as_str() {
                        // Check if it's a resource reference (not a parameter)
                        if !self.parameters.contains_key(ref_str)
                            && !resource_names.contains(ref_str)
                        {
                            // Check for AWS pseudo parameters
                            if !self.is_aws_pseudo_parameter(ref_str) {
                                errors.push(format!(
                                    "{} contains Ref to non-existent resource or parameter '{}'",
                                    context, ref_str
                                ));
                            }
                        }
                    }
                }

                // Check for Fn::GetAtt function
                if let Some(getatt_value) = obj.get("Fn::GetAtt") {
                    if let Some(getatt_array) = getatt_value.as_array() {
                        if !getatt_array.is_empty() {
                            if let Some(resource_ref) = getatt_array[0].as_str() {
                                if !resource_names.contains(resource_ref) {
                                    errors.push(format!(
                                        "{} contains Fn::GetAtt reference to non-existent resource '{}'",
                                        context, resource_ref
                                    ));
                                }
                            }
                        }
                    }
                }

                // Recursively check other intrinsic functions and nested objects
                for (key, nested_value) in obj {
                    let nested_context = format!("{}.{}", context, key);
                    errors.extend(self.validate_value_references(
                        &nested_context,
                        nested_value,
                        resource_names,
                    ));
                }
            }
            Value::Array(arr) => {
                // Recursively check array elements
                for (index, element) in arr.iter().enumerate() {
                    let array_context = format!("{}[{}]", context, index);
                    errors.extend(self.validate_value_references(
                        &array_context,
                        element,
                        resource_names,
                    ));
                }
            }
            _ => {
                // No references to validate in primitive values
            }
        }

        errors
    }

    /// Check if a reference string is a valid AWS pseudo parameter.
    ///
    /// AWS pseudo parameters are built-in references that CloudFormation provides
    /// automatically, such as the current AWS account ID or region. These don't
    /// need to be defined in the template and are always available.
    ///
    /// # Arguments
    ///
    /// * `reference` - The reference string to check
    ///
    /// # Returns
    ///
    /// `true` if the reference is a recognized AWS pseudo parameter, `false` otherwise.
    ///
    /// # Supported Pseudo Parameters
    ///
    /// - `AWS::AccountId` - The AWS account ID
    /// - `AWS::Region` - The AWS region
    /// - `AWS::StackName` - The name of the CloudFormation stack
    /// - `AWS::StackId` - The unique ID of the CloudFormation stack
    /// - `AWS::Partition` - The AWS partition (aws, aws-cn, aws-us-gov)
    /// - `AWS::URLSuffix` - The domain suffix for AWS URLs
    /// - `AWS::NotificationARNs` - Notification ARNs for the stack
    /// - `AWS::NoValue` - Represents a null value
    fn is_aws_pseudo_parameter(&self, reference: &str) -> bool {
        matches!(
            reference,
            "AWS::AccountId"
                | "AWS::NotificationARNs"
                | "AWS::NoValue"
                | "AWS::Partition"
                | "AWS::Region"
                | "AWS::StackId"
                | "AWS::StackName"
                | "AWS::URLSuffix"
        )
    }

    /// Detect circular dependencies between resources in the template.
    ///
    /// Circular dependencies occur when resources depend on each other in a loop,
    /// either directly or indirectly. CloudFormation cannot resolve such dependencies
    /// and will fail during stack operations. This method uses depth-first search
    /// to identify all circular dependency paths.
    ///
    /// # Returns
    ///
    /// A vector of strings describing each circular dependency found. Each string
    /// shows the complete dependency cycle. An empty vector indicates no circular
    /// dependencies exist.
    ///
    /// # Dependency Detection
    ///
    /// The method detects both types of dependencies:
    /// - **Explicit dependencies**: Specified in DependsOn attributes
    /// - **Implicit dependencies**: Created by Ref and Fn::GetAtt functions
    ///
    /// # Algorithm
    ///
    /// Uses a depth-first search with recursion stack tracking to detect cycles.
    /// When a cycle is found, it returns the complete path showing how the
    /// dependency loop occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    ///
    /// let template = CloudFormationTemplate::from_file(Path::new("template.yaml"))?;
    /// let cycles = template.detect_circular_dependencies();
    ///
    /// if cycles.is_empty() {
    ///     println!("No circular dependencies found!");
    /// } else {
    ///     println!("Found {} circular dependencies:", cycles.len());
    ///     for cycle in cycles {
    ///         println!("  - {}", cycle);
    ///     }
    /// }
    /// ```
    ///
    /// # Error Format
    ///
    /// Circular dependency errors show the complete cycle:
    /// ```text
    /// Circular dependency detected: ResourceA -> ResourceB -> ResourceC -> ResourceA
    /// ```
    pub fn detect_circular_dependencies(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for resource_name in self.resources.keys() {
            if !visited.contains(resource_name) {
                if let Some(cycle) = self.detect_cycle_util(
                    resource_name,
                    &mut visited,
                    &mut rec_stack,
                    &mut Vec::new(),
                ) {
                    errors.push(format!(
                        "Circular dependency detected: {}",
                        cycle.join(" -> ")
                    ));
                }
            }
        }

        errors
    }

    /// Depth-first search utility for detecting dependency cycles.
    ///
    /// This is the core algorithm for circular dependency detection. It uses
    /// DFS with a recursion stack to detect back edges that indicate cycles.
    ///
    /// # Arguments
    ///
    /// * `resource_name` - Current resource being examined
    /// * `visited` - Set of resources already processed
    /// * `rec_stack` - Set of resources in current recursion path
    /// * `path` - Current dependency path for cycle reconstruction
    ///
    /// # Returns
    ///
    /// `Some(cycle_path)` if a cycle is detected, `None` otherwise.
    ///
    /// # Algorithm Details
    ///
    /// 1. Mark current resource as visited and add to recursion stack
    /// 2. Examine all dependencies (explicit and implicit)
    /// 3. If a dependency is in the recursion stack, a cycle exists
    /// 4. Recursively check unvisited dependencies
    /// 5. Remove resource from recursion stack when done
    fn detect_cycle_util(
        &self,
        resource_name: &str,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(resource_name.to_string());
        rec_stack.insert(resource_name.to_string());
        path.push(resource_name.to_string());

        if let Some(resource) = self.resources.get(resource_name) {
            // Check explicit DependsOn dependencies
            if let Some(depends_on) = &resource.depends_on {
                for dependency in depends_on.to_vec() {
                    if rec_stack.contains(&dependency) {
                        // Found a cycle - return the cycle path
                        let cycle_start = path.iter().position(|x| x == &dependency).unwrap();
                        let mut cycle = path[cycle_start..].to_vec();
                        cycle.push(dependency);
                        return Some(cycle);
                    }

                    if !visited.contains(&dependency) {
                        if let Some(cycle) =
                            self.detect_cycle_util(&dependency, visited, rec_stack, path)
                        {
                            return Some(cycle);
                        }
                    }
                }
            }

            // Check implicit dependencies from Ref and GetAtt
            let implicit_deps = self.extract_implicit_dependencies(resource);
            for dependency in implicit_deps {
                if rec_stack.contains(&dependency) {
                    // Found a cycle - return the cycle path
                    let cycle_start = path.iter().position(|x| x == &dependency).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dependency);
                    return Some(cycle);
                }

                if !visited.contains(&dependency) {
                    if let Some(cycle) =
                        self.detect_cycle_util(&dependency, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                }
            }
        }

        rec_stack.remove(resource_name);
        path.pop();
        None
    }

    /// Extract implicit dependencies from a resource's properties.
    ///
    /// Implicit dependencies are created when a resource's properties reference
    /// other resources through intrinsic functions like Ref or Fn::GetAtt.
    /// CloudFormation automatically creates these dependencies, but they need
    /// to be considered when detecting circular dependencies.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to analyze for implicit dependencies
    ///
    /// # Returns
    ///
    /// A vector of resource names that this resource implicitly depends on.
    ///
    /// # Detection Logic
    ///
    /// Recursively examines all property values to find:
    /// - `{"Ref": "ResourceName"}` - Direct resource references
    /// - `{"Fn::GetAtt": ["ResourceName", "Attribute"]}` - Attribute references
    ///
    /// Only returns dependencies on actual resources (not parameters or pseudo parameters).
    ///
    /// # Use Cases
    ///
    /// - Circular dependency detection
    /// - Dependency graph visualization
    /// - Template analysis and optimization
    /// - Build order determination
    pub fn extract_implicit_dependencies(&self, resource: &Resource) -> Vec<String> {
        let mut dependencies = Vec::new();
        self.extract_dependencies_from_value(&resource.properties, &mut dependencies);
        dependencies
    }

    /// Extract dependencies from a properties map.
    ///
    /// This helper method examines all values in a resource's properties map
    /// to find implicit dependencies created by intrinsic functions.
    ///
    /// # Arguments
    ///
    /// * `value` - The properties map to examine
    /// * `dependencies` - Mutable vector to collect found dependencies
    fn extract_dependencies_from_value(
        &self,
        value: &HashMap<String, Value>,
        dependencies: &mut Vec<String>,
    ) {
        for prop_value in value.values() {
            self.extract_dependencies_from_json_value(prop_value, dependencies);
        }
    }

    /// Extract dependencies from a single JSON value, handling nested structures.
    ///
    /// This method recursively traverses JSON values to find intrinsic functions
    /// that create implicit dependencies. It handles objects, arrays, and primitive
    /// values appropriately.
    ///
    /// # Arguments
    ///
    /// * `value` - The JSON value to examine
    /// * `dependencies` - Mutable vector to collect found dependencies
    ///
    /// # Intrinsic Functions
    ///
    /// Recognizes these dependency-creating functions:
    /// - `Ref` - References to other resources
    /// - `Fn::GetAtt` - Attribute access on other resources
    ///
    /// Ignores references to parameters and AWS pseudo parameters since these
    /// don't create resource dependencies.
    fn extract_dependencies_from_json_value(&self, value: &Value, dependencies: &mut Vec<String>) {
        match value {
            Value::Object(obj) => {
                // Check for Ref function
                if let Some(ref_value) = obj.get("Ref") {
                    if let Some(ref_str) = ref_value.as_str() {
                        // Only add if it's a resource reference (not parameter or pseudo parameter)
                        if self.resources.contains_key(ref_str) {
                            dependencies.push(ref_str.to_string());
                        }
                    }
                }

                // Check for Fn::GetAtt function
                if let Some(getatt_value) = obj.get("Fn::GetAtt") {
                    if let Some(getatt_array) = getatt_value.as_array() {
                        if !getatt_array.is_empty() {
                            if let Some(resource_ref) = getatt_array[0].as_str() {
                                if self.resources.contains_key(resource_ref) {
                                    dependencies.push(resource_ref.to_string());
                                }
                            }
                        }
                    }
                }

                // Recursively check nested objects
                for nested_value in obj.values() {
                    self.extract_dependencies_from_json_value(nested_value, dependencies);
                }
            }
            Value::Array(arr) => {
                // Recursively check array elements
                for element in arr {
                    self.extract_dependencies_from_json_value(element, dependencies);
                }
            }
            _ => {
                // No dependencies in primitive values
            }
        }
    }

    /// Validate the template against CloudFormation Guard rules.
    ///
    /// This method performs policy-as-code validation using CloudFormation Guard
    /// rules for the specified compliance programs. It checks for security
    /// misconfigurations, compliance violations, and best practice deviations.
    ///
    /// # Arguments
    ///
    /// * `validator` - The Guard validator containing loaded compliance rules
    ///
    /// # Returns
    ///
    /// A `Result` containing the Guard validation results with any violations found.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The template cannot be serialized to JSON for Guard validation
    /// - The Guard validation process fails
    /// - There are issues with the loaded rules
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aws_dash::app::cfn_template::CloudFormationTemplate;
    /// use aws_dash::app::cfn_guard::{GuardValidator, ComplianceProgram};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let template = CloudFormationTemplate::from_file(Path::new("template.yaml"))?;
    /// let validator = GuardValidator::new(vec![ComplianceProgram::NIST80053R5]).await?;
    /// 
    /// let validation = template.validate_with_guard(&validator).await?;
    /// if !validation.compliant {
    ///     println!("Found {} violations", validation.violations.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_with_guard(&self, validator: &GuardValidator) -> Result<GuardValidation> {
        validator.validate_template(self).await
    }
}
