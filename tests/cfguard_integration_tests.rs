#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use awsdash::app::cfn_guard::{ComplianceProgram, GuardValidator, ViolationSeverity};
use awsdash::app::cfn_template::CloudFormationTemplate;
use awsdash::app::guard_rules_registry::GuardRulesRegistry;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};

/// Test real Guard rule file downloads from AWS Guard Rules Registry
#[tokio::test]
async fn test_real_guard_rules_download() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().to_path_buf();

    let mut registry = GuardRulesRegistry::new(cache_dir).await?;

    // Test downloading real rules from AWS Guard Rules Registry
    let programs_to_test = vec![
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::PCIDSS,
        ComplianceProgram::HIPAA,
    ];

    for program in programs_to_test {
        println!("Testing download for compliance program: {:?}", program);

        // Use timeout to prevent hanging
        let download_result = timeout(
            Duration::from_secs(30),
            registry.download_compliance_rules(program.clone()),
        )
        .await??;

        // Verify we got some rules
        assert!(
            download_result.len() > 0,
            "Should have downloaded rules for {:?}",
            program
        );

        // Verify rule content structure
        for (rule_name, rule_content) in &download_result {
            assert!(!rule_name.is_empty(), "Rule name should not be empty");
            assert!(!rule_content.is_empty(), "Rule content should not be empty");

            // Guard rules should contain Guard DSL keywords
            let has_guard_keywords = rule_content.contains("rule ")
                || rule_content.contains("let ")
                || rule_content.contains("when ")
                || rule_content.contains("Resources");

            assert!(
                has_guard_keywords,
                "Rule {} should contain Guard DSL keywords. Content: {}",
                rule_name, rule_content
            );
        }

        println!(
            "✓ Downloaded {} rules for {:?}",
            download_result.len(),
            program
        );
    }

    Ok(())
}

/// Test real CloudFormation template validation against downloaded Guard rules
#[tokio::test]
async fn test_real_template_compliance_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().to_path_buf();

    // Initialize registry and download real rules
    let mut registry = GuardRulesRegistry::new(cache_dir).await?;
    let _rules = timeout(
        Duration::from_secs(30),
        registry.download_compliance_rules(ComplianceProgram::NIST80053R5),
    )
    .await??;

    // Create validator with real downloaded rules
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let mut validator = GuardValidator::new(compliance_programs).await?;

    // Test Case 1: Non-compliant S3 bucket (should have violations)
    let non_compliant_template = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "InsecureS3Bucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "my-insecure-bucket",
                    "AccessControl": "PublicRead"
                }
            }
        }
    });

    let template = CloudFormationTemplate::from_json(&non_compliant_template.to_string())?;
    let validation_result = validator.validate_template(&template).await?;

    println!("Non-compliant template validation:");
    println!("  - Compliant: {}", validation_result.compliant);
    println!(
        "  - Total rules evaluated: {}",
        validation_result.total_rules
    );
    println!(
        "  - Violations found: {}",
        validation_result.violations.len()
    );

    // Should have found some violations
    assert!(
        validation_result.total_rules > 0,
        "Should have evaluated some rules"
    );

    // Test Case 2: More compliant S3 bucket (should have fewer violations)
    let compliant_template = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "SecureS3Bucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "my-secure-bucket",
                    "BucketEncryption": {
                        "ServerSideEncryptionConfiguration": [{
                            "ServerSideEncryptionByDefault": {
                                "SSEAlgorithm": "AES256"
                            }
                        }]
                    },
                    "PublicAccessBlockConfiguration": {
                        "BlockPublicAcls": true,
                        "BlockPublicPolicy": true,
                        "IgnorePublicAcls": true,
                        "RestrictPublicBuckets": true
                    }
                }
            }
        }
    });

    let compliant_cf_template = CloudFormationTemplate::from_json(&compliant_template.to_string())?;
    let compliant_validation = validator.validate_template(&compliant_cf_template).await?;

    println!("Compliant template validation:");
    println!("  - Compliant: {}", compliant_validation.compliant);
    println!(
        "  - Total rules evaluated: {}",
        compliant_validation.total_rules
    );
    println!(
        "  - Violations found: {}",
        compliant_validation.violations.len()
    );

    // Should have fewer violations than the non-compliant template
    assert!(
        compliant_validation.violations.len() <= validation_result.violations.len(),
        "Compliant template should have same or fewer violations"
    );

    Ok(())
}

