use crate::app::cloudformation_manager::parameter_dialog::ParameterSource;
use crate::app::projects::Project;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Parameter value with metadata for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValue {
    pub value: String,
    pub is_sensitive: bool,
    pub source: ParameterSource,
    pub last_used: DateTime<Utc>,
    pub description: Option<String>,
}

/// Environment-specific parameter values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentParameterValues {
    pub environment_name: String,
    pub parameters: HashMap<String, ParameterValue>,
    pub last_updated: DateTime<Utc>,
    pub last_deployment: Option<DateTime<Utc>>,
}

/// Historical parameter entry for reuse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterHistoryEntry {
    pub parameter_name: String,
    pub environment: String,
    pub value: String, // Non-sensitive values only
    pub timestamp: DateTime<Utc>,
    pub deployment_id: Option<String>,
    pub description: Option<String>,
}

/// Parameter persistence manager for CloudFormation Manager
pub struct ParameterPersistenceManager {
    base_path: PathBuf,
}

impl ParameterPersistenceManager {
    /// Create a new parameter persistence manager
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Get the parameters directory for a project
    fn get_parameters_dir(&self, project: &Project) -> PathBuf {
        if let Some(local_folder) = &project.local_folder {
            local_folder.join("parameters")
        } else {
            self.base_path.join(&project.short_name).join("parameters")
        }
    }

    /// Get the parameter file path for an environment
    fn get_parameter_file_path(&self, project: &Project, environment: &str) -> PathBuf {
        self.get_parameters_dir(project)
            .join(format!("{}-parameters.json", environment))
    }

    /// Get the parameter history file path
    fn get_parameter_history_path(&self, project: &Project) -> PathBuf {
        self.get_parameters_dir(project)
            .join("parameter_history.json")
    }

    /// Get the parameter sources file path
    fn get_parameter_sources_path(&self, project: &Project) -> PathBuf {
        self.get_parameters_dir(project)
            .join("parameter_sources.json")
    }

    /// Ensure the parameters directory exists
    fn ensure_parameters_dir(&self, project: &Project) -> Result<()> {
        let params_dir = self.get_parameters_dir(project);
        if !params_dir.exists() {
            fs::create_dir_all(&params_dir).with_context(|| {
                format!("Failed to create parameters directory: {:?}", params_dir)
            })?;
            info!("Created parameters directory: {:?}", params_dir);
        }
        Ok(())
    }

    /// Save parameter values for a specific environment
    pub fn save_environment_parameters(
        &self,
        project: &Project,
        environment: &str,
        parameters: &HashMap<String, String>,
        parameter_sources: &HashMap<String, ParameterSource>,
        sensitive_parameters: &[String],
    ) -> Result<()> {
        self.ensure_parameters_dir(project)?;

        let mut env_params = EnvironmentParameterValues {
            environment_name: environment.to_string(),
            parameters: HashMap::new(),
            last_updated: Utc::now(),
            last_deployment: None,
        };

        // Convert parameters to ParameterValue structs
        for (name, value) in parameters {
            let source = parameter_sources
                .get(name)
                .cloned()
                .unwrap_or(ParameterSource::Manual);
            let is_sensitive = sensitive_parameters.contains(name);

            env_params.parameters.insert(
                name.clone(),
                ParameterValue {
                    value: value.clone(),
                    is_sensitive,
                    source,
                    last_used: Utc::now(),
                    description: None,
                },
            );
        }

        let file_path = self.get_parameter_file_path(project, environment);
        let json_content = serde_json::to_string_pretty(&env_params)
            .with_context(|| "Failed to serialize environment parameters")?;

        fs::write(&file_path, json_content)
            .with_context(|| format!("Failed to write parameter file: {:?}", file_path))?;

        info!(
            "Saved {} parameters for environment {} in project {}",
            parameters.len(),
            environment,
            project.name
        );

        Ok(())
    }

