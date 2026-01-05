use super::{
    aws_services::*, cache::SharedResourceCache, child_resources::*, credentials::*,
    global_services::*, normalizers::*, query_timing::*, retry_tracker::retry_tracker,
    sdk_errors::categorize_error_string, state::*, tag_cache::TagCache,
};
use anyhow::{Context, Result};
use chrono::Utc;
use futures::future::BoxFuture;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info, warn};

/// Configuration for API pagination
#[derive(Debug, Clone)]
pub struct PaginationConfig {
    pub page_size: i32,
    pub max_items: usize,
    pub max_concurrent_requests: usize,
}

/// Result from a single parallel query
#[derive(Debug)]
pub struct QueryResult {
    pub account_id: String,
    pub region: String,
    pub resource_type: String,
    pub resources: Result<Vec<ResourceEntry>>,
    pub cache_key: String,
}

impl Default for PaginationConfig {
    fn default() -> Self {
        Self {
            page_size: 50,               // Balance between performance and API limits
            max_items: 1000,             // Prevent runaway queries
            max_concurrent_requests: 20, // Reasonable concurrency limit
        }
    }
}

#[derive(Debug)]
pub struct QueryProgress {
    pub account: String,
    pub region: String,
    pub resource_type: String,
    pub status: QueryStatus,
    pub message: String,
    pub items_processed: Option<usize>, // For pagination progress
    pub estimated_total: Option<usize>, // For progress indication
}

#[derive(Debug)]
pub enum QueryStatus {
    Started,
    InProgress,
    Completed,
    Failed,
    // Tag fetching status (Phase 1 - during normalization)
    FetchingTags,
    // Phase 2 enrichment statuses
    EnrichmentStarted,
    EnrichmentInProgress,
    EnrichmentCompleted,
}

pub struct AWSResourceClient {
    #[allow(dead_code)]
    normalizer_factory: NormalizerFactory,
    credential_coordinator: Arc<CredentialCoordinator>,
    pagination_config: PaginationConfig,
    tag_cache: Arc<TagCache>,
    // Services are now created lazily instead of pre-instantiated
}

