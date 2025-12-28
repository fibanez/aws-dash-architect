# AWS Explorer - Implemented Services

This document lists all AWS services currently implemented in the AWS Explorer feature of aws-dash.

## Summary

Total implemented services: **172 UI-registered resource types + 5 child resource types = 177 total** across **82 AWS services**

**Last Updated**: December 2025

### Child Resource Types (auto-queried, not UI-registered)
- `AWS::Bedrock::DataSource` (child of KnowledgeBase)
- `AWS::Bedrock::IngestionJob` (child of DataSource)
- `AWS::Bedrock::AgentAlias` (child of Agent)
- `AWS::Bedrock::AgentActionGroup` (child of Agent)
- `AWS::Bedrock::FlowAlias` (child of Flow)

## Services by Category

### Compute Services

#### EC2 (Elastic Compute Cloud)
- EC2 Instance (`AWS::EC2::Instance`)
- Security Group (`AWS::EC2::SecurityGroup`)
- VPC (Virtual Private Cloud) (`AWS::EC2::VPC`)
- EBS Volume (`AWS::EC2::Volume`)
- EBS Volume Attachment (`AWS::EC2::VolumeAttachment`)
- EBS Snapshot (`AWS::EC2::Snapshot`)
- AMI (Amazon Machine Image) (`AWS::EC2::Image`)
- Subnet (`AWS::EC2::Subnet`)
- Route Table (`AWS::EC2::RouteTable`)
- NAT Gateway (`AWS::EC2::NatGateway`)
- Network Interface (`AWS::EC2::NetworkInterface`)
- VPC Endpoint (`AWS::EC2::VPCEndpoint`)
- Network ACL (`AWS::EC2::NetworkAcl`)
- Key Pair (`AWS::EC2::KeyPair`)
- Internet Gateway (`AWS::EC2::InternetGateway`)
- Transit Gateway (`AWS::EC2::TransitGateway`)
- VPC Peering Connection (`AWS::EC2::VPCPeeringConnection`)
- VPC Flow Log (`AWS::EC2::FlowLog`)

#### Lambda
- Lambda Function (`AWS::Lambda::Function`)
- Lambda Layer (`AWS::Lambda::LayerVersion`)
- Lambda Event Source Mapping (`AWS::Lambda::EventSourceMapping`)

#### ECS (Elastic Container Service)
- ECS Cluster (`AWS::ECS::Cluster`)
- ECS Service (`AWS::ECS::Service`)
- ECS Task (`AWS::ECS::Task`)
- ECS Task Definition (`AWS::ECS::TaskDefinition`)
- ECS Fargate Service (`AWS::ECS::FargateService`)
- ECS Fargate Task (`AWS::ECS::FargateTask`)

#### EKS (Elastic Kubernetes Service)
- EKS Cluster (`AWS::EKS::Cluster`)
- EKS Fargate Profile (`AWS::EKS::FargateProfile`)

#### Batch
- Batch Job Queue (`AWS::Batch::JobQueue`)
- Batch Compute Environment (`AWS::Batch::ComputeEnvironment`)

#### App Runner
- App Runner Service (`AWS::AppRunner::Service`)
- App Runner Connection (`AWS::AppRunner::Connection`)

### Storage Services

#### S3 (Simple Storage Service)
- S3 Bucket (`AWS::S3::Bucket`)

#### EFS (Elastic File System)
- EFS File System (`AWS::EFS::FileSystem`)

#### FSx
- FSx File System (`AWS::FSx::FileSystem`)
- FSx Backup (`AWS::FSx::Backup`)

#### Transfer Family
- Transfer Family Server (`AWS::Transfer::Server`)

#### DataSync
- DataSync Task (`AWS::DataSync::Task`)

### Database Services

#### RDS (Relational Database Service)
- RDS DB Instance (`AWS::RDS::DBInstance`)
- RDS DB Cluster (`AWS::RDS::DBCluster`)
- RDS DB Snapshot (`AWS::RDS::DBSnapshot`)
- RDS DB Parameter Group (`AWS::RDS::DBParameterGroup`)
- RDS DB Subnet Group (`AWS::RDS::DBSubnetGroup`)