    /// Load parameter values for a specific environment
    pub fn load_environment_parameters(
        &self,
        project: &Project,
        environment: &str,
    ) -> Result<Option<EnvironmentParameterValues>> {
        let file_path = self.get_parameter_file_path(project, environment);

        if !file_path.exists() {
            debug!(
                "Parameter file does not exist for environment {}: {:?}",
                environment, file_path
            );
            return Ok(None);
        }

        let json_content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read parameter file: {:?}", file_path))?;

        let env_params: EnvironmentParameterValues = serde_json::from_str(&json_content)
            .with_context(|| format!("Failed to parse parameter file: {:?}", file_path))?;

        debug!(
            "Loaded {} parameters for environment {} from project {}",
            env_params.parameters.len(),
            environment,
            project.name
        );

        Ok(Some(env_params))
    }

    /// Get all available environments with parameter files
    pub fn get_environments_with_parameters(&self, project: &Project) -> Result<Vec<String>> {
        let params_dir = self.get_parameters_dir(project);

        if !params_dir.exists() {
            return Ok(Vec::new());
        }

        let mut environments = Vec::new();

        for entry in fs::read_dir(&params_dir)
            .with_context(|| format!("Failed to read parameters directory: {:?}", params_dir))?
        {
            let entry = entry?;
            let file_name = entry.file_name();

            if let Some(name_str) = file_name.to_str() {
                if name_str.ends_with("-parameters.json") {
                    let env_name = name_str
                        .strip_suffix("-parameters.json")
                        .unwrap_or(name_str)
                        .to_string();
                    environments.push(env_name);
                }
            }
        }

        environments.sort();
        Ok(environments)
    }

    /// Add parameter values to history (for non-sensitive parameters only)
    pub fn add_to_parameter_history(
        &self,
        project: &Project,
        environment: &str,
        parameters: &HashMap<String, String>,
        sensitive_parameters: &[String],
        deployment_id: Option<String>,
    ) -> Result<()> {
        self.ensure_parameters_dir(project)?;

        let history_path = self.get_parameter_history_path(project);
        let mut history: Vec<ParameterHistoryEntry> = if history_path.exists() {
            let json_content = fs::read_to_string(&history_path)
                .with_context(|| format!("Failed to read parameter history: {:?}", history_path))?;

            serde_json::from_str(&json_content)
                .with_context(|| "Failed to parse parameter history")?
        } else {
            Vec::new()
        };

        // Add new entries for non-sensitive parameters
        for (name, value) in parameters {
            if !sensitive_parameters.contains(name) {
                let entry = ParameterHistoryEntry {
                    parameter_name: name.clone(),
                    environment: environment.to_string(),
                    value: value.clone(),
                    timestamp: Utc::now(),
                    deployment_id: deployment_id.clone(),
                    description: None,
                };
                history.push(entry);
            }
        }

        // Keep only the last 100 entries per parameter to avoid file bloat
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let mut param_counts: HashMap<String, usize> = HashMap::new();
        history.retain(|entry| {
            let count = param_counts
                .entry(entry.parameter_name.clone())
                .or_insert(0);
            *count += 1;
            *count <= 100
        });

        let json_content = serde_json::to_string_pretty(&history)
            .with_context(|| "Failed to serialize parameter history")?;

        fs::write(&history_path, json_content)
            .with_context(|| format!("Failed to write parameter history: {:?}", history_path))?;

        debug!(
            "Added {} parameters to history for environment {} in project {}",
            parameters.len() - sensitive_parameters.len(),
            environment,
            project.name
        );

        Ok(())
    }

    /// Get parameter history for a specific parameter
    pub fn get_parameter_history(
        &self,
        project: &Project,
        parameter_name: &str,
        environment: Option<&str>,
    ) -> Result<Vec<ParameterHistoryEntry>> {
        let history_path = self.get_parameter_history_path(project);

        if !history_path.exists() {
            return Ok(Vec::new());
        }

        let json_content = fs::read_to_string(&history_path)
            .with_context(|| format!("Failed to read parameter history: {:?}", history_path))?;

        let full_history: Vec<ParameterHistoryEntry> = serde_json::from_str(&json_content)
            .with_context(|| "Failed to parse parameter history")?;

        let filtered_history: Vec<ParameterHistoryEntry> = full_history
            .into_iter()
            .filter(|entry| {
                entry.parameter_name == parameter_name
                    && (environment.is_none() || Some(entry.environment.as_str()) == environment)
            })
            .collect();

        Ok(filtered_history)
    }

