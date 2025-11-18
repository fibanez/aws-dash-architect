# AWS Services Implementation Roadmap
## Prioritized by AWS Service Popularity and Usage

This document lists AWS services that are **NOT YET IMPLEMENTED** in the AWS Explorer, prioritized by their real-world usage and popularity among AWS users.

## Summary

**Currently Implemented**: 156 resource types across 72 AWS services
**Hierarchical Child Resources**: 6 child resource types (Bedrock DataSource, IngestionJob, AgentAlias, AgentActionGroup, FlowAlias) with automatic nested querying
**Missing Resources**: 200+ critical resources across existing services
**Implementation Goal**: Complete coverage of all resources for top 100+ most-used AWS services

### Recent Updates
- ✅ **Hierarchical Child Resource System**: Implemented parent-child relationships for Bedrock resources (KnowledgeBase → DataSource → IngestionJob, Agent → AgentAlias/AgentActionGroup, Flow → FlowAlias)
- ✅ **Automatic Recursive Querying**: Children are automatically discovered when querying parent resources
- ✅ **Tree View Nesting**: Child resources display nested under their parents in the tree view

---

## 🔥 **TIER 1: CRITICAL MISSING RESOURCES** (Highest Priority)
*Essential resources missing from already-implemented services*

### **1. EC2 (Extended Resources)** ⭐⭐⭐⭐⭐
Missing critical EC2 resources that are essential for complete infrastructure management:
- `AWS::EC2::ElasticIP` - Elastic IP addresses (describe_addresses)
- `AWS::EC2::LaunchTemplate` - Launch Templates (describe_launch_templates)
- `AWS::EC2::PlacementGroup` - Placement Groups (describe_placement_groups)
- `AWS::EC2::ReservedInstance` - Reserved Instances (describe_reserved_instances)
- `AWS::EC2::SpotInstanceRequest` - Spot Instance Requests (describe_spot_instance_requests)
- `AWS::EC2::DHCPOptions` - DHCP Options Sets (describe_dhcp_options)
- `AWS::EC2::EgressOnlyInternetGateway` - Egress-Only Internet Gateways
- `AWS::EC2::VPNConnection` - VPN Connections (describe_vpn_connections)
- `AWS::EC2::VPNGateway` - VPN Gateways (describe_vpn_gateways)
- `AWS::EC2::CustomerGateway` - Customer Gateways (describe_customer_gateways)

### **2. CloudWatch (Complete Monitoring)** ⭐⭐⭐⭐⭐
Critical monitoring resources missing:
- `AWS::CloudWatch::Metric` - CloudWatch Metrics (list_metrics)
- `AWS::CloudWatch::CompositeAlarm` - Composite Alarms (describe_composite_alarms)
- `AWS::CloudWatch::InsightRule` - Contributor Insights Rules
- `AWS::CloudWatch::AnomalyDetector` - Anomaly Detectors
- `AWS::Logs::MetricFilter` - Log Metric Filters
- `AWS::Logs::SubscriptionFilter` - Log Subscription Filters
- `AWS::Logs::ResourcePolicy` - Resource Policies
- `AWS::Logs::QueryDefinition` - CloudWatch Insights Query Definitions

### **3. Glue (Complete Data Catalog)** ⭐⭐⭐⭐⭐
Missing all critical Glue resources except Jobs:
- `AWS::Glue::Crawler` - Glue Crawlers (list_crawlers)
- `AWS::Glue::Database` - Glue Databases (get_databases)
- `AWS::Glue::Table` - Glue Tables (get_tables)
- `AWS::Glue::Workflow` - Glue Workflows (list_workflows)
- `AWS::Glue::Trigger` - Glue Triggers (list_triggers)
- `AWS::Glue::Connection` - Glue Connections (get_connections)
- `AWS::Glue::DevEndpoint` - Development Endpoints
- `AWS::Glue::Partition` - Table Partitions
- `AWS::Glue::Schema` - Schema Registry
- `AWS::Glue::Registry` - Schema Registry
- `AWS::Glue::MLTransform` - ML Transforms

