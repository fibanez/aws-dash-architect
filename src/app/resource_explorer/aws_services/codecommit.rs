use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_codecommit as codecommit;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct CodeCommitService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CodeCommitService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CodeCommit repositories with optional detailed information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic repository info quickly.
    ///   If true (Phase 2), includes triggers, branches, etc.
    pub async fn list_repositories(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("CodeCommit", "list_repositories", Some(region));

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

        let client = codecommit::Client::new(&aws_config);
        let response = client.list_repositories().send().await?;

        let mut repositories = Vec::new();
        if let Some(repos_metadata) = response.repositories {
            for repo_metadata in repos_metadata {
                // Get detailed information for each repository
                if let Some(repo_name) = &repo_metadata.repository_name {
                    // Only fetch details if requested (Phase 2)
                    let mut repo_details = if include_details {
                        report_status("CodeCommit", "get_repository", Some(repo_name));
                        match self.get_repository_internal(&client, repo_name).await {
                            Ok(details) => details,
                            Err(e) => {
                                tracing::debug!(
                                    "Could not get repository details for {}: {}",
                                    repo_name,
                                    e
                                );
                                // Fallback to basic info from repository name ID pair
                                let mut basic_repo = serde_json::Map::new();
                                basic_repo.insert(
                                    "RepositoryName".to_string(),
                                    serde_json::Value::String(repo_name.clone()),
                                );
                                basic_repo.insert(
                                    "Name".to_string(),
                                    serde_json::Value::String(repo_name.clone()),
                                );
                                if let Some(repo_id) = &repo_metadata.repository_id {
                                    basic_repo.insert(
                                        "RepositoryId".to_string(),
                                        serde_json::Value::String(repo_id.clone()),
                                    );
                                }
                                basic_repo.insert(
                                    "Status".to_string(),
                                    serde_json::Value::String("Active".to_string()),
                                );
                                serde_json::Value::Object(basic_repo)
                            }
                        }
                    } else {
                        // Phase 1: basic repository info only
                        let mut basic_repo = serde_json::Map::new();
                        basic_repo.insert(
                            "RepositoryName".to_string(),
                            serde_json::Value::String(repo_name.clone()),
                        );
                        basic_repo.insert(
                            "Name".to_string(),
                            serde_json::Value::String(repo_name.clone()),
                        );
                        if let Some(repo_id) = &repo_metadata.repository_id {
                            basic_repo.insert(
                                "RepositoryId".to_string(),
                                serde_json::Value::String(repo_id.clone()),
                            );
                        }
                        basic_repo.insert(
                            "Status".to_string(),
                            serde_json::Value::String("Active".to_string()),
                        );
                        serde_json::Value::Object(basic_repo)
                    };

                    // Add additional details only if requested
                    if include_details {
                        if let serde_json::Value::Object(ref mut details) = repo_details {
                            // Get repository triggers
                            report_status("CodeCommit", "get_repository_triggers", Some(repo_name));
                            match self
                                .get_repository_triggers_internal(&client, repo_name)
                                .await
                            {
                                Ok(triggers) => {
                                    details.insert("Triggers".to_string(), triggers);
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Could not get triggers for {}: {}",
                                        repo_name,
                                        e
                                    );
                                }
                            }

                            // List branches
                            report_status("CodeCommit", "list_branches", Some(repo_name));
                            match self.list_branches_internal(&client, repo_name).await {
                                Ok(branches) => {
                                    details.insert("Branches".to_string(), branches);
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Could not get branches for {}: {}",
                                        repo_name,
                                        e
                                    );
                                }
                            }
                        }
                    }

                    repositories.push(repo_details);
                }
            }
        }

        report_status_done("CodeCommit", "list_repositories", Some(region));
        Ok(repositories)
    }

    /// Get detailed information for specific CodeCommit repository
    pub async fn describe_repository(
        &self,
        account_id: &str,
        region: &str,
        repository_name: &str,
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

        let client = codecommit::Client::new(&aws_config);
        self.get_repository_internal(&client, repository_name).await
    }

    async fn get_repository_internal(
        &self,
        client: &codecommit::Client,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_repository()
                .repository_name(repository_name)
                .send(),
        )
        .await
        .with_context(|| "get_repository timed out")?
        .with_context(|| format!("Failed to get repository {}", repository_name))?;

        if let Some(repository_metadata) = response.repository_metadata {
            return Ok(self.repository_metadata_to_json(&repository_metadata));
        }

        Err(anyhow::anyhow!("Repository {} not found", repository_name))
    }

    // Internal function to get repository triggers
    async fn get_repository_triggers_internal(
        &self,
        client: &codecommit::Client,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_repository_triggers()
                .repository_name(repository_name)
                .send(),
        )
        .await
        .with_context(|| "get_repository_triggers timed out")?;

        match response {
            Ok(result) => {
                let mut triggers = Vec::new();

                if let Some(trigger_list) = result.triggers {
                    for trigger in trigger_list {
                        let mut trigger_json = serde_json::Map::new();

                        trigger_json
                            .insert("Name".to_string(), serde_json::Value::String(trigger.name));
                        trigger_json.insert(
                            "DestinationArn".to_string(),
                            serde_json::Value::String(trigger.destination_arn),
                        );

                        // Events
                        let events: Vec<serde_json::Value> = trigger
                            .events
                            .iter()
                            .map(|e| serde_json::Value::String(format!("{:?}", e)))
                            .collect();
                        trigger_json.insert("Events".to_string(), serde_json::Value::Array(events));

                        // Branches (if any)
                        if let Some(branches) = trigger.branches {
                            let branches_json: Vec<serde_json::Value> = branches
                                .iter()
                                .map(|b| serde_json::Value::String(b.clone()))
                                .collect();
                            trigger_json.insert(
                                "Branches".to_string(),
                                serde_json::Value::Array(branches_json),
                            );
                        }

                        if let Some(custom_data) = trigger.custom_data {
                            trigger_json.insert(
                                "CustomData".to_string(),
                                serde_json::Value::String(custom_data),
                            );
                        }

                        triggers.push(serde_json::Value::Object(trigger_json));
                    }
                }

                Ok(serde_json::json!({
                    "Items": triggers,
                    "Count": triggers.len()
                }))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("RepositoryDoesNotExistException")
                    || error_str.contains("AccessDenied")
                {
                    Ok(serde_json::json!({
                        "Items": [],
                        "Note": "No triggers configured or access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get repository triggers: {}", e))
                }
            }
        }
    }

    // Internal function to list branches
    async fn list_branches_internal(
        &self,
        client: &codecommit::Client,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_branches()
                .repository_name(repository_name)
                .send(),
        )
        .await
        .with_context(|| "list_branches timed out")?;

        match response {
            Ok(result) => {
                let branches: Vec<serde_json::Value> = result
                    .branches
                    .unwrap_or_default()
                    .iter()
                    .map(|b| serde_json::Value::String(b.clone()))
                    .collect();

                Ok(serde_json::json!({
                    "Items": branches,
                    "Count": branches.len()
                }))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("RepositoryDoesNotExistException")
                    || error_str.contains("AccessDenied")
                {
                    Ok(serde_json::json!({
                        "Items": [],
                        "Note": "Repository not found or access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to list branches: {}", e))
                }
            }
        }
    }

    // Internal function to get branch details
    async fn get_branch_internal(
        &self,
        client: &codecommit::Client,
        repository_name: &str,
        branch_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_branch()
                .repository_name(repository_name)
                .branch_name(branch_name)
                .send(),
        )
        .await
        .with_context(|| "get_branch timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();

                if let Some(branch) = result.branch {
                    if let Some(name) = branch.branch_name {
                        json.insert("BranchName".to_string(), serde_json::Value::String(name));
                    }
                    if let Some(commit_id) = branch.commit_id {
                        json.insert("CommitId".to_string(), serde_json::Value::String(commit_id));
                    }
                }

                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("BranchDoesNotExistException")
                    || error_str.contains("AccessDenied")
                {
                    Ok(serde_json::json!({
                        "Note": "Branch not found or access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get branch: {}", e))
                }
            }
        }
    }

    /// Public function to get repository triggers
    pub async fn get_repository_triggers(
        &self,
        account_id: &str,
        region: &str,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        report_status(
            "CodeCommit",
            "get_repository_triggers",
            Some(repository_name),
        );

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = codecommit::Client::new(&aws_config);
        let result = self
            .get_repository_triggers_internal(&client, repository_name)
            .await;

        report_status_done(
            "CodeCommit",
            "get_repository_triggers",
            Some(repository_name),
        );
        result
    }

    /// Public function to list branches
    pub async fn list_branches(
        &self,
        account_id: &str,
        region: &str,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("CodeCommit", "list_branches", Some(repository_name));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = codecommit::Client::new(&aws_config);
        let result = self.list_branches_internal(&client, repository_name).await;

        report_status_done("CodeCommit", "list_branches", Some(repository_name));
        result
    }

    /// Public function to get branch details
    pub async fn get_branch(
        &self,
        account_id: &str,
        region: &str,
        repository_name: &str,
        branch_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("CodeCommit", "get_branch", Some(branch_name));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = codecommit::Client::new(&aws_config);
        let result = self
            .get_branch_internal(&client, repository_name, branch_name)
            .await;

        report_status_done("CodeCommit", "get_branch", Some(branch_name));
        result
    }

    fn repository_metadata_to_json(
        &self,
        repo: &codecommit::types::RepositoryMetadata,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(repository_name) = &repo.repository_name {
            json.insert(
                "RepositoryName".to_string(),
                serde_json::Value::String(repository_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(repository_name.clone()),
            );
        }

        if let Some(repository_id) = &repo.repository_id {
            json.insert(
                "RepositoryId".to_string(),
                serde_json::Value::String(repository_id.clone()),
            );
        }

        if let Some(repository_description) = &repo.repository_description {
            json.insert(
                "RepositoryDescription".to_string(),
                serde_json::Value::String(repository_description.clone()),
            );
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(repository_description.clone()),
            );
        }

        if let Some(default_branch) = &repo.default_branch {
            json.insert(
                "DefaultBranch".to_string(),
                serde_json::Value::String(default_branch.clone()),
            );
        }

        if let Some(last_modified_date) = repo.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(creation_date) = repo.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(clone_url_http) = &repo.clone_url_http {
            json.insert(
                "CloneUrlHttp".to_string(),
                serde_json::Value::String(clone_url_http.clone()),
            );
        }

        if let Some(clone_url_ssh) = &repo.clone_url_ssh {
            json.insert(
                "CloneUrlSsh".to_string(),
                serde_json::Value::String(clone_url_ssh.clone()),
            );
        }

        if let Some(arn) = &repo.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(account_id) = &repo.account_id {
            json.insert(
                "AccountId".to_string(),
                serde_json::Value::String(account_id.clone()),
            );
        }

        // Set status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// Get details for a specific CodeCommit repository (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_repository_details(
        &self,
        account_id: &str,
        region: &str,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        report_status(
            "CodeCommit",
            "get_repository_details",
            Some(repository_name),
        );
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = codecommit::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Get detailed repository information
        if let Ok(serde_json::Value::Object(repo_map)) =
            self.get_repository_internal(&client, repository_name).await
        {
            for (key, value) in repo_map {
                details.insert(key, value);
            }
        }

        // Get repository triggers
        if let Ok(triggers) = self
            .get_repository_triggers_internal(&client, repository_name)
            .await
        {
            details.insert("Triggers".to_string(), triggers);
        }

        // List branches
        if let Ok(branches) = self.list_branches_internal(&client, repository_name).await {
            details.insert("Branches".to_string(), branches);
        }

        report_status_done(
            "CodeCommit",
            "get_repository_details",
            Some(repository_name),
        );
        Ok(serde_json::Value::Object(details))
    }
}
