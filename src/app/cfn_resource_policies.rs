use std::collections::HashSet;

/// Represents the types of policies that can be applied to CloudFormation resources
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyType {
    Creation,
    Update,
}

/// Represents specific Creation Policy configurations available for different resource types
#[derive(Debug, Clone, PartialEq)]
pub enum CreationPolicyType {
    ResourceSignal,
    AutoScalingCreationPolicy,
    AppStreamStartFleet,
}

/// Represents specific Update Policy configurations available for different resource types
#[derive(Debug, Clone, PartialEq)]
pub enum UpdatePolicyType {
    AutoScalingRollingUpdate,
    AutoScalingReplacingUpdate,
    AutoScalingScheduledAction,
    CodeDeployLambdaAliasUpdate,
    UseOnlineResharding,
    EnableVersionUpgrade,
    AppStreamStopStart,
}

/// Resource type awareness utility for CloudFormation policies
pub struct ResourcePolicyManager;

impl ResourcePolicyManager {
    /// Returns true if the given resource type supports CreationPolicy
    pub fn supports_creation_policy(resource_type: &str) -> bool {
        matches!(
            resource_type,
            "AWS::AutoScaling::AutoScalingGroup"
                | "AWS::EC2::Instance"
                | "AWS::CloudFormation::WaitCondition"
                | "AWS::AppStream::Fleet"
        )
    }

    /// Returns true if the given resource type supports UpdatePolicy
    pub fn supports_update_policy(resource_type: &str) -> bool {
        matches!(
            resource_type,
            "AWS::AutoScaling::AutoScalingGroup"
                | "AWS::Lambda::Alias"
                | "AWS::ElastiCache::ReplicationGroup"
                | "AWS::OpenSearchService::Domain"
                | "AWS::Elasticsearch::Domain"
                | "AWS::AppStream::Fleet"
        )
    }

    /// Returns the available CreationPolicy configurations for a resource type
    pub fn get_creation_policy_types(resource_type: &str) -> Vec<CreationPolicyType> {
        match resource_type {
            "AWS::AutoScaling::AutoScalingGroup" => vec![
                CreationPolicyType::ResourceSignal,
                CreationPolicyType::AutoScalingCreationPolicy,
            ],
            "AWS::EC2::Instance" | "AWS::CloudFormation::WaitCondition" => {
                vec![CreationPolicyType::ResourceSignal]
            }
            "AWS::AppStream::Fleet" => vec![CreationPolicyType::AppStreamStartFleet],
            _ => vec![],
        }
    }

    /// Returns the available UpdatePolicy configurations for a resource type
    pub fn get_update_policy_types(resource_type: &str) -> Vec<UpdatePolicyType> {
        match resource_type {
            "AWS::AutoScaling::AutoScalingGroup" => vec![
                UpdatePolicyType::AutoScalingRollingUpdate,
                UpdatePolicyType::AutoScalingReplacingUpdate,
                UpdatePolicyType::AutoScalingScheduledAction,
            ],
            "AWS::Lambda::Alias" => vec![UpdatePolicyType::CodeDeployLambdaAliasUpdate],
            "AWS::ElastiCache::ReplicationGroup" => vec![UpdatePolicyType::UseOnlineResharding],
            "AWS::OpenSearchService::Domain" | "AWS::Elasticsearch::Domain" => {
                vec![UpdatePolicyType::EnableVersionUpgrade]
            }
            "AWS::AppStream::Fleet" => vec![UpdatePolicyType::AppStreamStopStart],
            _ => vec![],
        }
    }

