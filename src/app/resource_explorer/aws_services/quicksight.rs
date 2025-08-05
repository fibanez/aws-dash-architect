use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_quicksight as quicksight;
use std::sync::Arc;

pub struct QuickSightService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl QuickSightService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List QuickSight Data Sources
    pub async fn list_data_sources(
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

        let client = quicksight::Client::new(&aws_config);

        let mut data_sources = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.list_data_sources().aws_account_id(account_id);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(data_source_list) = &response.data_sources {
                if !data_source_list.is_empty() {
                    for data_source in data_source_list {
                        // Get detailed data source information
                        if let Some(data_source_id) = &data_source.data_source_id {
                            if let Ok(data_source_details) = self
                                .describe_data_source_internal(&client, account_id, data_source_id)
                                .await
                            {
                                data_sources.push(data_source_details);
                            } else {
                                // Fallback to basic data source info if describe fails
                                let mut data_source_json = serde_json::Map::new();
                                data_source_json.insert(
                                    "DataSourceId".to_string(),
                                    serde_json::Value::String(data_source_id.clone()),
                                );
                                if let Some(name) = &data_source.name {
                                    data_source_json.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(name.clone()),
                                    );
                                }
                                if let Some(r#type) = &data_source.r#type {
                                    data_source_json.insert(
                                        "Type".to_string(),
                                        serde_json::Value::String(format!("{:?}", r#type)),
                                    );
                                }
                                data_sources.push(serde_json::Value::Object(data_source_json));
                            }
                        }
                    }
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(data_sources)
    }

    /// List QuickSight Dashboards
    pub async fn list_dashboards(
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

        let client = quicksight::Client::new(&aws_config);

        let mut dashboards = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.list_dashboards().aws_account_id(account_id);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(dashboard_list) = &response.dashboard_summary_list {
                if !dashboard_list.is_empty() {
                    for dashboard in dashboard_list {
                        // Get detailed dashboard information
                        if let Some(dashboard_id) = &dashboard.dashboard_id {
                            if let Ok(dashboard_details) = self
                                .describe_dashboard_internal(&client, account_id, dashboard_id)
                                .await
                            {
                                dashboards.push(dashboard_details);
                            } else {
                                // Fallback to basic dashboard info if describe fails
                                let mut dashboard_json = serde_json::Map::new();
                                dashboard_json.insert(
                                    "DashboardId".to_string(),
                                    serde_json::Value::String(dashboard_id.clone()),
                                );
                                if let Some(name) = &dashboard.name {
                                    dashboard_json.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(name.clone()),
                                    );
                                }
                                if let Some(arn) = &dashboard.arn {
                                    dashboard_json.insert(
                                        "Arn".to_string(),
                                        serde_json::Value::String(arn.clone()),
                                    );
                                }
                                dashboards.push(serde_json::Value::Object(dashboard_json));
                            }
                        }
                    }
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(dashboards)
    }

    /// List QuickSight Data Sets
    pub async fn list_data_sets(
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

        let client = quicksight::Client::new(&aws_config);

        let mut data_sets = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.list_data_sets().aws_account_id(account_id);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(data_set_list) = &response.data_set_summaries {
                if !data_set_list.is_empty() {
                    for data_set in data_set_list {
                        // Get detailed data set information
                        if let Some(data_set_id) = &data_set.data_set_id {
                            if let Ok(data_set_details) = self
                                .describe_data_set_internal(&client, account_id, data_set_id)
                                .await
                            {
                                data_sets.push(data_set_details);
                            } else {
                                // Fallback to basic data set info if describe fails
                                let mut data_set_json = serde_json::Map::new();
                                data_set_json.insert(
                                    "DataSetId".to_string(),
                                    serde_json::Value::String(data_set_id.clone()),
                                );
                                if let Some(name) = &data_set.name {
                                    data_set_json.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(name.clone()),
                                    );
                                }
                                if let Some(arn) = &data_set.arn {
                                    data_set_json.insert(
                                        "Arn".to_string(),
                                        serde_json::Value::String(arn.clone()),
                                    );
                                }
                                data_sets.push(serde_json::Value::Object(data_set_json));
                            }
                        }
                    }
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(data_sets)
    }

    /// Get detailed information for specific Data Source
    pub async fn describe_data_source(
        &self,
        account_id: &str,
        region: &str,
        data_source_id: &str,
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

        let client = quicksight::Client::new(&aws_config);
        self.describe_data_source_internal(&client, account_id, data_source_id)
            .await
    }

    /// Get detailed information for specific Dashboard
    pub async fn describe_dashboard(
        &self,
        account_id: &str,
        region: &str,
        dashboard_id: &str,
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

        let client = quicksight::Client::new(&aws_config);
        self.describe_dashboard_internal(&client, account_id, dashboard_id)
            .await
    }

    /// Get detailed information for specific Data Set
    pub async fn describe_data_set(
        &self,
        account_id: &str,
        region: &str,
        data_set_id: &str,
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

        let client = quicksight::Client::new(&aws_config);
        self.describe_data_set_internal(&client, account_id, data_set_id)
            .await
    }

    async fn describe_data_source_internal(
        &self,
        client: &quicksight::Client,
        account_id: &str,
        data_source_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_data_source()
            .aws_account_id(account_id)
            .data_source_id(data_source_id)
            .send()
            .await?;

        if let Some(data_source) = response.data_source {
            Ok(self.data_source_to_json(&data_source))
        } else {
            Err(anyhow::anyhow!("Data Source {} not found", data_source_id))
        }
    }

    async fn describe_dashboard_internal(
        &self,
        client: &quicksight::Client,
        account_id: &str,
        dashboard_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_dashboard()
            .aws_account_id(account_id)
            .dashboard_id(dashboard_id)
            .send()
            .await?;

        if let Some(dashboard) = response.dashboard {
            Ok(self.dashboard_to_json(&dashboard))
        } else {
            Err(anyhow::anyhow!("Dashboard {} not found", dashboard_id))
        }
    }

    async fn describe_data_set_internal(
        &self,
        client: &quicksight::Client,
        account_id: &str,
        data_set_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_data_set()
            .aws_account_id(account_id)
            .data_set_id(data_set_id)
            .send()
            .await?;

        if let Some(data_set) = response.data_set {
            Ok(self.data_set_to_json(&data_set))
        } else {
            Err(anyhow::anyhow!("Data Set {} not found", data_set_id))
        }
    }

    fn data_source_to_json(
        &self,
        data_source: &quicksight::types::DataSource,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(data_source_id) = &data_source.data_source_id {
            json.insert(
                "DataSourceId".to_string(),
                serde_json::Value::String(data_source_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(data_source_id.clone()),
            );
        }
        if let Some(name) = &data_source.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(arn) = &data_source.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(r#type) = &data_source.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(format!("{:?}", r#type)),
            );
        }

        if let Some(status) = &data_source.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(format!("{:?}", status)),
            );
        }

        if let Some(created_time) = &data_source.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(last_updated_time) = &data_source.last_updated_time {
            json.insert(
                "LastUpdatedTime".to_string(),
                serde_json::Value::String(last_updated_time.to_string()),
            );
        }

        // Data Source Parameters - simplified for AWS SDK compatibility
        if let Some(_data_source_parameters) = &data_source.data_source_parameters {
            // Note: AWS SDK field structure varies - simplified representation
            let mut params_json = serde_json::Map::new();
            params_json.insert("HasParameters".to_string(), serde_json::Value::Bool(true));
            json.insert(
                "DataSourceParameters".to_string(),
                serde_json::Value::Object(params_json),
            );
        }

        // VPC Connection Properties
        if let Some(vpc_connection_properties) = &data_source.vpc_connection_properties {
            let mut vpc_json = serde_json::Map::new();
            vpc_json.insert(
                "VpcConnectionArn".to_string(),
                serde_json::Value::String(vpc_connection_properties.vpc_connection_arn.clone()),
            );
            json.insert(
                "VpcConnectionProperties".to_string(),
                serde_json::Value::Object(vpc_json),
            );
        }

        // SSL Properties
        if let Some(ssl_properties) = &data_source.ssl_properties {
            let mut ssl_json = serde_json::Map::new();
            ssl_json.insert(
                "DisableSsl".to_string(),
                serde_json::Value::Bool(ssl_properties.disable_ssl),
            );
            json.insert(
                "SslProperties".to_string(),
                serde_json::Value::Object(ssl_json),
            );
        }

        // Error Info
        if let Some(error_info) = &data_source.error_info {
            let mut error_json = serde_json::Map::new();
            if let Some(r#type) = &error_info.r#type {
                error_json.insert(
                    "Type".to_string(),
                    serde_json::Value::String(format!("{:?}", r#type)),
                );
            }
            if let Some(message) = &error_info.message {
                error_json.insert(
                    "Message".to_string(),
                    serde_json::Value::String(message.clone()),
                );
            }
            json.insert(
                "ErrorInfo".to_string(),
                serde_json::Value::Object(error_json),
            );
        }

        serde_json::Value::Object(json)
    }

    fn dashboard_to_json(&self, dashboard: &quicksight::types::Dashboard) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(dashboard_id) = &dashboard.dashboard_id {
            json.insert(
                "DashboardId".to_string(),
                serde_json::Value::String(dashboard_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(dashboard_id.clone()),
            );
        }
        if let Some(name) = &dashboard.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(arn) = &dashboard.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(version) = &dashboard.version {
            let mut version_json = serde_json::Map::new();
            if let Some(version_number) = &version.version_number {
                version_json.insert(
                    "VersionNumber".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(*version_number)),
                );
            }

            if let Some(status) = &version.status {
                version_json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(format!("{:?}", status)),
                );
            }

            if let Some(description) = &version.description {
                version_json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description.clone()),
                );
            }

            if let Some(created_time) = &version.created_time {
                version_json.insert(
                    "CreatedTime".to_string(),
                    serde_json::Value::String(created_time.to_string()),
                );
            }

            // Data Set ARNs Referenced - Note: field structure may vary by SDK version
            // This field is commented out until we can verify the correct field name and structure
            // if let Some(data_set_arns) = &version.data_set_arns_referenced {
            //     let data_set_arns_values: Vec<serde_json::Value> = data_set_arns.iter()
            //         .map(|arn| serde_json::Value::String(arn.clone()))
            //         .collect();
            //     version_json.insert("DataSetArnsReferenced".to_string(), serde_json::Value::Array(data_set_arns_values));
            // }

            json.insert(
                "Version".to_string(),
                serde_json::Value::Object(version_json),
            );
        }

        if let Some(created_time) = &dashboard.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(last_published_time) = &dashboard.last_published_time {
            json.insert(
                "LastPublishedTime".to_string(),
                serde_json::Value::String(last_published_time.to_string()),
            );
        }

        if let Some(last_updated_time) = &dashboard.last_updated_time {
            json.insert(
                "LastUpdatedTime".to_string(),
                serde_json::Value::String(last_updated_time.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn data_set_to_json(&self, data_set: &quicksight::types::DataSet) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(data_set_id) = &data_set.data_set_id {
            json.insert(
                "DataSetId".to_string(),
                serde_json::Value::String(data_set_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(data_set_id.clone()),
            );
        }
        if let Some(name) = &data_set.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(arn) = &data_set.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(import_mode) = &data_set.import_mode {
            json.insert(
                "ImportMode".to_string(),
                serde_json::Value::String(format!("{:?}", import_mode)),
            );
        }

        if let Some(created_time) = &data_set.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(last_updated_time) = &data_set.last_updated_time {
            json.insert(
                "LastUpdatedTime".to_string(),
                serde_json::Value::String(last_updated_time.to_string()),
            );
        }

        // Physical Table Map
        if let Some(physical_table_map) = &data_set.physical_table_map {
            if !physical_table_map.is_empty() {
                let mut physical_tables_json = serde_json::Map::new();
                for table_id in physical_table_map.keys() {
                    let mut table_json = serde_json::Map::new();

                    // Note: Physical table structure simplified for SDK compatibility
                    // The exact field structure may vary by AWS SDK version
                    table_json.insert(
                        "TableType".to_string(),
                        serde_json::Value::String("PhysicalTable".to_string()),
                    );

                    physical_tables_json
                        .insert(table_id.clone(), serde_json::Value::Object(table_json));
                }
                json.insert(
                    "PhysicalTableMap".to_string(),
                    serde_json::Value::Object(physical_tables_json),
                );
            }
        }

        // Logical Table Map
        if let Some(logical_table_map) = &data_set.logical_table_map {
            if !logical_table_map.is_empty() {
                let mut logical_tables_json = serde_json::Map::new();
                for (table_id, logical_table) in logical_table_map {
                    let mut table_json = serde_json::Map::new();
                    table_json.insert(
                        "Alias".to_string(),
                        serde_json::Value::String(logical_table.alias.clone()),
                    );
                    // Note: Source field structure simplified for SDK compatibility
                    table_json.insert(
                        "TableType".to_string(),
                        serde_json::Value::String("LogicalTable".to_string()),
                    );
                    logical_tables_json
                        .insert(table_id.clone(), serde_json::Value::Object(table_json));
                }
                json.insert(
                    "LogicalTableMap".to_string(),
                    serde_json::Value::Object(logical_tables_json),
                );
            }
        }

        // Consumed SpiceSize in Bytes
        if data_set.consumed_spice_capacity_in_bytes > 0 {
            json.insert(
                "ConsumedSpiceCapacityInBytes".to_string(),
                serde_json::Value::Number(serde_json::Number::from(
                    data_set.consumed_spice_capacity_in_bytes,
                )),
            );
        }

        serde_json::Value::Object(json)
    }
}
