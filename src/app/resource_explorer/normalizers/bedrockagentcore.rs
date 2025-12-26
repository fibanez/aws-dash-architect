use super::*;
use crate::app::resource_explorer::normalizers::utils::create_normalized_properties;
use crate::app::resource_explorer::state::{RelationshipType, ResourceEntry, ResourceRelationship};
use crate::app::resource_explorer::{assign_account_color, assign_region_color};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

// ==================== Agent Runtime ====================

pub struct BedrockAgentCoreAgentRuntimeNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreAgentRuntimeNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let runtime_id = raw_response
            .get("AgentRuntimeId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-runtime")
            .to_string();

        let display_name = raw_response
            .get("AgentRuntimeName")
            .and_then(|v| v.as_str())
            .unwrap_or(&runtime_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::AgentRuntime",
                &runtime_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::AgentRuntime {}: {}",
                    runtime_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::AgentRuntime".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: runtime_id,
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

        // Agent Runtime may use Memory resources
        for resource in all_resources {
            if resource.resource_type == "AWS::BedrockAgentCore::Memory" {
                // Check if memory is referenced in runtime config
                if let Some(memory_arn) = entry.raw_properties.get("MemoryArn") {
                    if let Some(memory_arn_str) = memory_arn.as_str() {
                        if resource
                            .raw_properties
                            .get("MemoryArn")
                            .and_then(|v| v.as_str())
                            == Some(memory_arn_str)
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

            // Agent Runtime may use credential providers
            if resource.resource_type == "AWS::BedrockAgentCore::ApiKeyCredentialProvider"
                || resource.resource_type == "AWS::BedrockAgentCore::OAuth2CredentialProvider"
            {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: resource.resource_id.clone(),
                    target_resource_type: resource.resource_type.clone(),
                });
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::AgentRuntime"
    }
}

// ==================== Agent Runtime Endpoint ====================

pub struct BedrockAgentCoreAgentRuntimeEndpointNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreAgentRuntimeEndpointNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let endpoint_id = raw_response
            .get("EndpointId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-endpoint")
            .to_string();

        let display_name = raw_response
            .get("EndpointName")
            .and_then(|v| v.as_str())
            .unwrap_or(&endpoint_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::AgentRuntimeEndpoint",
                &endpoint_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::AgentRuntimeEndpoint {}: {}",
                    endpoint_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::AgentRuntimeEndpoint".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: endpoint_id,
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

        // Endpoints belong to Agent Runtimes
        if let Some(runtime_id) = entry.raw_properties.get("AgentRuntimeId") {
            if let Some(runtime_id_str) = runtime_id.as_str() {
                for resource in all_resources {
                    if resource.resource_type == "AWS::BedrockAgentCore::AgentRuntime"
                        && resource.resource_id == runtime_id_str
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::AttachedTo,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::AgentRuntimeEndpoint"
    }
}

// ==================== Memory ====================

pub struct BedrockAgentCoreMemoryNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreMemoryNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let memory_id = raw_response
            .get("MemoryId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-memory")
            .to_string();

        let display_name = raw_response
            .get("MemoryName")
            .and_then(|v| v.as_str())
            .unwrap_or(&memory_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::BedrockAgentCore::Memory", &memory_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::Memory {}: {}",
                    memory_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::Memory".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: memory_id,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Memory is used by Agent Runtimes (reverse relationship handled in AgentRuntime normalizer)
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::Memory"
    }
}

// ==================== Gateway ====================

pub struct BedrockAgentCoreGatewayNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let gateway_id = raw_response
            .get("GatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-gateway")
            .to_string();

        let display_name = raw_response
            .get("GatewayName")
            .and_then(|v| v.as_str())
            .unwrap_or(&gateway_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::Gateway",
                &gateway_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::Gateway {}: {}",
                    gateway_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::Gateway".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: gateway_id,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Gateways may have targets, but those will be handled separately
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::Gateway"
    }
}

// ==================== Browser ====================

pub struct BedrockAgentCoreBrowserNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreBrowserNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let browser_id = raw_response
            .get("BrowserId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-browser")
            .to_string();

        let display_name = raw_response
            .get("BrowserName")
            .and_then(|v| v.as_str())
            .unwrap_or(&browser_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::Browser",
                &browser_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::Browser {}: {}",
                    browser_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::Browser".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: browser_id,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Browser sessions would be child resources
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::Browser"
    }
}

// ==================== Code Interpreter ====================

pub struct BedrockAgentCoreCodeInterpreterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreCodeInterpreterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let interpreter_id = raw_response
            .get("CodeInterpreterId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-interpreter")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&interpreter_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::CodeInterpreter",
                &interpreter_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::CodeInterpreter {}: {}",
                    interpreter_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::CodeInterpreter".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: interpreter_id,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Code Interpreters are used by Agent Runtimes (reverse relationship handled elsewhere)
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::CodeInterpreter"
    }
}

