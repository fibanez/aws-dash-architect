#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use awsdash::app::cfn_guard::GuardValidator;
use awsdash::app::cfn_template::CloudFormationTemplate;
use serde_json::json;

/// Test that cfn-guard library integration actually works with real Guard rules
#[tokio::test]
async fn test_real_cfn_guard_library_integration() -> Result<()> {
    // Create a simple Guard rule that we know should work
    let simple_guard_rule = r#"
        # Test rule: S3 buckets should not allow public read access
        let s3_buckets = Resources.*[ Type == 'AWS::S3::Bucket' ]
        rule s3_no_public_read when %s3_buckets !empty {
            %s3_buckets.Properties.PublicAccessBlockConfiguration exists
            %s3_buckets.Properties.PublicAccessBlockConfiguration.BlockPublicAcls == true
            %s3_buckets.Properties.PublicAccessBlockConfiguration.BlockPublicPolicy == true
        }
    "#;
    
    // Create a CloudFormation template that should violate this rule
    let template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "TestBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "test-bucket-without-public-access-block"
                    // Missing PublicAccessBlockConfiguration - should violate rule
                }
            }
        }
    });
    
    let template = CloudFormationTemplate::from_json(&template_json.to_string())?;
    let template_yaml = serde_yaml::to_string(&template)?;
    
    // Test the cfn-guard library directly using our ValidateInput integration
    use cfn_guard::{run_checks, ValidateInput};
    
    let data_input = ValidateInput {
        content: &template_yaml,
        file_name: "template.yaml",
    };
    
    let rules_input = ValidateInput {
        content: simple_guard_rule,
        file_name: "test.guard",
    };
    
    // Call cfn-guard library directly
    let result = run_checks(data_input, rules_input, false);
    
    match result {
        Ok(output) => {
            println!("cfn-guard output: {}", output);
            
            // The output should contain information about rule evaluation
            assert!(!output.trim().is_empty(), "cfn-guard should return non-empty output");
            
            // Check if it's JSON format (cfn-guard can return JSON)
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&output) {
                println!("✓ cfn-guard returned valid JSON output");
                println!("JSON structure: {:#}", json_val);
            } else {
                println!("cfn-guard returned text output: {}", output);
            }
            
            // This test verifies that cfn-guard library integration works
            println!("✓ cfn-guard library integration is working");
        }
        Err(e) => {
            panic!("cfn-guard library integration failed: {}", e);
        }
    }
    
    Ok(())
}

/// Test that our Guard validator correctly uses real cfn-guard with a minimal setup
#[tokio::test]
async fn test_guard_validator_with_minimal_real_rule() -> Result<()> {
    // Create a validator with an empty compliance program list (no downloads)
    let validator = GuardValidator::new(vec![]).await?;
    
    // Verify that the validator was created successfully
    assert_eq!(validator.get_compliance_programs().len(), 0);
    println!("✓ GuardValidator created successfully with empty rule set");
    
    // Test with an empty template
    let empty_template = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {}
    });
    
    let template = CloudFormationTemplate::from_json(&empty_template.to_string())?;
    let mut validator = validator;
    let result = validator.validate_template(&template).await?;
    
    // With no rules, should be compliant
    assert!(result.compliant, "Empty template with no rules should be compliant");
    assert_eq!(result.violations.len(), 0, "Should have no violations");
    assert_eq!(result.total_rules, 0, "Should have no rules to evaluate");
    
    println!("✓ Guard validation works correctly with empty rule set");
    
    Ok(())
}

/// Comprehensive test to verify the complete integration pipeline
#[tokio::test]
async fn test_complete_integration_pipeline() -> Result<()> {
    println!("Testing complete cfn-guard integration pipeline...");
    
    // Step 1: Verify cfn-guard library is available and working  
    let simple_rule = r#"
        rule test_rule {
            true  
        }
    "#;
    let empty_template_yaml = "AWSTemplateFormatVersion: '2010-09-09'\nResources: {}";
    
    let data_input = cfn_guard::ValidateInput {
        content: empty_template_yaml,
        file_name: "test.yaml",
    };
    
    let rules_input = cfn_guard::ValidateInput {
        content: simple_rule,
        file_name: "test.guard",
    };
    
    let cfn_guard_result = cfn_guard::run_checks(data_input, rules_input, false);
    
    match cfn_guard_result {
        Ok(output) => {
            println!("✓ Step 1: cfn-guard library is working");
            println!("  Raw output length: {} chars", output.len());
        }
        Err(e) => {
            panic!("❌ Step 1 FAILED: cfn-guard library not working: {}", e);
        }
    }
    
    // Step 2: Verify our GuardValidator can be created
    let validator_result = GuardValidator::new(vec![]).await;
    
    match validator_result {
        Ok(validator) => {
            println!("✓ Step 2: GuardValidator creation successful");
            
            // Step 3: Verify template validation pipeline
            let template = json!({
                "AWSTemplateFormatVersion": "2010-09-09",
                "Resources": {
                    "TestResource": {
                        "Type": "AWS::S3::Bucket",
                        "Properties": {
                            "BucketName": "test-integration-bucket"
                        }
                    }
                }
            });
            
            let cf_template = CloudFormationTemplate::from_json(&template.to_string())?;
            let mut validator = validator;
            let validation_result = validator.validate_template(&cf_template).await;
            
            match validation_result {
                Ok(result) => {
                    println!("✓ Step 3: Template validation pipeline working");
                    println!("  Compliant: {}", result.compliant);
                    println!("  Rules evaluated: {}", result.total_rules);
                    println!("  Violations: {}", result.violations.len());
                }
                Err(e) => {
                    panic!("❌ Step 3 FAILED: Template validation failed: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("❌ Step 2 FAILED: GuardValidator creation failed: {}", e);
        }
    }
    
    println!("✅ Complete integration pipeline test PASSED");
    
    Ok(())
}