### **4. CloudFront (CDN Resources)** ⭐⭐⭐⭐⭐
Already has distributions, but missing:
- `AWS::CloudFront::StreamingDistribution` - RTMP Distributions
- `AWS::CloudFront::OriginAccessIdentity` - OAI for S3
- `AWS::CloudFront::CachePolicy` - Cache Policies
- `AWS::CloudFront::OriginRequestPolicy` - Origin Request Policies
- `AWS::CloudFront::ResponseHeadersPolicy` - Response Headers Policies
- `AWS::CloudFront::Function` - CloudFront Functions
- `AWS::CloudFront::KeyGroup` - Key Groups for signed URLs
- `AWS::CloudFront::PublicKey` - Public Keys

### **5. S3 (Extended Storage)** ⭐⭐⭐⭐⭐
Missing critical S3 resources:
- `AWS::S3::Object` - S3 Objects (list_objects_v2)
- `AWS::S3::MultipartUpload` - Multipart Uploads (list_multipart_uploads)
- `AWS::S3::AccessPoint` - S3 Access Points (list_access_points)
- `AWS::S3::StorageLens` - S3 Storage Lens configurations
- `AWS::S3::BucketPolicy` - Bucket Policies
- `AWS::S3::BucketVersioning` - Versioning Configuration
- `AWS::S3::BucketEncryption` - Encryption Configuration
- `AWS::S3::BucketLifecycle` - Lifecycle Rules

### **6. Lambda (Extended Serverless)** ⭐⭐⭐⭐⭐
- `AWS::Lambda::LayerVersion` - Lambda Layers (list_layers)
- `AWS::Lambda::EventSourceMapping` - Event Source Mappings (list_event_source_mappings)
- `AWS::Lambda::Version` - Function Versions (list_versions_by_function)
- `AWS::Lambda::Alias` - Function Aliases (list_aliases)
- `AWS::Lambda::CodeSigningConfig` - Code Signing Configurations
- `AWS::Lambda::FunctionUrlConfig` - Function URLs

### **7. DynamoDB (Complete NoSQL)** ⭐⭐⭐⭐⭐
- `AWS::DynamoDB::GlobalTable` - Global Tables (list_global_tables)
- `AWS::DynamoDB::Backup` - On-Demand Backups (list_backups)
- `AWS::DynamoDB::ContinuousBackup` - Point-in-Time Recovery
- `AWS::DynamoDB::TableReplica` - Table Replicas
- `AWS::DynamoDB::Export` - Table Exports
- `AWS::DynamoDB::Import` - Table Imports
- `AWS::DynamoDB::ContributorInsights` - Contributor Insights

### **8. RDS (Extended Database)** ⭐⭐⭐⭐⭐
- `AWS::RDS::DBProxy` - RDS Proxies (describe_db_proxies)
- `AWS::RDS::DBProxyTargetGroup` - Proxy Target Groups
- `AWS::RDS::OptionGroup` - Option Groups (describe_option_groups)
- `AWS::RDS::EventSubscription` - Event Subscriptions (describe_event_subscriptions)
- `AWS::RDS::DBClusterSnapshot` - Cluster Snapshots
- `AWS::RDS::DBSecurityGroup` - DB Security Groups (Classic)
- `AWS::RDS::DBClusterParameterGroup` - Cluster Parameter Groups
- `AWS::RDS::GlobalCluster` - Aurora Global Clusters

### **9. IAM (Complete Identity)** ⭐⭐⭐⭐⭐
- `AWS::IAM::Group` - IAM Groups (list_groups)
- `AWS::IAM::InstanceProfile` - Instance Profiles (list_instance_profiles)
- `AWS::IAM::AccessKey` - Access Keys (list_access_keys)
- `AWS::IAM::MFADevice` - MFA Devices (list_mfa_devices)
- `AWS::IAM::SAMLProvider` - SAML Identity Providers (list_saml_providers)
- `AWS::IAM::OpenIDConnectProvider` - OIDC Providers
- `AWS::IAM::ServerCertificate` - Server Certificates
- `AWS::IAM::VirtualMFADevice` - Virtual MFA Devices
- `AWS::IAM::ServiceLinkedRole` - Service-Linked Roles

