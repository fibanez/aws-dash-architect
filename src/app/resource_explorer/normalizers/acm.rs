use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for ACM Certificates
pub struct AcmCertificateNormalizer;

impl ResourceNormalizer for AcmCertificateNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("CertificateArn")
            .and_then(|v| v.as_str())
            .and_then(|arn| arn.split('/').next_back())
            .unwrap_or("unknown-certificate")
            .to_string();

        let display_name = raw_response
            .get("DomainName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CertificateManager::Certificate".to_string(),
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
        // ACM certificates may be associated with:
        // - CloudFront distributions
        // - Application Load Balancers
        // - API Gateway custom domains
        // These relationships would be extracted here
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CertificateManager::Certificate"
    }
}
