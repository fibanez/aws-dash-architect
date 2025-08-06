use crate::app::resource_explorer::state::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub mod accessanalyzer;
pub mod acm;
pub mod acmpca;
pub mod amplify;
pub mod apigateway;
pub mod apigatewayv2;
pub mod apprunner;
pub mod appsync;
pub mod athena;
pub mod autoscaling;
pub mod backup;
pub mod batch;
pub mod bedrock;
pub mod cloudformation;
pub mod cloudfront;
pub mod cloudtrail;
pub mod cloudwatch;
pub mod connect;
pub mod codebuild;
pub mod codecommit;
pub mod codepipeline;
pub mod cognito;
pub mod config_rule;
pub mod configservice;
pub mod databrew;
pub mod datasync;
pub mod detective;
pub mod documentdb;
pub mod dynamodb;
pub mod ec2;
pub mod ec2_extended;
pub mod ecr;
pub mod ecs;
pub mod efs;
pub mod eks;
pub mod elasticache;
pub mod fsx;
pub mod globalaccelerator;
pub mod elb;
pub mod elbv2;
pub mod eventbridge;
pub mod fargate;
pub mod glue;
pub mod greengrass;
pub mod guardduty;
pub mod iam;
pub mod inspector;
pub mod iot;
pub mod kinesis;
pub mod kinesisfirehose;
pub mod kms;
pub mod lakeformation;
pub mod lambda;
pub mod lex;
pub mod macie;
pub mod logs;
pub mod mq;
pub mod msk;
pub mod neptune;
pub mod opensearch;
pub mod organizations_ou;
pub mod organizations_policy;
pub mod polly;
pub mod quicksight;
pub mod rds;
pub mod redshift;
pub mod rekognition;
pub mod route53;
pub mod s3;
pub mod sagemaker;
pub mod sagemaker_model;
pub mod sagemaker_training_job;
pub mod secretsmanager;
pub mod securityhub;
pub mod shield;
pub mod sns;
pub mod sqs;
pub mod ssm;
pub mod stepfunctions;
pub mod timestream;
pub mod transfer;
pub mod wafv2;
pub mod workspaces;
pub mod xray;

pub use accessanalyzer::*;
pub use acm::*;
pub use acmpca::*;
pub use amplify::*;
pub use apigateway::*;
pub use apigatewayv2::*;
pub use apprunner::*;
pub use appsync::*;
pub use athena::*;
pub use autoscaling::*;
pub use backup::*;
pub use batch::*;
pub use bedrock::*;
pub use cloudformation::*;
pub use cloudfront::*;
pub use cloudtrail::*;
pub use cloudwatch::*;
pub use connect::*;
pub use codebuild::*;
pub use codecommit::*;
pub use codepipeline::*;
pub use cognito::*;
pub use config_rule::*;
pub use configservice::*;
pub use databrew::*;
pub use datasync::*;
pub use detective::*;
pub use documentdb::*;
pub use dynamodb::*;
pub use ec2::*;
pub use ec2_extended::*;
pub use ecr::*;
pub use ecs::*;
pub use efs::*;
pub use eks::*;
pub use elasticache::*;
pub use fsx::*;
pub use globalaccelerator::*;
pub use elb::*;
pub use elbv2::*;
pub use eventbridge::*;
pub use fargate::*;
pub use glue::*;
pub use greengrass::*;
pub use guardduty::*;
pub use iam::*;
pub use inspector::*;
pub use iot::*;
pub use kinesis::*;
pub use kinesisfirehose::*;
pub use kms::*;
pub use lakeformation::*;
pub use lambda::*;
pub use lex::*;
pub use macie::*;
pub use logs::*;
pub use mq::*;
pub use msk::*;
pub use neptune::*;
pub use opensearch::*;
pub use organizations_ou::*;
pub use organizations_policy::*;
pub use polly::*;
pub use quicksight::*;
pub use rds::*;
pub use redshift::*;
pub use rekognition::*;
pub use route53::*;
pub use s3::*;
pub use sagemaker::*;
pub use sagemaker_model::*;
pub use sagemaker_training_job::*;
pub use secretsmanager::*;
pub use securityhub::*;
pub use shield::*;
pub use sns::*;
pub use sqs::*;
pub use ssm::*;
pub use stepfunctions::*;
pub use timestream::*;
pub use transfer::*;
pub use wafv2::*;
pub use workspaces::*;
pub use xray::*;

