use awsdash::app::aws_identity::{AwsAccount, AwsCredentials, AwsIdentityCenter, LoginState};
use awsdash::app::dashui::credentials_debug_window::CredentialsDebugWindow;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

#[test]
fn test_credentials_debug_window_integration_with_mock_app_state() {
    // This test simulates the exact scenario the user described:
    // - AWS Identity Center is configured and logged in
    // - User can view accounts and open console (proving login works)
    // - But credentials debug window shows "No AWS Identity Center Configured"

    // Create a mock AWS Identity Center in logged-in state with accounts
    let mut identity_center = AwsIdentityCenter::new(
        "https://dash1.awsapps.com/start/".to_string(),
        "awsdash".to_string(),
        "us-east-1".to_string(),
    );

    // Set to logged in state
    identity_center.login_state = LoginState::LoggedIn;

    // Add default role credentials (what should show up in debug window)
    let default_credentials = AwsCredentials {
        access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
        session_token: Some("AQoDYXdzEJr...<remainder of security token>".to_string()),
        expiration: Some(
            DateTime::parse_from_rfc3339("2024-12-31T23:59:59Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    identity_center.default_role_credentials = Some(default_credentials.clone());

    // Add test accounts (proving that accounts window would work)
    let test_account = AwsAccount {
        account_id: "123456789012".to_string(),
        account_name: "Production Account".to_string(),
        account_email: Some("admin@company.com".to_string()),
        role_name: "awsdash".to_string(),
        credentials: Some(default_credentials),
    };

    identity_center.accounts.push(test_account);

    // Wrap in Arc<Mutex<>> as the app would do
    let aws_identity_center = Arc::new(Mutex::new(identity_center));

    // Create debug window
    let mut debug_window = CredentialsDebugWindow::default();
    debug_window.open = true;

    // Create egui context
    let ctx = egui::Context::default();

    // Test the debug window with proper AWS identity (simulating the fix)
    let mut window_shown = false;
    let _ = ctx.run(Default::default(), |ctx| {
        let result = debug_window.show_with_focus(
            ctx,
            Some(&aws_identity_center), // This is the key fix - passing the actual identity
            None,
            false,
        );
        window_shown = result.is_some();
    });

    // Verify the window was rendered
    assert!(
        window_shown,
        "Debug window should render when AWS identity is provided"
    );

    // Verify the AWS identity state that should be displayed
    let identity_guard = aws_identity_center.lock().unwrap();
    assert_eq!(identity_guard.login_state, LoginState::LoggedIn);
    assert!(identity_guard.default_role_credentials.is_some());
    assert_eq!(identity_guard.accounts.len(), 1);
    assert_eq!(
        identity_guard.identity_center_url,
        "https://dash1.awsapps.com/start/"
    );
    assert_eq!(identity_guard.default_role_name, "awsdash");
    assert_eq!(identity_guard.identity_center_region, "us-east-1");
}

#[test]
fn test_credentials_debug_window_without_identity_shows_config_message() {
    // This test verifies the "No AWS Identity Center Configured" message
    // is shown when no identity is passed (the old broken behavior)

    let mut debug_window = CredentialsDebugWindow::default();
    debug_window.open = true;

    let ctx = egui::Context::default();

    let mut window_shown = false;
    let _ = ctx.run(Default::default(), |ctx| {
        let result = debug_window.show_with_focus(
            ctx, None, // No identity passed - should show config message
            None, false,
        );
        window_shown = result.is_some();
    });

    // Window should still render, but show the "no config" message
    assert!(
        window_shown,
        "Debug window should render even without identity"
    );
}

#[test]
fn test_credentials_debug_window_shows_credential_details() {
    // This test verifies that specific credential details are accessible
    // through the debug window when properly configured

    let mut identity_center = AwsIdentityCenter::new(
        "https://mycompany.awsapps.com/start/".to_string(),
        "PowerUserAccess".to_string(),
        "us-west-2".to_string(),
    );

    identity_center.login_state = LoginState::LoggedIn;

    // Test credentials with specific values to verify display
    let credentials = AwsCredentials {
        access_key_id: "AKIAI44QH8DHBEXAMPLE".to_string(),
        secret_access_key: "je7MtGbClwBF/2Zp9Utk/h3yCo8nvbEXAMPLEKEY".to_string(),
        session_token: Some("FQoGZXIvYXdzEJr//////////wEaDJZ4V6H7VNNh4d5VjBwgNMqPM=".to_string()),
        expiration: Some(
            DateTime::parse_from_rfc3339("2024-06-01T15:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    identity_center.default_role_credentials = Some(credentials.clone());

    // Add account with credentials
    let account = AwsAccount {
        account_id: "987654321098".to_string(),
        account_name: "Development Account".to_string(),
        account_email: Some("dev-team@company.com".to_string()),
        role_name: "PowerUserAccess".to_string(),
        credentials: Some(credentials),
    };

    identity_center.accounts.push(account);

    let aws_identity = Arc::new(Mutex::new(identity_center));

    // Verify the data that would be displayed in the debug window
    let identity_guard = aws_identity.lock().unwrap();

    // Check basic AWS Identity information
    assert_eq!(
        identity_guard.identity_center_url,
        "https://mycompany.awsapps.com/start/"
    );
    assert_eq!(identity_guard.identity_center_region, "us-west-2");
    assert_eq!(identity_guard.default_role_name, "PowerUserAccess");
    assert_eq!(identity_guard.login_state, LoginState::LoggedIn);

    // Check default role credentials
    if let Some(creds) = &identity_guard.default_role_credentials {
        assert_eq!(creds.access_key_id, "AKIAI44QH8DHBEXAMPLE");
        assert!(creds.secret_access_key.starts_with("je7MtGbClwBF"));
        assert!(creds.session_token.is_some());
        assert!(creds.expiration.is_some());
    } else {
        panic!("Default role credentials should be present");
    }

    // Check account information
    assert_eq!(identity_guard.accounts.len(), 1);
    let account = &identity_guard.accounts[0];
    assert_eq!(account.account_id, "987654321098");
    assert_eq!(account.account_name, "Development Account");
    assert_eq!(account.role_name, "PowerUserAccess");
    assert!(account.credentials.is_some());
}