// ==================== API Key Credential Provider ====================

pub struct BedrockAgentCoreApiKeyCredentialProviderNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreApiKeyCredentialProviderNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-provider")
            .to_string();

        let display_name = name.clone();

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::BedrockAgentCore::ApiKeyCredentialProvider", &name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::BedrockAgentCore::ApiKeyCredentialProvider {}: {}", name, e);

                Vec::new()

            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::ApiKeyCredentialProvider".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name,
            display_name,
            status: None,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Used by Agent Runtimes (reverse relationship)
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::ApiKeyCredentialProvider"
    }
}

// ==================== OAuth2 Credential Provider ====================

pub struct BedrockAgentCoreOAuth2CredentialProviderNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreOAuth2CredentialProviderNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-provider")
            .to_string();

        let display_name = name.clone();

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::BedrockAgentCore::OAuth2CredentialProvider", &name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::BedrockAgentCore::OAuth2CredentialProvider {}: {}", name, e);

                Vec::new()

            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::OAuth2CredentialProvider".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name,
            display_name,
            status: None,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Used by Agent Runtimes (reverse relationship)
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::OAuth2CredentialProvider"
    }
}

// ==================== Workload Identity ====================

pub struct BedrockAgentCoreWorkloadIdentityNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreWorkloadIdentityNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-identity")
            .to_string();

        let display_name = name.clone();

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::WorkloadIdentity",
                &name,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::WorkloadIdentity {}: {}",
                    name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::WorkloadIdentity".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name,
            display_name,
            status: None,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Workload identities may be associated with OAuth2 providers
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::WorkloadIdentity"
    }
}

// ==================== Agent Runtime Version (Child Resource) ====================

pub struct BedrockAgentCoreAgentRuntimeVersionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreAgentRuntimeVersionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let runtime_id = raw_response
            .get("AgentRuntimeId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-runtime")
            .to_string();

        let version = raw_response
            .get("AgentRuntimeVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-version")
            .to_string();

        let display_name = format!(
            "{} (v{})",
            raw_response
                .get("AgentRuntimeName")
                .and_then(|v| v.as_str())
                .unwrap_or(&runtime_id),
            version
        );

        let parent_runtime_id = raw_response
            .get("ParentRuntimeId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::AgentRuntimeVersion",
                &runtime_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::AgentRuntimeVersion {}: {}",
                    runtime_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::AgentRuntimeVersion".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: format!(
                "{}#{}",
                parent_runtime_id.as_ref().unwrap_or(&"unknown".to_string()),
                version
            ),
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: parent_runtime_id.clone(),
            parent_resource_type: Some("AWS::BedrockAgentCore::AgentRuntime".to_string()),
            is_child_resource: true,
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

        // Link to parent runtime
        if let Some(parent_id) = &entry.parent_resource_id {
            for resource in all_resources {
                if resource.resource_type == "AWS::BedrockAgentCore::AgentRuntime"
                    && &resource.resource_id == parent_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::ChildOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::AgentRuntimeVersion"
    }
}

// ==================== Gateway Target (Child Resource) ====================

pub struct BedrockAgentCoreGatewayTargetNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreGatewayTargetNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let target_id = raw_response
            .get("TargetId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-target")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&target_id)
            .to_string();

        let parent_gateway_id = raw_response
            .get("ParentGatewayId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::GatewayTarget",
                &target_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::GatewayTarget {}: {}",
                    target_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::GatewayTarget".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: format!(
                "{}#{}",
                parent_gateway_id.as_ref().unwrap_or(&"unknown".to_string()),
                target_id
            ),
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: parent_gateway_id.clone(),
            parent_resource_type: Some("AWS::BedrockAgentCore::Gateway".to_string()),
            is_child_resource: true,
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

        // Link to parent gateway
        if let Some(parent_id) = &entry.parent_resource_id {
            for resource in all_resources {
                if resource.resource_type == "AWS::BedrockAgentCore::Gateway"
                    && &resource.resource_id == parent_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::ChildOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::GatewayTarget"
    }
}

// ==================== Data Plane Resources ====================

// ==================== Memory Record (Child Resource) ====================