### **10. CloudFormation (Extended IaC)** ⭐⭐⭐⭐
- `AWS::CloudFormation::StackSet` - Stack Sets (list_stack_sets)
- `AWS::CloudFormation::ChangeSet` - Change Sets (list_change_sets)
- `AWS::CloudFormation::TypeRegistration` - Resource Type Registrations
- `AWS::CloudFormation::StackInstance` - Stack Set Instances
- `AWS::CloudFormation::Macro` - CloudFormation Macros
- `AWS::CloudFormation::Export` - Stack Exports

### **11. ECS (Extended Container)** ⭐⭐⭐⭐
- `AWS::ECS::ContainerInstance` - Container Instances (list_container_instances)
- `AWS::ECS::CapacityProvider` - Capacity Providers (describe_capacity_providers)
- `AWS::ECS::TaskSet` - Task Sets
- `AWS::ECS::ServiceRegistry` - Service Discovery Registries
- `AWS::ECS::Attribute` - ECS Attributes

### **12. EKS (Extended Kubernetes)** ⭐⭐⭐⭐
- `AWS::EKS::NodeGroup` - Managed Node Groups (list_nodegroups)
- `AWS::EKS::Addon` - EKS Add-ons (list_addons)
- `AWS::EKS::IdentityProviderConfig` - Identity Provider Configurations
- `AWS::EKS::PodIdentityAssociation` - Pod Identity Associations

### **13. SNS (Complete Messaging)** ⭐⭐⭐⭐
- `AWS::SNS::Subscription` - Topic Subscriptions (list_subscriptions)
- `AWS::SNS::PlatformApplication` - Mobile Push Platform Applications (list_platform_applications)
- `AWS::SNS::PlatformEndpoint` - Mobile Push Endpoints
- `AWS::SNS::TopicPolicy` - Topic Access Policies
- `AWS::SNS::DataProtectionPolicy` - Data Protection Policies
- `AWS::SNS::SMSSandboxPhoneNumber` - SMS Sandbox Numbers

### **14. SQS (Extended Queuing)** ⭐⭐⭐⭐
- `AWS::SQS::QueuePolicy` - Queue Access Policies
- `AWS::SQS::DeadLetterQueue` - Dead Letter Queue Configurations
- `AWS::SQS::MessageAttribute` - Message Attributes
- `AWS::SQS::QueueTag` - Queue Tags

### **15. API Gateway (Complete REST)** ⭐⭐⭐⭐
- `AWS::ApiGateway::Resource` - API Resources (get_resources)
- `AWS::ApiGateway::Method` - API Methods (get_method)
- `AWS::ApiGateway::Deployment` - API Deployments (get_deployments)
- `AWS::ApiGateway::Stage` - API Stages (get_stages)
- `AWS::ApiGateway::ApiKey` - API Keys (get_api_keys)
- `AWS::ApiGateway::UsagePlan` - Usage Plans (get_usage_plans)
- `AWS::ApiGateway::UsagePlanKey` - Usage Plan Keys
- `AWS::ApiGateway::DomainName` - Custom Domain Names
- `AWS::ApiGateway::BasePathMapping` - Base Path Mappings
- `AWS::ApiGateway::VpcLink` - VPC Links
- `AWS::ApiGateway::RequestValidator` - Request Validators
- `AWS::ApiGateway::Model` - Request/Response Models
- `AWS::ApiGateway::Authorizer` - API Authorizers
- `AWS::ApiGateway::DocumentationPart` - API Documentation

### **16. API Gateway V2 (Complete HTTP/WebSocket)** ⭐⭐⭐⭐
- `AWS::ApiGatewayV2::Route` - API Routes (get_routes)
- `AWS::ApiGatewayV2::Integration` - API Integrations (get_integrations)
- `AWS::ApiGatewayV2::Deployment` - API Deployments (get_deployments)
- `AWS::ApiGatewayV2::Stage` - API Stages (get_stages)
- `AWS::ApiGatewayV2::DomainName` - Custom Domain Names
- `AWS::ApiGatewayV2::ApiMapping` - API Mappings
- `AWS::ApiGatewayV2::Authorizer` - API Authorizers
- `AWS::ApiGatewayV2::RouteResponse` - Route Responses
- `AWS::ApiGatewayV2::IntegrationResponse` - Integration Responses
- `AWS::ApiGatewayV2::Model` - Data Models
- `AWS::ApiGatewayV2::VpcLink` - VPC Links

