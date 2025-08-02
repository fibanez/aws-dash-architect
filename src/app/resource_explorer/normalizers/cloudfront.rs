use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct CloudFrontDistributionNormalizer;

impl ResourceNormalizer for CloudFrontDistributionNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("Id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                raw_response
                    .get("DomainName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            })
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudFront::Distribution".to_string(),
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
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // CloudFront distributions can be associated with WAF WebACLs
        if let Some(web_acl_id) = entry
            .raw_properties
            .get("WebACLId")
            .and_then(|v| v.as_str())
        {
            if !web_acl_id.is_empty() {
                for resource in all_resources {
                    if resource.resource_type == "AWS::WAFv2::WebACL"
                        && resource.resource_id == web_acl_id
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }
        }

        // CloudFront distributions can use ACM certificates
        if let Some(viewer_cert) = entry
            .raw_properties
            .get("ViewerCertificate")
            .and_then(|v| v.as_object())
        {
            if let Some(acm_cert_arn) = viewer_cert
                .get("ACMCertificateArn")
                .and_then(|v| v.as_str())
            {
                // Extract certificate ID from ARN
                if let Some(cert_id) = acm_cert_arn.split('/').next_back() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::CertificateManager::Certificate"
                            && resource.resource_id == cert_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
            }
        }

        // CloudFront distributions can use S3 buckets as origins
        if let Some(origins) = entry
            .raw_properties
            .get("Origins")
            .and_then(|v| v.as_array())
        {
            for origin in origins {
                if let Some(domain_name) = origin.get("DomainName").and_then(|v| v.as_str()) {
                    // Check if this is an S3 domain
                    if domain_name.contains(".s3.") || domain_name.contains(".s3-") {
                        // Extract bucket name from S3 domain
                        let bucket_name = domain_name.split('.').next().unwrap_or("");
                        for resource in all_resources {
                            if resource.resource_type == "AWS::S3::Bucket"
                                && resource.display_name == bucket_name
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudFront::Distribution"
    }
}
