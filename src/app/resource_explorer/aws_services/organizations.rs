use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_organizations as organizations;
use std::sync::Arc;

pub struct OrganizationsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl OrganizationsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List AWS Organizations Organizational Units
    pub async fn list_organizational_units(
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

        let client = organizations::Client::new(&aws_config);

        // First, we need to get the root to list OUs
        let roots_response = client.list_roots().send().await?;

        let mut organizational_units = Vec::new();

        if let Some(roots) = roots_response.roots {
            for root in roots {
                if let Some(root_id) = &root.id {
                    // List OUs for this root
                    let mut paginator = client
                        .list_organizational_units_for_parent()
                        .parent_id(root_id)
                        .into_paginator()
                        .send();

                    while let Some(page) = paginator.next().await {
                        let page = page?;
                        if let Some(ous) = page.organizational_units {
                            for ou in ous {
                                let ou_json = self.organizational_unit_to_json(&ou);
                                organizational_units.push(ou_json);
                            }
                        }
                    }

                    // Recursively get OUs under each found OU
                    let current_ous = organizational_units.clone();
                    for ou_json in &current_ous {
                        if let Some(ou_id) = ou_json.get("Id").and_then(|v| v.as_str()) {
                            self.list_ous_recursively(&client, ou_id, &mut organizational_units)
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(organizational_units)
    }

    /// Recursively list OUs under a parent
    fn list_ous_recursively<'a>(
        &'a self,
        client: &'a organizations::Client,
        parent_id: &'a str,
        organizational_units: &'a mut Vec<serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a + Send>> {
        Box::pin(async move {
            let mut paginator = client
                .list_organizational_units_for_parent()
                .parent_id(parent_id)
                .into_paginator()
                .send();

            let mut child_ous = Vec::new();
            while let Some(page) = paginator.next().await {
                let page = page?;
                if let Some(ous) = page.organizational_units {
                    for ou in ous {
                        let ou_json = self.organizational_unit_to_json(&ou);
                        child_ous.push(ou_json.clone());
                        organizational_units.push(ou_json);
                    }
                }
            }

            // Recursively process child OUs
            for child_ou in &child_ous {
                if let Some(child_id) = child_ou.get("Id").and_then(|v| v.as_str()) {
                    self.list_ous_recursively(client, child_id, organizational_units)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Get detailed information for specific Organizational Unit
    pub async fn describe_organizational_unit(
        &self,
        account_id: &str,
        region: &str,
        ou_id: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client
            .describe_organizational_unit()
            .organizational_unit_id(ou_id)
            .send()
            .await?;

        if let Some(ou) = response.organizational_unit {
            Ok(self.organizational_unit_details_to_json(&ou))
        } else {
            Err(anyhow::anyhow!("Organizational Unit {} not found", ou_id))
        }
    }

    /// List AWS Organizations Service Control Policies
    pub async fn list_policies(
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

        let client = organizations::Client::new(&aws_config);

        // List Service Control Policies
        let mut paginator = client
            .list_policies()
            .filter(organizations::types::PolicyType::ServiceControlPolicy)
            .into_paginator()
            .send();

        let mut policies = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(policy_summaries) = page.policies {
                for policy in policy_summaries {
                    let policy_json = self.policy_summary_to_json(&policy);
                    policies.push(policy_json);
                }
            }
        }

        Ok(policies)
    }

    /// Get detailed information for specific Service Control Policy
    pub async fn describe_policy(
        &self,
        account_id: &str,
        region: &str,
        policy_id: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client.describe_policy().policy_id(policy_id).send().await?;

        if let Some(policy) = response.policy {
            Ok(self.policy_details_to_json(&policy))
        } else {
            Err(anyhow::anyhow!("Policy {} not found", policy_id))
        }
    }

    // JSON conversion methods
    fn organizational_unit_to_json(
        &self,
        ou: &organizations::types::OrganizationalUnit,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &ou.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(ou.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &ou.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &ou.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn organizational_unit_details_to_json(
        &self,
        ou: &organizations::types::OrganizationalUnit,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &ou.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(ou.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &ou.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &ou.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn policy_summary_to_json(
        &self,
        policy: &organizations::types::PolicySummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &policy.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(policy.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &policy.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &policy.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(policy_type) = &policy.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(policy_type.as_str().to_string()),
            );
        }

        if let Some(description) = &policy.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "AwsManaged".to_string(),
            serde_json::Value::Bool(policy.aws_managed),
        );

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn policy_details_to_json(&self, policy: &organizations::types::Policy) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(policy_summary) = &policy.policy_summary {
            if let Some(id) = &policy_summary.id {
                json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(
                        policy_summary.name.as_deref().unwrap_or(id).to_string(),
                    ),
                );
            }

            if let Some(name) = &policy_summary.name {
                json.insert(
                    "DisplayName".to_string(),
                    serde_json::Value::String(name.clone()),
                );
            }

            if let Some(arn) = &policy_summary.arn {
                json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
            }

            if let Some(policy_type) = &policy_summary.r#type {
                json.insert(
                    "Type".to_string(),
                    serde_json::Value::String(policy_type.as_str().to_string()),
                );
            }

            if let Some(description) = &policy_summary.description {
                json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description.clone()),
                );
            }

            json.insert(
                "AwsManaged".to_string(),
                serde_json::Value::Bool(policy_summary.aws_managed),
            );
        }

        if let Some(content) = &policy.content {
            json.insert(
                "Content".to_string(),
                serde_json::Value::String(content.clone()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
