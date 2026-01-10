//! Utility functions for agent framework

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Sanitize workspace name from LLM suggestion with collision detection
///
/// Converts to lowercase, replaces invalid chars with hyphens,
/// removes leading/trailing hyphens, limits length to 64 characters.
/// If the resulting workspace directory already exists, appends a counter
/// to make it unique (e.g., "my-tool-2", "my-tool-3").
///
/// # Examples
/// ```
/// use awsdash::app::agent_framework::utils::sanitize_workspace_name;
///
/// // Basic sanitization
/// assert_eq!(
///     sanitize_workspace_name("Lambda Dashboard").unwrap(),
///     "lambda-dashboard"
/// );
/// assert_eq!(
///     sanitize_workspace_name("S3 Bucket Explorer!").unwrap(),
///     "s3-bucket-explorer"
/// );
/// assert_eq!(
///     sanitize_workspace_name("my___tool___name").unwrap(),
///     "my-tool-name"
/// );
/// ```
pub fn sanitize_workspace_name(suggested: &str) -> Result<String> {
    // Basic sanitization
    let base_name = suggested
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(64)
        .collect::<String>();

    // Validate not empty
    if base_name.is_empty() {
        anyhow::bail!("Workspace name cannot be empty after sanitization");
    }

    // Get pages directory
    let pages_dir = dirs::data_local_dir()
        .context("Failed to get local data directory")?
        .join("awsdash/pages");

    // Check if base name is available
    let base_path = pages_dir.join(&base_name);
    if !base_path.exists() {
        return Ok(base_name);
    }

    // Find next available name with counter
    for counter in 2..=999 {
        let candidate_name = format!("{}-{}", base_name, counter);
        let candidate_path = pages_dir.join(&candidate_name);

        if !candidate_path.exists() {
            return Ok(candidate_name);
        }
    }

    // If we exhausted all counters, fail
    anyhow::bail!(
        "Could not find available workspace name for '{}' (tried up to {}-999)",
        base_name,
        base_name
    );
}

/// Get the workspace directory path for a given workspace name
///
/// Returns the absolute path to the workspace directory.
/// Does not create the directory - that should be done by file operation tools.
pub fn get_workspace_path(workspace_name: &str) -> Result<PathBuf> {
    let pages_dir = dirs::data_local_dir()
        .context("Failed to get local data directory")?
        .join("awsdash/pages");

    Ok(pages_dir.join(workspace_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sanitize_workspace_name_basic() {
        let result = sanitize_workspace_name("Lambda Dashboard").unwrap();
        assert_eq!(result, "lambda-dashboard");
    }

    #[test]
    fn test_sanitize_workspace_name_special_chars() {
        let result = sanitize_workspace_name("S3 Bucket Explorer!").unwrap();
        assert_eq!(result, "s3-bucket-explorer");
    }

    #[test]
    fn test_sanitize_workspace_name_consecutive_hyphens() {
        let result = sanitize_workspace_name("my___tool___name").unwrap();
        assert_eq!(result, "my-tool-name");
    }

    #[test]
    fn test_sanitize_workspace_name_leading_trailing() {
        let result = sanitize_workspace_name("  -tool-name-  ").unwrap();
        assert_eq!(result, "tool-name");
    }

    #[test]
    fn test_sanitize_workspace_name_unicode() {
        let result = sanitize_workspace_name("Dashboard™ & Metrics®").unwrap();
        assert_eq!(result, "dashboard-metrics");
    }

    #[test]
    fn test_sanitize_workspace_name_max_length() {
        // Create a name longer than 64 chars
        let long_name = "a".repeat(100);
        let result = sanitize_workspace_name(&long_name).unwrap();
        assert_eq!(result.len(), 64);
        assert_eq!(result, "a".repeat(64));
    }

    #[test]
    fn test_sanitize_workspace_name_empty() {
        let result = sanitize_workspace_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_sanitize_workspace_name_only_special_chars() {
        let result = sanitize_workspace_name("!!!---###");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_workspace_name_collision_detection() {
        // Create a temporary workspace to test collision detection
        let base_name = "test-collision-workspace";

        let pages_dir = dirs::data_local_dir()
            .unwrap()
            .join("awsdash/pages");

        // Create base directory
        let base_path = pages_dir.join(base_name);
        fs::create_dir_all(&base_path).unwrap();

        // First sanitization should return "test-collision-workspace-2"
        let result = sanitize_workspace_name(base_name).unwrap();
        assert_eq!(result, "test-collision-workspace-2");

        // Create the -2 directory
        let second_path = pages_dir.join("test-collision-workspace-2");
        fs::create_dir_all(&second_path).unwrap();

        // Second sanitization should return "test-collision-workspace-3"
        let result2 = sanitize_workspace_name(base_name).unwrap();
        assert_eq!(result2, "test-collision-workspace-3");

        // Cleanup
        fs::remove_dir_all(&base_path).ok();
        fs::remove_dir_all(&second_path).ok();
    }

    #[test]
    fn test_get_workspace_path() {
        let workspace_name = "my-page";
        let path = get_workspace_path(workspace_name).unwrap();

        let expected_suffix = if cfg!(target_os = "windows") {
            "awsdash\\pages\\my-page"
        } else {
            "awsdash/pages/my-page"
        };

        assert!(path.to_string_lossy().ends_with(expected_suffix));
    }

    #[test]
    fn test_sanitize_workspace_name_numbers() {
        let result = sanitize_workspace_name("Dashboard 2024").unwrap();
        assert_eq!(result, "dashboard-2024");
    }

    #[test]
    fn test_sanitize_workspace_name_mixed_case() {
        let result = sanitize_workspace_name("MyAwesomeTool").unwrap();
        assert_eq!(result, "myawesometool");
    }
}
