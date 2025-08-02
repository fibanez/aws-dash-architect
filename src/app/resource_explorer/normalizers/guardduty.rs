use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS GuardDuty Detectors
pub struct GuardDutyDetectorNormalizer;

impl ResourceNormalizer for GuardDutyDetectorNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DetectorId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-detector")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("GuardDuty-{}", resource_id))
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::GuardDuty::Detector".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
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
        // GuardDuty detectors may be associated with:
        // - IAM service roles
        // - S3 buckets for threat intelligence
        // - SNS topics for findings notifications
        // - CloudWatch events for automated response
        // These relationships would require additional API calls
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::GuardDuty::Detector"
    }
}