### **17. Kinesis (Complete Streaming)** ⭐⭐⭐⭐
- `AWS::Kinesis::StreamConsumer` - Stream Consumers (list_stream_consumers)
- `AWS::Kinesis::Shard` - Stream Shards (list_shards)
- `AWS::KinesisAnalytics::Application` - Analytics Applications
- `AWS::KinesisAnalyticsV2::Application` - Analytics Applications V2
- `AWS::KinesisVideo::Stream` - Video Streams
- `AWS::KinesisVideo::SignalingChannel` - Signaling Channels

### **18. SageMaker (Extended ML)** ⭐⭐⭐⭐
- `AWS::SageMaker::NotebookInstance` - Notebook Instances (list_notebook_instances)
- `AWS::SageMaker::ProcessingJob` - Processing Jobs (list_processing_jobs)
- `AWS::SageMaker::Pipeline` - ML Pipelines (list_pipelines)
- `AWS::SageMaker::FeatureGroup` - Feature Groups (list_feature_groups)
- `AWS::SageMaker::ModelPackage` - Model Packages
- `AWS::SageMaker::ModelPackageGroup` - Model Package Groups
- `AWS::SageMaker::Project` - SageMaker Projects
- `AWS::SageMaker::Domain` - SageMaker Studio Domains
- `AWS::SageMaker::UserProfile` - Studio User Profiles
- `AWS::SageMaker::App` - SageMaker Apps
- `AWS::SageMaker::Image` - SageMaker Images
- `AWS::SageMaker::Algorithm` - ML Algorithms
- `AWS::SageMaker::HyperParameterTuningJob` - Hyperparameter Tuning Jobs
- `AWS::SageMaker::LabelingJob` - Ground Truth Labeling Jobs
- `AWS::SageMaker::TransformJob` - Batch Transform Jobs
- `AWS::SageMaker::CompilationJob` - Model Compilation Jobs
- `AWS::SageMaker::AutoMLJob` - AutoML Jobs
- `AWS::SageMaker::ExperimentTrialComponent` - Experiment Components
- `AWS::SageMaker::ModelBiasJobDefinition` - Model Bias Monitoring
- `AWS::SageMaker::ModelQualityJobDefinition` - Model Quality Monitoring

### **19. CodeBuild (Extended CI)** ⭐⭐⭐⭐
- `AWS::CodeBuild::Build` - Build Executions (list_builds)
- `AWS::CodeBuild::ReportGroup` - Test Report Groups (list_report_groups)
- `AWS::CodeBuild::Report` - Test Reports
- `AWS::CodeBuild::SourceCredentials` - Source Repository Credentials
- `AWS::CodeBuild::Webhook` - Build Webhooks

---

## 🚀 **TIER 2: HIGH-VALUE NEW SERVICES** (High Priority)
*Very popular services not yet implemented*

### **Data & Analytics**
20. **Amazon EMR (Registration Fix Needed)** ⭐⭐⭐⭐
    - `AWS::EMR::Cluster` - EMR Clusters (list_clusters implemented but NOT registered)
    - `AWS::EMR::Step` - EMR Steps (list_steps)
    - `AWS::EMR::InstanceGroup` - Instance Groups
    - `AWS::EMR::InstanceFleet` - Instance Fleets
    - `AWS::EMR::SecurityConfiguration` - Security Configurations
    - `AWS::EMR::Studio` - EMR Studios
    - Note: Service has list_clusters but needs registration in get_default_resource_types()

### **AI/ML Services**
21. **Amazon Comprehend** ⭐⭐⭐⭐
    - `AWS::Comprehend::DocumentClassifier`
    - `AWS::Comprehend::EntityRecognizer`
    - NLP service widely used for text analysis

22. **Amazon Textract** ⭐⭐⭐⭐
    - `AWS::Textract::DocumentAnalysis`
    - Document processing service