/// Trait for normalizing different AWS service responses into consistent ResourceEntry format
pub trait ResourceNormalizer {
    /// Normalize raw AWS API response into ResourceEntry
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry>;

    /// Extract relationships between this resource and others
    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship>;

    /// Get the resource type this normalizer handles
    fn resource_type(&self) -> &'static str;
}

/// Factory for creating appropriate normalizers
pub struct NormalizerFactory;

impl NormalizerFactory {
    pub fn create_normalizer(
        resource_type: &str,
    ) -> Option<Box<dyn ResourceNormalizer + Send + Sync>> {
        match resource_type {
            "AWS::EC2::Instance" => Some(Box::new(EC2InstanceNormalizer)),
            "AWS::EC2::SecurityGroup" => Some(Box::new(EC2SecurityGroupNormalizer)),
            "AWS::EC2::VPC" => Some(Box::new(EC2VPCNormalizer)),
            "AWS::EC2::Volume" => Some(Box::new(EC2VolumeNormalizer)),
            "AWS::EC2::Snapshot" => Some(Box::new(EC2SnapshotNormalizer)),
            "AWS::EC2::Image" => Some(Box::new(EC2ImageNormalizer)),
            "AWS::EC2::Subnet" => Some(Box::new(EC2SubnetNormalizer)),
            "AWS::EC2::RouteTable" => Some(Box::new(EC2RouteTableNormalizer)),
            "AWS::EC2::NatGateway" => Some(Box::new(EC2NatGatewayNormalizer)),
            "AWS::EC2::NetworkInterface" => Some(Box::new(EC2NetworkInterfaceNormalizer)),
            "AWS::EC2::VPCEndpoint" => Some(Box::new(EC2VPCEndpointNormalizer)),
            "AWS::EC2::NetworkAcl" => Some(Box::new(EC2NetworkAclNormalizer)),
            "AWS::EC2::KeyPair" => Some(Box::new(EC2KeyPairNormalizer)),
            "AWS::EC2::InternetGateway" => Some(Box::new(EC2InternetGatewayNormalizer)),
            "AWS::EC2::TransitGateway" => Some(Box::new(EC2TransitGatewayNormalizer)),
            "AWS::EC2::VPCPeeringConnection" => Some(Box::new(EC2VPCPeeringConnectionNormalizer)),
            "AWS::EC2::FlowLog" => Some(Box::new(EC2FlowLogNormalizer)),
            "AWS::EC2::VolumeAttachment" => Some(Box::new(EC2VolumeAttachmentNormalizer)),
            "AWS::ECS::FargateService" => Some(Box::new(ECSFargateServiceNormalizer)),
            "AWS::ECS::FargateTask" => Some(Box::new(ECSFargateTaskNormalizer)),
            "AWS::EKS::FargateProfile" => Some(Box::new(EKSFargateProfileNormalizer)),
            "AWS::IAM::Role" => Some(Box::new(IAMRoleNormalizer)),
            "AWS::IAM::User" => Some(Box::new(IAMUserNormalizer)),
            "AWS::IAM::Policy" => Some(Box::new(IAMPolicyNormalizer)),
            "AWS::Bedrock::Model" => Some(Box::new(BedrockModelNormalizer)),
            "AWS::S3::Bucket" => Some(Box::new(S3BucketNormalizer)),
            "AWS::CloudFormation::Stack" => Some(Box::new(CloudFormationStackNormalizer)),
            "AWS::RDS::DBInstance" => Some(Box::new(RDSDBInstanceNormalizer)),
            "AWS::RDS::DBCluster" => Some(Box::new(RDSDBClusterNormalizer)),
            "AWS::RDS::DBSnapshot" => Some(Box::new(RDSDBSnapshotNormalizer)),
            "AWS::RDS::DBParameterGroup" => Some(Box::new(RDSDBParameterGroupNormalizer)),
            "AWS::RDS::DBSubnetGroup" => Some(Box::new(RDSDBSubnetGroupNormalizer)),
            "AWS::Lambda::Function" => Some(Box::new(LambdaFunctionNormalizer)),
            "AWS::Lambda::LayerVersion" => Some(Box::new(LambdaLayerVersionNormalizer)),
            "AWS::Lambda::EventSourceMapping" => Some(Box::new(LambdaEventSourceMappingNormalizer)),
            "AWS::DynamoDB::Table" => Some(Box::new(DynamoDBTableNormalizer)),
            "AWS::CloudWatch::Alarm" => Some(Box::new(CloudWatchAlarmNormalizer)),
            "AWS::CloudWatch::Dashboard" => Some(Box::new(CloudWatchDashboardNormalizer)),
            "AWS::ApiGateway::RestApi" => Some(Box::new(ApiGatewayRestApiNormalizer)),
            "AWS::SNS::Topic" => Some(Box::new(SNSTopicNormalizer)),
            "AWS::SQS::Queue" => Some(Box::new(SQSQueueNormalizer)),
            "AWS::ECS::Cluster" => Some(Box::new(ECSClusterNormalizer)),
            "AWS::ECS::Service" => Some(Box::new(ECSServiceNormalizer)),
            "AWS::ECS::Task" => Some(Box::new(ECSTaskNormalizer)),
            "AWS::ECS::TaskDefinition" => Some(Box::new(ECSTaskDefinitionNormalizer)),
            "AWS::EKS::Cluster" => Some(Box::new(EKSClusterNormalizer)),
            "AWS::ElasticLoadBalancing::LoadBalancer" => Some(Box::new(ELBLoadBalancerNormalizer)),
            "AWS::ElasticLoadBalancingV2::LoadBalancer" => {
                Some(Box::new(ELBv2LoadBalancerNormalizer))
            }
            "AWS::ElasticLoadBalancingV2::TargetGroup" => {
                Some(Box::new(ELBv2TargetGroupNormalizer))
            }
            "AWS::Logs::LogGroup" => Some(Box::new(LogsResourceNormalizer)),
            "AWS::ApiGatewayV2::Api" => Some(Box::new(ApiGatewayV2ResourceNormalizer)),
            "AWS::Kinesis::Stream" => Some(Box::new(KinesisResourceNormalizer)),
            "AWS::SageMaker::Endpoint" => Some(Box::new(SageMakerResourceNormalizer)),
            "AWS::SageMaker::TrainingJob" => Some(Box::new(SageMakerTrainingJobNormalizer)),
            "AWS::SageMaker::Model" => Some(Box::new(SageMakerModelNormalizer)),
            "AWS::Redshift::Cluster" => Some(Box::new(RedshiftResourceNormalizer)),
            "AWS::Glue::Job" => Some(Box::new(GlueResourceNormalizer)),
            "AWS::Athena::WorkGroup" => Some(Box::new(AthenaResourceNormalizer)),
            "AWS::Route53::HostedZone" => Some(Box::new(Route53HostedZoneNormalizer)),
            "AWS::EFS::FileSystem" => Some(Box::new(EfsFileSystemNormalizer)),
            "AWS::CloudTrail::Trail" => Some(Box::new(CloudTrailNormalizer)),
            "AWS::Config::ConfigurationRecorder" => Some(Box::new(ConfigServiceNormalizer)),
            "AWS::Config::ConfigRule" => Some(Box::new(ConfigRuleNormalizer)),
            "AWS::DataBrew::Job" => Some(Box::new(DataBrewJobNormalizer)),
            "AWS::DataBrew::Dataset" => Some(Box::new(DataBrewDatasetNormalizer)),
            "AWS::Detective::Graph" => Some(Box::new(DetectiveNormalizer)),
            "AWS::AccessAnalyzer::Analyzer" => Some(Box::new(AccessAnalyzerNormalizer)),
            "AWS::SSM::Parameter" => Some(Box::new(SSMParameterNormalizer)),
            "AWS::SSM::Document" => Some(Box::new(SSMDocumentNormalizer)),
            "AWS::Backup::BackupPlan" => Some(Box::new(BackupPlanNormalizer)),
            "AWS::Backup::BackupVault" => Some(Box::new(BackupVaultNormalizer)),
            "AWS::Events::EventBus" => Some(Box::new(EventBridgeEventBusNormalizer)),
            "AWS::Events::Rule" => Some(Box::new(EventBridgeRuleNormalizer)),
            "AWS::AppSync::GraphQLApi" => Some(Box::new(AppSyncGraphQLApiNormalizer)),
            "AWS::AmazonMQ::Broker" => Some(Box::new(MQBrokerNormalizer)),
            "AWS::MSK::Cluster" => Some(Box::new(MskNormalizer)),
            "AWS::LakeFormation::DataLakeSettings" => Some(Box::new(LakeFormationNormalizer)),
            "AWS::CodePipeline::Pipeline" => Some(Box::new(CodePipelinePipelineNormalizer)),
            "AWS::CodeBuild::Project" => Some(Box::new(CodeBuildProjectNormalizer)),
            "AWS::CodeCommit::Repository" => Some(Box::new(CodeCommitRepositoryNormalizer)),
            "AWS::IoT::Thing" => Some(Box::new(IoTThingNormalizer)),
            "AWS::GreengrassV2::ComponentVersion" => {
                Some(Box::new(GreengrassComponentVersionNormalizer))
            }
            "AWS::Organizations::OrganizationalUnit" => Some(Box::new(OrganizationsOUNormalizer)),
            "AWS::Organizations::Policy" => Some(Box::new(OrganizationsPolicyNormalizer)),
            "AWS::CertificateManager::Certificate" => Some(Box::new(AcmCertificateNormalizer)),
            "AWS::ACMPCA::CertificateAuthority" => Some(Box::new(AcmPcaNormalizer)),
            "AWS::AutoScaling::AutoScalingGroup" => Some(Box::new(AutoScalingGroupNormalizer)),
            "AWS::AutoScaling::ScalingPolicy" => Some(Box::new(AutoScalingPolicyNormalizer)),
            "AWS::WAFv2::WebACL" => Some(Box::new(WafV2WebAclNormalizer)),
            "AWS::GuardDuty::Detector" => Some(Box::new(GuardDutyDetectorNormalizer)),
            "AWS::SecurityHub::Hub" => Some(Box::new(SecurityHubNormalizer)),
            "AWS::CloudFront::Distribution" => Some(Box::new(CloudFrontDistributionNormalizer)),
            "AWS::ElastiCache::CacheCluster" => Some(Box::new(ElastiCacheCacheClusterNormalizer)),
            "AWS::ElastiCache::ReplicationGroup" => {
                Some(Box::new(ElastiCacheReplicationGroupNormalizer))
            }
            "AWS::Neptune::DBCluster" => Some(Box::new(NeptuneDBClusterNormalizer)),
            "AWS::Neptune::DBInstance" => Some(Box::new(NeptuneDBInstanceNormalizer)),
            "AWS::OpenSearchService::Domain" => Some(Box::new(OpenSearchDomainNormalizer)),
            "AWS::Cognito::UserPool" => Some(Box::new(CognitoNormalizer)),
            "AWS::Cognito::IdentityPool" => Some(Box::new(CognitoNormalizer)),
            "AWS::Cognito::UserPoolClient" => Some(Box::new(CognitoNormalizer)),
            "AWS::Batch::JobQueue" => Some(Box::new(BatchJobQueueNormalizer)),
            "AWS::Batch::ComputeEnvironment" => Some(Box::new(BatchComputeEnvironmentNormalizer)),
            "AWS::KinesisFirehose::DeliveryStream" => {
                Some(Box::new(KinesisFirehoseDeliveryStreamNormalizer))
            }
            "AWS::QuickSight::DataSource" => Some(Box::new(QuickSightDataSourceNormalizer)),
            "AWS::QuickSight::Dashboard" => Some(Box::new(QuickSightDashboardNormalizer)),
            "AWS::QuickSight::DataSet" => Some(Box::new(QuickSightDataSetNormalizer)),
            "AWS::Macie::Session" => Some(Box::new(MacieResourceNormalizer)),
            "AWS::Inspector::Configuration" => Some(Box::new(InspectorResourceNormalizer)),
            "AWS::Timestream::Database" => Some(Box::new(TimestreamResourceNormalizer)),
            "AWS::DocumentDB::Cluster" => Some(Box::new(DocumentDbResourceNormalizer)),
            "AWS::Transfer::Server" => Some(Box::new(TransferResourceNormalizer)),
            "AWS::DataSync::Task" => Some(Box::new(DataSyncResourceNormalizer)),
            "AWS::FSx::FileSystem" => Some(Box::new(FsxResourceNormalizer)),
            "AWS::FSx::Backup" => Some(Box::new(FsxBackupResourceNormalizer)),
            "AWS::WorkSpaces::Workspace" => Some(Box::new(WorkSpacesResourceNormalizer)),
            "AWS::WorkSpaces::Directory" => Some(Box::new(WorkSpacesDirectoryResourceNormalizer)),
            "AWS::AppRunner::Service" => Some(Box::new(AppRunnerResourceNormalizer)),
            "AWS::AppRunner::Connection" => Some(Box::new(AppRunnerConnectionResourceNormalizer)),
            "AWS::GlobalAccelerator::Accelerator" => Some(Box::new(GlobalAcceleratorNormalizer)),
            "AWS::Connect::Instance" => Some(Box::new(ConnectNormalizer)),
            "AWS::Amplify::App" => Some(Box::new(AmplifyNormalizer)),
            "AWS::Lex::Bot" => Some(Box::new(LexBotNormalizer)),
            "AWS::Rekognition::Collection" => Some(Box::new(RekognitionCollectionNormalizer)),
            "AWS::Rekognition::StreamProcessor" => Some(Box::new(RekognitionStreamProcessorNormalizer)),
            "AWS::Polly::Voice" => Some(Box::new(PollyVoiceNormalizer)),
            "AWS::Polly::Lexicon" => Some(Box::new(PollyLexiconNormalizer)),
            "AWS::Polly::SynthesisTask" => Some(Box::new(PollySynthesisTaskNormalizer)),
            "AWS::ECR::Repository" => Some(Box::new(EcrRepositoryNormalizer)),
            "AWS::KMS::Key" => Some(Box::new(KmsKeyNormalizer)),
            "AWS::SecretsManager::Secret" => Some(Box::new(SecretsManagerSecretNormalizer)),
            "AWS::StepFunctions::StateMachine" => Some(Box::new(StepFunctionsStateMachineNormalizer)),
            "AWS::XRay::SamplingRule" => Some(Box::new(XRaySamplingRuleNormalizer)),
            "AWS::Shield::Protection" => Some(Box::new(ShieldProtectionNormalizer)),
            "AWS::Shield::Subscription" => Some(Box::new(ShieldSubscriptionNormalizer)),
            _ => None,
        }
    }

