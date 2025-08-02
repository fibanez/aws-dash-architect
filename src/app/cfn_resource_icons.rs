use crate::log_trace;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tracing::warn;

/// Map of CloudFormation resource types to their corresponding icon paths
///
/// Icons are from the AWS Architecture Icons pack (https://aws.amazon.com/architecture/icons/)
/// Used under AWS's permitted usage terms for customers creating architecture diagrams
/// AWS Architecture Icons are provided by Amazon Web Services, Inc. and its affiliates
pub static RESOURCE_ICONS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // AccessAnalyzer Resources
    map.insert("AWS::AccessAnalyzer::Analyzer", "assets/Icons/Resource-Icons_02072025/Res_Networking-Content-Delivery/Res_Amazon-VPC_Network-Access-Analyzer_48.png");

    // AmazonMQ Resources
    map.insert("AWS::AmazonMQ::Broker", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-MQ_16.png");
    map.insert("AWS::AmazonMQ::Configuration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-MQ_16.png");
    map.insert("AWS::AmazonMQ::ConfigurationAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-MQ_16.png");

    // Amplify Resources
    map.insert("AWS::Amplify::App", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Front-End-Web-Mobile/16/Arch_AWS-Amplify_16.png");
    map.insert("AWS::Amplify::Branch", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Front-End-Web-Mobile/16/Arch_AWS-Amplify_16.png");
    map.insert("AWS::Amplify::Domain", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Front-End-Web-Mobile/16/Arch_AWS-Amplify_16.png");

    // ApiGateway Resources
    map.insert("AWS::ApiGateway::Account", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::ApiKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::Authorizer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::BasePathMapping", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::BasePathMappingV2", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::ClientCertificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::Deployment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::DocumentationPart", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::DocumentationVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::DomainName", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::DomainNameAccessAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::DomainNameV2", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::GatewayResponse", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::Method", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::Model", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::RequestValidator", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::Resource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::RestApi", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::Stage", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::UsagePlan", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::UsagePlanKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGateway::VpcLink", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");

    // ApiGatewayV2 Resources
    map.insert("AWS::ApiGatewayV2::Api", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::ApiGatewayManagedOverrides", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::ApiMapping", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::Authorizer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::Deployment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::DomainName", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::Integration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::IntegrationResponse", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::Model", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::Route", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::RouteResponse", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::Stage", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");
    map.insert("AWS::ApiGatewayV2::VpcLink", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-API-Gateway_16.png");

    // AppConfig Resources
    map.insert("AWS::AppConfig::Application", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::ConfigurationProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::Deployment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::DeploymentStrategy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::Environment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::Extension", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::ExtensionAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");
    map.insert("AWS::AppConfig::HostedConfigurationVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-AppConfig_16.png");

    // AppFlow Resources
    map.insert("AWS::AppFlow::Connector", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-AppFlow_16.png");
    map.insert("AWS::AppFlow::ConnectorProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-AppFlow_16.png");
    map.insert("AWS::AppFlow::Flow", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-AppFlow_16.png");

    // AppMesh Resources
    map.insert("AWS::AppMesh::GatewayRoute", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");
    map.insert("AWS::AppMesh::Mesh", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");
    map.insert("AWS::AppMesh::Route", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");
    map.insert("AWS::AppMesh::VirtualGateway", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");
    map.insert("AWS::AppMesh::VirtualNode", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");
    map.insert("AWS::AppMesh::VirtualRouter", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");
    map.insert("AWS::AppMesh::VirtualService", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-App-Mesh_16.png");

    // AppRunner Resources
    map.insert("AWS::AppRunner::AutoScalingConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-App-Runner_16.png");
    map.insert("AWS::AppRunner::ObservabilityConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-App-Runner_16.png");
    map.insert("AWS::AppRunner::Service", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-App-Runner_16.png");
    map.insert("AWS::AppRunner::VpcConnector", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-App-Runner_16.png");
    map.insert("AWS::AppRunner::VpcIngressConnection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-App-Runner_16.png");

    // AppSync Resources
    map.insert("AWS::AppSync::Api", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::ApiCache", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::ApiKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::ChannelNamespace", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::DataSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::DomainName", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::DomainNameApiAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::FunctionConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::GraphQLApi", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::GraphQLSchema", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::Resolver", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");
    map.insert("AWS::AppSync::SourceApiAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-AppSync_16.png");

    // ApplicationAutoScaling Resources
    map.insert("AWS::ApplicationAutoScaling::ScalableTarget", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Application-Auto-Scaling_16.png");
    map.insert("AWS::ApplicationAutoScaling::ScalingPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Application-Auto-Scaling_16.png");

    // Athena Resources
    map.insert("AWS::Athena::CapacityReservation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Athena_16.png");
    map.insert("AWS::Athena::DataCatalog", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Athena_16.png");
    map.insert("AWS::Athena::NamedQuery", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Athena_16.png");
    map.insert("AWS::Athena::PreparedStatement", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Athena_16.png");
    map.insert("AWS::Athena::WorkGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Athena_16.png");

    // AuditManager Resources
    map.insert("AWS::AuditManager::Assessment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Audit-Manager_16.png");

    // AutoScaling Resources
    map.insert("AWS::AutoScaling::AutoScalingGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Auto-Scaling_16.png");
    map.insert("AWS::AutoScaling::LaunchConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Auto-Scaling_16.png");
    map.insert("AWS::AutoScaling::LifecycleHook", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Auto-Scaling_16.png");
    map.insert("AWS::AutoScaling::ScalingPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Auto-Scaling_16.png");
    map.insert("AWS::AutoScaling::ScheduledAction", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Auto-Scaling_16.png");
    map.insert("AWS::AutoScaling::WarmPool", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Auto-Scaling_16.png");

    // Backup Resources
    map.insert(
        "AWS::Backup::BackupPlan",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::BackupSelection",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::BackupVault",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::Framework",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::LogicallyAirGappedBackupVault",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::ReportPlan",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::RestoreTestingPlan",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );
    map.insert(
        "AWS::Backup::RestoreTestingSelection",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_AWS-Backup_16.png",
    );

    // Batch Resources
    map.insert(
        "AWS::Batch::ComputeEnvironment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Batch_16.png",
    );
    map.insert(
        "AWS::Batch::ConsumableResource",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Batch_16.png",
    );
    map.insert(
        "AWS::Batch::JobDefinition",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Batch_16.png",
    );
    map.insert(
        "AWS::Batch::JobQueue",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Batch_16.png",
    );
    map.insert(
        "AWS::Batch::SchedulingPolicy",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Batch_16.png",
    );

    // Bedrock Resources
    map.insert("AWS::Bedrock::Agent", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::AgentAlias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::ApplicationInferenceProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::Blueprint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::DataAutomationProject", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::DataSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::Flow", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::FlowAlias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::FlowVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::Guardrail", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::GuardrailVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::KnowledgeBase", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::Prompt", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");
    map.insert("AWS::Bedrock::PromptVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Bedrock_16.png");

    // BillingConductor Resources
    map.insert("AWS::BillingConductor::BillingGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Cloud-Financial-Management/16/Arch_AWS-Billing-Conductor_16.png");
    map.insert("AWS::BillingConductor::CustomLineItem", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Cloud-Financial-Management/16/Arch_AWS-Billing-Conductor_16.png");
    map.insert("AWS::BillingConductor::PricingPlan", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Cloud-Financial-Management/16/Arch_AWS-Billing-Conductor_16.png");
    map.insert("AWS::BillingConductor::PricingRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Cloud-Financial-Management/16/Arch_AWS-Billing-Conductor_16.png");

    // Budgets Resources
    map.insert("AWS::Budgets::Budget", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Cloud-Financial-Management/16/Arch_AWS-Budgets_16.png");
    map.insert("AWS::Budgets::BudgetsAction", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Cloud-Financial-Management/16/Arch_AWS-Budgets_16.png");

    // CertificateManager Resources
    map.insert("AWS::CertificateManager::Account", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Certificate-Manager_16.png");
    map.insert("AWS::CertificateManager::Certificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Certificate-Manager_16.png");

    // Chatbot Resources
    map.insert("AWS::Chatbot::CustomAction", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Chatbot_16.png");
    map.insert("AWS::Chatbot::MicrosoftTeamsChannelConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Chatbot_16.png");
    map.insert("AWS::Chatbot::SlackChannelConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Chatbot_16.png");

    // CleanRooms Resources
    map.insert("AWS::CleanRooms::AnalysisTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::Collaboration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::ConfiguredTable", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::ConfiguredTableAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::IdMappingTable", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::IdNamespaceAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::Membership", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");
    map.insert("AWS::CleanRooms::PrivacyBudgetTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Clean-Rooms_16.png");

    // Cloud9 Resources
    map.insert("AWS::Cloud9::EnvironmentEC2", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-Cloud9_16.png");

    // CloudFormation Resources
    map.insert("AWS::CloudFormation::CustomResource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::GuardHook", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::HookDefaultVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::HookTypeConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::HookVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::LambdaHook", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::Macro", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::ModuleDefaultVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::ModuleVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::PublicTypeVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::Publisher", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::ResourceDefaultVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::ResourceVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::Stack", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::StackSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::TypeActivation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::WaitCondition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");
    map.insert("AWS::CloudFormation::WaitConditionHandle", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudFormation_16.png");

    // CloudFront Resources
    map.insert("AWS::CloudFront::AnycastIpList", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::CachePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::CloudFrontOriginAccessIdentity", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::ConnectionGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::ContinuousDeploymentPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::Distribution", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::DistributionTenant", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::Function", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::KeyGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::KeyValueStore", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::MonitoringSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::OriginAccessControl", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::OriginRequestPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::PublicKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::RealtimeLogConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::ResponseHeadersPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::StreamingDistribution", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");
    map.insert("AWS::CloudFront::VpcOrigin", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-CloudFront_16.png");

    // CloudTrail Resources
    map.insert("AWS::CloudTrail::Channel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudTrail_16.png");
    map.insert("AWS::CloudTrail::Dashboard", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudTrail_16.png");
    map.insert("AWS::CloudTrail::EventDataStore", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudTrail_16.png");
    map.insert("AWS::CloudTrail::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudTrail_16.png");
    map.insert("AWS::CloudTrail::Trail", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-CloudTrail_16.png");

    // CloudWatch Resources
    map.insert("AWS::CloudWatch::Alarm", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_Amazon-CloudWatch_16.png");
    map.insert("AWS::CloudWatch::AnomalyDetector", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_Amazon-CloudWatch_16.png");
    map.insert("AWS::CloudWatch::CompositeAlarm", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_Amazon-CloudWatch_16.png");
    map.insert("AWS::CloudWatch::Dashboard", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_Amazon-CloudWatch_16.png");
    map.insert("AWS::CloudWatch::InsightRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_Amazon-CloudWatch_16.png");
    map.insert("AWS::CloudWatch::MetricStream", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_Amazon-CloudWatch_16.png");

    // CodeArtifact Resources
    map.insert("AWS::CodeArtifact::Domain", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeArtifact_16.png");
    map.insert("AWS::CodeArtifact::PackageGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeArtifact_16.png");
    map.insert("AWS::CodeArtifact::Repository", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeArtifact_16.png");

    // CodeBuild Resources
    map.insert("AWS::CodeBuild::Fleet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeBuild_16.png");
    map.insert("AWS::CodeBuild::Project", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeBuild_16.png");
    map.insert("AWS::CodeBuild::ReportGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeBuild_16.png");
    map.insert("AWS::CodeBuild::SourceCredential", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeBuild_16.png");

    // CodeCommit Resources
    map.insert("AWS::CodeCommit::Repository", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeCommit_16.png");

    // CodeDeploy Resources
    map.insert("AWS::CodeDeploy::Application", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeDeploy_16.png");
    map.insert("AWS::CodeDeploy::DeploymentConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeDeploy_16.png");
    map.insert("AWS::CodeDeploy::DeploymentGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodeDeploy_16.png");

    // CodePipeline Resources
    map.insert("AWS::CodePipeline::CustomActionType", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodePipeline_16.png");
    map.insert("AWS::CodePipeline::Pipeline", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodePipeline_16.png");
    map.insert("AWS::CodePipeline::Webhook", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-CodePipeline_16.png");

    // Cognito Resources
    map.insert("AWS::Cognito::IdentityPool", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::IdentityPoolPrincipalTag", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::IdentityPoolRoleAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::LogDeliveryConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::ManagedLoginBranding", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPool", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolClient", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolDomain", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolIdentityProvider", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolResourceServer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolRiskConfigurationAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolUICustomizationAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolUser", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");
    map.insert("AWS::Cognito::UserPoolUserToGroupAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Cognito_16.png");

    // Comprehend Resources
    map.insert("AWS::Comprehend::DocumentClassifier", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Comprehend_16.png");
    map.insert("AWS::Comprehend::Flywheel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Comprehend_16.png");

    // Config Resources
    map.insert("AWS::Config::AggregationAuthorization", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::ConfigRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::ConfigurationAggregator", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::ConfigurationRecorder", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::ConformancePack", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::DeliveryChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::OrganizationConfigRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::OrganizationConformancePack", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::RemediationConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");
    map.insert("AWS::Config::StoredQuery", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Config_16.png");

    // Connect Resources
    map.insert("AWS::Connect::AgentStatus", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::ApprovedOrigin", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::ContactFlow", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::ContactFlowModule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::ContactFlowVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::EmailAddress", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::EvaluationForm", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::HoursOfOperation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::Instance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::InstanceStorageConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::IntegrationAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::PhoneNumber", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::PredefinedAttribute", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::Prompt", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::Queue", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::QuickConnect", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::RoutingProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::Rule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::SecurityKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::SecurityProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::TaskTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::TrafficDistributionGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::User", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::UserHierarchyGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::UserHierarchyStructure", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::View", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");
    map.insert("AWS::Connect::ViewVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Connect_16.png");

    // ControlTower Resources
    map.insert("AWS::ControlTower::EnabledBaseline", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Control-Tower_16.png");
    map.insert("AWS::ControlTower::EnabledControl", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Control-Tower_16.png");
    map.insert("AWS::ControlTower::LandingZone", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Control-Tower_16.png");

    // DMS Resources
    map.insert("AWS::DMS::Certificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::DataMigration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::DataProvider", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::Endpoint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::EventSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::InstanceProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::MigrationProject", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::ReplicationConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::ReplicationInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::ReplicationSubnetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");
    map.insert("AWS::DMS::ReplicationTask", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_AWS-Database-Migration-Service_16.png");

    // DataSync Resources
    map.insert("AWS::DataSync::Agent", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationAzureBlob", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationEFS", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationFSxLustre", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationFSxONTAP", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationFSxOpenZFS", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationFSxWindows", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationHDFS", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationNFS", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationObjectStorage", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationS3", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::LocationSMB", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::StorageSystem", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");
    map.insert("AWS::DataSync::Task", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Migration-Modernization/16/Arch_AWS-DataSync_16.png");

    // DataZone Resources
    map.insert("AWS::DataZone::Connection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::DataSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::Domain", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::Environment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::EnvironmentActions", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::EnvironmentBlueprintConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::EnvironmentProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::GroupProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::Project", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::ProjectMembership", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::SubscriptionTarget", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");
    map.insert("AWS::DataZone::UserProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-DataZone_16.png");

    // Detective Resources
    map.insert("AWS::Detective::Graph", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Detective_16.png");
    map.insert("AWS::Detective::MemberInvitation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Detective_16.png");
    map.insert("AWS::Detective::OrganizationAdmin", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Detective_16.png");

    // DevOpsGuru Resources
    map.insert("AWS::DevOpsGuru::LogAnomalyDetectionIntegration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-DevOps-Guru_16.png");
    map.insert("AWS::DevOpsGuru::NotificationChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-DevOps-Guru_16.png");
    map.insert("AWS::DevOpsGuru::ResourceCollection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-DevOps-Guru_16.png");

    // DirectoryService Resources
    map.insert("AWS::DirectoryService::MicrosoftAD", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Directory-Service_16.png");
    map.insert("AWS::DirectoryService::SimpleAD", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Directory-Service_16.png");

    // DocDB Resources
    map.insert("AWS::DocDB::DBCluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DocumentDB_16.png");
    map.insert("AWS::DocDB::DBClusterParameterGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DocumentDB_16.png");
    map.insert("AWS::DocDB::DBInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DocumentDB_16.png");
    map.insert("AWS::DocDB::DBSubnetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DocumentDB_16.png");
    map.insert("AWS::DocDB::EventSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DocumentDB_16.png");

    // DynamoDB Resources
    map.insert("AWS::DynamoDB::GlobalTable", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DynamoDB_16.png");
    map.insert("AWS::DynamoDB::Table", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-DynamoDB_16.png");

    // EC2 Resources
    map.insert(
        "AWS::EC2::CapacityReservation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::CapacityReservationFleet",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::CarrierGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::ClientVpnAuthorizationRule",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::ClientVpnEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::ClientVpnRoute",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::ClientVpnTargetNetworkAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::CustomerGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::DHCPOptions",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::EC2Fleet",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::EIP",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::EIPAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::EgressOnlyInternetGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::EnclaveCertificateIamRoleAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::FlowLog",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::GatewayRouteTableAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::Host",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAM",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAMAllocation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAMPool",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAMPoolCidr",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAMResourceDiscovery",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAMResourceDiscoveryAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::IPAMScope",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::Instance",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::InstanceConnectEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::InternetGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::KeyPair",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::LaunchTemplate",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::LocalGatewayRoute",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::LocalGatewayRouteTable",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::LocalGatewayRouteTableVPCAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::LocalGatewayRouteTableVirtualInterfaceGroupAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NatGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkAcl",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkAclEntry",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInsightsAccessScope",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInsightsAccessScopeAnalysis",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInsightsAnalysis",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInsightsPath",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInterface",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInterfaceAttachment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkInterfacePermission",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::NetworkPerformanceMetricSubscription",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::PlacementGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::PrefixList",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::Route",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::RouteServer",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::RouteServerAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::RouteServerEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::RouteServerPeer",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::RouteServerPropagation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::RouteTable",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SecurityGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SecurityGroupEgress",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SecurityGroupIngress",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SecurityGroupVpcAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SnapshotBlockPublicAccess",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SpotFleet",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::Subnet",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SubnetCidrBlock",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SubnetNetworkAclAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::SubnetRouteTableAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TrafficMirrorFilter",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TrafficMirrorFilterRule",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TrafficMirrorSession",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TrafficMirrorTarget",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayAttachment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayConnect",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayMulticastDomain",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayMulticastDomainAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayMulticastGroupMember",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayMulticastGroupSource",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayPeeringAttachment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayRoute",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayRouteTable",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayRouteTableAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayRouteTablePropagation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::TransitGatewayVpcAttachment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPC",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCBlockPublicAccessExclusion",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCBlockPublicAccessOptions",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCCidrBlock",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCDHCPOptionsAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCEndpointConnectionNotification",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCEndpointService",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCEndpointServicePermissions",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCGatewayAttachment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPCPeeringConnection",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPNConnection",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPNConnectionRoute",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPNGateway",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VPNGatewayRoutePropagation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VerifiedAccessEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VerifiedAccessGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VerifiedAccessInstance",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VerifiedAccessTrustProvider",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::Volume",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );
    map.insert(
        "AWS::EC2::VolumeAttachment",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-EC2_16.png",
    );

    // ECR Resources
    map.insert("AWS::ECR::PublicRepository", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");
    map.insert("AWS::ECR::PullThroughCacheRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");
    map.insert("AWS::ECR::RegistryPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");
    map.insert("AWS::ECR::RegistryScanningConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");
    map.insert("AWS::ECR::ReplicationConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");
    map.insert("AWS::ECR::Repository", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");
    map.insert("AWS::ECR::RepositoryCreationTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Registry_16.png");

    // ECS Resources
    map.insert("AWS::ECS::CapacityProvider", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");
    map.insert("AWS::ECS::Cluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");
    map.insert("AWS::ECS::ClusterCapacityProviderAssociations", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");
    map.insert("AWS::ECS::PrimaryTaskSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");
    map.insert("AWS::ECS::Service", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");
    map.insert("AWS::ECS::TaskDefinition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");
    map.insert("AWS::ECS::TaskSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Container-Service_16.png");

    // EFS Resources
    map.insert(
        "AWS::EFS::AccessPoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-EFS_16.png",
    );
    map.insert(
        "AWS::EFS::FileSystem",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-EFS_16.png",
    );
    map.insert(
        "AWS::EFS::MountTarget",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-EFS_16.png",
    );

    // EKS Resources
    map.insert("AWS::EKS::AccessEntry", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");
    map.insert("AWS::EKS::Addon", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");
    map.insert("AWS::EKS::Cluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");
    map.insert("AWS::EKS::FargateProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");
    map.insert("AWS::EKS::IdentityProviderConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");
    map.insert("AWS::EKS::Nodegroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");
    map.insert("AWS::EKS::PodIdentityAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Containers/16/Arch_Amazon-Elastic-Kubernetes-Service_16.png");

    // EMR Resources
    map.insert(
        "AWS::EMR::Cluster",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::InstanceFleetConfig",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::InstanceGroupConfig",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::SecurityConfiguration",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::Step",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::Studio",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::StudioSessionMapping",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );
    map.insert(
        "AWS::EMR::WALWorkspace",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-EMR_16.png",
    );

    // ElastiCache Resources
    map.insert("AWS::ElastiCache::CacheCluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::GlobalReplicationGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::ParameterGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::ReplicationGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::SecurityGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::SecurityGroupIngress", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::ServerlessCache", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::SubnetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::User", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");
    map.insert("AWS::ElastiCache::UserGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-ElastiCache_16.png");

    // ElasticBeanstalk Resources
    map.insert("AWS::ElasticBeanstalk::Application", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Elastic-Beanstalk_16.png");
    map.insert("AWS::ElasticBeanstalk::ApplicationVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Elastic-Beanstalk_16.png");
    map.insert("AWS::ElasticBeanstalk::ConfigurationTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Elastic-Beanstalk_16.png");
    map.insert("AWS::ElasticBeanstalk::Environment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Elastic-Beanstalk_16.png");

    // ElasticLoadBalancing Resources
    map.insert("AWS::ElasticLoadBalancing::LoadBalancer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");

    // ElasticLoadBalancingV2 Resources
    map.insert("AWS::ElasticLoadBalancingV2::Listener", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");
    map.insert("AWS::ElasticLoadBalancingV2::ListenerCertificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");
    map.insert("AWS::ElasticLoadBalancingV2::ListenerRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");
    map.insert("AWS::ElasticLoadBalancingV2::LoadBalancer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");
    map.insert("AWS::ElasticLoadBalancingV2::TargetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");
    map.insert("AWS::ElasticLoadBalancingV2::TrustStore", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");
    map.insert("AWS::ElasticLoadBalancingV2::TrustStoreRevocation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Elastic-Load-Balancing_16.png");

    // EntityResolution Resources
    map.insert("AWS::EntityResolution::IdMappingWorkflow", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Entity-Resolution_16.png");
    map.insert("AWS::EntityResolution::IdNamespace", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Entity-Resolution_16.png");
    map.insert("AWS::EntityResolution::MatchingWorkflow", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Entity-Resolution_16.png");
    map.insert("AWS::EntityResolution::PolicyStatement", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Entity-Resolution_16.png");
    map.insert("AWS::EntityResolution::SchemaMapping", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Entity-Resolution_16.png");

    // Events Resources
    map.insert("AWS::Events::ApiDestination", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");
    map.insert("AWS::Events::Archive", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");
    map.insert("AWS::Events::Connection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");
    map.insert("AWS::Events::Endpoint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");
    map.insert("AWS::Events::EventBus", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");
    map.insert("AWS::Events::EventBusPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");
    map.insert("AWS::Events::Rule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-EventBridge_16.png");

    // Evidently Resources
    map.insert("AWS::Evidently::Experiment", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Evidently_48.png");
    map.insert("AWS::Evidently::Feature", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Evidently_48.png");
    map.insert("AWS::Evidently::Launch", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Evidently_48.png");
    map.insert("AWS::Evidently::Project", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Evidently_48.png");
    map.insert("AWS::Evidently::Segment", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Evidently_48.png");

    // FSx Resources
    map.insert(
        "AWS::FSx::DataRepositoryAssociation",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-FSx_16.png",
    );
    map.insert(
        "AWS::FSx::FileSystem",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-FSx_16.png",
    );
    map.insert(
        "AWS::FSx::Snapshot",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-FSx_16.png",
    );
    map.insert(
        "AWS::FSx::StorageVirtualMachine",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-FSx_16.png",
    );
    map.insert(
        "AWS::FSx::Volume",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-FSx_16.png",
    );

    // FinSpace Resources
    map.insert("AWS::FinSpace::Environment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-FinSpace_16.png");

    // Forecast Resources
    map.insert("AWS::Forecast::Dataset", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Forecast_16.png");
    map.insert("AWS::Forecast::DatasetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Forecast_16.png");

    // FraudDetector Resources
    map.insert("AWS::FraudDetector::Detector", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");
    map.insert("AWS::FraudDetector::EntityType", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");
    map.insert("AWS::FraudDetector::EventType", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");
    map.insert("AWS::FraudDetector::Label", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");
    map.insert("AWS::FraudDetector::List", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");
    map.insert("AWS::FraudDetector::Outcome", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");
    map.insert("AWS::FraudDetector::Variable", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Fraud-Detector_16.png");

    // GameLift Resources
    map.insert("AWS::GameLift::Alias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::Build", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::ContainerFleet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::ContainerGroupDefinition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::Fleet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::GameServerGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::GameSessionQueue", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::Location", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::MatchmakingConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::MatchmakingRuleSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");
    map.insert("AWS::GameLift::Script", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Games/16/Arch_Amazon-GameLift_16.png");

    // GlobalAccelerator Resources
    map.insert("AWS::GlobalAccelerator::Accelerator", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-Global-Accelerator_16.png");
    map.insert("AWS::GlobalAccelerator::CrossAccountAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-Global-Accelerator_16.png");
    map.insert("AWS::GlobalAccelerator::EndpointGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-Global-Accelerator_16.png");
    map.insert("AWS::GlobalAccelerator::Listener", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_AWS-Global-Accelerator_16.png");

    // Glue Resources
    map.insert(
        "AWS::Glue::Classifier",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Connection",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Crawler",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::CustomEntityType",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::DataCatalogEncryptionSettings",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::DataQualityRuleset",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Database",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::DevEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Job",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::MLTransform",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Partition",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Registry",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Schema",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::SchemaVersion",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::SchemaVersionMetadata",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::SecurityConfiguration",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Table",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::TableOptimizer",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Trigger",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::UsageProfile",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );
    map.insert(
        "AWS::Glue::Workflow",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Glue_16.png",
    );

    // Greengrass Resources
    map.insert("AWS::Greengrass::ConnectorDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::ConnectorDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::CoreDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::CoreDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::DeviceDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::DeviceDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::FunctionDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::FunctionDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::Group", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::GroupVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::LoggerDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::LoggerDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::ResourceDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::ResourceDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::SubscriptionDefinition", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");
    map.insert("AWS::Greengrass::SubscriptionDefinitionVersion", "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Greengrass_Component-Public_48.png");

    // GroundStation Resources
    map.insert("AWS::GroundStation::Config", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Satellite/16/Arch_AWS-Ground-Station_16.png");
    map.insert("AWS::GroundStation::DataflowEndpointGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Satellite/16/Arch_AWS-Ground-Station_16.png");
    map.insert("AWS::GroundStation::MissionProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Satellite/16/Arch_AWS-Ground-Station_16.png");

    // GuardDuty Resources
    map.insert("AWS::GuardDuty::Detector", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::Filter", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::IPSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::MalwareProtectionPlan", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::Master", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::Member", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::PublishingDestination", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");
    map.insert("AWS::GuardDuty::ThreatIntelSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-GuardDuty_16.png");

    // HealthImaging Resources
    map.insert("AWS::HealthImaging::Datastore", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_AWS-HealthImaging_16.png");

    // HealthLake Resources
    map.insert("AWS::HealthLake::FHIRDatastore", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_AWS-HealthLake_16.png");

    // IAM Resources
    map.insert("AWS::IAM::AccessKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::Group", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::GroupPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::InstanceProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::ManagedPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::OIDCProvider", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::Policy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::Role", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::RolePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::SAMLProvider", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::ServerCertificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::ServiceLinkedRole", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::User", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::UserPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::UserToGroupAddition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");
    map.insert("AWS::IAM::VirtualMFADevice", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Identity-and-Access-Management_16.png");

    // Inspector Resources
    map.insert("AWS::Inspector::AssessmentTarget", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Inspector_16.png");
    map.insert("AWS::Inspector::AssessmentTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Inspector_16.png");
    map.insert("AWS::Inspector::ResourceGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Inspector_16.png");

    // IoT Resources
    map.insert("AWS::IoT::AccountAuditConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Authorizer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::BillingGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::CACertificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Certificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::CertificateProvider", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Command", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::CustomMetric", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Dimension", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::DomainConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::FleetMetric", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::JobTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Logging", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::MitigationAction", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Policy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::PolicyPrincipalAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::ProvisioningTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::ResourceSpecificLogging", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::RoleAlias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::ScheduledAudit", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::SecurityProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::SoftwarePackage", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::SoftwarePackageVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::Thing", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::ThingGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::ThingPrincipalAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::ThingType", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::TopicRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");
    map.insert("AWS::IoT::TopicRuleDestination", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Internet-of-Things/16/Arch_AWS-IoT-Core_16.png");

    // KMS Resources
    map.insert("AWS::KMS::Alias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Key-Management-Service_16.png");
    map.insert("AWS::KMS::Key", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Key-Management-Service_16.png");
    map.insert("AWS::KMS::ReplicaKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Key-Management-Service_16.png");

    // Kendra Resources
    map.insert("AWS::Kendra::DataSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Kendra_16.png");
    map.insert("AWS::Kendra::Faq", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Kendra_16.png");
    map.insert("AWS::Kendra::Index", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Kendra_16.png");

    // Kinesis Resources
    map.insert("AWS::Kinesis::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Kinesis_16.png");
    map.insert("AWS::Kinesis::Stream", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Kinesis_16.png");
    map.insert("AWS::Kinesis::StreamConsumer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Kinesis_16.png");

    // KinesisAnalytics Resources
    map.insert("AWS::KinesisAnalytics::Application", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Kinesis_16.png");
    map.insert("AWS::KinesisAnalytics::ApplicationOutput", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Kinesis_16.png");
    map.insert("AWS::KinesisAnalytics::ApplicationReferenceDataSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Kinesis_16.png");

    // KinesisFirehose Resources
    map.insert("AWS::KinesisFirehose::DeliveryStream", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Data-Firehose_16.png");

    // LakeFormation Resources
    map.insert("AWS::LakeFormation::DataCellsFilter", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");
    map.insert("AWS::LakeFormation::DataLakeSettings", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");
    map.insert("AWS::LakeFormation::Permissions", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");
    map.insert("AWS::LakeFormation::PrincipalPermissions", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");
    map.insert("AWS::LakeFormation::Resource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");
    map.insert("AWS::LakeFormation::Tag", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");
    map.insert("AWS::LakeFormation::TagAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_AWS-Lake-Formation_16.png");

    // Lambda Resources
    map.insert(
        "AWS::Lambda::Alias",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::CodeSigningConfig",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::EventInvokeConfig",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::EventSourceMapping",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::Function",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::LayerVersion",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::LayerVersionPermission",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::Permission",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::Url",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );
    map.insert(
        "AWS::Lambda::Version",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_AWS-Lambda_16.png",
    );

    // LaunchWizard Resources
    map.insert("AWS::LaunchWizard::Deployment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Launch-Wizard_16.png");

    // Lex Resources
    map.insert("AWS::Lex::Bot", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Lex_16.png");
    map.insert("AWS::Lex::BotAlias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Lex_16.png");
    map.insert("AWS::Lex::BotVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Lex_16.png");
    map.insert("AWS::Lex::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Lex_16.png");

    // LicenseManager Resources
    map.insert("AWS::LicenseManager::Grant", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-License-Manager_16.png");
    map.insert("AWS::LicenseManager::License", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-License-Manager_16.png");

    // Lightsail Resources
    map.insert("AWS::Lightsail::Alarm", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Bucket", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Certificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Container", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Database", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Disk", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Distribution", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::Instance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::LoadBalancer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::LoadBalancerTlsCertificate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");
    map.insert("AWS::Lightsail::StaticIp", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Compute/16/Arch_Amazon-Lightsail_16.png");

    // Location Resources
    map.insert(
        "AWS::Location::APIKey",
        "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Core_Device-Location_48.png",
    );
    map.insert(
        "AWS::Location::GeofenceCollection",
        "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Core_Device-Location_48.png",
    );
    map.insert("AWS::Location::Map", "assets/Icons/Resource-Icons_02072025/Res_Front-End-Web-Mobile/Res_Amazon-Location-Service_Map _48.png");
    map.insert(
        "AWS::Location::PlaceIndex",
        "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Core_Device-Location_48.png",
    );
    map.insert(
        "AWS::Location::RouteCalculator",
        "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Core_Device-Location_48.png",
    );
    map.insert(
        "AWS::Location::Tracker",
        "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Core_Device-Location_48.png",
    );
    map.insert(
        "AWS::Location::TrackerConsumer",
        "assets/Icons/Resource-Icons_02072025/Res_IoT/Res_AWS-IoT-Core_Device-Location_48.png",
    );

    // Logs Resources
    map.insert(
        "AWS::Logs::AccountPolicy",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::Delivery",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::DeliveryDestination",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::DeliverySource",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::Destination",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::Integration",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::LogAnomalyDetector",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::LogGroup",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::LogStream",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::MetricFilter",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::QueryDefinition",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::ResourcePolicy",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::SubscriptionFilter",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );
    map.insert(
        "AWS::Logs::Transformer",
        "assets/Icons/Resource-Icons_02072025/Res_General-Icons/Res_48_Light/Res_Logs_48_Light.png",
    );

    // MSK Resources
    map.insert("AWS::MSK::BatchScramSecret", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");
    map.insert("AWS::MSK::Cluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");
    map.insert("AWS::MSK::ClusterPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");
    map.insert("AWS::MSK::Configuration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");
    map.insert("AWS::MSK::Replicator", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");
    map.insert("AWS::MSK::ServerlessCluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");
    map.insert("AWS::MSK::VpcConnection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Managed-Streaming-for-Apache-Kafka_16.png");

    // Macie Resources
    map.insert("AWS::Macie::AllowList", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Macie_16.png");
    map.insert("AWS::Macie::CustomDataIdentifier", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Macie_16.png");
    map.insert("AWS::Macie::FindingsFilter", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Macie_16.png");
    map.insert("AWS::Macie::Session", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Macie_16.png");

    // ManagedBlockchain Resources
    map.insert("AWS::ManagedBlockchain::Accessor", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Blockchain/16/Arch_Amazon-Managed-Blockchain_16.png");
    map.insert("AWS::ManagedBlockchain::Member", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Blockchain/16/Arch_Amazon-Managed-Blockchain_16.png");
    map.insert("AWS::ManagedBlockchain::Node", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Blockchain/16/Arch_Amazon-Managed-Blockchain_16.png");

    // MediaConnect Resources
    map.insert("AWS::MediaConnect::Bridge", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::BridgeOutput", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::BridgeSource", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::Flow", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::FlowEntitlement", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::FlowOutput", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::FlowSource", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::FlowVpcInterface", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");
    map.insert("AWS::MediaConnect::Gateway", "assets/Icons/Resource-Icons_02072025/Res_Media-Services/Res_AWS-Elemental-MediaConnect_MediaConnect-Gateway_48.png");

    // MemoryDB Resources
    map.insert("AWS::MemoryDB::ACL", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-MemoryDB_16.png");
    map.insert("AWS::MemoryDB::Cluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-MemoryDB_16.png");
    map.insert("AWS::MemoryDB::MultiRegionCluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-MemoryDB_16.png");
    map.insert("AWS::MemoryDB::ParameterGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-MemoryDB_16.png");
    map.insert("AWS::MemoryDB::SubnetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-MemoryDB_16.png");
    map.insert("AWS::MemoryDB::User", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-MemoryDB_16.png");

    // Neptune Resources
    map.insert("AWS::Neptune::DBCluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Neptune_16.png");
    map.insert("AWS::Neptune::DBClusterParameterGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Neptune_16.png");
    map.insert("AWS::Neptune::DBInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Neptune_16.png");
    map.insert("AWS::Neptune::DBParameterGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Neptune_16.png");
    map.insert("AWS::Neptune::DBSubnetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Neptune_16.png");
    map.insert("AWS::Neptune::EventSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Neptune_16.png");

    // NetworkFirewall Resources
    map.insert("AWS::NetworkFirewall::Firewall", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Network-Firewall_16.png");
    map.insert("AWS::NetworkFirewall::FirewallPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Network-Firewall_16.png");
    map.insert("AWS::NetworkFirewall::LoggingConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Network-Firewall_16.png");
    map.insert("AWS::NetworkFirewall::RuleGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Network-Firewall_16.png");
    map.insert("AWS::NetworkFirewall::TLSInspectionConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Network-Firewall_16.png");

    // OpenSearchService Resources
    map.insert("AWS::OpenSearchService::Application", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-OpenSearch-Service_16.png");
    map.insert("AWS::OpenSearchService::Domain", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-OpenSearch-Service_16.png");

    // Organizations Resources
    map.insert("AWS::Organizations::Account", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Organizations_16.png");
    map.insert("AWS::Organizations::Organization", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Organizations_16.png");
    map.insert("AWS::Organizations::OrganizationalUnit", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Organizations_16.png");
    map.insert("AWS::Organizations::Policy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Organizations_16.png");
    map.insert("AWS::Organizations::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Organizations_16.png");

    // Panorama Resources
    map.insert("AWS::Panorama::ApplicationInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_AWS-Panorama_16.png");
    map.insert("AWS::Panorama::Package", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_AWS-Panorama_16.png");
    map.insert("AWS::Panorama::PackageVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_AWS-Panorama_16.png");

    // PaymentCryptography Resources
    map.insert("AWS::PaymentCryptography::Alias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Payment-Cryptography_16.png");
    map.insert("AWS::PaymentCryptography::Key", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Payment-Cryptography_16.png");

    // Personalize Resources
    map.insert("AWS::Personalize::Dataset", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Personalize_16.png");
    map.insert("AWS::Personalize::DatasetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Personalize_16.png");
    map.insert("AWS::Personalize::Schema", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Personalize_16.png");
    map.insert("AWS::Personalize::Solution", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Personalize_16.png");

    // Pinpoint Resources
    map.insert("AWS::Pinpoint::ADMChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::APNSChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::APNSSandboxChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::APNSVoipChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::APNSVoipSandboxChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::App", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::ApplicationSettings", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::BaiduChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::Campaign", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::EmailChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::EmailTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::EventStream", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::GCMChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::InAppTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::PushTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::SMSChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::Segment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::SmsTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");
    map.insert("AWS::Pinpoint::VoiceChannel", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Pinpoint_16.png");

    // Pipes Resources
    map.insert("AWS::Pipes::Pipe", "assets/Icons/Resource-Icons_02072025/Res_Application-Integration/Res_Amazon-EventBridge_Pipes_48.png");

    // Proton Resources
    map.insert("AWS::Proton::EnvironmentAccountConnection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Proton_16.png");
    map.insert("AWS::Proton::EnvironmentTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Proton_16.png");
    map.insert("AWS::Proton::ServiceTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Proton_16.png");

    // QLDB Resources
    map.insert("AWS::QLDB::Ledger", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Blockchain/16/Arch_Amazon-Quantum-Ledger-Database_16.png");
    map.insert("AWS::QLDB::Stream", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Blockchain/16/Arch_Amazon-Quantum-Ledger-Database_16.png");

    // QuickSight Resources
    map.insert("AWS::QuickSight::Analysis", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::CustomPermissions", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::Dashboard", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::DataSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::DataSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::Folder", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::RefreshSchedule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::Template", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::Theme", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::Topic", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");
    map.insert("AWS::QuickSight::VPCConnection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-QuickSight_16.png");

    // RDS Resources
    map.insert(
        "AWS::RDS::CustomDBEngineVersion",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBCluster",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBClusterParameterGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBInstance",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBParameterGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBProxy",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBProxyEndpoint",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBProxyTargetGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBSecurityGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBSecurityGroupIngress",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBShardGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::DBSubnetGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::EventSubscription",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::GlobalCluster",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::Integration",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );
    map.insert(
        "AWS::RDS::OptionGroup",
        "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-RDS_16.png",
    );

    // RUM Resources
    map.insert("AWS::RUM::AppMonitor", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_RUM_48.png");

    // Redshift Resources
    map.insert("AWS::Redshift::Cluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::ClusterParameterGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::ClusterSecurityGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::ClusterSecurityGroupIngress", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::ClusterSubnetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::EndpointAccess", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::EndpointAuthorization", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::EventSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::Integration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");
    map.insert("AWS::Redshift::ScheduledAction", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-Redshift_16.png");

    // RefactorSpaces Resources
    map.insert("AWS::RefactorSpaces::Application", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Migration-Hub_Refactor-Spaces-Applications_48.png");
    map.insert("AWS::RefactorSpaces::Environment", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Migration-Hub_Refactor-Spaces-Environments_48.png");
    map.insert("AWS::RefactorSpaces::Route", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Migration-Hub_Refactor-Spaces-Applications_48.png");
    map.insert("AWS::RefactorSpaces::Service", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Migration-Hub_Refactor-Spaces-Services_48.png");

    // Rekognition Resources
    map.insert("AWS::Rekognition::Collection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Rekognition_16.png");
    map.insert("AWS::Rekognition::Project", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Rekognition_16.png");
    map.insert("AWS::Rekognition::StreamProcessor", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Artificial-Intelligence/16/Arch_Amazon-Rekognition_16.png");

    // ResilienceHub Resources
    map.insert("AWS::ResilienceHub::App", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Resilience-Hub_16.png");
    map.insert("AWS::ResilienceHub::ResiliencyPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Resilience-Hub_16.png");

    // RoboMaker Resources
    map.insert("AWS::RoboMaker::Fleet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Robotics/16/Arch_AWS-RoboMaker_16.png");
    map.insert("AWS::RoboMaker::Robot", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Robotics/16/Arch_AWS-RoboMaker_16.png");
    map.insert("AWS::RoboMaker::RobotApplication", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Robotics/16/Arch_AWS-RoboMaker_16.png");
    map.insert("AWS::RoboMaker::RobotApplicationVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Robotics/16/Arch_AWS-RoboMaker_16.png");
    map.insert("AWS::RoboMaker::SimulationApplication", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Robotics/16/Arch_AWS-RoboMaker_16.png");
    map.insert("AWS::RoboMaker::SimulationApplicationVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Robotics/16/Arch_AWS-RoboMaker_16.png");

    // RolesAnywhere Resources
    map.insert("AWS::RolesAnywhere::CRL", "assets/Icons/Resource-Icons_02072025/Res_Security-Identity-Compliance/Res_AWS-Identity-Access-Management_IAM-Roles-Anywhere_48.png");
    map.insert("AWS::RolesAnywhere::Profile", "assets/Icons/Resource-Icons_02072025/Res_Security-Identity-Compliance/Res_AWS-Identity-Access-Management_IAM-Roles-Anywhere_48.png");
    map.insert("AWS::RolesAnywhere::TrustAnchor", "assets/Icons/Resource-Icons_02072025/Res_Security-Identity-Compliance/Res_AWS-Identity-Access-Management_IAM-Roles-Anywhere_48.png");

    // Route53 Resources
    map.insert("AWS::Route53::CidrCollection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");
    map.insert("AWS::Route53::DNSSEC", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");
    map.insert("AWS::Route53::HealthCheck", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");
    map.insert("AWS::Route53::HostedZone", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");
    map.insert("AWS::Route53::KeySigningKey", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");
    map.insert("AWS::Route53::RecordSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");
    map.insert("AWS::Route53::RecordSetGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Networking-Content-Delivery/16/Arch_Amazon-Route-53_16.png");

    // S3 Resources
    map.insert("AWS::S3::AccessGrant", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::AccessGrantsInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::AccessGrantsLocation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::AccessPoint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::Bucket", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::BucketPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::MultiRegionAccessPoint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::MultiRegionAccessPointPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::StorageLens", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");
    map.insert("AWS::S3::StorageLensGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Storage/16/Arch_Amazon-Simple-Storage-Service_16.png");

    // SES Resources
    map.insert("AWS::SES::ConfigurationSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::ConfigurationSetEventDestination", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::ContactList", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::DedicatedIpPool", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::EmailIdentity", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerAddonInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerAddonSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerArchive", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerIngressPoint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerRelay", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerRuleSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::MailManagerTrafficPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::ReceiptFilter", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::ReceiptRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::ReceiptRuleSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::Template", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");
    map.insert("AWS::SES::VdmAttributes", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Business-Applications/16/Arch_Amazon-Simple-Email-Service_16.png");

    // SNS Resources
    map.insert("AWS::SNS::Subscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Notification-Service_16.png");
    map.insert("AWS::SNS::Topic", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Notification-Service_16.png");
    map.insert("AWS::SNS::TopicInlinePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Notification-Service_16.png");
    map.insert("AWS::SNS::TopicPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Notification-Service_16.png");

    // SQS Resources
    map.insert("AWS::SQS::Queue", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Queue-Service_16.png");
    map.insert("AWS::SQS::QueueInlinePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Queue-Service_16.png");
    map.insert("AWS::SQS::QueuePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_Amazon-Simple-Queue-Service_16.png");

    // SSM Resources
    map.insert("AWS::SSM::Association", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::Document", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::MaintenanceWindow", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::MaintenanceWindowTarget", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::MaintenanceWindowTask", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::Parameter", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::PatchBaseline", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::ResourceDataSync", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");
    map.insert("AWS::SSM::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Systems-Manager_16.png");

    // SageMaker Resources
    map.insert("AWS::SageMaker::App", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::AppImageConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Cluster", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::CodeRepository", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::DataQualityJobDefinition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Device", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::DeviceFleet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Domain", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Endpoint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::EndpointConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::FeatureGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Image", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ImageVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::InferenceComponent", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::InferenceExperiment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::MlflowTrackingServer", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Model", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ModelBiasJobDefinition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ModelCard", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ModelExplainabilityJobDefinition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ModelPackage", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ModelPackageGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::ModelQualityJobDefinition", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::MonitoringSchedule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::NotebookInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::NotebookInstanceLifecycleConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::PartnerApp", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Pipeline", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Project", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Space", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::StudioLifecycleConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::UserProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");
    map.insert("AWS::SageMaker::Workteam", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Analytics/16/Arch_Amazon-SageMaker_16.png");

    // Scheduler Resources
    map.insert("AWS::Scheduler::Schedule", "assets/Icons/Resource-Icons_02072025/Res_Application-Integration/Res_Amazon-EventBridge_Scheduler_48.png");
    map.insert("AWS::Scheduler::ScheduleGroup", "assets/Icons/Resource-Icons_02072025/Res_Application-Integration/Res_Amazon-EventBridge_Scheduler_48.png");

    // SecretsManager Resources
    map.insert("AWS::SecretsManager::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Secrets-Manager_16.png");
    map.insert("AWS::SecretsManager::RotationSchedule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Secrets-Manager_16.png");
    map.insert("AWS::SecretsManager::Secret", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Secrets-Manager_16.png");
    map.insert("AWS::SecretsManager::SecretTargetAttachment", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Secrets-Manager_16.png");

    // SecurityHub Resources
    map.insert("AWS::SecurityHub::AutomationRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::ConfigurationPolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::DelegatedAdmin", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::FindingAggregator", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::Hub", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::Insight", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::OrganizationConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::PolicyAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::ProductSubscription", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::SecurityControl", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");
    map.insert("AWS::SecurityHub::Standard", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Security-Hub_16.png");

    // SecurityLake Resources
    map.insert("AWS::SecurityLake::AwsLogSource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Security-Lake_16.png");
    map.insert("AWS::SecurityLake::DataLake", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Security-Lake_16.png");
    map.insert("AWS::SecurityLake::Subscriber", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Security-Lake_16.png");
    map.insert("AWS::SecurityLake::SubscriberNotification", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Security-Lake_16.png");

    // ServiceCatalog Resources
    map.insert("AWS::ServiceCatalog::AcceptedPortfolioShare", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::CloudFormationProduct", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::CloudFormationProvisionedProduct", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::LaunchNotificationConstraint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::LaunchRoleConstraint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::LaunchTemplateConstraint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::Portfolio", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::PortfolioPrincipalAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::PortfolioProductAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::PortfolioShare", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::ResourceUpdateConstraint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::ServiceAction", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::ServiceActionAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::StackSetConstraint", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::TagOption", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");
    map.insert("AWS::ServiceCatalog::TagOptionAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Management-Governance/16/Arch_AWS-Service-Catalog_16.png");

    // Shield Resources
    map.insert("AWS::Shield::DRTAccess", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Shield_16.png");
    map.insert("AWS::Shield::ProactiveEngagement", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Shield_16.png");
    map.insert("AWS::Shield::Protection", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Shield_16.png");
    map.insert("AWS::Shield::ProtectionGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Shield_16.png");

    // Signer Resources
    map.insert("AWS::Signer::ProfilePermission", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Signer_16.png");
    map.insert("AWS::Signer::SigningProfile", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-Signer_16.png");

    // StepFunctions Resources
    map.insert("AWS::StepFunctions::Activity", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-Step-Functions_16.png");
    map.insert("AWS::StepFunctions::StateMachine", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-Step-Functions_16.png");
    map.insert("AWS::StepFunctions::StateMachineAlias", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-Step-Functions_16.png");
    map.insert("AWS::StepFunctions::StateMachineVersion", "assets/Icons/Architecture-Service-Icons_02072025/Arch_App-Integration/16/Arch_AWS-Step-Functions_16.png");

    // Synthetics Resources
    map.insert("AWS::Synthetics::Canary", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Synthetics_48.png");
    map.insert("AWS::Synthetics::Group", "assets/Icons/Resource-Icons_02072025/Res_Management-Governance/Res_Amazon-CloudWatch_Synthetics_48.png");

    // Timestream Resources
    map.insert("AWS::Timestream::Database", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Timestream_16.png");
    map.insert("AWS::Timestream::InfluxDBInstance", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Timestream_16.png");
    map.insert("AWS::Timestream::ScheduledQuery", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Timestream_16.png");
    map.insert("AWS::Timestream::Table", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Database/16/Arch_Amazon-Timestream_16.png");

    // Transfer Resources
    map.insert("AWS::Transfer::Agreement", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::Certificate", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::Connector", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::Profile", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::Server", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::User", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::WebApp", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");
    map.insert("AWS::Transfer::Workflow", "assets/Icons/Resource-Icons_02072025/Res_Migration-Modernization/Res_AWS-Transfer-Family_AWS-AS2_48.png");

    // VerifiedPermissions Resources
    map.insert("AWS::VerifiedPermissions::IdentitySource", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Verified-Permissions_16.png");
    map.insert("AWS::VerifiedPermissions::Policy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Verified-Permissions_16.png");
    map.insert("AWS::VerifiedPermissions::PolicyStore", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Verified-Permissions_16.png");
    map.insert("AWS::VerifiedPermissions::PolicyTemplate", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_Amazon-Verified-Permissions_16.png");

    // WAF Resources
    map.insert("AWS::WAF::ByteMatchSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAF::IPSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAF::Rule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAF::SizeConstraintSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAF::SqlInjectionMatchSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAF::WebACL", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAF::XssMatchSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");

    // WAFv2 Resources
    map.insert("AWS::WAFv2::IPSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAFv2::LoggingConfiguration", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAFv2::RegexPatternSet", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAFv2::RuleGroup", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAFv2::WebACL", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");
    map.insert("AWS::WAFv2::WebACLAssociation", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Security-Identity-Compliance/16/Arch_AWS-WAF_16.png");

    // WorkSpaces Resources
    map.insert("AWS::WorkSpaces::ConnectionAlias", "assets/Icons/Resource-Icons_02072025/Res_End-User-Computing/Res_Amazon-WorkSpaces-Family_Amazon-WorkSpaces-Core_48.png");
    map.insert("AWS::WorkSpaces::Workspace", "assets/Icons/Resource-Icons_02072025/Res_End-User-Computing/Res_Amazon-WorkSpaces-Family_Amazon-WorkSpaces-Core_48.png");
    map.insert("AWS::WorkSpaces::WorkspacesPool", "assets/Icons/Resource-Icons_02072025/Res_End-User-Computing/Res_Amazon-WorkSpaces-Family_Amazon-WorkSpaces-Core_48.png");

    // XRay Resources
    map.insert("AWS::XRay::Group", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-X-Ray_16.png");
    map.insert("AWS::XRay::ResourcePolicy", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-X-Ray_16.png");
    map.insert("AWS::XRay::SamplingRule", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-X-Ray_16.png");
    map.insert("AWS::XRay::TransactionSearchConfig", "assets/Icons/Architecture-Service-Icons_02072025/Arch_Developer-Tools/16/Arch_AWS-X-Ray_16.png");

    // Default icon for unknown resource types
    map.insert(
        "default",
        "assets/Icons/Architecture-Group-Icons_02072025/AWS-Cloud_32.png",
    );

    map
});

/// Get the icon path for a given CloudFormation resource type
pub fn get_icon_for_resource(resource_type: &str) -> &'static str {
    if let Some(icon_path) = RESOURCE_ICONS.get(resource_type) {
        log_trace!(
            "Found exact icon match for resource type: {} -> {}",
            resource_type,
            icon_path
        );
        return icon_path;
    }

    // Try to match by service prefix if exact match not found
    let service_prefix = resource_type
        .split("::")
        .take(2)
        .collect::<Vec<_>>()
        .join("::");
    log_trace!(
        "No exact match for {}, trying service prefix: {}",
        resource_type,
        service_prefix
    );

    // Check if we have any resources that start with this service prefix
    for (key, value) in RESOURCE_ICONS.iter() {
        if key.starts_with(&service_prefix) && *key != "default" {
            log_trace!("Found service prefix match: {} -> {}", key, value);
            return value;
        }
    }

    // Return default icon if no match found
    let default_icon = RESOURCE_ICONS.get("default").unwrap();
    warn!(
        "No icon found for resource type: {}, using default: {}",
        resource_type, default_icon
    );
    default_icon
}
