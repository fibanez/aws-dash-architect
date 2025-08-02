# AWS Service Implementation Roadmap

## Overview

This document provides a structured implementation plan for expanding AWS service coverage in the resource explorer. All implementations should follow the comprehensive patterns documented in `NEWSERVICES_TODO.md`.

**Current Status**: 43 services implemented with 90+ resource types  
**Target Goal**: Complete coverage of critical infrastructure monitoring services

---

## 🎉 COMPLETED PHASES - Summary

### 🚀 Phase 1: Critical Infrastructure Services ✅
**Milestone 1.1: VPC Networking Suite** ✅ - Implemented complete EC2 VPC networking with subnets, route tables, gateways, and network interfaces  
**Milestone 1.2: Load Balancer Services** ✅ - Added Classic ELB, ALB/NLB, and Target Groups with comprehensive load balancing visibility  
**Milestone 1.3: VPC Security Resources** ✅ - Implemented VPC endpoints, Network ACLs, and Key Pairs for complete security visibility

### 📦 Phase 2: Core Application Services ✅  
**Milestone 2.1: Enhanced Storage Services** ✅ - Enhanced S3 with bucket configurations (policies, encryption, versioning, lifecycle rules)  
**Milestone 2.2: Database Services** ✅ - Complete RDS support with instances, clusters, snapshots, parameter/subnet groups  
**Milestone 2.3: Container Services** ✅ - Full ECS implementation with clusters, services, tasks, and task definitions  
**Milestone 2.4: Enhanced Lambda Services** ✅ - Extended Lambda with layers and event source mappings for complete serverless ecosystem

### 📊 Phase 3: Monitoring & Operations ✅
**Milestone 3.1: Enhanced CloudWatch Services** ✅ - Implemented CloudWatch alarms and dashboards for comprehensive monitoring  
**Milestone 3.2: Systems Manager Implementation** ✅ - Added SSM parameters and documents for configuration management  
**Milestone 3.3: AWS Backup Implementation** ✅ - Implemented backup plans and vaults for data protection visibility

### 🏗️ Phase 4: Modern Application Services ✅
**Milestone 4.1: Event-Driven Architecture** ✅ - Added EventBridge (event buses, rules) and AppSync GraphQL APIs  
**Milestone 4.2: Message Queuing Services** ✅ - Implemented Amazon MQ brokers and enhanced SQS with DLQ relationships  
**Milestone 4.3: Developer Tools** ✅ - Complete CI/CD pipeline with CodePipeline, CodeBuild, and CodeCommit

### 🚀 Phase 5: Advanced & Emerging Services ✅
**Milestone 5.1: IoT and Edge Services** ✅ - Added IoT Core things and Greengrass component versions for edge computing  
**Milestone 5.2: Enhanced AI/ML Services** ✅ - Extended SageMaker with training jobs/models and enhanced Bedrock foundation models  
**Milestone 5.3: Governance and Compliance** ✅ - Implemented AWS Organizations with organizational units and service control policies

---

## 🔥 NEW PHASE 6: Security & Performance Infrastructure (Priority 1)
**Timeline**: 3-4 weeks  
**Goal**: Complete security posture monitoring and performance optimization visibility

### Milestone 6.1: Certificate and Security Management (Week 18)
**Objective**: SSL/TLS certificate lifecycle and security monitoring

#### Tasks:
1. **✅ Add AWS Certificate Manager (ACM)** - **COMPLETED**
   - **✅ Dependencies**: Add `aws-sdk-acm = "1.76"` to Cargo.toml
   - **✅ Service**: Create `aws_services/acm.rs`
   - **✅ Resource Type**: `AWS::CertificateManager::Certificate`
   - **✅ SDK Calls**: `ListCertificates()`, `DescribeCertificate()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: CertificateArn, DomainName, Status, Issuer, NotBefore, NotAfter, KeyUsages
   - **✅ Relationships**: Map to load balancers, CloudFront distributions, API Gateway

2. **Add Private Certificate Authority (ACM PCA)**
   - **Dependencies**: Add `aws-sdk-acmpca = "1.67"` to Cargo.toml
   - **Service**: Extend `aws_services/acm.rs`
   - **Resource Type**: `AWS::ACMPCA::CertificateAuthority`
   - **SDK Calls**: `ListCertificateAuthorities()`, `DescribeCertificateAuthority()`
   - **Implementation**: Add to ACMService
   - **Key Fields**: CertificateAuthorityArn, Status, Type, KeyAlgorithm, CreatedAt
   - **Relationships**: Certificate issuance hierarchy tracking

**Success Criteria**: Complete certificate lifecycle and PKI infrastructure visibility

### Milestone 6.2: Web Application Firewall (Week 19)
**Objective**: Web application security and threat monitoring

#### Tasks:
1. **✅ Add WAFv2 Web Application Firewall** - **COMPLETED**
   - **✅ Dependencies**: Add `aws-sdk-wafv2 = "1.75"` to Cargo.toml
   - **✅ Service**: Create `aws_services/wafv2.rs`
   - **✅ Resource Type**: `AWS::WAFv2::WebACL`
   - **✅ SDK Calls**: `ListWebACLs()`, `GetWebACL()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: WebACLArn, Name, Scope, DefaultAction, Rules, ManagedRuleGroupConfigs
   - **✅ Relationships**: Map to CloudFront, ALB, API Gateway protected resources

