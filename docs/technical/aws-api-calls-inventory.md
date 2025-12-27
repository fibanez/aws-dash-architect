# AWS API Calls Inventory

This document lists all AWS SDK API calls made by the application for each resource type. The purpose is to identify gaps where additional API calls may be needed for complete security and compliance verification.

## Architecture Overview

**Query-side calls** (in `src/app/resource_explorer/aws_services/`):
- Discover and fetch raw resource data via list/describe operations

**Normalizer-side calls** (in `src/app/resource_explorer/normalizers/`):
- Enrich resources with additional details, primarily via tag fetching

**Common Pattern**: Most resources use the Resource Groups Tagging API (`get_resources`) for tag enrichment.

---

## API Calls by Service

### Access Analyzer

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::AccessAnalyzer::Analyzer` | `list_analyzers` | Query | Lists all analyzers |
| `AWS::AccessAnalyzer::Finding` | `list_findings` | Query | Lists findings per analyzer |

**Gaps**: No `get_analyzer` for detailed config, no `get_finding` for finding details.

---

### ACM (Certificate Manager)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CertificateManager::Certificate` | `list_certificates` | Query | Lists certificates |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_certificate` for detailed cert info, expiration dates, validation status.

---

### ACM PCA (Private Certificate Authority)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ACMPCA::CertificateAuthority` | `list_certificate_authorities` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_certificate_authority` for CA details.

---

### Amplify

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Amplify::App` | `list_apps` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `get_app` for detailed configuration.

---

### API Gateway

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ApiGateway::RestApi` | `get_rest_apis` | Query | Lists REST APIs |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `get_rest_api` for detailed API config, no `get_stages`, no `get_resources` (API Gateway method).

---

### API Gateway V2

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ApiGatewayV2::Api` | `get_apis` | Query | Lists HTTP/WebSocket APIs |
| | `get_api` | Query | Detailed API info |

---

### AppConfig

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::AppConfig::Application` | `list_applications` | Query | Paginated |
| `AWS::AppConfig::Environment` | `list_environments` | Query | Per application |
| `AWS::AppConfig::ConfigurationProfile` | `list_configuration_profiles` | Query | Per application |

---

### AppRunner

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::AppRunner::Service` | `list_services` | Query | Lists services |
| `AWS::AppRunner::Connection` | `list_connections` | Query | Lists connections |
| | `get_resources` | Normalizer | Tag enrichment |

---

### AppSync

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::AppSync::GraphQLApi` | `list_graphql_apis` | Query | Lists GraphQL APIs |
| | `get_graphql_api` | Query | Detailed API info |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Athena

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Athena::WorkGroup` | `list_work_groups` | Query | Via paginator |

**Gaps**: No `get_work_group` for detailed workgroup configuration.

---

### Auto Scaling

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::AutoScaling::AutoScalingGroup` | `describe_auto_scaling_groups` | Query | Via paginator |
| `AWS::AutoScaling::ScalingPolicy` | `describe_policies` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

---

### AWS Backup

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Backup::BackupPlan` | `list_backup_plans` | Query (Phase 1) | Via paginator |
| | `get_backup_plan` | Query (Phase 2) | Plan details and rules |
| | `list_backup_selections` | Query (Phase 2) | Resources selected for backup |
| `AWS::Backup::BackupVault` | `list_backup_vaults` | Query (Phase 1) | Via paginator |
| | `describe_backup_vault` | Query (Phase 2) | Vault details |
| | `get_backup_vault_access_policy` | Query (Phase 2) | Vault access policy |
| | `list_recovery_points_by_backup_vault` | Query (Phase 2) | Recovery points in vault |
| | `get_resources` | Normalizer | Tag enrichment |

**Security Details Retrieved**:
- Backup plan rules (schedule, lifecycle, copy actions)
- Backup selection resources and IAM role
- Vault access policies
- Recovery point count and status

---

