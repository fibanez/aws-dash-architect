use awsdash::app::{
    aws_identity::{AwsAccount, AwsCredentials},
    cfn_dag::ResourceDag,
    cfn_template::CloudFormationTemplate,
    projects::{CloudFormationResource, Environment, Project, ResourceNode},
};
use serde::{Deserialize, Serialize};

/// Contract tests ensure that the public API remains stable
/// These tests will fail if any breaking changes are made to the public interface

#[test]
fn test_aws_credentials_contract() {
    // Test that credential structure fields exist
    let creds = AwsCredentials {
        access_key_id: String::new(),
        secret_access_key: String::new(),
        session_token: None,
        expiration: None,
    };

    let _access_key = &creds.access_key_id;
    let _secret_key = &creds.secret_access_key;
    let _token = &creds.session_token;
    let _expiry = &creds.expiration;
}

#[test]
fn test_aws_account_contract() {
    // Test account structure fields
    let account = AwsAccount {
        account_id: String::new(),
        account_name: String::new(),
        account_email: None,
        role_name: String::new(),
        credentials: None,
    };

    let _id = &account.account_id;
    let _name = &account.account_name;
    let _email = &account.account_email;
    let _role = &account.role_name;
    let _creds = &account.credentials;
}

#[test]
fn test_project_contract() {
    // Test project structure fields
    let project = Project {
        name: String::new(),
        description: String::new(),
        short_name: String::new(),
        created: chrono::Utc::now(),
        updated: chrono::Utc::now(),
        local_folder: None,
        git_url: None,
        environments: vec![],
        default_region: Some("us-east-1".to_string()),
        cfn_template: Some(CloudFormationTemplate::default()),
    };

    let _name = &project.name;
    let _desc = &project.description;
    let _short = &project.short_name;
    let _created = &project.created;
    let _updated = &project.updated;
    let _folder = &project.local_folder;
    let _git = &project.git_url;
    let _envs = &project.environments;
    let _region = &project.default_region;
    let _template = &project.cfn_template;
}

#[test]
fn test_environment_contract() {
    // Test environment structure
    let env = Environment {
        name: String::new(),
        aws_regions: vec![],
        aws_accounts: vec![],
        deployment_status: None,
    };

    let _env_name = &env.name;
    let _regions = &env.aws_regions;
    let _accounts = &env.aws_accounts;
}

#[test]
fn test_resource_node_contract() {
    let node = ResourceNode {
        resource_id: String::new(),
        depends_on: vec![],
    };

    let _id = &node.resource_id;
    let _deps = &node.depends_on;
}

#[test]
fn test_cloudformation_resource_contract() {
    let resource = CloudFormationResource::new(String::new(), String::new());

    let _id = &resource.resource_id;
    let _type = &resource.resource_type;
    let _props = &resource.properties;
}

#[test]
fn test_resource_dag_contract() {
    // Test DAG creation and basic operations
    let dag = ResourceDag::new();

    // Test that the type exists and can be created
    let _dag_size = std::mem::size_of_val(&dag);
}

/// This test ensures key trait implementations remain stable
#[test]
fn test_trait_implementations() {
    // Ensure key types implement expected traits
    fn assert_serde_traits<T: Serialize + for<'de> Deserialize<'de>>() {}

    assert_serde_traits::<AwsCredentials>();
    assert_serde_traits::<AwsAccount>();
    assert_serde_traits::<Project>();
    assert_serde_traits::<Environment>();
    assert_serde_traits::<CloudFormationResource>();
    assert_serde_traits::<ResourceNode>();
    assert_serde_traits::<ResourceDag>();
}