    pub fn get_supported_resource_types() -> Vec<&'static str> {
        vec![
            "AWS::EC2::Instance",
            "AWS::EC2::SecurityGroup",
            "AWS::EC2::VPC",
            "AWS::EC2::Volume",
            "AWS::EC2::Snapshot",
            "AWS::EC2::Image",
            "AWS::EC2::Subnet",
            "AWS::EC2::RouteTable",
            "AWS::EC2::NatGateway",
            "AWS::EC2::NetworkInterface",
            "AWS::EC2::VPCEndpoint",
            "AWS::EC2::NetworkAcl",
            "AWS::EC2::KeyPair",
            "AWS::EC2::InternetGateway",
            "AWS::EC2::TransitGateway",
            "AWS::EC2::VPCPeeringConnection", 
            "AWS::EC2::FlowLog",
            "AWS::EC2::VolumeAttachment",
            "AWS::ECS::FargateService",
            "AWS::ECS::FargateTask",
            "AWS::EKS::FargateProfile",
            "AWS::IAM::Role",
            "AWS::IAM::User",
            "AWS::IAM::Policy",
            "AWS::Bedrock::Model",
            "AWS::S3::Bucket",
            "AWS::CloudFormation::Stack",
            "AWS::RDS::DBInstance",
            "AWS::RDS::DBCluster",
            "AWS::RDS::DBSnapshot",
            "AWS::RDS::DBParameterGroup",
            "AWS::RDS::DBSubnetGroup",
            "AWS::Lambda::Function",
            "AWS::Lambda::LayerVersion",
            "AWS::Lambda::EventSourceMapping",
            "AWS::DynamoDB::Table",
            "AWS::CloudWatch::Alarm",
            "AWS::CloudWatch::Dashboard",
            "AWS::ApiGateway::RestApi",
            "AWS::SNS::Topic",
            "AWS::SQS::Queue",
            "AWS::ECS::Cluster",
            "AWS::ECS::Service",
            "AWS::ECS::Task",
            "AWS::ECS::TaskDefinition",
            "AWS::EKS::Cluster",
            "AWS::ElasticLoadBalancing::LoadBalancer",
            "AWS::ElasticLoadBalancingV2::LoadBalancer",
            "AWS::ElasticLoadBalancingV2::TargetGroup",
            "AWS::Logs::LogGroup",
            "AWS::ApiGatewayV2::Api",
            "AWS::Kinesis::Stream",
            "AWS::SageMaker::Endpoint",
            "AWS::SageMaker::TrainingJob",
            "AWS::SageMaker::Model",
            "AWS::Redshift::Cluster",
            "AWS::Glue::Job",
            "AWS::Athena::WorkGroup",
            "AWS::Route53::HostedZone",
            "AWS::EFS::FileSystem",
            "AWS::CloudTrail::Trail",
            "AWS::Config::ConfigurationRecorder",
            "AWS::Config::ConfigRule",
            "AWS::DataBrew::Job",
            "AWS::DataBrew::Dataset",
            "AWS::Detective::Graph",
            "AWS::AccessAnalyzer::Analyzer",
            "AWS::SSM::Parameter",
            "AWS::SSM::Document",
            "AWS::Backup::BackupPlan",
            "AWS::Backup::BackupVault",
            "AWS::Events::EventBus",
            "AWS::Events::Rule",
            "AWS::AppSync::GraphQLApi",
            "AWS::AmazonMQ::Broker",
            "AWS::MSK::Cluster",
            "AWS::LakeFormation::DataLakeSettings",
            "AWS::CodePipeline::Pipeline",
            "AWS::CodeBuild::Project",
            "AWS::CodeCommit::Repository",
            "AWS::IoT::Thing",
            "AWS::GreengrassV2::ComponentVersion",
            "AWS::Organizations::OrganizationalUnit",
            "AWS::Organizations::Policy",
            "AWS::CertificateManager::Certificate",
            "AWS::ACMPCA::CertificateAuthority",
            "AWS::AutoScaling::AutoScalingGroup",
            "AWS::AutoScaling::ScalingPolicy",
            "AWS::WAFv2::WebACL",
            "AWS::GuardDuty::Detector",
            "AWS::SecurityHub::Hub",
            "AWS::CloudFront::Distribution",
            "AWS::ElastiCache::CacheCluster",
            "AWS::ElastiCache::ReplicationGroup",
            "AWS::Neptune::DBCluster",
            "AWS::Neptune::DBInstance",
            "AWS::OpenSearchService::Domain",
            "AWS::Cognito::UserPool",
            "AWS::Cognito::IdentityPool",
            "AWS::Cognito::UserPoolClient",
            "AWS::Batch::JobQueue",
            "AWS::Batch::ComputeEnvironment",
            "AWS::KinesisFirehose::DeliveryStream",
            "AWS::QuickSight::DataSource",
            "AWS::QuickSight::Dashboard",
            "AWS::QuickSight::DataSet",
            "AWS::Macie::Session",
            "AWS::Inspector::Configuration",
            "AWS::Timestream::Database",
            "AWS::DocumentDB::Cluster",
            "AWS::Transfer::Server",
            "AWS::DataSync::Task",
            "AWS::FSx::FileSystem",
            "AWS::FSx::Backup",
            "AWS::WorkSpaces::Workspace",
            "AWS::WorkSpaces::Directory",
            "AWS::AppRunner::Service",
            "AWS::AppRunner::Connection",
            "AWS::GlobalAccelerator::Accelerator",
            "AWS::Connect::Instance",
            "AWS::Amplify::App",
            "AWS::Lex::Bot",
            "AWS::Rekognition::Collection",
            "AWS::Rekognition::StreamProcessor",
            "AWS::Polly::Voice",
            "AWS::Polly::Lexicon",
            "AWS::Polly::SynthesisTask",
            "AWS::ECR::Repository",
            "AWS::KMS::Key",
            "AWS::SecretsManager::Secret",
            "AWS::StepFunctions::StateMachine",
            "AWS::XRay::SamplingRule",
            "AWS::Shield::Protection",
            "AWS::Shield::Subscription",
        ]
    }
}