### AWS Batch

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Batch::ComputeEnvironment` | `describe_compute_environments` | Query | Paginated |
| `AWS::Batch::JobQueue` | `describe_job_queues` | Query | Paginated |

---

### Bedrock

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Bedrock::Model` | `list_foundation_models` | Query | Lists foundation models |
| `AWS::Bedrock::InferenceProfile` | `list_inference_profiles` | Query | Via paginator |
| `AWS::Bedrock::Guardrail` | `list_guardrails` | Query | Via paginator |
| `AWS::Bedrock::ProvisionedModelThroughput` | `list_provisioned_model_throughputs` | Query | Via paginator |
| `AWS::Bedrock::CustomModel` | `list_custom_models` | Query | Via paginator |
| | `get_custom_model` | Query | Detailed model info |
| `AWS::Bedrock::ImportedModel` | `list_imported_models` | Query | Via paginator |
| | `get_imported_model` | Query | Detailed model info |
| `AWS::Bedrock::EvaluationJob` | `list_evaluation_jobs` | Query | Via paginator |
| | `get_evaluation_job` | Query | Detailed job info |
| `AWS::Bedrock::ModelInvocationJob` | `list_model_invocation_jobs` | Query | Via paginator |
| | `get_model_invocation_job` | Query | Detailed job info |
| `AWS::Bedrock::ModelCustomizationJob` | `list_model_customization_jobs` | Query | Via paginator |

---

### Bedrock Agent

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Bedrock::Agent` | `list_agents` | Query | Via paginator |
| | `get_agent` | Query | Detailed agent info |
| `AWS::Bedrock::AgentAlias` | `list_agent_aliases` | Query | Per agent |
| `AWS::Bedrock::AgentActionGroup` | `list_agent_action_groups` | Query | Per agent |
| `AWS::Bedrock::KnowledgeBase` | `list_knowledge_bases` | Query | Via paginator |
| `AWS::Bedrock::DataSource` | `list_data_sources` | Query | Per knowledge base |
| `AWS::Bedrock::IngestionJob` | `list_ingestion_jobs` | Query | Per knowledge base/data source |
| `AWS::Bedrock::Prompt` | `list_prompts` | Query | Via paginator |
| | `get_prompt` | Query | Detailed prompt info |
| `AWS::Bedrock::Flow` | `list_flows` | Query | Via paginator |
| | `get_flow` | Query | Detailed flow info |
| `AWS::Bedrock::FlowAlias` | `list_flow_aliases` | Query | Per flow |

---

### Bedrock Agent Core (Control Plane)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::BedrockAgentCore::AgentRuntime` | `list_agent_runtimes` | Query | Via paginator |
| `AWS::BedrockAgentCore::AgentRuntimeEndpoint` | `list_agent_runtime_endpoints` | Query | Per agent runtime |
| `AWS::BedrockAgentCore::AgentRuntimeVersion` | `list_agent_runtime_versions` | Query | Per agent runtime |
| `AWS::BedrockAgentCore::Memory` | `list_memories` | Query | Via paginator |
| | `get_memory` | Query | Detailed memory info |
| `AWS::BedrockAgentCore::MemoryRecord` | `list_memory_records` | Query | Per memory |
| `AWS::BedrockAgentCore::Gateway` | `list_gateways` | Query | Via paginator |
| `AWS::BedrockAgentCore::GatewayTarget` | `list_gateway_targets` | Query | Per gateway |
| `AWS::BedrockAgentCore::Browser` | `list_browsers` | Query | Via paginator |
| `AWS::BedrockAgentCore::BrowserSession` | `list_browser_sessions` | Query | Per browser |
| `AWS::BedrockAgentCore::CodeInterpreter` | `list_code_interpreters` | Query | Via paginator |
| `AWS::BedrockAgentCore::CodeInterpreterSession` | `list_code_interpreter_sessions` | Query | Per code interpreter |
| `AWS::BedrockAgentCore::Event` | `list_events` | Query | Via paginator |
| `AWS::BedrockAgentCore::WorkloadIdentity` | `list_workload_identities` | Query | Via paginator |
| `AWS::BedrockAgentCore::ApiKeyCredentialProvider` | `list_api_key_credential_providers` | Query | Via paginator |
| `AWS::BedrockAgentCore::OAuth2CredentialProvider` | `list_oauth2_credential_providers` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

---

### CloudFormation

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CloudFormation::Stack` | `describe_stacks` | Query | Via paginator |
| | `get_template` | Query | Optional template retrieval |
| | `get_resources` | Normalizer | Tag enrichment |

---

### CloudFront

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CloudFront::Distribution` | `list_distributions` | Query | Paginated |
| | `get_distribution` | Query | Detailed distribution info |

**Gaps**: Tags require service-specific API, not Resource Groups Tagging.

---

