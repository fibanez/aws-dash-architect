#[cfg(test)]
mod global_services_tests {
    use awsdash::app::resource_explorer::global_services::{
        get_global_query_region, is_global_service, GlobalServiceRegistry,
    };

    #[test]
    fn test_iam_is_global() {
        assert!(is_global_service("AWS::IAM::Role"));
        assert!(is_global_service("AWS::IAM::User"));
        assert!(is_global_service("AWS::IAM::Policy"));
        assert!(is_global_service("AWS::IAM::Group"));
    }

    #[test]
    fn test_route53_is_global() {
        assert!(is_global_service("AWS::Route53::HostedZone"));
        assert!(is_global_service("AWS::Route53::HealthCheck"));
    }

    #[test]
    fn test_cloudfront_is_global() {
        assert!(is_global_service("AWS::CloudFront::Distribution"));
    }

    #[test]
    fn test_organizations_is_global() {
        assert!(is_global_service("AWS::Organizations::Organization"));
        assert!(is_global_service("AWS::Organizations::OrganizationalUnit"));
    }

    #[test]
    fn test_regional_services_are_not_global() {
        assert!(!is_global_service("AWS::EC2::Instance"));
        assert!(!is_global_service("AWS::Lambda::Function"));
        assert!(!is_global_service("AWS::S3::Bucket"));
        assert!(!is_global_service("AWS::RDS::DBInstance"));
        assert!(!is_global_service("AWS::DynamoDB::Table"));
        assert!(!is_global_service("AWS::ECS::Cluster"));
    }

    #[test]
    fn test_global_query_region() {
        assert_eq!(get_global_query_region(), "us-east-1");
    }

    #[test]
    fn test_registry_instance() {
        let registry = GlobalServiceRegistry::new();

        // Test a few key global services
        assert!(registry.is_global("AWS::IAM::Role"));
        assert!(registry.is_global("AWS::CloudFront::Distribution"));
        assert!(registry.is_global("AWS::Route53::HostedZone"));

        // Test regional services
        assert!(!registry.is_global("AWS::EC2::Instance"));
        assert!(!registry.is_global("AWS::Lambda::Function"));

        // Test query region
        assert_eq!(registry.get_query_region(), "us-east-1");
    }
}
