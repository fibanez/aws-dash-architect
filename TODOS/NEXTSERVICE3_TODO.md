# AWS Service Implementation Roadmap - Phase 3
## AI/ML, Enterprise Governance, and Advanced Infrastructure Services

## Overview

This document provides the third phase of AWS service implementation for comprehensive infrastructure monitoring coverage. These 7 additional milestones focus on AI/ML services, enterprise governance, and advanced infrastructure capabilities not covered in NEXTSERVICE_TODO.md or NEXTSERVICE2_TODO.md.

**Current Status**: 43 services implemented + 25 services (Phase 6-9) + 20 services (Phase 10-14)  
**This Phase**: 7 additional milestones covering 25+ AI/ML and enterprise services  
**Target Goal**: 115+ services for complete enterprise and AI/ML infrastructure monitoring

---

## 🤖 PHASE 15: AI/ML and Conversational Services (Priority 1)
**Timeline**: 3-4 weeks  
**Goal**: Artificial Intelligence and Machine Learning service monitoring

### Milestone 15.1: Conversational AI and Language Services (Week 37)
**Objective**: Chatbot, voice, and natural language processing monitoring

#### Tasks:
1. **Add Amazon Lex**
   - **Dependencies**: Add `aws-sdk-lexmodelsv2 = "1.67"`, `aws-sdk-lexruntimev2 = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/lex.rs`
   - **Resource Type**: `AWS::Lex::Bot`
   - **SDK Calls**: `ListBots()`, `DescribeBot()`, `ListBotVersions()`, `ListBotAliases()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: BotId, BotName, BotStatus, BotVersion, DataPrivacy, IdleSessionTTLInSeconds, RoleArn
   - **Relationships**: Map to IAM roles, CloudWatch logs, Lambda functions

2. **Add Amazon Polly**
   - **Dependencies**: Add `aws-sdk-polly = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/polly.rs`
   - **Resource Type**: `AWS::Polly::LexiconBucket`
   - **SDK Calls**: `DescribeVoices()`, `ListLexicons()`, `GetLexicon()`, `ListSpeechSynthesisTasks()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: VoiceId, LanguageCode, Gender, Engine, SupportedEngines, AdditionalLanguageCodes
   - **Relationships**: Map to S3 storage for audio output, lexicon management

**Success Criteria**: Conversational AI infrastructure and voice synthesis monitoring

### Milestone 15.2: Computer Vision and Language Processing (Week 38)
**Objective**: Visual AI and natural language understanding monitoring

#### Tasks:
1. **Add Amazon Rekognition**
   - **Dependencies**: Add `aws-sdk-rekognition = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/rekognition.rs`
   - **Resource Type**: `AWS::Rekognition::Collection`
   - **SDK Calls**: `ListCollections()`, `DescribeCollection()`, `ListStreamProcessors()`, `DescribeStreamProcessor()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: CollectionId, FaceCount, CreationTimestamp, Status, StatusMessage, StreamProcessorArn
   - **Relationships**: Map to S3 input/output buckets, IAM roles, Kinesis streams

2. **Add Amazon Comprehend**
   - **Dependencies**: Add `aws-sdk-comprehend = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/comprehend.rs`
   - **Resource Type**: `AWS::Comprehend::DocumentClassifier`
   - **SDK Calls**: `ListDocumentClassifiers()`, `DescribeDocumentClassifier()`, `ListEntitiesDetectionJobs()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DocumentClassifierArn, Status, LanguageCode, TrainingStartTime, TrainingEndTime, InputDataConfig
   - **Relationships**: Map to S3 training data, IAM roles, VPC configuration

**Success Criteria**: Computer vision and NLP service monitoring for AI workloads

---

## 🚀 PHASE 16: Enterprise ML and Fraud Detection (Priority 2)  
**Timeline**: 2-3 weeks  
**Goal**: Advanced ML services for business applications

### Milestone 16.1: ML Business Applications (Week 39)
**Objective**: Business-focused ML service monitoring