### CloudTrail

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CloudTrail::Trail` | `list_trails` | Query | Lists trails |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::CloudTrail::Event` | (events from trail data) | Query | Event data |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `get_trail` or `describe_trails` for detailed trail configuration, event selectors.

---

### CloudWatch

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CloudWatch::Alarm` | `describe_alarms` | Query | Via paginator |
| `AWS::CloudWatch::Dashboard` | `list_dashboards` | Query | Via paginator |

**Gaps**: No `get_dashboard` for dashboard body/content.

---

### CloudWatch Logs

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Logs::LogGroup` | `describe_log_groups` | Query | Paginated |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_metric_filters`, `describe_subscription_filters` for log group configs.

---

### CodeBuild

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CodeBuild::Project` | `list_projects` | Query | Lists project names |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `batch_get_projects` for detailed project configuration.

---

### CodeCommit

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CodeCommit::Repository` | `list_repositories` | Query | Lists repos |
| | `get_repository` | Query | Detailed repo config |
| | `get_repository_triggers` | Query | Webhook/SNS triggers |
| | `list_branches` | Query | Repository branches |
| | `get_branch` | Query | Branch details |

**Security Details Retrieved**:
- Repository ARN and clone URLs (HTTPS/SSH)
- Default branch and creation date
- Repository triggers (destination ARN, events, branches)
- Branch list and commit IDs

---

### CodeDeploy

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CodeDeploy::Application` | `list_applications` | Query | Paginated |
| `AWS::CodeDeploy::DeploymentGroup` | `list_deployment_groups` | Query | Per application |

**Gaps**: No `get_application`, `get_deployment_group` for detailed configs.

---

### CodePipeline

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::CodePipeline::Pipeline` | `list_pipelines` | Query | Lists pipelines |
| | `get_pipeline` | Query | Detailed pipeline config |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Cognito

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Cognito::UserPool` | `list_user_pools` | Query | Paginated |
| | `describe_user_pool` | Query | Detailed pool config |
| | `get_user_pool_mfa_config` | Query | MFA settings |
| | `list_user_pool_clients` | Query | App clients |
| | `describe_user_pool_client` | Query | Client details |
| `AWS::Cognito::IdentityPool` | `list_identity_pools` | Query | Paginated |
| | `describe_identity_pool` | Query | Auth providers, roles |
| `AWS::Cognito::UserPoolClient` | `list_user_pool_clients` | Query | Per user pool |
| | `describe_user_pool_client` | Query | Client details |

**Security Details Retrieved**:
- User Pool Policies (password policy, account recovery)
- MFA Configuration (OFF/ON/OPTIONAL, SMS/TOTP)
- Email/SMS Configuration
- User Pool Clients (app client IDs, token validity)
- Identity Pool auth providers and unauthenticated access settings

---

### Config (AWS Config)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Config::ConfigurationRecorder` | `describe_configuration_recorders` | Query | Lists recorders |
| | `describe_configuration_recorder_status` | Query | Recorder status |
| `AWS::Config::ConfigRule` | `describe_config_rules` | Query | Via paginator |
| | `describe_compliance_by_config_rule` | Query | Rule compliance |

---

### Connect

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Connect::Instance` | `list_instances` | Query | Via paginator |

---

### DataBrew

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::DataBrew::Dataset` | `list_datasets` | Query | Lists datasets |
| `AWS::DataBrew::Job` | `list_jobs` | Query | Lists jobs |
| | `get_resources` | Normalizer | Tag enrichment |

---

### DataSync

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::DataSync::Task` | `list_tasks` | Query | Paginated |
| `AWS::DataSync::Location` | `list_locations` | Query | Paginated |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Detective

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Detective::Graph` | `list_graphs` | Query | Lists graphs |
| `AWS::Detective::Member` | `list_members` | Query | Paginated per graph |

---

