use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_guardduty as guardduty;
use std::sync::Arc;

pub struct GuardDutyService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl GuardDutyService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List GuardDuty Detectors
    pub async fn list_detectors(
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

        let client = guardduty::Client::new(&aws_config);

        let mut detectors = Vec::new();

        // List detector IDs first
        let response = client.list_detectors().send().await?;

        if let Some(detector_ids) = response.detector_ids {
            for detector_id in detector_ids {
                if let Ok(detector_details) =
                    self.get_detector_internal(&client, &detector_id).await
                {
                    detectors.push(detector_details);
                } else {
                    // Fallback to basic detector info if get fails
                    let detector_json = self.detector_id_to_json(&detector_id);
                    detectors.push(detector_json);
                }
            }
        }

        Ok(detectors)
    }

    /// Get detailed information for specific GuardDuty Detector
    pub async fn describe_detector(
        &self,
        account_id: &str,
        region: &str,
        detector_id: &str,
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

        let client = guardduty::Client::new(&aws_config);
        self.get_detector_internal(&client, detector_id).await
    }

    async fn get_detector_internal(
        &self,
        client: &guardduty::Client,
        detector_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_detector()
            .detector_id(detector_id)
            .send()
            .await?;

        Ok(self.detector_to_json(&response, detector_id))
    }

    fn detector_id_to_json(&self, detector_id: &str) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        json.insert(
            "DetectorId".to_string(),
            serde_json::Value::String(detector_id.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(format!("GuardDuty-{}", detector_id)),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("UNKNOWN".to_string()),
        );
        serde_json::Value::Object(json)
    }

    fn detector_to_json(
        &self,
        detector_response: &guardduty::operation::get_detector::GetDetectorOutput,
        detector_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DetectorId".to_string(),
            serde_json::Value::String(detector_id.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(format!("GuardDuty-{}", detector_id)),
        );

        if let Some(created_at) = &detector_response.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.clone()),
            );
        }

        if let Some(updated_at) = &detector_response.updated_at {
            json.insert(
                "UpdatedAt".to_string(),
                serde_json::Value::String(updated_at.clone()),
            );
        }

        if let Some(service_role) = &detector_response.service_role {
            json.insert(
                "ServiceRole".to_string(),
                serde_json::Value::String(service_role.clone()),
            );
        }

        if let Some(status) = &detector_response.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(format!("{:?}", status)),
            );
        }

        if let Some(finding_publishing_frequency) = &detector_response.finding_publishing_frequency
        {
            json.insert(
                "FindingPublishingFrequency".to_string(),
                serde_json::Value::String(format!("{:?}", finding_publishing_frequency)),
            );
        }

        // Note: data_sources field is deprecated, using Features instead

        if let Some(features) = &detector_response.features {
            let features_array: Vec<serde_json::Value> = features
                .iter()
                .map(|feature| {
                    let mut feature_json = serde_json::Map::new();
                    if let Some(name) = &feature.name {
                        feature_json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(format!("{:?}", name)),
                        );
                    }
                    if let Some(status) = &feature.status {
                        feature_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(format!("{:?}", status)),
                        );
                    }
                    if let Some(updated_at) = &feature.updated_at {
                        feature_json.insert(
                            "UpdatedAt".to_string(),
                            serde_json::Value::String(updated_at.to_string()),
                        );
                    }
                    if let Some(additional_configuration) = &feature.additional_configuration {
                        let config_array: Vec<serde_json::Value> = additional_configuration
                            .iter()
                            .map(|config| {
                                let mut config_json = serde_json::Map::new();
                                if let Some(name) = &config.name {
                                    config_json.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(format!("{:?}", name)),
                                    );
                                }
                                if let Some(status) = &config.status {
                                    config_json.insert(
                                        "Status".to_string(),
                                        serde_json::Value::String(format!("{:?}", status)),
                                    );
                                }
                                if let Some(updated_at) = &config.updated_at {
                                    config_json.insert(
                                        "UpdatedAt".to_string(),
                                        serde_json::Value::String(updated_at.to_string()),
                                    );
                                }
                                serde_json::Value::Object(config_json)
                            })
                            .collect();
                        feature_json.insert(
                            "AdditionalConfiguration".to_string(),
                            serde_json::Value::Array(config_array),
                        );
                    }
                    serde_json::Value::Object(feature_json)
                })
                .collect();
            json.insert(
                "Features".to_string(),
                serde_json::Value::Array(features_array),
            );
        }

        if let Some(tags) = &detector_response.tags {
            let tags_array: Vec<serde_json::Value> = tags
                .iter()
                .map(|(key, value)| {
                    let mut tag_json = serde_json::Map::new();
                    tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    tag_json.insert(
                        "Value".to_string(),
                        serde_json::Value::String(value.clone()),
                    );
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Array(tags_array));
        }

        serde_json::Value::Object(json)
    }
}