#### DynamoDB
- DynamoDB Table (`AWS::DynamoDB::Table`)

#### ElastiCache
- Cache Cluster (`AWS::ElastiCache::CacheCluster`)
- Redis Replication Group (`AWS::ElastiCache::ReplicationGroup`)
- Cache Parameter Group (`AWS::ElastiCache::ParameterGroup`)

#### Neptune
- Graph Database Cluster (`AWS::Neptune::DBCluster`)
- Graph Database Instance (`AWS::Neptune::DBInstance`)

#### OpenSearch
- Search and Analytics Engine (`AWS::OpenSearchService::Domain`)

#### Redshift
- Redshift Cluster (`AWS::Redshift::Cluster`)

#### DocumentDB
- DocumentDB Cluster (`AWS::DocumentDB::Cluster`)

#### Timestream
- Timestream Database (`AWS::Timestream::Database`)

### Networking & Content Delivery

#### CloudFront
- Content Delivery Network (`AWS::CloudFront::Distribution`)

#### Route53
- Route53 Hosted Zone (`AWS::Route53::HostedZone`)

#### API Gateway
- API Gateway REST API (`AWS::ApiGateway::RestApi`)
- API Gateway v2 HTTP API (`AWS::ApiGatewayV2::Api`)

#### ELB (Elastic Load Balancing)
- Classic Load Balancer (`AWS::ElasticLoadBalancing::LoadBalancer`)
- Application/Network Load Balancer (`AWS::ElasticLoadBalancingV2::LoadBalancer`)
- Target Group (`AWS::ElasticLoadBalancingV2::TargetGroup`)

#### Global Accelerator
- Global Accelerator (`AWS::GlobalAccelerator::Accelerator`)

### Security, Identity & Compliance

#### IAM (Identity and Access Management)
- IAM Role (`AWS::IAM::Role`)
- IAM User (`AWS::IAM::User`)
- IAM Policy (`AWS::IAM::Policy`)

#### Certificate Manager
- SSL/TLS Certificate (`AWS::CertificateManager::Certificate`)
- Private Certificate Authority (`AWS::ACMPCA::CertificateAuthority`)

#### WAF & Shield
- Web Application Firewall (`AWS::WAFv2::WebACL`)
- Shield Protection (`AWS::Shield::Protection`)
- Shield Advanced Subscription (`AWS::Shield::Subscription`)

#### GuardDuty
- Threat Detection Service (`AWS::GuardDuty::Detector`)

#### Security Hub
- Security Hub Service (`AWS::SecurityHub::Hub`)

#### Detective
- Detective Security Investigation (`AWS::Detective::Graph`)

#### Access Analyzer
- IAM Access Analyzer (`AWS::AccessAnalyzer::Analyzer`)

#### Cognito
- User Pool (`AWS::Cognito::UserPool`)
- Identity Pool (`AWS::Cognito::IdentityPool`)
- User Pool Client (`AWS::Cognito::UserPoolClient`)

#### Macie
- Macie Session (`AWS::Macie::Session`)

#### Inspector
- Inspector Configuration (`AWS::Inspector::Configuration`)

#### KMS (Key Management Service)
- KMS Encryption Key (`AWS::KMS::Key`)

#### Secrets Manager
- Secrets Manager Secret (`AWS::SecretsManager::Secret`)

### Management & Governance

#### CloudFormation
- CloudFormation Stack (`AWS::CloudFormation::Stack`)

#### CloudWatch
- CloudWatch Alarm (`AWS::CloudWatch::Alarm`)
- CloudWatch Dashboard (`AWS::CloudWatch::Dashboard`)
- CloudWatch Log Group (`AWS::Logs::LogGroup`)

#### CloudTrail
- CloudTrail Trail (`AWS::CloudTrail::Trail`)
- CloudTrail Event (`AWS::CloudTrail::Event`)
- CloudTrail Event Data Store (`AWS::CloudTrail::EventDataStore`)

#### Config
- Config Configuration Recorder (`AWS::Config::ConfigurationRecorder`)
- Config Rule (`AWS::Config::ConfigRule`)

#### Systems Manager (SSM)
- Systems Manager Parameter (`AWS::SSM::Parameter`)
- Systems Manager Document (`AWS::SSM::Document`)