### DocumentDB

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::DocumentDB::Cluster` | `describe_db_clusters` | Query | Lists clusters |
| | `get_resources` | Normalizer | Tag enrichment |

---

### DynamoDB

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::DynamoDB::Table` | `list_tables` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_table` for detailed table config (billing mode, GSIs, encryption).

---

### EC2

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::EC2::Instance` | `describe_instances` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::VPC` | `describe_vpcs` | Query | Lists VPCs |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::SecurityGroup` | `describe_security_groups` | Query | Lists security groups |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::Subnet` | `describe_subnets` | Query | Lists subnets |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::Volume` | `describe_volumes` | Query | Lists volumes |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::Snapshot` | (implied from volumes) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::Image` | `describe_images` | Query | Lists AMIs |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::InternetGateway` | `describe_internet_gateways` | Query | Lists IGWs |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::RouteTable` | `describe_route_tables` | Query | Lists route tables |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::NatGateway` | `describe_nat_gateways` | Query | Lists NAT gateways |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::NetworkInterface` | `describe_network_interfaces` | Query | Lists ENIs |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::VPCEndpoint` | `describe_vpc_endpoints` | Query | Lists VPC endpoints |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::NetworkAcl` | `describe_network_acls` | Query | Lists NACLs |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::KeyPair` | `describe_key_pairs` | Query | Lists key pairs |
| | `get_resources` | Normalizer | Tag enrichment |

**EC2 Extended Resources:**

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::EC2::TransitGateway` | (from describe calls) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::VPCPeeringConnection` | (from describe calls) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::FlowLog` | (from describe calls) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::EC2::VolumeAttachment` | `describe_volumes` | Query | Attachment info in volume response |

**Gaps**: No `describe_instance_attribute` for detailed instance settings.

---

### ECR

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ECR::Repository` | `describe_repositories` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `get_repository_policy`, `get_lifecycle_policy` for security configs.

---

### ECS

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ECS::Cluster` | `list_clusters` | Query (Phase 1) | Via paginator |
| | `describe_clusters` | Query (Phase 2) | Cluster capacity, settings |
| | `list_container_instances` | Query (Phase 2) | Container instances in cluster |
| `AWS::ECS::Service` | `list_services` | Query (Phase 1) | Per cluster |
| | `describe_services` | Query (Phase 2) | Service details, deployments |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::ECS::Task` | (from cluster detail) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::ECS::TaskDefinition` | (from tasks) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::ECS::FargateService` | `list_services` | Query | Fargate launch type services |
| | `describe_services` | Query | Service details |
| `AWS::ECS::FargateTask` | `list_tasks` | Query | Fargate launch type tasks |
| | `describe_tasks` | Query | Task details |

**Security Details Retrieved**:
- Cluster capacity providers and default strategy
- Cluster settings (Container Insights, Execute Command)
- Container instance count
- Service deployment configuration and network mode
- Load balancer attachments

---

### EFS

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::EFS::FileSystem` | `describe_file_systems` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_file_system_policy`, `describe_mount_targets` for security configs.

---

### EKS

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::EKS::Cluster` | `list_clusters` | Query | Via paginator |
| | `describe_cluster` | Query | Detailed cluster info |
| `AWS::EKS::FargateProfile` | `list_fargate_profiles` | Query | Per cluster |
| | `describe_fargate_profile` | Query | Profile details |

**Gaps**: No `list_nodegroups`, `describe_nodegroup` for node configuration.

---

### ElastiCache

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ElastiCache::CacheCluster` | `describe_cache_clusters` | Query | Paginated |
| `AWS::ElastiCache::ReplicationGroup` | `describe_replication_groups` | Query | Paginated |
| `AWS::ElastiCache::ParameterGroup` | `describe_cache_parameter_groups` | Query | Paginated |

---

### Elastic Load Balancing (Classic)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ElasticLoadBalancing::LoadBalancer` | `describe_load_balancers` | Query | Lists CLBs |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_load_balancer_attributes` for detailed settings.

---

### Elastic Load Balancing V2 (ALB/NLB)

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::ElasticLoadBalancingV2::LoadBalancer` | `describe_load_balancers` | Query (Phase 1) | Lists ALBs/NLBs |
| | `describe_load_balancer_attributes` | Query (Phase 2) | Access logs, deletion protection |
| | `describe_listeners` | Query (Phase 2) | Listener protocols and certificates |
| `AWS::ElasticLoadBalancingV2::TargetGroup` | `describe_target_groups` | Query | Lists target groups |

**Security Details Retrieved**:
- Access logging configuration (S3 bucket, prefix)
- Deletion protection status
- Connection idle timeout
- HTTP/2 and routing settings
- Listener SSL/TLS certificates and policies

---

### EMR

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::EMR::Cluster` | `list_clusters` | Query (Phase 1) | Via paginator |
| | `describe_cluster` | Query (Phase 2) | Cluster configuration |
| | `list_instance_groups` | Query (Phase 2) | Instance group details |
| | `list_steps` | Query (Phase 2) | Cluster steps |
| | `get_resources` | Normalizer | Tag enrichment |