#### Tasks:
1. **Add Amazon Fraud Detector**
   - **Dependencies**: Add `aws-sdk-frauddetector = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/frauddetector.rs`
   - **Resource Type**: `AWS::FraudDetector::Detector`
   - **SDK Calls**: `GetDetectors()`, `GetModels()`, `GetRules()`, `GetVariables()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DetectorId, DetectorVersionId, Rules, ModelVersions, LastUpdatedTime, CreatedTime
   - **Relationships**: Map to event types, model endpoints, outcome configurations

2. **Add Amazon Personalize**
   - **Dependencies**: Add `aws-sdk-personalize = "1.67"`, `aws-sdk-personalizeruntime = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/personalize.rs`
   - **Resource Type**: `AWS::Personalize::Solution`
   - **SDK Calls**: `ListSolutions()`, `DescribeSolution()`, `ListDatasets()`, `DescribeDataset()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: SolutionArn, Name, Status, CreationDateTime, LastUpdatedDateTime, PerformHPO
   - **Relationships**: Map to datasets, campaigns, event trackers, S3 data sources

**Success Criteria**: Business ML application monitoring and fraud detection capabilities

### Milestone 16.2: Forecasting and Text Analysis (Week 40)
**Objective**: Predictive analytics and document processing monitoring

#### Tasks:
1. **Add Amazon Forecast**
   - **Dependencies**: Add `aws-sdk-forecast = "1.67"`, `aws-sdk-forecastquery = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/forecast.rs`
   - **Resource Type**: `AWS::Forecast::Dataset`
   - **SDK Calls**: `ListDatasets()`, `DescribeDataset()`, `ListPredictors()`, `DescribePredictor()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DatasetArn, DatasetName, Domain, DatasetType, Status, CreationTime, DataFrequency
   - **Relationships**: Map to S3 data sources, predictors, forecasts, IAM roles

2. **Add Amazon Textract**
   - **Dependencies**: Add `aws-sdk-textract = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/textract.rs`
   - **Resource Type**: `AWS::Textract::DocumentAnalysis`
   - **SDK Calls**: `GetDocumentAnalysis()`, `GetDocumentTextDetection()`, `StartDocumentAnalysis()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: JobId, JobStatus, DocumentLocation, JobTag, NotificationChannel, OutputConfig
   - **Relationships**: Map to S3 input/output buckets, SNS notifications, IAM roles

**Success Criteria**: Predictive analytics and document processing pipeline monitoring

---

## 🌐 PHASE 17: Network Optimization and Communication (Priority 3)
**Timeline**: 2-3 weeks  
**Goal**: Global network performance and communication services

### Milestone 17.1: Global Network Acceleration (Week 41)
**Objective**: Network performance optimization and global connectivity

#### Tasks:
1. **Add AWS Global Accelerator**
   - **Dependencies**: Add `aws-sdk-globalaccelerator = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/globalaccelerator.rs`
   - **Resource Type**: `AWS::GlobalAccelerator::Accelerator`
   - **SDK Calls**: `ListAccelerators()`, `DescribeAccelerator()`, `ListListeners()`, `DescribeListener()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: AcceleratorArn, Name, IpAddressType, Enabled, Status, DnsName, IpSets
   - **Relationships**: Map to listeners, endpoint groups, ALB/NLB targets

2. **Add AWS Network Manager**
   - **Dependencies**: Add `aws-sdk-networkmanager = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/networkmanager.rs`
   - **Resource Type**: `AWS::NetworkManager::GlobalNetwork`
   - **SDK Calls**: `DescribeGlobalNetworks()`, `GetNetworkResources()`, `GetNetworkTelemetry()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: GlobalNetworkArn, GlobalNetworkId, Description, State, Tags, CreatedAt
   - **Relationships**: Map to devices, links, sites, transit gateway attachments

**Success Criteria**: Global network performance monitoring and optimization tracking

### Milestone 17.2: Communication and Email Services (Week 42)
**Objective**: Enterprise communication and email delivery monitoring