#### Organizations
- Organization (`AWS::Organizations::Organization`)
- Organizations Root (`AWS::Organizations::Root`)
- Organizational Unit (`AWS::Organizations::OrganizationalUnit`)
- Organizations Account (`AWS::Organizations::Account`)
- Service Control Policy (`AWS::Organizations::Policy`)
- Delegated Administrator (`AWS::Organizations::DelegatedAdministrator`)
- Organization Handshake (`AWS::Organizations::Handshake`)
- Account Creation Status (`AWS::Organizations::CreateAccountStatus`)
- AWS Service Access (`AWS::Organizations::AwsServiceAccess`)

#### Backup
- Backup Plan (`AWS::Backup::BackupPlan`)
- Backup Vault (`AWS::Backup::BackupVault`)

### Application Integration

#### SNS (Simple Notification Service)
- SNS Topic (`AWS::SNS::Topic`)

#### SQS (Simple Queue Service)
- SQS Queue (`AWS::SQS::Queue`)

#### EventBridge
- EventBridge Event Bus (`AWS::Events::EventBus`)
- EventBridge Rule (`AWS::Events::Rule`)

#### Kinesis
- Kinesis Data Stream (`AWS::Kinesis::Stream`)

#### Kinesis Data Firehose
- Kinesis Data Firehose Delivery Stream (`AWS::KinesisFirehose::DeliveryStream`)

#### AppSync
- AppSync GraphQL API (`AWS::AppSync::GraphQLApi`)

#### Amazon MQ
- Amazon MQ Broker (`AWS::AmazonMQ::Broker`)

#### MSK (Managed Streaming for Apache Kafka)
- MSK Kafka Cluster (`AWS::MSK::Cluster`)

### Analytics

#### Athena
- Athena Workgroup (`AWS::Athena::WorkGroup`)

#### Glue
- Glue ETL Job (`AWS::Glue::Job`)

#### Lake Formation
- Lake Formation Data Lake Settings (`AWS::LakeFormation::DataLakeSettings`)

#### QuickSight
- QuickSight Data Source (`AWS::QuickSight::DataSource`)
- QuickSight Dashboard (`AWS::QuickSight::Dashboard`)
- QuickSight Data Set (`AWS::QuickSight::DataSet`)

### Machine Learning & AI

#### SageMaker
- SageMaker Endpoint (`AWS::SageMaker::Endpoint`)
- SageMaker Training Job (`AWS::SageMaker::TrainingJob`)
- SageMaker Model (`AWS::SageMaker::Model`)

#### Bedrock
- Bedrock Foundation Model (`AWS::Bedrock::Model`)
- Bedrock Inference Profile (`AWS::Bedrock::InferenceProfile`)
- Bedrock Guardrail (`AWS::Bedrock::Guardrail`)
- Bedrock Provisioned Model Throughput (`AWS::Bedrock::ProvisionedModelThroughput`)
- Bedrock Agent (`AWS::Bedrock::Agent`)
- Bedrock Knowledge Base (`AWS::Bedrock::KnowledgeBase`)
- Bedrock Custom Model (`AWS::Bedrock::CustomModel`)
- Bedrock Imported Model (`AWS::Bedrock::ImportedModel`)
- Bedrock Evaluation Job (`AWS::Bedrock::EvaluationJob`)
- Bedrock Model Invocation Job (`AWS::Bedrock::ModelInvocationJob`)
- Bedrock Prompt (`AWS::Bedrock::Prompt`)
- Bedrock Flow (`AWS::Bedrock::Flow`)
- Bedrock Model Customization Job (`AWS::Bedrock::ModelCustomizationJob`)

#### BedrockAgentCore
- AgentCore Runtime (`AWS::BedrockAgentCore::AgentRuntime`)
- AgentCore Runtime Endpoint (`AWS::BedrockAgentCore::AgentRuntimeEndpoint`)
- AgentCore Memory (`AWS::BedrockAgentCore::Memory`)
- AgentCore Gateway (`AWS::BedrockAgentCore::Gateway`)
- AgentCore Browser (`AWS::BedrockAgentCore::Browser`)
- AgentCore Code Interpreter (`AWS::BedrockAgentCore::CodeInterpreter`)
- AgentCore API Key Credential Provider (`AWS::BedrockAgentCore::ApiKeyCredentialProvider`)
- AgentCore OAuth2 Credential Provider (`AWS::BedrockAgentCore::OAuth2CredentialProvider`)
- AgentCore Workload Identity (`AWS::BedrockAgentCore::WorkloadIdentity`)