    /// Save parameter sources mapping
    pub fn save_parameter_sources(
        &self,
        project: &Project,
        parameter_sources: &HashMap<String, ParameterSource>,
    ) -> Result<()> {
        self.ensure_parameters_dir(project)?;

        let sources_path = self.get_parameter_sources_path(project);
        let json_content = serde_json::to_string_pretty(parameter_sources)
            .with_context(|| "Failed to serialize parameter sources")?;

        fs::write(&sources_path, json_content)
            .with_context(|| format!("Failed to write parameter sources: {:?}", sources_path))?;

        debug!(
            "Saved parameter sources for {} parameters in project {}",
            parameter_sources.len(),
            project.name
        );

        Ok(())
    }

    /// Load parameter sources mapping
    pub fn load_parameter_sources(
        &self,
        project: &Project,
    ) -> Result<HashMap<String, ParameterSource>> {
        let sources_path = self.get_parameter_sources_path(project);

        if !sources_path.exists() {
            return Ok(HashMap::new());
        }

        let json_content = fs::read_to_string(&sources_path)
            .with_context(|| format!("Failed to read parameter sources: {:?}", sources_path))?;

        let parameter_sources: HashMap<String, ParameterSource> =
            serde_json::from_str(&json_content)
                .with_context(|| "Failed to parse parameter sources")?;

        debug!(
            "Loaded parameter sources for {} parameters from project {}",
            parameter_sources.len(),
            project.name
        );

        Ok(parameter_sources)
    }

    /// Delete parameter data for an environment
    pub fn delete_environment_parameters(
        &self,
        project: &Project,
        environment: &str,
    ) -> Result<()> {
        let file_path = self.get_parameter_file_path(project, environment);

        if file_path.exists() {
            fs::remove_file(&file_path)
                .with_context(|| format!("Failed to delete parameter file: {:?}", file_path))?;

            info!(
                "Deleted parameters for environment {} in project {}",
                environment, project.name
            );
        }

        Ok(())
    }

    /// Copy parameters from one environment to another
    pub fn copy_environment_parameters(
        &self,
        project: &Project,
        source_environment: &str,
        target_environment: &str,
    ) -> Result<()> {
        if let Some(mut source_params) =
            self.load_environment_parameters(project, source_environment)?
        {
            // Update environment name and timestamps
            source_params.environment_name = target_environment.to_string();
            source_params.last_updated = Utc::now();
            source_params.last_deployment = None;

            let file_path = self.get_parameter_file_path(project, target_environment);
            let json_content = serde_json::to_string_pretty(&source_params)
                .with_context(|| "Failed to serialize copied parameters")?;

            fs::write(&file_path, json_content).with_context(|| {
                format!("Failed to write copied parameter file: {:?}", file_path)
            })?;

            info!(
                "Copied {} parameters from {} to {} in project {}",
                source_params.parameters.len(),
                source_environment,
                target_environment,
                project.name
            );
        } else {
            warn!(
                "Source environment {} has no parameters to copy in project {}",
                source_environment, project.name
            );
        }

        Ok(())
    }

    /// Get parameter statistics for the project
    pub fn get_parameter_statistics(&self, project: &Project) -> Result<ParameterStatistics> {
        let environments = self.get_environments_with_parameters(project)?;
        let mut stats = ParameterStatistics {
            total_environments: environments.len(),
            total_parameters: 0,
            sensitive_parameters: 0,
            parameter_store_parameters: 0,
            secrets_manager_parameters: 0,
            manual_parameters: 0,
            last_updated: None,
        };

        for env in &environments {
            if let Some(env_params) = self.load_environment_parameters(project, env)? {
                stats.total_parameters += env_params.parameters.len();

                if stats.last_updated.is_none()
                    || Some(env_params.last_updated) > stats.last_updated
                {
                    stats.last_updated = Some(env_params.last_updated);
                }

                for param in env_params.parameters.values() {
                    if param.is_sensitive {
                        stats.sensitive_parameters += 1;
                    }

                    match param.source {
                        ParameterSource::ParameterStore => stats.parameter_store_parameters += 1,
                        ParameterSource::SecretsManager => stats.secrets_manager_parameters += 1,
                        ParameterSource::Manual => stats.manual_parameters += 1,
                        ParameterSource::History => stats.manual_parameters += 1,
                    }
                }
            }
        }

        Ok(stats)
    }
}

