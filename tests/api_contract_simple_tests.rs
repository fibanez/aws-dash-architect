use awsdash::app::aws_identity::{AwsAccount, AwsCredentials};
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

/// This test ensures key trait implementations remain stable
#[test]
fn test_trait_implementations() {
    // Ensure key types implement expected traits
    fn assert_serde_traits<T: Serialize + for<'de> Deserialize<'de>>() {}

    assert_serde_traits::<AwsCredentials>();
    assert_serde_traits::<AwsAccount>();
}
