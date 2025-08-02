use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_route53 as route53;
use std::sync::Arc;

pub struct Route53Service {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl Route53Service {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Route53 Hosted Zones
    pub async fn list_hosted_zones(
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

        let client = route53::Client::new(&aws_config);
        let mut paginator = client.list_hosted_zones().into_paginator().send();

        let mut hosted_zones = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            for zone in page.hosted_zones {
                let zone_json = self.hosted_zone_to_json(&zone);
                hosted_zones.push(zone_json);
            }
        }

        Ok(hosted_zones)
    }

    /// Get detailed information for specific hosted zone
    pub async fn describe_hosted_zone(
        &self,
        account_id: &str,
        region: &str,
        hosted_zone_id: &str,
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

        let client = route53::Client::new(&aws_config);
        self.describe_hosted_zone_internal(&client, hosted_zone_id)
            .await
    }

    async fn describe_hosted_zone_internal(
        &self,
        client: &route53::Client,
        hosted_zone_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client.get_hosted_zone().id(hosted_zone_id).send().await?;

        if let Some(hosted_zone) = response.hosted_zone {
            Ok(self.hosted_zone_details_to_json(&hosted_zone))
        } else {
            Err(anyhow::anyhow!("Hosted zone {} not found", hosted_zone_id))
        }
    }

    fn hosted_zone_to_json(&self, hosted_zone: &route53::types::HostedZone) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Id".to_string(),
            serde_json::Value::String(hosted_zone.id.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(hosted_zone.id.clone()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(hosted_zone.name.clone()),
        );

        json.insert(
            "CallerReference".to_string(),
            serde_json::Value::String(hosted_zone.caller_reference.clone()),
        );

        if let Some(config) = &hosted_zone.config {
            let mut config_json = serde_json::Map::new();

            if let Some(comment) = &config.comment {
                config_json.insert(
                    "Comment".to_string(),
                    serde_json::Value::String(comment.clone()),
                );
            }

            config_json.insert(
                "PrivateZone".to_string(),
                serde_json::Value::Bool(config.private_zone),
            );

            json.insert("Config".to_string(), serde_json::Value::Object(config_json));
        }

        if let Some(resource_record_set_count) = hosted_zone.resource_record_set_count {
            json.insert(
                "ResourceRecordSetCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(resource_record_set_count)),
            );
        }

        if let Some(linked_service) = &hosted_zone.linked_service {
            let mut linked_service_json = serde_json::Map::new();

            if let Some(service_principal) = &linked_service.service_principal {
                linked_service_json.insert(
                    "ServicePrincipal".to_string(),
                    serde_json::Value::String(service_principal.clone()),
                );
            }

            if let Some(description) = &linked_service.description {
                linked_service_json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description.clone()),
                );
            }

            json.insert(
                "LinkedService".to_string(),
                serde_json::Value::Object(linked_service_json),
            );
        }

        // Add a default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn hosted_zone_details_to_json(
        &self,
        hosted_zone: &route53::types::HostedZone,
    ) -> serde_json::Value {
        // For detailed view, we can use the same conversion as the basic one
        // In a real implementation, we might fetch additional details like record sets
        self.hosted_zone_to_json(hosted_zone)
    }
}