#### Tasks:
1. **Add Amazon SES (Simple Email Service)**
   - **Dependencies**: Add `aws-sdk-sesv2 = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/ses.rs`
   - **Resource Type**: `AWS::SES::ConfigurationSet`
   - **SDK Calls**: `ListConfigurationSets()`, `GetConfigurationSet()`, `ListEmailIdentities()`, `GetEmailIdentity()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ConfigurationSetName, DeliveryOptions, ReputationOptions, TrackingOptions, SuppressionOptions
   - **Relationships**: Map to email identities, templates, sending quotas, bounce/complaint tracking

2. **Add Amazon Chime SDK**
   - **Dependencies**: Add `aws-sdk-chimesdkmeetings = "1.67"`, `aws-sdk-chimesdkmessaging = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/chime.rs`
   - **Resource Type**: `AWS::ChimeSDK::Meeting`
   - **SDK Calls**: `ListMeetings()`, `GetMeeting()`, `ListChannels()`, `DescribeChannel()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: MeetingId, ExternalMeetingId, MediaRegion, CreatedTimestamp, ChannelArn, Name
   - **Relationships**: Map to attendees, media pipelines, messaging channels

**Success Criteria**: Communication infrastructure and email delivery monitoring

---

## 🏗️ PHASE 18: Enterprise Governance and Migration (Priority 4)
**Timeline**: 2-3 weeks  
**Goal**: Enterprise governance, compliance, and migration monitoring

### Milestone 18.1: Service Governance and Compliance (Week 43)
**Objective**: Enterprise IT governance and service catalog monitoring

#### Tasks:
1. **Add AWS Service Catalog**
   - **Dependencies**: Add `aws-sdk-servicecatalog = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/servicecatalog.rs`
   - **Resource Type**: `AWS::ServiceCatalog::Portfolio`
   - **SDK Calls**: `ListPortfolios()`, `DescribePortfolio()`, `SearchProducts()`, `DescribeProduct()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: PortfolioId, PortfolioName, Description, ProviderName, CreatedTime, Tags
   - **Relationships**: Map to products, constraints, principals, launch paths

2. **Add AWS Control Tower**
   - **Dependencies**: Add `aws-sdk-controltower = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/controltower.rs`
   - **Resource Type**: `AWS::ControlTower::LandingZone`
   - **SDK Calls**: `GetLandingZone()`, `ListLandingZones()`, `ListEnabledControls()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: LandingZoneArn, Status, Version, Manifest, DriftStatus, LatestAvailableVersion
   - **Relationships**: Map to organizational units, guardrails, account factory

**Success Criteria**: Enterprise governance and multi-account management monitoring

### Milestone 18.2: Migration and Modernization (Week 44)
**Objective**: Application migration and modernization tracking

#### Tasks:
1. **Add AWS Migration Hub**
   - **Dependencies**: Add `aws-sdk-migrationhub = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/migrationhub.rs`
   - **Resource Type**: `AWS::MigrationHub::ProgressUpdateStream`
   - **SDK Calls**: `ListApplicationStates()`, `DescribeApplicationState()`, `ListMigrationTasks()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ApplicationId, ApplicationStatus, LastUpdatedTime, StatusDetail, ProgressPercent
   - **Relationships**: Map to migration tasks, discovery services, partner tools

2. **Add AWS Amplify (Full Platform)**
   - **Dependencies**: Add `aws-sdk-amplify = "1.67"`, `aws-sdk-amplifybackend = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/amplify.rs`
   - **Resource Type**: `AWS::Amplify::App`
   - **SDK Calls**: `ListApps()`, `GetApp()`, `ListBranches()`, `GetBranch()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: AppId, AppArn, Name, Platform, Repository, DefaultDomain, EnableBranchAutoBuild
   - **Relationships**: Map to branches, domains, webhooks, backend environments

**Success Criteria**: Migration progress tracking and modern application platform monitoring

---

## 🔧 PHASE 19: DevOps Integration and Monitoring (Priority 5)
**Timeline**: 2-3 weeks  
**Goal**: Advanced DevOps integration and operational monitoring

### Milestone 19.1: ChatOps and Distributed Tracing (Week 45)
**Objective**: DevOps communication and application performance monitoring

