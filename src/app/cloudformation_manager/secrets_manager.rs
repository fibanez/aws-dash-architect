use crate::app::projects::Project;
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_secretsmanager as secrets;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsManagerEntry {
    pub secret_name: String,
    pub secret_arn: String,
    pub description: Option<String>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsManagerResult {
    pub success: bool,
    pub secret_name: String,
    pub secret_arn: Option<String>,
    pub dynamic_reference: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateTransformation {
    pub original_template: String,
    pub transformed_template: String,
    pub transformations: Vec<TransformationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationDetail {
    pub resource_name: String,
    pub property_path: String,
    pub original_value: String,
    pub new_value: String,
    pub secret_name: String,
}

/// Secrets Manager for CloudFormation template integration
pub struct SecretsManagerClient {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SecretsManagerClient {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// Generate secret name using convention: /{app-name}/{environment}/{secret-name}
    pub fn generate_secret_name(project: &Project, environment: &str, secret_name: &str) -> String {
        format!("{}/{}/{}", project.short_name, environment, secret_name)
    }

    /// Detect if a parameter should be stored as a secret
    pub fn is_sensitive_parameter(parameter_name: &str, no_echo: bool) -> bool {
        if no_echo {
            return true;
        }

        let lowercase_name = parameter_name.to_lowercase();
        lowercase_name.contains("password")
            || lowercase_name.contains("secret")
            || lowercase_name.contains("key")
            || lowercase_name.contains("token")
            || lowercase_name.contains("credential")
            || lowercase_name.contains("auth")
            || lowercase_name.contains("api")
                && (lowercase_name.contains("key") || lowercase_name.contains("secret"))
    }

    /// Store a secret in AWS Secrets Manager
    #[allow(clippy::too_many_arguments)]
    pub async fn store_secret(
        &self,
        project: &Project,
        environment: &str,
        secret_name: &str,
        secret_value: &str,
        description: Option<&str>,
        account_id: &str,
        region: &str,
    ) -> Result<SecretsManagerResult> {
        let secret_name_full = Self::generate_secret_name(project, environment, secret_name);

        debug!(
            "Storing secret {} in Secrets Manager for account {} in region {}",
            secret_name_full, account_id, region
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

        let client = secrets::Client::new(&aws_config);

        // Try to create the secret first
        let mut create_request = client
            .create_secret()
            .name(&secret_name_full)
            .secret_string(secret_value);

        if let Some(desc) = description {
            create_request = create_request.description(desc);
        }

        // Add tags to identify the secret
        let mut tags = HashMap::new();
        tags.insert("Project".to_string(), project.name.clone());
        tags.insert("Environment".to_string(), environment.to_string());
        tags.insert("ManagedBy".to_string(), "AWSDash".to_string());

        let tag_list: Vec<secrets::types::Tag> = tags
            .iter()
            .map(|(key, value)| secrets::types::Tag::builder().key(key).value(value).build())
            .collect();

        create_request = create_request.set_tags(Some(tag_list));

        match create_request.send().await {
            Ok(response) => {
                let secret_arn = response.arn().unwrap_or_default().to_string();
                let dynamic_reference = format!(
                    "{{{{resolve:secretsmanager:{}:SecretString}}}}",
                    secret_name_full
                );

                info!(
                    "Successfully created secret {} with ARN {}",
                    secret_name_full, secret_arn
                );

                Ok(SecretsManagerResult {
                    success: true,
                    secret_name: secret_name_full,
                    secret_arn: Some(secret_arn),
                    dynamic_reference: Some(dynamic_reference),
                    error_message: None,
                })
            }
            Err(sdk_error) => {
                // Check if the secret already exists
                if sdk_error.to_string().contains("ResourceExistsException") {
                    debug!(
                        "Secret {} already exists, updating instead",
                        secret_name_full
                    );

                    // Update the existing secret
                    match client
                        .update_secret()
                        .secret_id(&secret_name_full)
                        .secret_string(secret_value)
                        .send()
                        .await
                    {
                        Ok(_) => {
                            // Get the secret ARN
                            match client
                                .describe_secret()
                                .secret_id(&secret_name_full)
                                .send()
                                .await
                            {
                                Ok(describe_response) => {
                                    let secret_arn =
                                        describe_response.arn().unwrap_or_default().to_string();
                                    let dynamic_reference = format!(
                                        "{{{{resolve:secretsmanager:{}:SecretString}}}}",
                                        secret_name_full
                                    );

                                    info!(
                                        "Successfully updated secret {} with ARN {}",
                                        secret_name_full, secret_arn
                                    );

                                    Ok(SecretsManagerResult {
                                        success: true,
                                        secret_name: secret_name_full,
                                        secret_arn: Some(secret_arn),
                                        dynamic_reference: Some(dynamic_reference),
                                        error_message: None,
                                    })
                                }
                                Err(describe_error) => {
                                    let error_msg = format!(
                                        "Failed to describe secret {}: {}",
                                        secret_name_full, describe_error
                                    );
                                    warn!("{}", error_msg);
                                    Ok(SecretsManagerResult {
                                        success: false,
                                        secret_name: secret_name_full,
                                        secret_arn: None,
                                        dynamic_reference: None,
                                        error_message: Some(error_msg),
                                    })
                                }
                            }
                        }
                        Err(update_error) => {
                            let error_msg = format!(
                                "Failed to update secret {}: {}",
                                secret_name_full, update_error
                            );
                            warn!("{}", error_msg);
                            Ok(SecretsManagerResult {
                                success: false,
                                secret_name: secret_name_full,
                                secret_arn: None,
                                dynamic_reference: None,
                                error_message: Some(error_msg),
                            })
                        }
                    }
                } else {
                    let error_msg = format!(
                        "Failed to create secret {}: {}",
                        secret_name_full, sdk_error
                    );
                    warn!("{}", error_msg);
                    Ok(SecretsManagerResult {
                        success: false,
                        secret_name: secret_name_full,
                        secret_arn: None,
                        dynamic_reference: None,
                        error_message: Some(error_msg),
                    })
                }
            }
        }
    }

    /// List secrets for a project and environment
    pub async fn list_secrets(
        &self,
        project: &Project,
        environment: &str,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<SecretsManagerEntry>> {
        debug!(
            "Listing secrets for project {} environment {} in account {} region {}",
            project.short_name, environment, account_id, region
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

        let client = secrets::Client::new(&aws_config);

        let mut secrets_list = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_secrets();

            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(response) => {
                    for secret in response.secret_list() {
                        if let (Some(name), Some(arn)) = (secret.name(), secret.arn()) {
                            // Filter secrets that belong to this project and environment
                            let project_prefix = format!("{}/{}/", project.short_name, environment);
                            if name.starts_with(&project_prefix) {
                                let tags = HashMap::new();
                                // Note: tags() method might not be available in all API responses
                                // For now, we'll skip tag collection from list_secrets response

                                secrets_list.push(SecretsManagerEntry {
                                    secret_name: name.to_string(),
                                    secret_arn: arn.to_string(),
                                    description: secret.description().map(|s| s.to_string()),
                                    tags,
                                });
                            }
                        }
                    }

                    next_token = response.next_token().map(|s| s.to_string());
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(sdk_error) => {
                    warn!("Failed to list secrets: {}", sdk_error);
                    return Err(sdk_error.into());
                }
            }
        }

        info!(
            "Found {} secrets for project {} environment {}",
            secrets_list.len(),
            project.short_name,
            environment
        );
        Ok(secrets_list)
    }

    /// Transform CloudFormation template to use Secrets Manager dynamic references
    pub fn transform_template_for_secrets_manager(
        &self,
        template: &str,
        project: &Project,
        environment: &str,
        secrets_mapping: &HashMap<String, String>, // parameter_name -> secret_name
        secret_results: &HashMap<String, SecretsManagerResult>,
    ) -> Result<TemplateTransformation> {
        debug!(
            "Transforming CloudFormation template to use Secrets Manager for {} secrets",
            secrets_mapping.len()
        );

        let mut cfn_template: Value = serde_json::from_str(template)
            .with_context(|| "Failed to parse CloudFormation template as JSON")?;

        let mut transformations = Vec::new();

        // Transform resource properties to use dynamic references
        if let Some(resources_section) = cfn_template.get_mut("Resources") {
            if let Some(resources_obj) = resources_section.as_object_mut() {
                for (resource_name, resource_def) in resources_obj.iter_mut() {
                    if let Some(resource_obj) = resource_def.as_object_mut() {
                        if let Some(properties) = resource_obj.get_mut("Properties") {
                            self.transform_properties_recursive(
                                properties,
                                resource_name,
                                "",
                                project,
                                environment,
                                secrets_mapping,
                                secret_results,
                                &mut transformations,
                            );
                        }
                    }
                }
            }
        }

        let transformed_template = serde_json::to_string_pretty(&cfn_template)
            .with_context(|| "Failed to serialize transformed CloudFormation template")?;

        info!(
            "Template transformation completed with {} transformations",
            transformations.len()
        );

        Ok(TemplateTransformation {
            original_template: template.to_string(),
            transformed_template,
            transformations,
        })
    }

    /// Recursively transform properties to use dynamic references
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::only_used_in_recursion)]
    #[allow(clippy::manual_map)]
    fn transform_properties_recursive(
        &self,
        value: &mut Value,
        resource_name: &str,
        property_path: &str,
        _project: &Project,
        _environment: &str,
        secrets_mapping: &HashMap<String, String>,
        secret_results: &HashMap<String, SecretsManagerResult>,
        transformations: &mut Vec<TransformationDetail>,
    ) {
        // First check if this value needs transformation to avoid borrowing conflicts
        let transformation_info = match value {
            Value::Object(ref obj) => {
                if let Some(ref_value) = obj.get("Ref") {
                    if let Some(param_name) = ref_value.as_str() {
                        if let Some(secret_name) = secrets_mapping.get(param_name) {
                            if let Some(secret_result) = secret_results.get(secret_name) {
                                if secret_result.success {
                                    if let Some(dynamic_ref) = &secret_result.dynamic_reference {
                                        Some((
                                            param_name.to_string(),
                                            secret_name.clone(),
                                            dynamic_ref.clone(),
                                        ))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        // If transformation is needed, capture original value and apply transformation
        if let Some((param_name, secret_name, dynamic_ref)) = transformation_info {
            let original_value = serde_json::to_string(value).unwrap_or_default();
            *value = Value::String(dynamic_ref.clone());

            transformations.push(TransformationDetail {
                resource_name: resource_name.to_string(),
                property_path: property_path.to_string(),
                original_value,
                new_value: dynamic_ref.clone(),
                secret_name,
            });

            info!(
                "Transformed parameter reference {} to dynamic reference in resource {}",
                param_name, resource_name
            );
            return;
        }

        match value {
            Value::Object(ref mut obj) => {
                // Recursively process nested objects
                for (key, nested_value) in obj.iter_mut() {
                    let nested_path = if property_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", property_path, key)
                    };

                    self.transform_properties_recursive(
                        nested_value,
                        resource_name,
                        &nested_path,
                        _project,
                        _environment,
                        secrets_mapping,
                        secret_results,
                        transformations,
                    );
                }
            }
            Value::Array(ref mut arr) => {
                for (index, array_value) in arr.iter_mut().enumerate() {
                    let array_path = format!("{}[{}]", property_path, index);
                    self.transform_properties_recursive(
                        array_value,
                        resource_name,
                        &array_path,
                        _project,
                        _environment,
                        secrets_mapping,
                        secret_results,
                        transformations,
                    );
                }
            }
            _ => {
                // For primitive values, no transformation needed
            }
        }
    }

    /// Get common secret naming patterns
    pub fn get_common_secret_patterns() -> Vec<&'static str> {
        vec![
            "password",
            "secret",
            "key",
            "token",
            "credential",
            "auth",
            "apikey",
            "api_key",
            "private_key",
            "private-key",
            "oauth",
            "jwt",
            "certificate",
            "cert",
            "passphrase",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_project() -> Project {
        Project {
            name: "Test Project".to_string(),
            description: "Test project for secrets manager".to_string(),
            short_name: "testapp".to_string(),
            created: Utc::now(),
            updated: Utc::now(),
            local_folder: None,
            git_url: None,
            environments: vec![],
            default_region: None,
            cfn_template: None,
            compliance_programs: vec![],
            guard_rules_enabled: false,
            custom_guard_rules: vec![],
            environment_compliance: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_generate_secret_name() {
        let project = create_test_project();
        let secret_name =
            SecretsManagerClient::generate_secret_name(&project, "prod", "database-password");
        assert_eq!(secret_name, "testapp/prod/database-password");
    }

    #[test]
    fn test_is_sensitive_parameter() {
        assert!(SecretsManagerClient::is_sensitive_parameter(
            "DatabasePassword",
            false
        ));
        assert!(SecretsManagerClient::is_sensitive_parameter(
            "ApiSecret",
            false
        ));
        assert!(SecretsManagerClient::is_sensitive_parameter(
            "AuthToken",
            false
        ));
        assert!(SecretsManagerClient::is_sensitive_parameter(
            "PrivateKey",
            false
        ));
        assert!(SecretsManagerClient::is_sensitive_parameter(
            "ApiKey", false
        ));
        assert!(SecretsManagerClient::is_sensitive_parameter(
            "SomeParam",
            true
        )); // NoEcho = true

        assert!(!SecretsManagerClient::is_sensitive_parameter(
            "InstanceType",
            false
        ));
        assert!(!SecretsManagerClient::is_sensitive_parameter(
            "Region", false
        ));
        assert!(!SecretsManagerClient::is_sensitive_parameter(
            "BucketName",
            false
        ));
    }

    #[test]
    fn test_transform_template_for_secrets_manager() {
        let template = r#"{
            "Resources": {
                "MyDatabase": {
                    "Type": "AWS::RDS::DBInstance",
                    "Properties": {
                        "MasterUsername": "admin",
                        "MasterUserPassword": {
                            "Ref": "DatabasePassword"
                        },
                        "DBInstanceClass": "db.t3.micro"
                    }
                }
            }
        }"#;

        let project = create_test_project();
        let mut secrets_mapping = HashMap::new();
        secrets_mapping.insert(
            "DatabasePassword".to_string(),
            "database-password".to_string(),
        );

        let mut secret_results = HashMap::new();
        secret_results.insert("database-password".to_string(), SecretsManagerResult {
            success: true,
            secret_name: "testapp/prod/database-password".to_string(),
            secret_arn: Some("arn:aws:secretsmanager:us-east-1:123456789012:secret:testapp/prod/database-password-AbCdEf".to_string()),
            dynamic_reference: Some("{{resolve:secretsmanager:testapp/prod/database-password:SecretString}}".to_string()),
            error_message: None,
        });

        let client = SecretsManagerClient::new(Arc::new(
            crate::app::resource_explorer::credentials::CredentialCoordinator::new_mock(),
        ));

        let result = client.transform_template_for_secrets_manager(
            template,
            &project,
            "prod",
            &secrets_mapping,
            &secret_results,
        );

        assert!(result.is_ok());
        let transformation = result.unwrap();

        // Check that the parameter reference was transformed
        assert!(transformation
            .transformed_template
            .contains("{{resolve:secretsmanager:testapp/prod/database-password:SecretString}}"));
        assert_eq!(transformation.transformations.len(), 1);

        let transform_detail = &transformation.transformations[0];
        assert_eq!(transform_detail.resource_name, "MyDatabase");
        assert_eq!(transform_detail.property_path, "MasterUserPassword");
        assert_eq!(transform_detail.secret_name, "database-password");
    }
}
