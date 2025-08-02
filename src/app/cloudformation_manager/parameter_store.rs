use crate::app::projects::Project;
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_ssm as ssm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterStoreEntry {
    pub name: String,
    pub value: String,
    pub parameter_type: String,
    pub description: Option<String>,
    pub secure_string: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterStoreResult {
    pub success: bool,
    pub parameter_name: String,
    pub transformed_template: Option<String>,
    pub error_message: Option<String>,
}

/// Parameter Store Manager for CloudFormation template parameter integration
pub struct ParameterStoreManager {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ParameterStoreManager {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// Generate parameter name using convention: /{app-name}/{environment}/{param-name}
    pub fn generate_parameter_name(
        project: &Project,
        environment: &str,
        parameter_name: &str,
    ) -> String {
        format!("/{}/{}/{}", project.short_name, environment, parameter_name)
    }

    /// Store a parameter value in AWS Parameter Store
    #[allow(clippy::too_many_arguments)]
    pub async fn store_parameter(
        &self,
        project: &Project,
        environment: &str,
        parameter_name: &str,
        parameter_value: &str,
        parameter_type: &str,
        description: Option<&str>,
        account_id: &str,
        region: &str,
        secure_string: bool,
    ) -> Result<ParameterStoreResult> {
        let param_store_name = Self::generate_parameter_name(project, environment, parameter_name);

        debug!(
            "Storing parameter {} in Parameter Store for account {} in region {}",
            param_store_name, account_id, region
        );

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = ssm::Client::new(&aws_config);

        let ssm_type = if secure_string {
            ssm::types::ParameterType::SecureString
        } else {
            match parameter_type {
                "String" => ssm::types::ParameterType::String,
                "StringList" => ssm::types::ParameterType::StringList,
                _ => ssm::types::ParameterType::String, // Default to String
            }
        };

        let mut request = client
            .put_parameter()
            .name(&param_store_name)
            .value(parameter_value)
            .r#type(ssm_type)
            .overwrite(true); // Allow updating existing parameters

        if let Some(desc) = description {
            request = request.description(desc);
        }

        match request.send().await {
            Ok(_) => {
                info!(
                    "Successfully stored parameter {} in Parameter Store",
                    param_store_name
                );
                Ok(ParameterStoreResult {
                    success: true,
                    parameter_name: param_store_name,
                    transformed_template: None,
                    error_message: None,
                })
            }
            Err(sdk_error) => {
                let error_msg = format!(
                    "Failed to store parameter {}: {}",
                    param_store_name, sdk_error
                );
                warn!("{}", error_msg);
                Ok(ParameterStoreResult {
                    success: false,
                    parameter_name: param_store_name,
                    transformed_template: None,
                    error_message: Some(error_msg),
                })
            }
        }
    }

    /// Retrieve parameters from Parameter Store for a given project and environment
    pub async fn get_parameters_by_path(
        &self,
        project: &Project,
        environment: &str,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<ParameterStoreEntry>> {
        let path_prefix = format!("/{}/{}", project.short_name, environment);

        debug!(
            "Retrieving parameters by path {} for account {} in region {}",
            path_prefix, account_id, region
        );

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = ssm::Client::new(&aws_config);

        let mut parameters = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client
                .get_parameters_by_path()
                .path(&path_prefix)
                .recursive(true)
                .with_decryption(true); // Decrypt SecureString parameters

            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(response) => {
                    for param in response.parameters() {
                        if let (Some(name), Some(value), Some(param_type)) =
                            (param.name(), param.value(), param.r#type())
                        {
                            parameters.push(ParameterStoreEntry {
                                name: name.to_string(),
                                value: value.to_string(),
                                parameter_type: param_type.as_str().to_string(),
                                description: None, // Parameter description not available in GetParametersByPath response
                                secure_string: matches!(
                                    param.r#type(),
                                    Some(ssm::types::ParameterType::SecureString)
                                ),
                            });
                        }
                    }

                    next_token = response.next_token().map(|s| s.to_string());
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(sdk_error) => {
                    warn!(
                        "Failed to retrieve parameters by path {}: {}",
                        path_prefix, sdk_error
                    );
                    return Err(sdk_error.into());
                }
            }
        }

        info!(
            "Retrieved {} parameters from path {}",
            parameters.len(),
            path_prefix
        );
        Ok(parameters)
    }

    /// Transform CloudFormation template to use Parameter Store references
    pub fn transform_template_for_parameter_store(
        &self,
        template: &str,
        project: &Project,
        environment: &str,
        parameters_to_store: &HashMap<String, String>,
    ) -> Result<String> {
        debug!(
            "Transforming CloudFormation template to use Parameter Store for {} parameters",
            parameters_to_store.len()
        );

        let mut cfn_template: serde_json::Value = serde_json::from_str(template)
            .with_context(|| "Failed to parse CloudFormation template as JSON")?;

        // Get the Parameters section
        if let Some(parameters_section) = cfn_template.get_mut("Parameters") {
            if let Some(params_obj) = parameters_section.as_object_mut() {
                for param_name in parameters_to_store.keys() {
                    if let Some(param_def) = params_obj.get_mut(param_name) {
                        if let Some(param_obj) = param_def.as_object_mut() {
                            // Get the original type
                            let original_type = param_obj
                                .get("Type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("String");

                            // Transform the parameter type to use Parameter Store
                            let new_type = format!("AWS::SSM::Parameter::Value<{}>", original_type);
                            param_obj
                                .insert("Type".to_string(), serde_json::Value::String(new_type));

                            // Set the default value to the Parameter Store path
                            let param_store_path =
                                Self::generate_parameter_name(project, environment, param_name);
                            param_obj.insert(
                                "Default".to_string(),
                                serde_json::Value::String(param_store_path.clone()),
                            );

                            // Add description if not present
                            if !param_obj.contains_key("Description") {
                                let description = format!(
                                    "Parameter stored in AWS Parameter Store at {}",
                                    param_store_path
                                );
                                param_obj.insert(
                                    "Description".to_string(),
                                    serde_json::Value::String(description),
                                );
                            }

                            info!(
                                "Transformed parameter {} to use Parameter Store",
                                param_name
                            );
                        }
                    }
                }
            }
        }

        let transformed_template = serde_json::to_string_pretty(&cfn_template)
            .with_context(|| "Failed to serialize transformed CloudFormation template")?;

        debug!("Template transformation completed successfully");
        Ok(transformed_template)
    }

    /// Get available parameter types for CloudFormation
    pub fn get_supported_parameter_types() -> Vec<&'static str> {
        vec![
            "String",
            "Number",
            "List<Number>",
            "CommaDelimitedList",
            "AWS::EC2::AvailabilityZone::Name",
            "AWS::EC2::Image::Id",
            "AWS::EC2::Instance::Id",
            "AWS::EC2::KeyPair::KeyName",
            "AWS::EC2::SecurityGroup::GroupName",
            "AWS::EC2::SecurityGroup::Id",
            "AWS::EC2::Subnet::Id",
            "AWS::EC2::Volume::Id",
            "AWS::EC2::VPC::Id",
            "AWS::Route53::HostedZone::Id",
            "List<AWS::EC2::AvailabilityZone::Name>",
            "List<AWS::EC2::Image::Id>",
            "List<AWS::EC2::Instance::Id>",
            "List<AWS::EC2::SecurityGroup::GroupName>",
            "List<AWS::EC2::SecurityGroup::Id>",
            "List<AWS::EC2::Subnet::Id>",
            "List<AWS::EC2::Volume::Id>",
            "List<AWS::EC2::VPC::Id>",
            "List<AWS::Route53::HostedZone::Id>",
        ]
    }

    /// Check if a parameter type should be stored as SecureString
    pub fn should_use_secure_string(parameter_name: &str, no_echo: bool) -> bool {
        if no_echo {
            return true;
        }

        let lowercase_name = parameter_name.to_lowercase();
        lowercase_name.contains("password")
            || lowercase_name.contains("secret")
            || lowercase_name.contains("key")
            || lowercase_name.contains("token")
            || lowercase_name.contains("credential")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_project() -> Project {
        Project {
            name: "Test Project".to_string(),
            description: "Test project for parameter store".to_string(),
            short_name: "testapp".to_string(),
            created: Utc::now(),
            updated: Utc::now(),
            local_folder: None,
            git_url: None,
            environments: vec![],
            default_region: Some("us-east-1".to_string()),
            cfn_template: None,
        }
    }

    #[test]
    fn test_generate_parameter_name() {
        let project = create_test_project();
        let param_name =
            ParameterStoreManager::generate_parameter_name(&project, "dev", "database-url");
        assert_eq!(param_name, "/testapp/dev/database-url");
    }

    #[test]
    fn test_should_use_secure_string() {
        assert!(ParameterStoreManager::should_use_secure_string(
            "DatabasePassword",
            false
        ));
        assert!(ParameterStoreManager::should_use_secure_string(
            "ApiSecret",
            false
        ));
        assert!(ParameterStoreManager::should_use_secure_string(
            "EncryptionKey",
            false
        ));
        assert!(ParameterStoreManager::should_use_secure_string(
            "AuthToken",
            false
        ));
        assert!(ParameterStoreManager::should_use_secure_string(
            "UserCredentials",
            false
        ));
        assert!(ParameterStoreManager::should_use_secure_string(
            "SomeParam",
            true
        )); // NoEcho = true

        assert!(!ParameterStoreManager::should_use_secure_string(
            "InstanceType",
            false
        ));
        assert!(!ParameterStoreManager::should_use_secure_string(
            "Region", false
        ));
    }

    #[test]
    fn test_transform_template_for_parameter_store() {
        let template = r#"{
            "Parameters": {
                "DatabasePassword": {
                    "Type": "String",
                    "Description": "Password for the database",
                    "NoEcho": true
                },
                "InstanceType": {
                    "Type": "String",
                    "Default": "t3.micro"
                }
            }
        }"#;

        let project = create_test_project();
        let mut parameters_to_store = HashMap::new();
        parameters_to_store.insert("DatabasePassword".to_string(), "secret123".to_string());

        let manager = ParameterStoreManager::new(Arc::new(
            crate::app::resource_explorer::credentials::CredentialCoordinator::new_mock(),
        ));

        let result = manager.transform_template_for_parameter_store(
            template,
            &project,
            "dev",
            &parameters_to_store,
        );

        assert!(result.is_ok());
        let transformed = result.unwrap();

        // Check that the parameter type was transformed
        assert!(transformed.contains("AWS::SSM::Parameter::Value<String>"));
        assert!(transformed.contains("/testapp/dev/DatabasePassword"));

        // Check that non-transformed parameters remain unchanged
        assert!(transformed.contains("t3.micro"));
    }
}
