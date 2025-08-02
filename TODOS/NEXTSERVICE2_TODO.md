# AWS Service Implementation Roadmap - Phase 2
## Extended Service Coverage for Infrastructure Monitoring

## Overview

This document provides the next phase of AWS service implementation for comprehensive infrastructure monitoring coverage. These 7 additional milestones focus on enterprise-critical services not covered in the primary NEXTSERVICE_TODO.md roadmap.

**Current Status**: 43 services implemented + 25 services planned (NEXTSERVICE_TODO.md)  
**This Phase**: 7 additional milestones covering 20+ specialized services  
**Target Goal**: 90+ services for complete enterprise infrastructure monitoring

---

## 🎯 PHASE 10: Streaming and Data Transfer Infrastructure (Priority 1)
**Timeline**: 3-4 weeks  
**Goal**: Real-time data streaming and secure file transfer monitoring

### Milestone 10.1: Managed Streaming for Apache Kafka (Week 27)
**Objective**: Enterprise event streaming and real-time data pipeline monitoring

#### Tasks:
1. **Add Amazon MSK (Managed Streaming for Kafka)**
   - **Dependencies**: Add `aws-sdk-kafka = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/msk.rs`
   - **Resource Type**: `AWS::MSK::Cluster`
   - **SDK Calls**: `ListClusters()`, `DescribeCluster()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ClusterArn, ClusterName, State, KafkaVersion, NumberOfBrokerNodes, BrokerNodeGroupInfo
   - **Relationships**: Map to VPC subnets, security groups, IAM roles

2. **Add MSK Configuration and Connect**
   - **Service**: Extend `aws_services/msk.rs`
   - **Resource Types**: `AWS::MSK::Configuration`, `AWS::MSKConnect::Connector`
   - **Dependencies**: Add `aws-sdk-kafkaconnect = "1.67"` to Cargo.toml
   - **SDK Calls**: `ListConfigurations()`, `DescribeConfiguration()`, `ListConnectors()`, `DescribeConnector()`
   - **Implementation**: Add to MSKService
   - **Key Fields**: ConfigurationArn, ServerProperties; ConnectorArn, ConnectorState, KafkaConnect
   - **Relationships**: Configuration usage in clusters, connector data flow mapping

**Success Criteria**: Complete event streaming infrastructure and data pipeline monitoring

### Milestone 10.2: Secure File Transfer and Data Movement (Week 28)
**Objective**: Enterprise file transfer and data synchronization monitoring

#### Tasks:
1. **Add AWS Transfer Family**
   - **Dependencies**: Add `aws-sdk-transfer = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/transfer.rs`
   - **Resource Type**: `AWS::Transfer::Server`
   - **SDK Calls**: `ListServers()`, `DescribeServer()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ServerId, State, EndpointType, IdentityProviderType, Protocols, Domain
   - **Relationships**: Map to S3/EFS storage, IAM roles, VPC endpoints

2. **Add AWS DataSync**
   - **Dependencies**: Add `aws-sdk-datasync = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/datasync.rs`
   - **Resource Type**: `AWS::DataSync::Task`
   - **SDK Calls**: `ListTasks()`, `DescribeTask()`, `ListLocations()`, `DescribeLocationS3()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: TaskArn, Status, SourceLocationArn, DestinationLocationArn, CloudWatchLogGroupArn
   - **Relationships**: Map source/destination storage endpoints, execution history

**Success Criteria**: Secure file transfer and automated data movement monitoring

---

## 📊 PHASE 11: Time Series and Document Databases (Priority 2)
**Timeline**: 2-3 weeks  
**Goal**: Specialized database monitoring for time-series and document workloads

### Milestone 11.1: Time Series Database Monitoring (Week 29)
**Objective**: IoT and metrics database performance monitoring

#### Tasks:
1. **Add Amazon Timestream**
   - **Dependencies**: Add `aws-sdk-timestreamwrite = "1.67"`, `aws-sdk-timestreamquery = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/timestream.rs`
   - **Resource Type**: `AWS::Timestream::Database`
   - **SDK Calls**: `ListDatabases()`, `DescribeDatabase()`, `ListTables()`, `DescribeTable()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DatabaseName, TableName, RetentionProperties, MagneticStoreRetentionPeriod, MemoryStoreRetentionPeriod
   - **Relationships**: Map to KMS encryption keys, IAM policies

