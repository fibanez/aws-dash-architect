use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_docdb as docdb;
use std::sync::Arc;
use tracing::warn;

pub struct DocumentDbService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl DocumentDbService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List DocumentDB clusters
    pub async fn list_clusters(
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

        let client = docdb::Client::new(&aws_config);

        let mut resources = Vec::new();

        // Try to describe clusters
        match client.describe_db_clusters().send().await {
            Ok(response) => {
                if let Some(clusters) = response.db_clusters {
                    for cluster in clusters {
                        let mut json = serde_json::Map::new();

                        if let Some(cluster_id) = &cluster.db_cluster_identifier {
                            json.insert(
                                "ResourceId".to_string(),
                                serde_json::Value::String(cluster_id.clone()),
                            );
                            json.insert(
                                "Id".to_string(),
                                serde_json::Value::String(cluster_id.clone()),
                            );
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(cluster_id.clone()),
                            );
                        }

                        json.insert(
                            "AccountId".to_string(),
                            serde_json::Value::String(account_id.to_string()),
                        );
                        json.insert(
                            "Service".to_string(),
                            serde_json::Value::String("Amazon DocumentDB".to_string()),
                        );
                        json.insert(
                            "Description".to_string(),
                            serde_json::Value::String(
                                "MongoDB-compatible database cluster".to_string(),
                            ),
                        );
                        json.insert(
                            "Region".to_string(),
                            serde_json::Value::String(region.to_string()),
                        );

                        if let Some(status) = &cluster.status {
                            json.insert(
                                "Status".to_string(),
                                serde_json::Value::String(status.clone()),
                            );
                        }

                        if let Some(engine) = &cluster.engine {
                            json.insert(
                                "Engine".to_string(),
                                serde_json::Value::String(engine.clone()),
                            );
                        }

                        if let Some(engine_version) = &cluster.engine_version {
                            json.insert(
                                "EngineVersion".to_string(),
                                serde_json::Value::String(engine_version.clone()),
                            );
                        }

                        resources.push(serde_json::Value::Object(json));
                    }
                } else {
                    // No clusters found but service is accessible
                    let mut json = serde_json::Map::new();
                    let resource_id = format!("documentdb-service-{}", account_id);
                    json.insert(
                        "ResourceId".to_string(),
                        serde_json::Value::String(resource_id.clone()),
                    );
                    json.insert("Id".to_string(), serde_json::Value::String(resource_id));
                    json.insert(
                        "AccountId".to_string(),
                        serde_json::Value::String(account_id.to_string()),
                    );
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String("DocumentDB Service".to_string()),
                    );
                    json.insert(
                        "Status".to_string(),
                        serde_json::Value::String("Available".to_string()),
                    );
                    json.insert(
                        "Service".to_string(),
                        serde_json::Value::String("Amazon DocumentDB".to_string()),
                    );
                    json.insert(
                        "Description".to_string(),
                        serde_json::Value::String(
                            "No clusters found, but service is available".to_string(),
                        ),
                    );
                    resources.push(serde_json::Value::Object(json));
                }
            }
            Err(e) => {
                warn!(
                    "DocumentDB not accessible for account {} in region {}: {}",
                    account_id, region, e
                );
                // Create entry indicating DocumentDB is not accessible
                let mut json = serde_json::Map::new();
                json.insert(
                    "AccountId".to_string(),
                    serde_json::Value::String(account_id.to_string()),
                );
                json.insert(
                    "ResourceId".to_string(),
                    serde_json::Value::String(format!("documentdb-{}", account_id)),
                );
                json.insert(
                    "Status".to_string(),
                    serde_json::Value::String("Unavailable".to_string()),
                );
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String("DocumentDB (Unavailable)".to_string()),
                );
                json.insert(
                    "Service".to_string(),
                    serde_json::Value::String("Amazon DocumentDB".to_string()),
                );
                json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(
                        "MongoDB-compatible database service (not accessible)".to_string(),
                    ),
                );
                resources.push(serde_json::Value::Object(json));
            }
        }

        Ok(resources)
    }

    /// Get detailed DocumentDB cluster information
    pub async fn get_cluster_details(
        &self,
        account_id: &str,
        region: &str,
        cluster_id: &str,
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

        let client = docdb::Client::new(&aws_config);
        let response = client
            .describe_db_clusters()
            .db_cluster_identifier(cluster_id)
            .send()
            .await?;

        let mut json = serde_json::Map::new();
        json.insert(
            "AccountId".to_string(),
            serde_json::Value::String(account_id.to_string()),
        );
        json.insert(
            "Service".to_string(),
            serde_json::Value::String("Amazon DocumentDB".to_string()),
        );
        json.insert(
            "Description".to_string(),
            serde_json::Value::String(
                "MongoDB-compatible database with enterprise features".to_string(),
            ),
        );
        json.insert(
            "Region".to_string(),
            serde_json::Value::String(region.to_string()),
        );
        json.insert(
            "Type".to_string(),
            serde_json::Value::String("Document Database Cluster".to_string()),
        );

        if let Some(clusters) = response.db_clusters {
            if let Some(cluster) = clusters.first() {
                if let Some(status) = &cluster.status {
                    json.insert(
                        "Status".to_string(),
                        serde_json::Value::String(status.clone()),
                    );
                }

                if let Some(engine) = &cluster.engine {
                    json.insert(
                        "Engine".to_string(),
                        serde_json::Value::String(engine.clone()),
                    );
                }

                if let Some(engine_version) = &cluster.engine_version {
                    json.insert(
                        "EngineVersion".to_string(),
                        serde_json::Value::String(engine_version.clone()),
                    );
                }

                if let Some(port) = cluster.port {
                    json.insert(
                        "Port".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(port)),
                    );
                }

                if let Some(backup_retention_period) = cluster.backup_retention_period {
                    json.insert(
                        "BackupRetentionPeriod".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            backup_retention_period,
                        )),
                    );
                }
            }
        }

        Ok(serde_json::Value::Object(json))
    }
}