**Security Details Retrieved**:
- Cluster security configuration name
- Instance groups (master, core, task) with sizes
- VPC and subnet configuration
- Termination protection status
- Recent cluster steps and status

---

### EventBridge

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Events::EventBus` | `list_event_buses` | Query (Phase 1) | Lists event buses |
| | `describe_event_bus` | Query (Phase 2) | Event bus policy and ARN |
| | `list_rules` | Query (Phase 2) | Rules for the event bus |
| | `list_targets_by_rule` | Query (Phase 2) | Targets per rule |
| | `list_archives` | Query (Phase 2) | Archives for the event bus |
| `AWS::Events::Rule` | `list_rules` | Query | Lists rules |
| | `describe_rule` | Query | Detailed rule info |

**Security Details Retrieved**:
- Event bus resource policy
- Rules attached to bus with schedules and patterns
- Rule targets (Lambda, SNS, SQS, etc.)
- Archive configurations

---

### FSx

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::FSx::FileSystem` | `describe_file_systems` | Query | Paginated |
| `AWS::FSx::Backup` | `describe_backups` | Query | Paginated |

---

### Global Accelerator

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::GlobalAccelerator::Accelerator` | `list_accelerators` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Glue

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Glue::Job` | `get_jobs` | Query (Phase 1) | Via paginator |
| | `get_job` | Query (Phase 2) | Detailed job configuration |
| | `get_job_runs` | Query (Phase 2) | Recent job execution history |
| | `get_triggers` | Query (Phase 2) | Job triggers |

**Security Details Retrieved**:
- Job IAM role and security configuration
- Worker type and number of workers
- Job bookmarks and retry settings
- Recent job runs with status and duration
- Trigger schedules and conditions

**Gaps**: No `get_databases`, `get_tables` for data catalog resources.

---

### Greengrass

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::GreengrassV2::ComponentVersion` | `list_components` | Query | Lists components |
| | `get_component` | Query | Detailed component info |
| | `get_resources` | Normalizer | Tag enrichment |

---

### GuardDuty

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::GuardDuty::Detector` | `list_detectors` | Query | Lists detectors |

**Gaps**: No `get_detector` for detailed detector configuration.

---

### IAM

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::IAM::User` | `list_users` | Query | Paginated |
| | `get_user` | Query | Detailed user info |
| | `list_attached_user_policies` | Query | Managed policies attached to user |
| | `list_groups_for_user` | Query | Groups user belongs to |
| | `list_user_policies` | Query | Inline policy names |
| | `get_user_policy` | Query | Inline policy document content |
| | `list_access_keys` | Query | Access key IDs, status, creation dates |
| | `list_mfa_devices` | Query | MFA device serial numbers |
| | `get_login_profile` | Query | Console access status |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::IAM::Role` | `list_roles` | Query | Paginated |
| | `get_role` | Query | Detailed role info |
| | `list_attached_role_policies` | Query | Managed policies attached to role |
| | `list_role_policies` | Query | Inline policy names |
| | `get_role_policy` | Query | Inline policy document content |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::IAM::Policy` | `list_policies` | Query | Paginated, customer-managed only |
| | `get_policy` | Query | Detailed policy info |
| | `get_policy_version` | Query | Policy document JSON |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: None - comprehensive IAM security data is now available.

---

### Inspector

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Inspector::Configuration` | `get_configuration` | Query | Gets config |
| | `get_resources` | Normalizer | Tag enrichment |

---

### IoT

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::IoT::Thing` | `list_things` | Query | Lists things |

**Gaps**: No `describe_thing` for detailed thing config.

---

### Kinesis

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Kinesis::Stream` | `list_streams` | Query | Lists streams |

**Gaps**: No `describe_stream` for detailed stream configuration.

---

### Kinesis Data Firehose

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::KinesisFirehose::DeliveryStream` | `list_delivery_streams` | Query | Lists streams |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_delivery_stream` for detailed config.

---

### KMS

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::KMS::Key` | `list_keys` | Query | Via paginator |
| | `describe_key` | Query | Detailed key info |
| | `get_key_policy` | Query | Key policy document JSON |
| | `get_key_rotation_status` | Query | Automatic rotation enabled/disabled |
| | `list_key_policies` | Query | Policy names (usually just "default") |
| | `list_grants` | Query | Key grants with operations and constraints |
| | `list_aliases` | Query | Key aliases and target IDs |

**Status**: Complete - Key policies, rotation status, grants, and aliases implemented.

---

### Lake Formation

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::LakeFormation::DataLakeSettings` | `get_data_lake_settings` | Query | Data lake admin settings |
| `AWS::LakeFormation::Permission` | `list_permissions` | Query | Paginated |

