#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Result, Context};
use aws_sdk_codeartifact as codeartifact;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct CodeArtifactService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CodeArtifactService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CodeArtifact domains
    pub async fn list_domains(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = codeartifact::Client::new(&aws_config);
        let mut domains = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_domains();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(domains_list) = response.domains {
                for domain in domains_list {
                    let domain_json = self.domain_to_json(&domain);
                    domains.push(domain_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(domains)
    }

    /// List CodeArtifact repositories
    pub async fn list_repositories(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = codeartifact::Client::new(&aws_config);
        let mut repositories = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_repositories();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(repositories_list) = response.repositories {
                for repository in repositories_list {
                    let repository_json = self.repository_to_json(&repository);
                    repositories.push(repository_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(repositories)
    }

    /// Describe CodeArtifact domain
    pub async fn describe_domain(
        &self,
        account: &str,
        region: &str,
        domain_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account, region))?;

        let client = codeartifact::Client::new(&aws_config);

        let response = client
            .describe_domain()
            .domain(domain_name)
            .send()
            .await
            .with_context(|| format!("Failed to describe CodeArtifact domain: {}", domain_name))?;

        if let Some(domain) = response.domain {
            Ok(self.domain_detail_to_json(&domain))
        } else {
            Err(anyhow::anyhow!("Domain not found: {}", domain_name))
        }
    }

    /// Describe CodeArtifact repository
    pub async fn describe_repository(
        &self,
        account: &str,
        region: &str,
        repository_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account, region))?;

        let client = codeartifact::Client::new(&aws_config);

        // We need the domain for describe_repository - try to extract from repository name or list all
        let repositories = self.list_repositories(account, region).await?;
        
        for repo in repositories {
            if let Some(name) = repo.get("Name").and_then(|v| v.as_str()) {
                if name == repository_name {
                    if let Some(domain) = repo.get("DomainName").and_then(|v| v.as_str()) {
                        let response = client
                            .describe_repository()
                            .domain(domain)
                            .repository(repository_name)
                            .send()
                            .await
                            .with_context(|| format!("Failed to describe CodeArtifact repository: {}", repository_name))?;

                        if let Some(repository) = response.repository {
                            return Ok(self.repository_detail_to_json(&repository));
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Repository not found: {}", repository_name))
    }

    fn domain_to_json(&self, domain: &codeartifact::types::DomainSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("DomainName".to_string(), serde_json::Value::String(domain.name.clone().unwrap_or_default()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(domain.name.clone().unwrap_or_default()));
        json.insert("Name".to_string(), serde_json::Value::String(domain.name.clone().unwrap_or_default()));

        if let Some(owner) = &domain.owner {
            json.insert("Owner".to_string(), serde_json::Value::String(owner.clone()));
        }

        if let Some(arn) = &domain.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(status) = &domain.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        if let Some(created_time) = domain.created_time {
            json.insert("CreatedTime".to_string(), serde_json::Value::String(created_time.to_string()));
        }

        if let Some(encryption_key) = &domain.encryption_key {
            json.insert("EncryptionKey".to_string(), serde_json::Value::String(encryption_key.clone()));
        }

        serde_json::Value::Object(json)
    }

    fn repository_to_json(&self, repository: &codeartifact::types::RepositorySummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("RepositoryName".to_string(), serde_json::Value::String(repository.name.clone().unwrap_or_default()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(repository.name.clone().unwrap_or_default()));
        json.insert("Name".to_string(), serde_json::Value::String(repository.name.clone().unwrap_or_default()));

        if let Some(domain_name) = &repository.domain_name {
            json.insert("DomainName".to_string(), serde_json::Value::String(domain_name.clone()));
        }

        if let Some(domain_owner) = &repository.domain_owner {
            json.insert("DomainOwner".to_string(), serde_json::Value::String(domain_owner.clone()));
        }

        if let Some(arn) = &repository.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(description) = &repository.description {
            json.insert("Description".to_string(), serde_json::Value::String(description.clone()));
        }

        if let Some(created_time) = repository.created_time {
            json.insert("CreatedTime".to_string(), serde_json::Value::String(created_time.to_string()));
        }

        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        serde_json::Value::Object(json)
    }

    fn domain_detail_to_json(&self, domain: &codeartifact::types::DomainDescription) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("DomainName".to_string(), serde_json::Value::String(domain.name.clone().unwrap_or_default()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(domain.name.clone().unwrap_or_default()));
        json.insert("Name".to_string(), serde_json::Value::String(domain.name.clone().unwrap_or_default()));

        if let Some(owner) = &domain.owner {
            json.insert("Owner".to_string(), serde_json::Value::String(owner.clone()));
        }

        if let Some(arn) = &domain.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(status) = &domain.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        if let Some(created_time) = domain.created_time {
            json.insert("CreatedTime".to_string(), serde_json::Value::String(created_time.to_string()));
        }

        if let Some(encryption_key) = &domain.encryption_key {
            json.insert("EncryptionKey".to_string(), serde_json::Value::String(encryption_key.clone()));
        }

        json.insert("RepositoryCount".to_string(), serde_json::Value::Number(serde_json::Number::from(domain.repository_count)));

        json.insert("AssetSizeBytes".to_string(), serde_json::Value::Number(serde_json::Number::from(domain.asset_size_bytes)));

        if let Some(s3_bucket_arn) = &domain.s3_bucket_arn {
            json.insert("S3BucketArn".to_string(), serde_json::Value::String(s3_bucket_arn.clone()));
        }

        serde_json::Value::Object(json)
    }

    fn repository_detail_to_json(&self, repository: &codeartifact::types::RepositoryDescription) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("RepositoryName".to_string(), serde_json::Value::String(repository.name.clone().unwrap_or_default()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(repository.name.clone().unwrap_or_default()));
        json.insert("Name".to_string(), serde_json::Value::String(repository.name.clone().unwrap_or_default()));

        if let Some(domain_name) = &repository.domain_name {
            json.insert("DomainName".to_string(), serde_json::Value::String(domain_name.clone()));
        }

        if let Some(domain_owner) = &repository.domain_owner {
            json.insert("DomainOwner".to_string(), serde_json::Value::String(domain_owner.clone()));
        }

        if let Some(arn) = &repository.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(description) = &repository.description {
            json.insert("Description".to_string(), serde_json::Value::String(description.clone()));
        }

        if let Some(created_time) = repository.created_time {
            json.insert("CreatedTime".to_string(), serde_json::Value::String(created_time.to_string()));
        }

        if let Some(_upstreams) = &repository.upstreams {
            json.insert("Upstreams".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_external_connections) = &repository.external_connections {
            json.insert("ExternalConnections".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        serde_json::Value::Object(json)
    }
}