#### Tasks:
1. **Add AWS Chatbot**
   - **Dependencies**: Add `aws-sdk-chatbot = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/chatbot.rs`
   - **Resource Type**: `AWS::Chatbot::SlackChannelConfiguration`
   - **SDK Calls**: `DescribeSlackChannelConfigurations()`, `DescribeChimeWebhookConfigurations()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ConfigurationName, SlackTeamId, SlackChannelId, IamRoleArn, SnsTopicArns, LoggingLevel
   - **Relationships**: Map to SNS topics, IAM roles, CloudWatch alarms, Slack/Teams channels

2. **Add AWS X-Ray (Enhanced)**
   - **Dependencies**: Add `aws-sdk-xray = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/xray.rs`
   - **Resource Type**: `AWS::XRay::SamplingRule`
   - **SDK Calls**: `GetSamplingRules()`, `GetTraceGraph()`, `GetTraceSummaries()`, `GetServiceGraph()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: RuleName, Priority, FixedRate, ReservoirSize, ServiceName, ServiceType, HTTPMethod
   - **Relationships**: Map to Lambda functions, API Gateway, ECS services, application traces

**Success Criteria**: DevOps communication integration and distributed application tracing

### Milestone 19.2: Advanced Auto Scaling and Hardware Security (Week 46)
**Objective**: Dynamic scaling and cryptographic security monitoring

#### Tasks:
1. **Add AWS Auto Scaling (Enhanced)**
   - **Dependencies**: Add `aws-sdk-autoscaling = "1.67"`, `aws-sdk-applicationautoscaling = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/autoscaling.rs`
   - **Resource Type**: `AWS::AutoScaling::AutoScalingGroup`
   - **SDK Calls**: `DescribeAutoScalingGroups()`, `DescribeScalingPolicies()`, `DescribeScheduledActions()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: AutoScalingGroupName, LaunchTemplate, MinSize, MaxSize, DesiredCapacity, TargetGroupARNs
   - **Relationships**: Map to launch templates, target groups, scaling policies, CloudWatch metrics

2. **Add AWS CloudHSM**
   - **Dependencies**: Add `aws-sdk-cloudhsmv2 = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/cloudhsm.rs`
   - **Resource Type**: `AWS::CloudHSM::Cluster`
   - **SDK Calls**: `DescribeClusters()`, `DescribeBackups()`, `ListTags()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ClusterId, ClusterState, HsmType, SubnetMapping, SecurityGroup, PreCoPassword
   - **Relationships**: Map to VPC subnets, security groups, backup schedules, HSM instances

**Success Criteria**: Dynamic scaling optimization and hardware security module monitoring

---

## 📋 Implementation Guidelines

### **CRITICAL: Follow NEWSERVICES_TODO.md Patterns**
All implementations MUST follow the comprehensive patterns documented in `NEWSERVICES_TODO.md`, including:

1. **AWS SDK Field Access Patterns** - Proper Option<T> handling, boolean fields, enum conversion
2. **Pagination Patterns** - Standard paginator vs manual token pagination  
3. **Describe API Patterns** - Internal helper method patterns
4. **JSON Conversion** - Manual conversion, never serde_json::to_value() on AWS types
5. **Error Handling** - Proper fallback patterns and error logging
6. **Testing Patterns** - Edge case validation and compilation verification
7. **Enhanced Resource Integration** - CRITICAL: Ensure describe_resource routing integration

### **Service Implementation Checklist**
For each new service implementation:

- [ ] Add AWS SDK dependency to Cargo.toml
- [ ] Create service file in `aws_services/` directory
- [ ] Implement list and describe methods following pagination patterns
- [ ] Create normalizer in `normalizers/` directory
- [ ] Register normalizer in `normalizers/mod.rs`
- [ ] Add service to `aws_client.rs` routing
- [ ] Add resource types to `dialogs.rs` UI selection
- [ ] Add describe_resource routing integration
- [ ] Test compilation with `cargo check`
- [ ] Verify UI integration and data flow

### **Enhanced Resource Integration Pattern**
⚠️ **MANDATORY INTEGRATION STEP** - Add to describe_resource routing in `aws_client.rs`:

