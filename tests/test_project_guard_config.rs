use anyhow::Result;
use awsdash::app::cfn_guard::ComplianceProgram;
use awsdash::app::projects::{AwsAccount, AwsRegion, Environment, Project};
use serde_json;
use std::collections::HashMap;
use tempfile::TempDir;

/// Test that Project can be created with default Guard configuration
#[test]
fn test_project_default_guard_config() {
    let project = Project::new(
        "TestProject".to_string(),
        "Test project for Guard config".to_string(),
        "testproj".to_string(),
    );

    // Should have default Guard configuration
    assert!(project.compliance_programs.is_empty()); // Default: no compliance programs
    assert!(project.guard_rules_enabled); // Default: enabled
    assert!(project.custom_guard_rules.is_empty()); // Default: no custom rules
    assert!(project.environment_compliance.is_empty()); // Default: no env-specific rules
}

/// Test setting compliance programs on a project
#[test]
fn test_project_set_compliance_programs() {
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project".to_string(),
        "testproj".to_string(),
    );

    // Set compliance programs
    project.compliance_programs = vec![
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::PCIDSS,
        ComplianceProgram::HIPAA,
    ];

    assert_eq!(project.compliance_programs.len(), 3);
    assert!(project
        .compliance_programs
        .contains(&ComplianceProgram::NIST80053R5));
    assert!(project
        .compliance_programs
        .contains(&ComplianceProgram::PCIDSS));
    assert!(project
        .compliance_programs
        .contains(&ComplianceProgram::HIPAA));
}

/// Test environment-specific compliance configuration
#[test]
fn test_environment_specific_compliance() {
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project".to_string(),
        "testproj".to_string(),
    );

    // Set different compliance programs for different environments
    let mut env_compliance = HashMap::new();
    env_compliance.insert("Dev".to_string(), vec![ComplianceProgram::NIST80053R5]);
    env_compliance.insert(
        "Prod".to_string(),
        vec![
            ComplianceProgram::NIST80053R5,
            ComplianceProgram::PCIDSS,
            ComplianceProgram::FedRAMP,
        ],
    );

    project.environment_compliance = env_compliance;

    // Verify Dev environment has less strict requirements
    let dev_compliance = project.environment_compliance.get("Dev").unwrap();
    assert_eq!(dev_compliance.len(), 1);
    assert!(dev_compliance.contains(&ComplianceProgram::NIST80053R5));

    // Verify Prod environment has more strict requirements
    let prod_compliance = project.environment_compliance.get("Prod").unwrap();
    assert_eq!(prod_compliance.len(), 3);
    assert!(prod_compliance.contains(&ComplianceProgram::FedRAMP));
}

/// Test custom Guard rules configuration
#[test]
fn test_custom_guard_rules() {
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project".to_string(),
        "testproj".to_string(),
    );

    // Add custom rule file paths
    project.custom_guard_rules = vec![
        "/path/to/custom/rule1.guard".to_string(),
        "/path/to/custom/rule2.guard".to_string(),
        "/path/to/custom/company-policy.guard".to_string(),
    ];

    assert_eq!(project.custom_guard_rules.len(), 3);
    assert!(project
        .custom_guard_rules
        .contains(&"/path/to/custom/company-policy.guard".to_string()));
}

/// Test Guard rules can be disabled
#[test]
fn test_guard_rules_disabled() {
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project".to_string(),
        "testproj".to_string(),
    );

    // Disable Guard validation
    project.guard_rules_enabled = false;

    assert!(!project.guard_rules_enabled);

    // Even with compliance programs set, should be disabled
    project.compliance_programs = vec![ComplianceProgram::NIST80053R5];
    assert!(!project.guard_rules_enabled);
}

/// Test project serialization with Guard configuration
#[test]
fn test_project_serialization_with_guard() {
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project with Guard config".to_string(),
        "testproj".to_string(),
    );

    // Configure Guard settings
    project.compliance_programs = vec![
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::Custom("MyCompanyPolicy".to_string()),
    ];
    project.guard_rules_enabled = true;
    project.custom_guard_rules = vec!["/custom/rule.guard".to_string()];

    let mut env_compliance = HashMap::new();
    env_compliance.insert("Prod".to_string(), vec![ComplianceProgram::FedRAMP]);
    project.environment_compliance = env_compliance;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&project).unwrap();

    // Verify Guard fields are present in JSON
    assert!(json.contains("compliance_programs"));
    assert!(json.contains("guard_rules_enabled"));
    assert!(json.contains("custom_guard_rules"));
    assert!(json.contains("environment_compliance"));
    assert!(json.contains("NIST80053R5"));
    assert!(json.contains("MyCompanyPolicy"));
    assert!(json.contains("FedRAMP"));
}