2. **Add Timestream InfluxDB**
   - **Dependencies**: Add `aws-sdk-timestreaminfluxdb = "1.67"` to Cargo.toml
   - **Service**: Extend `aws_services/timestream.rs`
   - **Resource Type**: `AWS::TimestreamInfluxDB::DbInstance`
   - **SDK Calls**: `ListDbInstances()`, `GetDbInstance()`
   - **Implementation**: Add to TimestreamService
   - **Key Fields**: Identifier, Name, DbInstanceType, AllocatedStorage, VpcSubnetIds, VpcSecurityGroupIds
   - **Relationships**: Map to VPC configuration, parameter groups

**Success Criteria**: Time-series database performance and capacity monitoring

### Milestone 11.2: Document Database Monitoring (Week 30)
**Objective**: MongoDB-compatible database monitoring

#### Tasks:
1. **Add Amazon DocumentDB**
   - **Dependencies**: Add `aws-sdk-docdb = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/documentdb.rs`
   - **Resource Type**: `AWS::DocDB::DBCluster`
   - **SDK Calls**: `DescribeDBClusters()`, `DescribeDBInstances()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DBClusterIdentifier, Engine, EngineVersion, Port, MasterUsername, Status, Endpoint
   - **Relationships**: Map to parameter groups, subnet groups, security groups

2. **Add DocumentDB Parameter and Subnet Groups**
   - **Service**: Extend `aws_services/documentdb.rs`
   - **Resource Types**: `AWS::DocDB::DBClusterParameterGroup`, `AWS::DocDB::DBSubnetGroup`
   - **SDK Calls**: `DescribeDBClusterParameterGroups()`, `DescribeDBSubnetGroups()`
   - **Implementation**: Add to DocumentDBService
   - **Key Fields**: DBClusterParameterGroupName, Family, Parameters; DBSubnetGroupName, VpcId, Subnets
   - **Relationships**: Configuration and network associations with clusters

**Success Criteria**: Document database performance and configuration monitoring

---

## 💻 PHASE 12: Virtual Desktop and Workspace Management (Priority 3)
**Timeline**: 2-3 weeks  
**Goal**: Virtual desktop infrastructure and workspace monitoring

### Milestone 12.1: Virtual Desktop Infrastructure (Week 31)
**Objective**: WorkSpaces and virtual desktop monitoring

#### Tasks:
1. **Add Amazon WorkSpaces**
   - **Dependencies**: Add `aws-sdk-workspaces = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/workspaces.rs`
   - **Resource Type**: `AWS::WorkSpaces::Workspace`
   - **SDK Calls**: `DescribeWorkspaces()`, `DescribeWorkspaceDirectories()`, `DescribeWorkspaceBundles()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: WorkspaceId, DirectoryId, UserName, BundleId, SubnetId, State, RootVolumeEncryptionEnabled
   - **Relationships**: Map to directories, bundles, VPC subnets

2. **Add WorkSpaces Web**
   - **Dependencies**: Add `aws-sdk-workspacesweb = "1.67"` to Cargo.toml
   - **Service**: Extend `aws_services/workspaces.rs`
   - **Resource Type**: `AWS::WorkSpacesWeb::Portal`
   - **SDK Calls**: `ListPortals()`, `GetPortal()`
   - **Implementation**: Add to WorkSpacesService
   - **Key Fields**: PortalArn, PortalEndpoint, PortalStatus, BrowserType, UserAccessLoggingSettings
   - **Relationships**: Map to identity providers, network settings

**Success Criteria**: Virtual desktop usage, performance, and security monitoring

### Milestone 12.2: Application Containerization and Serverless (Week 32)
**Objective**: Modern application deployment monitoring

#### Tasks:
1. **Add AWS App Runner**
   - **Dependencies**: Add `aws-sdk-apprunner = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/apprunner.rs`
   - **Resource Type**: `AWS::AppRunner::Service`
   - **SDK Calls**: `ListServices()`, `DescribeService()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ServiceArn, ServiceName, ServiceId, Status, SourceConfiguration, InstanceConfiguration
   - **Relationships**: Map to container registries, VPC connectors, auto scaling

2. **Add AWS Lightsail**
   - **Dependencies**: Add `aws-sdk-lightsail = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/lightsail.rs`
   - **Resource Type**: `AWS::Lightsail::Instance`
   - **SDK Calls**: `GetInstances()`, `GetContainerServices()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: Name, AvailabilityZone, BlueprintId, BundleId, State, PublicIpAddress
   - **Relationships**: Map to load balancers, databases, container services

