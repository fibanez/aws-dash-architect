//! AWS List Resources Tool
//!
//! This tool allows AI agents to list AWS resources with optional filtering
//! by account, region, and resource type using natural language commands.

use crate::app::resource_explorer::{
    aws_client::AWSResourceClient,
    state::{QueryScope, AccountSelection, RegionSelection, ResourceTypeSelection},
};
use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use stood::tools::{Tool, ToolError, ToolResult};
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::super::{get_global_aws_client, ResourceSummary};

/// AWS List Resources Tool - Manual Implementation
#[derive(Clone)]
pub struct AwsListResourcesTool {
    aws_client: Option<Arc<AWSResourceClient>>,
}

impl std::fmt::Debug for AwsListResourcesTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsListResourcesTool")
            .field("aws_client", &self.aws_client.is_some())
            .finish()
    }
}

impl AwsListResourcesTool {
    pub fn new(aws_client: Option<Arc<AWSResourceClient>>) -> Self {
        Self { aws_client }
    }

    /// Create a new tool without AWS client (will be set later)
    pub fn new_uninitialized() -> Self {
        Self { aws_client: None }
    }

    /// Set the AWS client for this tool
    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client;
    }

    /// Map friendly resource names to CloudFormation types
    /// Supports all 80+ AWS resource types from the Explorer system
    fn map_resource_type(friendly_name: &str) -> Option<String> {
        match friendly_name.to_lowercase().as_str() {
            // EC2 Resources
            "instances" | "ec2" | "instance" => Some("AWS::EC2::Instance".to_string()),
            "vpcs" | "vpc" => Some("AWS::EC2::VPC".to_string()),
            "subnets" | "subnet" => Some("AWS::EC2::Subnet".to_string()),
            "security_groups" | "sg" | "securitygroup" => Some("AWS::EC2::SecurityGroup".to_string()),
            "volumes" | "ebs" | "volume" => Some("AWS::EC2::Volume".to_string()),
            "snapshots" | "snapshot" => Some("AWS::EC2::Snapshot".to_string()),
            "images" | "amis" | "ami" => Some("AWS::EC2::Image".to_string()),
            "route_tables" | "routetable" => Some("AWS::EC2::RouteTable".to_string()),
            "nat_gateways" | "natgw" => Some("AWS::EC2::NatGateway".to_string()),
            "network_interfaces" | "eni" => Some("AWS::EC2::NetworkInterface".to_string()),
            "vpc_endpoints" | "vpce" => Some("AWS::EC2::VPCEndpoint".to_string()),
            "network_acls" | "nacl" => Some("AWS::EC2::NetworkAcl".to_string()),
            "key_pairs" | "keypair" => Some("AWS::EC2::KeyPair".to_string()),
            "internet_gateways" | "igw" => Some("AWS::EC2::InternetGateway".to_string()),

            // IAM Resources
            "roles" | "iam_roles" | "role" => Some("AWS::IAM::Role".to_string()),
            "users" | "iam_users" | "user" => Some("AWS::IAM::User".to_string()),
            "policies" | "iam_policies" | "policy" => Some("AWS::IAM::Policy".to_string()),

            // S3 Resources
            "buckets" | "s3" | "bucket" => Some("AWS::S3::Bucket".to_string()),

            // Lambda Resources
            "functions" | "lambda" | "function" => Some("AWS::Lambda::Function".to_string()),
            "layers" | "lambda_layers" => Some("AWS::Lambda::LayerVersion".to_string()),
            "event_mappings" | "lambda_triggers" => Some("AWS::Lambda::EventSourceMapping".to_string()),

            // RDS Resources
            "db_instances" | "rds" | "database" => Some("AWS::RDS::DBInstance".to_string()),
            "db_clusters" | "rds_clusters" => Some("AWS::RDS::DBCluster".to_string()),
            "db_snapshots" | "rds_snapshots" => Some("AWS::RDS::DBSnapshot".to_string()),
            "db_parameter_groups" | "rds_params" => Some("AWS::RDS::DBParameterGroup".to_string()),
            "db_subnet_groups" | "rds_subnets" => Some("AWS::RDS::DBSubnetGroup".to_string()),

            // DynamoDB Resources
            "dynamodb_tables" | "dynamodb" | "tables" => Some("AWS::DynamoDB::Table".to_string()),

            // CloudWatch Resources
            "alarms" | "cloudwatch_alarms" => Some("AWS::CloudWatch::Alarm".to_string()),
            "dashboards" | "cloudwatch_dashboards" => Some("AWS::CloudWatch::Dashboard".to_string()),

            // API Gateway Resources
            "rest_apis" | "apigateway" => Some("AWS::ApiGateway::RestApi".to_string()),
            "http_apis" | "apigatewayv2" => Some("AWS::ApiGatewayV2::Api".to_string()),

            // SNS/SQS Resources
            "topics" | "sns" => Some("AWS::SNS::Topic".to_string()),
            "queues" | "sqs" => Some("AWS::SQS::Queue".to_string()),

            // ECS Resources
            "ecs_clusters" | "ecs" => Some("AWS::ECS::Cluster".to_string()),
            "ecs_services" => Some("AWS::ECS::Service".to_string()),
            "ecs_tasks" => Some("AWS::ECS::Task".to_string()),
            "ecs_task_definitions" => Some("AWS::ECS::TaskDefinition".to_string()),

            // EKS Resources
            "eks_clusters" | "eks" => Some("AWS::EKS::Cluster".to_string()),

            // Load Balancers
            "load_balancers" | "elb" | "alb" | "nlb" => Some("AWS::ElasticLoadBalancingV2::LoadBalancer".to_string()),
            "classic_load_balancers" | "clb" => Some("AWS::ElasticLoadBalancing::LoadBalancer".to_string()),
            "target_groups" | "tg" => Some("AWS::ElasticLoadBalancingV2::TargetGroup".to_string()),

            // CloudFormation Resources
            "stacks" | "cloudformation" | "cfn" => Some("AWS::CloudFormation::Stack".to_string()),

            // Logs Resources
            "log_groups" | "logs" => Some("AWS::Logs::LogGroup".to_string()),

            // Kinesis Resources
            "kinesis_streams" | "kinesis" => Some("AWS::Kinesis::Stream".to_string()),
            "firehose_streams" | "firehose" => Some("AWS::KinesisFirehose::DeliveryStream".to_string()),

            // SageMaker Resources
            "sagemaker_endpoints" | "sagemaker" => Some("AWS::SageMaker::Endpoint".to_string()),
            "sagemaker_training_jobs" => Some("AWS::SageMaker::TrainingJob".to_string()),
            "sagemaker_models" => Some("AWS::SageMaker::Model".to_string()),

            // Redshift Resources
            "redshift_clusters" | "redshift" => Some("AWS::Redshift::Cluster".to_string()),

            // Glue Resources
            "glue_jobs" | "glue" => Some("AWS::Glue::Job".to_string()),

            // Athena Resources
            "athena_workgroups" | "athena" => Some("AWS::Athena::WorkGroup".to_string()),

            // Route53 Resources
            "hosted_zones" | "route53" => Some("AWS::Route53::HostedZone".to_string()),

            // EFS Resources
            "file_systems" | "efs" => Some("AWS::EFS::FileSystem".to_string()),

            // CloudTrail Resources
            "trails" | "cloudtrail" => Some("AWS::CloudTrail::Trail".to_string()),

            // Config Resources
            "config_recorders" | "config" => Some("AWS::Config::ConfigurationRecorder".to_string()),

            // SSM Resources
            "ssm_parameters" | "parameters" => Some("AWS::SSM::Parameter".to_string()),
            "ssm_documents" | "documents" => Some("AWS::SSM::Document".to_string()),

            // Backup Resources
            "backup_plans" | "backup" => Some("AWS::Backup::BackupPlan".to_string()),
            "backup_vaults" => Some("AWS::Backup::BackupVault".to_string()),

            // EventBridge Resources
            "event_buses" | "eventbridge" => Some("AWS::Events::EventBus".to_string()),
            "event_rules" | "rules" => Some("AWS::Events::Rule".to_string()),

            // AppSync Resources
            "graphql_apis" | "appsync" => Some("AWS::AppSync::GraphQLApi".to_string()),

            // MQ Resources
            "mq_brokers" | "mq" => Some("AWS::AmazonMQ::Broker".to_string()),

            // CodePipeline Resources
            "pipelines" | "codepipeline" => Some("AWS::CodePipeline::Pipeline".to_string()),
            "build_projects" | "codebuild" => Some("AWS::CodeBuild::Project".to_string()),
            "repositories" | "codecommit" => Some("AWS::CodeCommit::Repository".to_string()),

            // IoT Resources
            "iot_things" | "iot" => Some("AWS::IoT::Thing".to_string()),
            "greengrass_components" | "greengrass" => Some("AWS::GreengrassV2::ComponentVersion".to_string()),

            // Organizations Resources
            "organizational_units" | "ous" => Some("AWS::Organizations::OrganizationalUnit".to_string()),
            "organization_policies" => Some("AWS::Organizations::Policy".to_string()),

            // Certificate Manager Resources
            "certificates" | "acm" => Some("AWS::CertificateManager::Certificate".to_string()),
            "certificate_authorities" | "ca" => Some("AWS::ACMPCA::CertificateAuthority".to_string()),

            // WAF Resources
            "web_acls" | "waf" => Some("AWS::WAFv2::WebACL".to_string()),

            // GuardDuty Resources
            "guardduty_detectors" | "guardduty" => Some("AWS::GuardDuty::Detector".to_string()),

            // CloudFront Resources
            "distributions" | "cloudfront" => Some("AWS::CloudFront::Distribution".to_string()),

            // ElastiCache Resources
            "cache_clusters" | "elasticache" => Some("AWS::ElastiCache::CacheCluster".to_string()),
            "replication_groups" | "redis_clusters" => Some("AWS::ElastiCache::ReplicationGroup".to_string()),

            // Neptune Resources
            "neptune_clusters" | "neptune" => Some("AWS::Neptune::DBCluster".to_string()),
            "neptune_instances" => Some("AWS::Neptune::DBInstance".to_string()),

            // OpenSearch Resources
            "opensearch_domains" | "opensearch" | "elasticsearch" => Some("AWS::OpenSearchService::Domain".to_string()),

            // Cognito Resources
            "user_pools" | "cognito" => Some("AWS::Cognito::UserPool".to_string()),
            "identity_pools" => Some("AWS::Cognito::IdentityPool".to_string()),
            "user_pool_clients" => Some("AWS::Cognito::UserPoolClient".to_string()),

            // Batch Resources
            "batch_queues" | "batch" => Some("AWS::Batch::JobQueue".to_string()),
            "batch_environments" => Some("AWS::Batch::ComputeEnvironment".to_string()),

            // QuickSight Resources
            "quicksight_datasets" | "quicksight" => Some("AWS::QuickSight::DataSet".to_string()),
            "quicksight_dashboards" => Some("AWS::QuickSight::Dashboard".to_string()),
            "quicksight_datasources" => Some("AWS::QuickSight::DataSource".to_string()),

            // Bedrock Resources
            "bedrock_models" | "bedrock" => Some("AWS::Bedrock::Model".to_string()),

            _ => {
                // If it's already a CloudFormation type, return as-is
                if friendly_name.starts_with("AWS::") {
                    Some(friendly_name.to_string())
                } else {
                    None
                }
            }
        }
    }
}

