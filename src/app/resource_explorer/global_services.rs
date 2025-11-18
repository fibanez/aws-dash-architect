use std::collections::HashSet;

/// Registry of AWS global services that operate across all regions
/// These services return the same data regardless of which region is queried
pub struct GlobalServiceRegistry {
    global_resource_types: HashSet<&'static str>,
}

impl Default for GlobalServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalServiceRegistry {
    pub fn new() -> Self {
        let mut registry = HashSet::new();
        
        // IAM Resources - Global service
        registry.insert("AWS::IAM::User");
        registry.insert("AWS::IAM::Role");
        registry.insert("AWS::IAM::Policy");
        registry.insert("AWS::IAM::Group");
        registry.insert("AWS::IAM::InstanceProfile");
        registry.insert("AWS::IAM::AccessKey");
        registry.insert("AWS::IAM::MFA");
        
        // Route53 - Global DNS service
        registry.insert("AWS::Route53::HostedZone");
        registry.insert("AWS::Route53::HealthCheck");
        registry.insert("AWS::Route53::Domain");
        
        // CloudFront - Global CDN service
        registry.insert("AWS::CloudFront::Distribution");
        registry.insert("AWS::CloudFront::StreamingDistribution");
        registry.insert("AWS::CloudFront::OriginAccessIdentity");
        
        // Organizations - Global account management
        registry.insert("AWS::Organizations::Organization");
        registry.insert("AWS::Organizations::OrganizationalUnit");
        registry.insert("AWS::Organizations::Account");
        registry.insert("AWS::Organizations::Policy");
        registry.insert("AWS::Organizations::Root");
        
        // Shield - Global DDoS protection
        registry.insert("AWS::Shield::Protection");
        registry.insert("AWS::Shield::Subscription");
        registry.insert("AWS::Shield::ProactiveEngagement");
        
        // WAF - Global web application firewall (v2 has regional options, but WAF Classic is global)
        registry.insert("AWS::WAF::WebACL");
        registry.insert("AWS::WAF::Rule");
        registry.insert("AWS::WAF::RuleGroup");
        registry.insert("AWS::WAF::IPSet");
        registry.insert("AWS::WAF::ByteMatchSet");
        registry.insert("AWS::WAF::SqlInjectionMatchSet");
        registry.insert("AWS::WAF::XssMatchSet");
        
        // Global Accelerator - Global network optimization
        registry.insert("AWS::GlobalAccelerator::Accelerator");
        registry.insert("AWS::GlobalAccelerator::Listener");
        registry.insert("AWS::GlobalAccelerator::EndpointGroup");
        
        // Support - Global support service
        registry.insert("AWS::Support::Case");
        registry.insert("AWS::Support::TrustedAdvisor");
        
        // Cost and Billing - Global billing service
        registry.insert("AWS::Billing::Budget");
        registry.insert("AWS::Billing::CostCategory");
        registry.insert("AWS::CostExplorer::Report");
        
        Self { global_resource_types: registry }
    }
    
    /// Check if a resource type is a global service
    pub fn is_global(&self, resource_type: &str) -> bool {
        self.global_resource_types.contains(resource_type)
    }
    
    /// Get the query region for global services (default: us-east-1)
    pub fn get_query_region(&self) -> &'static str {
        "us-east-1"
    }
}

/// Convenience function to check if a resource type is global
pub fn is_global_service(resource_type: &str) -> bool {
    GlobalServiceRegistry::new().is_global(resource_type)
}

/// Get the default region to query for global services
pub fn get_global_query_region() -> &'static str {
    GlobalServiceRegistry::new().get_query_region()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_service_detection() {
        let registry = GlobalServiceRegistry::new();
        
        // Test known global services
        assert!(registry.is_global("AWS::IAM::Role"));
        assert!(registry.is_global("AWS::IAM::User"));
        assert!(registry.is_global("AWS::Route53::HostedZone"));
        assert!(registry.is_global("AWS::CloudFront::Distribution"));
        assert!(registry.is_global("AWS::Organizations::Organization"));
        
        // Test regional services (should return false)
        assert!(!registry.is_global("AWS::EC2::Instance"));
        assert!(!registry.is_global("AWS::Lambda::Function"));
        assert!(!registry.is_global("AWS::S3::Bucket"));
        assert!(!registry.is_global("AWS::RDS::DBInstance"));
    }
    
    #[test]
    fn test_convenience_function() {
        assert!(is_global_service("AWS::IAM::Role"));
        assert!(!is_global_service("AWS::EC2::Instance"));
    }
    
    #[test]
    fn test_query_region() {
        let registry = GlobalServiceRegistry::new();
        assert_eq!(registry.get_query_region(), "us-east-1");
        assert_eq!(get_global_query_region(), "us-east-1");
    }
}