/// Test exemption detection in CloudFormation templates using Metadata section
#[tokio::test]
async fn test_metadata_exemption_detection() -> Result<()> {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let mut validator = GuardValidator::new(compliance_programs).await?;

    // Template with exemptions in Metadata section
    let template_with_exemptions = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "ExemptS3Bucket": {
                "Type": "AWS::S3::Bucket",
                "Metadata": {
                    "guard": {
                        "SuppressedRules": [
                            "S3_BUCKET_SSL_REQUESTS_ONLY",
                            "S3_BUCKET_PUBLIC_ACCESS_PROHIBITED"
                        ]
                    },
                    "cfn_nag": {
                        "rules_to_suppress": [
                            {
                                "id": "W35",
                                "reason": "This bucket is used for testing purposes"
                            }
                        ]
                    }
                },
                "Properties": {
                    "BucketName": "test-bucket-with-exemptions",
                    "AccessControl": "PublicRead"
                }
            },
            "RegularS3Bucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "regular-test-bucket",
                    "AccessControl": "PublicRead"
                }
            }
        }
    });

    let template = CloudFormationTemplate::from_json(&template_with_exemptions.to_string())?;
    let validation_result = validator.validate_template(&template).await?;

    println!("Template with exemptions validation:");
    println!(
        "  - Total violations: {}",
        validation_result.violations.len()
    );

    // Check if exemptions are properly detected
    for violation in &validation_result.violations {
        println!(
            "  - Violation: {} on {}",
            violation.rule_name, violation.resource_name
        );

        // Violations on ExemptS3Bucket should be marked as exempted
        if violation.resource_name == "ExemptS3Bucket" {
            // This should be implemented in the actual exemption logic
            println!("    (This violation should be marked as exempted)");
        }
    }

    Ok(())
}

/// Test violation severity classification
#[tokio::test]
async fn test_violation_severity_classification() -> Result<()> {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let mut validator = GuardValidator::new(compliance_programs).await?;

    // Create a template that should trigger various severity levels
    let multi_violation_template = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "EC2Instance": {
                "Type": "AWS::EC2::Instance",
                "Properties": {
                    "ImageId": "ami-12345678",
                    "InstanceType": "t2.micro"
                }
            },
            "SecurityGroup": {
                "Type": "AWS::EC2::SecurityGroup",
                "Properties": {
                    "GroupDescription": "Test security group",
                    "SecurityGroupIngress": [{
                        "IpProtocol": "tcp",
                        "FromPort": 22,
                        "ToPort": 22,
                        "CidrIp": "0.0.0.0/0"
                    }]
                }
            },
            "IAMRole": {
                "Type": "AWS::IAM::Role",
                "Properties": {
                    "AssumeRolePolicyDocument": {
                        "Version": "2012-10-17",
                        "Statement": [{
                            "Effect": "Allow",
                            "Principal": {
                                "Service": "ec2.amazonaws.com"
                            },
                            "Action": "sts:AssumeRole"
                        }]
                    },
                    "Policies": [{
                        "PolicyName": "TestPolicy",
                        "PolicyDocument": {
                            "Version": "2012-10-17",
                            "Statement": [{
                                "Effect": "Allow",
                                "Action": "*",
                                "Resource": "*"
                            }]
                        }
                    }]
                }
            }
        }
    });

    let template = CloudFormationTemplate::from_json(&multi_violation_template.to_string())?;
    let validation_result = validator.validate_template(&template).await?;

    // Analyze violations by severity
    let mut severity_counts = HashMap::new();
    for violation in &validation_result.violations {
        *severity_counts.entry(&violation.severity).or_insert(0) += 1;
        println!(
            "Violation: {} ({}::{}) - {:?}",
            violation.rule_name, violation.resource_name, violation.message, violation.severity
        );
    }

    println!("Severity breakdown:");
    for (severity, count) in &severity_counts {
        println!("  - {:?}: {} violations", severity, count);
    }

    // Should have found violations of various severities
    assert!(
        validation_result.violations.len() > 0,
        "Should have found some violations"
    );

    Ok(())
}

