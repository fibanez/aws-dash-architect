use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for SQS Queues
pub struct SQSQueueNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for SQSQueueNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let queue_url = raw_response
            .get("QueueUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-queue-url")
            .to_string();

        // Extract queue name from URL for display and resource ID
        let queue_name = queue_url.split('/').next_back().unwrap_or(&queue_url);

        let display_name = extract_display_name(&raw_response, queue_name);
        let status = extract_status(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::SQS::Queue".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: queue_name.to_string(),
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: Vec::new(), // Will be filled below
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        };

        // Fetch tags from AWS
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for {} {}: {:?}",
                    entry.resource_type,
                    entry.resource_id,
                    e
                );
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Check for Dead Letter Queue relationship
        if let Some(redrive_policy) = entry.raw_properties.get("RedrivePolicy") {
            if let Some(redrive_policy_str) = redrive_policy.as_str() {
                // Parse the RedrivePolicy JSON to extract the deadLetterTargetArn
                if let Ok(redrive_json) =
                    serde_json::from_str::<serde_json::Value>(redrive_policy_str)
                {
                    if let Some(dlq_arn) = redrive_json
                        .get("deadLetterTargetArn")
                        .and_then(|v| v.as_str())
                    {
                        // Extract queue name from DLQ ARN
                        // ARN format: arn:aws:sqs:region:account-id:queue-name
                        if let Some(dlq_queue_name) = dlq_arn.split(':').next_back() {
                            // Find the matching DLQ in all_resources
                            for resource in all_resources {
                                if resource.resource_type == "AWS::SQS::Queue"
                                    && resource.resource_id == dlq_queue_name
                                {
                                    // Extract maxReceiveCount if available (for future use)
                                    let _max_receive_count = redrive_json
                                        .get("maxReceiveCount")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);

                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::DeadLetterQueue,
                                        target_resource_type: "AWS::SQS::Queue".to_string(),
                                        target_resource_id: dlq_queue_name.to_string(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check if this queue serves as a DLQ for other queues
        for resource in all_resources {
            if resource.resource_type == "AWS::SQS::Queue"
                && resource.resource_id != entry.resource_id
            {
                if let Some(other_redrive_policy) = resource.raw_properties.get("RedrivePolicy") {
                    if let Some(other_redrive_policy_str) = other_redrive_policy.as_str() {
                        if let Ok(other_redrive_json) =
                            serde_json::from_str::<serde_json::Value>(other_redrive_policy_str)
                        {
                            if let Some(dlq_arn) = other_redrive_json
                                .get("deadLetterTargetArn")
                                .and_then(|v| v.as_str())
                            {
                                if let Some(dlq_queue_name) = dlq_arn.split(':').next_back() {
                                    if dlq_queue_name == entry.resource_id {
                                        // This queue is the DLQ for the other queue
                                        let _max_receive_count = other_redrive_json
                                            .get("maxReceiveCount")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);

                                        relationships.push(ResourceRelationship {
                                            relationship_type: RelationshipType::ServesAsDlq,
                                            target_resource_type: "AWS::SQS::Queue".to_string(),
                                            target_resource_id: resource.resource_id.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::SQS::Queue"
    }
}
