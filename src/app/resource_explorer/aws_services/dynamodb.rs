use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_dynamodb as dynamodb;
use std::sync::Arc;

pub struct DynamoDBService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl DynamoDBService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List DynamoDB tables
    pub async fn list_tables(
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

        let client = dynamodb::Client::new(&aws_config);
        let mut paginator = client.list_tables().into_paginator().send();

        let mut tables = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(table_names) = page.table_names {
                for table_name in table_names {
                    // Get detailed table information
                    if let Ok(table_details) =
                        self.describe_table_internal(&client, &table_name).await
                    {
                        tables.push(table_details);
                    } else {
                        // Fallback to basic table name if describe fails
                        let mut basic_table = serde_json::Map::new();
                        basic_table.insert(
                            "TableName".to_string(),
                            serde_json::Value::String(table_name.clone()),
                        );
                        basic_table
                            .insert("Name".to_string(), serde_json::Value::String(table_name));
                        tables.push(serde_json::Value::Object(basic_table));
                    }
                }
            }
        }

        Ok(tables)
    }

    /// Get detailed information for specific DynamoDB table
    pub async fn describe_table(
        &self,
        account_id: &str,
        region: &str,
        table_name: &str,
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

        let client = dynamodb::Client::new(&aws_config);
        self.describe_table_internal(&client, table_name).await
    }

    async fn describe_table_internal(
        &self,
        client: &dynamodb::Client,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_table()
            .table_name(table_name)
            .send()
            .await?;

        if let Some(table) = response.table {
            Ok(self.table_to_json(&table))
        } else {
            Err(anyhow::anyhow!("Table {} not found", table_name))
        }
    }

    fn table_to_json(&self, table: &dynamodb::types::TableDescription) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(table_name) = &table.table_name {
            json.insert(
                "TableName".to_string(),
                serde_json::Value::String(table_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(table_name.clone()),
            );
        }

        if let Some(table_arn) = &table.table_arn {
            json.insert(
                "TableArn".to_string(),
                serde_json::Value::String(table_arn.clone()),
            );
        }

        if let Some(table_status) = &table.table_status {
            json.insert(
                "TableStatus".to_string(),
                serde_json::Value::String(table_status.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(table_status.as_str().to_string()),
            );
        }

        if let Some(creation_date_time) = table.creation_date_time {
            json.insert(
                "CreationDateTime".to_string(),
                serde_json::Value::String(creation_date_time.to_string()),
            );
        }

        if let Some(item_count) = table.item_count {
            json.insert(
                "ItemCount".to_string(),
                serde_json::Value::Number(item_count.into()),
            );
        }

        if let Some(table_size_bytes) = table.table_size_bytes {
            json.insert(
                "TableSizeBytes".to_string(),
                serde_json::Value::Number(table_size_bytes.into()),
            );
        }

        if let Some(key_schema) = &table.key_schema {
            let keys_json: Vec<serde_json::Value> = key_schema
                .iter()
                .map(|key| {
                    let mut key_json = serde_json::Map::new();
                    key_json.insert(
                        "AttributeName".to_string(),
                        serde_json::Value::String(key.attribute_name.clone()),
                    );
                    key_json.insert(
                        "KeyType".to_string(),
                        serde_json::Value::String(key.key_type.as_str().to_string()),
                    );
                    serde_json::Value::Object(key_json)
                })
                .collect();
            json.insert("KeySchema".to_string(), serde_json::Value::Array(keys_json));
        }

        if let Some(billing_mode_summary) = &table.billing_mode_summary {
            let mut billing_json = serde_json::Map::new();
            if let Some(billing_mode) = &billing_mode_summary.billing_mode {
                billing_json.insert(
                    "BillingMode".to_string(),
                    serde_json::Value::String(billing_mode.as_str().to_string()),
                );
            }
            json.insert(
                "BillingModeSummary".to_string(),
                serde_json::Value::Object(billing_json),
            );
        }

        serde_json::Value::Object(json)
    }
}
