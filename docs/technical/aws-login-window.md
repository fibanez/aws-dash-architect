# AWS Login Window

The AWS Identity Center login window provides a streamlined authentication interface with simplified URL input, comprehensive region selection, and clipboard integration for easy credential sharing.

## Overview

The AWS Login Window handles authentication through AWS Identity Center (formerly AWS SSO), managing the device authorization flow and credential acquisition. The window provides a user-friendly interface for entering Identity Center details and monitoring login progress.

## How to Use

**Starting a Login Session:**

1. Open the AWS Login window from the main menu
2. Enter your Identity Center short name (e.g., "mycompany" for mycompany.awsapps.com)
3. Select your AWS region from the dropdown
4. Enter your default role name (typically "awsdash")
5. Click "Start Login" to initiate device authorization

**Completing Device Authorization:**

1. Click "Open login page" to open AWS Identity Center in your browser
2. Use "Copy Link" button to copy the URL for sharing or opening in a different browser
3. Enter the verification code shown in the window
4. Complete the login in your browser
5. Click "I've completed the login" to fetch credentials

**Login Progress:**

- Spinner animation displays while credentials are being fetched
- Progress updates show authorization and credential status
- Error messages display if authentication fails

## How it Works

**Identity Center URL Handling:**

The window accepts flexible URL input and automatically validates it:

- **Short name input**: User enters just "mycompany" (recommended)
- **Full URL paste**: Automatically extracts short name from "https://mycompany.awsapps.com/start/"
- **Validation**: Filters to alphanumeric characters and hyphens only
- **URL construction**: Builds complete URL from validated short name

```rust
/// Extract short name from full URL or validate input
fn validate_and_clean_short_name(input: &str) -> String {
    // Handle full URLs by extracting short name
    let cleaned = if input.starts_with("https://") || input.starts_with("http://") {
        // Extract subdomain from URL
        extract_subdomain(input)
    } else {
        input.to_string()
    };

    // Filter to valid characters only
    cleaned
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}
```

**Region Selection:**

Comprehensive dropdown with all AWS regions:

- **Default Regions**: 16 commonly used regions (us-east-1, eu-west-1, etc.)
- **Opt-in Regions**: 11 additional regions requiring account opt-in
- **Region Display**: Shows both technical name and location (e.g., "us-east-1 (US East N. Virginia)")

**Clipboard Integration:**

Both "Open login page" and "Copy Link" buttons provide easy access to the login URL:

- **Open login page**: Opens the AWS Identity Center login in a new browser tab
- **Copy Link**: Copies the complete URL to clipboard for sharing or opening in different browser
- **Centered layout**: Both buttons use columns layout for proper horizontal centering
- **Consistent styling**: Regular buttons instead of mixed hyperlink/button styles

```rust
// Buttons with centered layout using columns
ui.columns(2, |columns| {
    columns[0].vertical_centered(|ui| {
        if ui.button("Open login page").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&login_url));
        }
    });
    columns[1].vertical_centered(|ui| {
        if ui.button("Copy Link").clicked() {
            ui.ctx().copy_text(login_url);
        }
    });
});
```

**Window Size Management:**

The login window uses a default size to prevent unwanted vertical growth:

- **Default size**: 450x400 pixels
- **Resizable**: Users can resize as needed
- **Consistent layout**: Window maintains predictable dimensions when content changes
- **No jumping**: Adding buttons during device authorization does not expand window vertically

```rust
let mut window = egui::Window::new("AWS Identity Center Login")
    .resizable(true)
    .min_width(450.0)
    .default_size(egui::Vec2::new(450.0, 400.0))
    .collapsible(false);
```

**Background Thread Credential Fetch:**

Credentials are fetched in a background thread to prevent UI freezing:

1. User clicks "I've completed the login"
2. Spinner starts animating immediately
3. Background thread spawned for credential fetch
4. Mutex released before blocking AWS SDK calls
5. UI continues rendering (spinner spins) during 30+ second wait
6. Credentials stored and login state updated when complete

```rust
// Spawn background thread without holding mutex
thread::spawn(move || {
    // Call blocking operation in scoped block
    let auth_result = {
        let mut identity = aws_identity.lock().unwrap();
        identity.complete_device_authorization()
    }; // Lock released - UI can render

    // Store credentials and update state
    if let Ok(mut identity) = aws_identity.lock() {
        identity.default_role_credentials = Some(creds);
        identity.login_state = LoginState::LoggedIn;
    }
});
```

## Implementation Details

**Key Files:**
- `src/app/dashui/aws_login_window.rs` - Login window UI and credential flow
- `src/app/aws_identity.rs` - AWS Identity Center integration
- `src/app/dashui/app/window_rendering.rs` - Window coordination and credential propagation