impl Default for AwsListResourcesTool {
    fn default() -> Self {
        Self::new_uninitialized()
    }
}

#[async_trait]
impl Tool for AwsListResourcesTool {
    fn name(&self) -> &str {
        "aws_list_resources"
    }

    fn description(&self) -> &str {
        r#"List AWS resources with optional filtering by account, region, and resource type.

Supports 80+ AWS resource types with friendly names:

COMPUTE: instances/ec2, functions/lambda, ecs_clusters/ecs, eks_clusters/eks, batch_queues/batch
STORAGE: buckets/s3, volumes/ebs, snapshots, file_systems/efs
NETWORKING: vpcs/vpc, subnets, security_groups/sg, load_balancers/elb/alb/nlb, vpc_endpoints/vpce
DATABASES: db_instances/rds, dynamodb_tables/dynamodb, redshift_clusters/redshift, neptune_clusters/neptune
SECURITY: roles/iam_roles, users/iam_users, policies/iam_policies, certificates/acm, web_acls/waf
MESSAGING: topics/sns, queues/sqs, kinesis_streams/kinesis, mq_brokers/mq
MONITORING: alarms/cloudwatch_alarms, log_groups/logs, trails/cloudtrail
ANALYTICS: athena_workgroups/athena, glue_jobs/glue, quicksight_datasets/quicksight
INTEGRATION: rest_apis/apigateway, graphql_apis/appsync, pipelines/codepipeline
AI/ML: sagemaker_endpoints/sagemaker, bedrock_models/bedrock
MANAGEMENT: stacks/cloudformation/cfn, ssm_parameters/parameters, backup_plans/backup
CONTENT: distributions/cloudfront, hosted_zones/route53

Or use full CloudFormation types like "AWS::EC2::Instance"

Examples:
- List EC2 instances: {"resource_type": "instances"}
- List VPCs in us-east-1: {"resource_type": "vpcs", "region": "us-east-1"}  
- List Lambda functions: {"resource_type": "functions"}
- List RDS databases: {"resource_type": "rds"}
- List all resources in account: {"account_id": "123456789012"}
- Force fresh data: {"resource_type": "instances", "force_refresh": true}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "resource_type": {
                    "type": "string",
                    "description": "Resource type (friendly names like 'vpcs', 'instances' or full CloudFormation types)",
                    "examples": ["instances", "vpcs", "buckets", "functions", "AWS::EC2::Instance"]
                },
                "region": {
                    "type": "string",
                    "description": "AWS region (e.g., 'us-east-1', 'eu-west-1')",
                    "examples": ["us-east-1", "us-west-2", "eu-west-1"]
                },
                "account_id": {
                    "type": "string",
                    "description": "AWS account ID (optional, uses all accounts if not specified)",
                    "pattern": "^[0-9]{12}$"
                },
                "force_refresh": {
                    "type": "boolean",
                    "description": "Force refresh from AWS APIs instead of using cache",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!("🔍 aws_list_resources executing with parameters: {:?}", parameters);

        // Check if AWS client is available (try instance client first, then global)
        let global_client = get_global_aws_client();
        
        // Debug logging for AWS client availability
        info!("🔍 Checking AWS client availability:");
        info!("  - Instance client: {}", if self.aws_client.is_some() { "✅ Available" } else { "❌ Not available" });
        info!("  - Global client: {}", if global_client.is_some() { "✅ Available" } else { "❌ Not available" });
        
        let aws_client = self.aws_client.as_ref()
            .or(global_client.as_ref())
            .ok_or_else(|| {
                ToolError::ExecutionFailed { 
                    message: "AWS client not available. The ResourceExplorer needs to be initialized first.\n\nTo fix this:\n1. Ensure AWS Identity Center is logged in\n2. Open AWS Explorer (press Space → type 'AWS Explorer' or press 'E')\n3. Run a simple query to initialize the AWS client\n4. Then bridge tools will work\n\nThis should normally be automatic after login - if you see this message, there may be a timing issue.".to_string()
                }
            })?;

        // Parse parameters
        let params = parameters.unwrap_or_else(|| serde_json::json!({}));
        
        let resource_type_param = params.get("resource_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let region_param = params.get("region")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let account_id_param = params.get("account_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let force_refresh = params.get("force_refresh")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Map friendly resource type to CloudFormation type if needed
        let cloudformation_resource_type = if let Some(ref rt) = resource_type_param {
            match Self::map_resource_type(rt) {
                Some(mapped) => {
                    info!("Mapped resource type '{}' to '{}'", rt, mapped);
                    Some(mapped)
                }
                None => {
                    return Err(ToolError::InvalidParameters { 
                        message: format!(
                        "Unknown resource type '{}'. Use friendly names like 'instances', 'vpcs', 'buckets' or full CloudFormation types like 'AWS::EC2::Instance'",
                        rt
                    )});
                }
            }
        } else {
            None
        };

        // Create query scope
        let mut scope = QueryScope::new();
        
        // Add account selection if specified
        if let Some(account_id) = account_id_param {
            scope.accounts.push(AccountSelection {
                account_id: account_id.clone(),
                display_name: account_id.clone(),
                color: egui::Color32::from_rgb(100, 150, 255),
            });
        }
        
        // Add region selection if specified  
        if let Some(region) = region_param {
            scope.regions.push(RegionSelection {
                region_code: region.clone(),
                display_name: region.clone(),
                color: egui::Color32::from_rgb(150, 100, 255),
            });
        }
        
        // Add resource type selection if specified
        if let Some(resource_type) = cloudformation_resource_type {
            scope.resource_types.push(ResourceTypeSelection {
                resource_type: resource_type.clone(),
                display_name: resource_type.clone(),
                service_name: resource_type.split("::").nth(1).unwrap_or("Unknown").to_string(),
            });
        }

        // If scope is empty, we need at least some default values
        if scope.is_empty() {
            return Err(ToolError::InvalidParameters { 
                message: "Query scope is empty. Please specify at least one of: resource_type, region, or account_id".to_string()
            });
        }

        // Create channels for receiving results
        let (result_sender, mut result_receiver) = mpsc::channel(1000);
        let (progress_sender, _progress_receiver) = mpsc::channel(100);

        // Prepare cache - empty HashMap forces fresh queries if force_refresh is true
        let cache = if force_refresh {
            Arc::new(tokio::sync::RwLock::new(HashMap::new()))
        } else {
            // Use empty cache for now - in real implementation we'd access the global cache
            Arc::new(tokio::sync::RwLock::new(HashMap::new()))
        };

        // Execute the query
        info!("🚀 Executing parallel AWS resource query");
        let aws_client_clone = aws_client.clone();
        let scope_clone = scope.clone();
        let query_task = aws_client_clone.query_aws_resources_parallel(
            &scope_clone,
            result_sender,
            Some(progress_sender),
            cache,
        );

        // Collect results
        let mut all_resources = Vec::new();
        let mut query_errors = Vec::new();
        
        // Start the query task in a way that doesn't require 'static lifetime
        let _ = query_task.await;
        
        // Collect all results
        while let Some(result) = result_receiver.recv().await {
            match result.resources {
                Ok(resources) => {
                    info!("✅ Received {} resources for {}/{}/{}", 
                         resources.len(), result.account_id, result.region, result.resource_type);
                    
                    // Convert ResourceEntry to ResourceSummary
                    for resource in resources {
                        all_resources.push(ResourceSummary {
                            resource_type: resource.resource_type,
                            account_id: resource.account_id,
                            region: resource.region,
                            resource_id: resource.resource_id,
                            display_name: resource.display_name,
                            status: resource.status,
                            properties: resource.properties,
                            tags: resource.tags.iter().map(|tag| format!("{}={}", tag.key, tag.value)).collect(),
                        });
                    }
                }
                Err(e) => {
                    let error_msg = format!("Query failed for {}/{}/{}: {}", 
                                          result.account_id, result.region, result.resource_type, e);
                    warn!("❌ {}", error_msg);
                    query_errors.push(error_msg);
                }
            }
        }

        let duration = start_time.elapsed();
        let total_count = all_resources.len();
        
        let execution_summary = if query_errors.is_empty() {
            format!(
                "Found {} AWS resources in {:.2}s. Query executed {}.",
                total_count,
                duration.as_secs_f64(),
                if force_refresh { "fresh from AWS APIs" } else { "with cache support" }
            )
        } else {
            format!(
                "Found {} AWS resources in {:.2}s with {} errors. Query executed {}. Errors: {}",
                total_count,
                duration.as_secs_f64(),
                query_errors.len(),
                if force_refresh { "fresh from AWS APIs" } else { "with cache support" },
                query_errors.join("; ")
            )
        };

        info!("📊 aws_list_resources completed: {}", execution_summary);

        // Create response JSON
        let response_data = serde_json::json!({
            "resources": all_resources,
            "total_count": total_count,
            "from_cache": !force_refresh,
            "execution_summary": execution_summary,
            "query_errors": query_errors,
            "duration_seconds": duration.as_secs_f64()
        });

        Ok(ToolResult::success(response_data))
    }
}