/// Helper functions for common normalization tasks
pub mod utils {
    use super::*;

    /// Extract display name from various AWS resource formats
    pub fn extract_display_name(raw: &serde_json::Value, fallback_id: &str) -> String {
        // Try different common name fields
        if let Some(name) = raw.get("Name").and_then(|v| v.as_str()) {
            return name.to_string();
        }

        if let Some(name) = raw.get("InstanceName").and_then(|v| v.as_str()) {
            return name.to_string();
        }

        if let Some(name) = raw.get("RoleName").and_then(|v| v.as_str()) {
            return name.to_string();
        }

        if let Some(name) = raw.get("UserName").and_then(|v| v.as_str()) {
            return name.to_string();
        }

        if let Some(name) = raw.get("PolicyName").and_then(|v| v.as_str()) {
            return name.to_string();
        }

        // Try to extract from tags
        if let Some(tags) = raw.get("Tags").and_then(|v| v.as_array()) {
            for tag in tags {
                if let (Some(key), Some(value)) = (
                    tag.get("Key").and_then(|k| k.as_str()),
                    tag.get("Value").and_then(|v| v.as_str()),
                ) {
                    if key == "Name" {
                        return value.to_string();
                    }
                }
            }
        }

        // Fallback to resource ID
        fallback_id.to_string()
    }

