use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_iam as iam;
use std::sync::Arc;

pub struct IAMService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl IAMService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List IAM roles (basic list data)
    pub async fn list_roles(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = iam::Client::new(&aws_config);
        let mut roles = Vec::new();

        // Use manual pagination for IAM roles
        let mut marker: Option<String> = None;

        loop {
            let mut request = client.list_roles().max_items(100);

            if let Some(m) = &marker {
                request = request.marker(m);
            }

            let response = request.send().await?;

            let role_list = response.roles;
            for role in role_list {
                let role_json = self.role_to_json(&role);
                roles.push(role_json);
            }

            // Check if we have more pages
            if response.is_truncated {
                marker = response.marker;
            } else {
                break;
            }
        }

        Ok(roles)
    }

    /// List IAM users (basic list data)
    pub async fn list_users(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = iam::Client::new(&aws_config);
        let mut users = Vec::new();

        let mut marker: Option<String> = None;

        loop {
            let mut request = client.list_users().max_items(100);

            if let Some(m) = &marker {
                request = request.marker(m);
            }

            let response = request.send().await?;

            let user_list = response.users;
            for user in user_list {
                let user_json = self.user_to_json(&user);
                users.push(user_json);
            }

            if response.is_truncated {
                marker = response.marker;
            } else {
                break;
            }
        }

        Ok(users)
    }

    /// List IAM policies (basic list data)
    pub async fn list_policies(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = iam::Client::new(&aws_config);
        let mut policies = Vec::new();

        let mut marker: Option<String> = None;

        loop {
            let mut request = client
                .list_policies()
                .max_items(100)
                .scope(iam::types::PolicyScopeType::Local); // Only customer-managed policies

            if let Some(m) = &marker {
                request = request.marker(m);
            }

            let response = request.send().await?;

            if let Some(policy_list) = response.policies {
                for policy in policy_list {
                    let policy_json = self.policy_to_json(&policy);
                    policies.push(policy_json);
                }
            }

            if response.is_truncated {
                marker = response.marker;
            } else {
                break;
            }
        }

        Ok(policies)
    }

    /// Get detailed information for a specific IAM role
    pub async fn describe_role(
        &self,
        account_id: &str,
        region: &str,
        role_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = iam::Client::new(&aws_config);
        let response = client.get_role().role_name(role_name).send().await?;

        if let Some(role) = response.role {
            let mut role_json = self.role_to_json(&role);

            // Add attached managed policies
            if let Ok(policies_response) = client
                .list_attached_role_policies()
                .role_name(role_name)
                .send()
                .await
            {
                if let Some(policies) = policies_response.attached_policies {
                    let policies_json: Vec<serde_json::Value> = policies
                        .into_iter()
                        .map(|p| {
                            let mut policy_json = serde_json::Map::new();
                            if let Some(name) = p.policy_name {
                                policy_json.insert(
                                    "PolicyName".to_string(),
                                    serde_json::Value::String(name),
                                );
                            }
                            if let Some(arn) = p.policy_arn {
                                policy_json.insert(
                                    "PolicyArn".to_string(),
                                    serde_json::Value::String(arn),
                                );
                            }
                            serde_json::Value::Object(policy_json)
                        })
                        .collect();

                    if let serde_json::Value::Object(ref mut map) = role_json {
                        map.insert(
                            "AttachedManagedPolicies".to_string(),
                            serde_json::Value::Array(policies_json),
                        );
                    }
                }
            }

            // Add inline policies
            if let Ok(inline_policies_response) = client
                .list_role_policies()
                .role_name(role_name)
                .send()
                .await
            {
                let inline_policy_names = inline_policies_response.policy_names;
                if !inline_policy_names.is_empty() {
                    let mut inline_policies = Vec::new();

                    for policy_name in inline_policy_names {
                        if let Ok(policy_response) = client
                            .get_role_policy()
                            .role_name(role_name)
                            .policy_name(&policy_name)
                            .send()
                            .await
                        {
                            let mut policy_json = serde_json::Map::new();
                            policy_json.insert(
                                "PolicyName".to_string(),
                                serde_json::Value::String(policy_name),
                            );
                            policy_json.insert(
                                "PolicyDocument".to_string(),
                                serde_json::Value::String(policy_response.policy_document),
                            );
                            inline_policies.push(serde_json::Value::Object(policy_json));
                        }
                    }

                    if let serde_json::Value::Object(ref mut map) = role_json {
                        map.insert(
                            "InlinePolicies".to_string(),
                            serde_json::Value::Array(inline_policies),
                        );
                    }
                }
            }

            Ok(role_json)
        } else {
            Err(anyhow::anyhow!("Role {} not found", role_name))
        }
    }

    /// Get detailed information for a specific IAM user
    pub async fn describe_user(
        &self,
        account_id: &str,
        region: &str,
        user_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = iam::Client::new(&aws_config);
        let response = client.get_user().user_name(user_name).send().await?;

        if let Some(user) = response.user {
            let mut user_json = self.user_to_json(&user);

            // Add attached managed policies
            if let Ok(policies_response) = client
                .list_attached_user_policies()
                .user_name(user_name)
                .send()
                .await
            {
                if let Some(policies) = policies_response.attached_policies {
                    let policies_json: Vec<serde_json::Value> = policies
                        .into_iter()
                        .map(|p| {
                            let mut policy_json = serde_json::Map::new();
                            if let Some(name) = p.policy_name {
                                policy_json.insert(
                                    "PolicyName".to_string(),
                                    serde_json::Value::String(name),
                                );
                            }
                            if let Some(arn) = p.policy_arn {
                                policy_json.insert(
                                    "PolicyArn".to_string(),
                                    serde_json::Value::String(arn),
                                );
                            }
                            serde_json::Value::Object(policy_json)
                        })
                        .collect();

                    if let serde_json::Value::Object(ref mut map) = user_json {
                        map.insert(
                            "AttachedManagedPolicies".to_string(),
                            serde_json::Value::Array(policies_json),
                        );
                    }
                }
            }

            // Add groups
            if let Ok(groups_response) = client
                .list_groups_for_user()
                .user_name(user_name)
                .send()
                .await
            {
                let groups = groups_response.groups;
                let groups_json: Vec<serde_json::Value> = groups
                    .into_iter()
                    .map(|g| {
                        let mut group_json = serde_json::Map::new();
                        group_json.insert(
                            "GroupName".to_string(),
                            serde_json::Value::String(g.group_name),
                        );
                        group_json.insert("Arn".to_string(), serde_json::Value::String(g.arn));
                        let path = &g.path;
                        group_json
                            .insert("Path".to_string(), serde_json::Value::String(path.clone()));
                        serde_json::Value::Object(group_json)
                    })
                    .collect();

                if let serde_json::Value::Object(ref mut map) = user_json {
                    map.insert("Groups".to_string(), serde_json::Value::Array(groups_json));
                }
            }

            // Add inline policies
            if let Ok(inline_policies_response) = client
                .list_user_policies()
                .user_name(user_name)
                .send()
                .await
            {
                let inline_policy_names = inline_policies_response.policy_names;
                if !inline_policy_names.is_empty() {
                    let mut inline_policies = Vec::new();

                    for policy_name in inline_policy_names {
                        if let Ok(policy_response) = client
                            .get_user_policy()
                            .user_name(user_name)
                            .policy_name(&policy_name)
                            .send()
                            .await
                        {
                            let mut policy_json = serde_json::Map::new();
                            policy_json.insert(
                                "PolicyName".to_string(),
                                serde_json::Value::String(policy_name),
                            );
                            policy_json.insert(
                                "PolicyDocument".to_string(),
                                serde_json::Value::String(policy_response.policy_document),
                            );
                            inline_policies.push(serde_json::Value::Object(policy_json));
                        }
                    }

                    if let serde_json::Value::Object(ref mut map) = user_json {
                        map.insert(
                            "InlinePolicies".to_string(),
                            serde_json::Value::Array(inline_policies),
                        );
                    }
                }
            }

            Ok(user_json)
        } else {
            Err(anyhow::anyhow!("User {} not found", user_name))
        }
    }

    /// Get detailed information for a specific IAM policy
    pub async fn describe_policy(
        &self,
        account_id: &str,
        region: &str,
        policy_arn: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = iam::Client::new(&aws_config);
        let response = client.get_policy().policy_arn(policy_arn).send().await?;

        if let Some(policy) = response.policy {
            let mut policy_json = self.policy_to_json(&policy);

            // Get the policy version (default version)
            if let Some(default_version_id) = &policy.default_version_id {
                if let Ok(version_response) = client
                    .get_policy_version()
                    .policy_arn(policy_arn)
                    .version_id(default_version_id)
                    .send()
                    .await
                {
                    if let Some(policy_version) = version_response.policy_version {
                        if let Some(document) = policy_version.document {
                            if let serde_json::Value::Object(ref mut map) = policy_json {
                                map.insert(
                                    "PolicyDocument".to_string(),
                                    serde_json::Value::String(document),
                                );
                            }
                        }
                    }
                }
            }

            Ok(policy_json)
        } else {
            Err(anyhow::anyhow!("Policy {} not found", policy_arn))
        }
    }

    // JSON conversion methods
    fn role_to_json(&self, role: &iam::types::Role) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "RoleName".to_string(),
            serde_json::Value::String(role.role_name.clone()),
        );
        json.insert(
            "RoleId".to_string(),
            serde_json::Value::String(role.role_id.clone()),
        );
        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(role.arn.clone()),
        );
        json.insert(
            "Path".to_string(),
            serde_json::Value::String(role.path.clone()),
        );
        json.insert(
            "CreateDate".to_string(),
            serde_json::Value::String(role.create_date.to_string()),
        );

        if let Some(description) = &role.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(max_session_duration) = &role.max_session_duration {
            json.insert(
                "MaxSessionDuration".to_string(),
                serde_json::Value::Number((*max_session_duration).into()),
            );
        }

        // Add assume role policy document
        if let Some(assume_role_policy) = &role.assume_role_policy_document {
            json.insert(
                "AssumeRolePolicyDocument".to_string(),
                serde_json::Value::String(assume_role_policy.clone()),
            );
        }

        // Add tags
        if let Some(ref tags) = role.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        tag_json.insert(
                            "Key".to_string(),
                            serde_json::Value::String(tag.key.clone()),
                        );
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(tag.value.clone()),
                        );
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    fn user_to_json(&self, user: &iam::types::User) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "UserName".to_string(),
            serde_json::Value::String(user.user_name.clone()),
        );
        json.insert(
            "UserId".to_string(),
            serde_json::Value::String(user.user_id.clone()),
        );
        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(user.arn.clone()),
        );
        json.insert(
            "Path".to_string(),
            serde_json::Value::String(user.path.clone()),
        );
        json.insert(
            "CreateDate".to_string(),
            serde_json::Value::String(user.create_date.to_string()),
        );

        if let Some(password_last_used) = &user.password_last_used {
            json.insert(
                "PasswordLastUsed".to_string(),
                serde_json::Value::String(password_last_used.to_string()),
            );
        }

        // Add tags
        if let Some(ref tags) = user.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        tag_json.insert(
                            "Key".to_string(),
                            serde_json::Value::String(tag.key.clone()),
                        );
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(tag.value.clone()),
                        );
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    fn policy_to_json(&self, policy: &iam::types::Policy) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &policy.policy_name {
            json.insert(
                "PolicyName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(id) = &policy.policy_id {
            json.insert(
                "PolicyId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(arn) = &policy.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(path) = &policy.path {
            json.insert("Path".to_string(), serde_json::Value::String(path.clone()));
        }

        if let Some(description) = &policy.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(create_date) = &policy.create_date {
            json.insert(
                "CreateDate".to_string(),
                serde_json::Value::String(create_date.to_string()),
            );
        }

        if let Some(update_date) = &policy.update_date {
            json.insert(
                "UpdateDate".to_string(),
                serde_json::Value::String(update_date.to_string()),
            );
        }

        if let Some(attachment_count) = &policy.attachment_count {
            json.insert(
                "AttachmentCount".to_string(),
                serde_json::Value::Number((*attachment_count).into()),
            );
        }

        // Add tags
        if let Some(ref tags) = policy.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        tag_json.insert(
                            "Key".to_string(),
                            serde_json::Value::String(tag.key.clone()),
                        );
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(tag.value.clone()),
                        );
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }
}