---

### Lambda

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Lambda::Function` | `list_functions` | Query | Via paginator |
| | `get_function` | Query | Get function with code location |
| | `get_function_configuration` | Query | Detailed configuration, VPC, layers, environment |
| | `get_policy` | Query | Resource-based policy JSON |
| | `get_function_concurrency` | Query | Reserved concurrency settings |
| | `list_function_url_configs` | Query | Function URL endpoints with CORS |
| | `get_function_code_signing_config` | Query | Code signing configuration |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::Lambda::LayerVersion` | (from functions) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::Lambda::EventSourceMapping` | (from functions) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |

**Status**: Complete - Detailed function configuration, policies, concurrency, URLs, and code signing implemented.

---

### Lex

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Lex::Bot` | `list_bots` | Query | Via paginator |

---

### Macie

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Macie::Session` | `get_macie_session` | Query | Gets session info |

---

### MQ (Amazon MQ)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::AmazonMQ::Broker` | `list_brokers` | Query | Lists brokers |
| | `describe_broker` | Query | Detailed broker info |

---

### MSK (Managed Streaming for Kafka)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::MSK::Cluster` | `list_clusters_v2` | Query | Lists clusters |

**Gaps**: No `describe_cluster_v2` for detailed cluster configuration.

---

### Neptune

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Neptune::DBCluster` | `describe_db_clusters` | Query | Paginated |
| `AWS::Neptune::DBInstance` | `describe_db_instances` | Query | Paginated |

---

### OpenSearch

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::OpenSearchService::Domain` | `list_domain_names` | Query (Phase 1) | Lists domains |
| | `describe_domain` | Query (Phase 2) | Domain configuration |
| | `describe_domain_config` | Query (Phase 2) | Advanced options |
| | `list_tags` | Query (Phase 2) | Domain tags |
| | `get_resources` | Normalizer | Tag enrichment |

**Security Details Retrieved**:
- VPC configuration and security groups
- Encryption at rest and node-to-node encryption
- Cognito authentication settings
- Fine-grained access control configuration
- Endpoint and domain status

---

### Organizations

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Organizations::Organization` | `describe_organization` | Query | Org details |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::Organizations::Root` | `list_roots` | Query | Via paginator |
| `AWS::Organizations::OrganizationalUnit` | (from roots) | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::Organizations::Account` | (from OUs) | Query | Via paginator |
| `AWS::Organizations::Policy` | `describe_policy` | Query | Policy details |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::Organizations::AwsServiceAccess` | `list_aws_service_access_for_organization` | Query | Enabled services |
| `AWS::Organizations::CreateAccountStatus` | `list_create_account_status` | Query | Account creation status |
| `AWS::Organizations::DelegatedAdministrator` | `list_delegated_administrators` | Query | Delegated admins |
| `AWS::Organizations::Handshake` | `list_handshakes_for_organization` | Query | Org handshakes |

---

### Polly

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Polly::Lexicon` | `list_lexicons` | Query | Paginated |
| `AWS::Polly::Voice` | `describe_voices` | Query | Available voices |
| `AWS::Polly::SynthesisTask` | `list_speech_synthesis_tasks` | Query | Async synthesis tasks |
| | `get_resources` | Normalizer | Tag enrichment |

---

### QuickSight

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::QuickSight::DataSource` | `list_data_sources` | Query | Paginated |
| `AWS::QuickSight::Dashboard` | `list_dashboards` | Query | Paginated |
| `AWS::QuickSight::DataSet` | `list_data_sets` | Query | Paginated |
| | `get_resources` | Normalizer | Tag enrichment |

---

### RDS

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::RDS::DBInstance` | `describe_db_instances` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::RDS::DBCluster` | `describe_db_clusters` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::RDS::DBSnapshot` | (from instances) | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::RDS::DBParameterGroup` | (from instances) | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::RDS::DBSubnetGroup` | `describe_db_subnet_groups` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Redshift

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Redshift::Cluster` | `describe_clusters` | Query (Phase 1) | Lists clusters |
| | `describe_logging_status` | Query (Phase 2) | Audit logging configuration |
| | `describe_cluster_snapshots` | Query (Phase 2) | Recent snapshots |