/// Test project deserialization with Guard configuration
#[test]
fn test_project_deserialization_with_guard() {
    let json = r#"{
        "name": "TestProject",
        "description": "Test project",
        "short_name": "testproj",
        "created": "2024-01-01T00:00:00Z",
        "updated": "2024-01-01T00:00:00Z",
        "local_folder": null,
        "git_url": null,
        "environments": [],
        "resources": [],
        "compliance_programs": [
            "NIST80053R5",
            "PCIDSS",
            {"Custom": "CompanyPolicy"}
        ],
        "guard_rules_enabled": true,
        "custom_guard_rules": [
            "/path/to/custom.guard"
        ],
        "environment_compliance": {
            "Prod": ["FedRAMP"]
        }
    }"#;

    let project: Project = serde_json::from_str(json).unwrap();

    assert_eq!(project.name, "TestProject");
    assert_eq!(project.compliance_programs.len(), 3);
    assert!(project.guard_rules_enabled);
    assert_eq!(project.custom_guard_rules.len(), 1);
    assert_eq!(project.environment_compliance.len(), 1);

    // Verify specific compliance program types
    assert!(project
        .compliance_programs
        .contains(&ComplianceProgram::NIST80053R5));
    assert!(project
        .compliance_programs
        .contains(&ComplianceProgram::PCIDSS));

    // Check custom compliance program
    let has_custom = project
        .compliance_programs
        .iter()
        .any(|p| matches!(p, ComplianceProgram::Custom(name) if name == "CompanyPolicy"));
    assert!(has_custom);
}

/// Test backward compatibility with existing projects (no Guard fields)
#[test]
fn test_backward_compatibility() {
    let json = r#"{
        "name": "LegacyProject",
        "description": "Legacy project without Guard config",
        "short_name": "legacy",
        "created": "2024-01-01T00:00:00Z",
        "updated": "2024-01-01T00:00:00Z",
        "local_folder": null,
        "git_url": null,
        "environments": [],
        "resources": []
    }"#;

    let project: Project = serde_json::from_str(json).unwrap();

    // Should use default values for Guard fields
    assert!(project.compliance_programs.is_empty());
    assert!(project.guard_rules_enabled); // Default: true
    assert!(project.custom_guard_rules.is_empty());
    assert!(project.environment_compliance.is_empty());
}

/// Test getting compliance programs for specific environment
#[test]
fn test_get_environment_compliance() {
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project".to_string(),
        "testproj".to_string(),
    );

    // Set global compliance programs
    project.compliance_programs = vec![ComplianceProgram::NIST80053R5];

    // Set environment-specific overrides
    let mut env_compliance = HashMap::new();
    env_compliance.insert(
        "Prod".to_string(),
        vec![ComplianceProgram::NIST80053R5, ComplianceProgram::FedRAMP],
    );
    project.environment_compliance = env_compliance;

    // Test getting compliance for environment with override
    let prod_compliance = project.get_compliance_programs_for_environment("Prod");
    assert_eq!(prod_compliance.len(), 2);
    assert!(prod_compliance.contains(&ComplianceProgram::FedRAMP));

    // Test getting compliance for environment without override (uses global)
    let dev_compliance = project.get_compliance_programs_for_environment("Dev");
    assert_eq!(dev_compliance.len(), 1);
    assert!(dev_compliance.contains(&ComplianceProgram::NIST80053R5));
    assert!(!dev_compliance.contains(&ComplianceProgram::FedRAMP));
}

/// Test project file persistence with Guard configuration
#[test]
fn test_project_file_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().join("test_project.json");

    // Create project with Guard configuration
    let mut project = Project::new(
        "TestProject".to_string(),
        "Test project for persistence".to_string(),
        "testproj".to_string(),
    );

    project.compliance_programs = vec![ComplianceProgram::NIST80053R5, ComplianceProgram::HIPAA];
    project.custom_guard_rules = vec!["/custom/hipaa-extra.guard".to_string()];

    // Save project to file
    project.save_to_path(&project_path)?;

    // Load project from file
    let loaded_project = Project::load_from_file(&project_path)?;

    // Verify Guard configuration was preserved
    assert_eq!(loaded_project.compliance_programs.len(), 2);
    assert!(loaded_project
        .compliance_programs
        .contains(&ComplianceProgram::HIPAA));
    assert_eq!(loaded_project.custom_guard_rules.len(), 1);
    assert_eq!(
        loaded_project.custom_guard_rules[0],
        "/custom/hipaa-extra.guard"
    );

    Ok(())
}
