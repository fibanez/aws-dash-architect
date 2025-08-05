use anyhow::{Result, Context};
use aws_sdk_workspaces as workspaces;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct WorkSpacesService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl WorkSpacesService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List WorkSpaces
    pub async fn list_workspaces(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = workspaces::Client::new(&aws_config);
        
        let mut workspaces_list = Vec::new();
        let mut next_token: Option<String> = None;
        
        loop {
            let mut request = client.describe_workspaces();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            match request.send().await {
                Ok(response) => {
                    if let Some(ws_list) = response.workspaces {
                        for ws in ws_list {
                            let ws_json = self.workspace_to_json(&ws);
                            workspaces_list.push(ws_json);
                        }
                    }
                    
                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list WorkSpaces in account {} region {}: {}", account_id, region, e);
                    break;
                }
            }
        }

        Ok(workspaces_list)
    }

    /// Get detailed information for a specific WorkSpace
    pub async fn describe_workspace(
        &self,
        account_id: &str,
        region: &str,
        workspace_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = workspaces::Client::new(&aws_config);
        let response = client
            .describe_workspaces()
            .workspace_ids(workspace_id)
            .send()
            .await?;

        if let Some(workspaces) = response.workspaces {
            if let Some(ws) = workspaces.first() {
                return Ok(self.workspace_details_to_json(ws));
            }
        }

        Err(anyhow::anyhow!("WorkSpace {} not found", workspace_id))
    }

    /// List WorkSpaces directories
    pub async fn list_directories(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = workspaces::Client::new(&aws_config);
        
        let mut directories = Vec::new();
        let mut next_token: Option<String> = None;
        
        loop {
            let mut request = client.describe_workspace_directories();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            match request.send().await {
                Ok(response) => {
                    if let Some(dir_list) = response.directories {
                        for dir in dir_list {
                            let dir_json = self.directory_to_json(&dir);
                            directories.push(dir_json);
                        }
                    }
                    
                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list WorkSpaces directories in account {} region {}: {}", account_id, region, e);
                    break;
                }
            }
        }

        Ok(directories)
    }

    // JSON conversion methods - CRITICAL: Avoid serde_json::to_value for AWS SDK types
    fn workspace_to_json(&self, ws: &workspaces::types::Workspace) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(workspace_id) = &ws.workspace_id {
            json.insert("WorkspaceId".to_string(), serde_json::Value::String(workspace_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(workspace_id.clone()));
        }

        if let Some(directory_id) = &ws.directory_id {
            json.insert("DirectoryId".to_string(), serde_json::Value::String(directory_id.clone()));
        }

        if let Some(user_name) = &ws.user_name {
            json.insert("UserName".to_string(), serde_json::Value::String(user_name.clone()));
        }

        if let Some(ip_address) = &ws.ip_address {
            json.insert("IpAddress".to_string(), serde_json::Value::String(ip_address.clone()));
        }

        if let Some(state) = &ws.state {
            json.insert("State".to_string(), serde_json::Value::String(state.as_str().to_string()));
        }

        if let Some(bundle_id) = &ws.bundle_id {
            json.insert("BundleId".to_string(), serde_json::Value::String(bundle_id.clone()));
        }

        if let Some(subnet_id) = &ws.subnet_id {
            json.insert("SubnetId".to_string(), serde_json::Value::String(subnet_id.clone()));
        }

        if let Some(computer_name) = &ws.computer_name {
            json.insert("ComputerName".to_string(), serde_json::Value::String(computer_name.clone()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::WorkSpaces::Workspace".to_string()));

        serde_json::Value::Object(json)
    }

    fn workspace_details_to_json(&self, ws: &workspaces::types::Workspace) -> serde_json::Value {
        let mut base_json = self.workspace_to_json(ws);
        
        if let Some(json_obj) = base_json.as_object_mut() {
            // Add additional detailed fields
            if let Some(error_code) = &ws.error_code {
                json_obj.insert("ErrorCode".to_string(), serde_json::Value::String(error_code.clone()));
            }

            if let Some(error_message) = &ws.error_message {
                json_obj.insert("ErrorMessage".to_string(), serde_json::Value::String(error_message.clone()));
            }

            if let Some(volume_encryption_key) = &ws.volume_encryption_key {
                json_obj.insert("VolumeEncryptionKey".to_string(), serde_json::Value::String(volume_encryption_key.clone()));
            }

            if let Some(user_volume_encryption_enabled) = &ws.user_volume_encryption_enabled {
                json_obj.insert("UserVolumeEncryptionEnabled".to_string(), serde_json::Value::Bool(*user_volume_encryption_enabled));
            }

            if let Some(root_volume_encryption_enabled) = &ws.root_volume_encryption_enabled {
                json_obj.insert("RootVolumeEncryptionEnabled".to_string(), serde_json::Value::Bool(*root_volume_encryption_enabled));
            }

            // Workspace properties
            if let Some(workspace_properties) = &ws.workspace_properties {
                let mut properties_json = serde_json::Map::new();
                
                if let Some(running_mode) = &workspace_properties.running_mode {
                    properties_json.insert("RunningMode".to_string(), serde_json::Value::String(running_mode.as_str().to_string()));
                }

                if let Some(running_mode_auto_stop_timeout_in_minutes) = &workspace_properties.running_mode_auto_stop_timeout_in_minutes {
                    properties_json.insert("RunningModeAutoStopTimeoutInMinutes".to_string(), serde_json::Value::Number(serde_json::Number::from(*running_mode_auto_stop_timeout_in_minutes)));
                }

                if let Some(root_volume_size_gib) = &workspace_properties.root_volume_size_gib {
                    properties_json.insert("RootVolumeSizeGib".to_string(), serde_json::Value::Number(serde_json::Number::from(*root_volume_size_gib)));
                }

                if let Some(user_volume_size_gib) = &workspace_properties.user_volume_size_gib {
                    properties_json.insert("UserVolumeSizeGib".to_string(), serde_json::Value::Number(serde_json::Number::from(*user_volume_size_gib)));
                }

                if let Some(compute_type_name) = &workspace_properties.compute_type_name {
                    properties_json.insert("ComputeTypeName".to_string(), serde_json::Value::String(compute_type_name.as_str().to_string()));
                }

                json_obj.insert("WorkspaceProperties".to_string(), serde_json::Value::Object(properties_json));
            }

            // Modification states
            if let Some(modification_states) = &ws.modification_states {
                let modifications_array: Vec<serde_json::Value> = modification_states
                    .iter()
                    .map(|modification| {
                        let mut mod_json = serde_json::Map::new();
                        if let Some(resource) = &modification.resource {
                            mod_json.insert("Resource".to_string(), serde_json::Value::String(resource.as_str().to_string()));
                        }
                        if let Some(state) = &modification.state {
                            mod_json.insert("State".to_string(), serde_json::Value::String(state.as_str().to_string()));
                        }
                        serde_json::Value::Object(mod_json)
                    })
                    .collect();
                json_obj.insert("ModificationStates".to_string(), serde_json::Value::Array(modifications_array));
            }
        }

        base_json
    }

    fn directory_to_json(&self, directory: &workspaces::types::WorkspaceDirectory) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(directory_id) = &directory.directory_id {
            json.insert("DirectoryId".to_string(), serde_json::Value::String(directory_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(directory_id.clone()));
        }

        if let Some(alias) = &directory.alias {
            json.insert("Alias".to_string(), serde_json::Value::String(alias.clone()));
        }

        if let Some(directory_name) = &directory.directory_name {
            json.insert("DirectoryName".to_string(), serde_json::Value::String(directory_name.clone()));
        }

        if let Some(registration_code) = &directory.registration_code {
            json.insert("RegistrationCode".to_string(), serde_json::Value::String(registration_code.clone()));
        }

        if let Some(directory_type) = &directory.directory_type {
            json.insert("DirectoryType".to_string(), serde_json::Value::String(directory_type.as_str().to_string()));
        }

        if let Some(state) = &directory.state {
            json.insert("State".to_string(), serde_json::Value::String(state.as_str().to_string()));
        }

        // Subnet IDs
        if let Some(subnet_ids) = &directory.subnet_ids {
            let subnets_array: Vec<serde_json::Value> = subnet_ids
                .iter()
                .map(|subnet_id| serde_json::Value::String(subnet_id.clone()))
                .collect();
            json.insert("SubnetIds".to_string(), serde_json::Value::Array(subnets_array));
        }

        // DNS IPs
        if let Some(dns_ip_addresses) = &directory.dns_ip_addresses {
            let dns_array: Vec<serde_json::Value> = dns_ip_addresses
                .iter()
                .map(|dns_ip| serde_json::Value::String(dns_ip.clone()))
                .collect();
            json.insert("DnsIpAddresses".to_string(), serde_json::Value::Array(dns_array));
        }

        if let Some(customer_user_name) = &directory.customer_user_name {
            json.insert("CustomerUserName".to_string(), serde_json::Value::String(customer_user_name.clone()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::WorkSpaces::Directory".to_string()));

        serde_json::Value::Object(json)
    }

    /// Get detailed information for a specific WorkSpaces directory
    pub async fn describe_directory(
        &self,
        account_id: &str,
        region: &str,
        directory_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = workspaces::Client::new(&aws_config);
        let response = client
            .describe_workspace_directories()
            .directory_ids(directory_id)
            .send()
            .await?;

        if let Some(directories) = response.directories {
            if let Some(directory) = directories.first() {
                return Ok(self.directory_to_json(directory));
            }
        }
        
        Err(anyhow::anyhow!("WorkSpaces directory {} not found", directory_id))
    }
}