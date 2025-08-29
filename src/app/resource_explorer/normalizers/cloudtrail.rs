use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for CloudTrail Trails
pub struct CloudTrailNormalizer;

/// Normalizer for CloudTrail Events
pub struct CloudTrailEventNormalizer;

impl ResourceNormalizer for CloudTrailNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-trail")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudTrail::Trail".to_string(),
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
        // CloudTrail trails can be related to S3 buckets, SNS topics, CloudWatch logs, etc.
        // Implementation would analyze S3BucketName, SnsTopicArn, CloudWatchLogsLogGroupArn
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudTrail::Trail"
    }
}

impl ResourceNormalizer for CloudTrailEventNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        // Extract EventId as the unique identifier
        let resource_id = raw_response
            .get("EventId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-event")
            .to_string();

        // Create display name using EventName and EventTime
        let event_name = raw_response
            .get("EventName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Event");
        
        let event_time = raw_response
            .get("EventTime")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let display_name = if !event_time.is_empty() {
            format!("{} ({})", event_name, event_time)
        } else {
            event_name.to_string()
        };

        // Determine status based on event properties
        let status = if raw_response.get("ErrorCode").is_some() {
            Some("Failed".to_string())
        } else {
            Some("Success".to_string())
        };

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudTrail::Event".to_string(),
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

        // CloudTrail events can be related to the resources they operated on
        if let Some(resources) = entry.raw_properties.get("Resources").and_then(|r| r.as_array()) {
            for resource in resources {
                if let (Some(resource_type), Some(resource_name)) = (
                    resource.get("ResourceType").and_then(|rt| rt.as_str()),
                    resource.get("ResourceName").and_then(|rn| rn.as_str()),
                ) {
                    // Find matching resource in all_resources
                    for other_resource in all_resources {
                        if other_resource.resource_type == resource_type 
                            && (other_resource.resource_id == resource_name || other_resource.display_name.contains(resource_name)) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: other_resource.resource_id.clone(),
                                target_resource_type: other_resource.resource_type.clone(),
                            });
                            break;
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudTrail::Event"
    }
}
