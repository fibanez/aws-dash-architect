use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_securityhub as securityhub;
use std::sync::Arc;

pub struct SecurityHubService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SecurityHubService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Security Hub hubs (there's only one per region per account)
    pub async fn list_hubs(
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

        let client = securityhub::Client::new(&aws_config);

        // Try to describe the hub to see if Security Hub is enabled
        match self.describe_hub_internal(&client).await {
            Ok(hub_details) => Ok(vec![hub_details]),
            Err(_) => {
                // If describe fails, Security Hub is probably not enabled in this region
                // Return empty list
                Ok(Vec::new())
            }
        }
    }

    /// Get detailed information for Security Hub
    pub async fn describe_hub(
        &self,
        account_id: &str,
        region: &str,
        _hub_arn: &str, // Not used since there's only one hub per region
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

        let client = securityhub::Client::new(&aws_config);
        self.describe_hub_internal(&client).await
    }

    async fn describe_hub_internal(
        &self,
        client: &securityhub::Client,
    ) -> Result<serde_json::Value> {
        let response = client.describe_hub().send().await?;

        Ok(self.hub_to_json(&response))
    }

    fn hub_to_json(
        &self,
        response: &securityhub::operation::describe_hub::DescribeHubOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        // Create a synthetic hub ARN for identification
        json.insert(
            "HubArn".to_string(),
            serde_json::Value::String("SecurityHub".to_string()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String("SecurityHub".to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String("Security Hub".to_string()),
        );

        if let Some(subscribed_at) = &response.subscribed_at {
            json.insert(
                "SubscribedAt".to_string(),
                serde_json::Value::String(subscribed_at.to_string()),
            );
        }

        json.insert(
            "AutoEnableControls".to_string(),
            serde_json::Value::Bool(response.auto_enable_controls.unwrap_or(false)),
        );

        if let Some(control_finding_generator) = &response.control_finding_generator {
            json.insert(
                "ControlFindingGenerator".to_string(),
                serde_json::Value::String(control_finding_generator.as_str().to_string()),
            );
        }

        // Status is always "ACTIVE" if we can describe the hub
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