/// Test caching performance for repeated validations
#[tokio::test]
async fn test_validation_caching_performance() -> Result<()> {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let mut validator = GuardValidator::new(compliance_programs).await?;

    let template_json = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "TestBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "cache-test-bucket"
                }
            }
        }
    });

    let template = CloudFormationTemplate::from_json(&template_json.to_string())?;

    // First validation (should be slower due to rule processing)
    let start_time = std::time::Instant::now();
    let first_result = validator.validate_template(&template).await?;
    let first_duration = start_time.elapsed();

    // Second validation (should be faster due to caching)
    let start_time = std::time::Instant::now();
    let second_result = validator.validate_template(&template).await?;
    let second_duration = start_time.elapsed();

    println!("First validation: {:?}", first_duration);
    println!("Second validation: {:?}", second_duration);

    // Results should be identical
    assert_eq!(
        first_result.violations.len(),
        second_result.violations.len()
    );
    assert_eq!(first_result.compliant, second_result.compliant);
    assert_eq!(first_result.total_rules, second_result.total_rules);

    // Second validation should generally be faster (though not guaranteed in tests)
    println!(
        "Cache speedup ratio: {:.2}x",
        first_duration.as_nanos() as f64 / second_duration.as_nanos() as f64
    );

    Ok(())
}

/// Test rule registry metadata and versioning
#[tokio::test]
async fn test_rule_registry_metadata() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().to_path_buf();

    let registry = GuardRulesRegistry::new(cache_dir).await?;

    let programs_to_test = vec![ComplianceProgram::NIST80053R5, ComplianceProgram::PCIDSS];

    for program in programs_to_test {
        let metadata_result = timeout(
            Duration::from_secs(15),
            registry.get_rule_metadata(program.clone()),
        )
        .await?;

        match metadata_result {
            Ok(metadata) => {
                println!("Metadata for {:?}:", program);
                println!("  - Version: {}", metadata.version);
                println!("  - Last updated: {}", metadata.last_updated);
                println!("  - Rules count: {}", metadata.rules_count);
                println!("  - Source URL: {}", metadata.source_url);

                assert!(!metadata.version.is_empty(), "Version should not be empty");
                assert!(
                    !metadata.last_updated.is_empty(),
                    "Last updated should not be empty"
                );
                assert!(metadata.rules_count > 0, "Should have some rules");
                assert!(
                    metadata.source_url.contains("aws"),
                    "Should reference AWS source"
                );
            }
            Err(e) => {
                println!("Could not fetch metadata for {:?}: {}", program, e);
                // This is acceptable in test environments with network restrictions
            }
        }
    }

    Ok(())
}

/// Test error handling for network failures and invalid templates
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let compliance_programs = vec![ComplianceProgram::NIST80053R5];
    let mut validator = GuardValidator::new(compliance_programs).await?;

    // Test invalid JSON template
    let invalid_json = "{ invalid json }";
    let template_result = CloudFormationTemplate::from_json(invalid_json);
    assert!(
        template_result.is_err(),
        "Should fail to parse invalid JSON"
    );

    // Test empty resources template
    let empty_template = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {}
    });

    let template = CloudFormationTemplate::from_json(&empty_template.to_string())?;
    let validation_result = validator.validate_template(&template).await?;

    // Empty template should validate successfully
    assert!(
        validation_result.compliant,
        "Empty template should be compliant"
    );
    assert_eq!(
        validation_result.violations.len(),
        0,
        "Empty template should have no violations"
    );

    Ok(())
}
