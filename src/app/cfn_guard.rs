//! CloudFormation Guard integration for policy-as-code validation.
//!
//! This module provides integration with AWS CloudFormation Guard for validating
//! CloudFormation templates against compliance rules and security policies.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::cfn_template::CloudFormationTemplate;
use crate::app::guard_rules_registry::GuardRulesRegistry;

/// Main validator for CloudFormation Guard rules
#[derive(Debug)]
pub struct GuardValidator {
    /// Mapping of rule names to their content
    rules: HashMap<String, String>,
    /// Compliance programs enabled for validation
    compliance_programs: Vec<ComplianceProgram>,
    /// Rules registry client for downloading and caching rules
    registry: GuardRulesRegistry,
}

/// Result of Guard validation containing violations and summary
#[derive(Debug, Clone)]
pub struct GuardValidation {
    /// List of violations found during validation
    pub violations: Vec<GuardViolation>,
    /// Whether the template is compliant (no violations)
    pub compliant: bool,
    /// Total number of rules evaluated
    pub total_rules: usize,
}

/// Individual violation found by Guard validation
#[derive(Debug, Clone)]
pub struct GuardViolation {
    /// Name of the rule that was violated
    pub rule_name: String,
    /// Name of the resource that violates the rule
    pub resource_name: String,
    /// Human-readable description of the violation
    pub message: String,
    /// Severity level of the violation
    pub severity: ViolationSeverity,
}

/// Severity levels for Guard violations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Available compliance programs for Guard validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceProgram {
    NIST80053R4,
    NIST80053R5,
    NIST800171,
    PCIDSS,
    HIPAA,
    SOC,
    FedRAMP,
    Custom(String),
}

impl GuardValidator {
    /// Create a new Guard validator with the specified compliance programs
    pub async fn new(compliance_programs: Vec<ComplianceProgram>) -> Result<Self> {
        // Create cache directory for Guard rules
        let cache_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("awsdash")
            .join("guard_rules");
            
        let mut registry = GuardRulesRegistry::new(cache_dir).await?;
        let mut all_rules = HashMap::new();
        
        // Load rules for each compliance program
        for program in &compliance_programs {
            // Try to get cached rules first, fall back to downloading
            let rules = match registry.get_cached_rules(program.clone()).await {
                Ok(cached_rules) => cached_rules,
                Err(_) => {
                    // No cache, try to download
                    registry.download_compliance_rules(program.clone()).await
                        .unwrap_or_else(|_| HashMap::new())
                }
            };
            
            // Merge rules into the validator
            for (rule_name, rule_content) in rules {
                all_rules.insert(rule_name, rule_content);
            }
        }
        
        Ok(GuardValidator {
            rules: all_rules,
            compliance_programs,
            registry,
        })
    }

    /// Get the list of compliance programs enabled for this validator
    pub fn get_compliance_programs(&self) -> &[ComplianceProgram] {
        &self.compliance_programs
    }

    /// Validate a CloudFormation template against the loaded Guard rules
    pub async fn validate_template(&self, template: &CloudFormationTemplate) -> Result<GuardValidation> {
        // For now, return a placeholder implementation
        // TODO: Integrate with actual cfn-guard library
        
        // Convert template to JSON for Guard validation
        let _template_json = serde_json::to_string_pretty(template)?;
        
        // Placeholder: Check for common security issues
        let violations = self.check_placeholder_violations(template);
        let compliant = violations.is_empty();
        
        Ok(GuardValidation {
            violations,
            compliant,
            total_rules: self.rules.len(),
        })
    }

    /// Placeholder method for basic violation checking
    /// TODO: Replace with actual Guard validation
    fn check_placeholder_violations(&self, template: &CloudFormationTemplate) -> Vec<GuardViolation> {
        let mut violations = Vec::new();
        
        // Check for S3 buckets without encryption (placeholder rule)
        for (resource_name, resource) in &template.resources {
            if resource.resource_type == "AWS::S3::Bucket" {
                // Check if PublicReadPolicy is enabled (insecure)
                if let Some(public_read) = resource.properties.get("PublicReadPolicy") {
                    if public_read.as_bool() == Some(true) {
                        violations.push(GuardViolation {
                            rule_name: "S3_BUCKET_PUBLIC_READ_PROHIBITED".to_string(),
                            resource_name: resource_name.clone(),
                            message: "S3 bucket should not allow public read access".to_string(),
                            severity: ViolationSeverity::High,
                        });
                    }
                }
            }
        }
        
        violations
    }
}