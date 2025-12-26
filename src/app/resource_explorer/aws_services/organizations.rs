use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_organizations as organizations;
use std::sync::Arc;

pub struct OrganizationsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl OrganizationsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List AWS Organizations Accounts
    pub async fn list_accounts(
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

        let client = organizations::Client::new(&aws_config);

        let mut accounts = Vec::new();
        let mut paginator = client.list_accounts().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(account_summaries) = page.accounts {
                for account in account_summaries {
                    let account_json = self.account_to_json(&account);
                    accounts.push(account_json);
                }
            }
        }

        Ok(accounts)
    }

    /// Get detailed information for specific Account
    pub async fn describe_account(
        &self,
        account_id: &str,
        region: &str,
        target_account_id: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client
            .describe_account()
            .account_id(target_account_id)
            .send()
            .await?;

        if let Some(account) = response.account {
            Ok(self.account_details_to_json(&account))
        } else {
            Err(anyhow::anyhow!("Account {} not found", target_account_id))
        }
    }

    /// List AWS Organizations Roots
    pub async fn list_roots(
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

        let client = organizations::Client::new(&aws_config);

        let mut roots = Vec::new();
        let mut paginator = client.list_roots().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(root_summaries) = page.roots {
                for root in root_summaries {
                    let root_json = self.root_to_json(&root);
                    roots.push(root_json);
                }
            }
        }

        Ok(roots)
    }

    /// Get Organization details (singleton resource)
    pub async fn describe_organization(
        &self,
        account_id: &str,
        region: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client.describe_organization().send().await?;

        if let Some(org) = response.organization {
            Ok(self.organization_to_json(&org))
        } else {
            Err(anyhow::anyhow!("Organization not found"))
        }
    }

    // ==================== Relationship Discovery Methods ====================

    /// List accounts under a specific parent (OU or Root)
    pub async fn list_accounts_for_parent(
        &self,
        account_id: &str,
        region: &str,
        parent_id: &str,
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

        let client = organizations::Client::new(&aws_config);

        let mut accounts = Vec::new();
        let mut paginator = client
            .list_accounts_for_parent()
            .parent_id(parent_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(account_summaries) = page.accounts {
                for account in account_summaries {
                    let account_json = self.account_to_json(&account);
                    accounts.push(account_json);
                }
            }
        }

        Ok(accounts)
    }

    /// List policies attached to a specific target (OU, Account, or Root)
    pub async fn list_policies_for_target(
        &self,
        account_id: &str,
        region: &str,
        target_id: &str,
        filter: organizations::types::PolicyType,
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

        let client = organizations::Client::new(&aws_config);

        let mut policies = Vec::new();
        let mut paginator = client
            .list_policies_for_target()
            .target_id(target_id)
            .filter(filter)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(policy_summaries) = page.policies {
                for policy in policy_summaries {
                    let policy_json = self.policy_summary_to_json(&policy);
                    policies.push(policy_json);
                }
            }
        }

        Ok(policies)
    }

    /// List targets that have a specific policy attached
    pub async fn list_targets_for_policy(
        &self,
        account_id: &str,
        region: &str,
        policy_id: &str,
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

        let client = organizations::Client::new(&aws_config);

        let mut targets = Vec::new();
        let mut paginator = client
            .list_targets_for_policy()
            .policy_id(policy_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(policy_targets) = page.targets {
                for target in policy_targets {
                    let mut target_json = serde_json::Map::new();

                    if let Some(target_id) = &target.target_id {
                        target_json.insert(
                            "TargetId".to_string(),
                            serde_json::Value::String(target_id.clone()),
                        );
                    }

                    if let Some(arn) = &target.arn {
                        target_json
                            .insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
                    }

                    if let Some(name) = &target.name {
                        target_json
                            .insert("Name".to_string(), serde_json::Value::String(name.clone()));
                    }

                    if let Some(target_type) = &target.r#type {
                        target_json.insert(
                            "Type".to_string(),
                            serde_json::Value::String(target_type.as_str().to_string()),
                        );
                    }

                    targets.push(serde_json::Value::Object(target_json));
                }
            }
        }

        Ok(targets)
    }

    /// List parents of a resource (account or OU)
    pub async fn list_parents(
        &self,
        account_id: &str,
        region: &str,
        child_id: &str,
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

        let client = organizations::Client::new(&aws_config);

        let mut parents = Vec::new();
        let mut paginator = client
            .list_parents()
            .child_id(child_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(parent_summaries) = page.parents {
                for parent in parent_summaries {
                    let mut parent_json = serde_json::Map::new();

                    if let Some(parent_id) = &parent.id {
                        parent_json.insert(
                            "Id".to_string(),
                            serde_json::Value::String(parent_id.clone()),
                        );
                    }

                    if let Some(parent_type) = &parent.r#type {
                        parent_json.insert(
                            "Type".to_string(),
                            serde_json::Value::String(parent_type.as_str().to_string()),
                        );
                    }

                    parents.push(serde_json::Value::Object(parent_json));
                }
            }
        }

        Ok(parents)
    }

    // ==================== Core Resource Methods ====================

    /// List AWS Organizations Organizational Units
    pub async fn list_organizational_units(
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

        let client = organizations::Client::new(&aws_config);

        // First, we need to get the root to list OUs
        let roots_response = client.list_roots().send().await?;

        let mut organizational_units = Vec::new();

        if let Some(roots) = roots_response.roots {
            for root in roots {
                if let Some(root_id) = &root.id {
                    // List OUs for this root
                    let mut paginator = client
                        .list_organizational_units_for_parent()
                        .parent_id(root_id)
                        .into_paginator()
                        .send();

                    while let Some(page) = paginator.next().await {
                        let page = page?;
                        if let Some(ous) = page.organizational_units {
                            for ou in ous {
                                let ou_json = self.organizational_unit_to_json(&ou);
                                organizational_units.push(ou_json);
                            }
                        }
                    }

                    // Recursively get OUs under each found OU
                    let current_ous = organizational_units.clone();
                    for ou_json in &current_ous {
                        if let Some(ou_id) = ou_json.get("Id").and_then(|v| v.as_str()) {
                            self.list_ous_recursively(&client, ou_id, &mut organizational_units)
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(organizational_units)
    }

    /// Recursively list OUs under a parent
    fn list_ous_recursively<'a>(
        &'a self,
        client: &'a organizations::Client,
        parent_id: &'a str,
        organizational_units: &'a mut Vec<serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a + Send>> {
        Box::pin(async move {
            let mut paginator = client
                .list_organizational_units_for_parent()
                .parent_id(parent_id)
                .into_paginator()
                .send();

            let mut child_ous = Vec::new();
            while let Some(page) = paginator.next().await {
                let page = page?;
                if let Some(ous) = page.organizational_units {
                    for ou in ous {
                        let ou_json = self.organizational_unit_to_json(&ou);
                        child_ous.push(ou_json.clone());
                        organizational_units.push(ou_json);
                    }
                }
            }

            // Recursively process child OUs
            for child_ou in &child_ous {
                if let Some(child_id) = child_ou.get("Id").and_then(|v| v.as_str()) {
                    self.list_ous_recursively(client, child_id, organizational_units)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Get detailed information for specific Organizational Unit
    pub async fn describe_organizational_unit(
        &self,
        account_id: &str,
        region: &str,
        ou_id: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client
            .describe_organizational_unit()
            .organizational_unit_id(ou_id)
            .send()
            .await?;

        if let Some(ou) = response.organizational_unit {
            Ok(self.organizational_unit_details_to_json(&ou))
        } else {
            Err(anyhow::anyhow!("Organizational Unit {} not found", ou_id))
        }
    }

    /// List AWS Organizations Service Control Policies
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

        let client = organizations::Client::new(&aws_config);

        // List Service Control Policies
        let mut paginator = client
            .list_policies()
            .filter(organizations::types::PolicyType::ServiceControlPolicy)
            .into_paginator()
            .send();

        let mut policies = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(policy_summaries) = page.policies {
                for policy in policy_summaries {
                    let policy_json = self.policy_summary_to_json(&policy);
                    policies.push(policy_json);
                }
            }
        }

        Ok(policies)
    }

    /// Get detailed information for specific Service Control Policy
    pub async fn describe_policy(
        &self,
        account_id: &str,
        region: &str,
        policy_id: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client.describe_policy().policy_id(policy_id).send().await?;

        if let Some(policy) = response.policy {
            Ok(self.policy_details_to_json(&policy))
        } else {
            Err(anyhow::anyhow!("Policy {} not found", policy_id))
        }
    }

    // ==================== Delegation & Administration ====================

    /// List delegated administrator accounts
    pub async fn list_delegated_administrators(
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

        let client = organizations::Client::new(&aws_config);

        let mut delegated_admins = Vec::new();
        let mut paginator = client
            .list_delegated_administrators()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(delegated_administrators) = page.delegated_administrators {
                for admin in delegated_administrators {
                    let admin_json = self.delegated_administrator_to_json(&admin);
                    delegated_admins.push(admin_json);
                }
            }
        }

        Ok(delegated_admins)
    }

    /// List AWS services delegated to a specific account
    pub async fn list_delegated_services_for_account(
        &self,
        account_id: &str,
        region: &str,
        delegated_account_id: &str,
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

        let client = organizations::Client::new(&aws_config);

        let mut services = Vec::new();
        let mut paginator = client
            .list_delegated_services_for_account()
            .account_id(delegated_account_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(delegated_services) = page.delegated_services {
                for service in delegated_services {
                    let mut service_json = serde_json::Map::new();

                    if let Some(service_principal) = &service.service_principal {
                        service_json.insert(
                            "ServicePrincipal".to_string(),
                            serde_json::Value::String(service_principal.clone()),
                        );
                    }

                    if let Some(delegation_enabled_date) = &service.delegation_enabled_date {
                        service_json.insert(
                            "DelegationEnabledDate".to_string(),
                            serde_json::Value::String(
                                delegation_enabled_date
                                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default(),
                            ),
                        );
                    }

                    services.push(serde_json::Value::Object(service_json));
                }
            }
        }

        Ok(services)
    }

    /// List handshakes for the organization
    pub async fn list_handshakes_for_organization(
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

        let client = organizations::Client::new(&aws_config);

        let mut handshakes = Vec::new();
        let mut paginator = client
            .list_handshakes_for_organization()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(handshake_summaries) = page.handshakes {
                for handshake in handshake_summaries {
                    let handshake_json = self.handshake_to_json(&handshake);
                    handshakes.push(handshake_json);
                }
            }
        }

        Ok(handshakes)
    }

    /// Get detailed information for specific Handshake
    pub async fn describe_handshake(
        &self,
        account_id: &str,
        region: &str,
        handshake_id: &str,
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

        let client = organizations::Client::new(&aws_config);
        let response = client
            .describe_handshake()
            .handshake_id(handshake_id)
            .send()
            .await?;

        if let Some(handshake) = response.handshake {
            Ok(self.handshake_details_to_json(&handshake))
        } else {
            Err(anyhow::anyhow!("Handshake {} not found", handshake_id))
        }
    }

    // JSON conversion methods

    fn account_to_json(&self, account: &organizations::types::Account) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &account.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(account.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &account.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(email) = &account.email {
            json.insert(
                "Email".to_string(),
                serde_json::Value::String(email.clone()),
            );
        }

        if let Some(arn) = &account.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(status) = &account.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(joined_method) = &account.joined_method {
            json.insert(
                "JoinedMethod".to_string(),
                serde_json::Value::String(joined_method.as_str().to_string()),
            );
        }

        if let Some(joined_timestamp) = &account.joined_timestamp {
            json.insert(
                "JoinedTimestamp".to_string(),
                serde_json::Value::String(
                    joined_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }

    fn account_details_to_json(
        &self,
        account: &organizations::types::Account,
    ) -> serde_json::Value {
        // For accounts, list and describe return the same structure
        self.account_to_json(account)
    }

    fn root_to_json(&self, root: &organizations::types::Root) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &root.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(root.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &root.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &root.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        // Policy types enabled for this root
        if let Some(policy_types) = &root.policy_types {
            let policy_type_strings: Vec<serde_json::Value> = policy_types
                .iter()
                .filter_map(|pt| pt.r#type.as_ref())
                .map(|t| serde_json::Value::String(t.as_str().to_string()))
                .collect();
            json.insert(
                "PolicyTypes".to_string(),
                serde_json::Value::Array(policy_type_strings),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn organization_to_json(&self, org: &organizations::types::Organization) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &org.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert("Name".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(arn) = &org.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(feature_set) = &org.feature_set {
            json.insert(
                "FeatureSet".to_string(),
                serde_json::Value::String(feature_set.as_str().to_string()),
            );
        }

        if let Some(master_account_arn) = &org.master_account_arn {
            json.insert(
                "MasterAccountArn".to_string(),
                serde_json::Value::String(master_account_arn.clone()),
            );
        }

        if let Some(master_account_id) = &org.master_account_id {
            json.insert(
                "MasterAccountId".to_string(),
                serde_json::Value::String(master_account_id.clone()),
            );
        }

        if let Some(master_account_email) = &org.master_account_email {
            json.insert(
                "MasterAccountEmail".to_string(),
                serde_json::Value::String(master_account_email.clone()),
            );
        }

        // Available policy types
        if let Some(available_policy_types) = &org.available_policy_types {
            let policy_type_strings: Vec<serde_json::Value> = available_policy_types
                .iter()
                .filter_map(|pt| pt.r#type.as_ref())
                .map(|t| serde_json::Value::String(t.as_str().to_string()))
                .collect();
            json.insert(
                "AvailablePolicyTypes".to_string(),
                serde_json::Value::Array(policy_type_strings),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn delegated_administrator_to_json(
        &self,
        admin: &organizations::types::DelegatedAdministrator,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &admin.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(admin.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &admin.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(email) = &admin.email {
            json.insert(
                "Email".to_string(),
                serde_json::Value::String(email.clone()),
            );
        }

        if let Some(arn) = &admin.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(status) = &admin.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(delegation_enabled_date) = &admin.delegation_enabled_date {
            json.insert(
                "DelegationEnabledDate".to_string(),
                serde_json::Value::String(
                    delegation_enabled_date
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        if let Some(joined_method) = &admin.joined_method {
            json.insert(
                "JoinedMethod".to_string(),
                serde_json::Value::String(joined_method.as_str().to_string()),
            );
        }

        if let Some(joined_timestamp) = &admin.joined_timestamp {
            json.insert(
                "JoinedTimestamp".to_string(),
                serde_json::Value::String(
                    joined_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }

    fn handshake_to_json(&self, handshake: &organizations::types::Handshake) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &handshake.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert("Name".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(arn) = &handshake.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(state) = &handshake.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            // Use state as status
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(action) = &handshake.action {
            json.insert(
                "Action".to_string(),
                serde_json::Value::String(action.as_str().to_string()),
            );
        }

        if let Some(requested_timestamp) = &handshake.requested_timestamp {
            json.insert(
                "RequestedTimestamp".to_string(),
                serde_json::Value::String(
                    requested_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        if let Some(expiration_timestamp) = &handshake.expiration_timestamp {
            json.insert(
                "ExpirationTimestamp".to_string(),
                serde_json::Value::String(
                    expiration_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }

    fn handshake_details_to_json(
        &self,
        handshake: &organizations::types::Handshake,
    ) -> serde_json::Value {
        // For handshakes, list and describe return the same structure
        self.handshake_to_json(handshake)
    }

    fn organizational_unit_to_json(
        &self,
        ou: &organizations::types::OrganizationalUnit,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &ou.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(ou.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &ou.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &ou.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn organizational_unit_details_to_json(
        &self,
        ou: &organizations::types::OrganizationalUnit,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &ou.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(ou.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &ou.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &ou.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn policy_summary_to_json(
        &self,
        policy: &organizations::types::PolicySummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &policy.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(policy.name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(name) = &policy.name {
            json.insert(
                "DisplayName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &policy.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(policy_type) = &policy.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(policy_type.as_str().to_string()),
            );
        }

        if let Some(description) = &policy.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "AwsManaged".to_string(),
            serde_json::Value::Bool(policy.aws_managed),
        );

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn policy_details_to_json(&self, policy: &organizations::types::Policy) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(policy_summary) = &policy.policy_summary {
            if let Some(id) = &policy_summary.id {
                json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(
                        policy_summary.name.as_deref().unwrap_or(id).to_string(),
                    ),
                );
            }

            if let Some(name) = &policy_summary.name {
                json.insert(
                    "DisplayName".to_string(),
                    serde_json::Value::String(name.clone()),
                );
            }

            if let Some(arn) = &policy_summary.arn {
                json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
            }

            if let Some(policy_type) = &policy_summary.r#type {
                json.insert(
                    "Type".to_string(),
                    serde_json::Value::String(policy_type.as_str().to_string()),
                );
            }

            if let Some(description) = &policy_summary.description {
                json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description.clone()),
                );
            }

            json.insert(
                "AwsManaged".to_string(),
                serde_json::Value::Bool(policy_summary.aws_managed),
            );
        }

        if let Some(content) = &policy.content {
            json.insert(
                "Content".to_string(),
                serde_json::Value::String(content.clone()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List account creation requests and their status
    pub async fn list_create_account_status(
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

        let client = organizations::Client::new(&aws_config);

        let mut results = Vec::new();
        let mut paginator = client.list_create_account_status().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page.context("Failed to list create account status")?;
            if let Some(statuses) = page.create_account_statuses {
                for status in statuses {
                    results.push(self.create_account_status_to_json(&status));
                }
            }
        }

        Ok(results)
    }

    /// Describe a specific account creation request
    pub async fn describe_create_account_status(
        &self,
        account_id: &str,
        region: &str,
        request_id: &str,
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

        let client = organizations::Client::new(&aws_config);

        let response = client
            .describe_create_account_status()
            .create_account_request_id(request_id)
            .send()
            .await
            .context("Failed to describe create account status")?;

        if let Some(status) = response.create_account_status {
            Ok(self.create_account_status_details_to_json(&status))
        } else {
            anyhow::bail!(
                "No create account status found for request ID: {}",
                request_id
            )
        }
    }

    /// List AWS services that have access to the organization
    pub async fn list_aws_service_access_for_organization(
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

        let client = organizations::Client::new(&aws_config);

        let mut results = Vec::new();
        let mut paginator = client
            .list_aws_service_access_for_organization()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page.context("Failed to list AWS service access")?;
            if let Some(services) = page.enabled_service_principals {
                for service in services {
                    results.push(self.aws_service_access_to_json(&service));
                }
            }
        }

        Ok(results)
    }

    // JSON Converters for CreateAccountStatus
    fn create_account_status_to_json(
        &self,
        status: &organizations::types::CreateAccountStatus,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &status.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(status.account_name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(account_name) = &status.account_name {
            json.insert(
                "AccountName".to_string(),
                serde_json::Value::String(account_name.clone()),
            );
        }

        if let Some(state) = &status.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(requested_timestamp) = &status.requested_timestamp {
            json.insert(
                "RequestedTimestamp".to_string(),
                serde_json::Value::String(
                    requested_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        if let Some(completed_timestamp) = &status.completed_timestamp {
            json.insert(
                "CompletedTimestamp".to_string(),
                serde_json::Value::String(
                    completed_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        if let Some(account_id) = &status.account_id {
            json.insert(
                "AccountId".to_string(),
                serde_json::Value::String(account_id.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn create_account_status_details_to_json(
        &self,
        status: &organizations::types::CreateAccountStatus,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &status.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(status.account_name.as_deref().unwrap_or(id).to_string()),
            );
        }

        if let Some(account_name) = &status.account_name {
            json.insert(
                "AccountName".to_string(),
                serde_json::Value::String(account_name.clone()),
            );
        }

        if let Some(state) = &status.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(requested_timestamp) = &status.requested_timestamp {
            json.insert(
                "RequestedTimestamp".to_string(),
                serde_json::Value::String(
                    requested_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        if let Some(completed_timestamp) = &status.completed_timestamp {
            json.insert(
                "CompletedTimestamp".to_string(),
                serde_json::Value::String(
                    completed_timestamp
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        if let Some(account_id) = &status.account_id {
            json.insert(
                "AccountId".to_string(),
                serde_json::Value::String(account_id.clone()),
            );
        }

        if let Some(gov_cloud_account_id) = &status.gov_cloud_account_id {
            json.insert(
                "GovCloudAccountId".to_string(),
                serde_json::Value::String(gov_cloud_account_id.clone()),
            );
        }

        if let Some(failure_reason) = &status.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(failure_reason.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    // JSON Converter for AwsServiceAccess
    fn aws_service_access_to_json(
        &self,
        service: &organizations::types::EnabledServicePrincipal,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(service_principal) = &service.service_principal {
            json.insert(
                "Id".to_string(),
                serde_json::Value::String(service_principal.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(service_principal.clone()),
            );
            json.insert(
                "ServicePrincipal".to_string(),
                serde_json::Value::String(service_principal.clone()),
            );
        }

        if let Some(date_enabled) = &service.date_enabled {
            json.insert(
                "DateEnabled".to_string(),
                serde_json::Value::String(
                    date_enabled
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ENABLED".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