2. **Add WAFv2 IP Sets and Rule Groups**
   - **Service**: Extend `aws_services/wafv2.rs`
   - **Resource Types**: `AWS::WAFv2::IPSet`, `AWS::WAFv2::RuleGroup`
   - **SDK Calls**: `ListIPSets()`, `GetIPSet()`, `ListRuleGroups()`, `GetRuleGroup()`
   - **Implementation**: Add to WAFv2Service
   - **Key Fields**: IPSetArn, Addresses, Scope; RuleGroupArn, Rules, Capacity
   - **Relationships**: Usage tracking in WebACLs

**Success Criteria**: Complete web application security posture visibility

### Milestone 6.3: Threat Detection and Security (Week 20)
**Objective**: Advanced threat detection and security monitoring

#### Tasks:
1. **✅ Add Amazon GuardDuty** - **COMPLETED**
   - **✅ Dependencies**: Add `aws-sdk-guardduty = "1.67"` to Cargo.toml
   - **✅ Service**: Create `aws_services/guardduty.rs`
   - **✅ Resource Type**: `AWS::GuardDuty::Detector`
   - **✅ SDK Calls**: `ListDetectors()`, `GetDetector()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: DetectorId, Status, ServiceRole, FindingPublishingFrequency, DataSources
   - **✅ Relationships**: Threat findings and member account associations

2. **Add Security Hub**
   - **Dependencies**: Add `aws-sdk-securityhub = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/securityhub.rs`
   - **Resource Type**: `AWS::SecurityHub::Hub`
   - **SDK Calls**: `DescribeHub()`, `GetEnabledStandards()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: HubArn, SubscribedAt, AutoEnableControls, EnabledStandards
   - **Relationships**: Security finding aggregation from multiple services

**Success Criteria**: Comprehensive threat detection and security posture monitoring

---

## 📈 Phase 7: Performance and Caching Infrastructure (Priority 2)
**Timeline**: 2-3 weeks  
**Goal**: Performance optimization and content delivery monitoring

### Milestone 7.1: Content Delivery and Caching (Week 21)
**Objective**: CDN and edge location performance monitoring

#### Tasks:
1. **✅ Add Amazon CloudFront** - **COMPLETED**
   - **✅ Dependencies**: Add `aws-sdk-cloudfront = "1.67"` to Cargo.toml
   - **✅ Service**: Create `aws_services/cloudfront.rs`
   - **✅ Resource Type**: `AWS::CloudFront::Distribution`
   - **✅ SDK Calls**: `ListDistributions()`, `GetDistribution()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: DistributionId, DomainName, Status, Origins, DefaultCacheBehavior, PriceClass
   - **✅ Relationships**: Map to S3 origins, ALB origins, custom origins

2. **Add CloudFront Functions and Cache Policies**
   - **Service**: Extend `aws_services/cloudfront.rs`
   - **Resource Types**: `AWS::CloudFront::Function`, `AWS::CloudFront::CachePolicy`
   - **SDK Calls**: `ListFunctions()`, `GetFunction()`, `ListCachePolicies()`, `GetCachePolicy()`
   - **Implementation**: Add to CloudFrontService
   - **Key Fields**: FunctionArn, Runtime, FunctionCode; PolicyId, PolicyConfig, CacheBehaviors
   - **Relationships**: Function associations with distributions, policy usage tracking

**Success Criteria**: Complete CDN performance and edge location monitoring

### Milestone 7.2: In-Memory Caching (Week 22)
**Objective**: Cache cluster performance and memory optimization monitoring

#### Tasks:
1. **✅ Add Amazon ElastiCache** - **COMPLETED**
   - **✅ Dependencies**: Add `aws-sdk-elasticache = "1.67"` to Cargo.toml
   - **✅ Service**: Create `aws_services/elasticache.rs`
   - **✅ Resource Type**: `AWS::ElastiCache::CacheCluster`
   - **✅ SDK Calls**: `DescribeCacheClusters()`, `DescribeReplicationGroups()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: CacheClusterId, Engine, EngineVersion, CacheNodeType, NumCacheNodes, Status
   - **✅ Relationships**: Map to parameter groups, subnet groups, security groups