**AwsLoginWindow Structure:**
```rust
pub struct AwsLoginWindow {
    pub open: bool,
    identity_center_short_name: String,  // Short name (e.g., "mycompany")
    identity_center_region: String,       // AWS region
    default_role_name: String,            // Default role name
    login_in_progress: bool,              // Login initiated
    completing_login: bool,               // Waiting for credentials
    error_message: Option<String>,        // Error display
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,
    pub accounts_window_open: bool,       // Accounts list visibility
}
```

**Login State Flow:**

1. **Idle**: Window open, waiting for user input
2. **LoginInProgress**: Device authorization requested, showing verification code
3. **CompletingLogin**: Background thread fetching credentials, spinner animating
4. **LoggedIn**: Credentials stored, accounts available
5. **Error**: Authentication failed, error message displayed

## Race Condition Prevention

The login flow prevents a critical race condition where the application could initialize with incomplete credentials:

**Problem**: If `LoginState::LoggedIn` is set before credentials are fetched, ResourceExplorer and Agent Framework initialize with an identity that has no credentials, causing all AWS API calls to fail.

**Solution**: Three-part fix ensuring credentials are ready before initialization:

1. **aws_identity.rs**: Remove premature `LoginState::LoggedIn` from `complete_device_authorization()`
2. **aws_login_window.rs**: Set `LoginState::LoggedIn` only after credentials are stored
3. **window_rendering.rs**: Check `default_role_credentials.is_some()` before initializing Explorer

```rust
// Only initialize if credentials exist
let has_credentials = if let Ok(identity) = aws_identity.lock() {
    identity.default_role_credentials.is_some()
} else {
    false
};

if has_credentials {
    self.resource_explorer.set_aws_identity_center(Some(aws_identity));
}
```

## UI Components

**Login Form:**
- Identity Center short name input with hint text
- Static ".awsapps.com/start/" suffix label
- Region dropdown with 27 AWS regions
- Default role name text input
- Start Login button

**Device Authorization:**
- Verification code display
- "Open login page" button (opens in new tab)
- "Copy Link" button (centered alongside Open button)
- "I've completed the login" action button
- Spinner animation during credential fetch

**Post-Login:**
- Success message with green styling
- "View Accounts" button to see account list
- "Logout" button to clear credentials
- Safe-to-close notification

**Accounts Window:**
- Collapsible list of AWS accounts
- Account name, ID, and email display
- Simplified interface (removed debug features)

## Developer Notes

**Extending URL Validation:**

Add support for custom Identity Center domains:

```rust
fn validate_custom_domain(input: &str) -> Option<String> {
    // Handle custom domain patterns
    if input.contains("my-custom-domain.com") {
        return Some(extract_identity_center_id(input));
    }
    None
}
```

**Adding Region Validation:**

Verify region selection against account's enabled regions:

```rust
async fn validate_region_enabled(region: &str) -> Result<bool> {
    // Query AWS to check if region is enabled
    let client = aws_sdk_account::Client::new(&config);
    client.get_region_opt_status()
        .region_name(region)
        .send()
        .await
}
```

**Custom Authentication Flows:**

Extend for alternative authentication methods:

```rust
enum AuthenticationMethod {
    IdentityCenter { url: String, region: String },
    DirectCredentials { access_key: String, secret_key: String },
    AssumeRole { role_arn: String, session_name: String },
}
```

## Testing

**Manual Testing Checklist:**
- [ ] Short name input accepts valid characters only
- [ ] Full URL paste extracts short name correctly
- [ ] Region dropdown shows all 27 regions
- [ ] "Copy Link" copies complete URL to clipboard
- [ ] Spinner animates smoothly during credential fetch
- [ ] Login completes successfully with valid credentials
- [ ] Error messages display clearly for invalid credentials
- [ ] Logout clears credentials and resets window state

**Automated Testing:**

The login window can be tested using the egui_kittest framework:

```rust
#[test]
fn test_url_validation() {
    let short_name = AwsLoginWindow::validate_and_clean_short_name(
        "https://mycompany.awsapps.com/start/"
    );
    assert_eq!(short_name, "mycompany");
}

#[test]
fn test_region_selection() {
    let window = AwsLoginWindow::default();
    assert_eq!(window.identity_center_region, "us-east-1");
}
```

## Security Considerations

**Credential Handling:**
- Credentials stored in memory only (no disk persistence)
- Mutex-protected access prevents concurrent modification
- Background thread isolation prevents UI thread credential exposure
- Automatic cleanup on logout or application exit

**URL Validation:**
- Input sanitization prevents injection attacks
- Character filtering ensures valid Identity Center URLs
- No execution of user-provided URLs without validation

**Error Messages:**
- Generic messages prevent information leakage
- No credential details in error output
- Logging sanitized to exclude sensitive data

## Related Documentation

- [Credential Management](credential-management.md) - Credential lifecycle and caching
- [Resource Explorer System](resource-explorer-system.md) - Multi-account resource queries
- [Agent Framework](agent-framework-v2.md) - AI agent AWS integration
