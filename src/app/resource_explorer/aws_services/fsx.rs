use anyhow::{Result, Context};
use aws_sdk_fsx as fsx;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct FsxService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl FsxService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List FSx file systems
    pub async fn list_file_systems(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = fsx::Client::new(&aws_config);
        
        let mut file_systems = Vec::new();
        let mut next_token: Option<String> = None;
        
        loop {
            let mut request = client.describe_file_systems();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            match request.send().await {
                Ok(response) => {
                    if let Some(fs_list) = response.file_systems {
                        for fs in fs_list {
                            let fs_json = self.file_system_to_json(&fs);
                            file_systems.push(fs_json);
                        }
                    }
                    
                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list FSx file systems in account {} region {}: {}", account_id, region, e);
                    break;
                }
            }
        }

        Ok(file_systems)
    }

    /// Get detailed information for a specific FSx file system
    pub async fn describe_file_system(
        &self,
        account_id: &str,
        region: &str,
        file_system_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = fsx::Client::new(&aws_config);
        let response = client
            .describe_file_systems()
            .file_system_ids(file_system_id)
            .send()
            .await?;

        if let Some(file_systems) = response.file_systems {
            if let Some(fs) = file_systems.first() {
                return Ok(self.file_system_details_to_json(fs));
            }
        }

        Err(anyhow::anyhow!("FSx file system {} not found", file_system_id))
    }

    /// List FSx backups
    pub async fn list_backups(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = fsx::Client::new(&aws_config);
        
        let mut backups = Vec::new();
        let mut next_token: Option<String> = None;
        
        loop {
            let mut request = client.describe_backups();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            match request.send().await {
                Ok(response) => {
                    if let Some(backup_list) = response.backups {
                        for backup in backup_list {
                            let backup_json = self.backup_to_json(&backup);
                            backups.push(backup_json);
                        }
                    }
                    
                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list FSx backups in account {} region {}: {}", account_id, region, e);
                    break;
                }
            }
        }

        Ok(backups)
    }

    // JSON conversion methods - CRITICAL: Avoid serde_json::to_value for AWS SDK types
    fn file_system_to_json(&self, fs: &fsx::types::FileSystem) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(file_system_id) = &fs.file_system_id {
            json.insert("FileSystemId".to_string(), serde_json::Value::String(file_system_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(file_system_id.clone()));
        }

        if let Some(file_system_type) = &fs.file_system_type {
            json.insert("FileSystemType".to_string(), serde_json::Value::String(file_system_type.as_str().to_string()));
        }

        if let Some(lifecycle) = &fs.lifecycle {
            json.insert("Lifecycle".to_string(), serde_json::Value::String(lifecycle.as_str().to_string()));
        }

        if let Some(storage_capacity) = &fs.storage_capacity {
            json.insert("StorageCapacity".to_string(), serde_json::Value::Number(serde_json::Number::from(*storage_capacity)));
        }

        if let Some(storage_type) = &fs.storage_type {
            json.insert("StorageType".to_string(), serde_json::Value::String(storage_type.as_str().to_string()));
        }

        if let Some(vpc_id) = &fs.vpc_id {
            json.insert("VpcId".to_string(), serde_json::Value::String(vpc_id.clone()));
        }

        if let Some(dns_name) = &fs.dns_name {
            json.insert("DNSName".to_string(), serde_json::Value::String(dns_name.clone()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::FSx::FileSystem".to_string()));

        serde_json::Value::Object(json)
    }

    fn file_system_details_to_json(&self, fs: &fsx::types::FileSystem) -> serde_json::Value {
        let mut base_json = self.file_system_to_json(fs);
        
        if let Some(json_obj) = base_json.as_object_mut() {
            // Add additional detailed fields
            if let Some(resource_arn) = &fs.resource_arn {
                json_obj.insert("ResourceARN".to_string(), serde_json::Value::String(resource_arn.clone()));
            }

            if let Some(creation_time) = &fs.creation_time {
                json_obj.insert("CreationTime".to_string(), serde_json::Value::String(creation_time.fmt(aws_smithy_types::date_time::Format::DateTime).unwrap_or_default()));
            }

            if let Some(kms_key_id) = &fs.kms_key_id {
                json_obj.insert("KmsKeyId".to_string(), serde_json::Value::String(kms_key_id.clone()));
            }

            if let Some(owner_id) = &fs.owner_id {
                json_obj.insert("OwnerId".to_string(), serde_json::Value::String(owner_id.clone()));
            }

            // Subnet IDs
            if let Some(subnet_ids) = &fs.subnet_ids {
                let subnets_array: Vec<serde_json::Value> = subnet_ids
                    .iter()
                    .map(|subnet_id| serde_json::Value::String(subnet_id.clone()))
                    .collect();
                json_obj.insert("SubnetIds".to_string(), serde_json::Value::Array(subnets_array));
            }

            // Network interface IDs
            if let Some(network_interface_ids) = &fs.network_interface_ids {
                let ni_array: Vec<serde_json::Value> = network_interface_ids
                    .iter()
                    .map(|ni_id| serde_json::Value::String(ni_id.clone()))
                    .collect();
                json_obj.insert("NetworkInterfaceIds".to_string(), serde_json::Value::Array(ni_array));
            }

            // Windows configuration
            if let Some(windows_config) = &fs.windows_configuration {
                let mut windows_json = serde_json::Map::new();
                
                if let Some(active_directory_id) = &windows_config.active_directory_id {
                    windows_json.insert("ActiveDirectoryId".to_string(), serde_json::Value::String(active_directory_id.clone()));
                }

                if let Some(throughput_capacity) = &windows_config.throughput_capacity {
                    windows_json.insert("ThroughputCapacity".to_string(), serde_json::Value::Number(serde_json::Number::from(*throughput_capacity)));
                }

                if let Some(weekly_maintenance_start_time) = &windows_config.weekly_maintenance_start_time {
                    windows_json.insert("WeeklyMaintenanceStartTime".to_string(), serde_json::Value::String(weekly_maintenance_start_time.clone()));
                }

                json_obj.insert("WindowsConfiguration".to_string(), serde_json::Value::Object(windows_json));
            }

            // Lustre configuration
            if let Some(lustre_config) = &fs.lustre_configuration {
                let mut lustre_json = serde_json::Map::new();
                
                if let Some(weekly_maintenance_start_time) = &lustre_config.weekly_maintenance_start_time {
                    lustre_json.insert("WeeklyMaintenanceStartTime".to_string(), serde_json::Value::String(weekly_maintenance_start_time.clone()));
                }

                if let Some(data_repository_configuration) = &lustre_config.data_repository_configuration {
                    let mut data_repo_json = serde_json::Map::new();
                    
                    if let Some(lifecycle) = &data_repository_configuration.lifecycle {
                        data_repo_json.insert("Lifecycle".to_string(), serde_json::Value::String(lifecycle.as_str().to_string()));
                    }

                    if let Some(import_path) = &data_repository_configuration.import_path {
                        data_repo_json.insert("ImportPath".to_string(), serde_json::Value::String(import_path.clone()));
                    }

                    if let Some(export_path) = &data_repository_configuration.export_path {
                        data_repo_json.insert("ExportPath".to_string(), serde_json::Value::String(export_path.clone()));
                    }

                    lustre_json.insert("DataRepositoryConfiguration".to_string(), serde_json::Value::Object(data_repo_json));
                }

                json_obj.insert("LustreConfiguration".to_string(), serde_json::Value::Object(lustre_json));
            }

            // Tags
            if let Some(tags) = &fs.tags {
                let tags_array: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        }
                        if let Some(value) = &tag.value {
                            tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                        }
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json_obj.insert("Tags".to_string(), serde_json::Value::Array(tags_array));
            }
        }

        base_json
    }

    fn backup_to_json(&self, backup: &fsx::types::Backup) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(backup_id) = &backup.backup_id {
            json.insert("BackupId".to_string(), serde_json::Value::String(backup_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(backup_id.clone()));
        }

        if let Some(lifecycle) = &backup.lifecycle {
            json.insert("Lifecycle".to_string(), serde_json::Value::String(lifecycle.as_str().to_string()));
        }

        if let Some(r#type) = &backup.r#type {
            json.insert("Type".to_string(), serde_json::Value::String(r#type.as_str().to_string()));
        }

        if let Some(file_system) = &backup.file_system {
            if let Some(file_system_id) = &file_system.file_system_id {
                json.insert("FileSystemId".to_string(), serde_json::Value::String(file_system_id.clone()));
            }
        }

        if let Some(creation_time) = &backup.creation_time {
            json.insert("CreationTime".to_string(), serde_json::Value::String(creation_time.fmt(aws_smithy_types::date_time::Format::DateTime).unwrap_or_default()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::FSx::Backup".to_string()));

        serde_json::Value::Object(json)
    }

    /// Get detailed information for a specific FSx backup
    pub async fn describe_backup(
        &self,
        account_id: &str,
        region: &str,
        backup_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = fsx::Client::new(&aws_config);
        let response = client
            .describe_backups()
            .backup_ids(backup_id)
            .send()
            .await?;

        if let Some(backups) = response.backups {
            if let Some(backup) = backups.first() {
                return Ok(self.backup_to_json(backup));
            }
        }
        
        Err(anyhow::anyhow!("FSx backup {} not found", backup_id))
    }
}