#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use awsdash::app::cfn_guard::GuardValidator;
use awsdash::app::cfn_template::CloudFormationTemplate;
use serde_json::json;

/// Test that UI components can handle real cfn-guard results
#[tokio::test]
async fn test_ui_with_real_guard_results() -> Result<()> {
    // Create a validator with no compliance programs (empty rule set)
    let mut validator = GuardValidator::new(vec![]).await?;
    
    // Create a simple CloudFormation template
    let template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "TestBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "test-ui-integration-bucket"
                }
            }
        }
    });
    
    let template = CloudFormationTemplate::from_json(&template_json.to_string())?;
    
    // Validate the template (should be compliant with no rules)
    let validation_result = validator.validate_template(&template).await?;
    
    println!("UI Integration Test Results:");
    println!("  - Compliant: {}", validation_result.compliant);
    println!("  - Total rules: {}", validation_result.total_rules);
    println!("  - Violations: {}", validation_result.violations.len());
    println!("  - Compliant rules: {}", validation_result.rule_results.compliant_rules.len());
    println!("  - Violation rules: {}", validation_result.rule_results.violation_rules.len());
    println!("  - Exempted rules: {}", validation_result.rule_results.exempted_rules.len());
    println!("  - Not applicable rules: {}", validation_result.rule_results.not_applicable_rules.len());
    
    // Verify the structure is correct for UI consumption
    assert!(validation_result.compliant, "Empty template with no rules should be compliant");
    assert_eq!(validation_result.violations.len(), 0, "Should have no violations");
    assert_eq!(validation_result.total_rules, 0, "Should have no rules to evaluate");
    
    // Verify rule_results structure is populated correctly
    assert_eq!(validation_result.rule_results.compliant_rules.len(), 0, "Should have no compliant rules");
    assert_eq!(validation_result.rule_results.violation_rules.len(), 0, "Should have no violation rules");
    assert_eq!(validation_result.rule_results.exempted_rules.len(), 0, "Should have no exempted rules");
    assert_eq!(validation_result.rule_results.not_applicable_rules.len(), 0, "Should have no not-applicable rules");
    
    println!("✅ UI integration structure is correct for empty rule set");
    
    // Test that violations window can be instantiated with this data
    use awsdash::app::dashui::guard_violations_window::GuardViolationsWindow;
    use awsdash::app::dashui::window_focus::FocusableWindow;
    
    let mut violations_window = GuardViolationsWindow::new();
    violations_window.show("test-template", validation_result);
    
    assert!(violations_window.is_open(), "Violations window should be open");
    
    println!("✅ GuardViolationsWindow accepts real cfn-guard results");
    
    Ok(())
}