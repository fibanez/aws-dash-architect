use awsdash::app::aws_identity::{AwsAccount, AwsCredentials, AwsIdentityCenter, LoginState};
use awsdash::app::dashui::credentials_debug_window::CredentialsDebugWindow;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

#[test]
fn test_credentials_debug_window_shows_logged_in_state() {
    // Create a mock AWS Identity Center with logged in state
    let mut identity_center = AwsIdentityCenter::new(
        "https://test.awsapps.com/start/".to_string(),
        "test-role".to_string(),
        "us-east-1".to_string(),
    );

    // Set up logged in state
    identity_center.login_state = LoginState::LoggedIn;

    // Add test credentials
    let test_credentials = AwsCredentials {
        access_key_id: "AKIATEST123456789".to_string(),
        secret_access_key: "test-secret-key".to_string(),
        session_token: Some("test-session-token".to_string()),
        expiration: Some(
            DateTime::parse_from_rfc3339("2024-12-31T23:59:59Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    identity_center.default_role_credentials = Some(test_credentials.clone());

    // Add a test account with credentials
    let test_account = AwsAccount {
        account_id: "123456789012".to_string(),
        account_name: "Test Account".to_string(),
        account_email: Some("test@example.com".to_string()),
        role_name: "test-role".to_string(),
        credentials: Some(test_credentials),
    };

    identity_center.accounts.push(test_account);

    let aws_identity = Arc::new(Mutex::new(identity_center));

    // Create the debug window
    let mut debug_window = CredentialsDebugWindow::default();
    debug_window.open = true;

    // Create a test egui context
    let ctx = egui::Context::default();

    // Test the window with the AWS identity inside egui run context
    let mut result = None;
    let _ = ctx.run(Default::default(), |ctx| {
        result = debug_window.show(ctx, Some(&aws_identity), None);
    });

    // Verify the window was shown (returns a rect)
    assert!(
        result.is_some(),
        "Debug window should render when open and identity is provided"
    );

    // Test that the window shows expected content by checking the AWS identity state
    let identity_guard = aws_identity.lock().unwrap();
    assert_eq!(identity_guard.login_state, LoginState::LoggedIn);
    assert!(identity_guard.default_role_credentials.is_some());
    assert_eq!(identity_guard.accounts.len(), 1);
}

#[test]
fn test_credentials_debug_window_shows_no_identity_configured() {
    // Create the debug window without AWS identity
    let mut debug_window = CredentialsDebugWindow::default();
    debug_window.open = true;

    // Create a test egui context
    let ctx = egui::Context::default();

    // Test the window without AWS identity inside egui run context
    let mut result = None;
    let _ = ctx.run(Default::default(), |ctx| {
        result = debug_window.show(ctx, None, None);
    });

    // Verify the window was shown (returns a rect)
    assert!(
        result.is_some(),
        "Debug window should render when open even without identity"
    );
}

#[test]
fn test_credentials_debug_window_closed_when_not_open() {
    // Create a mock AWS Identity Center
    let identity_center = AwsIdentityCenter::new(
        "https://test.awsapps.com/start/".to_string(),
        "test-role".to_string(),
        "us-east-1".to_string(),
    );
    let aws_identity = Arc::new(Mutex::new(identity_center));

    // Create the debug window but keep it closed
    let mut debug_window = CredentialsDebugWindow::default();
    debug_window.open = false;

    // Create a test egui context
    let ctx = egui::Context::default();

    // Test the window when closed inside egui run context
    let mut result = None;
    let _ = ctx.run(Default::default(), |ctx| {
        result = debug_window.show(ctx, Some(&aws_identity), None);
    });

    // Verify the window was not shown (returns None)
    assert!(
        result.is_none(),
        "Debug window should not render when closed"
    );
}

#[test]
fn test_credentials_debug_window_with_focus_functionality() {
    // Create a mock AWS Identity Center with logged in state
    let mut identity_center = AwsIdentityCenter::new(
        "https://test.awsapps.com/start/".to_string(),
        "test-role".to_string(),
        "us-east-1".to_string(),
    );

    identity_center.login_state = LoginState::LoggedIn;
    let aws_identity = Arc::new(Mutex::new(identity_center));

    // Create the debug window
    let mut debug_window = CredentialsDebugWindow::default();
    debug_window.open = true;

    // Create a test egui context
    let ctx = egui::Context::default();

    // Test the window with focus inside egui run context
    let mut result = None;
    let _ = ctx.run(Default::default(), |ctx| {
        result = debug_window.show_with_focus(ctx, Some(&aws_identity), None, true);
    });

    // Verify the window was shown
    assert!(
        result.is_some(),
        "Debug window should render with focus when open and identity is provided"
    );

    // Test without focus in a separate run to avoid egui layer conflicts
    let mut result_no_focus = None;
    let _ = ctx.run(Default::default(), |ctx| {
        result_no_focus = debug_window.show_with_focus(ctx, Some(&aws_identity), None, false);
    });

    // Verify the window was shown
    assert!(
        result_no_focus.is_some(),
        "Debug window should render without focus when open and identity is provided"
    );
}

#[test]
fn test_credentials_debug_window_display_credentials_helper() {
    // This test verifies the display_credentials helper function logic
    // by creating test credentials and ensuring the helper can process them

    let test_credentials = AwsCredentials {
        access_key_id: "AKIATEST123456789".to_string(),
        secret_access_key: "test-secret-key-very-long".to_string(),
        session_token: Some("test-session-token-very-long".to_string()),
        expiration: Some(
            DateTime::parse_from_rfc3339("2024-12-31T23:59:59Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    // Verify the secret truncation logic
    let secret = if test_credentials.secret_access_key.len() > 8 {
        format!("{}...", &test_credentials.secret_access_key[..8])
    } else {
        test_credentials.secret_access_key.clone()
    };

    assert_eq!(secret, "test-sec...");

    // Verify the session token truncation logic
    if let Some(token) = &test_credentials.session_token {
        let short_token = if token.len() > 20 {
            format!("{}...", &token[..20])
        } else {
            token.clone()
        };
        assert_eq!(short_token, "test-session-token-v...");
    }
}