#### Lex
- Lex Bot (`AWS::Lex::Bot`)

#### Polly
- Polly Voice (`AWS::Polly::Voice`)
- Polly Lexicon (`AWS::Polly::Lexicon`)
- Polly Synthesis Task (`AWS::Polly::SynthesisTask`)

#### Rekognition
- Rekognition Collection (`AWS::Rekognition::Collection`)
- Rekognition Stream Processor (`AWS::Rekognition::StreamProcessor`)

### Developer Tools

#### CodePipeline
- CodePipeline Pipeline (`AWS::CodePipeline::Pipeline`)

#### CodeBuild
- CodeBuild Project (`AWS::CodeBuild::Project`)

#### CodeCommit
- CodeCommit Repository (`AWS::CodeCommit::Repository`)

#### CodeDeploy
- CodeDeploy Application (`AWS::CodeDeploy::Application`)
- CodeDeploy Deployment Group (`AWS::CodeDeploy::DeploymentGroup`)

#### CodeArtifact
- CodeArtifact Domain (`AWS::CodeArtifact::Domain`)
- CodeArtifact Repository (`AWS::CodeArtifact::Repository`)

#### ECR (Elastic Container Registry)
- ECR Container Registry (`AWS::ECR::Repository`)

### Compute Services Extensions

#### Auto Scaling
- Auto Scaling Group (`AWS::AutoScaling::AutoScalingGroup`)
- Auto Scaling Policy (`AWS::AutoScaling::ScalingPolicy`)

### Application Performance Monitoring

#### X-Ray
- X-Ray Sampling Rule (`AWS::XRay::SamplingRule`)

### Security & DDoS Protection

#### Shield Advanced
- Shield Protection (`AWS::Shield::Protection`)
- Shield Advanced Subscription (`AWS::Shield::Subscription`)

### Workflow Orchestration

#### Step Functions
- Step Functions State Machine (`AWS::StepFunctions::StateMachine`)

### Data Preparation & Processing

#### DataBrew
- DataBrew Job (`AWS::DataBrew::Job`)
- DataBrew Dataset (`AWS::DataBrew::Dataset`)

### Application Configuration

#### AppConfig
- AppConfig Application (`AWS::AppConfig::Application`)
- AppConfig Environment (`AWS::AppConfig::Environment`)
- AppConfig Configuration Profile (`AWS::AppConfig::ConfigurationProfile`)

### IoT Services

#### IoT Core
- IoT Thing (`AWS::IoT::Thing`)

#### Greengrass
- Greengrass Component Version (`AWS::GreengrassV2::ComponentVersion`)

### End User Computing

#### WorkSpaces
- WorkSpaces Workspace (`AWS::WorkSpaces::Workspace`)
- WorkSpaces Directory (`AWS::WorkSpaces::Directory`)

### Customer Engagement

#### Connect
- Connect Instance (`AWS::Connect::Instance`)

### Mobile & Web App Development

#### Amplify
- Amplify App (`AWS::Amplify::App`)

## Implementation Details

The AWS services are implemented in the following locations:

1. **Service Modules**: `/src/app/resource_explorer/aws_services/` - Contains individual service implementations
2. **Resource Type Definitions**: `/src/app/resource_explorer/dialogs.rs` - The `get_default_resource_types()` function
3. **AWS Client**: `/src/app/resource_explorer/aws_client.rs` - Handles API calls to AWS services

Each service module typically implements:
- List operations to discover resources
- Describe operations to get detailed properties
- Resource type mappings for CloudFormation compatibility

The implementation supports:
- Multi-account resource discovery
- Multi-region resource discovery  
- Real-time resource loading with progress updates
- Caching for performance optimization
- Detailed property retrieval on demand