**Success Criteria**: Simplified application deployment and container service monitoring

---

## 🛡️ PHASE 13: Advanced Security and Compliance (Priority 4)
**Timeline**: 2-3 weeks  
**Goal**: Enhanced security monitoring and compliance tracking

### Milestone 13.1: Data Security and Privacy (Week 33)
**Objective**: Data classification and privacy monitoring

#### Tasks:
1. **Add Amazon Macie**
   - **Dependencies**: Add `aws-sdk-macie2 = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/macie.rs`
   - **Resource Type**: `AWS::Macie::Session`
   - **SDK Calls**: `GetMacieSession()`, `ListClassificationJobs()`, `GetClassificationJob()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ServiceRole, Status, FindingPublishingFrequency, ClassificationJobs
   - **Relationships**: Map to S3 buckets, KMS keys, SNS topics

2. **Add AWS Inspector**
   - **Dependencies**: Add `aws-sdk-inspector2 = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/inspector.rs`
   - **Resource Type**: `AWS::Inspector::AssessmentTarget`
   - **SDK Calls**: `ListAccountPermissions()`, `ListFindings()`, `GetConfiguration()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: AccountId, Status, ResourceTypes, EcrConfiguration, Ec2Configuration
   - **Relationships**: Map to EC2 instances, ECR repositories, Lambda functions

**Success Criteria**: Data security classification and vulnerability assessment monitoring

### Milestone 13.2: DDoS Protection and Resource Management (Week 34)
**Objective**: DDoS protection and resource organization monitoring

#### Tasks:
1. **Add AWS Shield**
   - **Dependencies**: Add `aws-sdk-shield = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/shield.rs`
   - **Resource Type**: `AWS::Shield::Protection`
   - **SDK Calls**: `ListProtections()`, `DescribeProtection()`, `DescribeSubscription()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: ProtectionId, Name, ResourceArn, HealthCheckIds, SubscriptionState
   - **Relationships**: Map to protected resources (ELB, CloudFront, Route53, Global Accelerator)

2. **Add AWS Resource Groups**
   - **Dependencies**: Add `aws-sdk-resourcegroups = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/resourcegroups.rs`
   - **Resource Type**: `AWS::ResourceGroups::Group`
   - **SDK Calls**: `ListGroups()`, `GetGroup()`, `GetGroupQuery()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: GroupArn, Name, Description, ResourceQuery, Tags
   - **Relationships**: Track grouped resources across all AWS services

**Success Criteria**: DDoS protection status and resource organization monitoring

---

## 🎮 PHASE 14: Specialized Application Services (Priority 5)
**Timeline**: 2-3 weeks  
**Goal**: Gaming, media, and specialized application monitoring

### Milestone 14.1: Gaming and Media Services (Week 35)
**Objective**: Game server and media processing monitoring

#### Tasks:
1. **Add Amazon GameLift**
   - **Dependencies**: Add `aws-sdk-gamelift = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/gamelift.rs`
   - **Resource Type**: `AWS::GameLift::Fleet`
   - **SDK Calls**: `ListFleets()`, `DescribeFleetAttributes()`, `ListGameServerGroups()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: FleetId, FleetArn, Name, Status, InstanceType, EC2InstanceType, DesiredInstances
   - **Relationships**: Map to scaling policies, game sessions, player sessions

2. **Add AWS Elemental MediaLive**
   - **Dependencies**: Add `aws-sdk-medialive = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/medialive.rs`
   - **Resource Type**: `AWS::MediaLive::Channel`
   - **SDK Calls**: `ListChannels()`, `DescribeChannel()`, `ListInputs()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: Arn, ChannelClass, Destinations, EncoderSettings, InputAttachments, Name, State
   - **Relationships**: Map to inputs, outputs, MediaPackage channels

**Success Criteria**: Game server capacity and media streaming monitoring

