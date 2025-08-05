use awsdash::app::cfn_guard::{
    GuardValidator, GuardValidation, GuardViolation, ViolationSeverity, ComplianceProgram
};
use awsdash::app::cfn_template::CloudFormationTemplate;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

/// Test that GuardValidator can be created with basic compliance programs
#[tokio::test]
async fn test_guard_validator_creation() {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let validator = GuardValidator::new(compliance_programs).await;
    
    assert!(validator.is_ok());
    let validator = validator.unwrap();
    assert_eq!(validator.get_compliance_programs().len(), 1);
}

/// Test that GuardValidator can validate a simple CloudFormation template
#[tokio::test]
async fn test_basic_template_validation() {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let validator = GuardValidator::new(compliance_programs).await.unwrap();
    
    // Create a simple S3 bucket template
    let template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "MyBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "my-test-bucket"
                }
            }
        }
    });
    
    let template = CloudFormationTemplate::from_json(&template_json.to_string()).unwrap();
    let result = validator.validate_template(&template).await;
    
    assert!(result.is_ok());
    let validation = result.unwrap();
    assert_eq!(validation.total_rules > 0);
}

/// Test that GuardValidator detects violations in non-compliant templates
#[tokio::test]
async fn test_violation_detection() {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let validator = GuardValidator::new(compliance_programs).await.unwrap();
    
    // Create a non-compliant S3 bucket (no encryption)
    let template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "InsecureBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "insecure-bucket",
                    "PublicReadPolicy": true
                }
            }
        }
    });
    
    let template = CloudFormationTemplate::from_json(&template_json.to_string()).unwrap();
    let result = validator.validate_template(&template).await;
    
    assert!(result.is_ok());
    let validation = result.unwrap();
    assert!(!validation.compliant);
    assert!(validation.violations.len() > 0);
}

/// Test GuardViolation structure and methods
#[test]
fn test_guard_violation_structure() {
    let violation = GuardViolation {
        rule_name: "S3_BUCKET_SSL_REQUESTS_ONLY".to_string(),
        resource_name: "MyBucket".to_string(),
        message: "S3 bucket should enforce SSL requests only".to_string(),
        severity: ViolationSeverity::High,
    };
    
    assert_eq!(violation.rule_name, "S3_BUCKET_SSL_REQUESTS_ONLY");
    assert_eq!(violation.resource_name, "MyBucket");
    assert_eq!(violation.severity, ViolationSeverity::High);
    assert!(violation.message.contains("SSL"));
}

/// Test ComplianceProgram enum variants
#[test]
fn test_compliance_program_variants() {
    let programs = vec![
        ComplianceProgram::NIST80053R4,
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::NIST800171,
        ComplianceProgram::PCIDSS,
        ComplianceProgram::HIPAA,
        ComplianceProgram::SOC,
        ComplianceProgram::FedRAMP,
        ComplianceProgram::Custom("MyCustomProgram".to_string()),
    ];
    
    assert_eq!(programs.len(), 8);
    
    // Test that Custom variant works
    if let ComplianceProgram::Custom(name) = &programs[7] {
        assert_eq!(name, "MyCustomProgram");
    } else {
        panic!("Expected Custom compliance program");
    }
}

/// Test ViolationSeverity enum and ordering
#[test]
fn test_violation_severity() {
    let critical = ViolationSeverity::Critical;
    let high = ViolationSeverity::High;
    let medium = ViolationSeverity::Medium;
    let low = ViolationSeverity::Low;
    
    // Test that we can create all severity levels
    assert_eq!(format!("{:?}", critical), "Critical");
    assert_eq!(format!("{:?}", high), "High");
    assert_eq!(format!("{:?}", medium), "Medium");
    assert_eq!(format!("{:?}", low), "Low");
}

/// Test GuardValidation aggregation methods
#[test]
fn test_guard_validation_aggregation() {
    let violations = vec![
        GuardViolation {
            rule_name: "RULE1".to_string(),
            resource_name: "Resource1".to_string(),
            message: "Message 1".to_string(),
            severity: ViolationSeverity::Critical,
        },
        GuardViolation {
            rule_name: "RULE2".to_string(),
            resource_name: "Resource2".to_string(),
            message: "Message 2".to_string(),
            severity: ViolationSeverity::Low,
        },
    ];
    
    let validation = GuardValidation {
        violations: violations.clone(),
        compliant: false,
        total_rules: 10,
    };
    
    assert!(!validation.compliant);
    assert_eq!(validation.violations.len(), 2);
    assert_eq!(validation.total_rules, 10);
    
    // Test violation severity groupings
    let critical_violations: Vec<_> = validation.violations.iter()
        .filter(|v| matches!(v.severity, ViolationSeverity::Critical))
        .collect();
    assert_eq!(critical_violations.len(), 1);
}

/// Test that GuardValidator handles empty templates gracefully
#[tokio::test]
async fn test_empty_template_validation() {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let validator = GuardValidator::new(compliance_programs).await.unwrap();
    
    let empty_template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {}
    });
    
    let template = CloudFormationTemplate::from_json(&empty_template_json.to_string()).unwrap();
    let result = validator.validate_template(&template).await;
    
    assert!(result.is_ok());
    let validation = result.unwrap();
    assert!(validation.compliant); // Empty template should be compliant
    assert_eq!(validation.violations.len(), 0);
}

/// Test multiple compliance programs integration
#[tokio::test]
async fn test_multiple_compliance_programs() {
    let compliance_programs = vec![
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::PCIDSS,
    ];
    let validator = GuardValidator::new(compliance_programs).await.unwrap();
    
    assert_eq!(validator.get_compliance_programs().len(), 2);
    
    // Test validation with multiple programs
    let template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "TestBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "test-bucket"
                }
            }
        }
    });
    
    let template = CloudFormationTemplate::from_json(&template_json.to_string()).unwrap();
    let result = validator.validate_template(&template).await;
    
    assert!(result.is_ok());
    // Should have rules from both compliance programs
    let validation = result.unwrap();
    assert!(validation.total_rules > 0);
}