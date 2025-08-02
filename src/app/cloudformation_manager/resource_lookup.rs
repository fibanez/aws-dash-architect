use crate::app::resource_explorer::{
    AWSResourceClient, AccountSelection, QueryScope, RegionSelection, ResourceEntry,
    ResourceTypeSelection,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// AWS resource information for parameter selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsResourceInfo {
    pub id: String,
    pub name: Option<String>,
    pub resource_type: String,
    pub arn: Option<String>,
    pub region: String,
    pub account_id: String,
    pub tags: HashMap<String, String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

/// Resource cache entry with TTL
#[derive(Debug, Clone)]
struct CachedResources {
    resources: Vec<AwsResourceInfo>,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedResources {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// AWS resource lookup service for CloudFormation parameter selection
/// This service delegates to the existing AWS Explorer infrastructure
pub struct ResourceLookupService {
    aws_client: Arc<AWSResourceClient>,
    cache: Arc<RwLock<HashMap<String, CachedResources>>>,
    cache_ttl: Duration,
}

impl ResourceLookupService {
    pub fn new(aws_client: Arc<AWSResourceClient>) -> Self {
        Self {
            aws_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get AWS resources for a specific CloudFormation parameter type
    pub async fn get_resources_for_parameter_type(
        &self,
        parameter_type: &str,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<AwsResourceInfo>> {
        let aws_resource_type = self.map_parameter_type_to_resource_type(parameter_type)?;

        info!(
            "Looking up AWS resources of type {} for parameter type {} in account {} region {}",
            aws_resource_type, parameter_type, account_id, region
        );

        // Check cache first
        let cache_key = format!(
            "{}:{}:{}:{}",
            aws_resource_type, account_id, region, parameter_type
        );
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if !cached.is_expired() {
                    debug!("Returning cached resources for {}", cache_key);
                    return Ok(cached.resources.clone());
                }
            }
        }

        // Get fresh data from AWS
        let resources = self
            .fetch_resources(&aws_resource_type, account_id, region)
            .await?;

        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedResources {
                    resources: resources.clone(),
                    cached_at: Instant::now(),
                    ttl: self.cache_ttl,
                },
            );
        }

        Ok(resources)
    }

    /// Search resources by name or ID with fuzzy matching
    pub async fn search_resources(
        &self,
        parameter_type: &str,
        account_id: &str,
        region: &str,
        query: &str,
    ) -> Result<Vec<AwsResourceInfo>> {
        let all_resources = self
            .get_resources_for_parameter_type(parameter_type, account_id, region)
            .await?;

        if query.is_empty() {
            return Ok(all_resources);
        }

        let query_lower = query.to_lowercase();
        let all_count = all_resources.len();
        let filtered: Vec<AwsResourceInfo> = all_resources
            .into_iter()
            .filter(|resource| {
                // Match on ID, name, or tags
                resource.id.to_lowercase().contains(&query_lower)
                    || resource
                        .name
                        .as_ref()
                        .map(|name| name.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || resource
                        .tags
                        .values()
                        .any(|tag_value| tag_value.to_lowercase().contains(&query_lower))
            })
            .collect();

        debug!(
            "Filtered {} resources to {} matches for query '{}'",
            all_count,
            filtered.len(),
            query
        );

        Ok(filtered)
    }

    /// Get all supported CloudFormation parameter types that can use resource lookup
    pub fn get_supported_parameter_types(&self) -> Vec<String> {
        vec![
            "AWS::EC2::VPC::Id".to_string(),
            "AWS::EC2::Subnet::Id".to_string(),
            "AWS::EC2::SecurityGroup::Id".to_string(),
            "AWS::EC2::KeyPair::KeyName".to_string(),
            "AWS::EC2::Instance::Id".to_string(),
            "AWS::RDS::DBSubnetGroup::Name".to_string(),
            "AWS::RDS::DBClusterParameterGroup::Name".to_string(),
            "AWS::S3::Bucket::Name".to_string(),
            "AWS::IAM::Role::Arn".to_string(),
            "AWS::Lambda::Function::Name".to_string(),
            "AWS::ECS::Cluster::Name".to_string(),
            "AWS::EKS::Cluster::Name".to_string(),
            "AWS::Route53::HostedZone::Id".to_string(),
        ]
    }

    /// Check if a parameter type supports resource lookup
    pub fn supports_resource_lookup(&self, parameter_type: &str) -> bool {
        self.get_supported_parameter_types()
            .contains(&parameter_type.to_string())
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.cache.write().await;
        let initial_count = cache.len();

        cache.retain(|_, cached| !cached.is_expired());

        let cleaned_count = initial_count - cache.len();
        if cleaned_count > 0 {
            debug!("Cleaned up {} expired cache entries", cleaned_count);
        }
    }

    /// Map CloudFormation parameter type to AWS resource type
    fn map_parameter_type_to_resource_type(&self, parameter_type: &str) -> Result<String> {
        let mapping = match parameter_type {
            "AWS::EC2::VPC::Id" => "AWS::EC2::VPC",
            "AWS::EC2::Subnet::Id" => "AWS::EC2::Subnet",
            "AWS::EC2::SecurityGroup::Id" => "AWS::EC2::SecurityGroup",
            "AWS::EC2::KeyPair::KeyName" => "AWS::EC2::KeyPair",
            "AWS::EC2::Instance::Id" => "AWS::EC2::Instance",
            "AWS::RDS::DBSubnetGroup::Name" => "AWS::RDS::DBSubnetGroup",
            "AWS::RDS::DBClusterParameterGroup::Name" => "AWS::RDS::DBClusterParameterGroup",
            "AWS::S3::Bucket::Name" => "AWS::S3::Bucket",
            "AWS::IAM::Role::Arn" => "AWS::IAM::Role",
            "AWS::Lambda::Function::Name" => "AWS::Lambda::Function",
            "AWS::ECS::Cluster::Name" => "AWS::ECS::Cluster",
            "AWS::EKS::Cluster::Name" => "AWS::EKS::Cluster",
            "AWS::Route53::HostedZone::Id" => "AWS::Route53::HostedZone",
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported parameter type for resource lookup: {}",
                    parameter_type
                ));
            }
        };

        Ok(mapping.to_string())
    }

    /// Fetch resources from AWS using the existing AWS Explorer infrastructure
    async fn fetch_resources(
        &self,
        resource_type: &str,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<AwsResourceInfo>> {
        // Create query scope for the specific resource type, account, and region
        let query_scope = QueryScope {
            accounts: vec![AccountSelection {
                account_id: account_id.to_string(),
                display_name: format!("Account {}", account_id),
                color: egui::Color32::from_rgb(100, 150, 200), // Default color
            }],
            regions: vec![RegionSelection {
                region_code: region.to_string(),
                display_name: region.to_string(),
                color: egui::Color32::from_rgb(150, 100, 200), // Default color
            }],
            resource_types: vec![ResourceTypeSelection {
                resource_type: resource_type.to_string(),
                display_name: resource_type.to_string(),
                service_name: resource_type
                    .split("::")
                    .nth(1)
                    .unwrap_or("Unknown")
                    .to_string(),
            }],
        };

        // Query using the existing AWS Explorer infrastructure
        let mut cache = HashMap::new();
        let resource_entries = self
            .aws_client
            .query_aws_resources(&query_scope, None, &mut cache)
            .await?;

        // Convert ResourceEntry objects to AwsResourceInfo for CloudFormation parameter selection
        let mut resources = Vec::new();
        for entry in resource_entries {
            if entry.resource_type == resource_type
                && entry.account_id == account_id
                && entry.region == region
            {
                let aws_resource_info = self.convert_resource_entry_to_info(entry);
                resources.push(aws_resource_info);
            }
        }

        info!(
            "Found {} resources of type {} in account {} region {}",
            resources.len(),
            resource_type,
            account_id,
            region
        );

        Ok(resources)
    }

    /// Convert a ResourceEntry from AWS Explorer to AwsResourceInfo for parameter selection
    fn convert_resource_entry_to_info(&self, entry: ResourceEntry) -> AwsResourceInfo {
        // Extract name from display_name or properties
        let name = if !entry.display_name.is_empty() && entry.display_name != entry.resource_id {
            Some(entry.display_name)
        } else {
            // Try to extract name from properties
            entry
                .properties
                .get("Name")
                .or_else(|| entry.properties.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        // Extract ARN from properties if available
        let arn = entry
            .properties
            .get("Arn")
            .or_else(|| entry.properties.get("arn"))
            .or_else(|| entry.properties.get("ResourceArn"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract description from properties
        let description = entry
            .properties
            .get("Description")
            .or_else(|| entry.properties.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Convert tags from ResourceTag to HashMap
        let tags: HashMap<String, String> = entry
            .tags
            .into_iter()
            .map(|tag| (tag.key, tag.value))
            .collect();

        AwsResourceInfo {
            id: entry.resource_id,
            name,
            resource_type: entry.resource_type,
            arn,
            region: entry.region,
            account_id: entry.account_id,
            tags,
            description,
            status: entry.status,
        }
    }
}
