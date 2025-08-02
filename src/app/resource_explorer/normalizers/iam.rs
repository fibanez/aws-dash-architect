use super::{utils::*, ResourceNormalizer};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct IAMRoleNormalizer;

impl ResourceNormalizer for IAMRoleNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let role_name = raw_response
            .get("RoleName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-role")
            .to_string();

        let role_id = raw_response
            .get("RoleId")
            .and_then(|v| v.as_str())
            .unwrap_or(&role_name)
            .to_string();

        let display_name = role_name.clone();
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::IAM::Role".to_string(),
            account_id: account.to_string(),
            region: region.to_string(), // IAM is global but we track by region for consistency
            resource_id: role_id,
            display_name,
            status: None, // IAM roles don't have a state
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Find attached policies (if we have them in the data)
        if let Some(attached_policies) = entry
            .raw_properties
            .get("AttachedManagedPolicies")
            .and_then(|policies| policies.as_array())
        {
            for policy in attached_policies {
                if let Some(policy_arn) = policy.get("PolicyArn").and_then(|arn| arn.as_str()) {
                    // Extract policy name from ARN for matching
                    if let Some(policy_name) = policy_arn.split('/').next_back() {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: policy_name.to_string(),
                            target_resource_type: "AWS::IAM::Policy".to_string(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::IAM::Role"
    }
}

pub struct IAMUserNormalizer;

impl ResourceNormalizer for IAMUserNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let user_name = raw_response
            .get("UserName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-user")
            .to_string();

        let user_id = raw_response
            .get("UserId")
            .and_then(|v| v.as_str())
            .unwrap_or(&user_name)
            .to_string();

        let display_name = user_name.clone();
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::IAM::User".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: user_id,
            display_name,
            status: None,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Find attached policies
        if let Some(attached_policies) = entry
            .raw_properties
            .get("AttachedManagedPolicies")
            .and_then(|policies| policies.as_array())
        {
            for policy in attached_policies {
                if let Some(policy_arn) = policy.get("PolicyArn").and_then(|arn| arn.as_str()) {
                    if let Some(policy_name) = policy_arn.split('/').next_back() {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: policy_name.to_string(),
                            target_resource_type: "AWS::IAM::Policy".to_string(),
                        });
                    }
                }
            }
        }

        // Find groups this user belongs to
        if let Some(groups) = entry
            .raw_properties
            .get("Groups")
            .and_then(|groups| groups.as_array())
        {
            for group in groups {
                if let Some(group_name) = group.get("GroupName").and_then(|name| name.as_str()) {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: group_name.to_string(),
                        target_resource_type: "AWS::IAM::Group".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::IAM::User"
    }
}

pub struct IAMPolicyNormalizer;

impl ResourceNormalizer for IAMPolicyNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let policy_name = raw_response
            .get("PolicyName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-policy")
            .to_string();

        let policy_id = raw_response
            .get("PolicyId")
            .and_then(|v| v.as_str())
            .unwrap_or(&policy_name)
            .to_string();

        let display_name = policy_name.clone();
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::IAM::Policy".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: policy_id,
            display_name,
            status: None,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Policies are referenced by other resources, no outbound relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::IAM::Policy"
    }
}