/// Parameter statistics for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterStatistics {
    pub total_environments: usize,
    pub total_parameters: usize,
    pub sensitive_parameters: usize,
    pub parameter_store_parameters: usize,
    pub secrets_manager_parameters: usize,
    pub manual_parameters: usize,
    pub last_updated: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project() -> Project {
        Project {
            name: "Test Project".to_string(),
            description: "Test project for parameter persistence".to_string(),
            short_name: "testapp".to_string(),
            created: Utc::now(),
            updated: Utc::now(),
            local_folder: None,
            git_url: None,
            environments: vec![],
            default_region: Some("us-east-1".to_string()),
            cfn_template: None,
        }
    }

    #[test]
    fn test_parameter_persistence_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ParameterPersistenceManager::new(temp_dir.path().to_path_buf());

        let project = create_test_project();
        let params_dir = manager.get_parameters_dir(&project);

        assert!(params_dir.to_string_lossy().contains("testapp"));
        assert!(params_dir.to_string_lossy().contains("parameters"));
    }

    #[test]
    fn test_save_and_load_environment_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ParameterPersistenceManager::new(temp_dir.path().to_path_buf());
        let project = create_test_project();

        let mut parameters = HashMap::new();
        parameters.insert("InstanceType".to_string(), "t3.medium".to_string());
        parameters.insert("DatabasePassword".to_string(), "secret123".to_string());

        let mut parameter_sources = HashMap::new();
        parameter_sources.insert("InstanceType".to_string(), ParameterSource::Manual);
        parameter_sources.insert(
            "DatabasePassword".to_string(),
            ParameterSource::SecretsManager,
        );

        let sensitive_parameters = vec!["DatabasePassword".to_string()];

        // Save parameters
        let result = manager.save_environment_parameters(
            &project,
            "dev",
            &parameters,
            &parameter_sources,
            &sensitive_parameters,
        );
        assert!(result.is_ok());

        // Load parameters
        let loaded = manager
            .load_environment_parameters(&project, "dev")
            .unwrap();
        assert!(loaded.is_some());

        let env_params = loaded.unwrap();
        assert_eq!(env_params.environment_name, "dev");
        assert_eq!(env_params.parameters.len(), 2);

        assert!(env_params.parameters.contains_key("InstanceType"));
        assert!(env_params.parameters.contains_key("DatabasePassword"));

        let db_password = &env_params.parameters["DatabasePassword"];
        assert!(db_password.is_sensitive);
        assert_eq!(db_password.source, ParameterSource::SecretsManager);
    }

    #[test]
    fn test_parameter_history() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ParameterPersistenceManager::new(temp_dir.path().to_path_buf());
        let project = create_test_project();

        let mut parameters = HashMap::new();
        parameters.insert("InstanceType".to_string(), "t3.medium".to_string());
        parameters.insert("DatabasePassword".to_string(), "secret123".to_string());

        let sensitive_parameters = vec!["DatabasePassword".to_string()];

        // Add to history
        let result = manager.add_to_parameter_history(
            &project,
            "dev",
            &parameters,
            &sensitive_parameters,
            Some("deployment-123".to_string()),
        );
        assert!(result.is_ok());

        // Get history for InstanceType (non-sensitive)
        let history = manager
            .get_parameter_history(&project, "InstanceType", Some("dev"))
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value, "t3.medium");
        assert_eq!(history[0].deployment_id, Some("deployment-123".to_string()));

        // Get history for DatabasePassword (sensitive - should be empty)
        let sensitive_history = manager
            .get_parameter_history(&project, "DatabasePassword", Some("dev"))
            .unwrap();
        assert_eq!(sensitive_history.len(), 0);
    }
}
