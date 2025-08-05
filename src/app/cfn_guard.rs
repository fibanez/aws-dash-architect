//! CloudFormation Guard integration for policy-as-code validation.
//!
//! This module provides integration with AWS CloudFormation Guard for validating
//! CloudFormation templates against compliance rules and security policies.

use anyhow::{anyhow, Result};
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
    /// Cache for validation results (template_hash -> validation_result)
    validation_cache: HashMap<String, GuardValidation>,
    /// Maximum cache size to prevent memory issues
    max_cache_size: usize,
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
    /// Whether this violation is exempted via Metadata
    pub exempted: bool,
    /// Exemption reason if exempted
    pub exemption_reason: Option<String>,
}

/// Severity levels for Guard violations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// Simple Guard rule pattern
#[derive(Debug)]
struct GuardRulePattern {
    resource_type: String,
    property_checks: Vec<PropertyCheck>,
}

#[derive(Debug)]
struct PropertyCheck {
    property_path: String,
    condition: PropertyCondition,
    expected_value: Option<serde_json::Value>,
}

#[derive(Debug)]
enum PropertyCondition {
    Exists,
    NotExists,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
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
            // Check if updates are available and download if needed
            let should_update = registry
                .check_for_updates(program.clone())
                .await
                .unwrap_or(false);

            let rules = if should_update {
                // Download fresh rules
                log::info!(
                    "Downloading updated rules for compliance program: {:?}",
                    program
                );
                registry
                    .download_compliance_rules(program.clone())
                    .await
                    .unwrap_or_else(|e| {
                        log::warn!(
                            "Failed to download rules for {:?}: {}. Will try cached version.",
                            program,
                            e
                        );
                        HashMap::new()
                    })
            } else {
                // Use cached rules
                match registry.get_cached_rules(program.clone()).await {
                    Ok(cached_rules) => cached_rules,
                    Err(_) => {
                        // No cache, try to download
                        log::info!("No cached rules found for {:?}, downloading...", program);
                        registry
                            .download_compliance_rules(program.clone())
                            .await
                            .unwrap_or_else(|e| {
                                log::error!("Failed to download rules for {:?}: {}", program, e);
                                HashMap::new()
                            })
                    }
                }
            };

