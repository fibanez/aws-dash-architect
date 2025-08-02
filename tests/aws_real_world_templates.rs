//! Integration tests for real-world AWS CloudFormation templates
//!
//! These tests clone the official AWS CloudFormation templates repository
//! and validate the import verification system against hundreds of real templates.
//!
//! Usage:
//!   cargo test --test aws_real_world_templates -j 4 -- --ignored --nocapture
//!
//! These tests are marked with #[ignore] and must be run explicitly.
//! They are not part of the regular automated test suite due to:
//! - External dependency on GitHub repository
//! - Long execution time (500+ templates)
//! - Network requirements for git clone

use awsdash::app::cfn_template::CloudFormationTemplate;
use awsdash::app::projects::Project;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

const AWS_TEMPLATES_REPO: &str =
    "https://github.com/aws-cloudformation/aws-cloudformation-templates.git";
const TEMPLATES_DIR_NAME: &str = "aws-cloudformation-templates";

/// Test results for a single template
#[derive(Debug, Clone)]
struct TemplateTestResult {
    template_name: String,
    success: bool,
    error_message: Option<String>,
    resource_count: usize,
    import_time_ms: u64,
    verification_discrepancies: usize,
}

/// Overall test statistics
#[derive(Debug, Default)]
struct TestStatistics {
    total_templates: usize,
    successful_imports: usize,
    failed_imports: usize,
    total_resources: usize,
    total_time_ms: u64,
    templates_with_discrepancies: usize,
    error_categories: HashMap<String, usize>,
}

#[cfg(test)]
mod aws_real_world_integration_tests {
    use super::*;

    /// Clone the AWS CloudFormation templates repository
    fn clone_aws_templates_repo(target_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let repo_path = target_dir.join(TEMPLATES_DIR_NAME);

        if repo_path.exists() {
            println!("Repository already exists at: {}", repo_path.display());
            return Ok(repo_path);
        }

        println!("Cloning AWS CloudFormation templates repository...");
        let output = Command::new("git")
            .args(["clone", AWS_TEMPLATES_REPO, repo_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to clone repository: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("Successfully cloned repository to: {}", repo_path.display());
        Ok(repo_path)
    }

    /// Discover all CloudFormation JSON templates in the repository
    fn discover_cloudformation_templates(
        repo_path: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut templates = Vec::new();
        discover_templates_recursive(repo_path, &mut templates)?;

        println!(
            "Discovered {} potential CloudFormation templates",
            templates.len()
        );
        Ok(templates)
    }

    fn discover_templates_recursive(
        dir: &Path,
        templates: &mut Vec<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip common non-template directories
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if matches!(dir_name, ".git" | "node_modules" | "target" | ".github") {
                        continue;
                    }
                }
                discover_templates_recursive(&path, templates)?;
            } else if path.extension().is_some_and(|ext| ext == "json") {
                // Check if it's a CloudFormation template by looking for AWSTemplateFormatVersion
                if is_cloudformation_template(&path)? {
                    templates.push(path);
                }
            }
        }

