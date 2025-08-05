#[cfg(test)]
mod tests {
    use awsdash::app::aws_identity::{
        AwsAccount, AwsCredentials, AwsIdentityCenter, DeviceAuthorizationData, LoginState,
    };
    use chrono::Utc;

    #[test]
    fn test_new_identity_center() {
        let identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        assert_eq!(
            identity_center.identity_center_url,
            "https://example.awsapps.com/start"
        );
        assert_eq!(identity_center.default_role_name, "awsdash");
        assert_eq!(identity_center.identity_center_region, "us-east-1");
        assert_eq!(
            identity_center.start_url,
            "https://example.awsapps.com/start"
        );
        assert_eq!(identity_center.login_state, LoginState::NotLoggedIn);
        assert!(identity_center.access_token.is_none());
        assert!(identity_center.accounts.is_empty());
    }

    #[test]
    fn test_logout() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        // Set some state
        identity_center.login_state = LoginState::LoggedIn;
        identity_center.access_token = Some("mock_token".to_string());
        identity_center.accounts.push(AwsAccount {
            account_id: "123456789012".to_string(),
            account_name: "TestAccount".to_string(),
            account_email: Some("test@example.com".to_string()),
            role_name: "awsdash".to_string(),
            credentials: None,
        });

        // Logout
        identity_center.logout();

        // Verify state is reset
        assert_eq!(identity_center.login_state, LoginState::NotLoggedIn);
        assert!(identity_center.access_token.is_none());
        assert!(identity_center.accounts.is_empty());
        assert!(identity_center.available_roles.is_empty());
        assert!(identity_center.last_refresh.is_none());
        assert!(identity_center.token_expiration.is_none());
        assert!(identity_center.default_role_credentials.is_none());
    }

    #[test]
    fn test_initialize() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        let result = identity_center.initialize();
        assert!(result.is_ok());
        assert_eq!(identity_center.login_state, LoginState::NotLoggedIn);
        assert!(identity_center.access_token.is_none());
    }

    #[test]
    fn test_are_credentials_expired() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        // Test with no account
        assert!(identity_center.are_credentials_expired("123456789012"));

        // Add account with expired credentials
        let mut expired_account = AwsAccount {
            account_id: "123456789012".to_string(),
            account_name: "TestAccount".to_string(),
            account_email: Some("test@example.com".to_string()),
            role_name: "awsdash".to_string(),
            credentials: None,
        };

        // Set expired credentials
        let expired_time = Some(Utc::now() - chrono::Duration::hours(1));
        expired_account.credentials = Some(AwsCredentials {
            access_key_id: "ASIATESTKEYID".to_string(),
            secret_access_key: "TESTSecretKey123".to_string(),
            session_token: Some("TestSessionToken".to_string()),
            expiration: expired_time,
        });

        identity_center.accounts.push(expired_account);
        assert!(identity_center.are_credentials_expired("123456789012"));

        // Test with valid credentials - update the account in place
        identity_center.accounts[0].credentials = Some(AwsCredentials {
            access_key_id: "ASIATESTKEYID2".to_string(),
            secret_access_key: "TESTSecretKey456".to_string(),
            session_token: Some("TestSessionToken2".to_string()),
            expiration: Some(Utc::now() + chrono::Duration::minutes(10)),
        });
        assert!(!identity_center.are_credentials_expired("123456789012"));
    }

    #[test]
    fn test_update_account() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        // Add initial account
        let account1 = AwsAccount {
            account_id: "123456789012".to_string(),
            account_name: "TestAccount".to_string(),
            account_email: Some("test@example.com".to_string()),
            role_name: "awsdash".to_string(),
            credentials: None,
        };
        identity_center.update_account(account1);
        assert_eq!(identity_center.accounts.len(), 1);

        // Update existing account
        let account1_updated = AwsAccount {
            account_id: "123456789012".to_string(),
            account_name: "TestAccount Updated".to_string(),
            account_email: Some("test@example.com".to_string()),
            role_name: "PowerUserAccess".to_string(),
            credentials: None,
        };
        identity_center.update_account(account1_updated);
        assert_eq!(identity_center.accounts.len(), 1);
        assert_eq!(
            identity_center.accounts[0].account_name,
            "TestAccount Updated"
        );
        assert_eq!(identity_center.accounts[0].role_name, "PowerUserAccess");

        // Add new account
        let account2 = AwsAccount {
            account_id: "210987654321".to_string(),
            account_name: "TestAccount2".to_string(),
            account_email: Some("test2@example.com".to_string()),
            role_name: "ReadOnlyAccess".to_string(),
            credentials: None,
        };
        identity_center.update_account(account2);
        assert_eq!(identity_center.accounts.len(), 2);
    }

    #[test]
    fn test_get_account_roles() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        // Test with no roles
        let roles = identity_center.get_account_roles("123456789012");
        assert!(roles.is_empty());

        // Add some roles
        identity_center.available_roles.insert(
            "123456789012".to_string(),
            vec![
                "awsdash".to_string(),
                "PowerUserAccess".to_string(),
                "ReadOnlyAccess".to_string(),
            ],
        );

        let roles = identity_center.get_account_roles("123456789012");
        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&"awsdash".to_string()));
        assert!(roles.contains(&"PowerUserAccess".to_string()));
        assert!(roles.contains(&"ReadOnlyAccess".to_string()));
    }

    #[test]
    fn test_login_state_default() {
        let state = LoginState::default();
        assert_eq!(state, LoginState::NotLoggedIn);
    }

    #[test]
    fn test_login_state_transitions() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://example.awsapps.com/start".to_string(),
            "awsdash".to_string(),
            "us-east-1".to_string(),
        );

        // Start not logged in
        assert_eq!(identity_center.login_state, LoginState::NotLoggedIn);

        // Mock device authorization
        let device_auth = DeviceAuthorizationData {
            device_code: "test_device_code".to_string(),
            user_code: "TEST-CODE".to_string(),
            verification_uri: "https://device.sso.region.amazonaws.com/".to_string(),
            verification_uri_complete: Some(
                "https://device.sso.region.amazonaws.com/?user_code=TEST-CODE".to_string(),
            ),
            expires_in: 600,
            interval: 5,
            start_time: Utc::now(),
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
        };

        identity_center.login_state = LoginState::DeviceAuthorization(device_auth);

        match &identity_center.login_state {
            LoginState::DeviceAuthorization(data) => {
                assert_eq!(data.user_code, "TEST-CODE");
            }
            _ => panic!("Expected DeviceAuthorization state"),
        }

        // Transition to logged in
        identity_center.login_state = LoginState::LoggedIn;
        assert_eq!(identity_center.login_state, LoginState::LoggedIn);

        // Transition to error
        identity_center.login_state = LoginState::Error("Test error".to_string());
        match &identity_center.login_state {
            LoginState::Error(msg) => {
                assert_eq!(msg, "Test error");
            }
            _ => panic!("Expected Error state"),
        }
    }
}
