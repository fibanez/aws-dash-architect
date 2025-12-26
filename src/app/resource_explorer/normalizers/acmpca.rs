use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Private Certificate Authority
pub struct AcmPcaNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for AcmPcaNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("Arn")
            .and_then(|v| v.as_str())
            .and_then(|arn| arn.split('/').next_back())
            .unwrap_or("unknown-ca")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .or_else(|| {
                // Try to extract common name from subject
                raw_response
                    .get("CertificateAuthorityConfiguration")
                    .and_then(|config| config.get("Subject"))
                    .and_then(|subject| subject.get("CommonName"))
                    .and_then(|cn| cn.as_str())
            })
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::ACMPCA::CertificateAuthority",
                &resource_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ACMPCA::CertificateAuthority {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ACMPCA::CertificateAuthority".to_string(),
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
        __entry: &ResourceEntry,
        __all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Private CAs may be associated with:
        // - S3 buckets (for CRL storage)
        // - Certificates issued by this CA
        // - IAM roles for permissions
        // These relationships would be extracted here
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ACMPCA::CertificateAuthority"
    }
}