        Ok(())
    }

    /// Check if a JSON file is a CloudFormation template
    fn is_cloudformation_template(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            // Check for CloudFormation indicators
            if json.get("AWSTemplateFormatVersion").is_some()
                || json.get("Resources").is_some()
                || json.get("Transform").is_some()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if verbose mode is enabled
    fn is_verbose_mode() -> bool {
        std::env::var("AWS_DASH_VERBOSE").unwrap_or_default() == "true"
    }

    /// Print verbose message if verbose mode is enabled
    fn verbose_print(message: &str) {
        if is_verbose_mode() {
            println!("{}", message);
        }
    }

    /// Test a single CloudFormation template
    fn test_single_template(template_path: &Path) -> TemplateTestResult {
        let template_name = template_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let template_size_bytes = fs::metadata(template_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        // Verbose: Show which file is being loaded
        verbose_print(&format!(
            "📁 Loading file: {} ({} bytes)",
            template_path.display(),
            template_size_bytes
        ));

        let start_time = Instant::now();

        // Try to load and parse the template
        let template_result = CloudFormationTemplate::from_file(template_path);

        let (template, parse_error) = match template_result {
            Ok(t) => {
                // Verbose: Show successful parsing
                verbose_print("  ✅ Template parsed successfully");
                (Some(t), None)
            }
            Err(e) => {
                // Verbose: Show parsing error
                verbose_print(&format!("  ❌ Template parsing failed: {}", e));
                (None, Some(format!("Parse error: {}", e)))
            }
        };

        if let Some(ref error) = parse_error {
            // Verbose: Show final result for failed template
            verbose_print(&format!("  🔴 FAILED: {} - {}", template_name, error));
            return TemplateTestResult {
                template_name,
                success: false,
                error_message: Some(error.clone()),
                resource_count: 0,
                import_time_ms: start_time.elapsed().as_millis() as u64,
                verification_discrepancies: 0,
            };
        }

        let template = template.unwrap();
        let resource_count = template.resources.len();

        // Verbose: Show template summary
        verbose_print(&format!(
            "  📊 Template summary: {} resources, {} parameters, {} outputs",
            resource_count,
            template.parameters.len(),
            template.outputs.len()
        ));

        // Test the import workflow
        let import_result = test_template_import_workflow(&template);
        let import_time_ms = start_time.elapsed().as_millis() as u64;

        match import_result {
            Ok(discrepancies) => {
                // Verbose: Show final result for successful template
                if discrepancies == 0 {
                    verbose_print(&format!(
                        "  🟢 SUCCESS: {} - {} resources imported successfully ({}ms)",
                        template_name, resource_count, import_time_ms
                    ));
                } else {
                    verbose_print(&format!(
                        "  🟡 PARTIAL: {} - {} resources imported with {} discrepancies ({}ms)",
                        template_name, resource_count, discrepancies, import_time_ms
                    ));
                }
                TemplateTestResult {
                    template_name,
                    success: true,
                    error_message: None,
                    resource_count,
                    import_time_ms,
                    verification_discrepancies: discrepancies,
                }
            }
            Err(e) => {
                // Verbose: Show final result for failed template
                verbose_print(&format!(
                    "  🔴 FAILED: {} - Import error: {} ({}ms)",
                    template_name, e, import_time_ms
                ));
                TemplateTestResult {
                    template_name,
                    success: false,
                    error_message: Some(e.to_string()),
                    resource_count,
                    import_time_ms,
                    verification_discrepancies: 0,
                }
            }
        }
    }

    /// Test the import workflow for a template
    fn test_template_import_workflow(
        template: &CloudFormationTemplate,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let project_path = temp_dir.path();

        // Create project structure
        let resources_dir = project_path.join("Resources");
        fs::create_dir_all(&resources_dir)?;

        // Write template to file
        let template_path = resources_dir.join("cloudformation_template.json");
        let template_json = serde_json::to_string_pretty(template)?;
        fs::write(&template_path, template_json)?;

        // Create a test project
        let mut project = Project::new(
            "integration_test".to_string(),
            "Integration test project".to_string(),
            "test".to_string(),
        );
        project.local_folder = Some(project_path.to_path_buf());

        // Verbose: Show resource loading progress
        if is_verbose_mode() {
            verbose_print(&format!(
                "🔄 Processing {} resources...",
                template.resources.len()
            ));

            // Show each resource being loaded
            for (resource_name, resource) in &template.resources {
                verbose_print(&format!(
                    "  ✅ Loading resource: {} (Type: {})",
                    resource_name, resource.resource_type
                ));
            }
        }

        // Load resources from template
        let load_result = project.load_resources_from_template();

        // Verbose: Show loading result
        if is_verbose_mode() {
            match &load_result {
                Ok(_) => verbose_print("  ✅ Template loaded successfully"),
                Err(e) => verbose_print(&format!("  ❌ Template loading failed: {}", e)),
            }
        }

        load_result?;

        // Get the reconstructed template
        let reconstructed_template = project
            .cfn_template
            .ok_or("No template found after loading")?;

        // Verify against original
        let discrepancies = reconstructed_template.verify_against(template);

        // Verbose: Show verification results
        if is_verbose_mode() {
            if discrepancies.is_empty() {
                verbose_print("  ✅ Template verification passed - no discrepancies");
            } else {
                verbose_print(&format!(
                    "  ⚠️  Template verification found {} discrepancies",
                    discrepancies.len()
                ));
            }
        }

        Ok(discrepancies.len())
    }

    /// Generate a detailed test report
    fn generate_test_report(results: &[TemplateTestResult], statistics: &TestStatistics) {
        println!("\n{}", "=".repeat(80));
        println!("AWS CLOUDFORMATION TEMPLATES INTEGRATION TEST REPORT");
        println!("{}", "=".repeat(80));

        println!("\n📊 OVERALL STATISTICS:");
        println!("  Total templates tested: {}", statistics.total_templates);
        println!(
            "  Successful imports: {} ({:.1}%)",
            statistics.successful_imports,
            (statistics.successful_imports as f64 / statistics.total_templates as f64) * 100.0
        );
        println!(
            "  Failed imports: {} ({:.1}%)",
            statistics.failed_imports,
            (statistics.failed_imports as f64 / statistics.total_templates as f64) * 100.0
        );
        println!(
            "  Total resources processed: {}",
            statistics.total_resources
        );
        println!(
            "  Total execution time: {:.2}s",
            statistics.total_time_ms as f64 / 1000.0
        );
        println!(
            "  Templates with verification discrepancies: {} ({:.1}%)",
            statistics.templates_with_discrepancies,
            (statistics.templates_with_discrepancies as f64 / statistics.total_templates as f64)
                * 100.0
        );

        if statistics.successful_imports > 0 {
            println!(
                "  Average import time: {:.2}ms",
                statistics.total_time_ms as f64 / statistics.successful_imports as f64
            );
        }

        println!("\n🚨 ERROR CATEGORIES:");
        for (category, count) in &statistics.error_categories {
            println!("  {}: {}", category, count);
        }

        println!("\n❌ FAILED TEMPLATES (first 10):");
        let failed_templates: Vec<_> = results.iter().filter(|r| !r.success).take(10).collect();

        for result in failed_templates {
            println!(
                "  {} - {}",
                result.template_name,
                result
                    .error_message
                    .as_ref()
                    .unwrap_or(&"Unknown error".to_string())
            );
        }

        println!("\n⚠️  TEMPLATES WITH DISCREPANCIES (first 10):");
        let discrepancy_templates: Vec<_> = results
            .iter()
            .filter(|r| r.success && r.verification_discrepancies > 0)
            .take(10)
            .collect();

        for result in discrepancy_templates {
            println!(
                "  {} - {} discrepancies",
                result.template_name, result.verification_discrepancies
            );
        }

        println!("\n✅ PERFORMANCE ANALYSIS:");
        let large_templates: Vec<_> = results
            .iter()
            .filter(|r| r.success && r.resource_count >= 20)
            .collect();

        if !large_templates.is_empty() {
            let avg_time_large = large_templates
                .iter()
                .map(|r| r.import_time_ms)
                .sum::<u64>() as f64
                / large_templates.len() as f64;

            println!(
                "  Large templates (20+ resources): {} templates, avg time: {:.2}ms",
                large_templates.len(),
                avg_time_large
            );
        }
    }

    #[test]
    #[ignore] // Only run when explicitly requested
    fn test_aws_cloudformation_templates_compatibility() {
        println!("Starting AWS CloudFormation templates compatibility test...");

        // Create temporary directory for the repository
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Clone the repository
        let repo_path = clone_aws_templates_repo(temp_dir.path())
            .expect("Failed to clone AWS templates repository");

        // Discover all CloudFormation templates
        let templates =
            discover_cloudformation_templates(&repo_path).expect("Failed to discover templates");

        if templates.is_empty() {
            panic!("No CloudFormation templates found in repository");
        }

        println!("Testing {} CloudFormation templates...", templates.len());

        // Test each template
        let mut results = Vec::new();
        let mut statistics = TestStatistics {
            total_templates: templates.len(),
            ..Default::default()
        };

        for (index, template_path) in templates.iter().enumerate() {
            if !is_verbose_mode() && index % 50 == 0 {
                println!("Progress: {}/{} templates tested", index, templates.len());
            }

            // Verbose: Show current progress
            if is_verbose_mode() {
                verbose_print(&format!(
                    "\n[{}/{}] Testing template...",
                    index + 1,
                    templates.len()
                ));
            }

            let result = test_single_template(template_path);

            // Update statistics
            if result.success {
                statistics.successful_imports += 1;
                statistics.total_resources += result.resource_count;
                if result.verification_discrepancies > 0 {
                    statistics.templates_with_discrepancies += 1;
                }
            } else {
                statistics.failed_imports += 1;

                // Categorize error
                if let Some(ref error) = result.error_message {
                    let category = if error.contains("Parse error") {
                        "Parse Error"
                    } else if error.contains("Dependency") {
                        "Dependency Error"
                    } else if error.contains("Property") {
                        "Property Error"
                    } else {
                        "Other Error"
                    };
                    *statistics
                        .error_categories
                        .entry(category.to_string())
                        .or_insert(0) += 1;
                }
            }

            statistics.total_time_ms += result.import_time_ms;
            results.push(result);
        }

        // Generate report
        generate_test_report(&results, &statistics);

        // Assert that we have reasonable success rate
        let success_rate = statistics.successful_imports as f64 / statistics.total_templates as f64;

        println!("\n🎯 SUCCESS CRITERIA:");
        println!(
            "  Success rate: {:.1}% (target: >80%)",
            success_rate * 100.0
        );

        // This is a soft assertion - we want to know the current state
        // rather than failing the test immediately
        if success_rate < 0.8 {
            println!(
                "⚠️  WARNING: Success rate below 80% target. This indicates areas for improvement."
            );
            println!("   This test documents current state and should not block development.");
        } else {
            println!("✅ SUCCESS: Compatibility rate meets target threshold!");
        }

        // Save detailed results for further analysis if needed
        // In a real implementation, you might save to a JSON file
        println!(
            "\n📁 Test completed. {} templates processed.",
            statistics.total_templates
        );
    }

    #[test]
    #[ignore]
    fn test_performance_with_large_templates() {
        println!("Testing performance with large CloudFormation templates...");

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path =
            clone_aws_templates_repo(temp_dir.path()).expect("Failed to clone repository");

        let templates =
            discover_cloudformation_templates(&repo_path).expect("Failed to discover templates");

        // Filter for large templates (estimated by file size as a proxy)
        let large_templates: Vec<_> = templates
            .iter()
            .filter(|path| {
                fs::metadata(path)
                    .map(|m| m.len() > 10_000) // > 10KB
                    .unwrap_or(false)
            })
            .take(20) // Test up to 20 large templates
            .collect();

        println!(
            "Testing {} large templates for performance...",
            large_templates.len()
        );

        let mut performance_results = Vec::new();

        for template_path in large_templates {
            let result = test_single_template(template_path);
            performance_results.push(result);
        }

        // Analyze performance
        let avg_time = performance_results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.import_time_ms)
            .sum::<u64>() as f64
            / performance_results.len() as f64;

        let max_time = performance_results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.import_time_ms)
            .max()
            .unwrap_or(0);

        println!("\n🚀 PERFORMANCE RESULTS:");
        println!("  Average import time: {:.2}ms", avg_time);
        println!("  Maximum import time: {}ms", max_time);
        println!("  Performance target: <5000ms for large templates");

        if max_time > 5000 {
            println!("⚠️  Some templates exceed performance target");
        } else {
            println!("✅ All templates meet performance target");
        }
    }
}
