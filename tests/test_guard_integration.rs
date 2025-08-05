use awsdash::app::cfn_guard::{ComplianceProgram, GuardValidator};
use awsdash::app::cfn_template::CloudFormationTemplate;
use serde_json::json;

/// Test basic Guard integration with CloudFormationTemplate
#[tokio::test]
async fn test_guard_template_integration() {
    // Create a simple template for testing
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
    let validator = GuardValidator::new(vec![ComplianceProgram::NIST80053R5])
        .await
        .unwrap();

    // Test the integration
    let result = template.validate_with_guard(&validator).await;
    assert!(result.is_ok());

    let validation = result.unwrap();
    // Should have 0 total_rules since we haven't loaded actual rules yet
    assert_eq!(validation.total_rules, 0);
    assert!(validation.compliant); // Should be compliant with empty rules
}

/// Test Guard integration with violating template  
#[tokio::test]
async fn test_guard_template_with_violations() {
    // Create a template with a violation (public read policy)
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
    let validator = GuardValidator::new(vec![ComplianceProgram::NIST80053R5])
        .await
        .unwrap();

    // Test the integration
    let result = template.validate_with_guard(&validator).await;
    assert!(result.is_ok());

    let validation = result.unwrap();
    // Should detect the violation from our placeholder logic
    assert!(!validation.compliant);
    assert_eq!(validation.violations.len(), 1);
    assert_eq!(
        validation.violations[0].rule_name,
        "S3_BUCKET_PUBLIC_READ_PROHIBITED"
    );
}