```rust
match resource.resource_type.as_str() {
    "AWS::Lex::Bot" => {
        self.lex_service.describe_bot(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Polly::LexiconBucket" => {
        self.polly_service.get_lexicon(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Rekognition::Collection" => {
        self.rekognition_service.describe_collection(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Comprehend::DocumentClassifier" => {
        self.comprehend_service.describe_document_classifier(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::FraudDetector::Detector" => {
        self.frauddetector_service.get_detector(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Personalize::Solution" => {
        self.personalize_service.describe_solution(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Forecast::Dataset" => {
        self.forecast_service.describe_dataset(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::GlobalAccelerator::Accelerator" => {
        self.globalaccelerator_service.describe_accelerator(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::SES::ConfigurationSet" => {
        self.ses_service.get_configuration_set(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::ServiceCatalog::Portfolio" => {
        self.servicecatalog_service.describe_portfolio(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::MigrationHub::ProgressUpdateStream" => {
        self.migrationhub_service.describe_application_state(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Amplify::App" => {
        self.amplify_service.get_app(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Chatbot::SlackChannelConfiguration" => {
        self.chatbot_service.describe_slack_channel_configuration(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::AutoScaling::AutoScalingGroup" => {
        self.autoscaling_service.describe_auto_scaling_group(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::CloudHSM::Cluster" => {
        self.cloudhsm_service.describe_cluster(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    // Add ALL new services with describe methods here
    _ => {
        Err(anyhow::anyhow!("Describe not implemented for resource type: {}", resource.resource_type))
    }
}
```

### **Testing Strategy**
- Use chunked testing approach: `./scripts/test-chunks.sh fast`
- Verify compilation before implementation: `cargo check`
- Test resource discovery in UI after implementation
- Validate describe functionality through UI detailed views

---

## 🎯 Success Metrics

**Phase 15 Completion Criteria:**
- Conversational AI infrastructure monitoring (Lex, Polly)
- Computer vision and NLP service tracking (Rekognition, Comprehend)
- AI/ML workload performance and usage monitoring

**Phase 16 Completion Criteria:**
- Business ML application monitoring (Fraud Detector, Personalize)
- Predictive analytics pipeline tracking (Forecast)
- Document processing workflow monitoring (Textract)

**Phase 17 Completion Criteria:**
- Global network performance optimization (Global Accelerator)
- Network infrastructure management (Network Manager)
- Communication and email delivery monitoring (SES, Chime)

**Phase 18 Completion Criteria:**
- Enterprise governance and service catalog tracking (Service Catalog, Control Tower)
- Migration progress and modernization monitoring (Migration Hub, Amplify)
- Multi-account management visibility

**Phase 19 Completion Criteria:**
- DevOps communication integration (Chatbot, X-Ray)
- Advanced auto scaling optimization monitoring
- Hardware security module tracking (CloudHSM)

**Overall Goal**: Achieve 115+ AWS services coverage with comprehensive AI/ML, enterprise governance, network optimization, and advanced DevOps capabilities.

## 🚀 Service Coverage Summary

**Combined Coverage (All Three Roadmaps):**
- **Current**: 43 services implemented
- **NEXTSERVICE_TODO.md**: +25 services (Phases 6-9) = 69 total
- **NEXTSERVICE2_TODO.md**: +20 services (Phases 10-14) = 89 total  
- **NEXTSERVICE3_TODO.md**: +25 services (Phases 15-19) = **115+ total services**

This comprehensive roadmap achieves complete enterprise AWS infrastructure monitoring including:
- **AI/ML Workloads**: 8 major AI services (Lex, Polly, Rekognition, Comprehend, Fraud Detector, Personalize, Forecast, Textract)
- **Network Optimization**: Global acceleration and network management
- **Enterprise Governance**: Service catalog, control tower, migration tracking
- **DevOps Integration**: ChatOps, distributed tracing, advanced scaling
- **Communication**: Email delivery, video conferencing, messaging
- **Security**: Hardware security modules, advanced compliance

Result: **Complete enterprise AWS infrastructure monitoring across all service categories and workload types.**