use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Backup Plans
pub struct BackupPlanNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BackupPlanNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let backup_plan_id = raw_response
            .get("BackupPlanId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-plan")
            .to_string();

        // Use BackupPlanName if available, otherwise fallback to BackupPlanId
        let display_name = raw_response
            .get("BackupPlanName")
            .and_then(|v| v.as_str())
            .unwrap_or(&backup_plan_id)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Backup::BackupPlan", &backup_plan_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Backup::BackupPlan {}: {}",
                    backup_plan_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::Backup::BackupPlan".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: backup_plan_id,
            display_name,
            status,
            properties: raw_response,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Map to backup vaults referenced in the backup plan rules
        if let Some(rules) = entry.properties.get("Rules").and_then(|v| v.as_array()) {
            for rule in rules {
                if let Some(target_vault) = rule.get("TargetBackupVault").and_then(|v| v.as_str()) {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::Backup::BackupVault"
                            && resource.resource_id == target_vault
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: target_vault.to_string(),
                                target_resource_type: "AWS::Backup::BackupVault".to_string(),
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Backup::BackupPlan"
    }
}

/// Normalizer for Backup Vaults
pub struct BackupVaultNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BackupVaultNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let backup_vault_name = raw_response
            .get("BackupVaultName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-vault")
            .to_string();

        let display_name = extract_display_name(&raw_response, &backup_vault_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::Backup::BackupVault",
                &backup_vault_name,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Backup::BackupVault {}: {}",
                    backup_vault_name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::Backup::BackupVault".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: backup_vault_name,
            display_name,
            status,
            properties: raw_response,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Backup vaults store recovery points but don't have direct relationships
        // with other AWS resources beyond being referenced by backup plans
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Backup::BackupVault"
    }
}
