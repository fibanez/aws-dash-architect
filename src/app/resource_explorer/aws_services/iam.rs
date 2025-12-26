use super::super::credentials::CredentialCoordinator;
use super::super::normalizers::expand_embedded_json;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_iam as iam;
use percent_encoding::percent_decode;
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

    /// List IAM roles with optional detailed policy information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic role info quickly.
    ///   If true (Phase 2), includes attached and inline policies.
    pub async fn list_roles(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
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

        report_status("IAM", "list_roles", Some(account_id));

        let roles_result = self
            .list_roles_internal(&client, account_id, include_details)
            .await;
        report_status_done("IAM", "list_roles", Some(account_id));
        roles_result
    }

    async fn list_roles_internal(
        &self,
        client: &iam::Client,
        _account_id: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
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
                let mut role_json = self.role_to_json(&role);
                let role_name = role.role_name.as_str();

                // Only fetch policy details if requested (Phase 2)
                if include_details {
                    // Add attached managed policies
                    report_status("IAM", "role_policies", Some(role_name));
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

                    // Add inline policies with their documents
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
                                        serde_json::Value::String(policy_name.clone()),
                                    );

                                    // Decode URL-encoded policy document
                                    let doc = policy_response.policy_document;
                                    let decoded_doc =
                                        percent_decode(doc.as_bytes()).decode_utf8_lossy();
                                    policy_json.insert(
                                        "PolicyDocument".to_string(),
                                        serde_json::Value::String(decoded_doc.to_string()),
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
                }

                // Apply JSON expansion to expand embedded JSON strings
                roles.push(expand_embedded_json(role_json));
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

    /// List IAM users with optional comprehensive security details
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic user info quickly.
    ///   If true (Phase 2), includes policies, access keys, MFA, groups, etc.
    pub async fn list_users(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
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

        report_status("IAM", "list_users", Some(account_id));

        let users_result = self.list_users_internal(&client, include_details).await;
        report_status_done("IAM", "list_users", Some(account_id));
        users_result
    }

    async fn list_users_internal(
        &self,
        client: &iam::Client,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
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
                let mut user_json = self.user_to_json(&user);
                let user_name = user.user_name.as_str();

                // Only fetch details if requested (Phase 2)
                if include_details {
                    // Report status for user details
                    report_status("IAM", "user_details", Some(user_name));

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

                    // Add inline policies with their documents
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
                                        serde_json::Value::String(policy_name.clone()),
                                    );

                                    // Decode URL-encoded policy document
                                    let doc = policy_response.policy_document;
                                    let decoded_doc =
                                        percent_decode(doc.as_bytes()).decode_utf8_lossy();
                                    policy_json.insert(
                                        "PolicyDocument".to_string(),
                                        serde_json::Value::String(decoded_doc.to_string()),
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

                    // Add access keys for security audit
                    if let Ok(access_keys_response) =
                        client.list_access_keys().user_name(user_name).send().await
                    {
                        let access_keys = access_keys_response.access_key_metadata;
                        let access_keys_json: Vec<serde_json::Value> = access_keys
                            .into_iter()
                            .map(|key| {
                                let mut key_json = serde_json::Map::new();
                                if let Some(access_key_id) = key.access_key_id {
                                    key_json.insert(
                                        "AccessKeyId".to_string(),
                                        serde_json::Value::String(access_key_id),
                                    );
                                }
                                if let Some(status) = key.status {
                                    key_json.insert(
                                        "Status".to_string(),
                                        serde_json::Value::String(status.as_str().to_string()),
                                    );
                                }
                                if let Some(create_date) = key.create_date {
                                    key_json.insert(
                                        "CreateDate".to_string(),
                                        serde_json::Value::String(create_date.to_string()),
                                    );
                                }
                                serde_json::Value::Object(key_json)
                            })
                            .collect();

                        if let serde_json::Value::Object(ref mut map) = user_json {
                            map.insert(
                                "AccessKeys".to_string(),
                                serde_json::Value::Array(access_keys_json),
                            );
                        }
                    }

                    // Add MFA devices
                    if let Ok(mfa_response) =
                        client.list_mfa_devices().user_name(user_name).send().await
                    {
                        let mfa_devices = mfa_response.mfa_devices;
                        let mfa_json: Vec<serde_json::Value> = mfa_devices
                            .into_iter()
                            .map(|device| {
                                let mut device_json = serde_json::Map::new();
                                device_json.insert(
                                    "SerialNumber".to_string(),
                                    serde_json::Value::String(device.serial_number),
                                );
                                device_json.insert(
                                    "EnableDate".to_string(),
                                    serde_json::Value::String(device.enable_date.to_string()),
                                );
                                serde_json::Value::Object(device_json)
                            })
                            .collect();

                        if let serde_json::Value::Object(ref mut map) = user_json {
                            map.insert(
                                "MFADevices".to_string(),
                                serde_json::Value::Array(mfa_json),
                            );
                        }
                    }

                    // Add login profile (console access) status
                    match client.get_login_profile().user_name(user_name).send().await {
                        Ok(login_response) => {
                            if let Some(login_profile) = login_response.login_profile {
                                let mut profile_json = serde_json::Map::new();
                                profile_json.insert(
                                    "CreateDate".to_string(),
                                    serde_json::Value::String(
                                        login_profile.create_date.to_string(),
                                    ),
                                );
                                profile_json.insert(
                                    "PasswordResetRequired".to_string(),
                                    serde_json::Value::Bool(login_profile.password_reset_required),
                                );
                                if let serde_json::Value::Object(ref mut map) = user_json {
                                    map.insert(
                                        "LoginProfile".to_string(),
                                        serde_json::Value::Object(profile_json),
                                    );
                                    map.insert(
                                        "ConsoleAccess".to_string(),
                                        serde_json::Value::Bool(true),
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            // NoSuchEntity error means no console access - this is expected
                            if let serde_json::Value::Object(ref mut map) = user_json {
                                map.insert(
                                    "ConsoleAccess".to_string(),
                                    serde_json::Value::Bool(false),
                                );
                            }
                        }
                    }

                    // Add user groups
                    if let Ok(groups_response) = client
                        .list_groups_for_user()
                        .user_name(user_name)
                        .send()
                        .await
                    {
                        let groups_json: Vec<serde_json::Value> = groups_response
                            .groups
                            .into_iter()
                            .map(|g| {
                                let mut group_json = serde_json::Map::new();
                                group_json.insert(
                                    "GroupName".to_string(),
                                    serde_json::Value::String(g.group_name),
                                );
                                group_json.insert(
                                    "GroupId".to_string(),
                                    serde_json::Value::String(g.group_id),
                                );
                                group_json
                                    .insert("Arn".to_string(), serde_json::Value::String(g.arn));
                                serde_json::Value::Object(group_json)
                            })
                            .collect();

                        if let serde_json::Value::Object(ref mut map) = user_json {
                            map.insert("Groups".to_string(), serde_json::Value::Array(groups_json));
                        }
                    }
                } // end if include_details

                // Apply JSON expansion to expand embedded JSON strings
                users.push(expand_embedded_json(user_json));
            }

            if response.is_truncated {
                marker = response.marker;
            } else {
                break;
            }
        }

        Ok(users)
    }

    /// List IAM policies with optional policy documents
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic policy info quickly.
    ///   If true (Phase 2), includes policy document and versions.
    pub async fn list_policies(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
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

        report_status("IAM", "list_policies", Some(account_id));

        let policies_result = self.list_policies_internal(&client, include_details).await;
        report_status_done("IAM", "list_policies", Some(account_id));
        policies_result
    }

    async fn list_policies_internal(
        &self,
        client: &iam::Client,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
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
                    let mut policy_json = self.policy_to_json(&policy);

                    // Only fetch details if requested (Phase 2)
                    if include_details {
                        let policy_name = policy.policy_name.as_deref().unwrap_or("unknown");
                        report_status("IAM", "policy_details", Some(policy_name));

                        // Get the policy document for the default version
                        if let (Some(arn), Some(default_version_id)) =
                            (policy.arn.as_ref(), policy.default_version_id.as_ref())
                        {
                            if let Ok(version_response) = client
                                .get_policy_version()
                                .policy_arn(arn)
                                .version_id(default_version_id)
                                .send()
                                .await
                            {
                                if let Some(policy_version) = version_response.policy_version {
                                    if let Some(doc) = policy_version.document {
                                        // Decode URL-encoded policy document
                                        let decoded_doc =
                                            percent_decode(doc.as_bytes()).decode_utf8_lossy();

                                        if let serde_json::Value::Object(ref mut map) = policy_json
                                        {
                                            map.insert(
                                                "PolicyDocument".to_string(),
                                                serde_json::Value::String(decoded_doc.to_string()),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Get list of all policy versions
                        if let Some(arn) = policy.arn.as_ref() {
                            if let Ok(versions_response) =
                                client.list_policy_versions().policy_arn(arn).send().await
                            {
                                if let Some(versions) = versions_response.versions {
                                    let versions_json: Vec<serde_json::Value> = versions
                                        .into_iter()
                                        .map(|v| {
                                            let mut version_json = serde_json::Map::new();
                                            if let Some(version_id) = v.version_id {
                                                version_json.insert(
                                                    "VersionId".to_string(),
                                                    serde_json::Value::String(version_id),
                                                );
                                            }
                                            version_json.insert(
                                                "IsDefaultVersion".to_string(),
                                                serde_json::Value::Bool(v.is_default_version),
                                            );
                                            if let Some(create_date) = v.create_date {
                                                version_json.insert(
                                                    "CreateDate".to_string(),
                                                    serde_json::Value::String(
                                                        create_date.to_string(),
                                                    ),
                                                );
                                            }
                                            serde_json::Value::Object(version_json)
                                        })
                                        .collect();

                                    if let serde_json::Value::Object(ref mut map) = policy_json {
                                        map.insert(
                                            "PolicyVersions".to_string(),
                                            serde_json::Value::Array(versions_json),
                                        );
                                    }
                                }
                            }
                        }
                    } // end if include_details

                    // Apply JSON expansion to expand embedded JSON strings
                    policies.push(expand_embedded_json(policy_json));
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

            Ok(expand_embedded_json(role_json))
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

            // Add access keys for security audit
            if let Ok(access_keys_response) =
                client.list_access_keys().user_name(user_name).send().await
            {
                let access_keys = access_keys_response.access_key_metadata;
                if !access_keys.is_empty() {
                    let access_keys_json: Vec<serde_json::Value> = access_keys
                        .into_iter()
                        .map(|key| {
                            let mut key_json = serde_json::Map::new();
                            if let Some(access_key_id) = key.access_key_id {
                                key_json.insert(
                                    "AccessKeyId".to_string(),
                                    serde_json::Value::String(access_key_id),
                                );
                            }
                            if let Some(status) = key.status {
                                key_json.insert(
                                    "Status".to_string(),
                                    serde_json::Value::String(status.as_str().to_string()),
                                );
                            }
                            if let Some(create_date) = key.create_date {
                                key_json.insert(
                                    "CreateDate".to_string(),
                                    serde_json::Value::String(create_date.to_string()),
                                );
                            }
                            serde_json::Value::Object(key_json)
                        })
                        .collect();

                    if let serde_json::Value::Object(ref mut map) = user_json {
                        map.insert(
                            "AccessKeys".to_string(),
                            serde_json::Value::Array(access_keys_json),
                        );
                    }
                }
            }

            // Add MFA devices
            if let Ok(mfa_response) = client.list_mfa_devices().user_name(user_name).send().await {
                let mfa_devices = mfa_response.mfa_devices;
                if !mfa_devices.is_empty() {
                    let mfa_json: Vec<serde_json::Value> = mfa_devices
                        .into_iter()
                        .map(|device| {
                            let mut device_json = serde_json::Map::new();
                            device_json.insert(
                                "SerialNumber".to_string(),
                                serde_json::Value::String(device.serial_number),
                            );
                            device_json.insert(
                                "EnableDate".to_string(),
                                serde_json::Value::String(device.enable_date.to_string()),
                            );
                            serde_json::Value::Object(device_json)
                        })
                        .collect();

                    if let serde_json::Value::Object(ref mut map) = user_json {
                        map.insert("MFADevices".to_string(), serde_json::Value::Array(mfa_json));
                    }
                } else if let serde_json::Value::Object(ref mut map) = user_json {
                    // Explicitly show no MFA for security visibility
                    map.insert(
                        "MFADevices".to_string(),
                        serde_json::Value::Array(Vec::new()),
                    );
                }
            }

            // Add login profile (console access) status
            match client.get_login_profile().user_name(user_name).send().await {
                Ok(login_response) => {
                    if let Some(login_profile) = login_response.login_profile {
                        let mut profile_json = serde_json::Map::new();
                        profile_json.insert(
                            "CreateDate".to_string(),
                            serde_json::Value::String(login_profile.create_date.to_string()),
                        );
                        profile_json.insert(
                            "PasswordResetRequired".to_string(),
                            serde_json::Value::Bool(login_profile.password_reset_required),
                        );
                        if let serde_json::Value::Object(ref mut map) = user_json {
                            map.insert(
                                "LoginProfile".to_string(),
                                serde_json::Value::Object(profile_json),
                            );
                            map.insert("ConsoleAccess".to_string(), serde_json::Value::Bool(true));
                        }
                    }
                }
                Err(_) => {
                    // NoSuchEntity error means no console access - this is expected
                    if let serde_json::Value::Object(ref mut map) = user_json {
                        map.insert("ConsoleAccess".to_string(), serde_json::Value::Bool(false));
                    }
                }
            }

            Ok(expand_embedded_json(user_json))
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

            Ok(expand_embedded_json(policy_json))
        } else {
            Err(anyhow::anyhow!("Policy {} not found", policy_arn))
        }
    }

    /// Get details for a specific IAM role (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_role_details(
        &self,
        account_id: &str,
        region: &str,
        role_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("IAM", "get_role_details", Some(role_name));
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = iam::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Fetch attached managed policies
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
                            policy_json
                                .insert("PolicyName".to_string(), serde_json::Value::String(name));
                        }
                        if let Some(arn) = p.policy_arn {
                            policy_json
                                .insert("PolicyArn".to_string(), serde_json::Value::String(arn));
                        }
                        serde_json::Value::Object(policy_json)
                    })
                    .collect();

                details.insert(
                    "AttachedManagedPolicies".to_string(),
                    serde_json::Value::Array(policies_json),
                );
            }
        }

        // Fetch inline policies with documents
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
                            serde_json::Value::String(policy_name.clone()),
                        );

                        let doc = policy_response.policy_document;
                        let decoded_doc = percent_decode(doc.as_bytes()).decode_utf8_lossy();
                        policy_json.insert(
                            "PolicyDocument".to_string(),
                            serde_json::Value::String(decoded_doc.to_string()),
                        );

                        inline_policies.push(serde_json::Value::Object(policy_json));
                    }
                }

                details.insert(
                    "InlinePolicies".to_string(),
                    serde_json::Value::Array(inline_policies),
                );
            }
        }

        report_status_done("IAM", "get_role_details", Some(role_name));
        Ok(expand_embedded_json(serde_json::Value::Object(details)))
    }

    /// Get details for a specific IAM user (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_user_details(
        &self,
        account_id: &str,
        region: &str,
        user_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("IAM", "get_user_details", Some(user_name));
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = iam::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Fetch attached managed policies
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
                            policy_json
                                .insert("PolicyName".to_string(), serde_json::Value::String(name));
                        }
                        if let Some(arn) = p.policy_arn {
                            policy_json
                                .insert("PolicyArn".to_string(), serde_json::Value::String(arn));
                        }
                        serde_json::Value::Object(policy_json)
                    })
                    .collect();

                details.insert(
                    "AttachedManagedPolicies".to_string(),
                    serde_json::Value::Array(policies_json),
                );
            }
        }

        // Fetch inline policies with documents
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
                            serde_json::Value::String(policy_name.clone()),
                        );

                        let doc = policy_response.policy_document;
                        let decoded_doc = percent_decode(doc.as_bytes()).decode_utf8_lossy();
                        policy_json.insert(
                            "PolicyDocument".to_string(),
                            serde_json::Value::String(decoded_doc.to_string()),
                        );

                        inline_policies.push(serde_json::Value::Object(policy_json));
                    }
                }

                details.insert(
                    "InlinePolicies".to_string(),
                    serde_json::Value::Array(inline_policies),
                );
            }
        }

        // Fetch access keys for security audit
        if let Ok(access_keys_response) =
            client.list_access_keys().user_name(user_name).send().await
        {
            let access_keys = access_keys_response.access_key_metadata;
            let access_keys_json: Vec<serde_json::Value> = access_keys
                .into_iter()
                .map(|key| {
                    let mut key_json = serde_json::Map::new();
                    if let Some(access_key_id) = key.access_key_id {
                        key_json.insert(
                            "AccessKeyId".to_string(),
                            serde_json::Value::String(access_key_id),
                        );
                    }
                    if let Some(status) = key.status {
                        key_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(status.as_str().to_string()),
                        );
                    }
                    if let Some(create_date) = key.create_date {
                        key_json.insert(
                            "CreateDate".to_string(),
                            serde_json::Value::String(create_date.to_string()),
                        );
                    }
                    serde_json::Value::Object(key_json)
                })
                .collect();

            details.insert(
                "AccessKeys".to_string(),
                serde_json::Value::Array(access_keys_json),
            );
        }

        // Fetch MFA devices
        if let Ok(mfa_response) = client.list_mfa_devices().user_name(user_name).send().await {
            let mfa_devices = mfa_response.mfa_devices;
            let mfa_json: Vec<serde_json::Value> = mfa_devices
                .into_iter()
                .map(|device| {
                    let mut device_json = serde_json::Map::new();
                    device_json.insert(
                        "SerialNumber".to_string(),
                        serde_json::Value::String(device.serial_number),
                    );
                    device_json.insert(
                        "EnableDate".to_string(),
                        serde_json::Value::String(device.enable_date.to_string()),
                    );
                    serde_json::Value::Object(device_json)
                })
                .collect();

            details.insert("MFADevices".to_string(), serde_json::Value::Array(mfa_json));
        }

        // Fetch login profile (console access) status
        match client.get_login_profile().user_name(user_name).send().await {
            Ok(login_response) => {
                if let Some(login_profile) = login_response.login_profile {
                    let mut profile_json = serde_json::Map::new();
                    profile_json.insert(
                        "CreateDate".to_string(),
                        serde_json::Value::String(login_profile.create_date.to_string()),
                    );
                    profile_json.insert(
                        "PasswordResetRequired".to_string(),
                        serde_json::Value::Bool(login_profile.password_reset_required),
                    );
                    details.insert(
                        "LoginProfile".to_string(),
                        serde_json::Value::Object(profile_json),
                    );
                    details.insert("ConsoleAccess".to_string(), serde_json::Value::Bool(true));
                }
            }
            Err(_) => {
                // NoSuchEntity error means no console access - this is expected
                details.insert("ConsoleAccess".to_string(), serde_json::Value::Bool(false));
            }
        }

        // Fetch user groups
        if let Ok(groups_response) = client
            .list_groups_for_user()
            .user_name(user_name)
            .send()
            .await
        {
            let groups_json: Vec<serde_json::Value> = groups_response
                .groups
                .into_iter()
                .map(|g| {
                    let mut group_json = serde_json::Map::new();
                    group_json.insert(
                        "GroupName".to_string(),
                        serde_json::Value::String(g.group_name),
                    );
                    group_json.insert("GroupId".to_string(), serde_json::Value::String(g.group_id));
                    group_json.insert("Arn".to_string(), serde_json::Value::String(g.arn));
                    serde_json::Value::Object(group_json)
                })
                .collect();

            details.insert("Groups".to_string(), serde_json::Value::Array(groups_json));
        }

        report_status_done("IAM", "get_user_details", Some(user_name));
        Ok(expand_embedded_json(serde_json::Value::Object(details)))
    }

    /// Get details for a specific IAM policy (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_policy_details(
        &self,
        account_id: &str,
        region: &str,
        policy_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("IAM", "get_policy_details", Some(policy_arn));
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = iam::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Fetch policy info to get default version
        if let Ok(policy_response) = client.get_policy().policy_arn(policy_arn).send().await {
            if let Some(policy) = policy_response.policy {
                // Get the policy document for the default version
                if let Some(default_version_id) = &policy.default_version_id {
                    if let Ok(version_response) = client
                        .get_policy_version()
                        .policy_arn(policy_arn)
                        .version_id(default_version_id)
                        .send()
                        .await
                    {
                        if let Some(policy_version) = version_response.policy_version {
                            if let Some(doc) = policy_version.document {
                                let decoded_doc =
                                    percent_decode(doc.as_bytes()).decode_utf8_lossy();
                                details.insert(
                                    "PolicyDocument".to_string(),
                                    serde_json::Value::String(decoded_doc.to_string()),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Fetch all policy versions
        if let Ok(versions_response) = client
            .list_policy_versions()
            .policy_arn(policy_arn)
            .send()
            .await
        {
            if let Some(versions) = versions_response.versions {
                let versions_json: Vec<serde_json::Value> = versions
                    .into_iter()
                    .map(|v| {
                        let mut version_json = serde_json::Map::new();
                        if let Some(version_id) = v.version_id {
                            version_json.insert(
                                "VersionId".to_string(),
                                serde_json::Value::String(version_id),
                            );
                        }
                        version_json.insert(
                            "IsDefaultVersion".to_string(),
                            serde_json::Value::Bool(v.is_default_version),
                        );
                        if let Some(create_date) = v.create_date {
                            version_json.insert(
                                "CreateDate".to_string(),
                                serde_json::Value::String(create_date.to_string()),
                            );
                        }
                        serde_json::Value::Object(version_json)
                    })
                    .collect();

                details.insert(
                    "PolicyVersions".to_string(),
                    serde_json::Value::Array(versions_json),
                );
            }
        }

        report_status_done("IAM", "get_policy_details", Some(policy_arn));
        Ok(expand_embedded_json(serde_json::Value::Object(details)))
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
