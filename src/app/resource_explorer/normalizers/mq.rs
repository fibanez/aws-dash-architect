use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Amazon MQ Brokers
pub struct MQBrokerNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for MQBrokerNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let broker_id = raw_response
            .get("BrokerId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-broker")
            .to_string();

        // Use broker name if available, otherwise fallback to broker ID
        let display_name = raw_response
            .get("BrokerName")
            .and_then(|v| v.as_str())
            .unwrap_or(&broker_id)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::AmazonMQ::Broker", &broker_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::AmazonMQ::Broker {}: {}",
                    broker_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::AmazonMQ::Broker".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: broker_id,
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
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Check for subnet relationships (if broker is VPC-deployed)
        if let Some(subnet_ids) = entry.raw_properties.get("SubnetIds") {
            if let Some(subnet_array) = subnet_ids.as_array() {
                for subnet_id_value in subnet_array {
                    if let Some(subnet_id) = subnet_id_value.as_str() {
                        // Find matching subnet in all_resources
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::Subnet"
                                && resource.resource_id == subnet_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::DeployedIn,
                                    target_resource_type: "AWS::EC2::Subnet".to_string(),
                                    target_resource_id: subnet_id.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Check for security group relationships
        if let Some(security_groups) = entry.raw_properties.get("SecurityGroups") {
            if let Some(sg_array) = security_groups.as_array() {
                for sg_value in sg_array {
                    if let Some(sg_id) = sg_value.as_str() {
                        // Find matching security group in all_resources
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::SecurityGroup"
                                && resource.resource_id == sg_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::ProtectedBy,
                                    target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                                    target_resource_id: sg_id.to_string(),
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
        "AWS::AmazonMQ::Broker"
    }
}