2. **Add ElastiCache Parameter and Subnet Groups**
   - **Service**: Extend `aws_services/elasticache.rs`
   - **Resource Types**: `AWS::ElastiCache::ParameterGroup`, `AWS::ElastiCache::SubnetGroup`
   - **SDK Calls**: `DescribeCacheParameterGroups()`, `DescribeCacheSubnetGroups()`
   - **Implementation**: Add to ElastiCacheService
   - **Key Fields**: ParameterGroupName, Family, Parameters; SubnetGroupName, VpcId, Subnets
   - **Relationships**: Configuration and network associations with clusters

**Success Criteria**: Complete cache performance monitoring and memory optimization visibility

---

## 🎯 Phase 8: Specialized Data Services (Priority 3)
**Timeline**: 2-3 weeks  
**Goal**: Specialized database and analytics service monitoring

### Milestone 8.1: Graph and Search Databases (Week 23) ✅
**Objective**: Specialized database monitoring for complex data relationships

#### Tasks:
1. **✅ Add Amazon Neptune**
   - **✅ Dependencies**: Add `aws-sdk-neptune = "1.67"` to Cargo.toml
   - **✅ Service**: Create `aws_services/neptune.rs`
   - **✅ Resource Types**: `AWS::Neptune::DBCluster`, `AWS::Neptune::DBInstance`
   - **✅ SDK Calls**: `DescribeDBClusters()`, `DescribeDBInstances()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: DBClusterIdentifier, Engine, Status, Endpoint, ReaderEndpoint, ClusterMembers
   - **✅ Relationships**: Map to parameter groups, subnet groups, security groups

2. **✅ Add Amazon OpenSearch**
   - **✅ Dependencies**: Add `aws-sdk-opensearch = "1.67"` to Cargo.toml
   - **✅ Service**: Create `aws_services/opensearch.rs`
   - **✅ Resource Type**: `AWS::OpenSearchService::Domain`
   - **✅ SDK Calls**: `ListDomainNames()`, `DescribeDomain()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: DomainName, EngineVersion, InstanceType, InstanceCount, StorageType
   - **✅ Relationships**: Map to VPC, IAM roles, KMS keys, Cognito

**✅ Success Criteria**: Specialized database performance and configuration monitoring

### Milestone 8.2: Identity and User Management (Week 24)
**Objective**: User authentication and identity federation monitoring

#### Tasks:
1. **✅ Add Amazon Cognito**
   - **✅ Dependencies**: Add `aws-sdk-cognitoidentityprovider = "1.67"` to Cargo.toml
   - **✅ Service**: Create `aws_services/cognito.rs`
   - **✅ Resource Type**: `AWS::Cognito::UserPool`
   - **✅ SDK Calls**: `ListUserPools()`, `DescribeUserPool()`
   - **✅ Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **✅ Key Fields**: UserPoolId, UserPoolName, Policies, Schema, EmailConfiguration, SmsConfiguration
   - **✅ Relationships**: Map to identity pools, user pool clients, domains

2. **✅ Add Cognito Identity Pools and Clients**
   - **✅ Dependencies**: Add `aws-sdk-cognitoidentity = "1.67"` to Cargo.toml
   - **✅ Service**: Extend `aws_services/cognito.rs`
   - **✅ Resource Types**: `AWS::Cognito::IdentityPool`, `AWS::Cognito::UserPoolClient`
   - **✅ SDK Calls**: `ListIdentityPools()`, `DescribeIdentityPool()`, `ListUserPoolClients()`
   - **✅ Implementation**: Add to CognitoService
   - **✅ Key Fields**: IdentityPoolId, IdentityProviders, Roles; ClientId, ClientName, GenerateSecret
   - **✅ Relationships**: Authentication flow mapping and federated identity tracking

**✅ Success Criteria**: Complete user identity and authentication monitoring

---

## 🚀 Phase 9: Analytics and Business Intelligence (Priority 4)
**Timeline**: 2-3 weeks  
**Goal**: Data analytics and business intelligence monitoring

### Milestone 9.1: Batch Processing and Analytics (Week 25)
**Objective**: Batch workload and data processing monitoring

#### Tasks:
1. **Add AWS Batch**
   - **Dependencies**: Add `aws-sdk-batch = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/batch.rs`
   - **Resource Type**: `AWS::Batch::JobQueue`
   - **SDK Calls**: `DescribeJobQueues()`, `DescribeComputeEnvironments()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: JobQueueName, State, Priority, ComputeEnvironmentOrder
   - **Relationships**: Map to compute environments, job definitions

2. **Add Kinesis Data Firehose**
   - **Dependencies**: Add `aws-sdk-firehose = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/kinesisfirehose.rs`
   - **Resource Type**: `AWS::KinesisFirehose::DeliveryStream`
   - **SDK Calls**: `ListDeliveryStreams()`, `DescribeDeliveryStream()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DeliveryStreamName, DeliveryStreamType, Destinations, Status
   - **Relationships**: Map to S3, Redshift, Elasticsearch destinations

