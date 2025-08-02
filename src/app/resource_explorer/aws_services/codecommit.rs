use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_codecommit as codecommit;
use std::sync::Arc;

pub struct CodeCommitService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CodeCommitService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CodeCommit repositories
    pub async fn list_repositories(
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

        let client = codecommit::Client::new(&aws_config);
        let response = client.list_repositories().send().await?;

        let mut repositories = Vec::new();
        if let Some(repos_metadata) = response.repositories {
            for repo_metadata in repos_metadata {
                // Get detailed information for each repository
                if let Some(repo_name) = &repo_metadata.repository_name {
                    match self.get_repository_internal(&client, repo_name).await {
                        Ok(repo_details) => repositories.push(repo_details),
                        Err(_) => {
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
                            repositories.push(serde_json::Value::Object(basic_repo));
                        }
                    }
                }
            }
        }

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
        let response = client
            .get_repository()
            .repository_name(repository_name)
            .send()
            .await?;

        if let Some(repository_metadata) = response.repository_metadata {
            return Ok(self.repository_metadata_to_json(&repository_metadata));
        }

        Err(anyhow::anyhow!("Repository {} not found", repository_name))
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
}
