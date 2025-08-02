# AWS Explorer - Implemented Services

This document lists all AWS services currently implemented in the AWS Explorer feature of aws-dash.

## Summary

Total implemented services: **90 resource types** across **43 AWS services**

## Services by Category

### Compute Services

#### EC2 (Elastic Compute Cloud)
- EC2 Instance
- Security Group  
- VPC (Virtual Private Cloud)
- EBS Volume
- EBS Snapshot
- AMI (Amazon Machine Image)
- Subnet
- Route Table
- NAT Gateway
- Network Interface
- VPC Endpoint
- Network ACL
- Key Pair
- Internet Gateway

#### Lambda
- Lambda Function
- Lambda Layer
- Lambda Event Source Mapping

#### ECS (Elastic Container Service)
- ECS Cluster
- ECS Service
- ECS Task
- ECS Task Definition

#### EKS (Elastic Kubernetes Service)
- EKS Cluster

#### Batch
- Batch Job Queue
- Batch Compute Environment

### Storage Services

#### S3 (Simple Storage Service)
- S3 Bucket

#### EFS (Elastic File System)
- EFS File System

### Database Services

#### RDS (Relational Database Service)
- RDS DB Instance
- RDS DB Cluster
- RDS DB Snapshot
- RDS DB Parameter Group
- RDS DB Subnet Group

#### DynamoDB
- DynamoDB Table

#### ElastiCache
- Cache Cluster
- Redis Replication Group
- Cache Parameter Group

#### Neptune
- Graph Database Cluster
- Graph Database Instance

#### OpenSearch
- Search and Analytics Engine

#### Redshift
- Redshift Cluster

### Networking & Content Delivery

#### CloudFront
- Content Delivery Network

#### Route53
- Route53 Hosted Zone

#### API Gateway
- API Gateway REST API
- API Gateway v2 HTTP API

#### ELB (Elastic Load Balancing)
- Classic Load Balancer
- Application/Network Load Balancer (ELBv2)
- Target Group (ELBv2)

### Security, Identity & Compliance

#### IAM (Identity and Access Management)
- IAM Role
- IAM User
- IAM Policy

#### Certificate Manager
- SSL/TLS Certificate
- Private Certificate Authority

#### WAF & Shield
- Web Application Firewall (WAFv2)

#### GuardDuty
- Threat Detection Service

#### Cognito
- User Pool
- Identity Pool
- User Pool Client

### Management & Governance

#### CloudFormation
- CloudFormation Stack

#### CloudWatch
- CloudWatch Alarm
- CloudWatch Dashboard
- CloudWatch Log Group

#### CloudTrail
- CloudTrail Trail

#### Config
- Config Configuration Recorder

#### Systems Manager (SSM)
- Systems Manager Parameter
- Systems Manager Document

#### Organizations
- Organizational Unit
- Service Control Policy

#### Backup
- Backup Plan
- Backup Vault

### Application Integration

#### SNS (Simple Notification Service)
- SNS Topic

#### SQS (Simple Queue Service)
- SQS Queue

#### EventBridge
- EventBridge Event Bus
- EventBridge Rule

#### Kinesis
- Kinesis Data Stream

#### Kinesis Data Firehose
- Kinesis Data Firehose Delivery Stream

#### AppSync
- AppSync GraphQL API

#### Amazon MQ
- Amazon MQ Broker

### Analytics

#### Athena
- Athena Workgroup

#### Glue
- Glue ETL Job

#### QuickSight
- QuickSight Data Source
- QuickSight Dashboard
- QuickSight Data Set

### Machine Learning

#### SageMaker
- SageMaker Endpoint
- SageMaker Training Job
- SageMaker Model

#### Bedrock
- Bedrock Model

### Developer Tools

#### CodePipeline
- CodePipeline Pipeline

#### CodeBuild
- CodeBuild Project

#### CodeCommit
- CodeCommit Repository

### IoT Services

#### IoT Core
- IoT Thing

#### Greengrass
- Greengrass Component Version

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