    /// Returns a user-friendly description of a CreationPolicy type
    pub fn get_creation_policy_description(policy_type: &CreationPolicyType) -> &'static str {
        match policy_type {
            CreationPolicyType::ResourceSignal => {
                "Wait for signals from the resource indicating successful creation"
            }
            CreationPolicyType::AutoScalingCreationPolicy => {
                "Configure minimum successful instance percentage for Auto Scaling Group creation"
            }
            CreationPolicyType::AppStreamStartFleet => {
                "Automatically start the AppStream fleet after creation"
            }
        }
    }

    /// Returns a user-friendly description of an UpdatePolicy type
    pub fn get_update_policy_description(policy_type: &UpdatePolicyType) -> &'static str {
        match policy_type {
            UpdatePolicyType::AutoScalingRollingUpdate => {
                "Perform rolling updates on Auto Scaling Group instances"
            }
            UpdatePolicyType::AutoScalingReplacingUpdate => {
                "Replace the entire Auto Scaling Group during updates"
            }
            UpdatePolicyType::AutoScalingScheduledAction => {
                "Configure how scheduled actions are handled during updates"
            }
            UpdatePolicyType::CodeDeployLambdaAliasUpdate => {
                "Use AWS CodeDeploy for Lambda alias traffic shifting"
            }
            UpdatePolicyType::UseOnlineResharding => {
                "Use online resharding for ElastiCache cluster updates"
            }
            UpdatePolicyType::EnableVersionUpgrade => {
                "Enable in-place version upgrades without replacement"
            }
            UpdatePolicyType::AppStreamStopStart => {
                "Control AppStream fleet stop/start behavior during updates"
            }
        }
    }

    /// Returns all resource types that support any policies (for UI filtering)
    pub fn get_all_policy_supporting_resources() -> HashSet<&'static str> {
        let mut resources = HashSet::new();
        resources.insert("AWS::AutoScaling::AutoScalingGroup");
        resources.insert("AWS::EC2::Instance");
        resources.insert("AWS::CloudFormation::WaitCondition");
        resources.insert("AWS::AppStream::Fleet");
        resources.insert("AWS::Lambda::Alias");
        resources.insert("AWS::ElastiCache::ReplicationGroup");
        resources.insert("AWS::OpenSearchService::Domain");
        resources.insert("AWS::Elasticsearch::Domain");
        resources
    }

    /// Returns validation rules for CreationPolicy configurations
    pub fn validate_creation_policy_config(
        resource_type: &str,
        policy_type: &CreationPolicyType,
        config: &serde_json::Value,
    ) -> Result<(), String> {
        match (resource_type, policy_type) {
            (
                "AWS::AutoScaling::AutoScalingGroup",
                CreationPolicyType::AutoScalingCreationPolicy,
            ) => {
                if let Some(percent) = config.get("MinSuccessfulInstancesPercent") {
                    if let Some(val) = percent.as_u64() {
                        if val > 100 {
                            return Err("MinSuccessfulInstancesPercent must be between 0 and 100"
                                .to_string());
                        }
                    }
                }
                Ok(())
            }
            (_, CreationPolicyType::ResourceSignal) => {
                if let Some(count) = config.get("Count") {
                    if let Some(val) = count.as_u64() {
                        if val == 0 {
                            return Err("ResourceSignal Count must be greater than 0".to_string());
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Returns default configuration templates for CreationPolicy types
    pub fn get_creation_policy_template(policy_type: &CreationPolicyType) -> serde_json::Value {
        match policy_type {
            CreationPolicyType::ResourceSignal => serde_json::json!({
                "Count": 1,
                "Timeout": "PT5M"
            }),
            CreationPolicyType::AutoScalingCreationPolicy => serde_json::json!({
                "MinSuccessfulInstancesPercent": 100
            }),
            CreationPolicyType::AppStreamStartFleet => serde_json::json!(true),
        }
    }

    /// Returns default configuration templates for UpdatePolicy types
    pub fn get_update_policy_template(policy_type: &UpdatePolicyType) -> serde_json::Value {
        match policy_type {
            UpdatePolicyType::AutoScalingRollingUpdate => serde_json::json!({
                "MinInstancesInService": 1,
                "MaxBatchSize": 1,
                "PauseTime": "PT0S",
                "WaitOnResourceSignals": false
            }),
            UpdatePolicyType::AutoScalingReplacingUpdate => serde_json::json!({
                "WillReplace": true
            }),
            UpdatePolicyType::AutoScalingScheduledAction => serde_json::json!({
                "IgnoreUnmodifiedGroupSizeProperties": true
            }),
            UpdatePolicyType::CodeDeployLambdaAliasUpdate => serde_json::json!({
                "ApplicationName": "",
                "DeploymentGroupName": ""
            }),
            UpdatePolicyType::UseOnlineResharding => serde_json::json!(true),
            UpdatePolicyType::EnableVersionUpgrade => serde_json::json!(true),
            UpdatePolicyType::AppStreamStopStart => serde_json::json!({
                "StopBeforeUpdate": true,
                "StartAfterUpdate": true
            }),
        }
    }

    /// Helper function to get all deletion policy options
    pub fn get_deletion_policy_options() -> Vec<&'static str> {
        vec!["Delete", "Retain", "Snapshot"]
    }

    /// Helper function to get all update replace policy options
    pub fn get_update_replace_policy_options() -> Vec<&'static str> {
        vec!["Delete", "Retain", "Snapshot"]
    }

    /// Returns resources that support the Snapshot deletion policy
    pub fn supports_snapshot_policy(resource_type: &str) -> bool {
        matches!(
            resource_type,
            "AWS::EC2::Volume"
                | "AWS::ElastiCache::CacheCluster"
                | "AWS::ElastiCache::ReplicationGroup"
                | "AWS::Neptune::DBCluster"
                | "AWS::RDS::DBCluster"
                | "AWS::RDS::DBInstance"
                | "AWS::Redshift::Cluster"
        )
    }

    /// Returns the available deletion policy options for a specific resource type
    pub fn get_available_deletion_policies(resource_type: &str) -> Vec<&'static str> {
        let mut policies = vec!["Delete", "Retain"];
        if Self::supports_snapshot_policy(resource_type) {
            policies.push("Snapshot");
        }
        policies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_creation_policy() {
        assert!(ResourcePolicyManager::supports_creation_policy(
            "AWS::AutoScaling::AutoScalingGroup"
        ));
        assert!(ResourcePolicyManager::supports_creation_policy(
            "AWS::EC2::Instance"
        ));
        assert!(!ResourcePolicyManager::supports_creation_policy(
            "AWS::S3::Bucket"
        ));
    }

    #[test]
    fn test_supports_update_policy() {
        assert!(ResourcePolicyManager::supports_update_policy(
            "AWS::AutoScaling::AutoScalingGroup"
        ));
        assert!(ResourcePolicyManager::supports_update_policy(
            "AWS::Lambda::Alias"
        ));
        assert!(!ResourcePolicyManager::supports_update_policy(
            "AWS::S3::Bucket"
        ));
    }

    #[test]
    fn test_get_creation_policy_types() {
        let asg_policies =
            ResourcePolicyManager::get_creation_policy_types("AWS::AutoScaling::AutoScalingGroup");
        assert_eq!(asg_policies.len(), 2);
        assert!(asg_policies.contains(&CreationPolicyType::ResourceSignal));
        assert!(asg_policies.contains(&CreationPolicyType::AutoScalingCreationPolicy));

        let ec2_policies = ResourcePolicyManager::get_creation_policy_types("AWS::EC2::Instance");
        assert_eq!(ec2_policies.len(), 1);
        assert!(ec2_policies.contains(&CreationPolicyType::ResourceSignal));
    }

    #[test]
    fn test_deletion_policy_options() {
        let policies =
            ResourcePolicyManager::get_available_deletion_policies("AWS::RDS::DBInstance");
        assert_eq!(policies.len(), 3);
        assert!(policies.contains(&"Snapshot"));

        let policies = ResourcePolicyManager::get_available_deletion_policies("AWS::S3::Bucket");
        assert_eq!(policies.len(), 2);
        assert!(!policies.contains(&"Snapshot"));
    }

    #[test]
    fn test_policy_templates() {
        let template = ResourcePolicyManager::get_creation_policy_template(
            &CreationPolicyType::ResourceSignal,
        );
        assert_eq!(template["Count"], 1);
        assert_eq!(template["Timeout"], "PT5M");

        let template = ResourcePolicyManager::get_update_policy_template(
            &UpdatePolicyType::AutoScalingRollingUpdate,
        );
        assert_eq!(template["MinInstancesInService"], 1);
        assert_eq!(template["MaxBatchSize"], 1);
    }
}
