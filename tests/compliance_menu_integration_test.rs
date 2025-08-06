use awsdash::app::cfn_guard::ComplianceProgram;
use awsdash::app::dashui::menu::{ComplianceStatus, MenuAction};
use awsdash::app::projects::Project;

#[test]
fn test_compliance_program_short_names() {
    assert_eq!(ComplianceProgram::NIST80053R5.short_name(), "NIST 800-53 R5");
    assert_eq!(ComplianceProgram::PCIDSS.short_name(), "PCI DSS");
    assert_eq!(ComplianceProgram::HIPAA.short_name(), "HIPAA");
    assert_eq!(ComplianceProgram::Custom("Custom Rule".to_string()).short_name(), "Custom Rule");
}

#[test]
fn test_project_with_compliance_programs() {
    let mut project = Project::default();
    
    // Add compliance programs
    project.compliance_programs = vec![
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::PCIDSS,
    ];
    project.guard_rules_enabled = true;
    
    // Verify compliance programs are stored
    assert_eq!(project.compliance_programs.len(), 2);
    assert!(project.guard_rules_enabled);
    
    // Test display names
    let display_names: Vec<&str> = project.compliance_programs
        .iter()
        .map(|p| p.short_name())
        .collect();
    assert_eq!(display_names, vec!["NIST 800-53 R5", "PCI DSS"]);
}

#[test]
fn test_compliance_status_values() {
    // Test different compliance status variants
    match ComplianceStatus::Compliant {
        ComplianceStatus::Compliant => assert!(true),
        _ => assert!(false, "Should match Compliant variant"),
    }
    
    match ComplianceStatus::Violations(5) {
        ComplianceStatus::Violations(count) => assert_eq!(count, 5),
        _ => assert!(false, "Should match Violations variant"),
    }
    
    match ComplianceStatus::Validating {
        ComplianceStatus::Validating => assert!(true),
        _ => assert!(false, "Should match Validating variant"),
    }
}

#[test]
fn test_menu_actions() {
    // Test that our new action exists
    match MenuAction::ValidateCompliance {
        MenuAction::ValidateCompliance => assert!(true),
        _ => assert!(false, "Should match ValidateCompliance action"),
    }
    
    match MenuAction::ShowComplianceDetails {
        MenuAction::ShowComplianceDetails => assert!(true),
        _ => assert!(false, "Should match ShowComplianceDetails action"),
    }
}