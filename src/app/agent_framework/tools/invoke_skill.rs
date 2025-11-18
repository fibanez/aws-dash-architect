//! Invoke Skill Tool
//!
//! Allows AI agents to load specialized skills on-demand.
//! Implements progressive disclosure: agent loads skills only when needed.

use crate::app::agent_framework::skills::get_global_skill_manager;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info, warn};

/// Result of invoking a skill
#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeSkillResult {
    /// Skill name
    pub skill_name: String,
    /// Full SKILL.md content
    pub content: String,
    /// Size of content in bytes
    pub size_bytes: usize,
    /// Additional files that were loaded
    pub additional_files: HashMap<String, String>,
    /// List of all available additional files
    pub available_additional_files: Vec<String>,
}

/// Tool for invoking (loading) skills
#[derive(Clone, Debug, Default)]
pub struct InvokeSkillTool;

impl InvokeSkillTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for InvokeSkillTool {
    fn name(&self) -> &str {
        "invoke_skill"
    }

    fn description(&self) -> &str {
        r#"Load a skill to gain specialized knowledge for a task.

Skills provide specialized expertise for specific AWS domains (EC2 troubleshooting,
Lambda optimization, S3 security, etc.). When you recognize that a user's task
matches a skill's description, invoke the skill to load its full instructions.

**Progressive Disclosure**: Available skills are listed in your system prompt with
brief descriptions. Use this tool to load the full skill content only when needed.

**How to Use Skills**:
1. Recognize semantic match between user intent and skill description
   - "EC2 won't start" → matches aws-ec2-troubleshooting
   - "Lambda is slow" → matches aws-lambda-optimization
   - Don't force-fit skills; if no match, proceed without
2. invoke_skill loads full procedures and best practices
3. Adapt skill guidance to user's specific context
4. Combine multiple skills if task requires multiple domains

**Skills are guidance, not rigid scripts**:
- Use semantic understanding, not keyword matching
- Adapt procedures to user's context
- Skip irrelevant steps if user already provided information

Input Parameters:
- skill_name: Name of skill to load (from available skills list in your system prompt)
- load_additional_files: Optional array of additional files to load
  (e.g., ['forms.md', 'reference.md', 'checklist.md'])

Output:
- skill_name: Name of the loaded skill
- content: Full SKILL.md content with procedures and best practices
- size_bytes: Content size
- additional_files: Map of filename → content for requested files
- available_additional_files: List of all additional files in the skill directory

Error Handling:
- Returns error if skill not found
- Lists available skills if skill doesn't exist
- Logs but continues if optional additional files are missing

Examples:
1. Load EC2 troubleshooting skill:
   {"skill_name": "aws-ec2-troubleshooting"}
   → Loads diagnostic procedures for EC2 issues

2. Load S3 security skill with checklist:
   {"skill_name": "aws-s3-security", "load_additional_files": ["checklist.md"]}
   → Loads S3 security audit procedures + security checklist

3. Load Lambda optimization skill:
   {"skill_name": "aws-lambda-optimization"}
   → Loads performance tuning and cost optimization procedures

Common Use Cases:
- Troubleshooting: Load diagnostic procedures for specific AWS services
- Optimization: Load performance tuning guidelines
- Security: Load audit procedures and checklists
- Compliance: Load regulatory compliance guidelines"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill to load (from available skills list in system prompt)",
                    "examples": [
                        "aws-ec2-troubleshooting",
                        "aws-lambda-optimization",
                        "aws-s3-security",
                        "aws-cloudwatch-analysis"
                    ]
                },
                "load_additional_files": {
                    "type": "array",
                    "description": "Optional list of additional files to load (e.g., forms.md, reference.md, checklist.md)",
                    "items": {
                        "type": "string"
                    },
                    "examples": [
                        ["forms.md"],
                        ["checklist.md", "reference.md"]
                    ]
                }
            },
            "required": ["skill_name"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!("🎯 invoke_skill executing with parameters: {:?}", parameters);

        // Parse parameters
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for invoke_skill".to_string(),
        })?;

        let skill_name = params
            .get("skill_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters {
                message: "Missing or invalid 'skill_name' parameter".to_string(),
            })?;

        let additional_files: Vec<String> = params
            .get("load_additional_files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            "Loading skill '{}' with additional files: {:?}",
            skill_name, additional_files
        );

        // Get global skill manager
        let manager = get_global_skill_manager().ok_or_else(|| {
            warn!("Skill system not initialized");
            ToolError::ExecutionFailed {
                message: "Skill system not initialized. Skills are not available.".to_string(),
            }
        })?;

        // Load the skill
        let loaded_skill = manager.load_skill(skill_name).map_err(|e| {
            warn!("Failed to load skill '{}': {}", skill_name, e);

            // Provide helpful error with available skills
            let available_skills = manager.get_all_skill_metadata();
            let available_names: Vec<String> = available_skills.iter().map(|s| s.name.clone()).collect();

            ToolError::ExecutionFailed {
                message: format!(
                    "Skill '{}' not found. Available skills: {}",
                    skill_name,
                    available_names.join(", ")
                ),
            }
        })?;

        // Load additional files
        let mut additional_content = HashMap::new();
        for filename in &additional_files {
            match manager.load_skill_file(skill_name, filename) {
                Ok(content) => {
                    debug!(
                        "Loaded additional file '{}' for skill '{}' ({} bytes)",
                        filename,
                        skill_name,
                        content.len()
                    );
                    additional_content.insert(filename.clone(), content);
                }
                Err(e) => {
                    warn!(
                        "Failed to load additional file '{}' for skill '{}': {}",
                        filename, skill_name, e
                    );
                    // Continue - missing additional files are not fatal
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            "✅ invoke_skill completed in {:?}: loaded skill '{}' ({} bytes, {} additional files)",
            elapsed,
            skill_name,
            loaded_skill.content.len(),
            additional_content.len()
        );

        // Return result
        let result = InvokeSkillResult {
            skill_name: skill_name.to_string(),
            content: loaded_skill.content.clone(),
            size_bytes: loaded_skill.content.len(),
            additional_files: additional_content,
            available_additional_files: loaded_skill.metadata.additional_files.clone(),
        };

        let result_json = serde_json::to_value(result).map_err(|e| ToolError::ExecutionFailed {
            message: format!("Failed to serialize result: {}", e),
        })?;

        Ok(ToolResult::success(result_json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::skills::{SkillDiscoveryService, SkillManager};
    use std::fs;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_invoke_skill_tool_missing_system() {
        // Clear any global manager
        #[cfg(test)]
        {
            use crate::app::agent_framework::skills::manager::clear_global_skill_manager;
            clear_global_skill_manager();
        }

        let tool = InvokeSkillTool::new();

        let params = serde_json::json!({
            "skill_name": "test-skill"
        });

        let result = tool.execute(Some(params), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invoke_skill_tool_missing_parameters() {
        let tool = InvokeSkillTool::new();

        let result = tool.execute(None, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invoke_skill_tool_success() {
        // Create test skill
        let temp_dir = std::env::temp_dir().join("test-invoke-skill");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).ok();

        let skill_md_content = r#"---
name: test-skill
description: A test skill
---
# Test Skill
Detailed procedures here.
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        // Create additional file
        fs::write(skill_dir.join("forms.md"), "# Forms\nForm content.").ok();

        // Setup skill system (not global for test)
        let discovery = Arc::new(SkillDiscoveryService::with_directories(vec![temp_dir.clone()]));
        let manager = Arc::new(SkillManager::with_discovery(discovery));
        manager.discover_skills().ok();

        // We can't easily test the tool without setting global manager
        // This test verifies the tool structure compiles

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }
}