### **Communication & Messaging**
23. **Amazon SES (Simple Email Service)** ⭐⭐⭐⭐
    - `AWS::SES::ConfigurationSet`
    - `AWS::SES::EmailIdentity`
    - `AWS::SES::Template`
    - `AWS::SES::ReceiptRule`
    - `AWS::SES::ReceiptFilter`
    - Essential for application email functionality

24. **Amazon Pinpoint** ⭐⭐⭐⭐
    - `AWS::Pinpoint::App`
    - `AWS::Pinpoint::Campaign`
    - `AWS::Pinpoint::Segment`
    - `AWS::Pinpoint::Journey`
    - Mobile engagement and analytics

### **Developer Tools**
25. **AWS CodeStar** ⭐⭐⭐
    - `AWS::CodeStar::Project`
    - Integrated development workflow

26. **AWS CodeGuru** ⭐⭐⭐
    - `AWS::CodeGuru::ReviewAssociation`
    - `AWS::CodeGuru::Profiler`
    - AI-powered code review

---

## 🎯 **TIER 3: SPECIALIZED SERVICES** (Medium Priority)
*Services used in specific industries or advanced use cases*

### **Enterprise & Migration**
27. **AWS Migration Hub** ⭐⭐⭐
    - `AWS::MigrationHub::ProgressUpdateStream`
    - `AWS::MigrationHub::MigrationTask`
    - Application migration tracking

28. **AWS Service Catalog** ⭐⭐⭐
    - `AWS::ServiceCatalog::Portfolio`
    - `AWS::ServiceCatalog::Product`
    - `AWS::ServiceCatalog::ProvisionedProduct`
    - `AWS::ServiceCatalog::Constraint`
    - Enterprise IT governance

29. **AWS Control Tower** ⭐⭐⭐
    - `AWS::ControlTower::LandingZone`
    - `AWS::ControlTower::Guardrail`
    - Multi-account governance

### **Cost Management**
30. **AWS Budgets** ⭐⭐⭐
    - `AWS::Budgets::Budget`
    - `AWS::Budgets::BudgetAction`
    - Cost monitoring and alerts

31. **AWS Cost Explorer** ⭐⭐⭐
    - `AWS::CostExplorer::CostCategory`
    - `AWS::CostExplorer::AnomalyMonitor`
    - `AWS::CostExplorer::AnomalySubscription`
    - Cost analysis and optimization

### **Advanced Security**
32. **AWS Network Firewall** ⭐⭐⭐
    - `AWS::NetworkFirewall::Firewall`
    - `AWS::NetworkFirewall::FirewallPolicy`
    - `AWS::NetworkFirewall::RuleGroup`
    - Network security for VPCs

33. **AWS Resource Access Manager** ⭐⭐⭐
    - `AWS::RAM::ResourceShare`
    - `AWS::RAM::Permission`
    - Cross-account resource sharing

### **Business Applications**
34. **Amazon WorkMail** ⭐⭐⭐
    - `AWS::WorkMail::Organization`
    - `AWS::WorkMail::User`
    - `AWS::WorkMail::Group`
    - Business email service

35. **Amazon Chime SDK** ⭐⭐⭐
    - `AWS::ChimeSDK::Meeting`
    - `AWS::ChimeSDK::Channel`
    - `AWS::ChimeSDK::AppInstance`
    - Video conferencing integration

---

## 🔧 **TIER 4: NICHE & EMERGING SERVICES** (Lower Priority)
*Specialized services for specific use cases or newer offerings*

### **AI/ML Specialized**
36. **Amazon Forecast** ⭐⭐
    - `AWS::Forecast::Dataset`
    - `AWS::Forecast::DatasetGroup`
    - `AWS::Forecast::Predictor`
    - Time series forecasting

37. **Amazon Personalize** ⭐⭐
    - `AWS::Personalize::Solution`
    - `AWS::Personalize::Dataset`
    - `AWS::Personalize::Campaign`
    - Recommendation engines

38. **Amazon Fraud Detector** ⭐⭐
    - `AWS::FraudDetector::Detector`
    - `AWS::FraudDetector::Model`
    - Fraud detection ML

### **Industry-Specific**
39. **Amazon GameLift** ⭐⭐
    - `AWS::GameLift::Fleet`
    - `AWS::GameLift::GameSession`
    - `AWS::GameLift::MatchmakingConfiguration`
    - Gaming infrastructure