**Security Details Retrieved**:
- Audit logging bucket and prefix
- VPC and security group configuration
- Encryption status
- Recent snapshots with creation times

---

### Rekognition

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Rekognition::Collection` | `list_collections` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::Rekognition::StreamProcessor` | `list_stream_processors` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Resource Groups Tagging API

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| (cross-service) | `get_resources` | Normalizer | Universal tag fetching |
| | `get_tag_keys` | Query | Lists all tag keys |
| | `get_tag_values` | Query | Values for specific key |

---

### Route 53

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Route53::HostedZone` | `list_hosted_zones` | Query | Via paginator |
| | `get_hosted_zone` | Query | Detailed zone info |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `list_resource_record_sets` for DNS records.

---

### S3

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::S3::Bucket` | `list_buckets` | Query | Lists all buckets |
| | `get_bucket_location` | Query | Bucket region |
| | `get_bucket_versioning` | Query | Versioning config |
| | `get_bucket_encryption` | Query | Encryption config |
| | `get_bucket_policy_status` | Query | Public access status |
| | `get_bucket_logging` | Query | Logging config |
| | `get_bucket_policy` | Query | Bucket policy |
| | `get_bucket_lifecycle_configuration` | Query | Lifecycle rules |
| | `get_bucket_acl` | Query | Bucket ACL with grants and owner |
| | `get_public_access_block` | Query | Public access block settings |
| | `get_bucket_replication` | Query | Cross-region/account replication rules |
| | `get_bucket_cors` | Query | CORS configuration |
| | `get_bucket_website` | Query | Static website hosting config |
| | `get_bucket_notification_configuration` | Query | Event notifications (Lambda, SQS, SNS) |

**Status**: Complete security view with ACL, public access block, replication, CORS, website, and notifications.

---

### SageMaker

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::SageMaker::Endpoint` | `list_endpoints` | Query | Via paginator |
| `AWS::SageMaker::Model` | `list_models` | Query | Via paginator |
| `AWS::SageMaker::TrainingJob` | `list_training_jobs` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

**Gaps**: No `describe_endpoint`, `describe_model` for detailed configs.

---

### Secrets Manager

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::SecretsManager::Secret` | `list_secrets` | Query | Via paginator |
| | `describe_secret` | Query | Detailed secret info |

**Gaps**: No `get_resource_policy` for secret access policies.

---

### Security Hub

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::SecurityHub::Hub` | (implied) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Shield

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Shield::Subscription` | `describe_subscription` | Query | Gets subscription |
| `AWS::Shield::Protection` | `list_protections` | Query | Via paginator |
| | `describe_protection` | Query | Protection details |

---

### SNS

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::SNS::Topic` | `list_topics` | Query | Via paginator |
| | `get_topic_attributes` | Query | Encryption, policy, delivery settings |
| | `list_subscriptions_by_topic` | Query | Topic subscriptions |
| | `get_subscription_attributes` | Query | Subscription details |
| | `list_tags_for_resource` | Query | Topic/subscription tags |
| | `get_resources` | Normalizer | Tag enrichment |

**Security Details Retrieved**:
- KmsMasterKeyId - CMK for encryption
- Policy - Topic access policy JSON
- DeliveryPolicy - Delivery retry settings
- FifoTopic - FIFO topic status
- ContentBasedDeduplication - Dedup settings
- Subscriptions with protocol, endpoint, filter policy

---

### SQS

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::SQS::Queue` | `list_queues` | Query | Lists queues |
| | `get_queue_attributes` | Query | Full attributes including KMS, Policy, SSE |
| | `list_queue_tags` | Query | Queue tags |
| | `list_dead_letter_source_queues` | Query | Find queues using this as DLQ |

**Security Details Retrieved**:
- KmsMasterKeyId - CMK for encryption
- KmsDataKeyReusePeriodSeconds - Key reuse period
- SqsManagedSseEnabled - SQS-managed SSE status
- Policy - Queue access policy JSON
- RedrivePolicy - DLQ configuration
- RedriveAllowPolicy - Who can use this as DLQ
- FifoQueue - FIFO queue status
- ContentBasedDeduplication - Dedup settings

---

