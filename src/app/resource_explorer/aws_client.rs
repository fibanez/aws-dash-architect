use super::{aws_services::*, credentials::*, global_services::*, normalizers::*, state::*};
use anyhow::{Context, Result};
use chrono::Utc;
use futures::future::BoxFuture;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
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
}

pub struct AWSResourceClient {
    #[allow(dead_code)]
    normalizer_factory: NormalizerFactory,
    credential_coordinator: Arc<CredentialCoordinator>,
    pagination_config: PaginationConfig,
    // Services are now created lazily instead of pre-instantiated
}

impl AWSResourceClient {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            normalizer_factory: NormalizerFactory,
            credential_coordinator,
            pagination_config: PaginationConfig::default(),
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

    fn get_cloudtrail_service(&self) -> CloudTrailService {
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

    // Phase 2 Batch 1: High-value services
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

    // Phase 2 Batch 2: Analytics & search services
    fn get_opensearch_service(&self) -> OpenSearchService {
        OpenSearchService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_quicksight_service(&self) -> QuickSightService {
        QuickSightService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_backup_service(&self) -> BackupService {
        BackupService::new(Arc::clone(&self.credential_coordinator))
    }

    // Phase 2 Batch 3: Identity & messaging services
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

    // Phase 2 Batch 4: Load balancing & networking services
    fn get_elb_service(&self) -> ELBService {
        ELBService::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_elbv2_service(&self) -> ELBv2Service {
        ELBv2Service::new(Arc::clone(&self.credential_coordinator))
    }

    fn get_ssm_service(&self) -> SSMService {
        SSMService::new(Arc::clone(&self.credential_coordinator))
    }

    // Phase 2 Batch 5: DevOps & CI/CD services
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

    // Phase 2 Batch 6: IoT & App services
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

    // Phase 2 Batch 7: Compute & Data services
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

    /// Query AWS resources for all combinations of accounts, regions, and resource types in parallel
    /// Results are sent back as they arrive via the progress_sender channel
    pub async fn query_aws_resources_parallel(
        &self,
        scope: &QueryScope,
        result_sender: mpsc::Sender<QueryResult>,
        progress_sender: Option<mpsc::Sender<QueryProgress>>,
        cache: Arc<tokio::sync::RwLock<HashMap<String, Vec<ResourceEntry>>>>,
    ) -> Result<()> {
        info!(
            "Starting parallel AWS resource queries for {} accounts, {} regions, {} resource types",
            scope.accounts.len(),
            scope.regions.len(),
            scope.resource_types.len()
        );

        // Create semaphore to limit concurrent requests
        let semaphore = Arc::new(Semaphore::new(
            self.pagination_config.max_concurrent_requests,
        ));

        // Create futures for all combinations
        let mut futures: FuturesUnordered<BoxFuture<'static, ()>> = FuturesUnordered::new();
        let mut total_queries = 0;
        
        // Track which global services have been queried per account to avoid duplicates
        let mut queried_global_services: HashSet<(String, String)> = HashSet::new();
        let global_registry = GlobalServiceRegistry::new();

        for account in &scope.accounts {
            for resource_type in &scope.resource_types {
                // Check if this is a global service
                if global_registry.is_global(&resource_type.resource_type) {
                    // For global services, only query once per account
                    let global_key = (account.account_id.clone(), resource_type.resource_type.clone());
                    
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
                    
                    // Check cache first
                    {
                        let cache_read = cache.read().await;
                        if let Some(cached_resources) = cache_read.get(&cache_key) {
                            info!("Using cached global resources for {}", cache_key);

                            // Send cached result immediately
                            let cached_result = QueryResult {
                                account_id: account.account_id.clone(),
                                region: "Global".to_string(),
                                resource_type: resource_type.resource_type.clone(),
                                resources: Ok(cached_resources.clone()),
                                cache_key: cache_key.clone(),
                            };

                            if let Err(e) = result_sender.send(cached_result).await {
                                warn!("Failed to send cached global result: {}", e);
                            }
                            continue;
                        }
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
                        // Acquire semaphore permit
                        let _permit = semaphore_clone.acquire().await.unwrap();

                        // Send start progress for global service
                        if let Some(sender) = &progress_sender_clone {
                            let _ = sender
                                .send(QueryProgress {
                                    account: account_id.clone(),
                                    region: "Global".to_string(),
                                    resource_type: resource_type_str.clone(),
                                    status: QueryStatus::Started,
                                    message: format!(
                                        "Querying global service {}",
                                        display_name
                                    ),
                                    items_processed: Some(0),
                                    estimated_total: None,
                                })
                                .await;
                        }

                        // Execute the query from the global region
                        let query_result = client
                            .query_resource_type(&account_id, &query_region, &resource_type_str)
                            .await;

                        // Handle the result and transform to Global region
                        let resources_result = match query_result {
                            Ok(mut resources) => {
                                // Mark all resources as Global region
                                for resource in &mut resources {
                                    resource.region = "Global".to_string();
                                }
                                
                                let resource_count = resources.len();
                                info!(
                                    "Global service query completed: {} resources for {}",
                                    resource_count, cache_key_clone
                                );

                                // Cache the results
                                let mut cache_write = cache_clone.write().await;
                                cache_write.insert(cache_key_clone.clone(), resources.clone());

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
                            account_id,
                            region: "Global".to_string(),
                            resource_type: resource_type_str,
                            resources: resources_result,
                            cache_key: cache_key_clone,
                        };

                        if let Err(e) = result_sender_clone.send(result).await {
                            warn!("Failed to send global query result: {}", e);
                        }
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

                    // Check cache first
                    {
                        let cache_read = cache.read().await;
                        if let Some(cached_resources) = cache_read.get(&cache_key) {
                            info!("Using cached resources for {}", cache_key);

                            // Send cached result immediately
                            let cached_result = QueryResult {
                                account_id: account.account_id.clone(),
                                region: region.region_code.clone(),
                                resource_type: resource_type.resource_type.clone(),
                                resources: Ok(cached_resources.clone()),
                                cache_key: cache_key.clone(),
                            };

                            if let Err(e) = result_sender.send(cached_result).await {
                                warn!("Failed to send cached result: {}", e);
                            }
                            continue;
                        }
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
                        // Acquire semaphore permit
                        let _permit = semaphore_clone.acquire().await.unwrap();

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
                        let query_result = client
                            .query_resource_type(&account_id, &region_code, &resource_type_str)
                            .await;

                        // Handle the result
                        let resources_result = match query_result {
                            Ok(resources) => {
                                let resource_count = resources.len();
                                info!(
                                    "Parallel query completed: {} resources for {}",
                                    resource_count, cache_key_clone
                                );

                                // Cache the results
                                {
                                    let mut cache_write = cache_clone.write().await;
                                    cache_write.insert(cache_key_clone.clone(), resources.clone());
                                }

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
                            account_id,
                            region: region_code,
                            resource_type: resource_type_str,
                            resources: resources_result,
                            cache_key: cache_key_clone,
                        };

                        if let Err(e) = result_sender_clone.send(query_result).await {
                            warn!("Failed to send query result: {}", e);
                        }
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
        while (futures.next().await).is_some() {
            // Results are sent via the result_sender channel as they complete
        }

        info!("All parallel queries completed");
        Ok(())
    }

    /// Legacy sequential method for backward compatibility
    pub async fn query_aws_resources(
        &self,
        scope: &QueryScope,
        progress_sender: Option<mpsc::Sender<QueryProgress>>,
        cache: &mut HashMap<String, Vec<ResourceEntry>>,
    ) -> Result<Vec<ResourceEntry>> {
        // Convert to Arc<RwLock> for the new method
        let cache_arc = Arc::new(tokio::sync::RwLock::new(cache.clone()));

        // Create channels for results
        let (result_sender, mut result_receiver) = mpsc::channel::<QueryResult>(1000);

        // Start parallel queries
        let query_future = self.query_aws_resources_parallel(
            scope,
            result_sender,
            progress_sender,
            cache_arc.clone(),
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

        // Update the original cache
        let final_cache = cache_arc.read().await;
        *cache = final_cache.clone();

        // Extract relationships between resources
        self.extract_all_relationships(&mut all_resources);

        Ok(all_resources)
    }

    /// Query a specific resource type for a given account and region
    async fn query_resource_type(
        &self,
        account: &str,
        region: &str,
        resource_type: &str,
    ) -> Result<Vec<ResourceEntry>> {
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
            "AWS::IAM::Role" => self.get_iam_service().list_roles(account, region).await?,
            "AWS::IAM::User" => self.get_iam_service().list_users(account, region).await?,
            "AWS::IAM::Policy" => {
                self.get_iam_service()
                    .list_policies(account, region)
                    .await?
            }
            "AWS::Bedrock::Model" => {
                self.get_bedrock_service()
                    .list_foundation_models(account, region)
                    .await?
            }
            "AWS::S3::Bucket" => self.get_s3_service().list_buckets(account, region).await?,
            "AWS::CloudFormation::Stack" => {
                self.get_cloudformation_service()
                    .list_stacks(account, region)
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
            "AWS::Lambda::Function" => {
                self.get_lambda_service()
                    .list_functions(account, region)
                    .await?
            }
            "AWS::DynamoDB::Table" => {
                self.get_dynamodb_service()
                    .list_tables(account, region)
                    .await?
            }
            "AWS::CloudWatch::Alarm" => {
                self.get_cloudwatch_service()
                    .list_alarms(account, region)
                    .await?
            }
            "AWS::ApiGateway::RestApi" => {
                self.get_apigateway_service()
                    .list_rest_apis(account, region)
                    .await?
            }
            "AWS::SNS::Topic" => self.get_sns_service().list_topics(account, region).await?,
            "AWS::SQS::Queue" => self.get_sqs_service().list_queues(account, region).await?,
            "AWS::ECS::Cluster" => {
                self.get_ecs_service()
                    .list_clusters(account, region)
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
                    .list_clusters(account, region)
                    .await?
            }
            "AWS::Glue::Job" => self.get_glue_service().list_jobs(account, region).await?,
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
                    .list_clusters(account, region)
                    .await?
            }
            "AWS::SecretsManager::Secret" => {
                self.get_secretsmanager_service()
                    .list_secrets(account, region)
                    .await?
            }
            "AWS::KMS::Key" => self.get_kms_service().list_keys(account, region).await?,
            "AWS::StepFunctions::StateMachine" => {
                self.get_stepfunctions_service()
                    .list_state_machines(account, region)
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
            // Phase 2 Batch 1: High-value services
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
            // Phase 2 Batch 2: Analytics & search services
            "AWS::OpenSearchService::Domain" => {
                self.get_opensearch_service()
                    .list_domains(account, region)
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
                    .list_backup_plans(account, region)
                    .await?
            }
            "AWS::Backup::BackupVault" => {
                self.get_backup_service()
                    .list_backup_vaults(account, region)
                    .await?
            }
            // Phase 2 Batch 3: Identity & messaging services
            "AWS::Cognito::UserPool" => {
                self.get_cognito_service()
                    .list_user_pools(account, region)
                    .await?
            }
            "AWS::Cognito::IdentityPool" => {
                self.get_cognito_service()
                    .list_identity_pools(account, region)
                    .await?
            }
            "AWS::MQ::Broker" => self.get_mq_service().list_brokers(account, region).await?,
            "AWS::Organizations::Policy" => {
                self.get_organizations_service()
                    .list_policies(account, region)
                    .await?
            }
            "AWS::Organizations::OrganizationalUnit" => {
                self.get_organizations_service()
                    .list_organizational_units(account, region)
                    .await?
            }
            // Phase 2 Batch 4: Load balancing & networking services
            "AWS::ElasticLoadBalancing::LoadBalancer" => {
                self.get_elb_service()
                    .list_load_balancers(account, region)
                    .await?
            }
            "AWS::ElasticLoadBalancingV2::LoadBalancer" => {
                self.get_elbv2_service()
                    .list_load_balancers(account, region)
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
            // Phase 2 Batch 5: DevOps & CI/CD services
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
            "AWS::CodeCommit::Repository" => {
                self.get_codecommit_service()
                    .list_repositories(account, region)
                    .await?
            }
            "AWS::Events::EventBus" => {
                self.get_eventbridge_service()
                    .list_event_buses(account, region)
                    .await?
            }
            "AWS::Events::Rule" => {
                self.get_eventbridge_service()
                    .list_rules(account, region)
                    .await?
            }
            // Phase 2 Batch 6: IoT & App services
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
            // Phase 2 Batch 7: Compute & Data services
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
            "AWS::FSx::Backup" => {
                self.get_fsx_service()
                    .list_backups(account, region)
                    .await?
            }
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
            "AWS::Lex::Bot" => {
                self.get_lex_service()
                    .list_bots(account, region)
                    .await?
            }
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
                return Ok(Vec::new());
            }
        };

        // Normalize the resources
        self.normalize_resources(raw_resources, account, region, resource_type)
    }

    /// Normalize raw AWS API responses into ResourceEntry format
    fn normalize_resources(
        &self,
        raw_resources: Vec<serde_json::Value>,
        account: &str,
        region: &str,
        resource_type: &str,
    ) -> Result<Vec<ResourceEntry>> {
        let normalizer = NormalizerFactory::create_normalizer(resource_type)
            .context("No normalizer available for resource type")?;

        let mut normalized_resources = Vec::new();
        let query_timestamp = Utc::now(); // Capture when this query was executed
        for raw_resource in raw_resources {
            match normalizer.normalize(raw_resource, account, region, query_timestamp) {
                Ok(resource) => normalized_resources.push(resource),
                Err(e) => {
                    warn!("Failed to normalize resource: {}", e);
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
            "AWS::Bedrock::Model" => {
                self.format_bedrock_error(root_error, display_name, account_id, region, role_info)
            }
            _ => {
                // Generic AWS error formatting
                self.format_generic_aws_error(
                    &error_str,
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

    /// Format generic AWS errors
    fn format_generic_aws_error(
        &self,
        error_str: &str,
        display_name: &str,
        account_id: &str,
        region: &str,
        role_info: &str,
    ) -> String {
        // Check for common AWS SDK error patterns
        if error_str.contains("CredentialsNotLoaded") || error_str.contains("NoCredentialsError") {
            format!(
                "Failed to query {} in account {} region {}: Credentials error - {} credentials are invalid or expired",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("TimeoutError") || error_str.contains("timeout") {
            format!(
                "Failed to query {} in account {} region {}: Request timeout - network connectivity issue with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("EndpointResolutionError") {
            format!(
                "Failed to query {} in account {} region {}: Endpoint resolution error - service may not be available in {} with {}",
                display_name, account_id, region, region, role_info
            )
        } else if error_str.contains("DispatchFailure") {
            format!(
                "Failed to query {} in account {} region {}: Network dispatch failure - connectivity issue with {}",
                display_name, account_id, region, role_info
            )
        } else if error_str.contains("ConstructionFailure") {
            format!(
                "Failed to query {} in account {} region {}: Request construction error - invalid request with {}",
                display_name, account_id, region, role_info
            )
        } else {
            format!(
                "Failed to query {} in account {} region {}: {} - {}",
                display_name, account_id, region, error_str, role_info
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
                self.get_redshift_service()
                    .describe_cluster(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Glue::Job" => {
                self.get_glue_service()
                    .describe_job(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
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
                self.get_stepfunctions_service()
                    .describe_state_machine(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
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
            // Phase 2 Batch 1: High-value services
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
            // Phase 2 Batch 2: Analytics & search services
            "AWS::OpenSearchService::Domain" => {
                self.get_opensearch_service()
                    .describe_domain(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
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
                self.get_backup_service()
                    .describe_backup_plan(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            "AWS::Backup::BackupVault" => {
                self.get_backup_service()
                    .describe_backup_vault(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
                    .await
            }
            // Phase 2 Batch 3: Identity & messaging services
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
            "AWS::Organizations::Policy" => {
                self.get_organizations_service()
                    .describe_policy(
                        &resource.account_id,
                        &resource.region,
                        &resource.resource_id,
                    )
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
            // Phase 2 Batch 4: Load balancing & networking services
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
            // Phase 2 Batch 5: DevOps & CI/CD services
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
            // Phase 2 Batch 6: IoT & App services
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
            // Phase 2 Batch 7: Compute & Data services
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
}

impl Clone for AWSResourceClient {
    fn clone(&self) -> Self {
        Self {
            normalizer_factory: NormalizerFactory,
            credential_coordinator: Arc::clone(&self.credential_coordinator),
            pagination_config: self.pagination_config.clone(),
            // Services are now created lazily instead of pre-instantiated
        }
    }
}