    /// Extract status from various AWS resource formats
    pub fn extract_status(raw: &serde_json::Value) -> Option<String> {
        // Try different common status fields
        if let Some(state) = raw.get("State").and_then(|v| v.as_str()) {
            return Some(state.to_string());
        }

        if let Some(state) = raw
            .get("InstanceState")
            .and_then(|s| s.get("Name"))
            .and_then(|n| n.as_str())
        {
            return Some(state.to_string());
        }

        if let Some(status) = raw.get("Status").and_then(|v| v.as_str()) {
            return Some(status.to_string());
        }

        None
    }

    /// Extract tags from AWS resource
    pub fn extract_tags(raw: &serde_json::Value) -> Vec<ResourceTag> {
        let mut tags = Vec::new();

        if let Some(tag_array) = raw.get("Tags").and_then(|v| v.as_array()) {
            for tag in tag_array {
                if let (Some(key), Some(value)) = (
                    tag.get("Key").and_then(|k| k.as_str()),
                    tag.get("Value").and_then(|v| v.as_str()),
                ) {
                    tags.push(ResourceTag {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
            }
        }

        tags
    }

    /// Create normalized properties object from raw AWS response
    pub fn create_normalized_properties(raw: &serde_json::Value) -> serde_json::Value {
        let mut normalized = serde_json::Map::new();

        // Extract common fields that we want to normalize across all resource types
        if let Some(id) = raw
            .get("InstanceId")
            .or_else(|| raw.get("VpcId"))
            .or_else(|| raw.get("GroupId"))
            .or_else(|| raw.get("RoleId"))
            .or_else(|| raw.get("UserId"))
            .or_else(|| raw.get("PolicyId"))
        {
            normalized.insert("id".to_string(), id.clone());
        }

        if let Some(arn) = raw.get("Arn") {
            normalized.insert("arn".to_string(), arn.clone());
        }

        if let Some(created) = raw.get("CreateDate").or_else(|| raw.get("LaunchTime")) {
            normalized.insert("created_date".to_string(), created.clone());
        }

        serde_json::Value::Object(normalized)
    }
}
