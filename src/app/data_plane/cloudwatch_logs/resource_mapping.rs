//! Resource to CloudWatch Log Group Mapping
//!
//! Maps AWS resource types to their associated CloudWatch Log Group naming patterns.

#![warn(clippy::all, rust_2018_idioms)]

/// Check if a resource type has associated CloudWatch Logs
pub fn has_cloudwatch_logs(resource_type: &str) -> bool {
    matches!(
        resource_type,
        // CloudWatch Logs
        "AWS::Logs::LogGroup"
            // Compute
            | "AWS::Lambda::Function"
            | "AWS::ECS::Task"
            | "AWS::ECS::Service"
            | "AWS::EC2::Instance"
            | "AWS::EKS::Cluster"
            | "AWS::ElasticBeanstalk::Environment"
            // API & Application Integration
            | "AWS::ApiGateway::RestApi"
            | "AWS::ApiGatewayV2::Api"
            | "AWS::AppSync::GraphQLApi"
            | "AWS::StepFunctions::StateMachine"
            // Database
            | "AWS::RDS::DBInstance"
            | "AWS::RDS::DBCluster"
            | "AWS::DynamoDB::Table"
            // Analytics & Data Processing
            | "AWS::Glue::Job"
            | "AWS::MSK::Cluster"
            | "AWS::CodeBuild::Project"
            | "AWS::DataSync::Task"
            // Machine Learning
            | "AWS::SageMaker::Endpoint"
            | "AWS::SageMaker::EndpointConfig"
            | "AWS::SageMaker::Model"
            // Security & Identity
            | "AWS::Cognito::UserPool"
            | "AWS::Transfer::Server"
            | "AWS::WAFv2::WebACL"
            // Networking
            | "AWS::VPC::FlowLog"
            | "AWS::Route53::HostedZone"
            | "AWS::ElasticLoadBalancingV2::LoadBalancer"
            // Storage & Migration
            | "AWS::StorageGateway::Gateway"
    )
}

/// Get the CloudWatch Log Group name pattern for a resource
///
/// Returns a log group name if the resource type has a standard pattern.
/// For some resource types, the log group name can be derived from the resource ARN or name.
pub fn get_log_group_name(
    resource_type: &str,
    resource_name: &str,
    _resource_arn: Option<&str>,
) -> Option<String> {
    match resource_type {
        "AWS::Logs::LogGroup" => {
            // CloudWatch Log Group: the resource name IS the log group name
            Some(resource_name.to_string())
        }
        "AWS::Lambda::Function" => {
            // Lambda: /aws/lambda/{function-name}
            Some(format!("/aws/lambda/{}", resource_name))
        }
        "AWS::ApiGateway::RestApi" => {
            // API Gateway REST API: /aws/apigateway/{api-name}
            // Note: May need to extract API ID from ARN for accurate log group
            Some(format!("/aws/apigateway/{}", resource_name))
        }
        "AWS::ApiGatewayV2::Api" => {
            // API Gateway V2 (HTTP/WebSocket): /aws/apigateway/{api-id}
            Some(format!("/aws/apigateway/{}", resource_name))
        }
        "AWS::ECS::Task" => {
            // ECS: /ecs/{cluster-name}/{task-definition-family}
            // This is a common pattern but may vary - user might need to configure
            Some(format!("/ecs/{}", resource_name))
        }
        "AWS::RDS::DBInstance" => {
            // RDS Instance: /aws/rds/instance/{instance-id}/error
            // Also: /aws/rds/instance/{instance-id}/slowquery, /aws/rds/instance/{instance-id}/general
            Some(format!("/aws/rds/instance/{}/error", resource_name))
        }
        "AWS::RDS::DBCluster" => {
            // RDS Cluster: /aws/rds/cluster/{cluster-id}/error
            Some(format!("/aws/rds/cluster/{}/error", resource_name))
        }
        "AWS::CodeBuild::Project" => {
            // CodeBuild: /aws/codebuild/{project-name}
            Some(format!("/aws/codebuild/{}", resource_name))
        }
        "AWS::StepFunctions::StateMachine" => {
            // Step Functions: /aws/states/{state-machine-name}
            // Note: May need CloudWatch Logs enabled on the state machine
            Some(format!("/aws/states/{}", resource_name))
        }
        "AWS::EKS::Cluster" => {
            // EKS: /aws/eks/{cluster-name}/cluster
            Some(format!("/aws/eks/{}/cluster", resource_name))
        }
        "AWS::ElasticBeanstalk::Environment" => {
            // Elastic Beanstalk: /aws/elasticbeanstalk/{environment-name}
            Some(format!("/aws/elasticbeanstalk/{}", resource_name))
        }
        "AWS::EC2::Instance" => {
            // EC2: Custom log groups via CloudWatch Agent
            // Pattern varies, but often: /aws/ec2/{instance-id} or custom
            // Return None as it's too variable
            None
        }
        "AWS::ECS::Service" => {
            // ECS Service: Similar to Task, often /ecs/{cluster-name} or /aws/ecs/{service-name}
            Some(format!(
                "/aws/ecs/containerinsights/{}/performance",
                resource_name
            ))
        }
        "AWS::AppSync::GraphQLApi" => {
            // AppSync: /aws/appsync/apis/{api-id}
            Some(format!("/aws/appsync/apis/{}", resource_name))
        }
        "AWS::DynamoDB::Table" => {
            // DynamoDB Streams logs (if enabled)
            // Note: Not automatic, requires configuration
            None
        }
        "AWS::Glue::Job" => {
            // Glue: /aws-glue/jobs/output or /aws-glue/jobs/error
            Some(format!("/aws-glue/jobs/{}", resource_name))
        }
        "AWS::MSK::Cluster" => {
            // MSK: /aws/msk/cluster/{cluster-name}/broker
            Some(format!("/aws/msk/cluster/{}/broker", resource_name))
        }
        "AWS::DataSync::Task" => {
            // DataSync: /aws/datasync
            Some("/aws/datasync".to_string())
        }
        "AWS::SageMaker::Endpoint" => {
            // SageMaker Endpoint: /aws/sagemaker/Endpoints/{endpoint-name}
            Some(format!("/aws/sagemaker/Endpoints/{}", resource_name))
        }
        "AWS::SageMaker::EndpointConfig" => {
            // SageMaker Endpoint Config: logs via endpoint
            None
        }
        "AWS::SageMaker::Model" => {
            // SageMaker Model: logs via training jobs or endpoints
            None
        }
        "AWS::Cognito::UserPool" => {
            // Cognito: /aws/cognito/userpools/{user-pool-id}
            Some(format!("/aws/cognito/userpools/{}", resource_name))
        }
        "AWS::Transfer::Server" => {
            // Transfer Family: /aws/transfer/{server-id}
            Some(format!("/aws/transfer/{}", resource_name))
        }
        "AWS::WAFv2::WebACL" => {
            // WAF: aws-waf-logs-{name}
            Some(format!("aws-waf-logs-{}", resource_name))
        }
        "AWS::VPC::FlowLog" => {
            // VPC Flow Logs: Custom, user-defined log group
            // Pattern varies widely
            None
        }
        "AWS::Route53::HostedZone" => {
            // Route 53 Query Logs: /aws/route53/{hosted-zone-id}
            Some(format!("/aws/route53/{}", resource_name))
        }
        "AWS::ElasticLoadBalancingV2::LoadBalancer" => {
            // ALB/NLB Access Logs: typically sent to S3, not CloudWatch Logs by default
            // But can be configured for CloudWatch
            None
        }
        "AWS::StorageGateway::Gateway" => {
            // Storage Gateway: /aws/storagegateway/{gateway-id}
            Some(format!("/aws/storagegateway/{}", resource_name))
        }
        _ => None,
    }
}

