use anyhow::{Result, Context};
use aws_sdk_lakeformation as lakeformation;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct LakeFormationService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl LakeFormationService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// Get Lake Formation Data Lake Settings (there's only one per account/region)
    pub async fn list_data_lake_settings(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = lakeformation::Client::new(&aws_config);
        
        match self.describe_data_lake_settings_internal(&client).await {
            Ok(settings) => Ok(vec![settings]),
            Err(_) => {
                // If describe fails, Lake Formation might not be configured
                Ok(Vec::new())
            }
        }
    }

    /// Get detailed information for Lake Formation Data Lake Settings
    pub async fn describe_data_lake_settings(
        &self,
        account_id: &str,
        region: &str,
        _settings_id: &str, // Not used since there's only one settings per account/region
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = lakeformation::Client::new(&aws_config);
        self.describe_data_lake_settings_internal(&client).await
    }

    async fn describe_data_lake_settings_internal(
        &self,
        client: &lakeformation::Client,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_data_lake_settings()
            .send()
            .await?;

        Ok(self.data_lake_settings_to_json(&response))
    }

    fn data_lake_settings_to_json(&self, response: &lakeformation::operation::get_data_lake_settings::GetDataLakeSettingsOutput) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        // Create a synthetic resource ID for identification
        json.insert("ResourceId".to_string(), serde_json::Value::String("DataLakeSettings".to_string()));
        json.insert("Name".to_string(), serde_json::Value::String("Lake Formation Data Lake Settings".to_string()));

        if let Some(data_lake_settings) = &response.data_lake_settings {
            if let Some(data_lake_admins) = &data_lake_settings.data_lake_admins {
                let admins_json: Vec<serde_json::Value> = data_lake_admins
                    .iter()
                    .map(|admin| {
                        let mut admin_json = serde_json::Map::new();
                        if let Some(data_lake_principal_identifier) = &admin.data_lake_principal_identifier {
                            admin_json.insert("DataLakePrincipalIdentifier".to_string(), serde_json::Value::String(data_lake_principal_identifier.clone()));
                        }
                        serde_json::Value::Object(admin_json)
                    })
                    .collect();
                json.insert("DataLakeAdmins".to_string(), serde_json::Value::Array(admins_json));
            }

            if let Some(create_database_default_permissions) = &data_lake_settings.create_database_default_permissions {
                let db_perms_json: Vec<serde_json::Value> = create_database_default_permissions
                    .iter()
                    .map(|perm| {
                        let mut perm_json = serde_json::Map::new();
                        if let Some(principal) = &perm.principal {
                            if let Some(data_lake_principal_identifier) = &principal.data_lake_principal_identifier {
                                perm_json.insert("Principal".to_string(), serde_json::Value::String(data_lake_principal_identifier.clone()));
                            }
                        }
                        if let Some(permissions) = &perm.permissions {
                            let perms_array: Vec<serde_json::Value> = permissions
                                .iter()
                                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                                .collect();
                            perm_json.insert("Permissions".to_string(), serde_json::Value::Array(perms_array));
                        }
                        serde_json::Value::Object(perm_json)
                    })
                    .collect();
                json.insert("CreateDatabaseDefaultPermissions".to_string(), serde_json::Value::Array(db_perms_json));
            }

            if let Some(create_table_default_permissions) = &data_lake_settings.create_table_default_permissions {
                let table_perms_json: Vec<serde_json::Value> = create_table_default_permissions
                    .iter()
                    .map(|perm| {
                        let mut perm_json = serde_json::Map::new();
                        if let Some(principal) = &perm.principal {
                            if let Some(data_lake_principal_identifier) = &principal.data_lake_principal_identifier {
                                perm_json.insert("Principal".to_string(), serde_json::Value::String(data_lake_principal_identifier.clone()));
                            }
                        }
                        if let Some(permissions) = &perm.permissions {
                            let perms_array: Vec<serde_json::Value> = permissions
                                .iter()
                                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                                .collect();
                            perm_json.insert("Permissions".to_string(), serde_json::Value::Array(perms_array));
                        }
                        serde_json::Value::Object(perm_json)
                    })
                    .collect();
                json.insert("CreateTableDefaultPermissions".to_string(), serde_json::Value::Array(table_perms_json));
            }

            if let Some(parameters) = &data_lake_settings.parameters {
                let params_json: serde_json::Map<String, serde_json::Value> = parameters
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect();
                json.insert("Parameters".to_string(), serde_json::Value::Object(params_json));
            }

            if let Some(trusted_resource_owners) = &data_lake_settings.trusted_resource_owners {
                let owners_json: Vec<serde_json::Value> = trusted_resource_owners
                    .iter()
                    .map(|owner| serde_json::Value::String(owner.clone()))
                    .collect();
                json.insert("TrustedResourceOwners".to_string(), serde_json::Value::Array(owners_json));
            }

            if let Some(allow_external_data_filtering) = data_lake_settings.allow_external_data_filtering {
                json.insert("AllowExternalDataFiltering".to_string(), serde_json::Value::Bool(allow_external_data_filtering));
            }

            if let Some(allow_full_table_external_data_access) = data_lake_settings.allow_full_table_external_data_access {
                json.insert("AllowFullTableExternalDataAccess".to_string(), serde_json::Value::Bool(allow_full_table_external_data_access));
            }

            if let Some(external_data_filtering_allow_list) = &data_lake_settings.external_data_filtering_allow_list {
                let allow_list_json: Vec<serde_json::Value> = external_data_filtering_allow_list
                    .iter()
                    .map(|principal| {
                        let mut principal_json = serde_json::Map::new();
                        if let Some(data_lake_principal_identifier) = &principal.data_lake_principal_identifier {
                            principal_json.insert("DataLakePrincipalIdentifier".to_string(), serde_json::Value::String(data_lake_principal_identifier.clone()));
                        }
                        serde_json::Value::Object(principal_json)
                    })
                    .collect();
                json.insert("ExternalDataFilteringAllowList".to_string(), serde_json::Value::Array(allow_list_json));
            }

            if let Some(authorized_session_tag_value_list) = &data_lake_settings.authorized_session_tag_value_list {
                let tags_json: Vec<serde_json::Value> = authorized_session_tag_value_list
                    .iter()
                    .map(|tag| serde_json::Value::String(tag.clone()))
                    .collect();
                json.insert("AuthorizedSessionTagValueList".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        // Status is always ACTIVE if we can get the settings
        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        serde_json::Value::Object(json)
    }

    /// List Lake Formation permissions
    pub async fn list_permissions(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = lakeformation::Client::new(&aws_config);
        let mut permissions = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_permissions();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(principal_resource_permissions) = response.principal_resource_permissions {
                for permission in principal_resource_permissions {
                    let permission_json = self.permission_to_json(&permission);
                    permissions.push(permission_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(permissions)
    }

    fn permission_to_json(&self, permission: &lakeformation::types::PrincipalResourcePermissions) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(principal) = &permission.principal {
            if let Some(data_lake_principal_identifier) = &principal.data_lake_principal_identifier {
                json.insert("Principal".to_string(), serde_json::Value::String(data_lake_principal_identifier.clone()));
            }
        }

        if let Some(resource) = &permission.resource {
            let mut resource_json = serde_json::Map::new();
            
            if let Some(_catalog) = &resource.catalog {
                resource_json.insert("Catalog".to_string(), serde_json::Value::Object(serde_json::Map::new()));
            }
            
            if let Some(database) = &resource.database {
                let mut db_json = serde_json::Map::new();
                if let Some(catalog_id) = &database.catalog_id {
                    db_json.insert("CatalogId".to_string(), serde_json::Value::String(catalog_id.clone()));
                }
                db_json.insert("Name".to_string(), serde_json::Value::String(database.name.clone()));
                resource_json.insert("Database".to_string(), serde_json::Value::Object(db_json));
            }
            
            if let Some(table) = &resource.table {
                let mut table_json = serde_json::Map::new();
                if let Some(catalog_id) = &table.catalog_id {
                    table_json.insert("CatalogId".to_string(), serde_json::Value::String(catalog_id.clone()));
                }
                table_json.insert("DatabaseName".to_string(), serde_json::Value::String(table.database_name.clone()));
                if let Some(name) = &table.name {
                    table_json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
                }
                resource_json.insert("Table".to_string(), serde_json::Value::Object(table_json));
            }
            
            json.insert("Resource".to_string(), serde_json::Value::Object(resource_json));
        }

        if let Some(permissions) = &permission.permissions {
            let perms_json: Vec<serde_json::Value> = permissions
                .iter()
                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                .collect();
            json.insert("Permissions".to_string(), serde_json::Value::Array(perms_json));
        }

        if let Some(permissions_with_grant_option) = &permission.permissions_with_grant_option {
            let grant_perms_json: Vec<serde_json::Value> = permissions_with_grant_option
                .iter()
                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                .collect();
            json.insert("PermissionsWithGrantOption".to_string(), serde_json::Value::Array(grant_perms_json));
        }

        serde_json::Value::Object(json)
    }
}