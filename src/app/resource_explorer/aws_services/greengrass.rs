use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_greengrassv2 as greengrassv2;
use std::sync::Arc;

pub struct GreengrassService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl GreengrassService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Greengrass component versions
    pub async fn list_component_versions(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
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

        let client = greengrassv2::Client::new(&aws_config);

        // First get list of components, then get their versions
        let components_response = client.list_components().send().await?;

        let mut component_versions = Vec::new();

        if let Some(components) = components_response.components {
            for component in components {
                if let Some(_component_name) = &component.component_name {
                    // Create a basic component entry since list_component_versions requires ARN
                    let basic_component = self.component_to_json(&component);
                    component_versions.push(basic_component);
                }
            }
        }

        Ok(component_versions)
    }

    /// Get detailed information for specific Greengrass component version
    pub async fn describe_component_version(
        &self,
        account_id: &str,
        region: &str,
        component_arn: &str,
    ) -> Result<serde_json::Value> {
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

        let client = greengrassv2::Client::new(&aws_config);
        let response = client.get_component().arn(component_arn).send().await?;

        let mut component_details = serde_json::Map::new();

        // Extract component name and version from ARN
        let arn_parts: Vec<&str> = component_arn.split(':').collect();
        if arn_parts.len() >= 6 {
            let component_name = arn_parts[5].split('/').nth(1).unwrap_or("unknown");
            let component_version = arn_parts[5].split('/').nth(2).unwrap_or("unknown");

            component_details.insert(
                "Arn".to_string(),
                serde_json::Value::String(component_arn.to_string()),
            );
            component_details.insert(
                "ComponentName".to_string(),
                serde_json::Value::String(component_name.to_string()),
            );
            component_details.insert(
                "ComponentVersion".to_string(),
                serde_json::Value::String(component_version.to_string()),
            );
        }

        // The GetComponent API returns a recipe and tags, not metadata
        // Convert Blob to string (recipe is typically JSON)
        if let Ok(recipe_str) = String::from_utf8(response.recipe.into_inner()) {
            component_details.insert("Recipe".to_string(), serde_json::Value::String(recipe_str));
        }

        if let Some(tags) = response.tags {
            let tags_map: serde_json::Map<String, serde_json::Value> = tags
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            component_details.insert("Tags".to_string(), serde_json::Value::Object(tags_map));
        }

        Ok(serde_json::Value::Object(component_details))
    }

    #[allow(dead_code)]
    fn component_version_to_json(
        &self,
        component_version: &greengrassv2::types::ComponentVersionListItem,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(component_name) = &component_version.component_name {
            json.insert(
                "ComponentName".to_string(),
                serde_json::Value::String(component_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(component_name.clone()),
            );
        }

        if let Some(component_version) = &component_version.component_version {
            json.insert(
                "ComponentVersion".to_string(),
                serde_json::Value::String(component_version.clone()),
            );
        }

        if let Some(arn) = &component_version.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        // ComponentVersionListItem only has component_name, component_version, and arn

        // Set status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn component_to_json(&self, component: &greengrassv2::types::Component) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(component_name) = &component.component_name {
            json.insert(
                "ComponentName".to_string(),
                serde_json::Value::String(component_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(component_name.clone()),
            );
        }

        if let Some(arn) = &component.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(latest_version) = &component.latest_version {
            if let Some(component_version) = &latest_version.component_version {
                json.insert(
                    "ComponentVersion".to_string(),
                    serde_json::Value::String(component_version.clone()),
                );
            }
            if let Some(creation_timestamp) = latest_version.creation_timestamp {
                json.insert(
                    "CreationTimestamp".to_string(),
                    serde_json::Value::String(creation_timestamp.to_string()),
                );
            }
            if let Some(description) = &latest_version.description {
                json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description.clone()),
                );
            }
            if let Some(publisher) = &latest_version.publisher {
                json.insert(
                    "Publisher".to_string(),
                    serde_json::Value::String(publisher.clone()),
                );
            }
        }

        // Set status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