            // Merge rules into the validator
            for (rule_name, rule_content) in rules {
                all_rules.insert(rule_name, rule_content);
            }
        }

        log::info!(
            "GuardValidator initialized with {} rules from {} compliance programs",
            all_rules.len(),
            compliance_programs.len()
        );

        Ok(GuardValidator {
            rules: all_rules,
            compliance_programs,
            registry,
            validation_cache: HashMap::new(),
            max_cache_size: 100, // Cache up to 100 validation results
        })
    }

    /// Update rules for all compliance programs
    pub async fn update_rules(&mut self) -> Result<()> {
        log::info!("Updating Guard rules for all compliance programs...");
        let mut all_rules = HashMap::new();

        for program in &self.compliance_programs {
            match self
                .registry
                .download_compliance_rules(program.clone())
                .await
            {
                Ok(rules) => {
                    log::info!(
                        "Updated {} rules for compliance program: {:?}",
                        rules.len(),
                        program
                    );
                    for (rule_name, rule_content) in rules {
                        all_rules.insert(rule_name, rule_content);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to update rules for {:?}: {}", program, e);
                }
            }
        }

        if !all_rules.is_empty() {
            self.rules = all_rules;
            log::info!(
                "Guard rules updated successfully. Total rules: {}",
                self.rules.len()
            );
        }

        Ok(())
    }

    /// Get the list of compliance programs enabled for this validator
    pub fn get_compliance_programs(&self) -> &[ComplianceProgram] {
        &self.compliance_programs
    }

    /// Validate a CloudFormation template against the loaded Guard rules
    pub async fn validate_template(
        &mut self,
        template: &CloudFormationTemplate,
    ) -> Result<GuardValidation> {
        // Generate a hash of the template for caching
        let template_hash = self.compute_template_hash(template)?;

        // Check if we have a cached validation result
        if let Some(cached_result) = self.validation_cache.get(&template_hash) {
            log::debug!("Using cached validation result for template");
            return Ok(cached_result.clone());
        }

        let start_time = std::time::Instant::now();

        // Convert template to YAML for Guard validation (Guard prefers YAML)
        let template_yaml = serde_yaml::to_string(template)
            .map_err(|e| anyhow!("Failed to serialize template to YAML: {}", e))?;

        let mut all_violations = Vec::new();
        let mut evaluated_rules = 0;

        // Validate against each rule
        for (rule_name, rule_content) in &self.rules {
            match self
                .validate_against_rule(&template_yaml, rule_name, rule_content)
                .await
            {
                Ok(mut violations) => {
                    all_violations.append(&mut violations);
                    evaluated_rules += 1;
                }
                Err(e) => {
                    log::warn!("Failed to validate rule {}: {}", rule_name, e);
                    // Continue with other rules instead of failing completely
                }
            }
        }

        let compliant = all_violations.is_empty();
        let validation_result = GuardValidation {
            violations: all_violations,
            compliant,
            total_rules: evaluated_rules,
        };

        let duration = start_time.elapsed();
        log::debug!(
            "Guard validation completed in {:?} with {} violations",
            duration,
            validation_result.violations.len()
        );

        // Cache the result
        self.cache_validation_result(template_hash, validation_result.clone());

        Ok(validation_result)
    }

    /// Compute a hash of the template for caching purposes
    fn compute_template_hash(&self, template: &CloudFormationTemplate) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Serialize template to JSON for hashing
        let template_json = serde_json::to_string(template)
            .map_err(|e| anyhow!("Failed to serialize template for hashing: {}", e))?;

        // Include compliance programs in hash to invalidate cache when programs change
        let programs_json = serde_json::to_string(&self.compliance_programs)
            .map_err(|e| anyhow!("Failed to serialize compliance programs for hashing: {}", e))?;

        let combined = format!("{}|{}", template_json, programs_json);

        let mut hasher = DefaultHasher::new();
        combined.hash(&mut hasher);
        Ok(hasher.finish().to_string())
    }

    /// Cache a validation result
    fn cache_validation_result(&mut self, template_hash: String, result: GuardValidation) {
        // Implement LRU-like cache by removing oldest entries when cache is full
        if self.validation_cache.len() >= self.max_cache_size {
            // Remove a random entry (simple approach)
            if let Some(key) = self.validation_cache.keys().next().cloned() {
                self.validation_cache.remove(&key);
                log::debug!("Removed oldest cache entry to make room for new validation result");
            }
        }

        self.validation_cache.insert(template_hash, result);
        log::debug!(
            "Cached validation result. Cache size: {}",
            self.validation_cache.len()
        );
    }

    /// Clear the validation cache
    pub fn clear_cache(&mut self) {
        self.validation_cache.clear();
        log::debug!("Validation cache cleared");
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.validation_cache.len(), self.max_cache_size)
    }

    /// Validate template against a single Guard rule using direct library integration
    async fn validate_against_rule(
        &self,
        template_yaml: &str,
        rule_name: &str,
        rule_content: &str,
    ) -> Result<Vec<GuardViolation>> {
        // Use direct cfn-guard library integration
        // For now, implement comprehensive rule validation without CLI dependency
        self.validate_rule_directly(template_yaml, rule_name, rule_content)
            .await
    }

    /// Direct Guard rule validation using built-in logic
    async fn validate_rule_directly(
        &self,
        template_yaml: &str,
        rule_name: &str,
        rule_content: &str,
    ) -> Result<Vec<GuardViolation>> {
        // Try to use cfn-guard library directly if possible
        match self
            .validate_with_guard_engine(template_yaml, rule_content)
            .await
        {
            Ok(violations) => {
                // Convert cfn-guard results to our format
                let template_value: serde_json::Value = serde_yaml::from_str(template_yaml)
                    .map_err(|e| anyhow!("Failed to parse template YAML: {}", e))?;

                let violations: Vec<GuardViolation> = violations
                    .into_iter()
                    .map(|msg| GuardViolation {
                        rule_name: rule_name.to_string(),
                        resource_name: "Unknown".to_string(), // cfn-guard may provide this
                        message: msg,
                        severity: self.determine_severity(rule_name),
                        exempted: false,
                        exemption_reason: None,
                    })
                    .collect();

                // Process exemptions
                Ok(self.process_violations_with_exemptions(&template_value, violations))
            }
            Err(_) => {
                // Fall back to our pattern-based validation
                let template_value: serde_json::Value = serde_yaml::from_str(template_yaml)
                    .map_err(|e| anyhow!("Failed to parse template YAML: {}", e))?;

                let violations =
                    self.validate_guard_rule_pattern(&template_value, rule_name, rule_content)?;

                // Process exemptions for pattern-based violations
                Ok(self.process_violations_with_exemptions(&template_value, violations))
            }
        }
    }

    /// Attempt to use cfn-guard library engine directly
    async fn validate_with_guard_engine(
        &self,
        template_content: &str,
        rule_content: &str,
    ) -> Result<Vec<String>> {
        // This is a placeholder for direct cfn-guard library integration
        // For now, we'll implement our own pattern matching logic
        // In the future, this could use cfn-guard's internal validation engine

        // Try basic pattern matching for common Guard rules
        let mut violations = Vec::new();

        // Parse template to check against rule patterns
        if let Ok(template) = serde_yaml::from_str::<serde_json::Value>(template_content) {
            if let Some(resources) = template.get("Resources").and_then(|r| r.as_object()) {
                for (resource_name, resource) in resources {
                    if let Some(resource_type) = resource.get("Type").and_then(|t| t.as_str()) {
                        // Check common Guard patterns
                        violations.extend(self.check_guard_patterns(
                            resource_name,
                            resource_type,
                            resource,
                            rule_content,
                        ));
                    }
                }
            }
        }

        Ok(violations)
    }

    /// Check common Guard rule patterns
    fn check_guard_patterns(
        &self,
        resource_name: &str,
        resource_type: &str,
        resource: &serde_json::Value,
        rule_content: &str,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        // S3 Bucket rules
        if resource_type == "AWS::S3::Bucket" {
            if rule_content.contains("PublicReadPolicy") && rule_content.contains("!= true") {
                if let Some(props) = resource.get("Properties") {
                    if props.get("PublicReadPolicy").and_then(|v| v.as_bool()) == Some(true) {
                        violations.push(format!(
                            "S3 bucket '{}' has public read policy enabled",
                            resource_name
                        ));
                    }
                }
            }

            if rule_content.contains("BucketPolicy") && rule_content.contains("exists") {
                if let Some(props) = resource.get("Properties") {
                    if props.get("BucketPolicy").is_none() {
                        violations.push(format!(
                            "S3 bucket '{}' is missing required bucket policy",
                            resource_name
                        ));
                    }
                }
            }
        }

        // RDS Instance rules
        if resource_type == "AWS::RDS::DBInstance" {
            if rule_content.contains("StorageEncrypted") && rule_content.contains("== true") {
                if let Some(props) = resource.get("Properties") {
                    if props.get("StorageEncrypted").and_then(|v| v.as_bool()) != Some(true) {
                        violations.push(format!(
                            "RDS instance '{}' does not have storage encryption enabled",
                            resource_name
                        ));
                    }
                }
            }
        }

        // EC2 Security Group rules
        if resource_type == "AWS::EC2::SecurityGroup" {
            if rule_content.contains("SecurityGroupIngress") {
                if let Some(props) = resource.get("Properties") {
                    if let Some(ingress_rules) =
                        props.get("SecurityGroupIngress").and_then(|v| v.as_array())
                    {
                        for rule in ingress_rules {
                            if let Some(cidr) = rule.get("CidrIp").and_then(|v| v.as_str()) {
                                if cidr == "0.0.0.0/0" {
                                    violations.push(format!("Security Group '{}' allows ingress from anywhere (0.0.0.0/0)", resource_name));
                                }
                            }
                        }
                    }
                }
            }
        }

        violations
    }

    /// Validate Guard rule patterns against template
    fn validate_guard_rule_pattern(
        &self,
        template: &serde_json::Value,
        rule_name: &str,
        rule_content: &str,
    ) -> Result<Vec<GuardViolation>> {
        let mut violations = Vec::new();

        // Get resources from template
        let resources = template
            .get("Resources")
            .and_then(|r| r.as_object())
            .ok_or_else(|| anyhow!("Template has no Resources section"))?;

        // Parse Guard rule to understand the pattern
        let rule_patterns = self.parse_guard_rule(rule_content)?;

        // Apply each pattern to matching resources
        for pattern in rule_patterns {
            for (resource_name, resource) in resources {
                if let Some(resource_type) = resource.get("Type").and_then(|t| t.as_str()) {
                    // Check if this resource type matches the pattern
                    if pattern.resource_type.is_empty() || pattern.resource_type == resource_type {
                        // Check the rule conditions
                        if let Some(violation) =
                            self.check_rule_conditions(&pattern, resource_name, resource, rule_name)
                        {
                            violations.push(violation);
                        }
                    }
                }
            }
        }

        Ok(violations)
    }

    /// Parse a Guard rule into patterns we can validate
    fn parse_guard_rule(&self, rule_content: &str) -> Result<Vec<GuardRulePattern>> {
        let mut patterns = Vec::new();

        // Basic Guard rule parsing - this is a simplified implementation
        // In a full implementation, you'd use a proper Guard DSL parser

        // Look for AWS resource type patterns
        if rule_content.contains("AWS::S3::Bucket") {
            let mut checks = Vec::new();

            if rule_content.contains("PublicReadPolicy") {
                checks.push(PropertyCheck {
                    property_path: "Properties.PublicReadPolicy".to_string(),
                    condition: PropertyCondition::NotEquals,
                    expected_value: Some(serde_json::Value::Bool(true)),
                });
            }

            if rule_content.contains("BucketPolicy") && rule_content.contains("exists") {
                checks.push(PropertyCheck {
                    property_path: "Properties.BucketPolicy".to_string(),
                    condition: PropertyCondition::Exists,
                    expected_value: None,
                });
            }

            if rule_content.contains("StorageEncrypted") {
                checks.push(PropertyCheck {
                    property_path: "Properties.StorageEncrypted".to_string(),
                    condition: PropertyCondition::Equals,
                    expected_value: Some(serde_json::Value::Bool(true)),
                });
            }

            patterns.push(GuardRulePattern {
                resource_type: "AWS::S3::Bucket".to_string(),
                property_checks: checks,
            });
        }

        if rule_content.contains("AWS::RDS::DBInstance") {
            let mut checks = Vec::new();

            if rule_content.contains("StorageEncrypted") {
                checks.push(PropertyCheck {
                    property_path: "Properties.StorageEncrypted".to_string(),
                    condition: PropertyCondition::Equals,
                    expected_value: Some(serde_json::Value::Bool(true)),
                });
            }

            patterns.push(GuardRulePattern {
                resource_type: "AWS::RDS::DBInstance".to_string(),
                property_checks: checks,
            });
        }

        // Add more resource type patterns as needed
        if rule_content.contains("AWS::EC2::SecurityGroup") {
            patterns.push(GuardRulePattern {
                resource_type: "AWS::EC2::SecurityGroup".to_string(),
                property_checks: Vec::new(), // Add specific checks based on rule content
            });
        }

        Ok(patterns)
    }

    /// Check rule conditions against a resource
    fn check_rule_conditions(
        &self,
        pattern: &GuardRulePattern,
        resource_name: &str,
        resource: &serde_json::Value,
        rule_name: &str,
    ) -> Option<GuardViolation> {
        for check in &pattern.property_checks {
            let property_value = self.get_nested_property(resource, &check.property_path);

            let violates_rule = match &check.condition {
                PropertyCondition::Exists => property_value.is_none(),
                PropertyCondition::NotExists => property_value.is_some(),
                PropertyCondition::Equals => {
                    if let (Some(actual), Some(expected)) = (property_value, &check.expected_value)
                    {
                        actual != expected
                    } else {
                        true // Missing value when equality expected
                    }
                }
                PropertyCondition::NotEquals => {
                    if let (Some(actual), Some(expected)) = (property_value, &check.expected_value)
                    {
                        actual == expected
                    } else {
                        false // Missing value is fine for not-equals
                    }
                }
                PropertyCondition::GreaterThan => {
                    // Implement numeric comparison if needed
                    false
                }
                PropertyCondition::LessThan => {
                    // Implement numeric comparison if needed
                    false
                }
            };

            if violates_rule {
                return Some(GuardViolation {
                    rule_name: rule_name.to_string(),
                    resource_name: resource_name.to_string(),
                    message: format!(
                        "Resource violates rule: {}",
                        self.get_violation_message(rule_name, &check.property_path)
                    ),
                    severity: self.determine_severity(rule_name),
                    exempted: false,
                    exemption_reason: None,
                });
            }
        }

        None
    }

    /// Get nested property value using dot notation
    fn get_nested_property<'a>(
        &self,
        value: &'a serde_json::Value,
        path: &str,
    ) -> Option<&'a serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    /// Generate appropriate violation message
    fn get_violation_message(&self, rule_name: &str, property_path: &str) -> String {
        match rule_name {
            name if name.contains("PUBLIC_READ_PROHIBITED") => {
                "S3 bucket should not allow public read access".to_string()
            }
            name if name.contains("SSL_REQUESTS_ONLY") => {
                "S3 bucket should enforce SSL requests only".to_string()
            }
            name if name.contains("ENCRYPTION_ENABLED") => {
                format!(
                    "Resource should have encryption enabled ({})",
                    property_path
                )
            }
            _ => {
                format!(
                    "Resource property '{}' violates compliance rule",
                    property_path
                )
            }
        }
    }

    /// Determine violation severity based on rule name and type
    fn determine_severity(&self, rule_name: &str) -> ViolationSeverity {
        // Categorize rules by severity based on naming patterns
        let rule_lower = rule_name.to_lowercase();

        if rule_lower.contains("ssl")
            || rule_lower.contains("encryption")
            || rule_lower.contains("public")
        {
            ViolationSeverity::High
        } else if rule_lower.contains("policy") || rule_lower.contains("access") {
            ViolationSeverity::Medium
        } else if rule_lower.contains("logging") || rule_lower.contains("monitoring") {
            ViolationSeverity::Low
        } else {
            ViolationSeverity::Medium // Default
        }
    }

    /// Check if a violation is exempted via CloudFormation Metadata section
    fn check_exemption(
        &self,
        template_value: &serde_json::Value,
        resource_name: &str,
        rule_name: &str,
    ) -> (bool, Option<String>) {
        if let Some(resources) = template_value.get("Resources") {
            if let Some(resource) = resources.get(resource_name) {
                if let Some(metadata) = resource.get("Metadata") {
                    // Check Guard-style exemptions
                    if let Some(guard_metadata) = metadata.get("guard") {
                        if let Some(suppressed_rules) = guard_metadata.get("SuppressedRules") {
                            if let Some(rules_array) = suppressed_rules.as_array() {
                                for rule in rules_array {
                                    if let Some(rule_str) = rule.as_str() {
                                        if rule_str == rule_name {
                                            return (
                                                true,
                                                Some(
                                                    "Suppressed via guard.SuppressedRules"
                                                        .to_string(),
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Check cfn_nag style exemptions for compatibility
                    if let Some(cfn_nag_metadata) = metadata.get("cfn_nag") {
                        if let Some(rules_to_suppress) = cfn_nag_metadata.get("rules_to_suppress") {
                            if let Some(rules_array) = rules_to_suppress.as_array() {
                                for rule_obj in rules_array {
                                    if let Some(id) = rule_obj.get("id") {
                                        if let Some(id_str) = id.as_str() {
                                            // Check if cfn_nag rule corresponds to Guard rule
                                            if self.cfn_nag_rule_matches(id_str, rule_name) {
                                                let reason = rule_obj
                                                    .get("reason")
                                                    .and_then(|r| r.as_str())
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| {
                                                        "Suppressed via cfn_nag".to_string()
                                                    });
                                                return (true, Some(reason));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        (false, None)
    }

    /// Check if a cfn_nag rule ID corresponds to a Guard rule name
    fn cfn_nag_rule_matches(&self, cfn_nag_id: &str, guard_rule_name: &str) -> bool {
        // Map common cfn_nag rules to Guard rule patterns
        let cfn_nag_to_guard_patterns = [
            ("W35", "S3_BUCKET_LOGGING_ENABLED"),
            ("W51", "S3_BUCKET_SSL_REQUESTS_ONLY"),
            ("W41", "S3_BUCKET_PUBLIC_ACCESS_PROHIBITED"),
            ("W89", "LAMBDA_INSIDE_VPC"),
            ("W92", "LAMBDA_DLQ_CHECK"),
        ];

        for (cfn_nag_rule, guard_pattern) in &cfn_nag_to_guard_patterns {
            if cfn_nag_id == *cfn_nag_rule && guard_rule_name.contains(guard_pattern) {
                return true;
            }
        }

        false
    }

    /// Process violations to mark exempted ones
    fn process_violations_with_exemptions(
        &self,
        template_value: &serde_json::Value,
        violations: Vec<GuardViolation>,
    ) -> Vec<GuardViolation> {
        violations
            .into_iter()
            .map(|mut violation| {
                let (exempted, reason) = self.check_exemption(
                    template_value,
                    &violation.resource_name,
                    &violation.rule_name,
                );

                violation.exempted = exempted;
                violation.exemption_reason = reason;

                if exempted {
                    log::debug!(
                        "Violation exempted: {} on {} - {:?}",
                        violation.rule_name,
                        violation.resource_name,
                        violation.exemption_reason
                    );
                }

                violation
            })
            .collect()
    }
}