40. **AWS Media Services** ⭐⭐
    - `AWS::MediaLive::Channel`
    - `AWS::MediaPackage::Channel`
    - `AWS::MediaConvert::JobTemplate`
    - `AWS::MediaStore::Container`
    - Video processing and delivery

41. **AWS Ground Station** ⭐
    - `AWS::GroundStation::MissionProfile`
    - `AWS::GroundStation::DataflowEndpointGroup`
    - Satellite communication

### **Development & Testing**
42. **AWS Device Farm** ⭐⭐
    - `AWS::DeviceFarm::Project`
    - `AWS::DeviceFarm::DevicePool`
    - Mobile app testing

43. **AWS CodeGuru Profiler** ⭐⭐
    - `AWS::CodeGuruProfiler::ProfilingGroup`
    - Application performance profiling

### **Infrastructure Extensions**
44. **AWS Outposts** ⭐⭐
    - `AWS::Outposts::Outpost`
    - `AWS::Outposts::Site`
    - Hybrid cloud infrastructure

45. **AWS Wavelength** ⭐
    - `AWS::EC2::CarrierGateway`
    - `AWS::Wavelength::Zone`
    - Ultra-low latency applications

### **Advanced Analytics**
46. **Amazon Managed Blockchain** ⭐
    - `AWS::ManagedBlockchain::Network`
    - `AWS::ManagedBlockchain::Node`
    - `AWS::ManagedBlockchain::Member`
    - Blockchain infrastructure

47. **AWS Lake Formation** (Extended) ⭐⭐
    - `AWS::LakeFormation::Resource`
    - `AWS::LakeFormation::Permissions`
    - `AWS::LakeFormation::DataCellsFilter`
    - Data lake permissions

48. **Amazon Braket** ⭐
    - `AWS::Braket::Device`
    - `AWS::Braket::QuantumTask`
    - Quantum computing service

---

## 📊 **Implementation Priority Matrix**

| Priority Level | Resource Count | Use Case | Timeline |
|---------------|---------------|----------|----------|
| **Tier 1 (Critical Resources)** | 200+ resources | Complete existing services | 8-10 weeks |
| **Tier 2 (High-Value Services)** | 7 new services | Advanced enterprise features | 4-6 weeks |
| **Tier 3 (Specialized)** | 9 services | Industry-specific needs | 6-8 weeks |
| **Tier 4 (Niche)** | 13+ services | Emerging/specialized use cases | 8+ weeks |

---

## 🎯 **Implementation Strategy**

### **Phase 1: Complete Core Services (Weeks 1-10)**
Focus on Tier 1 - completing all missing resources for already-implemented services:
- EC2 complete infrastructure resources
- CloudWatch complete monitoring suite
- Glue complete data catalog
- CloudFront CDN features
- S3 advanced features
- Lambda serverless extensions
- DynamoDB global features
- RDS proxy and advanced features
- IAM complete identity management
- Complete API Gateway REST/HTTP features

### **Phase 2: High-Value New Services (Weeks 11-16)**  
Implement Tier 2 services for advanced enterprise capabilities:
- EMR (fix registration), SES, Comprehend, Textract
- Advanced developer tools and communication services

### **Phase 3: Specialized Solutions (Weeks 17-24)**
Add Tier 3 services for specific industry needs:
- Migration tools, cost management, advanced security
- Business applications and governance tools

### **Phase 4: Emerging Technologies (Ongoing)**
Implement Tier 4 services as needed:
- Cutting-edge AI/ML services, industry-specific tools
- Experimental and quantum computing services

---

## 📋 **Success Metrics**

**Tier 1 Completion**: 90%+ coverage of AWS resource types for core services  
**Tier 2 Completion**: 95%+ coverage of enterprise AWS environments  
**Tier 3 Completion**: 98%+ coverage of specialized industry use cases  
**Tier 4 Completion**: Complete AWS service ecosystem coverage

**Final Goal**: Support for 350+ AWS resource types covering 99%+ of real-world AWS usage patterns.

---

*Note: Star ratings (⭐) represent relative popularity and usage frequency in typical AWS environments based on AWS documentation, industry surveys, and community usage patterns.*