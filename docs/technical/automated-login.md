# Automated Login for AWS Dash

AWS Dash now supports automated login functionality similar to Selenium for web browsers. This allows you to programmatically trigger the AWS Identity Center login process.

## Overview

The automated login feature enables you to:
- Start AWS Dash with automatic login via command-line arguments
- Use environment variables to configure login parameters
- Programmatically trigger login in tests using egui_kittest
- Build automation scripts for repetitive tasks

## Command-Line Usage

### Using Command-Line Arguments

```bash
cargo run -- --auto-login \
  --identity-url https://mycompany.awsapps.com/start/ \
  --region us-east-1 \
  --role DeveloperRole
```

### Using Environment Variables

```bash
export AWS_DASH_AUTO_LOGIN=true
export AWS_IDENTITY_CENTER_URL=https://mycompany.awsapps.com/start/
export AWS_IDENTITY_CENTER_REGION=us-east-1
export AWS_DEFAULT_ROLE=DeveloperRole

cargo run
```

### Command-Line Options

- `--auto-login`: Enable automatic login on startup
- `--identity-url`: AWS Identity Center URL (e.g., https://mycompany.awsapps.com/start/)
- `--region`: AWS Region for Identity Center (e.g., us-east-1)
- `--role`: Default AWS role name to assume

## Testing with egui_kittest

The `tests/automated_login_test.rs` file demonstrates how to automate the login process in tests:

```rust
use awsdash::app::dashui::aws_login_window::AwsLoginWindow;
use egui_kittest::Harness;

#[test]
fn test_automated_aws_login() {
    let login_window = Arc::new(Mutex::new(AwsLoginWindow::default()));
    
    // Set login parameters
    {
        let mut window = login_window.lock().unwrap();
        window.identity_center_url = "https://mycompany.awsapps.com/start/".to_string();
        window.identity_center_region = "us-east-1".to_string();
        window.default_role_name = "DeveloperRole".to_string();
    }
    
    // Create UI harness and trigger login
    let mut harness = Harness::new(move |ctx| {
        let mut window = login_window.lock().unwrap();
        window.open = true;
        window.show(ctx, None);
    });
    
    // Trigger login
    {
        let mut window = login_window.lock().unwrap();
        window.start_login();
    }
    
    harness.run();
}
```

## Example Automation Script

Run the example automation script:

```bash
cargo run --bin auto_login_example
```

This demonstrates programmatic launching of AWS Dash with auto-login enabled.

## Implementation Details

### Main Application Changes

1. **Command-Line Parsing**: Added clap dependency and argument parsing in `src/main.rs`
2. **Auto-Login Flag**: Added `should_auto_login` field to `DashApp`
3. **Login Trigger**: Auto-login logic in the `update` method checks the flag and triggers login
4. **Public API**: Made `start_login()` and credential fields public in `AwsLoginWindow`

### Testing Infrastructure

- Uses `egui_kittest` for UI automation
- Supports both direct state manipulation and UI interaction simulation
- Can be integrated into CI/CD pipelines for automated testing

## Security Considerations

⚠️ **Warning**: Storing AWS credentials or Identity Center URLs in scripts or environment variables may pose security risks. Consider:

- Using secure credential storage mechanisms
- Implementing proper access controls
- Avoiding hardcoded credentials in source code
- Using temporary credentials when possible

## Future Enhancements

Potential improvements to the automation system:

1. **Browser Automation**: Integrate with actual browser automation tools for the SSO flow
2. **Headless Mode**: Support running without GUI for CI/CD environments
3. **Credential Caching**: Implement secure credential caching
4. **Multi-Account Support**: Automate switching between multiple AWS accounts
5. **Session Management**: Automatic session refresh and re-authentication