impl AWSResourceClient {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            normalizer_factory: NormalizerFactory,
            credential_coordinator,
            pagination_config: PaginationConfig::default(),
            tag_cache: Arc::new(TagCache::new()),
        }
    }

    /// Get credential coordinator for bridge tools
    pub fn get_credential_coordinator(&self) -> Arc<CredentialCoordinator> {
        Arc::clone(&self.credential_coordinator)
    }

    // Lazy service getters - create services only when needed
    fn get_ec2_service(&self) -> EC2Service {
        EC2Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_iam_service(&self) -> IAMService {
        IAMService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_autoscaling_service(&self) -> AutoScalingService {
        AutoScalingService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_bedrock_service(&self) -> BedrockService {
        BedrockService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_bedrock_agent_service(&self) -> BedrockAgentService {
        BedrockAgentService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_bedrock_agentcore_control_service(&self) -> BedrockAgentCoreControlService {
        BedrockAgentCoreControlService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_s3_service(&self) -> S3Service {
        S3Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_cloudformation_service(&self) -> CloudFormationService {
        CloudFormationService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_rds_service(&self) -> RDSService {
        RDSService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_lambda_service(&self) -> LambdaService {
        LambdaService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_dynamodb_service(&self) -> DynamoDBService {
        DynamoDBService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_cloudwatch_service(&self) -> CloudWatchService {
        CloudWatchService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_apigateway_service(&self) -> ApiGatewayService {
        ApiGatewayService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_sns_service(&self) -> SNSService {
        SNSService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_sqs_service(&self) -> SQSService {
        SQSService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_ecs_service(&self) -> ECSService {
        ECSService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_eks_service(&self) -> EKSService {
        EKSService::new(Arc::clone(&self.credential_coordinator))
    }

    pub fn get_logs_service(&self) -> LogsService {
        LogsService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_apigatewayv2_service(&self) -> ApiGatewayV2Service {
        ApiGatewayV2Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_kinesis_service(&self) -> KinesisService {
        KinesisService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_sagemaker_service(&self) -> SageMakerService {
        SageMakerService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_redshift_service(&self) -> RedshiftService {
        RedshiftService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_glue_service(&self) -> GlueService {
        GlueService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_lakeformation_service(&self) -> LakeFormationService {
        LakeFormationService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_athena_service(&self) -> AthenaService {
        AthenaService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_ecr_service(&self) -> EcrService {
        EcrService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_emr_service(&self) -> EmrService {
        EmrService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_secretsmanager_service(&self) -> SecretsManagerService {
        SecretsManagerService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_kms_service(&self) -> KmsService {
        KmsService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_stepfunctions_service(&self) -> StepFunctionsService {
        StepFunctionsService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_route53_service(&self) -> Route53Service {
        Route53Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_efs_service(&self) -> EfsService {
        EfsService::new(Arc::clone(&self.credential_coordinator))
    }

    pub fn get_cloudtrail_service(&self) -> CloudTrailService {
        CloudTrailService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_config_service(&self) -> ConfigService {
        ConfigService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_databrew_service(&self) -> DataBrewService {
        DataBrewService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_codeartifact_service(&self) -> CodeArtifactService {
        CodeArtifactService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_codedeploy_service(&self) -> CodeDeployService {
        CodeDeployService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_appconfig_service(&self) -> AppConfigService {
        AppConfigService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_cloudtraildata_service(&self) -> CloudTrailDataService {
        CloudTrailDataService::new(Arc::clone(&self.credential_coordinator))
    }

    // High-value AWS services
    fn get_acm_service(&self) -> AcmService {
        AcmService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_wafv2_service(&self) -> WafV2Service {
        WafV2Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_cloudfront_service(&self) -> CloudFrontService {
        CloudFrontService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_elasticache_service(&self) -> ElastiCacheService {
        ElastiCacheService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_guardduty_service(&self) -> GuardDutyService {
        GuardDutyService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_securityhub_service(&self) -> SecurityHubService {
        SecurityHubService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_detective_service(&self) -> DetectiveService {
        DetectiveService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_accessanalyzer_service(&self) -> AccessAnalyzerService {
        AccessAnalyzerService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_macie_service(&self) -> MacieService {
        MacieService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_inspector_service(&self) -> InspectorService {
        InspectorService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_timestream_service(&self) -> TimestreamService {
        TimestreamService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_documentdb_service(&self) -> DocumentDbService {
        DocumentDbService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_transfer_service(&self) -> TransferService {
        TransferService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_datasync_service(&self) -> DataSyncService {
        DataSyncService::new(Arc::clone(&self.credential_coordinator))
    }

    // Analytics & search services
    fn get_opensearch_service(&self) -> OpenSearchService {
        OpenSearchService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_quicksight_service(&self) -> QuickSightService {
        QuickSightService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_backup_service(&self) -> BackupService {
        BackupService::new(Arc::clone(&self.credential_coordinator))
    }

    // Identity & messaging services
    fn get_cognito_service(&self) -> CognitoService {
        CognitoService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_mq_service(&self) -> MQService {
        MQService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_msk_service(&self) -> MskService {
        MskService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_organizations_service(&self) -> OrganizationsService {
        OrganizationsService::new(Arc::clone(&self.credential_coordinator))
    }

    // Load balancing & networking services
    fn get_elb_service(&self) -> ELBService {
        ELBService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_elbv2_service(&self) -> ELBv2Service {
        ELBv2Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_ssm_service(&self) -> SSMService {
        SSMService::new(Arc::clone(&self.credential_coordinator))
    }

    // DevOps & CI/CD services
    fn get_codepipeline_service(&self) -> CodePipelineService {
        CodePipelineService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_codebuild_service(&self) -> CodeBuildService {
        CodeBuildService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_codecommit_service(&self) -> CodeCommitService {
        CodeCommitService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_eventbridge_service(&self) -> EventBridgeService {
        EventBridgeService::new(Arc::clone(&self.credential_coordinator))
    }

    // IoT & App services
    fn get_appsync_service(&self) -> AppSyncService {
        AppSyncService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_iot_service(&self) -> IoTService {
        IoTService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_greengrass_service(&self) -> GreengrassService {
        GreengrassService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_acmpca_service(&self) -> AcmPcaService {
        AcmPcaService::new(Arc::clone(&self.credential_coordinator))
    }

    // Compute & Data services
    fn get_neptune_service(&self) -> NeptuneService {
        NeptuneService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_batch_service(&self) -> BatchService {
        BatchService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_kinesisfirehose_service(&self) -> KinesisFirehoseService {
        KinesisFirehoseService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_fsx_service(&self) -> FsxService {
        FsxService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_workspaces_service(&self) -> WorkSpacesService {
        WorkSpacesService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_xray_service(&self) -> XRayService {
        XRayService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_shield_service(&self) -> ShieldService {
        ShieldService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_apprunner_service(&self) -> AppRunnerService {
        AppRunnerService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_globalaccelerator_service(&self) -> GlobalAcceleratorService {
        GlobalAcceleratorService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_connect_service(&self) -> ConnectService {
        ConnectService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_amplify_service(&self) -> AmplifyService {
        AmplifyService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_lex_service(&self) -> LexService {
        LexService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_rekognition_service(&self) -> RekognitionService {
        RekognitionService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_polly_service(&self) -> PollyService {
        PollyService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_resource_tagging_service(&self) -> ResourceTaggingService {
        ResourceTaggingService::new(Arc::clone(&self.credential_coordinator))
    }

    // ============================================================================
    // Public Tag Operations
    // ============================================================================

    /// Get the tag cache for direct access
    pub fn get_tag_cache(&self) -> Arc<TagCache> {
        Arc::clone(&self.tag_cache)
    }

    /// Fetch tags for a specific resource using the Resource Groups Tagging API
    ///
    /// This method tries the universal API first, then falls back to service-specific methods
    pub async fn fetch_tags_for_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        account: &str,
        region: &str,
    ) -> Result<Vec<ResourceTag>> {
        let start = Instant::now();

        // Check cache first
        if let Some(cached_tags) = self
            .tag_cache
            .get(resource_type, resource_id, account, region)
            .await
        {
            log_query_op("TAGS", "cache_hit", &format!("{}:{}", resource_type, resource_id));
            return Ok(cached_tags);
        }

        log_query_op("TAGS", "fetch_start", &format!("{}:{} in {}/{}", resource_type, resource_id, account, region));

        tracing::debug!(
            "Fetching tags for {}: {} in {}/{}",
            resource_type,
            resource_id,
            account,
            region
        );

        // Determine service-specific fetching strategy based on resource type
        let tagging_service = self.get_resource_tagging_service();
        let tags = match resource_type {
            "AWS::EC2::Instance"
            | "AWS::EC2::Volume"
            | "AWS::EC2::Snapshot"
            | "AWS::EC2::Image"
            | "AWS::EC2::VPC"
            | "AWS::EC2::SecurityGroup"
            | "AWS::EC2::Subnet"
            | "AWS::EC2::InternetGateway"
            | "AWS::EC2::RouteTable"
            | "AWS::EC2::NatGateway"
            | "AWS::EC2::NetworkInterface"
            | "AWS::EC2::VPCEndpoint"
            | "AWS::EC2::NetworkAcl"
            | "AWS::EC2::KeyPair"
            | "AWS::EC2::FlowLog"           // fl-* prefix
            | "AWS::EC2::TransitGateway"    // tgw-* prefix
            | "AWS::EC2::VPCPeeringConnection" // pcx-* prefix
            | "AWS::EC2::VolumeAttachment"  // Special case: inherits from volume
            | "AWS::EC2::ElasticIP"
            | "AWS::EC2::LaunchTemplate"
            | "AWS::EC2::PlacementGroup"
            | "AWS::EC2::ReservedInstance"
            | "AWS::EC2::SpotInstanceRequest"
            | "AWS::EC2::DHCPOptions"
            | "AWS::EC2::EgressOnlyInternetGateway"
            | "AWS::EC2::VPNConnection"
            | "AWS::EC2::VPNGateway"
            | "AWS::EC2::CustomerGateway"
            => {
                tracing::debug!("Fetching EC2 tags for {} resource: {}", resource_type, resource_id);
                tagging_service.get_ec2_tags(account, region, resource_id).await?
            }
            "AWS::S3::Bucket" => {
                tagging_service.get_s3_bucket_tags(account, region, resource_id).await?
            }
            "AWS::Lambda::Function" => {
                // Lambda list_tags requires full ARN, not just function name
                let arn = format!("arn:aws:lambda:{}:{}:function:{}", region, account, resource_id);
                tagging_service.get_lambda_tags(account, region, &arn).await?
            }
            "AWS::Lambda::EventSourceMapping" => {
                // Event source mapping ARN format
                let arn = format!("arn:aws:lambda:{}:{}:event-source-mapping:{}", region, account, resource_id);
                tagging_service.get_lambda_tags(account, region, &arn).await?
            }
            "AWS::Lambda::LayerVersion" => {
                // Layer version - resource_id should already be ARN or we construct it
                if resource_id.starts_with("arn:") {
                    tagging_service.get_lambda_tags(account, region, resource_id).await?
                } else {
                    // resource_id might be layer:version format
                    let arn = format!("arn:aws:lambda:{}:{}:layer:{}", region, account, resource_id);
                    tagging_service.get_lambda_tags(account, region, &arn).await?
                }
            }
            "AWS::IAM::User" => {
                tagging_service.get_iam_user_tags(account, region, resource_id).await?
            }
            "AWS::IAM::Role" => {
                tagging_service.get_iam_role_tags(account, region, resource_id).await?
            }
            "AWS::IAM::Policy" => {
                // IAM policies use ARN for tagging, not name
                tagging_service.get_iam_policy_tags(account, region, resource_id).await?
            }
            "AWS::IAM::ServerCertificate" => {
                tagging_service.get_iam_server_certificate_tags(account, region, resource_id).await?
            }
            "AWS::Organizations::Account" | "AWS::Organizations::Root" | "AWS::Organizations::OrganizationalUnit" | "AWS::Organizations::Policy" => {
                tagging_service.get_organizations_tags(account, region, resource_id).await?
            }
            "AWS::RDS::DBInstance"
            | "AWS::RDS::DBCluster"
            | "AWS::RDS::DBSnapshot"
            | "AWS::RDS::DBClusterSnapshot"
            | "AWS::RDS::OptionGroup" => {
                tagging_service.get_rds_tags(account, region, resource_id).await?
            }
            "AWS::DynamoDB::Table" => {
                tagging_service.get_dynamodb_tags(account, region, resource_id).await?
            }
            "AWS::SQS::Queue" => {
                // SQS uses queue URL, constructed from queue name
                tagging_service.get_sqs_queue_tags(account, region, resource_id).await?
            }
            "AWS::SNS::Topic" => {
                tagging_service.get_sns_topic_tags(account, region, resource_id).await?
            }
            "AWS::KMS::Key" => {
                // KMS accepts key ID, ARN, or alias
                tagging_service.get_kms_key_tags(account, region, resource_id).await?
            }
            "AWS::CloudFront::Distribution" => {
                // CloudFront is global, uses ARN
                tagging_service.get_cloudfront_distribution_tags(account, region, resource_id).await?
            }
            // EKS - construct ARN if not provided
            "AWS::EKS::Cluster" => {
                let arn = if resource_id.starts_with("arn:") {
                    resource_id.to_string()
                } else {
                    format!("arn:aws:eks:{}:{}:cluster/{}", region, account, resource_id)
                };
                tagging_service.get_eks_resource_tags(account, region, &arn).await?
            }
            "AWS::EKS::FargateProfile" => {
                // FargateProfile ARN: arn:aws:eks:region:account:fargateprofile/cluster-name/profile-name
                if resource_id.starts_with("arn:") {
                    tagging_service.get_eks_resource_tags(account, region, resource_id).await?
                } else {
                    // Fallback to universal API with constructed ARN
                    let arn = format!("arn:aws:eks:{}:{}:fargateprofile/{}", region, account, resource_id);
                    tagging_service.get_tags_for_arn(account, region, &arn).await?
                }
            }
            "AWS::EKS::Addon" | "AWS::EKS::IdentityProviderConfig" => {
                if resource_id.starts_with("arn:") {
                    tagging_service.get_eks_resource_tags(account, region, resource_id).await?
                } else {
                    // These require ARNs - return empty if not provided
                    tracing::warn!(
                        "Cannot fetch tags for {} - resource_id is not an ARN: {}",
                        resource_type,
                        resource_id
                    );
                    Vec::new()
                }
            }
            // ECS - construct ARN if not provided
            "AWS::ECS::Cluster" => {
                let arn = if resource_id.starts_with("arn:") {
                    resource_id.to_string()
                } else {
                    format!("arn:aws:ecs:{}:{}:cluster/{}", region, account, resource_id)
                };
                tagging_service.get_ecs_resource_tags(account, region, &arn).await?
            }
            "AWS::ECS::Service" | "AWS::ECS::FargateService" => {
                // Service ARN: arn:aws:ecs:region:account:service/cluster-name/service-name
                // Since we only have service name, we can't construct full ARN
                if resource_id.starts_with("arn:") {
                    tagging_service.get_ecs_resource_tags(account, region, resource_id).await?
                } else {
                    // Try to use universal tagging API by constructing partial ARN
                    // Services require cluster context, so this may fail
                    let arn = format!("arn:aws:ecs:{}:{}:service/{}", region, account, resource_id);
                    tagging_service.get_tags_for_arn(account, region, &arn).await?
                }
            }
            "AWS::ECS::Task" | "AWS::ECS::FargateTask" => {
                if resource_id.starts_with("arn:") {
                    tagging_service.get_ecs_resource_tags(account, region, resource_id).await?
                } else {
                    let arn = format!("arn:aws:ecs:{}:{}:task/{}", region, account, resource_id);
                    tagging_service.get_tags_for_arn(account, region, &arn).await?
                }
            }
            "AWS::ECS::TaskDefinition" => {
                let arn = if resource_id.starts_with("arn:") {
                    resource_id.to_string()
                } else {
                    // TaskDefinition ARN: arn:aws:ecs:region:account:task-definition/family:revision
                    format!("arn:aws:ecs:{}:{}:task-definition/{}", region, account, resource_id)
                };
                tagging_service.get_ecs_resource_tags(account, region, &arn).await?
            }
            "AWS::ECS::CapacityProvider" | "AWS::ECS::TaskSet" => {
                if resource_id.starts_with("arn:") {
                    tagging_service.get_ecs_resource_tags(account, region, resource_id).await?
                } else {
                    let arn = format!("arn:aws:ecs:{}:{}:capacity-provider/{}", region, account, resource_id);
                    tagging_service.get_tags_for_arn(account, region, &arn).await?
                }
            }
            // CloudFormation - StackId is already an ARN format
            "AWS::CloudFormation::Stack" => {
                // StackId is in ARN format, use universal API
                if resource_id.starts_with("arn:") {
                    tagging_service.get_tags_for_arn(account, region, resource_id).await?
                } else {
                    // Fallback: construct ARN from stack name (less reliable)
                    let arn = format!("arn:aws:cloudformation:{}:{}:stack/{}", region, account, resource_id);
                    tagging_service.get_tags_for_arn(account, region, &arn).await?
                }
            }
            // Amazon MQ - construct ARN from broker ID
            "AWS::AmazonMQ::Broker" => {
                // MQ broker ARN format: arn:aws:mq:region:account:broker:broker-id
                let arn = format!("arn:aws:mq:{}:{}:broker:{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // CodePipeline - construct ARN from pipeline name
            "AWS::CodePipeline::Pipeline" => {
                let arn = format!("arn:aws:codepipeline:{}:{}:{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // MSK - resource_id should already be ClusterArn, but handle both cases
            "AWS::MSK::Cluster" => {
                if resource_id.starts_with("arn:") {
                    tagging_service.get_tags_for_arn(account, region, resource_id).await?
                } else {
                    // Construct ARN if only cluster name provided
                    let arn = format!("arn:aws:kafka:{}:{}:cluster/{}", region, account, resource_id);
                    tagging_service.get_tags_for_arn(account, region, &arn).await?
                }
            }
            // API Gateway v1 - construct ARN from REST API ID
            "AWS::ApiGateway::RestApi" => {
                // API Gateway ARN format: arn:aws:apigateway:region::/restapis/api-id
                let arn = format!("arn:aws:apigateway:{}::/restapis/{}", region, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // API Gateway v2 - construct ARN from API ID
            "AWS::ApiGatewayV2::Api" => {
                // API Gateway v2 ARN format: arn:aws:apigateway:region::/apis/api-id
                let arn = format!("arn:aws:apigateway:{}::/apis/{}", region, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // Route53 - uses dedicated tag API
            "AWS::Route53::HostedZone" => {
                tagging_service.get_route53_hosted_zone_tags(account, region, resource_id).await?
            }
            // EventBridge - construct ARN and use universal API
            "AWS::Events::EventBus" => {
                let arn = format!("arn:aws:events:{}:{}:event-bus/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            "AWS::Events::Rule" => {
                // Rule ARN format: arn:aws:events:region:account:rule/[event-bus-name/]rule-name
                // Since we only have rule name, assume default bus
                let arn = format!("arn:aws:events:{}:{}:rule/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // Lex v2 - uses dedicated tag API
            "AWS::Lex::Bot" | "AWS::Lex::BotAlias" => {
                tagging_service.get_lex_tags(account, region, resource_id).await?
            }
            // QuickSight - uses dedicated tag API
            "AWS::QuickSight::Dashboard"
            | "AWS::QuickSight::DataSet"
            | "AWS::QuickSight::DataSource" => {
                tagging_service.get_quicksight_tags(account, region, resource_id).await?
            }
            // Batch - uses dedicated tag API
            "AWS::Batch::ComputeEnvironment" | "AWS::Batch::JobQueue" => {
                tagging_service.get_batch_tags(account, region, resource_id).await?
            }
            // ACM - uses dedicated tag API
            "AWS::CertificateManager::Certificate" => {
                tagging_service.get_acm_certificate_tags(account, region, resource_id).await?
            }
            // Amplify - uses dedicated tag API
            "AWS::Amplify::App" => {
                tagging_service.get_amplify_tags(account, region, resource_id).await?
            }
            // AppSync - uses dedicated tag API
            "AWS::AppSync::GraphQLApi" => {
                tagging_service.get_appsync_tags(account, region, resource_id).await?
            }
            // Config - uses dedicated tag API
            "AWS::Config::ConfigRule" | "AWS::Config::ConfigurationRecorder" => {
                tagging_service.get_config_tags(account, region, resource_id).await?
            }
            // Timestream - construct ARN and use universal API
            "AWS::Timestream::Database" => {
                let arn = format!("arn:aws:timestream:{}:{}:database/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            "AWS::Timestream::Table" => {
                // Table ARN includes database name, resource_id should be "database/table"
                let arn = format!("arn:aws:timestream:{}:{}:database/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // XRay - construct ARN and use universal API
            "AWS::XRay::SamplingRule" => {
                let arn = format!("arn:aws:xray:{}:{}:sampling-rule/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // CodeBuild - construct ARN from project name
            "AWS::CodeBuild::Project" => {
                let arn = format!("arn:aws:codebuild:{}:{}:project/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // SSM Parameter - construct ARN from parameter name
            "AWS::SSM::Parameter" => {
                // SSM parameter names can start with / or not, ARN format handles both
                let param_path = if resource_id.starts_with('/') {
                    resource_id.to_string()
                } else {
                    format!("/{}", resource_id)
                };
                let arn = format!("arn:aws:ssm:{}:{}:parameter{}", region, account, param_path);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // SSM Document - construct ARN from document name
            "AWS::SSM::Document" => {
                let arn = format!("arn:aws:ssm:{}:{}:document/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // CloudWatch Logs - construct ARN from log group name
            "AWS::Logs::LogGroup" => {
                // Log group names can contain / and special characters
                let arn = format!("arn:aws:logs:{}:{}:log-group:{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // Cognito Identity Pool - construct ARN from pool ID
            "AWS::Cognito::IdentityPool" => {
                // Identity pool ID format: region:guid
                let arn = format!("arn:aws:cognito-identity:{}:{}:identitypool/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // Cognito User Pool - construct ARN from pool ID
            "AWS::Cognito::UserPool" => {
                let arn = format!("arn:aws:cognito-idp:{}:{}:userpool/{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // CodeCommit - construct ARN from repository name
            "AWS::CodeCommit::Repository" => {
                let arn = format!("arn:aws:codecommit:{}:{}:{}", region, account, resource_id);
                tagging_service.get_tags_for_arn(account, region, &arn).await?
            }
            // Resources that explicitly don't support tagging - return empty immediately
            "AWS::Organizations::CreateAccountStatus"
            | "AWS::Organizations::AwsServiceAccess"
            | "AWS::Organizations::DelegatedAdministrator"
            | "AWS::Organizations::Handshake"
            | "AWS::Organizations::Organization"
            | "AWS::CloudWatch::Metric"
            | "AWS::BedrockAgentCore::AgentRuntime"
            | "AWS::BedrockAgentCore::AgentRuntimeEndpoint"
            | "AWS::BedrockAgentCore::Memory"
            | "AWS::BedrockAgentCore::Gateway"
            | "AWS::BedrockAgentCore::Browser"
            | "AWS::BedrockAgentCore::CodeInterpreter"
            | "AWS::BedrockAgentCore::ApiKeyCredentialProvider"
            | "AWS::BedrockAgentCore::OAuth2CredentialProvider"
            | "AWS::BedrockAgentCore::WorkloadIdentity"
            | "AWS::BedrockAgentCore::AgentRuntimeVersion"
            | "AWS::BedrockAgentCore::GatewayTarget"
            | "AWS::BedrockAgentCore::MemoryRecord"
            | "AWS::BedrockAgentCore::Event"
            | "AWS::BedrockAgentCore::BrowserSession"
            | "AWS::BedrockAgentCore::CodeInterpreterSession" => {
                tracing::debug!(
                    "Skipping tag fetch for {}: {} - resource type does not support tagging",
                    resource_type,
                    resource_id
                );
                Vec::new()
            }
            // Default: Try universal Resource Groups Tagging API
            _ => {
                // Construct ARN if we have resource_id in ARN format
                if resource_id.starts_with("arn:") {
                    tagging_service.get_tags_for_arn(account, region, resource_id).await?
                } else {
                    // If not an ARN, return empty tags (service doesn't support tagging or needs specific implementation)
                    tracing::warn!(
                        "Cannot fetch tags for {}: {} - not an ARN and no service-specific implementation",
                        resource_type,
                        resource_id
                    );
                    Vec::new()
                }
            }
        };

        // Cache the result
        self.tag_cache
            .set(resource_type, resource_id, account, region, tags.clone())
            .await;

        let elapsed_ms = start.elapsed().as_millis();
        log_query_op("TAGS", "fetch_done", &format!("{}:{} ({} tags, {}ms)", resource_type, resource_id, tags.len(), elapsed_ms));

        Ok(tags)
    }

    /// Invalidate cached tags for a specific resource
    pub async fn invalidate_resource_tags(
        &self,
        resource_type: &str,
        resource_id: &str,
        account: &str,
        region: &str,
    ) {
        self.tag_cache
            .invalidate(resource_type, resource_id, account, region)
            .await;
    }

    /// Invalidate all cached tags
    pub async fn invalidate_all_tags(&self) {
        self.tag_cache.invalidate_all().await;
    }

    /// Get tag cache statistics
    pub async fn get_tag_cache_stats(&self) -> super::tag_cache::CacheStats {
        self.tag_cache.get_stats().await
    }

    // ============================================================================
    // Resource Query Methods
    // ============================================================================

    /// Query AWS resources for all combinations of accounts, regions, and resource types in parallel
    /// Results are sent back as they arrive via the progress_sender channel
    pub async fn query_aws_resources_parallel(
        &self,
        scope: &QueryScope,
        result_sender: mpsc::Sender<QueryResult>,
        progress_sender: Option<mpsc::Sender<QueryProgress>>,
        cache: Arc<SharedResourceCache>,
    ) -> Result<()> {
        info!(
            "Starting parallel AWS resource queries for {} accounts, {} regions, {} resource types",
            scope.accounts.len(),
            scope.regions.len(),
            scope.resource_types.len()
        );

        // Build list of expected queries for tracking
        let global_registry = GlobalServiceRegistry::new();
        let mut expected_queries: Vec<String> = Vec::new();
        let mut seen_globals: HashSet<(String, String)> = HashSet::new();

        for account in &scope.accounts {
            for resource_type in &scope.resource_types {
                if global_registry.is_global(&resource_type.resource_type) {
                    let key = (account.account_id.clone(), resource_type.resource_type.clone());
                    if !seen_globals.contains(&key) {
                        seen_globals.insert(key);
                        expected_queries.push(format!("{}:Global:{}", account.account_id, resource_type.resource_type));
                    }
                } else {
                    for region in &scope.regions {
                        expected_queries.push(format!("{}:{}:{}", account.account_id, region.region_code, resource_type.resource_type));
                    }
                }
            }
        }

        // Start phase tracking with expected queries
        super::query_timing::start_phase("PHASE1", expected_queries);

        // Reset concurrency counters for new phase
        super::query_timing::reset_concurrency_counters();

        // Clear retry tracker state for new query phase
        retry_tracker().clear_query_state();

        // Create semaphore to limit concurrent requests
        let semaphore = Arc::new(Semaphore::new(
            self.pagination_config.max_concurrent_requests,
        ));

        // Create futures for all combinations
        let mut futures: FuturesUnordered<BoxFuture<'static, ()>> = FuturesUnordered::new();
        let mut total_queries = 0;

        // Track which global services have been queried per account to avoid duplicates
        let mut queried_global_services: HashSet<(String, String)> = HashSet::new();

        for account in &scope.accounts {
            for resource_type in &scope.resource_types {
                // Check if this is a global service
                if global_registry.is_global(&resource_type.resource_type) {
                    // For global services, only query once per account
                    let global_key = (
                        account.account_id.clone(),
                        resource_type.resource_type.clone(),
                    );

                    if queried_global_services.contains(&global_key) {
                        // Already queried this global service for this account, skip
                        continue;
                    }

                    queried_global_services.insert(global_key);

                    // Query from the designated global region (us-east-1)
                    let query_region = global_registry.get_query_region();
                    let cache_key = format!(
                        "{}:Global:{}",
                        account.account_id, resource_type.resource_type
                    );

                    // Check cache first (using SharedResourceCache)
                    if let Some(cached_resources) = cache.get_resources_owned(&cache_key) {
                        info!("Using cached global resources for {}", cache_key);

                        // Track cache hit in query_timing (so it doesn't appear as MISSING)
                        let tracking_key = format!("{}:Global:{}", account.account_id, resource_type.resource_type);
                        super::query_timing::query_start(&tracking_key);
                        super::query_timing::query_done(&tracking_key, "cached");

                        // Send cached result immediately
                        let cached_result = QueryResult {
                            account_id: account.account_id.clone(),
                            region: "Global".to_string(),
                            resource_type: resource_type.resource_type.clone(),
                            resources: Ok(cached_resources),
                            cache_key: cache_key.clone(),
                        };

                        if let Err(e) = result_sender.send(cached_result).await {
                            warn!("Failed to send cached global result: {}", e);
                        }
                        continue;
                    }

                    // Create query future for global service
                    let account_id = account.account_id.clone();
                    let resource_type_str = resource_type.resource_type.clone();
                    let display_name = resource_type.display_name.clone();
                    let client = self.clone();
                    let semaphore_clone = semaphore.clone();
                    let progress_sender_clone = progress_sender.clone();
                    let result_sender_clone = result_sender.clone();
                    let cache_clone = cache.clone();
                    let cache_key_clone = cache_key.clone();
                    let query_region = query_region.to_string();

                    let future = async move {
                        // Acquire semaphore permit - handle closed semaphore gracefully
                        let _permit = match semaphore_clone.acquire().await {
                            Ok(permit) => permit,
                            Err(_) => {
                                warn!("Semaphore closed, aborting global query");
                                return;
                            }
                        };

                        // THEORY LOGGING: Track global service future lifecycle
                        let query_id = format!("{}:Global:{}", account_id, resource_type_str);
                        info!("🚀 [FUTURE START] {} - acquired semaphore (global service)", query_id);
                        let start_time = std::time::Instant::now();

                        // Send start progress for global service
                        if let Some(sender) = &progress_sender_clone {
                            let _ = sender
                                .send(QueryProgress {
                                    account: account_id.clone(),
                                    region: "Global".to_string(),
                                    resource_type: resource_type_str.clone(),
                                    status: QueryStatus::Started,
                                    message: format!("Querying global service {}", display_name),
                                    items_processed: Some(0),
                                    estimated_total: None,
                                })
                                .await;
                        }

                        // Execute the query from the global region
                        info!("🔍 [API CALL START] {} - calling AWS API (global)", query_id);
                        let query_result = client
                            .query_resource_type(&account_id, &query_region, &resource_type_str, progress_sender_clone.as_ref())
                            .await;
                        let elapsed = start_time.elapsed();
                        info!("📊 [API CALL END] {} - completed in {:?} (global)", query_id, elapsed);

                        // Handle the result
                        let resources_result = match query_result {
                            Ok(mut resources) => {
                                // For true global services, mark as Global region
                                // For hybrid-global services (S3), preserve the actual region
                                // which was already set during the query (e.g., from get_bucket_location)
                                if resource_type_str != "AWS::S3::Bucket" {
                                    for resource in &mut resources {
                                        resource.region = "Global".to_string();
                                    }
                                }

                                let resource_count = resources.len();
                                info!(
                                    "Global service query completed: {} resources for {}",
                                    resource_count, cache_key_clone
                                );

                                // Cache the results (using SharedResourceCache)
                                cache_clone.insert_resources_owned(cache_key_clone.clone(), resources.clone());

                                // Send completion progress
                                if let Some(sender) = &progress_sender_clone {
                                    let _ = sender
                                        .send(QueryProgress {
                                            account: account_id.clone(),
                                            region: "Global".to_string(),
                                            resource_type: resource_type_str.clone(),
                                            status: QueryStatus::Completed,
                                            message: format!(
                                                "Found {} global {}",
                                                resource_count, display_name
                                            ),
                                            items_processed: Some(resource_count),
                                            estimated_total: Some(resource_count),
                                        })
                                        .await;
                                }

                                Ok(resources)
                            }
                            Err(e) => {
                                let query_id = format!("{}:Global:{}", account_id, resource_type_str);
                                let error_str = e.to_string();

                                // Categorize the error for retry tracking
                                let error_category = categorize_error_string(
                                    &error_str,
                                    &display_name,
                                    "query",
                                );

                                // Record transient errors for visibility
                                if error_category.is_retryable() {
                                    retry_tracker().record_transient_error(
                                        &query_id,
                                        error_category.clone(),
                                    );
                                } else {
                                    retry_tracker().record_failure(&query_id, error_category);
                                }

                                error!("Failed to query global service {}: {}", cache_key_clone, e);

                                // Send failure progress
                                if let Some(sender) = &progress_sender_clone {
                                    let _ = sender
                                        .send(QueryProgress {
                                            account: account_id.clone(),
                                            region: "Global".to_string(),
                                            resource_type: resource_type_str.clone(),
                                            status: QueryStatus::Failed,
                                            message: format!("Failed: {}", e),
                                            items_processed: None,
                                            estimated_total: None,
                                        })
                                        .await;
                                }

                                Err(e)
                            }
                        };

                        // Send the result
                        let result = QueryResult {
                            account_id: account_id.clone(),
                            region: "Global".to_string(),
                            resource_type: resource_type_str.clone(),
                            resources: resources_result,
                            cache_key: cache_key_clone,
                        };

                        info!("📤 [SEND RESULT] {}:Global:{} - sending to channel", account_id, resource_type_str);
                        if let Err(e) = result_sender_clone.send(result).await {
                            warn!("Failed to send global query result: {}", e);
                        }
                        info!("✅ [FUTURE END] {}:Global:{} - future completed successfully", account_id, resource_type_str);
                    };

                    futures.push(Box::pin(future));
                    total_queries += 1;
                } else {
                    // Regular regional service - query for each selected region
                    for region in &scope.regions {
                        let cache_key = format!(
                            "{}:{}:{}",
                            account.account_id, region.region_code, resource_type.resource_type
                        );

                        // Check cache first (using SharedResourceCache)
                        if let Some(cached_resources) = cache.get_resources_owned(&cache_key) {
                            info!("Using cached resources for {}", cache_key);

                            // Track cache hit in query_timing (so it doesn't appear as MISSING)
                            super::query_timing::query_start(&cache_key);
                            super::query_timing::query_done(&cache_key, "cached");

                            // Send cached result immediately
                            let cached_result = QueryResult {
                                account_id: account.account_id.clone(),
                                region: region.region_code.clone(),
                                resource_type: resource_type.resource_type.clone(),
                                resources: Ok(cached_resources),
                                cache_key: cache_key.clone(),
                            };

                            if let Err(e) = result_sender.send(cached_result).await {
                                warn!("Failed to send cached result: {}", e);
                            }
                            continue;
                        }

                        // Create parallel query future
                        let account_id = account.account_id.clone();
                        let region_code = region.region_code.clone();
                        let resource_type_str = resource_type.resource_type.clone();
                        let display_name = resource_type.display_name.clone();
                        let client = self.clone();
                        let semaphore_clone = semaphore.clone();
                        let progress_sender_clone = progress_sender.clone();
                        let result_sender_clone = result_sender.clone();
                        let cache_clone = cache.clone();
                        let cache_key_clone = cache_key.clone();

                        let future = async move {
                            // Acquire semaphore permit - handle closed semaphore gracefully
                            let _permit = match semaphore_clone.acquire().await {
                                Ok(permit) => permit,
                                Err(_) => {
                                    warn!("Semaphore closed, aborting region query");
                                    return;
                                }
                            };

                            // THEORY LOGGING: Track future lifecycle
                            let query_id = format!("{}:{}:{}", account_id, region_code, resource_type_str);
                            info!("🚀 [FUTURE START] {} - acquired semaphore", query_id);
                            let start_time = std::time::Instant::now();

                            // Send start progress
                            if let Some(sender) = &progress_sender_clone {
                                let _ = sender
                                    .send(QueryProgress {
                                        account: account_id.clone(),
                                        region: region_code.clone(),
                                        resource_type: resource_type_str.clone(),
                                        status: QueryStatus::Started,
                                        message: format!(
                                            "Starting parallel query for {}",
                                            display_name
                                        ),
                                        items_processed: Some(0),
                                        estimated_total: None,
                                    })
                                    .await;
                            }

                            // Execute the query
                            info!("🔍 [API CALL START] {} - calling AWS API", query_id);
                            let query_result = client
                                .query_resource_type(&account_id, &region_code, &resource_type_str, progress_sender_clone.as_ref())
                                .await;
                            let elapsed = start_time.elapsed();
                            info!("📊 [API CALL END] {} - completed in {:?}", query_id, elapsed);

                            // Handle the result
                            let resources_result = match query_result {
                                Ok(resources) => {
                                    let resource_count = resources.len();
                                    tracing::debug!(
                                        "Parallel query completed: {} resources for {}",
                                        resource_count, cache_key_clone
                                    );

                                    // Cache the results (using SharedResourceCache)
                                    cache_clone.insert_resources_owned(cache_key_clone.clone(), resources.clone());

                                    // Send completion progress
                                    if let Some(sender) = &progress_sender_clone {
                                        let _ = sender
                                            .send(QueryProgress {
                                                account: account_id.clone(),
                                                region: region_code.clone(),
                                                resource_type: resource_type_str.clone(),
                                                status: QueryStatus::Completed,
                                                message: format!(
                                                    "Parallel query completed for {} ({} items)",
                                                    display_name, resource_count
                                                ),
                                                items_processed: Some(resource_count),
                                                estimated_total: Some(resource_count),
                                            })
                                            .await;
                                    }

                                    Ok(resources)
                                }
                                Err(e) => {
                                    // Get credential information for error context
                                    let role_info = match client
                                        .credential_coordinator
                                        .get_credentials_for_account(&account_id)
                                        .await
                                    {
                                        Ok(creds) => format!("role: {}", creds.role_name),
                                        Err(_) => "role: unknown".to_string(),
                                    };

                                    // Create detailed service-specific error message
                                    let detailed_error = client.format_service_error(
                                        &e,
                                        &resource_type_str,
                                        &display_name,
                                        &account_id,
                                        &region_code,
                                        &role_info,
                                    );

                                    // Categorize the error for retry tracking
                                    let error_category = categorize_error_string(
                                        &detailed_error,
                                        &display_name,
                                        "query",
                                    );

                                    // Record transient errors for visibility
                                    if error_category.is_retryable() {
                                        retry_tracker().record_transient_error(
                                            &query_id,
                                            error_category.clone(),
                                        );
                                    } else {
                                        retry_tracker().record_failure(&query_id, error_category);
                                    }

                                    error!("Parallel query failed: {}", detailed_error);

                                    // Send failure progress
                                    if let Some(sender) = &progress_sender_clone {
                                        let _ = sender
                                            .send(QueryProgress {
                                                account: account_id.clone(),
                                                region: region_code.clone(),
                                                resource_type: resource_type_str.clone(),
                                                status: QueryStatus::Failed,
                                                message: detailed_error,
                                                items_processed: Some(0),
                                                estimated_total: None,
                                            })
                                            .await;
                                    }

                                    Err(e)
                                }
                            };

                            // Send result back
                            let query_result = QueryResult {
                                account_id: account_id.clone(),
                                region: region_code.clone(),
                                resource_type: resource_type_str.clone(),
                                resources: resources_result,
                                cache_key: cache_key_clone,
                            };

                            info!("📤 [SEND RESULT] {}:{}:{} - sending to channel", account_id, region_code, resource_type_str);
                            if let Err(e) = result_sender_clone.send(query_result).await {
                                warn!("Failed to send query result: {}", e);
                            }
                            info!("✅ [FUTURE END] {}:{}:{} - future completed successfully", account_id, region_code, resource_type_str);
                        };

                        futures.push(Box::pin(future));
                        total_queries += 1;
                    }
                }
            }
        }

        info!(
            "Executing {} parallel queries with max concurrency of {}",
            total_queries, self.pagination_config.max_concurrent_requests
        );

        // Execute all futures concurrently
        let mut completed_count = 0;
        while (futures.next().await).is_some() {
            completed_count += 1;
            info!("🔄 [FUTURES LOOP] {}/{} futures completed", completed_count, total_queries);

            // Periodic watchdog pulse every 10 completions to track progress
            if completed_count % 10 == 0 {
                super::query_timing::watchdog_pulse();
            }

            // Run stuck query diagnostics if progress seems slow (every 50 completions)
            if completed_count % 50 == 0 {
                super::query_timing::diagnose_stuck_operations();
            }
        }
        info!("🏁 [FUTURES LOOP] All {} futures finished", completed_count);

        // Log concurrency and tag fetch summary before ending phase
        super::query_timing::log_concurrency_summary();
        super::query_timing::log_tag_fetch_summary();

        // End phase tracking and log summary with any anomalies
        super::query_timing::end_phase("PHASE1");

        // Log cache statistics at end of phase
        cache.log_stats();

        // CRITICAL: Explicitly drop senders to close channels.
        // This signals to receivers that no more messages will be sent,
        // allowing them to exit their recv() loops. Without this, the
        // receiver in window.rs waits forever causing a deadlock when
        // querying multiple resource types.
        drop(result_sender);
        drop(progress_sender);

        info!("All parallel queries completed");
        Ok(())
    }

    /// Convenience wrapper around query_aws_resources_parallel() that collects all
    /// results into a single Vec<ResourceEntry> and extracts resource relationships.
    ///
    /// Used by: CloudFormation Manager's ResourceLookupService (resource_lookup.rs)
    /// for populating parameter dropdowns with AWS resources (e.g., EC2 instance IDs,
    /// S3 bucket names, Lambda function ARNs).
    ///
    /// Why this exists: Simplifies API for callers that need synchronous result
    /// collection rather than streaming results via channels. Internally delegates
    /// to query_aws_resources_parallel() but handles channel setup/teardown and
    /// aggregates QueryResults into a single vector.
    ///
    /// Returns: Aggregated Vec<ResourceEntry> with relationships extracted
    pub async fn query_aws_resources(
        &self,
        scope: &QueryScope,
        progress_sender: Option<mpsc::Sender<QueryProgress>>,
        cache: Arc<SharedResourceCache>,
    ) -> Result<Vec<ResourceEntry>> {
        // Create channels for results
        let (result_sender, mut result_receiver) = mpsc::channel::<QueryResult>(1000);

        // Start parallel queries
        let query_future = self.query_aws_resources_parallel(
            scope,
            result_sender,
            progress_sender,
            cache,
        );

        // Collect results
        let mut all_resources = Vec::new();

        // Run queries and collect results concurrently
        tokio::select! {
            _ = query_future => {},
            _ = async {
                while let Some(result) = result_receiver.recv().await {
                    match result.resources {
                        Ok(resources) => {
                            all_resources.extend(resources);
                        }
                        Err(_) => {
                            // Errors are already logged in the query method
                        }
                    }
                }
            } => {}
        }

        // Extract relationships between resources
        self.extract_all_relationships(&mut all_resources);

        Ok(all_resources)
    }

    /// Query a specific resource type for a given account and region
    ///
    /// If progress_sender is provided, sends FetchingTags progress updates during normalization.
    async fn query_resource_type(
        &self,
        account: &str,
        region: &str,
        resource_type: &str,
        progress_sender: Option<&mpsc::Sender<QueryProgress>>,
    ) -> Result<Vec<ResourceEntry>> {
        // Use "Global" for tracking key if this is a global service
        // This matches the Phase 1 tracking key format
        let global_registry = GlobalServiceRegistry::new();
        let tracking_region = if global_registry.is_global(resource_type) {
            "Global"
        } else {
            region
        };
        let query_key = format!("{}:{}:{}", account, tracking_region, resource_type);
        let query_start_time = Instant::now();
        super::query_timing::query_start(&query_key);

        let raw_resources = match resource_type {
            "AWS::EC2::Instance" => {
                self.get_ec2_service()
                    .list_instances(account, region)
                    .await?
            }
            "AWS::EC2::SecurityGroup" => {
                self.get_ec2_service()
                    .list_security_groups(account, region)
                    .await?
            }
            "AWS::EC2::VPC" => self.get_ec2_service().list_vpcs(account, region).await?,
            "AWS::EC2::Volume" => self.get_ec2_service().list_volumes(account, region).await?,
            "AWS::EC2::Snapshot" => {
                self.get_ec2_service()
                    .list_snapshots(account, region)
                    .await?
            }
            "AWS::EC2::Image" => self.get_ec2_service().list_amis(account, region).await?,
            "AWS::EC2::Subnet" => self.get_ec2_service().list_subnets(account, region).await?,
            "AWS::EC2::InternetGateway" => {
                self.get_ec2_service()
                    .list_internet_gateways(account, region)
                    .await?
            }
            "AWS::EC2::TransitGateway" => {
                self.get_ec2_service()
                    .list_transit_gateways(account, region)
                    .await?
            }
            "AWS::EC2::VPCPeeringConnection" => {
                self.get_ec2_service()
                    .list_vpc_peering_connections(account, region)
                    .await?
            }
            "AWS::EC2::FlowLog" => {
                self.get_ec2_service()
                    .list_flow_logs(account, region)
                    .await?
            }
            "AWS::EC2::VolumeAttachment" => {
                self.get_ec2_service()
                    .list_volume_attachments(account, region)
                    .await?
            }
            "AWS::EC2::ElasticIP" => {
                self.get_ec2_service()
                    .list_elastic_ips(account, region)
                    .await?
            }
            "AWS::EC2::LaunchTemplate" => {
                self.get_ec2_service()
                    .list_launch_templates(account, region)
                    .await?
            }
            "AWS::EC2::PlacementGroup" => {
                self.get_ec2_service()
                    .list_placement_groups(account, region)
                    .await?
            }
            "AWS::EC2::ReservedInstance" => {
                self.get_ec2_service()
                    .list_reserved_instances(account, region)
                    .await?
            }
            "AWS::EC2::SpotInstanceRequest" => {
                self.get_ec2_service()
                    .list_spot_instance_requests(account, region)
                    .await?
            }
            "AWS::EC2::DHCPOptions" => {
                self.get_ec2_service()
                    .list_dhcp_options(account, region)
                    .await?
            }
            "AWS::EC2::EgressOnlyInternetGateway" => {
                self.get_ec2_service()
                    .list_egress_only_internet_gateways(account, region)
                    .await?
            }
            "AWS::EC2::VPNConnection" => {
                self.get_ec2_service()
                    .list_vpn_connections(account, region)
                    .await?
            }
            "AWS::EC2::VPNGateway" => {
                self.get_ec2_service()
                    .list_vpn_gateways(account, region)
                    .await?
            }
            "AWS::EC2::CustomerGateway" => {
                self.get_ec2_service()
                    .list_customer_gateways(account, region)
                    .await?
            }
            "AWS::ECS::FargateService" => {
                self.get_ecs_service()
                    .list_fargate_services(account, region)
                    .await?
            }
            "AWS::ECS::FargateTask" => {
                self.get_ecs_service()
                    .list_fargate_tasks(account, region)
                    .await?
            }
            "AWS::EKS::FargateProfile" => {
                self.get_eks_service()
                    .list_fargate_profiles(account, region)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::IAM::Role" => {
                self.get_iam_service()
                    .list_roles(account, region, false)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::IAM::User" => {
                self.get_iam_service()
                    .list_users(account, region, false)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::IAM::Policy" => {
                self.get_iam_service()
                    .list_policies(account, region, false)
                    .await?
            }
            "AWS::Bedrock::Model" => {
                self.get_bedrock_service()
                    .list_foundation_models(account, region)
                    .await?
            }
            "AWS::Bedrock::InferenceProfile" => {
                self.get_bedrock_service()
                    .list_inference_profiles(account, region)
                    .await?
            }
            "AWS::Bedrock::Guardrail" => {
                self.get_bedrock_service()
                    .list_guardrails(account, region)
                    .await?
            }
            "AWS::Bedrock::ProvisionedModelThroughput" => {
                self.get_bedrock_service()
                    .list_provisioned_model_throughputs(account, region)
                    .await?
            }
            "AWS::Bedrock::Agent" => {
                self.get_bedrock_agent_service()
                    .list_agents(account, region)
                    .await?
            }
            "AWS::Bedrock::KnowledgeBase" => {
                self.get_bedrock_agent_service()
                    .list_knowledge_bases(account, region)
                    .await?
            }
            "AWS::Bedrock::CustomModel" => {
                self.get_bedrock_service()
                    .list_custom_models(account, region)
                    .await?
            }
            "AWS::Bedrock::ImportedModel" => {
                self.get_bedrock_service()
                    .list_imported_models(account, region)
                    .await?
            }
            "AWS::Bedrock::EvaluationJob" => {
                self.get_bedrock_service()
                    .list_evaluation_jobs(account, region)
                    .await?
            }
            "AWS::Bedrock::ModelInvocationJob" => {
                self.get_bedrock_service()
                    .list_model_invocation_jobs(account, region)
                    .await?
            }
            "AWS::Bedrock::Prompt" => {
                self.get_bedrock_agent_service()
                    .list_prompts(account, region)
                    .await?
            }
            "AWS::Bedrock::Flow" => {
                self.get_bedrock_agent_service()
                    .list_flows(account, region)
                    .await?
            }
            "AWS::Bedrock::ModelCustomizationJob" => {
                self.get_bedrock_service()
                    .list_model_customization_jobs(account, region)
                    .await?
            }
            // BedrockAgentCore - Control Plane Resources
            "AWS::BedrockAgentCore::AgentRuntime" => {
                self.get_bedrock_agentcore_control_service()
                    .list_agent_runtimes(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::AgentRuntimeEndpoint" => {
                self.get_bedrock_agentcore_control_service()
                    .list_agent_runtime_endpoints(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::Memory" => {
                self.get_bedrock_agentcore_control_service()
                    .list_memories(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::Gateway" => {
                self.get_bedrock_agentcore_control_service()
                    .list_gateways(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::Browser" => {
                self.get_bedrock_agentcore_control_service()
                    .list_browsers(account, region)
                    .await?
            }
            // BedrockAgentCore - Additional Control Plane Resources
            "AWS::BedrockAgentCore::CodeInterpreter" => {
                self.get_bedrock_agentcore_control_service()
                    .list_code_interpreters(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::ApiKeyCredentialProvider" => {
                self.get_bedrock_agentcore_control_service()
                    .list_api_key_credential_providers(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::OAuth2CredentialProvider" => {
                self.get_bedrock_agentcore_control_service()
                    .list_oauth2_credential_providers(account, region)
                    .await?
            }
            "AWS::BedrockAgentCore::WorkloadIdentity" => {
                self.get_bedrock_agentcore_control_service()
                    .list_workload_identities(account, region)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::S3::Bucket" => {
                self.get_s3_service()
                    .list_buckets(account, region, false)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::CloudFormation::Stack" => {
                self.get_cloudformation_service()
                    .list_stacks(account, region, false)
                    .await?
            }
            "AWS::RDS::DBInstance" => {
                self.get_rds_service()
                    .list_db_instances(account, region)
                    .await?
            }
            "AWS::RDS::DBCluster" => {
                self.get_rds_service()
                    .list_db_clusters(account, region)
                    .await?
            }
            "AWS::RDS::DBSnapshot" => {
                self.get_rds_service()
                    .list_db_snapshots(account, region)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::Lambda::Function" => {
                self.get_lambda_service()
                    .list_functions(account, region, false)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::DynamoDB::Table" => {
                self.get_dynamodb_service()
                    .list_tables(account, region, false)
                    .await?
            }
            "AWS::CloudWatch::Alarm" => {
                self.get_cloudwatch_service()
                    .list_alarms(account, region)
                    .await?
            }
            "AWS::CloudWatch::CompositeAlarm" => {
                self.get_cloudwatch_service()
                    .list_composite_alarms(account, region)
                    .await?
            }
            "AWS::CloudWatch::Metric" => {
                self.get_cloudwatch_service()
                    .list_metrics(account, region)
                    .await?
            }
            "AWS::CloudWatch::InsightRule" => {
                self.get_cloudwatch_service()
                    .list_insight_rules(account, region)
                    .await?
            }
            "AWS::CloudWatch::AnomalyDetector" => {
                self.get_cloudwatch_service()
                    .list_anomaly_detectors(account, region)
                    .await?
            }
            "AWS::ApiGateway::RestApi" => {
                self.get_apigateway_service()
                    .list_rest_apis(account, region)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::SNS::Topic" => {
                self.get_sns_service()
                    .list_topics(account, region, false)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::SQS::Queue" => {
                self.get_sqs_service()
                    .list_queues(account, region, false)
                    .await?
            }
            "AWS::ECS::Cluster" => {
                self.get_ecs_service()
                    .list_clusters(account, region, false)
                    .await?
            }
            "AWS::EKS::Cluster" => {
                self.get_eks_service()
                    .list_clusters(account, region)
                    .await?
            }
            "AWS::Logs::LogGroup" => {
                self.get_logs_service()
                    .list_log_groups(account, region)
                    .await?
            }
            "AWS::Logs::LogStream" => {
                self.get_logs_service()
                    .list_log_streams(account, region)
                    .await?
            }
            "AWS::Logs::MetricFilter" => {
                self.get_logs_service()
                    .list_metric_filters(account, region)
                    .await?
            }
            "AWS::Logs::SubscriptionFilter" => {
                self.get_logs_service()
                    .list_subscription_filters(account, region)
                    .await?
            }
            "AWS::Logs::ResourcePolicy" => {
                self.get_logs_service()
                    .list_resource_policies(account, region)
                    .await?
            }
            "AWS::Logs::QueryDefinition" => {
                self.get_logs_service()
                    .list_query_definitions(account, region)
                    .await?
            }
            "AWS::ApiGatewayV2::Api" => {
                self.get_apigatewayv2_service()
                    .list_apis(account, region)
                    .await?
            }
            "AWS::Kinesis::Stream" => {
                self.get_kinesis_service()
                    .list_streams(account, region)
                    .await?
            }
            "AWS::SageMaker::Endpoint" => {
                self.get_sagemaker_service()
                    .list_endpoints(account, region)
                    .await?
            }
            "AWS::Redshift::Cluster" => {
                self.get_redshift_service()
                    .list_clusters(account, region, false)
                    .await?
            }
            "AWS::Glue::Job" => {
                self.get_glue_service()
                    .list_jobs(account, region, false)
                    .await?
            }
            "AWS::LakeFormation::DataLakeSettings" => {
                self.get_lakeformation_service()
                    .list_data_lake_settings(account, region)
                    .await?
            }
            "AWS::Athena::WorkGroup" => {
                self.get_athena_service()
                    .list_work_groups(account, region)
                    .await?
            }
            "AWS::ECR::Repository" => {
                self.get_ecr_service()
                    .list_repositories(account, region)
                    .await?
            }
            "AWS::EMR::Cluster" => {
                self.get_emr_service()
                    .list_clusters(account, region, false)
                    .await?
            }
            "AWS::SecretsManager::Secret" => {
                self.get_secretsmanager_service()
                    .list_secrets(account, region)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::KMS::Key" => {
                self.get_kms_service()
                    .list_keys(account, region, false)
                    .await?
            }
            "AWS::StepFunctions::StateMachine" => {
                self.get_stepfunctions_service()
                    .list_state_machines(account, region, false)
                    .await?
            }
            "AWS::Route53::HostedZone" => {
                self.get_route53_service()
                    .list_hosted_zones(account, region)
                    .await?
            }
            "AWS::EFS::FileSystem" => {
                self.get_efs_service()
                    .list_file_systems(account, region)
                    .await?
            }
            "AWS::CloudTrail::Trail" => {
                self.get_cloudtrail_service()
                    .list_trails(account, region)
                    .await?
            }
            "AWS::CloudTrail::Event" => {
                // Query recent CloudTrail management events from the 90-day event history
                use super::aws_services::cloudtrail::LookupEventsParams;

                let params = LookupEventsParams {
                    start_time: Some(chrono::Utc::now() - chrono::Duration::days(7)), // Last 7 days
                    end_time: Some(chrono::Utc::now()),
                    lookup_attribute: None, // No filtering
                    max_results: 50,        // Reasonable default
                    event_category: None,   // Management events (default)
                };

                self.get_cloudtrail_service()
                    .lookup_events(account, region, params)
                    .await?
            }
            "AWS::Config::ConfigurationRecorder" => {
                self.get_config_service()
                    .list_configuration_recorders(account, region)
                    .await?
            }
            "AWS::Config::ConfigRule" => {
                self.get_config_service()
                    .list_config_rules(account, region)
                    .await?
            }
            "AWS::DataBrew::Job" => {
                self.get_databrew_service()
                    .list_jobs(account, region)
                    .await?
            }
            "AWS::DataBrew::Dataset" => {
                self.get_databrew_service()
                    .list_datasets(account, region)
                    .await?
            }
            "AWS::CodeArtifact::Domain" => {
                self.get_codeartifact_service()
                    .list_domains(account, region)
                    .await?
            }
            "AWS::CodeArtifact::Repository" => {
                self.get_codeartifact_service()
                    .list_repositories(account, region)
                    .await?
            }
            "AWS::CodeDeploy::Application" => {
                self.get_codedeploy_service()
                    .list_applications(account, region)
                    .await?
            }
            "AWS::CodeDeploy::DeploymentGroup" => {
                self.get_codedeploy_service()
                    .list_deployment_groups(account, region)
                    .await?
            }
            "AWS::AppConfig::Application" => {
                self.get_appconfig_service()
                    .list_applications(account, region)
                    .await?
            }
            "AWS::AppConfig::Environment" => {
                self.get_appconfig_service()
                    .list_environments(account, region)
                    .await?
            }
            "AWS::AppConfig::ConfigurationProfile" => {
                self.get_appconfig_service()
                    .list_configuration_profiles(account, region)
                    .await?
            }
            "AWS::CloudTrail::EventDataStore" => {
                self.get_cloudtraildata_service()
                    .list_event_data_stores(account, region)
                    .await?
            }
            // High-value AWS services
            "AWS::CertificateManager::Certificate" => {
                self.get_acm_service()
                    .list_certificates(account, region)
                    .await?
            }
            "AWS::WAFv2::WebACL" => {
                self.get_wafv2_service()
                    .list_web_acls(account, region)
                    .await?
            }
            "AWS::CloudFront::Distribution" => {
                self.get_cloudfront_service()
                    .list_distributions(account, region)
                    .await?
            }
            "AWS::ElastiCache::CacheCluster" => {
                self.get_elasticache_service()
                    .list_cache_clusters(account, region)
                    .await?
            }
            "AWS::ElastiCache::ReplicationGroup" => {
                self.get_elasticache_service()
                    .list_replication_groups(account, region)
                    .await?
            }
            "AWS::ElastiCache::ParameterGroup" => {
                self.get_elasticache_service()
                    .list_cache_parameter_groups(account, region)
                    .await?
            }
            "AWS::GuardDuty::Detector" => {
                self.get_guardduty_service()
                    .list_detectors(account, region)
                    .await?
            }
            "AWS::SecurityHub::Hub" => {
                self.get_securityhub_service()
                    .list_hubs(account, region)
                    .await?
            }
            "AWS::Detective::Graph" => {
                self.get_detective_service()
                    .list_graphs(account, region)
                    .await?
            }
            "AWS::AccessAnalyzer::Analyzer" => {
                self.get_accessanalyzer_service()
                    .list_analyzers(account, region)
                    .await?
            }
            // Analytics & search services
            "AWS::OpenSearchService::Domain" => {
                self.get_opensearch_service()
                    .list_domains(account, region, false)
                    .await?
            }
            "AWS::QuickSight::DataSource" => {
                self.get_quicksight_service()
                    .list_data_sources(account, region)
                    .await?
            }
            "AWS::QuickSight::Dashboard" => {
                self.get_quicksight_service()
                    .list_dashboards(account, region)
                    .await?
            }
            "AWS::QuickSight::DataSet" => {
                self.get_quicksight_service()
                    .list_data_sets(account, region)
                    .await?
            }
            "AWS::Backup::BackupPlan" => {
                self.get_backup_service()
                    .list_backup_plans(account, region, false)
                    .await?
            }
            "AWS::Backup::BackupVault" => {
                self.get_backup_service()
                    .list_backup_vaults(account, region, false)
                    .await?
            }
            // Identity & messaging services
            // Phase 1: Quick list without details for fast UI update
            "AWS::Cognito::UserPool" => {
                self.get_cognito_service()
                    .list_user_pools(account, region, false)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::Cognito::IdentityPool" => {
                self.get_cognito_service()
                    .list_identity_pools(account, region, false)
                    .await?
            }
            "AWS::MQ::Broker" => self.get_mq_service().list_brokers(account, region).await?,
            "AWS::Organizations::Account" => {
                self.get_organizations_service()
                    .list_accounts(account, region)
                    .await?
            }
            "AWS::Organizations::DelegatedAdministrator" => {
                self.get_organizations_service()
                    .list_delegated_administrators(account, region)
                    .await?
            }
            "AWS::Organizations::Handshake" => {
                self.get_organizations_service()
                    .list_handshakes_for_organization(account, region)
                    .await?
            }
            "AWS::Organizations::CreateAccountStatus" => {
                self.get_organizations_service()
                    .list_create_account_status(account, region)
                    .await?
            }
            "AWS::Organizations::AwsServiceAccess" => {
                self.get_organizations_service()
                    .list_aws_service_access_for_organization(account, region)
                    .await?
            }
            "AWS::Organizations::Organization" => {
                // Organization is a singleton resource - no list operation
                // Users must use describe to query it
                Vec::new()
            }
            "AWS::Organizations::OrganizationalUnit" => {
                self.get_organizations_service()
                    .list_organizational_units(account, region)
                    .await?
            }
            "AWS::Organizations::Policy" => {
                self.get_organizations_service()
                    .list_policies(account, region)
                    .await?
            }
            "AWS::Organizations::Root" => {
                self.get_organizations_service()
                    .list_roots(account, region)
                    .await?
            }
            // Load balancing & networking services
            "AWS::ElasticLoadBalancing::LoadBalancer" => {
                self.get_elb_service()
                    .list_load_balancers(account, region)
                    .await?
            }
            "AWS::ElasticLoadBalancingV2::LoadBalancer" => {
                self.get_elbv2_service()
                    .list_load_balancers(account, region, false)
                    .await?
            }
            "AWS::ElasticLoadBalancingV2::TargetGroup" => {
                self.get_elbv2_service()
                    .list_target_groups(account, region)
                    .await?
            }
            "AWS::SSM::Parameter" => {
                self.get_ssm_service()
                    .list_parameters(account, region)
                    .await?
            }
            "AWS::SSM::Document" => {
                self.get_ssm_service()
                    .list_documents(account, region)
                    .await?
            }
            // DevOps & CI/CD services
            "AWS::CodePipeline::Pipeline" => {
                self.get_codepipeline_service()
                    .list_pipelines(account, region)
                    .await?
            }
            "AWS::CodeBuild::Project" => {
                self.get_codebuild_service()
                    .list_projects(account, region)
                    .await?
            }
            // Phase 1: Quick list without details for fast UI update
            "AWS::CodeCommit::Repository" => {
                self.get_codecommit_service()
                    .list_repositories(account, region, false)
                    .await?
            }
            "AWS::Events::EventBus" => {
                self.get_eventbridge_service()
                    .list_event_buses(account, region, false)
                    .await?
            }
            "AWS::Events::Rule" => {
                self.get_eventbridge_service()
                    .list_rules(account, region)
                    .await?
            }
            // IoT & App services
            "AWS::AppSync::GraphQLApi" => {
                self.get_appsync_service()
                    .list_graphql_apis(account, region)
                    .await?
            }
            "AWS::IoT::Thing" => self.get_iot_service().list_things(account, region).await?,
            "AWS::Greengrass::ComponentVersion" => {
                self.get_greengrass_service()
                    .list_component_versions(account, region)
                    .await?
            }
            "AWS::ACMPCA::CertificateAuthority" => {
                self.get_acmpca_service()
                    .list_certificate_authorities(account, region)
                    .await?
            }
            "AWS::AutoScaling::AutoScalingGroup" => {
                self.get_autoscaling_service()
                    .list_auto_scaling_groups(account, region)
                    .await?
            }
            "AWS::AutoScaling::ScalingPolicy" => {
                self.get_autoscaling_service()
                    .list_scaling_policies(account, region)
                    .await?
            }
            // Compute & Data services
            "AWS::Neptune::DBCluster" => {
                self.get_neptune_service()
                    .list_db_clusters(account, region)
                    .await?
            }
            "AWS::Batch::JobQueue" => {
                self.get_batch_service()
                    .list_job_queues(account, region)
                    .await?
            }
            "AWS::Batch::ComputeEnvironment" => {
                self.get_batch_service()
                    .list_compute_environments(account, region)
                    .await?
            }
            "AWS::KinesisFirehose::DeliveryStream" => {
                self.get_kinesisfirehose_service()
                    .list_delivery_streams(account, region)
                    .await?
            }
            "AWS::MSK::Cluster" => {
                self.get_msk_service()
                    .list_clusters(account, region)
                    .await?
            }
            "AWS::Macie::Session" => {
                self.get_macie_service()
                    .list_classification_jobs(account, region)
                    .await?
            }
            "AWS::Inspector::Configuration" => {
                self.get_inspector_service()
                    .list_findings(account, region)
                    .await?
            }
            "AWS::Timestream::Database" => {
                self.get_timestream_service()
                    .list_databases(account, region)
                    .await?
            }
            "AWS::DocumentDB::Cluster" => {
                self.get_documentdb_service()
                    .list_clusters(account, region)
                    .await?
            }
            "AWS::Transfer::Server" => {
                self.get_transfer_service()
                    .list_servers(account, region)
                    .await?
            }
            "AWS::DataSync::Task" => {
                self.get_datasync_service()
                    .list_tasks(account, region)
                    .await?
            }
            "AWS::FSx::FileSystem" => {
                self.get_fsx_service()
                    .list_file_systems(account, region)
                    .await?
            }
            "AWS::FSx::Backup" => self.get_fsx_service().list_backups(account, region).await?,
            "AWS::WorkSpaces::Workspace" => {
                self.get_workspaces_service()
                    .list_workspaces(account, region)
                    .await?
            }
            "AWS::WorkSpaces::Directory" => {
                self.get_workspaces_service()
                    .list_directories(account, region)
                    .await?
            }
            "AWS::XRay::SamplingRule" => {
                self.get_xray_service()
                    .list_sampling_rules(account, region)
                    .await?
            }
            "AWS::Shield::Protection" => {
                self.get_shield_service()
                    .list_protections(account, region)
                    .await?
            }
            "AWS::Shield::Subscription" => {
                self.get_shield_service()
                    .list_subscriptions(account, region)
                    .await?
            }
            "AWS::AppRunner::Service" => {
                self.get_apprunner_service()
                    .list_services(account, region)
                    .await?
            }
            "AWS::AppRunner::Connection" => {
                self.get_apprunner_service()
                    .list_connections(account, region)
                    .await?
            }
            "AWS::GlobalAccelerator::Accelerator" => {
                self.get_globalaccelerator_service()
                    .list_accelerators(account, region)
                    .await?
            }
            "AWS::Connect::Instance" => {
                self.get_connect_service()
                    .list_instances(account, region)
                    .await?
            }
            "AWS::Amplify::App" => {
                self.get_amplify_service()
                    .list_apps(account, region)
                    .await?
            }
            "AWS::Lex::Bot" => self.get_lex_service().list_bots(account, region).await?,
            "AWS::Rekognition::Collection" => {
                self.get_rekognition_service()
                    .list_collections(account, region)
                    .await?
            }
            "AWS::Rekognition::StreamProcessor" => {
                self.get_rekognition_service()
                    .list_stream_processors(account, region)
                    .await?
            }
            "AWS::Polly::Voice" => {
                self.get_polly_service()
                    .describe_voices(account, region)
                    .await?
            }
            "AWS::Polly::Lexicon" => {
                self.get_polly_service()
                    .list_lexicons(account, region)
                    .await?
            }
            "AWS::Polly::SynthesisTask" => {
                self.get_polly_service()
                    .list_speech_synthesis_tasks(account, region)
                    .await?
            }
            _ => {
                warn!("Unsupported resource type: {}", resource_type);
                super::query_timing::query_failed(&query_key, "unsupported resource type");
                return Ok(Vec::new());
            }
        };

        // Normalize the parent resources (with async tag fetching)
        let mut all_entries = self
            .normalize_resources(raw_resources, account, region, resource_type, progress_sender)
            .await?;

        // Query child resources recursively
        let child_config = ChildResourceConfig::new();
        if child_config.has_children(resource_type) {
            let mut all_children = Vec::new();

            for parent_entry in &all_entries {
                match self
                    .query_children_recursive(
                        parent_entry,
                        &child_config,
                        0, // current depth
                        3, // max depth (prevents infinite loops)
                    )
                    .await
                {
                    Ok(children) => {
                        all_children.extend(children);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to query children for {} ({}): {}",
                            parent_entry.resource_id, parent_entry.resource_type, e
                        );
                        // Continue despite errors - don't fail entire query
                    }
                }
            }

            // Add all children to the result
            all_entries.extend(all_children);
        }

        let _query_elapsed = query_start_time.elapsed().as_millis();
        super::query_timing::query_done(&query_key, &format!("{} resources", all_entries.len()));

        Ok(all_entries)
    }

    /// Query child resources recursively for a parent resource
    fn query_children_recursive<'a>(
        &'a self,
        parent: &'a ResourceEntry,
        child_config: &'a ChildResourceConfig,
        current_depth: usize,
        max_depth: usize,
    ) -> BoxFuture<'a, Result<Vec<ResourceEntry>>> {
        Box::pin(async move {
            // Prevent infinite recursion
            if current_depth >= max_depth {
                warn!(
                    "Max recursion depth {} reached for resource {} (type: {})",
                    max_depth, parent.resource_id, parent.resource_type
                );
                return Ok(vec![]);
            }

            let mut all_descendants = Vec::new();

            // Get direct children
            match self.query_child_resources(parent, child_config).await {
                Ok(children) => {
                    // Recursively get grandchildren for each child
                    for child in children {
                        match self
                            .query_children_recursive(
                                &child,
                                child_config,
                                current_depth + 1,
                                max_depth,
                            )
                            .await
                        {
                            Ok(grandchildren) => {
                                all_descendants.extend(grandchildren);
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to query grandchildren for {} ({}): {}",
                                    child.resource_id, child.resource_type, e
                                );
                                // Continue despite errors - don't fail entire hierarchy
                            }
                        }

                        all_descendants.push(child);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to query children for {} ({}): {}",
                        parent.resource_id, parent.resource_type, e
                    );
                    // Continue despite errors - don't fail parent query
                }
            }

            Ok(all_descendants)
        })
    }

    /// Query child resources for a parent resource
    async fn query_child_resources(
        &self,
        parent: &ResourceEntry,
        child_config: &ChildResourceConfig,
    ) -> Result<Vec<ResourceEntry>> {
        let mut all_children = Vec::new();

        // Check if this parent has children
        if let Some(child_defs) = child_config.get_children(&parent.resource_type) {
            for child_def in child_defs {
                let children = match &child_def.query_method {
                    ChildQueryMethod::SingleParent { param_name } => {
                        self.query_child_with_single_parent(
                            &parent.account_id,
                            &parent.region,
                            &child_def.child_type,
                            param_name,
                            &parent.resource_id,
                            parent,
                        )
                        .await?
                    }
                    ChildQueryMethod::MultiParent { params } => {
                        let parent_params = self.extract_parent_params(parent, params)?;
                        self.query_child_with_multi_parent(
                            &parent.account_id,
                            &parent.region,
                            &child_def.child_type,
                            &parent_params,
                            parent,
                        )
                        .await?
                    }
                };

                all_children.extend(children);
            }
        }

        Ok(all_children)
    }

    /// Query child resources that require a single parent ID
    async fn query_child_with_single_parent(
        &self,
        account: &str,
        region: &str,
        child_type: &str,
        _param_name: &str,
        parent_id: &str,
        parent: &ResourceEntry,
    ) -> Result<Vec<ResourceEntry>> {
        let raw_children = match child_type {
            "AWS::Bedrock::DataSource" => {
                self.get_bedrock_agent_service()
                    .list_data_sources(account, region, parent_id)
                    .await?
            }
            "AWS::Bedrock::AgentAlias" => {
                self.get_bedrock_agent_service()
                    .list_agent_aliases(account, region, parent_id)
                    .await?
            }
            "AWS::Bedrock::FlowAlias" => {
                self.get_bedrock_agent_service()
                    .list_flow_aliases(account, region, parent_id)
                    .await?
            }
            _ => {
                warn!("Unsupported child resource type: {}", child_type);
                return Ok(vec![]);
            }
        };

        self.normalize_child_resources(
            raw_children,
            child_type,
            account,
            region,
            Some(parent_id.to_string()),
            Some(parent.resource_type.clone()),
        )
        .await
    }

    /// Query child resources that require multiple parent parameters
    async fn query_child_with_multi_parent(
        &self,
        account: &str,
        region: &str,
        child_type: &str,
        parent_params: &HashMap<String, String>,
        parent: &ResourceEntry,
    ) -> Result<Vec<ResourceEntry>> {
        let raw_children = match child_type {
            "AWS::Bedrock::IngestionJob" => {
                let kb_id = parent_params
                    .get("knowledge_base_id")
                    .context("Missing knowledge_base_id")?;
                let ds_id = parent_params
                    .get("data_source_id")
                    .context("Missing data_source_id")?;

                self.get_bedrock_agent_service()
                    .list_ingestion_jobs(account, region, kb_id, ds_id)
                    .await?
            }
            "AWS::Bedrock::AgentActionGroup" => {
                let agent_id = parent_params.get("agent_id").context("Missing agent_id")?;
                let agent_version = parent_params
                    .get("agent_version")
                    .context("Missing agent_version")?;

                self.get_bedrock_agent_service()
                    .list_agent_action_groups(account, region, agent_id, agent_version)
                    .await?
            }
            _ => {
                warn!(
                    "Unsupported multi-parent child resource type: {}",
                    child_type
                );
                return Ok(vec![]);
            }
        };

        self.normalize_child_resources(
            raw_children,
            child_type,
            account,
            region,
            Some(parent.resource_id.clone()),
            Some(parent.resource_type.clone()),
        )
        .await
    }

    /// Extract parent parameters from parent resource properties
    fn extract_parent_params(
        &self,
        parent: &ResourceEntry,
        _param_names: &[&str],
    ) -> Result<HashMap<String, String>> {
        let mut params = HashMap::new();

        // For DataSource querying IngestionJobs, we need both kb_id and ds_id
        if parent.resource_type == "AWS::Bedrock::DataSource" {
            // kb_id comes from parent's parent
            if let Some(kb_id) = &parent.parent_resource_id {
                params.insert("knowledge_base_id".to_string(), kb_id.clone());
            } else {
                return Err(anyhow::anyhow!(
                    "DataSource missing parent_resource_id for knowledge_base_id"
                ));
            }
            // ds_id is the DataSource's own ID
            params.insert("data_source_id".to_string(), parent.resource_id.clone());
        }

        // For Agent querying AgentActionGroups, we need agent_id and agent_version
        if parent.resource_type == "AWS::Bedrock::Agent" {
            params.insert("agent_id".to_string(), parent.resource_id.clone());
            // Extract version from properties (default to "DRAFT" if not specified)
            let version = parent
                .properties
                .get("Version")
                .and_then(|v| v.as_str())
                .unwrap_or("DRAFT")
                .to_string();
            params.insert("agent_version".to_string(), version);
        }

        Ok(params)
    }

    /// Normalize child resources with parent tracking (async for tag fetching)
    async fn normalize_child_resources(
        &self,
        raw_resources: Vec<serde_json::Value>,
        resource_type: &str,
        account: &str,
        region: &str,
        parent_id: Option<String>,
        parent_type: Option<String>,
    ) -> Result<Vec<ResourceEntry>> {
        let normalizer = NormalizerFactory::create_normalizer(resource_type)
            .context("No async normalizer available for child resource type")?;

        let query_timestamp = Utc::now();

        // Process all child resources concurrently for faster tag fetching
        let futures: Vec<_> = raw_resources
            .into_iter()
            .map(|raw_resource| {
                normalizer.normalize(raw_resource, account, region, query_timestamp, self)
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut normalized_resources = Vec::new();
        for result in results {
            match result {
                Ok(mut resource) => {
                    // Mark as child resource and set parent info
                    resource.is_child_resource = true;
                    resource.parent_resource_id = parent_id.clone();
                    resource.parent_resource_type = parent_type.clone();

                    // Add bidirectional relationship
                    if let (Some(ref parent_id_val), Some(ref parent_type_val)) =
                        (&parent_id, &parent_type)
                    {
                        resource.relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::ChildOf,
                            target_resource_id: parent_id_val.clone(),
                            target_resource_type: parent_type_val.clone(),
                        });
                    }

                    normalized_resources.push(resource);
                }
                Err(e) => {
                    warn!("Failed to normalize child resource: {}", e);
                }
            }
        }

        Ok(normalized_resources)
    }

    /// Normalize raw AWS API responses into ResourceEntry format (async for tag fetching)
    ///
    /// If a progress_sender is provided, sends FetchingTags progress updates as resources
    /// are normalized (which includes tag fetching).
    async fn normalize_resources(
        &self,
        raw_resources: Vec<serde_json::Value>,
        account: &str,
        region: &str,
        resource_type: &str,
        progress_sender: Option<&mpsc::Sender<QueryProgress>>,
    ) -> Result<Vec<ResourceEntry>> {
        let normalizer = NormalizerFactory::create_normalizer(resource_type)
            .context("No async normalizer available for resource type")?;

        let query_timestamp = Utc::now(); // Capture when this query was executed
        let total = raw_resources.len();

        // Process all resources concurrently using FuturesUnordered for progress tracking
        let mut futures: futures::stream::FuturesUnordered<_> = raw_resources
            .into_iter()
            .map(|raw_resource| {
                normalizer.normalize(raw_resource, account, region, query_timestamp, self)
            })
            .collect();

        let mut normalized_resources = Vec::new();
        let mut processed = 0;
        let mut last_progress_report = std::time::Instant::now();
        let progress_interval_ms = 500; // Report progress every 500ms

        use futures::StreamExt;
        while let Some(result) = futures.next().await {
            processed += 1;
            match result {
                Ok(resource) => normalized_resources.push(resource),
                Err(e) => {
                    warn!("Failed to normalize resource: {}", e);
                }
            }

            // Report progress at intervals to avoid flooding
            if let Some(sender) = progress_sender {
                let elapsed = last_progress_report.elapsed().as_millis();
                if elapsed >= progress_interval_ms || processed == total {
                    let _ = sender
                        .send(QueryProgress {
                            account: account.to_string(),
                            region: region.to_string(),
                            resource_type: resource_type.to_string(),
                            status: QueryStatus::FetchingTags,
                            message: format!("Fetching tags ({}/{})", processed, total),
                            items_processed: Some(processed),
                            estimated_total: Some(total),
                        })
                        .await;
                    last_progress_report = std::time::Instant::now();
                }
            }
        }

        // Sort resources by display name for consistent ordering
        normalized_resources.sort_by(|a, b| a.display_name.cmp(&b.display_name));

        Ok(normalized_resources)
    }

    /// Extract relationships between all resources
    fn extract_all_relationships(&self, resources: &mut [ResourceEntry]) {
        for i in 0..resources.len() {
            if let Some(normalizer) =
                NormalizerFactory::create_normalizer(&resources[i].resource_type)
            {
                let relationships = normalizer.extract_relationships(&resources[i], resources);
                resources[i].relationships = relationships;
            }
        }
    }

    /// Format detailed service-specific error messages with role context
    fn format_service_error(
        &self,
        error: &anyhow::Error,
        resource_type: &str,
        display_name: &str,
        account_id: &str,
        region: &str,
        role_info: &str,
    ) -> String {
        // Convert anyhow error to string for analysis
        let error_str = error.to_string();
        let error_debug = format!("{:?}", error);

        // Check if this is an AWS SDK error by examining the error chain
        let root_error = error.root_cause();

        // Try to match specific AWS service errors based on resource type
        match resource_type {
            "AWS::EC2::Instance" | "AWS::EC2::SecurityGroup" | "AWS::EC2::VPC" => {
                self.format_ec2_error(root_error, display_name, account_id, region, role_info)
            }
            "AWS::IAM::Role" | "AWS::IAM::User" | "AWS::IAM::Policy" => {
                self.format_iam_error(root_error, display_name, account_id, region, role_info)
            }
            "AWS::Bedrock::Model"
            | "AWS::Bedrock::InferenceProfile"
            | "AWS::Bedrock::Guardrail"
            | "AWS::Bedrock::ProvisionedModelThroughput"
            | "AWS::Bedrock::Agent"
            | "AWS::Bedrock::KnowledgeBase"
            | "AWS::Bedrock::CustomModel"
            | "AWS::Bedrock::ImportedModel"
            | "AWS::Bedrock::EvaluationJob"
            | "AWS::Bedrock::ModelInvocationJob"
            | "AWS::Bedrock::Prompt"
            | "AWS::Bedrock::Flow"
            | "AWS::Bedrock::ModelCustomizationJob" => {
                self.format_bedrock_error(root_error, display_name, account_id, region, role_info)
            }
            _ => {
                // Generic AWS error formatting
                self.format_generic_aws_error(
                    &error_str,
                    &error_debug,
                    display_name,
                    account_id,
                    region,
                    role_info,
                )
            }
        }
    }

    /// Format EC2 service specific errors
    fn format_ec2_error(
        &self,
        error: &dyn std::error::Error,
        display_name: &str,
        account_id: &str,
        region: &str,
        role_info: &str,
    ) -> String {
        let error_str = error.to_string();

        // Check for common EC2 error patterns
        if error_str.contains("UnauthorizedOperation") {
            format!(
                "Failed to query {} in account {} region {}: Access denied (UnauthorizedOperation) - {} lacks EC2 permissions for {}",
                display_name, account_id, region, role_info, display_name.to_lowercase()
            )
        } else if error_str.contains("InvalidRegion") {
            format!(
                "Failed to query {} in account {} region {}: Invalid region (InvalidRegion) - region {} may not be enabled for account {} with {}",
                display_name, account_id, region, region, account_id, role_info
            )
        } else if error_str.contains("DryRunOperation") {
            format!(
                "Failed to query {} in account {} region {}: Dry run operation (DryRunOperation) - {} has dry-run permissions only",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("RequestLimitExceeded") {
            format!(
                "Failed to query {} in account {} region {}: Request limit exceeded (RequestLimitExceeded) - too many API calls, retry later with {}",
                display_name, account_id, region, role_info
            )
        } else {
            format!(
                "Failed to query {} in account {} region {}: EC2 service error - {} - {}",
                display_name, account_id, region, error_str, role_info
            )
        }
    }

    /// Format IAM service specific errors
    fn format_iam_error(
        &self,
        error: &dyn std::error::Error,
        display_name: &str,
        account_id: &str,
        region: &str,
        role_info: &str,
    ) -> String {
        let error_str = error.to_string();

        // Check for common IAM error patterns
        if error_str.contains("AccessDenied") {
            format!(
                "Failed to query {} in account {} region {}: Access denied (AccessDenied) - {} lacks IAM permissions for {}",
                display_name, account_id, region, role_info, display_name.to_lowercase()
            )
        } else if error_str.contains("NoSuchEntity") {
            format!(
                "Failed to query {} in account {} region {}: Entity not found (NoSuchEntity) - IAM entity may not exist or {} lacks permissions",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("InvalidInput") {
            format!(
                "Failed to query {} in account {} region {}: Invalid input (InvalidInput) - malformed request parameters with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("ServiceFailure") {
            format!(
                "Failed to query {} in account {} region {}: AWS service failure (ServiceFailure) - temporary IAM service issue with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("LimitExceeded") {
            format!(
                "Failed to query {} in account {} region {}: Limit exceeded (LimitExceeded) - IAM rate limit or quota exceeded with {}",
                display_name, account_id, region, role_info
            )
        } else {
            format!(
                "Failed to query {} in account {} region {}: IAM service error - {} - {}",
                display_name, account_id, region, error_str, role_info
            )
        }
    }

    /// Format Bedrock service specific errors
    fn format_bedrock_error(
        &self,
        error: &dyn std::error::Error,
        display_name: &str,
        account_id: &str,
        region: &str,
        role_info: &str,
    ) -> String {
        let error_str = error.to_string();

        // Check for common Bedrock error patterns
        if error_str.contains("AccessDeniedException") {
            format!(
                "Failed to query {} in account {} region {}: Access denied (AccessDeniedException) - {} lacks Bedrock permissions or Bedrock not enabled",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("ValidationException") {
            format!(
                "Failed to query {} in account {} region {}: Validation error (ValidationException) - invalid request parameters with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("ThrottlingException") {
            format!(
                "Failed to query {} in account {} region {}: Request throttled (ThrottlingException) - too many Bedrock API calls, retry later with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("InternalServerException") {
            format!(
                "Failed to query {} in account {} region {}: Internal server error (InternalServerException) - temporary Bedrock service issue with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("ResourceNotFoundException") {
            format!(
                "Failed to query {} in account {} region {}: Resource not found (ResourceNotFoundException) - Bedrock models may not be available in {} with {}",
                display_name, account_id, region, region, role_info
            )
        } else {
            format!(
                "Failed to query {} in account {} region {}: Bedrock service error - {} - {}",
                display_name, account_id, region, error_str, role_info
            )
        }
    }

    /// Format generic AWS errors with detailed, actionable messages
    fn format_generic_aws_error(
        &self,
        error_str: &str,
        error_debug: &str,
        display_name: &str,
        account_id: &str,
        region: &str,
        role_info: &str,
    ) -> String {
        let detail = if error_str.contains("service error") {
            error_debug
        } else {
            error_str
        };

        // Check for common AWS SDK error patterns

        // Region not enabled/available errors
        if detail.contains("InvalidToken")
            || detail.contains("InvalidIdentityToken")
            || detail.contains("ExpiredTokenException")
        {
            format!(
                "Failed to query {} in account {} region {}: Region unavailable or not enabled - {} is not accessible with current credentials. The region may need to be enabled in AWS Organizations or the account may not have access to this region. ({})",
                display_name, account_id, region, region, role_info
            )
        } else if detail.contains("OptInRequired")
            || detail.contains("SubscriptionRequiredException")
        {
            format!(
                "Failed to query {} in account {} region {}: Region opt-in required - {} requires explicit opt-in through AWS Console. Enable this region in the AWS Account settings. ({})",
                display_name, account_id, region, region, role_info
            )
        } else if detail.contains("AuthFailure")
            || detail.contains("UnauthorizedAccess")
            || detail.contains("AccessDeniedException")
            || detail.contains("AccessDenied")
        {
            format!(
                "Failed to query {} in account {} region {}: Access denied - {} does not have permission to query this resource type. Check IAM policies for {} permissions. ({})",
                display_name, account_id, region, role_info, display_name, role_info
            )
        } else if detail.contains("CredentialsNotLoaded")
            || detail.contains("NoCredentialsError")
        {
            format!(
                "Failed to query {} in account {} region {}: Credentials error - {} credentials are invalid or expired. Try refreshing SSO credentials with 'aws sso login'. ({})",
                display_name, account_id, region, role_info, role_info
            )
        } else if detail.contains("TimeoutError") || detail.contains("timeout") {
            format!(
                "Failed to query {} in account {} region {}: Request timeout - network connectivity issue or slow response from AWS. Check network connection. ({})",
                display_name, account_id, region, role_info
            )
        } else if detail.contains("EndpointResolutionError") {
            format!(
                "Failed to query {} in account {} region {}: Endpoint resolution error - service {} may not be available in {}. ({})",
                display_name, account_id, region, display_name, region, role_info
            )
        } else if detail.contains("DispatchFailure") {
            format!(
                "Failed to query {} in account {} region {}: Network dispatch failure - connectivity issue. Check network connection and VPN if applicable. ({})",
                display_name, account_id, region, role_info
            )
        } else if detail.contains("ConstructionFailure") {
            format!(
                "Failed to query {} in account {} region {}: Request construction error - invalid request parameters. ({})",
                display_name, account_id, region, role_info
            )
        } else if detail.contains("ServiceUnavailable")
            || detail.contains("InternalServerError")
        {
            format!(
                "Failed to query {} in account {} region {}: AWS service temporarily unavailable - try again later. ({})",
                display_name, account_id, region, role_info
            )
        } else if detail.contains("ThrottlingException") || detail.contains("Throttling") {
            format!(
                "Failed to query {} in account {} region {}: Request throttled - too many API calls. AWS Dash will retry automatically. ({})",
                display_name, account_id, region, role_info
            )
        } else {
            // For unknown errors, provide the full error string for debugging
            format!(
                "Failed to query {} in account {} region {}: {} ({})",
                display_name, account_id, region, detail, role_info
            )
        }
    }

    /// Generic describe method that routes to the appropriate resource-specific method
    pub async fn describe_resource(&self, resource: &ResourceEntry) -> Result<serde_json::Value> {
        match resource.resource_type.as_str() {
            "AWS::EC2::Instance" => {
                self.get_ec2_service()
                    .describe_instance(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::EC2::VPC" => {
                self.get_ec2_service()
                    .describe_vpc(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::EC2::SecurityGroup" => {
                self.get_ec2_service()
                    .describe_security_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::IAM::Role" => {
                self.get_iam_service()
                    .describe_role(
                        &resource.account_id,
                        &resource.region,
                        &resource.display_name,
                    )
                    .await
            }
            "AWS::IAM::User" => {
                self.get_iam_service()
                    .describe_user(
                        &resource.account_id,
                        &resource.region,
                        &resource.display_name,
                    )
                    .await
            }
            "AWS::IAM::Policy" => {
                // For IAM policies, we need to use the ARN which is stored in the raw_properties
                if let Some(arn) = resource.raw_properties.get("Arn").and_then(|v| v.as_str()) {
                    self.get_iam_service()
                        .describe_policy(&resource.account_id, &resource.region, arn)
                        .await
                } else {
                    Err(anyhow::anyhow!(
                        "Policy ARN not found in resource properties"
                    ))
                }
            }
            "AWS::Bedrock::Model" => {
                self.get_bedrock_service()
                    .describe_foundation_model(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::InferenceProfile" => {
                self.get_bedrock_service()
                    .describe_inference_profile(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::Guardrail" => {
                self.get_bedrock_service()
                    .describe_guardrail(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::ProvisionedModelThroughput" => {
                self.get_bedrock_service()
                    .describe_provisioned_model_throughput(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::Agent" => {
                self.get_bedrock_agent_service()
                    .describe_agent(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::KnowledgeBase" => {
                self.get_bedrock_agent_service()
                    .describe_knowledge_base(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::CustomModel" => {
                self.get_bedrock_service()
                    .describe_custom_model(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::ImportedModel" => {
                self.get_bedrock_service()
                    .describe_imported_model(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::EvaluationJob" => {
                self.get_bedrock_service()
                    .describe_evaluation_job(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::ModelInvocationJob" => {
                self.get_bedrock_service()
                    .describe_model_invocation_job(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::Prompt" => {
                self.get_bedrock_agent_service()
                    .describe_prompt(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::Flow" => {
                self.get_bedrock_agent_service()
                    .describe_flow(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Bedrock::ModelCustomizationJob" => {
                self.get_bedrock_service()
                    .describe_model_customization_job(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // BedrockAgentCore - Describe methods
            "AWS::BedrockAgentCore::AgentRuntime" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_agent_runtime(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::AgentRuntimeEndpoint" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_agent_runtime_endpoint(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::Memory" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_memory(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::Gateway" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_gateway(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::Browser" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_browser(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // BedrockAgentCore - Additional Describe methods
            "AWS::BedrockAgentCore::CodeInterpreter" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_code_interpreter(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::ApiKeyCredentialProvider" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_api_key_credential_provider(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::OAuth2CredentialProvider" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_oauth2_credential_provider(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::BedrockAgentCore::WorkloadIdentity" => {
                self.get_bedrock_agentcore_control_service()
                    .describe_workload_identity(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::S3::Bucket" => {
                self.get_s3_service()
                    .describe_bucket(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudFormation::Stack" => {
                self.get_cloudformation_service()
                    .describe_stack(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::RDS::DBInstance" => {
                self.get_rds_service()
                    .describe_db_instance(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::RDS::DBCluster" => {
                self.get_rds_service()
                    .describe_db_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::RDS::DBSnapshot" => {
                self.get_rds_service()
                    .describe_db_snapshot(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Lambda::Function" => {
                self.get_lambda_service()
                    .describe_function(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::DynamoDB::Table" => {
                self.get_dynamodb_service()
                    .describe_table(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudWatch::Alarm" => {
                self.get_cloudwatch_service()
                    .describe_alarm(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudWatch::CompositeAlarm"
            | "AWS::CloudWatch::Metric"
            | "AWS::CloudWatch::InsightRule"
            | "AWS::CloudWatch::AnomalyDetector"
            | "AWS::Logs::LogStream"
            | "AWS::Logs::MetricFilter"
            | "AWS::Logs::SubscriptionFilter"
            | "AWS::Logs::ResourcePolicy"
            | "AWS::Logs::QueryDefinition" => Ok(resource.raw_properties.clone()),
            "AWS::ApiGateway::RestApi" => {
                self.get_apigateway_service()
                    .describe_rest_api(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ApiGatewayV2::Api" => {
                self.get_apigatewayv2_service()
                    .describe_api(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SNS::Topic" => {
                self.get_sns_service()
                    .describe_topic(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SQS::Queue" => {
                self.get_sqs_service()
                    .describe_queue(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ECS::Cluster" => {
                self.get_ecs_service()
                    .describe_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::EKS::Cluster" => {
                self.get_eks_service()
                    .describe_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Logs::LogGroup" => {
                self.get_logs_service()
                    .describe_log_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Kinesis::Stream" => {
                self.get_kinesis_service()
                    .describe_stream(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SageMaker::Endpoint" => {
                self.get_sagemaker_service()
                    .describe_endpoint(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Redshift::Cluster" => {
                let cluster_identifier = resource
                    .raw_properties
                    .get("ClusterIdentifier")
                    .or_else(|| resource.raw_properties.get("Name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_redshift_service()
                    .get_cluster_details(&resource.account_id, &resource.region, cluster_identifier)
                    .await
            }
            "AWS::Glue::Job" => {
                let job_name = resource
                    .raw_properties
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_glue_service()
                    .get_job_details(&resource.account_id, &resource.region, job_name)
                    .await
            }
            "AWS::LakeFormation::DataLakeSettings" => {
                self.get_lakeformation_service()
                    .describe_data_lake_settings(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Athena::WorkGroup" => {
                self.get_athena_service()
                    .describe_work_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ECR::Repository" => {
                self.get_ecr_service()
                    .describe_repository(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::EMR::Cluster" => {
                self.get_emr_service()
                    .describe_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SecretsManager::Secret" => {
                self.get_secretsmanager_service()
                    .describe_secret(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::KMS::Key" => {
                self.get_kms_service()
                    .describe_key(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::StepFunctions::StateMachine" => {
                let state_machine_arn = resource
                    .raw_properties
                    .get("StateMachineArn")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_stepfunctions_service()
                    .get_state_machine_details(
                        &resource.account_id,
                        &resource.region,
                        state_machine_arn,
                    )
                    .await
            }
            "AWS::Route53::HostedZone" => {
                self.get_route53_service()
                    .describe_hosted_zone(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::EFS::FileSystem" => {
                self.get_efs_service()
                    .describe_file_system(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudTrail::Trail" => {
                self.get_cloudtrail_service()
                    .describe_trail(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudTrail::Event" => {
                // For CloudTrail Events, we already have all the details in raw_properties
                // Return the event details as-is since lookup_events provides comprehensive data
                Ok(resource.raw_properties.clone())
            }
            "AWS::Config::ConfigurationRecorder" => {
                self.get_config_service()
                    .describe_configuration_recorder(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Config::ConfigRule" => {
                self.get_config_service()
                    .describe_config_rule(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::DataBrew::Job" => {
                self.get_databrew_service()
                    .describe_job(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::DataBrew::Dataset" => {
                self.get_databrew_service()
                    .describe_dataset(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CodeArtifact::Domain" => {
                self.get_codeartifact_service()
                    .describe_domain(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CodeArtifact::Repository" => {
                self.get_codeartifact_service()
                    .describe_repository(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CodeDeploy::Application" => {
                self.get_codedeploy_service()
                    .describe_application(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CodeDeploy::DeploymentGroup" => {
                self.get_codedeploy_service()
                    .describe_deployment_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AppConfig::Application" => {
                self.get_appconfig_service()
                    .describe_application(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AppConfig::Environment" => {
                self.get_appconfig_service()
                    .describe_environment(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AppConfig::ConfigurationProfile" => {
                self.get_appconfig_service()
                    .describe_configuration_profile(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudTrail::EventDataStore" => {
                self.get_cloudtraildata_service()
                    .describe_event_data_store(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // High-value AWS services
            "AWS::CertificateManager::Certificate" => {
                self.get_acm_service()
                    .describe_certificate(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::WAFv2::WebACL" => {
                self.get_wafv2_service()
                    .get_web_acl(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CloudFront::Distribution" => {
                self.get_cloudfront_service()
                    .describe_distribution(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ElastiCache::CacheCluster" => {
                self.get_elasticache_service()
                    .describe_cache_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ElastiCache::ReplicationGroup" => {
                self.get_elasticache_service()
                    .describe_replication_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ElastiCache::ParameterGroup" => {
                self.get_elasticache_service()
                    .describe_cache_parameter_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::GuardDuty::Detector" => {
                self.get_guardduty_service()
                    .describe_detector(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SecurityHub::Hub" => {
                self.get_securityhub_service()
                    .describe_hub(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Detective::Graph" => {
                self.get_detective_service()
                    .describe_graph(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AccessAnalyzer::Analyzer" => {
                self.get_accessanalyzer_service()
                    .describe_analyzer(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // Analytics & search services
            "AWS::OpenSearchService::Domain" => {
                let domain_name = resource
                    .raw_properties
                    .get("DomainName")
                    .or_else(|| resource.raw_properties.get("Name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_opensearch_service()
                    .get_domain_details(&resource.account_id, &resource.region, domain_name)
                    .await
            }
            "AWS::QuickSight::DataSource" => {
                self.get_quicksight_service()
                    .describe_data_source(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::QuickSight::Dashboard" => {
                self.get_quicksight_service()
                    .describe_dashboard(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::QuickSight::DataSet" => {
                self.get_quicksight_service()
                    .describe_data_set(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Backup::BackupPlan" => {
                let backup_plan_id = resource
                    .raw_properties
                    .get("BackupPlanId")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_backup_service()
                    .get_backup_plan_details(&resource.account_id, &resource.region, backup_plan_id)
                    .await
            }
            "AWS::Backup::BackupVault" => {
                let vault_name = resource
                    .raw_properties
                    .get("BackupVaultName")
                    .or_else(|| resource.raw_properties.get("Name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_backup_service()
                    .get_backup_vault_details(&resource.account_id, &resource.region, vault_name)
                    .await
            }
            // Identity & messaging services
            "AWS::Cognito::UserPool" => {
                self.get_cognito_service()
                    .describe_user_pool(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Cognito::IdentityPool" => {
                self.get_cognito_service()
                    .describe_identity_pool(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::MQ::Broker" => {
                self.get_mq_service()
                    .describe_broker(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Organizations::Account" => {
                self.get_organizations_service()
                    .describe_account(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Organizations::DelegatedAdministrator" => {
                // DelegatedAdministrator doesn't have a describe operation - list returns full details
                // Return the raw_properties as detailed_properties
                Ok(resource.raw_properties.clone())
            }
            "AWS::Organizations::Handshake" => {
                self.get_organizations_service()
                    .describe_handshake(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Organizations::CreateAccountStatus" => {
                self.get_organizations_service()
                    .describe_create_account_status(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Organizations::AwsServiceAccess" => {
                // AwsServiceAccess doesn't have a describe operation - list returns full details
                // Return the raw_properties as detailed_properties
                Ok(resource.raw_properties.clone())
            }
            "AWS::Organizations::Organization" => {
                self.get_organizations_service()
                    .describe_organization(&resource.account_id, &resource.region)
                    .await
            }
            "AWS::Organizations::OrganizationalUnit" => {
                self.get_organizations_service()
                    .describe_organizational_unit(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Organizations::Policy" => {
                self.get_organizations_service()
                    .describe_policy(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Organizations::Root" => {
                // Roots don't have a describe operation - list returns full details
                // Return the raw_properties as detailed_properties
                Ok(resource.raw_properties.clone())
            }
            // Load balancing & networking services
            "AWS::ElasticLoadBalancing::LoadBalancer" => {
                self.get_elb_service()
                    .describe_load_balancer(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ElasticLoadBalancingV2::LoadBalancer" => {
                self.get_elbv2_service()
                    .describe_load_balancer(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ElasticLoadBalancingV2::TargetGroup" => {
                self.get_elbv2_service()
                    .describe_target_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SSM::Parameter" => {
                self.get_ssm_service()
                    .describe_parameter(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SSM::Document" => {
                self.get_ssm_service()
                    .describe_document(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // DevOps & CI/CD services
            "AWS::CodePipeline::Pipeline" => {
                self.get_codepipeline_service()
                    .describe_pipeline(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CodeBuild::Project" => {
                self.get_codebuild_service()
                    .describe_project(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::CodeCommit::Repository" => {
                self.get_codecommit_service()
                    .describe_repository(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Events::EventBus" => {
                self.get_eventbridge_service()
                    .describe_event_bus(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Events::Rule" => {
                self.get_eventbridge_service()
                    .describe_rule(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // IoT & App services
            "AWS::AppSync::GraphQLApi" => {
                self.get_appsync_service()
                    .describe_graphql_api(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::IoT::Thing" => {
                self.get_iot_service()
                    .describe_thing(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Greengrass::ComponentVersion" => {
                self.get_greengrass_service()
                    .describe_component_version(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::ACMPCA::CertificateAuthority" => {
                self.get_acmpca_service()
                    .describe_certificate_authority(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AutoScaling::AutoScalingGroup" => {
                self.get_autoscaling_service()
                    .describe_auto_scaling_group(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // Compute & Data services
            "AWS::Neptune::DBCluster" => {
                self.get_neptune_service()
                    .describe_db_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Batch::JobQueue" => {
                self.get_batch_service()
                    .describe_job_queue(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Batch::ComputeEnvironment" => {
                self.get_batch_service()
                    .describe_compute_environment(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::KinesisFirehose::DeliveryStream" => {
                self.get_kinesisfirehose_service()
                    .describe_delivery_stream(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::MSK::Cluster" => {
                self.get_msk_service()
                    .describe_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Macie::Session" => {
                self.get_macie_service()
                    .get_macie_session(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Inspector::Configuration" => {
                self.get_inspector_service()
                    .get_inspector_configuration(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Timestream::Database" => {
                self.get_timestream_service()
                    .get_timestream_service(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::DocumentDB::Cluster" => {
                self.get_documentdb_service()
                    .get_cluster_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Transfer::Server" => {
                self.get_transfer_service()
                    .describe_server(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::DataSync::Task" => {
                self.get_datasync_service()
                    .describe_task(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::FSx::FileSystem" => {
                self.get_fsx_service()
                    .describe_file_system(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::FSx::Backup" => {
                self.get_fsx_service()
                    .describe_backup(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::WorkSpaces::Workspace" => {
                self.get_workspaces_service()
                    .describe_workspace(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::WorkSpaces::Directory" => {
                self.get_workspaces_service()
                    .describe_directory(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::XRay::SamplingRule" => {
                self.get_xray_service()
                    .describe_sampling_rule(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Shield::Protection" => {
                self.get_shield_service()
                    .describe_protection(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AppRunner::Service" => {
                self.get_apprunner_service()
                    .describe_service(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::AppRunner::Connection" => {
                // App Runner connections don't have detailed describe operations
                // Return the raw properties as detailed info
                Ok(resource.raw_properties.clone())
            }
            "AWS::GlobalAccelerator::Accelerator" => {
                self.get_globalaccelerator_service()
                    .describe_accelerator(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Connect::Instance" => {
                self.get_connect_service()
                    .describe_instance(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Amplify::App" => {
                self.get_amplify_service()
                    .get_app(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Lex::Bot" => {
                self.get_lex_service()
                    .describe_bot(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Rekognition::Collection" => {
                self.get_rekognition_service()
                    .describe_collection(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Rekognition::StreamProcessor" => {
                self.get_rekognition_service()
                    .describe_stream_processor(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Polly::Lexicon" => {
                self.get_polly_service()
                    .get_lexicon(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Polly::SynthesisTask" => {
                self.get_polly_service()
                    .get_speech_synthesis_task(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Polly::Voice" => {
                // Voices don't have individual describe operations
                Ok(resource.raw_properties.clone())
            }
            _ => Err(anyhow::anyhow!(
                "Describe operation not supported for resource type: {}",
                resource.resource_type
            )),
        }
    }

    /// Start Phase 2 enrichment for resources that support detail fetching
    ///
    /// This method spawns a background task that fetches detailed information
    /// for each resource and sends updates via the result_sender channel.
    /// The cache is automatically updated with enriched resources.
    pub fn start_phase2_enrichment(
        &self,
        resources: Vec<ResourceEntry>,
        _result_sender: mpsc::Sender<QueryResult>,
        progress_sender: Option<mpsc::Sender<QueryProgress>>,
        cache: Arc<SharedResourceCache>,
    ) {
        let client = self.clone();

        tokio::spawn(async move {
            // Filter resources that support Phase 2 enrichment
            let enrichable_types = [
                "AWS::Lambda::Function",
                "AWS::KMS::Key",
                "AWS::IAM::Role",
                "AWS::IAM::User",
                "AWS::IAM::Policy",
                "AWS::S3::Bucket",
                "AWS::SQS::Queue",
                "AWS::SNS::Topic",
                "AWS::Cognito::UserPool",
                "AWS::Cognito::IdentityPool",
                "AWS::CodeCommit::Repository",
                "AWS::DynamoDB::Table",
                "AWS::CloudFormation::Stack",
                "AWS::ECS::Cluster",
                "AWS::ECS::Service",
                "AWS::ElasticLoadBalancingV2::LoadBalancer",
                "AWS::EMR::Cluster",
                "AWS::Events::EventBus",
                "AWS::Glue::Job",
                "AWS::Backup::BackupPlan",
                "AWS::Backup::BackupVault",
                "AWS::StepFunctions::StateMachine",
                "AWS::OpenSearchService::Domain",
                "AWS::Redshift::Cluster",
            ];

            let resources_to_enrich: Vec<_> = resources
                .into_iter()
                .filter(|r| enrichable_types.contains(&r.resource_type.as_str()))
                .collect();

            if resources_to_enrich.is_empty() {
                log_query_event("PHASE2: No resources to enrich");
                return;
            }

            let total = resources_to_enrich.len();
            let _phase2_timer = QueryTimer::new("PHASE2", &format!("{} resources to enrich", total));
            info!("Starting Phase 2 enrichment for {} resources (parallel)", total);

            // Send enrichment started progress
            if let Some(ref sender) = progress_sender {
                let _ = sender
                    .send(QueryProgress {
                        account: "All".to_string(),
                        region: "All".to_string(),
                        resource_type: "Phase 2 Enrichment".to_string(),
                        status: QueryStatus::EnrichmentStarted,
                        message: format!("Starting detail fetch for {} resources", total),
                        items_processed: Some(0),
                        estimated_total: Some(total),
                    })
                    .await;
            }

            // Log current cache state before enrichment
            {
                let cache_keys = cache.resource_keys();
                tracing::info!(
                    "Phase 2: Current cache contains {} keys: {:?}",
                    cache_keys.len(),
                    cache_keys
                );
            }

            // Build list of (cache_key, resource) pairs for parallel processing
            let mut work_items: Vec<(String, ResourceEntry)> = Vec::with_capacity(total);
            for resource in resources_to_enrich {
                // Use "Global" for global services to match cache key format
                let cache_region = if super::global_services::is_global_service(&resource.resource_type) {
                    "Global".to_string()
                } else {
                    resource.region.clone()
                };
                let cache_key = format!(
                    "{}:{}:{}",
                    resource.account_id, cache_region, resource.resource_type
                );
                work_items.push((cache_key, resource));
            }

            // Log summary of cache state at start
            {
                let cache_keys = cache.resource_keys();
                let mut total_resources = 0usize;
                let mut total_with_details = 0usize;
                for key in &cache_keys {
                    if let Some(resources) = cache.get_resources_owned(key) {
                        total_resources += resources.len();
                        total_with_details += resources.iter().filter(|r| r.detailed_properties.is_some()).count();
                    }
                }
                tracing::info!(
                    "Phase 2 START: {} cache keys, {} total resources, {} already enriched",
                    cache_keys.len(), total_resources, total_with_details
                );
            }

            // Use semaphore to limit concurrent API calls (similar to Phase 1)
            let semaphore = Arc::new(Semaphore::new(20));
            let processed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let updated_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let failed_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            // Create parallel futures for all resources
            let mut futures: FuturesUnordered<_> = work_items
                .into_iter()
                .map(|(cache_key, resource)| {
                    let client = client.clone();
                    let semaphore = semaphore.clone();
                    let cache = cache.clone();
                    let progress_sender = progress_sender.clone();
                    let processed = processed.clone();
                    let updated_count = updated_count.clone();
                    let failed_count = failed_count.clone();

                    async move {
                        // Acquire semaphore permit
                        let _permit = semaphore.acquire().await.expect("Semaphore closed");

                        let resource_id = resource.resource_id.clone();
                        let resource_type = resource.resource_type.clone();
                        let account_id = resource.account_id.clone();
                        let region = resource.region.clone();

                        // Fetch details
                        let details_result = client.fetch_resource_details(&resource).await;

                        match details_result {
                            Ok(details) => {
                                tracing::debug!(
                                    "Phase 2: Got details for {} ({})",
                                    resource_id,
                                    resource_type
                                );

                                // Update cache with enriched resource (using SharedResourceCache)
                                // Need to read, modify, and write back since we can't get_mut
                                if let Some(mut cached_resources) = cache.get_resources_owned(&cache_key) {
                                    if let Some(cached) = cached_resources
                                        .iter_mut()
                                        .find(|r| r.resource_id == resource_id)
                                    {
                                        let merged = Self::merge_properties(
                                            &cached.raw_properties,
                                            &details,
                                        );
                                        let timestamp = Utc::now();

                                        // Store detailed properties in BOTH places during migration:
                                        // 1. Legacy: ResourceEntry.detailed_properties
                                        cached.detailed_properties = Some(merged.clone());
                                        cached.detailed_timestamp = Some(timestamp);

                                        // 2. New: Separate detailed_properties cache
                                        let detailed_key = super::cache::SharedResourceCache::resource_key(cached);
                                        cache.insert_detailed(
                                            detailed_key,
                                            super::cache::DetailedData {
                                                properties: merged,
                                                timestamp,
                                            },
                                        );

                                        // Write back the modified list
                                        cache.insert_resources_owned(cache_key.clone(), cached_resources);

                                        updated_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        tracing::debug!(
                                            "Phase 2: Updated cache for {} in key {}",
                                            resource_id,
                                            cache_key
                                        );
                                    } else {
                                        failed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        tracing::warn!(
                                            "Phase 2: Resource {} not found in cache under key {}",
                                            resource_id,
                                            cache_key
                                        );
                                    }
                                } else {
                                    failed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    tracing::warn!(
                                        "Phase 2: Cache key {} not found",
                                        cache_key
                                    );
                                }
                            }
                            Err(e) => {
                                failed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                tracing::warn!(
                                    "Phase 2: Failed to fetch details for {} ({}): {}",
                                    resource_id,
                                    resource_type,
                                    e
                                );
                            }
                        }

                        let current_processed = processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                        // Send progress update periodically (every 5 resources or at completion)
                        if current_processed % 5 == 0 || current_processed == total {
                            if let Some(ref sender) = progress_sender {
                                let _ = sender
                                    .send(QueryProgress {
                                        account: account_id,
                                        region,
                                        resource_type: "Phase 2 Enrichment".to_string(),
                                        status: QueryStatus::EnrichmentInProgress,
                                        message: format!("Enriched {}/{} resources", current_processed, total),
                                        items_processed: Some(current_processed),
                                        estimated_total: Some(total),
                                    })
                                    .await;
                            }
                        }
                    }
                })
                .collect();

            // Process all futures
            while futures.next().await.is_some() {}

            let final_processed = processed.load(std::sync::atomic::Ordering::Relaxed);
            let final_updated = updated_count.load(std::sync::atomic::Ordering::Relaxed);
            let final_failed = failed_count.load(std::sync::atomic::Ordering::Relaxed);

            // Log summary of cache state at end
            {
                let cache_keys = cache.resource_keys();
                let mut total_resources = 0usize;
                let mut total_with_details = 0usize;
                for key in &cache_keys {
                    if let Some(resources) = cache.get_resources_owned(key) {
                        total_resources += resources.len();
                        total_with_details += resources.iter().filter(|r| r.detailed_properties.is_some()).count();
                    }
                }
                tracing::info!(
                    "Phase 2 END: {} total resources, {} now enriched (processed={}, updated={}, failed={})",
                    total_resources, total_with_details, final_processed, final_updated, final_failed
                );
            }

            // Send enrichment completed progress
            if let Some(ref sender) = progress_sender {
                let _ = sender
                    .send(QueryProgress {
                        account: "All".to_string(),
                        region: "All".to_string(),
                        resource_type: "Phase 2 Enrichment".to_string(),
                        status: QueryStatus::EnrichmentCompleted,
                        message: format!("Completed detail fetch for {} resources", total),
                        items_processed: Some(total),
                        estimated_total: Some(total),
                    })
                    .await;
            }

            info!("Phase 2 enrichment completed for {} resources (parallel)", total);
        });
    }

    /// Fetch detailed information for a single resource (Phase 2)
    async fn fetch_resource_details(&self, resource: &ResourceEntry) -> Result<serde_json::Value> {
        match resource.resource_type.as_str() {
            "AWS::Lambda::Function" => {
                self.get_lambda_service()
                    .get_function_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::KMS::Key" => {
                self.get_kms_service()
                    .get_key_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::IAM::Role" => {
                self.get_iam_service()
                    .get_role_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::IAM::User" => {
                self.get_iam_service()
                    .get_user_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::IAM::Policy" => {
                // For policies, the resource_id is the policy ARN
                self.get_iam_service()
                    .get_policy_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::S3::Bucket" => {
                self.get_s3_service()
                    .get_bucket_details(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::SQS::Queue" => {
                // For SQS, resource_id might be the queue URL or name
                // The get_queue_details expects queue_url
                let queue_url = if resource.resource_id.starts_with("https://") {
                    resource.resource_id.clone()
                } else {
                    // Try to get queue URL from raw_properties
                    resource
                        .raw_properties
                        .get("QueueUrl")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&resource.resource_id)
                        .to_string()
                };
                self.get_sqs_service()
                    .get_queue_details(&resource.account_id, &resource.region, &queue_url)
                    .await
            }
            "AWS::SNS::Topic" => {
                // For SNS, resource_id might be the topic ARN
                let topic_arn = if resource.resource_id.starts_with("arn:") {
                    resource.resource_id.clone()
                } else {
                    resource
                        .raw_properties
                        .get("TopicArn")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&resource.resource_id)
                        .to_string()
                };
                self.get_sns_service()
                    .get_topic_details(&resource.account_id, &resource.region, &topic_arn)
                    .await
            }
            "AWS::Cognito::UserPool" => {
                // For Cognito, resource_id is the pool ID
                let pool_id = resource
                    .raw_properties
                    .get("Id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_cognito_service()
                    .get_user_pool_details(&resource.account_id, &resource.region, pool_id)
                    .await
            }
            "AWS::Cognito::IdentityPool" => {
                let pool_id = resource
                    .raw_properties
                    .get("IdentityPoolId")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_cognito_service()
                    .get_identity_pool_details(&resource.account_id, &resource.region, pool_id)
                    .await
            }
            "AWS::CodeCommit::Repository" => {
                let repo_name = resource
                    .raw_properties
                    .get("RepositoryName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_codecommit_service()
                    .get_repository_details(&resource.account_id, &resource.region, repo_name)
                    .await
            }
            "AWS::DynamoDB::Table" => {
                let table_name = resource
                    .raw_properties
                    .get("TableName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_dynamodb_service()
                    .get_table_details(&resource.account_id, &resource.region, table_name)
                    .await
            }
            "AWS::CloudFormation::Stack" => {
                let stack_name = resource
                    .raw_properties
                    .get("StackName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_cloudformation_service()
                    .get_stack_details(&resource.account_id, &resource.region, stack_name)
                    .await
            }
            "AWS::ECS::Cluster" => {
                let cluster_name = resource
                    .raw_properties
                    .get("ClusterName")
                    .or_else(|| resource.raw_properties.get("Name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_ecs_service()
                    .get_cluster_details(&resource.account_id, &resource.region, cluster_name)
                    .await
            }
            "AWS::ECS::Service" => {
                // For ECS services, we need the service ARN for full details
                let service_arn = resource
                    .raw_properties
                    .get("ServiceArn")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_ecs_service()
                    .get_service_details(&resource.account_id, &resource.region, service_arn)
                    .await
            }
            "AWS::ElasticLoadBalancingV2::LoadBalancer" => {
                let lb_arn = resource
                    .raw_properties
                    .get("LoadBalancerArn")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_elbv2_service()
                    .get_load_balancer_details(&resource.account_id, &resource.region, lb_arn)
                    .await
            }
            "AWS::EMR::Cluster" => {
                let cluster_id = resource
                    .raw_properties
                    .get("ClusterId")
                    .or_else(|| resource.raw_properties.get("Id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_emr_service()
                    .get_cluster_details(&resource.account_id, &resource.region, cluster_id)
                    .await
            }
            "AWS::Events::EventBus" => {
                let event_bus_name = resource
                    .raw_properties
                    .get("Name")
                    .or_else(|| resource.raw_properties.get("EventBusName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resource.resource_id);
                self.get_eventbridge_service()
                    .get_event_bus_details(&resource.account_id, &resource.region, event_bus_name)
                    .await
            }
            _ => Err(anyhow::anyhow!(
                "Phase 2 enrichment not supported for resource type: {}",
                resource.resource_type
            )),
        }
    }

    /// Merge Phase 2 enrichment details with Phase 1 raw_properties
    ///
    /// This creates a combined JSON object that includes both the original
    /// resource properties from Phase 1 (list operations) and the detailed
    /// properties from Phase 2 (describe operations).
    fn merge_properties(
        raw_properties: &serde_json::Value,
        enrichment_details: &serde_json::Value,
    ) -> serde_json::Value {
        // Start with a clone of raw_properties
        let mut merged = raw_properties.clone();

        // If both are objects, merge the enrichment details directly into raw_properties
        // Phase 2 fields are added at the top level alongside Phase 1 fields
        if let (Some(merged_obj), Some(details_obj)) =
            (merged.as_object_mut(), enrichment_details.as_object())
        {
            // Merge Phase 2 fields directly at the top level
            // This allows verification and display to find fields without nested lookup
            for (key, value) in details_obj {
                merged_obj.insert(key.clone(), value.clone());
            }
        } else if enrichment_details.is_object() {
            // raw_properties isn't an object but enrichment is - use enrichment as base
            merged = enrichment_details.clone();
        }
        // If enrichment isn't an object, just return raw_properties as-is

        merged
    }
}

impl Clone for AWSResourceClient {
    fn clone(&self) -> Self {
        Self {
            normalizer_factory: NormalizerFactory,
            credential_coordinator: Arc::clone(&self.credential_coordinator),
            pagination_config: self.pagination_config.clone(),
            tag_cache: Arc::clone(&self.tag_cache),
            // Services are now created lazily instead of pre-instantiated
        }
    }
}
