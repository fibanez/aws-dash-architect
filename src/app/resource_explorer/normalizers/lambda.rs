use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Lambda Functions
pub struct LambdaFunctionNormalizer;

impl ResourceNormalizer for LambdaFunctionNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let function_name = raw_response
            .get("FunctionName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-function")
            .to_string();

        let display_name = extract_display_name(&raw_response, &function_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Lambda::Function".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: function_name,
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
        // Lambda functions can have relationships with IAM roles, VPCs, etc.
        // but we'd need to parse the function configuration for that
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Lambda::Function"
    }
}

/// Normalizer for Lambda Layer Versions
pub struct LambdaLayerVersionNormalizer;

impl ResourceNormalizer for LambdaLayerVersionNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let layer_name = raw_response
            .get("LayerName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-layer")
            .to_string();

        let display_name = extract_display_name(&raw_response, &layer_name);
        let status = Some("Available".to_string()); // Layers don't have status, default to Available
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Lambda::LayerVersion".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: layer_name,
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
        // Lambda layers are used by functions but don't have direct dependencies
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Lambda::LayerVersion"
    }
}

/// Normalizer for Lambda Event Source Mappings
pub struct LambdaEventSourceMappingNormalizer;

impl ResourceNormalizer for LambdaEventSourceMappingNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let uuid = raw_response
            .get("UUID")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-mapping")
            .to_string();

        // Create a more meaningful display name from function and event source
        let display_name = if let (Some(function_name), Some(event_source_arn)) = (
            raw_response.get("FunctionName").and_then(|v| v.as_str()),
            raw_response.get("EventSourceArn").and_then(|v| v.as_str()),
        ) {
            // Extract service name from event source ARN for cleaner display
            let event_source_service = event_source_arn.split(':').nth(2).unwrap_or("unknown");
            format!("{} -> {}", event_source_service, function_name)
        } else {
            uuid.clone()
        };

        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Lambda::EventSourceMapping".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: uuid,
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

        // Map to Lambda function
        if let Some(function_name) = entry
            .raw_properties
            .get("FunctionName")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::Lambda::Function"
                    && resource.resource_id == function_name
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: function_name.to_string(),
                        target_resource_type: "AWS::Lambda::Function".to_string(),
                    });
                }
            }
        }

        // Could potentially map to event sources (SQS, Kinesis, etc.) if they're in the resource list
        // This would require parsing the EventSourceArn and matching to other AWS resources

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Lambda::EventSourceMapping"
    }
}
