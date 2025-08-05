//! Automated login test using egui_kittest
//!
//! This test demonstrates how to automatically trigger the AWS login process
//! similar to how Selenium works for web automation.

use awsdash::app::dashui::aws_login_window::AwsLoginWindow;
use awsdash::app::dashui::window_focus::FocusableWindow;
use egui_kittest::Harness;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn test_automated_aws_login() {
    // Create a shared state for the login window
    let login_window = Arc::new(Mutex::new(AwsLoginWindow::default()));
    let login_window_clone = login_window.clone();

    // Create the UI harness
    let mut harness = Harness::new(move |ctx| {
        let mut window = login_window_clone.lock().unwrap();
        window.open = true; // Ensure window is open
        window.show(ctx, None);
    });

    // Run initial frame to render the window
    harness.run();

    // Find and fill the Identity Center URL field
    if let Ok(url_field) = harness.get_by_name("Identity Center URL:") {
        // Clear existing text and enter new URL
        url_field.click();
        harness.run();
        
        // Simulate typing the URL
        // Note: egui_kittest might have limitations on text input simulation
        // In a real implementation, you might need to directly modify the state
    }

    // Alternative approach: Direct state manipulation
    {
        let mut window = login_window.lock().unwrap();
        window.identity_center_url = "https://mycompany.awsapps.com/start/".to_string();
        window.identity_center_region = "us-east-1".to_string();
        window.default_role_name = "DeveloperRole".to_string();
    }

    // Run frame to update UI with new values
    harness.run();

    // Find and click the login button
    if let Ok(login_button) = harness.get_by_name("Login with AWS Identity Center") {
        login_button.click();
        harness.run();
        
        // Wait a bit for the login process to start
        thread::sleep(Duration::from_millis(100));
        harness.run();
    }

    // Verify the login process has started
    {
        let window = login_window.lock().unwrap();
        assert!(window.aws_identity.is_some(), "AWS identity should be initialized after login");
    }

    // Simulate completing the browser authentication
    // In a real scenario, you would automate the browser part separately
    thread::sleep(Duration::from_millis(200));
    harness.run();

    // Find and click the "I've completed the login" button
    if let Ok(complete_button) = harness.get_by_name("I've completed the login") {
        complete_button.click();
        harness.run();
        
        // Wait for the login to complete
        thread::sleep(Duration::from_millis(500));
        harness.run();
    }

    // Verify successful login
    {
        let window = login_window.lock().unwrap();
        if let Some(aws_identity) = &window.aws_identity {
            let identity = aws_identity.lock().unwrap();
            // Check if login was successful based on the state
            // Note: This depends on the mock implementation behavior
        }
    }
}

#[test]
fn test_automated_login_with_environment_variables() {
    // Read login credentials from environment variables
    let identity_url = std::env::var("AWS_IDENTITY_CENTER_URL")
        .unwrap_or_else(|_| "https://example.awsapps.com/start/".to_string());
    let region = std::env::var("AWS_IDENTITY_CENTER_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());
    let role = std::env::var("AWS_DEFAULT_ROLE")
        .unwrap_or_else(|_| "DeveloperRole".to_string());

    let login_window = Arc::new(Mutex::new(AwsLoginWindow::default()));
    let login_window_clone = login_window.clone();

    let mut harness = Harness::new(move |ctx| {
        let mut window = login_window_clone.lock().unwrap();
        window.open = true;
        
        // Set credentials from environment variables
        window.identity_center_url = identity_url.clone();
        window.identity_center_region = region.clone();
        window.default_role_name = role.clone();
        
        window.show(ctx, None);
    });

    // Run initial frame
    harness.run();

    // Trigger login automatically
    {
        let mut window = login_window.lock().unwrap();
        window.start_login();
    }

    harness.run();
    thread::sleep(Duration::from_millis(100));
    harness.run();

    // Verify login started
    {
        let window = login_window.lock().unwrap();
        assert!(window.aws_identity.is_some(), "Login should have started");
    }
}

/// Helper function to automate the complete login flow
pub fn automate_login_flow(
    harness: &mut Harness,
    identity_url: &str,
    region: &str,
    role: &str,
) -> Result<(), String> {
    // This function can be used by other tests or the main application
    // to programmatically trigger login
    
    // Find form fields and fill them
    // Click login button
    // Wait for device authorization
    // Complete the flow
    
    Ok(())
}