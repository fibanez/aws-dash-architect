//! Read CloudFormation Template Tool for Bridge Agent
//!
//! This tool allows the Bridge Agent to read the CloudFormation template from the
//! current project's Resources folder, providing access to the complete template
//! structure including all resources, parameters, outputs, and metadata.

use async_trait::async_trait;
use serde_json;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{error, info};

use super::super::{get_global_current_project};

/// ReadCloudFormationTemplate tool for accessing project CloudFormation templates
#[derive(Clone)]
pub struct ReadCloudFormationTemplateTool {
}

impl std::fmt::Debug for ReadCloudFormationTemplateTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadCloudFormationTemplateTool").finish()
    }
}

impl ReadCloudFormationTemplateTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for ReadCloudFormationTemplateTool {
    fn name(&self) -> &str {
        "read_cloudformation_template"
    }

    fn description(&self) -> &str {
        "Read the CloudFormation template from the current project's Resources folder. This tool provides access to the complete template structure including all resources, parameters, outputs, and metadata.

This tool requires:
- A project must be currently open and loaded
- The project must have a CloudFormation template in its Resources folder

Usage:
- This tool takes no parameters. Leave the input blank or empty.
- Returns the complete CloudFormation template as JSON
- Includes all template sections: Resources, Parameters, Outputs, Metadata, etc.
- Template is read from the project's Resources/cloudformation_template.json file"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _parameters: Option<serde_json::Value>,
        agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        info!("🔥 STEP 1: ReadCloudFormationTemplate tool execution STARTED");

        // Get session ID for context
        let session_id = agent_context
            .map(|ctx| ctx.agent_id.clone())
            .unwrap_or_else(|| "default-session".to_string());

        info!("🔥 STEP 2: Session ID determined: {}", session_id);

        // Check if there's a current project available
        info!("🔥 STEP 3: Attempting to get global current project...");
        let global_project = get_global_current_project();
        
        if global_project.is_none() {
            error!("🔥 STEP 3 FAILED: get_global_current_project() returned None");
            error!("🔥 DEBUG: This means no project has been set in global state");
            error!("🔥 DEBUG: Check if set_global_current_project() was called when project loaded");
            return Err(ToolError::ExecutionFailed {
                message: "No project is currently open. Please open a project first using the Project Command Palette (press Space → type 'Project' or press 'P').".to_string(),
            });
        }
        
        let global_project = global_project.unwrap();
        info!("🔥 STEP 3 SUCCESS: Global project found");

        info!("🔥 STEP 4: Attempting to acquire project lock...");
        let project = match global_project.lock() {
            Ok(proj) => {
                info!("🔥 STEP 4 SUCCESS: Project lock acquired successfully");
                proj
            },
            Err(e) => {
                error!("🔥 STEP 4 FAILED: Failed to lock global project: {}", e);
                error!("🔥 DEBUG: This indicates a mutex poisoning or deadlock issue");
                return Err(ToolError::ExecutionFailed {
                    message: "Failed to access project data".to_string(),
                });
            }
        };

        info!("🔥 STEP 5: Project locked successfully, checking project details...");
        info!("🔥 STEP 5 DEBUG: Project name: '{}'", project.name);
        info!("🔥 STEP 5 DEBUG: Project description: '{}'", project.description);
        info!("🔥 STEP 5 DEBUG: Project has cfn_template: {}", project.cfn_template.is_some());

        if let Some(folder) = &project.local_folder {
            info!("🔥 STEP 5 DEBUG: Project local folder: {}", folder.display());
        } else {
            info!("🔥 STEP 5 DEBUG: Project has no local folder set");
        }

        // Check if the project has a CloudFormation template loaded
        info!("🔥 STEP 6: Checking for CloudFormation template...");
        let template = match project.cfn_template.as_ref() {
            Some(tmpl) => {
                info!("🔥 STEP 6 SUCCESS: CloudFormation template found");
                tmpl
            },
            None => {
                error!("🔥 STEP 6 FAILED: Project does not have a CloudFormation template loaded");
                error!("🔥 DEBUG: project.cfn_template is None");
                error!("🔥 DEBUG: Make sure to load/import a CloudFormation template in the project");
                return Err(ToolError::ExecutionFailed {
                    message: "The current project does not have a CloudFormation template loaded. Please import or create a CloudFormation template first.".to_string(),
                });
            }
        };

        info!("🔥 STEP 7: Template found, analyzing template structure...");
        info!("🔥 STEP 7 DEBUG: Template format version: {:?}", template.aws_template_format_version);
        info!("🔥 STEP 7 DEBUG: Template description: {:?}", template.description);
        info!("🔥 STEP 7 DEBUG: Resources count: {}", template.resources.len());
        info!("🔥 STEP 7 DEBUG: Parameters count: {}", template.parameters.len());
        info!("🔥 STEP 7 DEBUG: Outputs count: {}", template.outputs.len());
        
        if !template.resources.is_empty() {
            info!("🔥 STEP 7 DEBUG: First 5 resource names: {:?}", 
                template.resources.keys().take(5).collect::<Vec<_>>());
        }

        info!("🔥 STEP 8: Attempting to serialize template to JSON...");
        let template_json = match serde_json::to_value(template) {
            Ok(json) => {
                info!("🔥 STEP 8 SUCCESS: Template serialized to JSON successfully");
                info!("🔥 STEP 8 DEBUG: JSON size: {} bytes", json.to_string().len());
                json
            },
            Err(e) => {
                error!("🔥 STEP 8 FAILED: Failed to serialize CloudFormation template: {}", e);
                error!("🔥 DEBUG: Serialization error details: {:?}", e);
                return Err(ToolError::ExecutionFailed {
                    message: "Failed to convert CloudFormation template to JSON".to_string(),
                });
            }
        };

        // Get some basic statistics about the template
        let resource_count = template.resources.len();
        let parameter_count = template.parameters.len();
        let output_count = template.outputs.len();
        
        info!("🔥 STEP 9: Creating response object...");
        info!("🔥 STEP 9 DEBUG: Resource count: {}", resource_count);
        info!("🔥 STEP 9 DEBUG: Parameter count: {}", parameter_count);
        info!("🔥 STEP 9 DEBUG: Output count: {}", output_count);

        // Create response with template and metadata
        let response = serde_json::json!({
            "success": true,
            "project_name": project.name,
            "template": template_json,
            "statistics": {
                "resource_count": resource_count,
                "parameter_count": parameter_count,
                "output_count": output_count,
                "format_version": template.aws_template_format_version.as_ref().unwrap_or(&"2010-09-09".to_string())
            },
            "summary": format!(
                "CloudFormation template from project '{}' contains {} resources, {} parameters, and {} outputs",
                project.name, resource_count, parameter_count, output_count
            )
        });

        info!("🔥 STEP 10: Response created successfully");
        info!("🔥 STEP 10 DEBUG: Response size: {} bytes", response.to_string().len());
        
        info!("✅ 🔥 FINAL: ReadCloudFormationTemplate completed successfully: {} resources, {} parameters, {} outputs", 
            resource_count, parameter_count, output_count);

        Ok(ToolResult::success(response))
    }
}

impl Default for ReadCloudFormationTemplateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::projects::Project;
    use crate::app::cfn_template::CloudFormationTemplate;
    use crate::app::bridge::tools_registry::set_global_current_project;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_read_cloudformation_template_no_project() {
        let tool = ReadCloudFormationTemplateTool::new();
        
        // Clear global project
        set_global_current_project(None);

        let result = tool.execute(None, None).await;
        
        assert!(result.is_err());
        if let Err(ToolError::ExecutionFailed { message }) = result {
            assert!(message.contains("No project is currently open"));
        }
    }

    #[tokio::test]
    async fn test_read_cloudformation_template_no_template() {
        let tool = ReadCloudFormationTemplateTool::new();
        
        // Create a project without a template
        let mut project = Project::new(
            "Test Project".to_string(),
            "Test Description".to_string(),
            "test-project".to_string(),
        );
        project.cfn_template = None; // Ensure no template
        
        set_global_current_project(Some(Arc::new(Mutex::new(project))));

        let result = tool.execute(None, None).await;
        
        assert!(result.is_err());
        if let Err(ToolError::ExecutionFailed { message }) = result {
            assert!(message.contains("does not have a CloudFormation template"));
        }
    }

    #[tokio::test]
    async fn test_read_cloudformation_template_success() {
        let tool = ReadCloudFormationTemplateTool::new();
        
        // Create a project with a template
        let mut project = Project::new(
            "Test Project".to_string(),
            "Test Description".to_string(),
            "test-project".to_string(),
        );
        
        let mut template = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            resources: HashMap::new(),
            parameters: HashMap::new(),
            outputs: HashMap::new(),
            description: Some("Test template".to_string()),
            metadata: HashMap::new(),
            mappings: HashMap::new(),
            conditions: HashMap::new(),
            transform: None,
            rules: HashMap::new(),
        };
        
        // Add a test resource
        let test_resource = crate::app::cfn_template::Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            properties: Some(serde_json::json!({
                "BucketName": "test-bucket"
            })),
            depends_on: None,
            metadata: None,
            creation_policy: None,
            update_policy: None,
            deletion_policy: None,
            update_replace_policy: None,
            condition: None,
        };
        
        template.resources.insert("TestBucket".to_string(), test_resource);
        project.cfn_template = Some(template);
        
        set_global_current_project(Some(Arc::new(Mutex::new(project))));

        let result = tool.execute(None, None).await.unwrap();
        
        assert!(result.success);
        let response = result.content;
        assert_eq!(response["project_name"], "Test Project");
        assert_eq!(response["statistics"]["resource_count"], 1);
        assert_eq!(response["statistics"]["parameter_count"], 0);
        assert_eq!(response["statistics"]["output_count"], 0);
        assert!(response["template"]["Resources"]["TestBucket"]["Type"] == "AWS::S3::Bucket");
    }
}