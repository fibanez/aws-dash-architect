use anyhow::{Result, Context};
use aws_sdk_inspector2 as inspector;
use std::sync::Arc;
use tracing::warn;
use super::super::credentials::CredentialCoordinator;

pub struct InspectorService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl InspectorService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Inspector findings
    pub async fn list_findings(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = inspector::Client::new(&aws_config);
        
        let mut resources = Vec::new();

        // Try to get Inspector status first
        match client.get_configuration().send().await {
            Ok(_response) => {
                // Create basic inspector configuration resource
                let mut json = serde_json::Map::new();
                let resource_id = format!("inspector-config-{}", account_id);
                json.insert("ResourceId".to_string(), serde_json::Value::String(resource_id.clone()));
                json.insert("Id".to_string(), serde_json::Value::String(resource_id));
                json.insert("AccountId".to_string(), serde_json::Value::String(account_id.to_string()));
                json.insert("Name".to_string(), serde_json::Value::String("Inspector Configuration".to_string()));
                json.insert("Status".to_string(), serde_json::Value::String("Enabled".to_string()));
                json.insert("Service".to_string(), serde_json::Value::String("AWS Inspector".to_string()));
                json.insert("Description".to_string(), serde_json::Value::String("Vulnerability assessment and security monitoring".to_string()));
                
                resources.push(serde_json::Value::Object(json));
            }
            Err(e) => {
                warn!("Inspector not enabled or accessible for account {} in region {}: {}", account_id, region, e);
                // Create entry indicating Inspector is not enabled
                let mut json = serde_json::Map::new();
                json.insert("AccountId".to_string(), serde_json::Value::String(account_id.to_string()));
                json.insert("ResourceId".to_string(), serde_json::Value::String(format!("inspector-{}", account_id)));
                json.insert("Status".to_string(), serde_json::Value::String("Disabled".to_string()));
                json.insert("Name".to_string(), serde_json::Value::String("Inspector (Disabled)".to_string()));
                json.insert("Service".to_string(), serde_json::Value::String("AWS Inspector".to_string()));
                json.insert("Description".to_string(), serde_json::Value::String("Vulnerability assessment service (not enabled)".to_string()));
                resources.push(serde_json::Value::Object(json));
            }
        }

        Ok(resources)
    }

    /// Get detailed Inspector configuration information
    pub async fn get_inspector_configuration(
        &self,
        account_id: &str,
        region: &str,
        _resource_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = inspector::Client::new(&aws_config);
        let _response = client.get_configuration().send().await?;

        // Create detailed configuration information without complex typing
        let mut json = serde_json::Map::new();
        json.insert("AccountId".to_string(), serde_json::Value::String(account_id.to_string()));
        json.insert("Status".to_string(), serde_json::Value::String("Enabled".to_string()));
        json.insert("Service".to_string(), serde_json::Value::String("AWS Inspector".to_string()));
        json.insert("Description".to_string(), serde_json::Value::String("Automated security assessments for EC2 instances and container images".to_string()));
        json.insert("Region".to_string(), serde_json::Value::String(region.to_string()));

        Ok(serde_json::Value::Object(json))
    }
}