### Milestone 14.2: Cost Management and Optimization (Week 36)
**Objective**: Cost monitoring and resource optimization

#### Tasks:
1. **Add AWS Cost and Billing Management**
   - **Dependencies**: Add `aws-sdk-costexplorer = "1.67"`, `aws-sdk-budgets = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/costmanagement.rs`
   - **Resource Type**: `AWS::Budgets::Budget`
   - **SDK Calls**: `DescribeBudgets()`, `GetCostAndUsage()`, `GetRightsizingRecommendation()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: BudgetName, BudgetType, TimeUnit, BudgetLimit, CostFilters, TimePeriod
   - **Relationships**: Map to cost allocation tags, services, usage patterns

2. **Add AWS Compute Optimizer**
   - **Dependencies**: Add `aws-sdk-computeoptimizer = "1.67"` to Cargo.toml
   - **Service**: Extend `aws_services/costmanagement.rs`
   - **Resource Type**: `AWS::ComputeOptimizer::RecommendationSummary`
   - **SDK Calls**: `GetRecommendationSummaries()`, `GetEC2InstanceRecommendations()`, `GetLambdaFunctionRecommendations()`
   - **Implementation**: Add to CostManagementService
   - **Key Fields**: ResourceType, AccountId, RecommendationsCount, SavingsOpportunity, CurrentInstanceType
   - **Relationships**: Map to EC2 instances, Lambda functions, Auto Scaling groups

**Success Criteria**: Cost optimization and resource rightsizing monitoring

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
    "AWS::MSK::Cluster" => {
        self.msk_service.describe_cluster(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Transfer::Server" => {
        self.transfer_service.describe_server(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Timestream::Database" => {
        self.timestream_service.describe_database(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::DocDB::DBCluster" => {
        self.documentdb_service.describe_db_cluster(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::WorkSpaces::Workspace" => {
        self.workspaces_service.describe_workspace(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::AppRunner::Service" => {
        self.apprunner_service.describe_service(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Macie::Session" => {
        self.macie_service.get_macie_session(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Shield::Protection" => {
        self.shield_service.describe_protection(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::ResourceGroups::Group" => {
        self.resourcegroups_service.get_group(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::GameLift::Fleet" => {
        self.gamelift_service.describe_fleet_attributes(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::Budgets::Budget" => {
        self.costmanagement_service.describe_budget(&resource.account_id, &resource.region, &resource.resource_id).await
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

**Phase 10 Completion Criteria:**
- Event streaming pipeline monitoring (MSK)
- Secure file transfer tracking (Transfer Family)
- Data synchronization monitoring (DataSync)

**Phase 11 Completion Criteria:**
- Time-series database performance monitoring (Timestream)
- Document database configuration tracking (DocumentDB)
- IoT and metrics data pipeline visibility

**Phase 12 Completion Criteria:**
- Virtual desktop infrastructure monitoring (WorkSpaces)
- Container service deployment tracking (App Runner)
- Simplified application platform monitoring (Lightsail)

**Phase 13 Completion Criteria:**
- Data security and privacy monitoring (Macie)
- Vulnerability assessment tracking (Inspector)
- DDoS protection status monitoring (Shield)
- Resource organization and tagging visibility (Resource Groups)

**Phase 14 Completion Criteria:**
- Game server capacity and performance monitoring (GameLift)
- Media streaming pipeline monitoring (MediaLive)
- Cost optimization and rightsizing recommendations (Cost Explorer, Compute Optimizer)
- Budget tracking and cost allocation monitoring

**Overall Goal**: Achieve 90+ AWS services coverage with specialized workload monitoring including streaming, databases, virtual desktops, security, gaming, and cost optimization.

## 🚀 Service Coverage Summary

**Combined Coverage (NEXTSERVICE_TODO.md + NEXTSERVICE2_TODO.md):**
- **Current**: 43 services implemented
- **NEXTSERVICE_TODO.md**: +25 services (Phases 6-9)
- **NEXTSERVICE2_TODO.md**: +20 services (Phases 10-14)
- **Total Target**: 90+ services with comprehensive enterprise infrastructure monitoring

This extended roadmap covers the remaining critical AWS services for complete enterprise infrastructure visibility, including specialized workloads, advanced security, cost optimization, and modern application platforms.