/// Get all possible log group patterns for a resource
///
/// Some resources may have multiple log groups (e.g., RDS has error, slowquery, general)
pub fn get_all_log_group_patterns(resource_type: &str, resource_name: &str) -> Vec<String> {
    match resource_type {
        "AWS::RDS::DBInstance" => {
            vec![
                format!("/aws/rds/instance/{}/error", resource_name),
                format!("/aws/rds/instance/{}/slowquery", resource_name),
                format!("/aws/rds/instance/{}/general", resource_name),
            ]
        }
        "AWS::RDS::DBCluster" => {
            vec![
                format!("/aws/rds/cluster/{}/error", resource_name),
                format!("/aws/rds/cluster/{}/audit", resource_name),
            ]
        }
        _ => {
            // For most resources, just return the single pattern
            if let Some(pattern) = get_log_group_name(resource_type, resource_name, None) {
                vec![pattern]
            } else {
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_cloudwatch_logs() {
        assert!(has_cloudwatch_logs("AWS::Logs::LogGroup"));
        assert!(has_cloudwatch_logs("AWS::Lambda::Function"));
        assert!(has_cloudwatch_logs("AWS::ApiGateway::RestApi"));
        assert!(has_cloudwatch_logs("AWS::ECS::Task"));
        assert!(has_cloudwatch_logs("AWS::DynamoDB::Table")); // DynamoDB has CloudWatch Logs via Contributor Insights
        assert!(!has_cloudwatch_logs("AWS::S3::Bucket"));
        assert!(!has_cloudwatch_logs("AWS::IAM::Role"));
    }

    #[test]
    fn test_get_log_group_name_log_group() {
        let log_group = get_log_group_name("AWS::Logs::LogGroup", "/aws/lambda/my-function", None);
        assert_eq!(log_group, Some("/aws/lambda/my-function".to_string()));

        // Test with another pattern
        let log_group2 = get_log_group_name("AWS::Logs::LogGroup", "/custom/app/logs", None);
        assert_eq!(log_group2, Some("/custom/app/logs".to_string()));
    }

    #[test]
    fn test_get_log_group_name_lambda() {
        let log_group = get_log_group_name("AWS::Lambda::Function", "my-function", None);
        assert_eq!(log_group, Some("/aws/lambda/my-function".to_string()));
    }

    #[test]
    fn test_get_log_group_name_rds() {
        let log_group = get_log_group_name("AWS::RDS::DBInstance", "my-db-instance", None);
        assert_eq!(
            log_group,
            Some("/aws/rds/instance/my-db-instance/error".to_string())
        );
    }

    #[test]
    fn test_get_all_log_group_patterns_rds() {
        let patterns = get_all_log_group_patterns("AWS::RDS::DBInstance", "my-db");
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"/aws/rds/instance/my-db/error".to_string()));
        assert!(patterns.contains(&"/aws/rds/instance/my-db/slowquery".to_string()));
        assert!(patterns.contains(&"/aws/rds/instance/my-db/general".to_string()));
    }

    #[test]
    fn test_get_all_log_group_patterns_lambda() {
        let patterns = get_all_log_group_patterns("AWS::Lambda::Function", "my-function");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0], "/aws/lambda/my-function");
    }

    #[test]
    fn test_unsupported_resource_type() {
        assert!(!has_cloudwatch_logs("AWS::S3::Bucket"));
        assert_eq!(
            get_log_group_name("AWS::S3::Bucket", "my-bucket", None),
            None
        );
        assert_eq!(
            get_all_log_group_patterns("AWS::S3::Bucket", "my-bucket").len(),
            0
        );
    }
}