pub struct BedrockAgentCoreMemoryRecordNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreMemoryRecordNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let record_id = raw_response
            .get("MemoryRecordId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-record")
            .to_string();

        let parent_memory_id = raw_response
            .get("ParentMemoryId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let display_name = format!("Memory Record {}", &record_id[..record_id.len().min(8)]);

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::MemoryRecord",
                &record_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::MemoryRecord {}: {}",
                    record_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::MemoryRecord".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: format!(
                "{}#{}",
                parent_memory_id.as_ref().unwrap_or(&"unknown".to_string()),
                record_id
            ),
            display_name,
            status: None,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: parent_memory_id.clone(),
            parent_resource_type: Some("AWS::BedrockAgentCore::Memory".to_string()),
            is_child_resource: true,
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

        // Link to parent memory
        if let Some(parent_id) = &entry.parent_resource_id {
            for resource in all_resources {
                if resource.resource_type == "AWS::BedrockAgentCore::Memory"
                    && &resource.resource_id == parent_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::ChildOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::MemoryRecord"
    }
}

// ==================== Event (Child Resource) ====================

pub struct BedrockAgentCoreEventNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreEventNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let event_id = raw_response
            .get("EventId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-event")
            .to_string();

        let memory_id = raw_response
            .get("MemoryId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let session_id = raw_response
            .get("SessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-session");

        let display_name = format!(
            "Event {} (Session: {})",
            &event_id[..event_id.len().min(8)],
            &session_id[..session_id.len().min(8)]
        );

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::BedrockAgentCore::Event", &event_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::Event {}: {}",
                    event_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::Event".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: format!(
                "{}#{}",
                memory_id.as_ref().unwrap_or(&"unknown".to_string()),
                event_id
            ),
            display_name,
            status: None,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: memory_id.clone(),
            parent_resource_type: Some("AWS::BedrockAgentCore::Memory".to_string()),
            is_child_resource: true,
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

        // Link to parent memory
        if let Some(parent_id) = &entry.parent_resource_id {
            for resource in all_resources {
                if resource.resource_type == "AWS::BedrockAgentCore::Memory"
                    && &resource.resource_id == parent_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::ChildOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::Event"
    }
}

// ==================== Browser Session (Child Resource) ====================

pub struct BedrockAgentCoreBrowserSessionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreBrowserSessionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let session_id = raw_response
            .get("SessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-session")
            .to_string();

        let parent_browser_id = raw_response
            .get("ParentBrowserId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                format!("Browser Session {}", &session_id[..session_id.len().min(8)])
            });

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::BrowserSession",
                &session_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::BrowserSession {}: {}",
                    session_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::BrowserSession".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: format!(
                "{}#{}",
                parent_browser_id.as_ref().unwrap_or(&"unknown".to_string()),
                session_id
            ),
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: parent_browser_id.clone(),
            parent_resource_type: Some("AWS::BedrockAgentCore::Browser".to_string()),
            is_child_resource: true,
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

        // Link to parent browser
        if let Some(parent_id) = &entry.parent_resource_id {
            for resource in all_resources {
                if resource.resource_type == "AWS::BedrockAgentCore::Browser"
                    && &resource.resource_id == parent_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::ChildOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::BrowserSession"
    }
}

// ==================== Code Interpreter Session (Child Resource) ====================

pub struct BedrockAgentCoreCodeInterpreterSessionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentCoreCodeInterpreterSessionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let session_id = raw_response
            .get("SessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-session")
            .to_string();

        let parent_interpreter_id = raw_response
            .get("ParentCodeInterpreterId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                format!(
                    "Code Interpreter Session {}",
                    &session_id[..session_id.len().min(8)]
                )
            });

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = create_normalized_properties(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::BedrockAgentCore::CodeInterpreterSession",
                &session_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::BedrockAgentCore::CodeInterpreterSession {}: {}",
                    session_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::BedrockAgentCore::CodeInterpreterSession".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: format!(
                "{}#{}",
                parent_interpreter_id
                    .as_ref()
                    .unwrap_or(&"unknown".to_string()),
                session_id
            ),
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: parent_interpreter_id.clone(),
            parent_resource_type: Some("AWS::BedrockAgentCore::CodeInterpreter".to_string()),
            is_child_resource: true,
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

        // Link to parent code interpreter
        if let Some(parent_id) = &entry.parent_resource_id {
            for resource in all_resources {
                if resource.resource_type == "AWS::BedrockAgentCore::CodeInterpreter"
                    && &resource.resource_id == parent_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::ChildOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::BedrockAgentCore::CodeInterpreterSession"
    }
}
