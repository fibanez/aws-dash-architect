//! Resource type mapping for CloudTrail Events
//!
//! Maps AWS CloudFormation resource types to CloudTrail lookup attributes.

#![warn(clippy::all, rust_2018_idioms)]

/// Check if a resource type is supported by CloudTrail
///
/// CloudTrail records API calls for ALL AWS services, so all resource types are supported.
/// This function always returns true.
pub fn has_cloudtrail_support(_resource_type: &str) -> bool {
    // CloudTrail logs ALL AWS API calls, so all resources are supported
    true
}

/// Get CloudTrail lookup attribute value for a resource
///
/// Returns the resource type as the lookup attribute value.
/// CloudTrail filters events by resource type.
///
/// # Arguments
/// * `resource_type` - CloudFormation resource type
/// * `_resource_name` - Resource name/physical ID (not used for lookup)
/// * `_resource_arn` - Resource ARN (not used for lookup)
///
/// # Returns
/// Resource type for CloudTrail filtering
///
/// # Example
/// ```
/// let lookup_value = get_cloudtrail_lookup_value(
///     "AWS::EC2::Instance",
///     "my-instance",
///     Some("arn:aws:ec2:us-east-1:123456789012:instance/i-1234567890abcdef0")
/// );
/// assert_eq!(lookup_value, Some("AWS::EC2::Instance".to_string()));
/// ```
pub fn get_cloudtrail_lookup_value(
    resource_type: &str,
    _resource_name: &str,
    _resource_arn: Option<&str>,
) -> Option<String> {
    // CloudTrail uses resource type for filtering
    Some(resource_type.to_string())
}

/// Get resource name for CloudTrail filtering (if applicable)
///
/// Some resources benefit from filtering by resource name in addition to type.
///
/// # Arguments
/// * `resource_type` - CloudFormation resource type
/// * `resource_name` - Resource name/physical ID
///
/// # Returns
/// Resource name if it's useful for filtering, None otherwise
pub fn get_resource_name_for_filtering(resource_type: &str, resource_name: &str) -> Option<String> {
    // For most resources, the resource name is useful for filtering
    // Only skip it for very generic types
    match resource_type {
        // These types benefit from name filtering
        "AWS::EC2::Instance"
        | "AWS::S3::Bucket"
        | "AWS::Lambda::Function"
        | "AWS::RDS::DBInstance"
        | "AWS::DynamoDB::Table"
        | "AWS::ECS::Service"
        | "AWS::ECS::Cluster"
        | "AWS::EKS::Cluster"
        | "AWS::ElasticLoadBalancingV2::LoadBalancer"
        | "AWS::ElasticLoadBalancingV2::TargetGroup"
        | "AWS::ApiGateway::RestApi"
        | "AWS::SNS::Topic"
        | "AWS::SQS::Queue"
        | "AWS::KMS::Key" => Some(resource_name.to_string()),

        // For other types, name filtering might not be as useful
        _ => None,
    }
}

/// Get suggested event names for a resource type
///
/// Returns common CloudTrail event names associated with a resource type
/// to help users understand what events they might see.
///
/// # Arguments
/// * `resource_type` - CloudFormation resource type
///
/// # Returns
/// List of common event names for this resource type
pub fn get_common_event_names(resource_type: &str) -> Vec<&'static str> {
    match resource_type {
        "AWS::EC2::Instance" => vec![
            "RunInstances",
            "TerminateInstances",
            "StartInstances",
            "StopInstances",
            "RebootInstances",
            "ModifyInstanceAttribute",
        ],
        "AWS::S3::Bucket" => vec![
            "CreateBucket",
            "DeleteBucket",
            "PutBucketPolicy",
            "PutBucketEncryption",
            "PutBucketVersioning",
        ],
        "AWS::Lambda::Function" => vec![
            "CreateFunction",
            "UpdateFunctionCode",
            "UpdateFunctionConfiguration",
            "DeleteFunction",
            "Invoke",
        ],
        "AWS::RDS::DBInstance" => vec![
            "CreateDBInstance",
            "ModifyDBInstance",
            "DeleteDBInstance",
            "RebootDBInstance",
            "StartDBInstance",
            "StopDBInstance",
        ],
        "AWS::DynamoDB::Table" => vec![
            "CreateTable",
            "UpdateTable",
            "DeleteTable",
            "PutItem",
            "UpdateItem",
            "DeleteItem",
        ],
        "AWS::IAM::Role" => vec![
            "CreateRole",
            "DeleteRole",
            "PutRolePolicy",
            "AttachRolePolicy",
            "DetachRolePolicy",
        ],
        "AWS::IAM::User" => vec![
            "CreateUser",
            "DeleteUser",
            "PutUserPolicy",
            "AttachUserPolicy",
            "CreateAccessKey",
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_resources_supported() {
        // CloudTrail supports all AWS resource types
        assert!(has_cloudtrail_support("AWS::EC2::Instance"));
        assert!(has_cloudtrail_support("AWS::S3::Bucket"));
        assert!(has_cloudtrail_support("AWS::Lambda::Function"));
        assert!(has_cloudtrail_support("AWS::SomeService::SomeResource"));
        assert!(has_cloudtrail_support("anything"));
    }

    #[test]
    fn test_lookup_value() {
        let value = get_cloudtrail_lookup_value(
            "AWS::EC2::Instance",
            "my-instance",
            Some("arn:aws:ec2:us-east-1:123456789012:instance/i-1234567890abcdef0"),
        );
        assert_eq!(value, Some("AWS::EC2::Instance".to_string()));
    }

    #[test]
    fn test_resource_name_filtering() {
        // EC2 instances benefit from name filtering
        assert!(get_resource_name_for_filtering("AWS::EC2::Instance", "my-instance").is_some());

        // S3 buckets benefit from name filtering
        assert!(get_resource_name_for_filtering("AWS::S3::Bucket", "my-bucket").is_some());

        // Generic resources might not
        assert!(
            get_resource_name_for_filtering("AWS::CloudFormation::Stack", "my-stack").is_none()
        );
    }

    #[test]
    fn test_common_event_names() {
        let events = get_common_event_names("AWS::EC2::Instance");
        assert!(events.contains(&"RunInstances"));
        assert!(events.contains(&"TerminateInstances"));

        let lambda_events = get_common_event_names("AWS::Lambda::Function");
        assert!(lambda_events.contains(&"CreateFunction"));
        assert!(lambda_events.contains(&"Invoke"));

        // Unknown resource type returns empty list
        let unknown_events = get_common_event_names("AWS::Unknown::Resource");
        assert!(unknown_events.is_empty());
    }
}