### SSM (Systems Manager)

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::SSM::Parameter` | `describe_parameters` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |
| `AWS::SSM::Document` | `list_documents` | Query | Via paginator |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Step Functions

**Status**: Complete

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::StepFunctions::StateMachine` | `list_state_machines` | Query (Phase 1) | Via paginator |
| | `describe_state_machine` | Query (Phase 2) | Workflow definition |
| | `list_executions` | Query (Phase 2) | Recent executions per status |

**Security Details Retrieved**:
- State machine definition (ASL JSON)
- IAM role ARN
- Logging configuration
- Recent executions with status breakdown (Running, Succeeded, Failed, etc.)
- Execution statistics

---

### Timestream

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Timestream::Database` | `describe_endpoints` | Query | Gets endpoints |
| | `get_resources` | Normalizer | Tag enrichment |

---

### Transfer Family

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::Transfer::Server` | `list_servers` | Query | Paginated |
| `AWS::Transfer::User` | `list_users` | Query | Paginated per server |

---

### WAFv2

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::WAFv2::WebACL` | (implied) | Query | |
| | `get_resources` | Normalizer | Tag enrichment |

---

### WorkSpaces

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::WorkSpaces::Workspace` | `describe_workspaces` | Query | Paginated |
| `AWS::WorkSpaces::Directory` | `describe_workspace_directories` | Query | Paginated |

---

### X-Ray

| Resource Type | API Call | Side | Notes |
|---------------|----------|------|-------|
| `AWS::XRay::SamplingRule` | `get_sampling_rules` | Query | Gets rules |
| | `get_resources` | Normalizer | Tag enrichment |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| AWS Services Covered | 92 |
| Unique API Methods | 200+ |
| Query-side calls (Phase 1) | All services |
| Query-side calls (Phase 2) | 24 resource types |
| Normalizer-side calls | Primarily `get_resources` (tags) |

## Common API Patterns

| Pattern | Usage | Count |
|---------|-------|-------|
| `list_*` | Resource discovery (Phase 1) | 70+ |
| `describe_*` | Detailed resource info (Phase 1/2) | 40+ |
| `get_*` | Configuration retrieval (Phase 2) | 35+ |
| `get_resources` | Tag enrichment | Universal |

## Key Gaps for Security/Compliance

### Completed Services (Two-Phase Loading)

The following services have comprehensive security details via Phase 2 enrichment:
- **IAM**: Attached policies, policy versions, access keys, MFA devices, login profiles
- **S3**: ACL, public access block, replication, CORS, website, notifications
- **Lambda**: Function configuration, policies, concurrency, URLs, code signing
- **KMS**: Key policies, rotation status, grants, aliases
- **SQS/SNS**: Queue/topic attributes, encryption, policies, subscriptions
- **Cognito**: MFA configuration, user pool clients, identity pool auth providers
- **CodeCommit**: Repository triggers, branches, commit history
- **CloudFormation**: Stack resources, outputs, template
- **ECS**: Cluster capacity, settings, service deployments
- **ELBv2**: Load balancer attributes, listeners, certificates
- **EMR**: Cluster configuration, instance groups, steps
- **EventBridge**: Event bus policies, rules, targets, archives
- **Glue**: Job configuration, runs, triggers
- **AWS Backup**: Plan rules, selections, vault policies, recovery points
- **Step Functions**: State machine definitions, execution history
- **OpenSearch**: Domain configuration, encryption, access control
- **Redshift**: Logging status, snapshots

### Remaining Gaps

**High Priority:**
1. **EC2**: Missing `describe_instance_attribute` for detailed settings
2. **ECR**: Missing `get_repository_policy`, `get_lifecycle_policy`
3. **RDS/DynamoDB**: Missing detailed describe calls for encryption/audit settings

**Medium Priority:**
1. **EKS**: Missing `list_nodegroups`, `describe_nodegroup`
2. **CloudTrail**: Missing `describe_trails` for trail configuration
3. **GuardDuty**: Missing `get_detector` for detector settings
4. **CloudWatch Logs**: Missing `describe_metric_filters`, `describe_subscription_filters`
5. **Secrets Manager**: Missing `get_resource_policy`

---

## Related Documentation

- [Resource Explorer System](resource-explorer-system.md) - Resource discovery architecture
- [AWS Service Integration Patterns](aws-service-integration-patterns.md) - Adding new services
- [Resource Normalizers](resource-normalizers.md) - Data transformation
