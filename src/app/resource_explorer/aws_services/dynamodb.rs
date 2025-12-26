use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_dynamodb as dynamodb;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct DynamoDBService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl DynamoDBService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List DynamoDB tables with optional detailed information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic table info quickly.
    ///   If true (Phase 2), includes PITR, TTL, and tags.
    pub async fn list_tables(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("DynamoDB", "list_tables", Some(region));

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
                    // Get basic table information via describe_table
                    if let Ok(mut table_details) =
                        self.describe_table_internal(&client, &table_name).await
                    {
                        // Only fetch additional details if requested (Phase 2)
                        if include_details {
                            if let serde_json::Value::Object(ref mut details) = table_details {
                                // Get the table ARN for tags
                                let table_arn = details
                                    .get("TableArn")
                                    .and_then(|v| v.as_str())
                                    .map(String::from);

                                // Get Point-in-Time Recovery status
                                report_status(
                                    "DynamoDB",
                                    "describe_continuous_backups",
                                    Some(&table_name),
                                );
                                match self
                                    .describe_continuous_backups_internal(&client, &table_name)
                                    .await
                                {
                                    Ok(pitr) => {
                                        details.insert("ContinuousBackups".to_string(), pitr);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get continuous backups for {}: {}",
                                            table_name,
                                            e
                                        );
                                    }
                                }

                                // Get Time-to-Live configuration
                                report_status(
                                    "DynamoDB",
                                    "describe_time_to_live",
                                    Some(&table_name),
                                );
                                match self
                                    .describe_time_to_live_internal(&client, &table_name)
                                    .await
                                {
                                    Ok(ttl) => {
                                        details.insert("TimeToLive".to_string(), ttl);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get TTL for {}: {}",
                                            table_name,
                                            e
                                        );
                                    }
                                }

                                // Get tags (requires table ARN)
                                if let Some(arn) = table_arn {
                                    report_status(
                                        "DynamoDB",
                                        "list_tags_of_resource",
                                        Some(&table_name),
                                    );
                                    match self.list_tags_of_resource_internal(&client, &arn).await {
                                        Ok(tags) => {
                                            details.insert("Tags".to_string(), tags);
                                        }
                                        Err(e) => {
                                            tracing::debug!(
                                                "Could not get tags for {}: {}",
                                                table_name,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }

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

        report_status_done("DynamoDB", "list_tables", Some(region));
        Ok(tables)
    }

    /// Get detailed information for a single DynamoDB table (Phase 2 enrichment)
    ///
    /// This function fetches detailed information for a single table,
    /// including Point-in-Time Recovery, TTL, and tags.
    /// Used for incremental detail fetching after the initial fast list.
    pub async fn get_table_details(
        &self,
        account_id: &str,
        region: &str,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("DynamoDB", "get_table_details", Some(table_name));

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
        let mut details = serde_json::Map::new();

        // Get table ARN first via describe_table
        let table_arn = self
            .describe_table_internal(&client, table_name)
            .await
            .ok()
            .and_then(|table_info| {
                table_info
                    .get("TableArn")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            });

        // Get Point-in-Time Recovery status
        report_status("DynamoDB", "describe_continuous_backups", Some(table_name));
        match self
            .describe_continuous_backups_internal(&client, table_name)
            .await
        {
            Ok(pitr) => {
                details.insert("ContinuousBackups".to_string(), pitr);
            }
            Err(e) => {
                tracing::debug!("Could not get continuous backups for {}: {}", table_name, e);
            }
        }

        // Get Time-to-Live configuration
        report_status("DynamoDB", "describe_time_to_live", Some(table_name));
        match self
            .describe_time_to_live_internal(&client, table_name)
            .await
        {
            Ok(ttl) => {
                details.insert("TimeToLive".to_string(), ttl);
            }
            Err(e) => {
                tracing::debug!("Could not get TTL for {}: {}", table_name, e);
            }
        }

        // Get tags (requires table ARN)
        if let Some(arn) = table_arn {
            report_status("DynamoDB", "list_tags_of_resource", Some(table_name));
            match self.list_tags_of_resource_internal(&client, &arn).await {
                Ok(tags) => {
                    details.insert("Tags".to_string(), tags);
                }
                Err(e) => {
                    tracing::debug!("Could not get tags for {}: {}", table_name, e);
                }
            }
        }

        report_status_done("DynamoDB", "get_table_details", Some(table_name));
        Ok(serde_json::Value::Object(details))
    }

    /// Get detailed information for specific DynamoDB table
    pub async fn describe_table(
        &self,
        account_id: &str,
        region: &str,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("DynamoDB", "describe_table", Some(table_name));

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
        let result = self.describe_table_internal(&client, table_name).await;

        report_status_done("DynamoDB", "describe_table", Some(table_name));
        result
    }

    async fn describe_table_internal(
        &self,
        client: &dynamodb::Client,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.describe_table().table_name(table_name).send(),
        )
        .await
        .with_context(|| "describe_table timed out")?
        .with_context(|| format!("Failed to describe table {}", table_name))?;

        if let Some(table) = response.table {
            Ok(self.table_to_json(&table))
        } else {
            Err(anyhow::anyhow!("Table {} not found", table_name))
        }
    }

    /// Get Point-in-Time Recovery (PITR) status for a table
    pub async fn describe_continuous_backups(
        &self,
        account_id: &str,
        region: &str,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("DynamoDB", "describe_continuous_backups", Some(table_name));

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
        let result = self
            .describe_continuous_backups_internal(&client, table_name)
            .await;

        report_status_done("DynamoDB", "describe_continuous_backups", Some(table_name));
        result
    }

    async fn describe_continuous_backups_internal(
        &self,
        client: &dynamodb::Client,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_continuous_backups()
                .table_name(table_name)
                .send(),
        )
        .await
        .with_context(|| "describe_continuous_backups timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();

                if let Some(backup_desc) = result.continuous_backups_description {
                    // Continuous backups status
                    json.insert(
                        "ContinuousBackupsStatus".to_string(),
                        serde_json::Value::String(
                            backup_desc.continuous_backups_status.as_str().to_string(),
                        ),
                    );

                    // Point-in-Time Recovery description
                    if let Some(pitr_desc) = backup_desc.point_in_time_recovery_description {
                        let mut pitr_json = serde_json::Map::new();

                        if let Some(status) = pitr_desc.point_in_time_recovery_status {
                            pitr_json.insert(
                                "PointInTimeRecoveryStatus".to_string(),
                                serde_json::Value::String(status.as_str().to_string()),
                            );
                        }

                        if let Some(earliest) = pitr_desc.earliest_restorable_date_time {
                            pitr_json.insert(
                                "EarliestRestorableDateTime".to_string(),
                                serde_json::Value::String(earliest.to_string()),
                            );
                        }

                        if let Some(latest) = pitr_desc.latest_restorable_date_time {
                            pitr_json.insert(
                                "LatestRestorableDateTime".to_string(),
                                serde_json::Value::String(latest.to_string()),
                            );
                        }

                        json.insert(
                            "PointInTimeRecoveryDescription".to_string(),
                            serde_json::Value::Object(pitr_json),
                        );
                    }
                }

                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("TableNotFoundException") {
                    Ok(serde_json::json!({
                        "ContinuousBackupsStatus": null,
                        "Note": "Table not found"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get continuous backups: {}", e))
                }
            }
        }
    }

    /// Get Time-to-Live (TTL) configuration for a table
    pub async fn describe_time_to_live(
        &self,
        account_id: &str,
        region: &str,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("DynamoDB", "describe_time_to_live", Some(table_name));

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
        let result = self
            .describe_time_to_live_internal(&client, table_name)
            .await;

        report_status_done("DynamoDB", "describe_time_to_live", Some(table_name));
        result
    }

    async fn describe_time_to_live_internal(
        &self,
        client: &dynamodb::Client,
        table_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.describe_time_to_live().table_name(table_name).send(),
        )
        .await
        .with_context(|| "describe_time_to_live timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();

                if let Some(ttl_desc) = result.time_to_live_description {
                    if let Some(status) = ttl_desc.time_to_live_status {
                        json.insert(
                            "TimeToLiveStatus".to_string(),
                            serde_json::Value::String(status.as_str().to_string()),
                        );
                    }

                    if let Some(attribute_name) = ttl_desc.attribute_name {
                        json.insert(
                            "AttributeName".to_string(),
                            serde_json::Value::String(attribute_name),
                        );
                    }
                } else {
                    json.insert(
                        "TimeToLiveStatus".to_string(),
                        serde_json::Value::String("DISABLED".to_string()),
                    );
                    json.insert(
                        "Note".to_string(),
                        serde_json::Value::String("TTL not configured".to_string()),
                    );
                }

                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("ResourceNotFoundException") {
                    Ok(serde_json::json!({
                        "TimeToLiveStatus": null,
                        "Note": "Table not found"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get TTL: {}", e))
                }
            }
        }
    }

    /// Get tags for a DynamoDB table
    pub async fn list_tags_of_resource(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("DynamoDB", "list_tags_of_resource", Some(resource_arn));

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
        let result = self
            .list_tags_of_resource_internal(&client, resource_arn)
            .await;

        report_status_done("DynamoDB", "list_tags_of_resource", Some(resource_arn));
        result
    }

    async fn list_tags_of_resource_internal(
        &self,
        client: &dynamodb::Client,
        resource_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_tags_of_resource()
                .resource_arn(resource_arn)
                .send(),
        )
        .await
        .with_context(|| "list_tags_of_resource timed out")?;

        match response {
            Ok(result) => {
                let mut tags_json = serde_json::Map::new();

                if let Some(tags) = result.tags {
                    for tag in tags {
                        // DynamoDB Tag type has non-optional key and value fields
                        tags_json.insert(tag.key, serde_json::Value::String(tag.value));
                    }
                }

                if tags_json.is_empty() {
                    Ok(serde_json::json!({
                        "Tags": {},
                        "Note": "No tags configured"
                    }))
                } else {
                    Ok(serde_json::Value::Object(tags_json))
                }
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("ResourceNotFoundException") {
                    Ok(serde_json::json!({
                        "Tags": null,
                        "Note": "Resource not found"
                    }))
                } else if error_str.contains("AccessDeniedException") {
                    Ok(serde_json::json!({
                        "Tags": null,
                        "Note": "Access denied for tags"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get tags: {}", e))
                }
            }
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

        // Key Schema
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

        // Attribute Definitions
        if let Some(attribute_definitions) = &table.attribute_definitions {
            let attrs_json: Vec<serde_json::Value> = attribute_definitions
                .iter()
                .map(|attr| {
                    let mut attr_json = serde_json::Map::new();
                    attr_json.insert(
                        "AttributeName".to_string(),
                        serde_json::Value::String(attr.attribute_name.clone()),
                    );
                    attr_json.insert(
                        "AttributeType".to_string(),
                        serde_json::Value::String(attr.attribute_type.as_str().to_string()),
                    );
                    serde_json::Value::Object(attr_json)
                })
                .collect();
            json.insert(
                "AttributeDefinitions".to_string(),
                serde_json::Value::Array(attrs_json),
            );
        }

        // Billing Mode Summary
        if let Some(billing_mode_summary) = &table.billing_mode_summary {
            let mut billing_json = serde_json::Map::new();
            if let Some(billing_mode) = &billing_mode_summary.billing_mode {
                billing_json.insert(
                    "BillingMode".to_string(),
                    serde_json::Value::String(billing_mode.as_str().to_string()),
                );
            }
            if let Some(last_update) = billing_mode_summary.last_update_to_pay_per_request_date_time
            {
                billing_json.insert(
                    "LastUpdateToPayPerRequestDateTime".to_string(),
                    serde_json::Value::String(last_update.to_string()),
                );
            }
            json.insert(
                "BillingModeSummary".to_string(),
                serde_json::Value::Object(billing_json),
            );
        }

        // Provisioned Throughput
        if let Some(throughput) = &table.provisioned_throughput {
            let mut throughput_json = serde_json::Map::new();
            if let Some(read) = throughput.read_capacity_units {
                throughput_json.insert(
                    "ReadCapacityUnits".to_string(),
                    serde_json::Value::Number(read.into()),
                );
            }
            if let Some(write) = throughput.write_capacity_units {
                throughput_json.insert(
                    "WriteCapacityUnits".to_string(),
                    serde_json::Value::Number(write.into()),
                );
            }
            if !throughput_json.is_empty() {
                json.insert(
                    "ProvisionedThroughput".to_string(),
                    serde_json::Value::Object(throughput_json),
                );
            }
        }

        // Global Secondary Indexes
        if let Some(gsis) = &table.global_secondary_indexes {
            let gsi_json: Vec<serde_json::Value> = gsis
                .iter()
                .map(|gsi| {
                    let mut gsi_obj = serde_json::Map::new();
                    if let Some(name) = &gsi.index_name {
                        gsi_obj.insert(
                            "IndexName".to_string(),
                            serde_json::Value::String(name.clone()),
                        );
                    }
                    if let Some(status) = &gsi.index_status {
                        gsi_obj.insert(
                            "IndexStatus".to_string(),
                            serde_json::Value::String(status.as_str().to_string()),
                        );
                    }
                    if let Some(key_schema) = &gsi.key_schema {
                        let keys: Vec<serde_json::Value> = key_schema
                            .iter()
                            .map(|k| {
                                serde_json::json!({
                                    "AttributeName": k.attribute_name,
                                    "KeyType": k.key_type.as_str()
                                })
                            })
                            .collect();
                        gsi_obj.insert("KeySchema".to_string(), serde_json::Value::Array(keys));
                    }
                    if let Some(projection) = &gsi.projection {
                        let mut proj_json = serde_json::Map::new();
                        if let Some(proj_type) = &projection.projection_type {
                            proj_json.insert(
                                "ProjectionType".to_string(),
                                serde_json::Value::String(proj_type.as_str().to_string()),
                            );
                        }
                        if let Some(non_key_attrs) = &projection.non_key_attributes {
                            let attrs: Vec<serde_json::Value> = non_key_attrs
                                .iter()
                                .map(|a| serde_json::Value::String(a.clone()))
                                .collect();
                            proj_json.insert(
                                "NonKeyAttributes".to_string(),
                                serde_json::Value::Array(attrs),
                            );
                        }
                        gsi_obj.insert(
                            "Projection".to_string(),
                            serde_json::Value::Object(proj_json),
                        );
                    }
                    serde_json::Value::Object(gsi_obj)
                })
                .collect();
            json.insert(
                "GlobalSecondaryIndexes".to_string(),
                serde_json::Value::Array(gsi_json),
            );
        }

        // Local Secondary Indexes
        if let Some(lsis) = &table.local_secondary_indexes {
            let lsi_json: Vec<serde_json::Value> = lsis
                .iter()
                .map(|lsi| {
                    let mut lsi_obj = serde_json::Map::new();
                    if let Some(name) = &lsi.index_name {
                        lsi_obj.insert(
                            "IndexName".to_string(),
                            serde_json::Value::String(name.clone()),
                        );
                    }
                    if let Some(key_schema) = &lsi.key_schema {
                        let keys: Vec<serde_json::Value> = key_schema
                            .iter()
                            .map(|k| {
                                serde_json::json!({
                                    "AttributeName": k.attribute_name,
                                    "KeyType": k.key_type.as_str()
                                })
                            })
                            .collect();
                        lsi_obj.insert("KeySchema".to_string(), serde_json::Value::Array(keys));
                    }
                    if let Some(projection) = &lsi.projection {
                        let mut proj_json = serde_json::Map::new();
                        if let Some(proj_type) = &projection.projection_type {
                            proj_json.insert(
                                "ProjectionType".to_string(),
                                serde_json::Value::String(proj_type.as_str().to_string()),
                            );
                        }
                        lsi_obj.insert(
                            "Projection".to_string(),
                            serde_json::Value::Object(proj_json),
                        );
                    }
                    serde_json::Value::Object(lsi_obj)
                })
                .collect();
            json.insert(
                "LocalSecondaryIndexes".to_string(),
                serde_json::Value::Array(lsi_json),
            );
        }

        // SSE Description (encryption)
        if let Some(sse_desc) = &table.sse_description {
            let mut sse_json = serde_json::Map::new();
            if let Some(status) = &sse_desc.status {
                sse_json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(status.as_str().to_string()),
                );
            }
            if let Some(sse_type) = &sse_desc.sse_type {
                sse_json.insert(
                    "SSEType".to_string(),
                    serde_json::Value::String(sse_type.as_str().to_string()),
                );
            }
            if let Some(kms_key_arn) = &sse_desc.kms_master_key_arn {
                sse_json.insert(
                    "KMSMasterKeyArn".to_string(),
                    serde_json::Value::String(kms_key_arn.clone()),
                );
            }
            json.insert(
                "SSEDescription".to_string(),
                serde_json::Value::Object(sse_json),
            );
        }

        // Stream Specification
        if let Some(stream_spec) = &table.stream_specification {
            let mut stream_json = serde_json::Map::new();
            // stream_enabled is a bool, not Option<bool>
            stream_json.insert(
                "StreamEnabled".to_string(),
                serde_json::Value::Bool(stream_spec.stream_enabled),
            );
            if let Some(view_type) = &stream_spec.stream_view_type {
                stream_json.insert(
                    "StreamViewType".to_string(),
                    serde_json::Value::String(view_type.as_str().to_string()),
                );
            }
            json.insert(
                "StreamSpecification".to_string(),
                serde_json::Value::Object(stream_json),
            );
        }

        // Latest Stream ARN
        if let Some(stream_arn) = &table.latest_stream_arn {
            json.insert(
                "LatestStreamArn".to_string(),
                serde_json::Value::String(stream_arn.clone()),
            );
        }

        // Table Class
        if let Some(table_class_summary) = &table.table_class_summary {
            if let Some(table_class) = &table_class_summary.table_class {
                json.insert(
                    "TableClass".to_string(),
                    serde_json::Value::String(table_class.as_str().to_string()),
                );
            }
        }

        // Deletion Protection
        if let Some(deletion_protection) = table.deletion_protection_enabled {
            json.insert(
                "DeletionProtectionEnabled".to_string(),
                serde_json::Value::Bool(deletion_protection),
            );
        }

        serde_json::Value::Object(json)
    }
}