**Success Criteria**: Batch processing and real-time analytics monitoring

### Milestone 9.2: Business Intelligence (Week 26)
**Objective**: BI dashboard and reporting monitoring

#### Tasks:
1. **Add Amazon QuickSight**
   - **Dependencies**: Add `aws-sdk-quicksight = "1.67"` to Cargo.toml
   - **Service**: Create `aws_services/quicksight.rs`
   - **Resource Type**: `AWS::QuickSight::DataSource`
   - **SDK Calls**: `ListDataSources()`, `DescribeDataSource()`
   - **Implementation**: Follow NEWSERVICES_TODO.md service creation patterns
   - **Key Fields**: DataSourceId, Name, Type, Status, DataSourceParameters
   - **Relationships**: Map to dashboards, datasets, analyses

2. **Add QuickSight Dashboards and Datasets**
   - **Service**: Extend `aws_services/quicksight.rs`
   - **Resource Types**: `AWS::QuickSight::Dashboard`, `AWS::QuickSight::DataSet`
   - **SDK Calls**: `ListDashboards()`, `DescribeDashboard()`, `ListDataSets()`
   - **Implementation**: Add to QuickSightService
   - **Key Fields**: DashboardId, DashboardName, Version; DataSetId, ImportMode, PhysicalTableMap
   - **Relationships**: Dashboard usage and data lineage tracking

**Success Criteria**: Complete business intelligence and reporting monitoring

---

## 📋 Implementation Guidelines

### **CRITICAL: Follow NEWSERVICES_TODO.md**
All implementations MUST follow the comprehensive patterns documented in `NEWSERVICES_TODO.md`, including:

1. **AWS SDK Field Access Patterns** - Proper Option<T> handling, boolean fields, enum conversion
2. **Pagination Patterns** - Standard paginator vs manual token pagination  
3. **Describe API Patterns** - Internal helper method patterns
4. **JSON Conversion** - Manual conversion, never serde_json::to_value() on AWS types
5. **Error Handling** - Proper fallback patterns and error logging
6. **Testing Patterns** - Edge case validation and compilation verification
7. **Enhanced Resource Integration** - CRITICAL: Ensure describe_resource routing integration

### **CRITICAL: Enhanced Resource Integration Pattern**
⚠️ **MANDATORY INTEGRATION STEP** - This step is essential for UI data flow and was discovered to be missing across multiple services:

**For ALL services with enhanced describe methods, you MUST integrate them into the describe_resource routing method in `aws_client.rs`:**

```rust
// In src/app/resource_explorer/aws_client.rs, method describe_resource()
match resource.resource_type.as_str() {
    "AWS::CertificateManager::Certificate" => {
        self.acm_service.describe_certificate(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::WAFv2::WebACL" => {
        self.wafv2_service.get_web_acl(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    "AWS::GuardDuty::Detector" => {
        self.guardduty_service.get_detector(&resource.account_id, &resource.region, &resource.resource_id).await
    }
    // Add ALL new services with describe methods here
    _ => {
        Err(anyhow::anyhow!("Describe not implemented for resource type: {}", resource.resource_type))
    }
}
```

**Why This Is Critical:**
- Enhanced describe methods provide detailed configuration data (encryption, policies, lifecycle rules, etc.)
- Without this integration, enhanced data never reaches the UI through the detailed_properties field
- The describe_resource method is the central routing point for detailed resource inspection

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

### **Testing Strategy**
- Use chunked testing approach: `./scripts/test-chunks.sh fast`
- Verify compilation before implementation: `cargo check`
- Test resource discovery in UI after implementation
- Validate describe functionality through UI detailed views

---

## 🎯 Success Metrics

**Phase 6 Completion Criteria:**
- SSL/TLS certificate lifecycle monitoring
- Web application firewall visibility 
- Threat detection and security posture monitoring

**Phase 7 Completion Criteria:**
- CDN performance and edge location monitoring
- Cache cluster performance optimization
- Content delivery optimization visibility

**Phase 8 Completion Criteria:**
- Graph database relationship monitoring
- Search and analytics database performance
- User identity and authentication tracking

**Phase 9 Completion Criteria:**
- Batch processing workload monitoring
- Real-time data pipeline visibility
- Business intelligence dashboard tracking

**Overall Goal**: Achieve comprehensive AWS infrastructure monitoring with 60+ services covering security, performance, analytics, and specialized workloads.