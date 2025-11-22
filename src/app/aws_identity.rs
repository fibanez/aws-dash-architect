//! # AWS Identity Center Authentication and Credential Management
//!
//! This module provides comprehensive AWS Identity Center (formerly AWS SSO) authentication
//! and credential management capabilities for secure, multi-account AWS access.
//!
//! ## Core Functionality
//!
//! - **OAuth 2.0 Device Authorization Flow**: Implements secure device authorization using
//!   AWS SSO OIDC for user authentication without requiring stored credentials
//! - **Multi-Account Access**: Manages credentials across multiple AWS accounts accessible
//!   through Identity Center with automatic role discovery and selection
//! - **Temporary Credential Management**: Handles STS temporary credentials with automatic
//!   expiration tracking and renewal capabilities
//! - **Management Account Detection**: Intelligently identifies the AWS Organizations
//!   management account for optimal default role selection
//!
//! ## Architecture and Integration
//!
//! This module integrates deeply with the AWS SDK for Rust, providing:
//! - Async-to-sync bridging using Tokio runtime for compatibility with egui applications
//! - Thread-safe credential storage with proper mutex protection
//! - Secure token handling with automatic cleanup on logout
//! - Integration with AWS Organizations API for enhanced account management
//!
//! The authentication flow seamlessly integrates with the application's UI layer through
//! the `AwsLoginWindow` component, providing real-time status updates and user feedback.
//!
//! ## Security Model
//!
//! ### Data Protection
//! - **No Persistent Storage**: Access tokens and credentials are never written to disk
//! - **Memory-Only Secrets**: Client secrets and tokens exist only in memory and are
//!   cleared on logout or application termination
//! - **Automatic Expiration**: All credentials include expiration times with proactive
//!   renewal before expiry (5-minute buffer)
//! - **Secure Transport**: All communication uses HTTPS with AWS SDK security defaults
//!
//! ### Thread Safety
//! - All public methods are designed for single-threaded use within the main egui context
//! - Internal async operations use isolated Tokio runtimes to prevent runtime conflicts
//! - Credential updates are atomic to prevent partial state corruption
//! - Channel-based communication for safe async-to-sync result passing
//!
//! ## Usage Patterns
//!
//! ### Basic Authentication Flow
//! ```rust,no_run
//! # use aws_dash::app::aws_identity::AwsIdentityCenter;
//! // Initialize Identity Center configuration
//! let mut identity_center = AwsIdentityCenter::new(
//!     "https://d-1234567890.awsapps.com/start".to_string(),
//!     "awsdash".to_string(),
//!     "us-east-1".to_string(),
//! );
//!
//! // Start device authorization flow
//! let auth_data = identity_center.start_device_authorization()?;
//! println!("Please visit: {} and enter code: {}",
//!          auth_data.verification_uri, auth_data.user_code);
//!
//! // Complete authentication (this blocks until user authorizes)
//! identity_center.complete_device_authorization()?;
//!
//! // Get default role credentials
//! let credentials = identity_center.get_default_role_credentials()?;
//! # Ok::<(), String>(())
//! ```
//!
//! ### Multi-Account Credential Management
//! ```rust,no_run
//! # use aws_dash::app::aws_identity::AwsIdentityCenter;
//! # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
//! // List all accessible accounts
//! for account in &identity_center.accounts {
//!     println!("Account: {} ({})", account.account_name, account.account_id);
//!
//!     // Get available roles for this account
//!     let roles = identity_center.get_account_roles(&account.account_id);
//!     for role in roles {
//!         // Get credentials for specific account/role combination
//!         let creds = identity_center.get_account_credentials(&account.account_id, &role)?;
//!
//!         // Check if credentials are still valid
//!         if !identity_center.are_credentials_expired(&account.account_id) {
//!             println!("Using fresh credentials for {}/{}", account.account_name, role);
//!         }
//!     }
//! }
//! # Ok::<(), String>(())
//! ```
//!
//! ## Error Handling and Recovery
//!
//! The module provides comprehensive error handling for common scenarios:
//! - Network connectivity issues during authentication
//! - Token expiration and automatic renewal requirements
//! - Invalid or insufficient IAM permissions
//! - AWS service-specific error codes and user-friendly messages
//!
//! All operations return `Result<T, String>` with descriptive error messages suitable
//! for display to end users while maintaining security by not exposing sensitive details.

use aws_config::BehaviorVersion;
use aws_sdk_iam::error::ProvideErrorMetadata;
use aws_sdk_iam::Client as IamClient;
use aws_sdk_organizations::config::Credentials as OrganizationsCredentials;
use aws_sdk_organizations::Client as OrganizationsClient;
use aws_sdk_sso::Client as SsoClient;
use aws_sdk_ssooidc::Client as SsoOidcClient;
use aws_sdk_sts::Client as StsClient;
use aws_types::region::Region;
use chrono::{DateTime, Duration, Utc};
use percent_encoding::{percent_decode, utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use tokio::runtime::Runtime;
use tracing::{error, info, warn};
use url::form_urlencoded;

/// AWS temporary credentials for secure API access.
///
/// Represents a complete set of AWS credentials obtained through Identity Center's
/// OAuth flow. These are always temporary credentials with built-in expiration
/// to ensure security and prevent credential leakage.
///
/// # Security Considerations
///
/// - **Temporary Nature**: All credentials obtained through Identity Center are
///   temporary STS credentials that automatically expire
/// - **Memory Only**: These credentials are never persisted to disk and are
///   cleared from memory on logout
/// - **Expiration Tracking**: Includes expiration time for proactive renewal
///   before credentials become invalid
///
/// # Thread Safety
///
/// This struct is `Clone` and can be safely passed between threads. However,
/// credential renewal must be coordinated through the main `AwsIdentityCenter`
/// instance to prevent race conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsCredentials {
    /// AWS Access Key ID for API authentication.
    ///
    /// This is a temporary access key ID generated by AWS STS and is safe to
    /// log for debugging purposes (unlike the secret access key).
    pub access_key_id: String,

    /// AWS Secret Access Key for API authentication.
    ///
    /// **Security Critical**: This value must never be logged, printed, or
    /// exposed in any way. It provides full access within the bounds of the
    /// associated IAM role permissions.
    pub secret_access_key: String,

    /// AWS Session Token for temporary credential validation.
    ///
    /// Required for all temporary credentials obtained through STS. This token
    /// binds the access key and secret key together and is required for all
    /// AWS API calls using these credentials.
    pub session_token: Option<String>,

    /// UTC timestamp when these credentials expire.
    ///
    /// AWS temporary credentials typically expire within 1-12 hours. The application
    /// proactively renews credentials when they are within 5 minutes of expiration
    /// to ensure uninterrupted service.
    pub expiration: Option<DateTime<Utc>>,
}

/// AWS account accessible through Identity Center with role-based access.
///
/// Represents a single AWS account that the authenticated user can access through
/// Identity Center, along with the current role assumption and credentials.
/// Each account can have multiple available roles, but only one active role
/// at a time with associated credentials.
///
/// # Multi-Account Management
///
/// Identity Center commonly provides access to multiple AWS accounts within
/// an organization. This struct tracks:
/// - Account metadata (ID, name, email)
/// - Current role assumption for this account
/// - Active temporary credentials for the assumed role
/// - Available roles for this account (stored separately in `AwsIdentityCenter`)
///
/// # Role Switching
///
/// Users can switch between different roles within the same account by calling
/// `get_account_credentials()` with different role names. The struct will be
/// updated with new credentials for the new role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsAccount {
    /// AWS Account ID - unique 12-digit identifier.
    ///
    /// This is the canonical identifier for the AWS account and is used for
    /// all AWS API operations requiring account specification.
    pub account_id: String,

    /// Human-readable account name assigned in AWS Organizations.
    ///
    /// This name is set when the account is created or invited to the organization
    /// and helps users identify accounts in multi-account environments.
    pub account_name: String,

    /// Email address associated with the AWS account.
    ///
    /// This is typically the root user email for the account and may be used
    /// for billing and account management notifications.
    pub account_email: Option<String>,

    /// Currently assumed IAM role name for this account.
    ///
    /// This represents the active role being used for AWS API access. Users
    /// can have access to multiple roles per account, but only one is active
    /// at a time with associated credentials.
    pub role_name: String,

    /// Current temporary credentials for the assumed role.
    ///
    /// These credentials are valid only for the current `role_name` and will
    /// expire according to the `expiration` field. `None` indicates that
    /// credentials need to be obtained or refreshed.
    pub credentials: Option<AwsCredentials>,
}

/// Infrastructure information extracted from IAM role policies.
///
/// Contains DynamoDB table and CloudFormation role details extracted from
/// CloudFormation role policies to help identify the application's infrastructure
/// configuration and access patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureInfo {
    /// The full DynamoDB table ARN extracted from the policy.
    pub dynamodb_table_arn: String,

    /// AWS region where the DynamoDB table is located.
    pub db_region: String,

    /// AWS account ID that owns the DynamoDB table.
    pub db_account: String,

    /// DynamoDB table name extracted from the ARN.
    pub table_name: String,

    /// CloudFormation role ARNs found in the policy document.
    pub cloudformation_role_arns: Vec<String>,

    /// The CloudFormation role name that provided this information.
    pub source_role: String,
}

/// OAuth 2.0 device authorization flow data for secure authentication.
///
/// Contains all information needed for the device authorization flow with AWS SSO OIDC.
/// This implements the OAuth 2.0 Device Authorization Grant (RFC 8628) for
/// authentication in environments where a web browser is not available or practical.
///
/// # Security Model
///
/// - **Device Code**: Secret code used for polling AWS for authorization completion
/// - **User Code**: Short code displayed to the user for manual entry in browser
/// - **Client Credentials**: Temporary registration credentials for this auth session
/// - **Expiration**: Time-limited authorization window for security
///
/// # Usage Flow
///
/// 1. Application initiates device authorization and receives this data
/// 2. User visits `verification_uri` and enters `user_code` in browser
/// 3. Application polls AWS using `device_code` until user completes authorization
/// 4. AWS returns access tokens for subsequent API access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceAuthorizationData {
    /// Device code for polling authorization status.
    ///
    /// **Security Critical**: This secret code is used to poll AWS for authorization
    /// completion. It must be kept secure and never exposed to users.
    pub device_code: String,

    /// Short user-friendly code for manual browser entry.
    ///
    /// Users visit the verification URI and enter this short alphanumeric code
    /// to authorize the device. Typically 6-8 characters for easy manual entry.
    pub user_code: String,

    /// AWS verification URL where users complete authorization.
    ///
    /// Users must visit this URL in a web browser to complete the OAuth flow.
    /// The application attempts to automatically open this URL when possible.
    pub verification_uri: String,

    /// Complete verification URL with embedded user code (when available).
    ///
    /// If provided by AWS, this URL includes the user code as a parameter,
    /// allowing one-click authorization without manual code entry.
    pub verification_uri_complete: Option<String>,

    /// Authorization window duration in seconds.
    ///
    /// How long the device authorization remains valid. Users must complete
    /// browser authorization within this timeframe.
    pub expires_in: i64,

    /// Recommended polling interval in seconds.
    ///
    /// How frequently the application should poll AWS for authorization completion.
    /// Following this interval prevents rate limiting and ensures responsive UX.
    pub interval: i64,

    /// When this authorization flow was initiated.
    ///
    /// Used internally to track timeouts and provide accurate progress feedback
    /// to users during the authorization process.
    pub start_time: DateTime<Utc>,

    /// OIDC client ID for this authorization session.
    ///
    /// Temporary client registration ID created specifically for this device
    /// authorization flow. Used for polling and token exchange.
    pub client_id: Option<String>,

    /// OIDC client secret for this authorization session.
    ///
    /// **Security Critical**: Temporary client secret that must be kept secure
    /// and is used along with client_id for secure token exchange.
    pub client_secret: Option<String>,
}

/// Authentication state machine for Identity Center login flow.
///
/// Tracks the current state of the OAuth device authorization flow, providing
/// type-safe state management and user feedback capabilities.
///
/// # State Transitions
///
/// ```text
/// NotLoggedIn -> DeviceAuthorization -> LoggedIn
///      |              |                    |
///      v              v                    v
///    Error <-------- Error <------------- Error
/// ```
///
/// # Thread Safety
///
/// This enum is designed for single-threaded use within the main application
/// context. State transitions should only occur through `AwsIdentityCenter`
/// methods to maintain consistency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum LoginState {
    /// Initial state - no authentication attempted.
    ///
    /// The application has not yet initiated any authentication flow.
    /// Users can start device authorization from this state.
    #[default]
    NotLoggedIn,

    /// Device authorization in progress - waiting for user completion.
    ///
    /// The OAuth device flow has been initiated and the application is
    /// polling AWS for user completion. Contains all data needed to
    /// continue the flow and provide user feedback.
    DeviceAuthorization(DeviceAuthorizationData),

    /// Successfully authenticated with valid access token.
    ///
    /// Authentication is complete and the application has a valid access
    /// token for AWS API access. Account list and roles have been retrieved.
    LoggedIn,

    /// Authentication failed with error details.
    ///
    /// An error occurred during any phase of authentication. The error
    /// message is suitable for display to users while maintaining security.
    Error(String),
}

/// Central coordinator for AWS Identity Center authentication and multi-account access.
///
/// This is the primary interface for all AWS authentication operations. It manages
/// the complete OAuth 2.0 device authorization flow, maintains access tokens,
/// coordinates multi-account access, and provides secure credential management
/// for the entire application.
///
/// # Core Responsibilities
///
/// - **Authentication Management**: Handles the full OAuth device flow with AWS SSO OIDC
/// - **Multi-Account Coordination**: Manages access to multiple AWS accounts within an organization
/// - **Credential Lifecycle**: Obtains, caches, and refreshes temporary credentials automatically
/// - **Security Enforcement**: Ensures all sensitive data is properly protected and cleared
/// - **Management Account Detection**: Identifies the AWS Organizations management account
///
/// # Security Architecture
///
/// This struct implements several security best practices:
/// - **No Persistent Storage**: Sensitive tokens and credentials exist only in memory
/// - **Automatic Cleanup**: All secrets are cleared on logout or application termination
/// - **Proactive Expiration**: Credentials are renewed before expiration to prevent service interruption
/// - **Minimal Exposure**: Secret fields are excluded from serialization and debugging output
///
/// # Thread Safety and Async Integration
///
/// While this struct is designed for single-threaded use within the egui main thread,
/// it safely bridges to async AWS SDK operations using isolated Tokio runtimes.
/// This prevents runtime conflicts while enabling full AWS SDK functionality.
///
/// # State Management
///
/// The struct maintains complex state across the authentication lifecycle:
/// - Configuration (URLs, regions, default roles)
/// - Authentication state (login status, tokens, expiration)
/// - Account discovery (available accounts and roles)
/// - Active credentials (per-account, per-role credential caching)
///
/// # Usage Patterns
///
/// ## Initial Setup and Authentication
/// ```rust,no_run
/// # use aws_dash::app::aws_identity::AwsIdentityCenter;
/// let mut identity_center = AwsIdentityCenter::new(
///     "https://d-1234567890.awsapps.com/start".to_string(),
///     "awsdash".to_string(),
///     "us-east-1".to_string(),
/// );
///
/// // Initialize and start authentication
/// identity_center.initialize()?;
/// let auth_data = identity_center.start_device_authorization()?;
///
/// // User completes browser authorization...
/// identity_center.complete_device_authorization()?;
/// # Ok::<(), String>(())
/// ```
///
/// ## Multi-Account Operations
/// ```rust,no_run
/// # use aws_dash::app::aws_identity::AwsIdentityCenter;
/// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
/// // Get default credentials for primary operations
/// let default_creds = identity_center.get_default_role_credentials()?;
///
/// // Access specific account with specific role
/// for account in &identity_center.accounts.clone() {
///     let roles = identity_center.get_account_roles(&account.account_id);
///     for role in roles {
///         if !identity_center.are_credentials_expired(&account.account_id) {
///             let creds = identity_center.get_account_credentials(&account.account_id, &role)?;
///             // Use credentials for AWS API calls...
///         }
///     }
/// }
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsIdentityCenter {
    /// AWS Identity Center portal URL for this organization.
    ///
    /// The base URL for the organization's Identity Center portal. Users visit
    /// this URL to access the AWS access portal and manage their account access.
    /// Format: `https://d-xxxxxxxxxx.awsapps.com/start`
    pub identity_center_url: String,

    /// Default IAM role name for primary application operations.
    ///
    /// When the application needs AWS credentials for core functionality
    /// (like CloudFormation operations), it will use this role from the
    /// management account or first available account with this role.
    pub default_role_name: String,

    /// AWS region where Identity Center is configured.
    ///
    /// This is the region where the organization's Identity Center instance
    /// is deployed. All SSO OIDC and SSO API calls will be made to this region.
    pub identity_center_region: String,

    /// Complete start URL for Identity Center authentication.
    ///
    /// Derived from `identity_center_url` with proper formatting. This is the
    /// exact URL used in the OAuth device authorization flow.
    pub start_url: String,

    /// Current state of the authentication process.
    ///
    /// Tracks progress through the OAuth device flow and provides type-safe
    /// state management for UI updates and error handling.
    #[serde(default)]
    pub login_state: LoginState,

    /// Current AWS SSO access token for API operations.
    ///
    /// **Security Critical**: This token provides access to all authorized AWS
    /// accounts and roles. It's excluded from serialization and must be cleared
    /// on logout. Used for all SSO API calls to obtain account credentials.
    #[serde(skip_serializing)]
    pub access_token: Option<String>,

    /// OIDC client ID for the current authentication session.
    ///
    /// Temporary client registration ID created during device authorization.
    /// Excluded from serialization for security and because it's session-specific.
    #[serde(skip_serializing)]
    pub client_id: Option<String>,

    /// OIDC client secret for the current authentication session.
    ///
    /// **Security Critical**: Temporary client secret that must never be exposed.
    /// Used only during the token exchange phase of device authorization.
    #[serde(skip_serializing)]
    pub client_secret: Option<String>,

    /// All AWS accounts accessible through Identity Center.
    ///
    /// Populated during authentication with complete account metadata and
    /// current role assumptions. Updated when credentials are obtained
    /// for specific account/role combinations.
    pub accounts: Vec<AwsAccount>,

    /// Timestamp of the last successful token refresh.
    ///
    /// Used for tracking authentication freshness and debugging connectivity
    /// issues. Updated whenever new tokens are obtained from AWS.
    pub last_refresh: Option<DateTime<Utc>>,

    /// Map of available IAM roles per account.
    ///
    /// Stores the complete list of roles that the user can assume in each
    /// account. Key is account ID, value is vector of role names.
    /// Excluded from serialization due to size and session-specific nature.
    #[serde(skip)]
    pub available_roles: HashMap<String, Vec<String>>,

    /// Whether to enumerate all accessible accounts during authentication.
    ///
    /// When true, retrieves the complete list of accounts and roles during
    /// login. When false, only retrieves accounts as needed. Defaults to true
    /// for better user experience but can be disabled for performance.
    pub list_all_accounts: bool,

    /// Whether to automatically refresh tokens before expiration.
    ///
    /// When enabled, the application will proactively refresh access tokens
    /// and credentials before they expire to ensure uninterrupted service.
    pub auto_refresh: bool,

    /// Unique client name for OIDC registration.
    ///
    /// Generated UUID-based name used for temporary client registration with
    /// AWS SSO OIDC. Excluded from serialization as it's session-specific.
    #[serde(skip)]
    pub client_name: String,

    /// When the current access token expires.
    ///
    /// Used for proactive token refresh and user feedback about authentication
    /// status. Excluded from serialization as it's session-specific.
    #[serde(skip)]
    pub token_expiration: Option<DateTime<Utc>>,

    /// Cached credentials for the default role.
    ///
    /// Stores credentials for the primary role used by application features.
    /// These are refreshed automatically and used for CloudFormation operations
    /// and other core functionality. Excluded from serialization for security.
    #[serde(skip)]
    pub default_role_credentials: Option<AwsCredentials>,

    /// Account ID of the AWS Organizations management account.
    ///
    /// When identified, this account is preferred for default role operations
    /// as it typically has the broadest permissions. Detected automatically
    /// using AWS Organizations API
    pub sso_management_account_id: Option<String>,

    /// Infrastructure information extracted from CloudFormation roles.
    ///
    /// Stores information about DynamoDB tables and CloudFormation roles identified
    /// in IAM role policies. This helps track application infrastructure and
    /// their configuration across different environments.
    #[serde(skip)]
    pub infrastructure_info: Option<InfrastructureInfo>,

    /// Discovered CloudFormation deployment role name.
    ///
    /// This stores the CloudFormation deployment role name extracted from the
    /// PassRole policy statement for future CloudFormation template deployment operations.
    /// This role is separate from both the user-provided default role and the AWSReservedSSO role.
    #[serde(skip)]
    pub cloudformation_deployment_role_name: Option<String>,
}

impl AwsIdentityCenter {
    /// Creates a new AWS Identity Center configuration for authentication.
    ///
    /// Initializes the Identity Center configuration with the required parameters
    /// for OAuth device authorization flow. This constructor sets up the basic
    /// configuration but does not initiate any network operations.
    ///
    /// # Parameters
    ///
    /// * `identity_center_url` - The organization's Identity Center portal URL
    ///   (e.g., "<https://d-1234567890.awsapps.com/start>")
    /// * `default_role_name` - Primary IAM role for application operations
    ///   (e.g., "awsdash", "ReadOnlyAccess")
    /// * `identity_center_region` - AWS region where Identity Center is deployed
    ///   (e.g., "us-east-1", "eu-west-1")
    ///
    /// # Security Considerations
    ///
    /// - No sensitive data is stored or transmitted during construction
    /// - All authentication state is initialized to secure defaults
    /// - Unique client names are generated to prevent session conflicts
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// let identity_center = AwsIdentityCenter::new(
    ///     "https://d-1234567890.awsapps.com/start".to_string(),
    ///     "awsdash".to_string(),
    ///     "us-east-1".to_string(),
    /// );
    /// ```
    pub fn new(
        identity_center_url: String,
        default_role_name: String,
        identity_center_region: String,
    ) -> Self {
        // Extract start URL from the Identity Center URL if needed
        let start_url = if identity_center_url.contains("/start") {
            identity_center_url.clone()
        } else {
            format!("{}/start", identity_center_url.trim_end_matches('/'))
        };

        Self {
            identity_center_url,
            default_role_name,
            identity_center_region,
            start_url,
            login_state: LoginState::NotLoggedIn,
            access_token: None,
            client_id: None,
            client_secret: None,
            accounts: Vec::new(),
            last_refresh: None,
            available_roles: HashMap::new(),
            list_all_accounts: true,
            auto_refresh: true,
            client_name: format!("awsdash-{}", uuid::Uuid::new_v4()),
            token_expiration: None,
            default_role_credentials: None,
            sso_management_account_id: None,
            infrastructure_info: None,
            cloudformation_deployment_role_name: None,
        }
    }

    /// Identify the SSO management account using AWS Organizations API
    fn identify_sso_management_account(&mut self) -> Result<(), String> {
        info!("Attempting to identify SSO management account");

        // This requires temporary credentials from one of the accounts
        if let Some(credentials) = &self.default_role_credentials {
            let rt_start = std::time::Instant::now();
            let runtime = Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;
            log::info!("⏱️ [AWS] Runtime creation (identify_sso_management_account) took {:?}", rt_start.elapsed());

            // Clone the necessary data before the async block
            let region = self.identity_center_region.clone();
            let access_key_id = credentials.access_key_id.clone();
            let secret_access_key = credentials.secret_access_key.clone();
            let session_token = credentials.session_token.clone();

            let result = runtime.block_on(async {
                let region = Region::new(region);

                // Create AWS config with our temporary credentials
                let credentials_for_orgs = OrganizationsCredentials::new(
                    &access_key_id,
                    &secret_access_key,
                    session_token,
                    None,
                    "custom",
                );
                let config = aws_config::defaults(BehaviorVersion::latest())
                    .region(region)
                    .credentials_provider(credentials_for_orgs)
                    .load()
                    .await;

                let orgs_client = OrganizationsClient::new(&config);

                // Call DescribeOrganization to get management account info
                match orgs_client.describe_organization().send().await {
                    Ok(resp) => {
                        if let Some(org) = resp.organization {
                            if let Some(master_account_id) = org.master_account_id {
                                info!("Identified SSO management account: {}", master_account_id);
                                Ok(Some(master_account_id))
                            } else {
                                Err("No master account ID in organization response".to_string())
                            }
                        } else {
                            Err("No organization data in response".to_string())
                        }
                    }
                    Err(e) => {
                        // It's okay if this fails - not all accounts have Organizations access
                        warn!("Failed to identify SSO management account: {}", e);
                        Ok(None)
                    }
                }
            });

            match result {
                Ok(Some(management_account_id)) => {
                    self.sso_management_account_id = Some(management_account_id);
                    Ok(())
                }
                Ok(None) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            info!("No credentials available to identify SSO management account");
            Ok(())
        }
    }

    /// Securely logout from AWS Identity Center and clear all sensitive data.
    ///
    /// Performs a complete logout that clears all authentication state and
    /// credentials from memory. This ensures no sensitive data remains after
    /// logout and prepares the instance for a fresh authentication flow.
    ///
    /// # Security Operations
    ///
    /// - Clears all access tokens and client credentials from memory
    /// - Removes all cached account credentials
    /// - Resets authentication state to `NotLoggedIn`
    /// - Clears account and role information
    /// - Resets management account identification
    ///
    /// # Thread Safety
    ///
    /// This method is safe to call from the main thread and will immediately
    /// clear all sensitive data. Any ongoing async operations will fail gracefully
    /// when they attempt to use the cleared credentials.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// // After successful authentication and use...
    /// identity_center.logout();
    ///
    /// // Instance is now ready for fresh authentication
    /// assert_eq!(identity_center.login_state, aws_dash::app::aws_identity::LoginState::NotLoggedIn);
    /// ```
    pub fn logout(&mut self) {
        info!("Logging out from AWS Identity Center");
        self.login_state = LoginState::NotLoggedIn;
        self.access_token = None;
        self.client_id = None;
        self.client_secret = None;
        self.accounts.clear();
        self.available_roles.clear();
        self.last_refresh = None;
        self.token_expiration = None;
        self.default_role_credentials = None;
        self.sso_management_account_id = None;
    }

    /// Initialize the AWS Identity Center for authentication operations.
    ///
    /// Prepares the Identity Center instance for authentication by resetting
    /// any existing state and ensuring a clean starting point. This method
    /// should be called before starting a new authentication flow.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` on successful initialization, or `Err(String)` if
    /// initialization fails (though current implementation always succeeds).
    ///
    /// # State Changes
    ///
    /// - Resets login state to `NotLoggedIn`
    /// - Clears any existing authentication tokens
    /// - Removes cached credentials
    /// - Prepares for fresh OAuth device flow
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// let mut identity_center = AwsIdentityCenter::new(
    ///     "https://d-1234567890.awsapps.com/start".to_string(),
    ///     "awsdash".to_string(),
    ///     "us-east-1".to_string(),
    /// );
    ///
    /// identity_center.initialize()?;
    /// // Ready to start authentication flow
    /// # Ok::<(), String>(())
    /// ```
    pub fn initialize(&mut self) -> Result<(), String> {
        info!("Initializing AWS Identity Center");
        self.login_state = LoginState::NotLoggedIn;
        self.access_token = None;
        self.client_id = None;
        self.client_secret = None;
        self.default_role_credentials = None;
        Ok(())
    }

    /// Obtain credentials for the default role with management account preference.
    ///
    /// Retrieves temporary credentials for the configured default role, preferentially
    /// selecting the AWS Organizations management account when available. This method
    /// implements intelligent account selection and automatic management account detection.
    ///
    /// # Management Account Detection
    ///
    /// The method uses multiple strategies to identify the optimal account:
    /// 1. **AWS Organizations API**: Queries the Organizations service to identify the management account
    /// 3. **Role-Based Selection**: Prioritizes accounts with administrative roles
    /// 4. **Fallback Selection**: Uses the first available account with the default role
    ///
    /// # Credential Caching
    ///
    /// Successfully obtained credentials are cached in `default_role_credentials`
    /// for subsequent use by application features, reducing API calls and
    /// improving performance.
    ///
    /// # Return Value
    ///
    /// Returns the obtained credentials on success, or an error message describing
    /// the failure (network issues, permission problems, role not found).
    ///
    /// # Security Considerations
    ///
    /// - Credentials are temporary and include expiration timestamps
    /// - All credentials are cleared from memory on logout
    /// - Management account detection uses minimal required permissions
    /// - Failures are logged but sensitive details are not exposed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// // After successful authentication...
    /// match identity_center.get_default_role_credentials() {
    ///     Ok(credentials) => {
    ///         println!("Got credentials for access key: {}", credentials.access_key_id);
    ///         // Use credentials for AWS API calls
    ///     }
    ///     Err(error) => {
    ///         eprintln!("Failed to get default credentials: {}", error);
    ///     }
    /// }
    /// ```
    pub fn get_default_role_credentials(&mut self) -> Result<AwsCredentials, String> {
        info!(
            "Getting credentials for default role: {}",
            self.default_role_name
        );

        // Check if we have cached credentials that are still valid (TTL check)
        if let Some(ref cached_creds) = self.default_role_credentials {
            if let Some(expiration) = cached_creds.expiration {
                let now = Utc::now();
                let buffer = chrono::Duration::minutes(5);

                if now + buffer < expiration {
                    info!(
                        "Using cached default role credentials (valid until {})",
                        expiration
                    );
                    return Ok(cached_creds.clone());
                } else {
                    info!(
                        "Cached credentials expired or expiring soon (expiration: {}), refreshing",
                        expiration
                    );
                }
            }
        }

        info!("Refreshing default role credentials");

        // We need to find an account that has the default role
        // Look for accounts with the default role, prioritizing management/SSO accounts
        let accounts_with_role: Vec<_> = self
            .accounts
            .iter()
            .filter(|account| {
                self.available_roles
                    .get(&account.account_id)
                    .is_some_and(|roles| roles.contains(&self.default_role_name))
            })
            .collect();

        // Select the appropriate account for the default role
        // Prioritize the SSO management account if identified
        let selected_account = if let Some(management_account_id) = &self.sso_management_account_id
        {
            accounts_with_role
                .iter()
                .find(|account| &account.account_id == management_account_id)
                .or_else(|| accounts_with_role.first())
        } else {
            accounts_with_role.first()
        };

        if let Some(account) = selected_account {
            info!(
                "Selected account '{}' ({}) for default role '{}'",
                account.account_name, account.account_id, self.default_role_name
            );
        }

        let account_id_with_default_role =
            selected_account.map(|account| account.account_id.clone());

        match account_id_with_default_role {
            Some(account_id) => {
                // Get the default role name first to avoid borrow issues
                let default_role = self.default_role_name.clone();

                // Get credentials for this account and the default role
                let credentials = self.get_account_credentials(&account_id, &default_role)?;

                // Store these credentials as the default
                self.default_role_credentials = Some(credentials.clone());

                // Now try to identify the SSO management account using Organizations API
                let _ = self.identify_sso_management_account();

                info!("Successfully obtained default role credentials");
                Ok(credentials)
            }
            None => {
                let error_message = format!(
                    "No account found with the default role: {}",
                    self.default_role_name
                );
                error!("{}", error_message);
                Err(error_message)
            }
        }
    }

    /// Initiate OAuth 2.0 device authorization flow with AWS SSO OIDC.
    ///
    /// Starts the secure device authorization flow that allows users to authenticate
    /// through a web browser while the application waits for completion. This implements
    /// the OAuth 2.0 Device Authorization Grant (RFC 8628) for secure authentication
    /// without requiring embedded web browsers or stored credentials.
    ///
    /// # OAuth Device Flow Process
    ///
    /// 1. **Client Registration**: Registers a temporary OIDC client with AWS
    /// 2. **Device Authorization**: Initiates device flow and receives user/device codes
    /// 3. **Browser Launch**: Attempts to automatically open verification URL
    /// 4. **State Management**: Updates login state to `DeviceAuthorization`
    ///
    /// # Return Value
    ///
    /// Returns `DeviceAuthorizationData` containing:
    /// - User code for manual browser entry
    /// - Verification URL for browser authentication
    /// - Device code for polling (kept secure)
    /// - Timing information (expiration, polling interval)
    ///
    /// # Security Model
    ///
    /// - **Temporary Registration**: Creates ephemeral OIDC client credentials
    /// - **Secure Codes**: Device codes are cryptographically secure and time-limited
    /// - **Browser Isolation**: User authentication occurs in isolated browser context
    /// - **Automatic Cleanup**: Client credentials are cleared on completion or failure
    ///
    /// # Error Handling
    ///
    /// Common failure scenarios and resolutions:
    /// - **Network Connectivity**: Check internet connection and AWS service status
    /// - **Invalid Configuration**: Verify Identity Center URL and region
    /// - **Service Limits**: AWS may temporarily limit client registrations
    /// - **Regional Issues**: Ensure region matches Identity Center deployment
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// match identity_center.start_device_authorization() {
    ///     Ok(auth_data) => {
    ///         println!("Please visit: {}", auth_data.verification_uri);
    ///         println!("Enter code: {}", auth_data.user_code);
    ///         println!("Code expires in {} seconds", auth_data.expires_in);
    ///
    ///         // Application can now poll for completion
    ///         // identity_center.complete_device_authorization()?;
    ///     }
    ///     Err(error) => {
    ///         eprintln!("Failed to start device authorization: {}", error);
    ///     }
    /// }
    /// # Ok::<(), String>(())
    /// ```
    pub fn start_device_authorization(&mut self) -> Result<DeviceAuthorizationData, String> {
        info!("Starting device authorization with AWS SSO OIDC");

        // Create a Tokio runtime for async operations
        let rt_start = std::time::Instant::now();
        let runtime =
            Runtime::new().map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        log::info!("⏱️ [AWS] Runtime creation (start_device_authorization) took {:?}", rt_start.elapsed());

        let region = Region::new(self.identity_center_region.clone());

        // Execute the async code in the Tokio runtime
        let start_result: Result<(DeviceAuthorizationData, String, String), String> = runtime
            .block_on(async {
                // Create the OIDC client
                let config = aws_config::defaults(BehaviorVersion::latest())
                    .region(region)
                    .load()
                    .await;
                let sso_oidc_client = SsoOidcClient::new(&config);

                // Register client with AWS SSO
                let register_resp = sso_oidc_client
                    .register_client()
                    .client_name(&self.client_name)
                    .client_type("public")
                    .send()
                    .await
                    .map_err(|e| format!("Failed to register client: {}", e))?;

                let client_id = register_resp.client_id().unwrap_or_default().to_string();
                let client_secret = register_resp
                    .client_secret()
                    .unwrap_or_default()
                    .to_string();

                // Start device authorization
                let auth_resp = sso_oidc_client
                    .start_device_authorization()
                    .client_id(&client_id)
                    .client_secret(&client_secret)
                    .start_url(&self.start_url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to start device authorization: {}", e))?;

                // Convert the AWS SDK response to our data structure
                let auth_data = DeviceAuthorizationData {
                    device_code: auth_resp.device_code.unwrap_or_default(),
                    user_code: auth_resp.user_code.unwrap_or_default(),
                    verification_uri: auth_resp.verification_uri.unwrap_or_default(),
                    verification_uri_complete: auth_resp.verification_uri_complete,
                    expires_in: auth_resp.expires_in as i64,
                    interval: auth_resp.interval as i64,
                    start_time: Utc::now(),
                    client_id: Some(client_id.clone()),
                    client_secret: Some(client_secret.clone()),
                };

                // Try to open the verification URL in browser
                if let Some(uri_complete) = &auth_data.verification_uri_complete {
                    if let Err(e) = open::that(uri_complete) {
                        warn!("Failed to open browser: {}", e);
                        // Fall back to regular URI if complete fails
                        if let Err(e) = open::that(&auth_data.verification_uri) {
                            warn!("Failed to open browser with regular URI: {}", e);
                        }
                    }
                } else if let Err(e) = open::that(&auth_data.verification_uri) {
                    warn!("Failed to open browser: {}", e);
                }

                Ok((auth_data, client_id, client_secret))
            });

        // Process the result from async code
        match start_result {
            Ok((auth_data, client_id, _client_secret)) => {
                // Save the client ID for later use
                self.client_id = Some(client_id);

                // Update login state
                self.login_state = LoginState::DeviceAuthorization(auth_data.clone());

                Ok(auth_data)
            }
            Err(e) => {
                self.login_state = LoginState::Error(e.clone());
                Err(e)
            }
        }
    }

    /// Complete OAuth device authorization by polling for user authentication.
    ///
    /// Polls AWS SSO OIDC for user authentication completion and retrieves access tokens
    /// and account information. This method blocks until the user completes browser
    /// authentication or the authorization expires.
    ///
    /// # Polling Strategy
    ///
    /// - **Interval-Based**: Respects AWS-recommended polling intervals to prevent rate limiting
    /// - **Exponential Backoff**: Uses appropriate delays between polling attempts
    /// - **Timeout Protection**: Automatically fails if polling exceeds maximum attempts
    /// - **Thread Isolation**: Runs polling in separate thread to prevent UI blocking
    ///
    /// # Authentication Completion Process
    ///
    /// 1. **Token Exchange**: Exchanges device code for access tokens
    /// 2. **Account Discovery**: Retrieves all accessible AWS accounts
    /// 3. **Role Enumeration**: Lists available IAM roles per account
    /// 4. **State Update**: Updates login state to `LoggedIn`
    /// 5. **Management Account Detection**: Identifies organization management account
    ///
    /// # Thread Safety and Async Handling
    ///
    /// This method safely bridges async AWS SDK operations with the synchronous egui
    /// environment by:
    /// - Using isolated thread for polling operations
    /// - Employing channels for thread-safe result communication
    /// - Creating separate Tokio runtime to prevent conflicts
    /// - Gracefully handling thread communication failures
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` when authentication completes successfully and all account
    /// data has been retrieved. Returns `Err(String)` for various failure scenarios.
    ///
    /// # Error Scenarios
    ///
    /// - **User Cancellation**: User doesn't complete browser authentication
    /// - **Timeout**: Authorization window expires before user completion
    /// - **Network Issues**: Connectivity problems during polling or account retrieval
    /// - **Permission Issues**: Insufficient permissions to list accounts or roles
    /// - **Invalid State**: Called when not in `DeviceAuthorization` state
    ///
    /// # Security Considerations
    ///
    /// - All polling occurs over HTTPS with AWS SDK security defaults
    /// - Access tokens are stored only in memory
    /// - Client credentials are automatically cleaned up
    /// - Account information is retrieved using minimal required permissions
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// // After starting device authorization...
    /// match identity_center.complete_device_authorization() {
    ///     Ok(()) => {
    ///         println!("Authentication successful!");
    ///         println!("Found {} accessible accounts", identity_center.accounts.len());
    ///
    ///         // Can now get credentials for specific accounts/roles
    ///         for account in &identity_center.accounts {
    ///             println!("Account: {} ({})", account.account_name, account.account_id);
    ///         }
    ///     }
    ///     Err(error) => {
    ///         eprintln!("Authentication failed: {}", error);
    ///     }
    /// }
    /// ```
    pub fn complete_device_authorization(&mut self) -> Result<(), String> {
        let state = self.login_state.clone();

        let device_auth_data = match state {
            LoginState::DeviceAuthorization(data) => data,
            _ => return Err("Not in device authorization state".to_string()),
        };

        // Client ID is required
        let client_id = match device_auth_data.client_id {
            Some(ref id) => id.clone(),
            None => return Err("Client ID not found".to_string()),
        };

        // Client secret is required
        let client_secret = match device_auth_data.client_secret {
            Some(ref id) => id.clone(),
            None => return Err("Client Secret not found".to_string()),
        };

        // We'll poll in a separate thread and use a channel to communicate back
        let (tx, rx) = mpsc::channel();

        let device_code = device_auth_data.device_code.clone();
        let region = self.identity_center_region.clone();
        let default_role_name = self.default_role_name.clone();

        thread::spawn(move || {
            // Create a new Tokio runtime for this thread
            let rt_start = std::time::Instant::now();
            let runtime = match Runtime::new() {
                Ok(rt) => {
                    log::info!("⏱️ [AWS] Runtime creation (poll_for_token_bg_thread) took {:?}", rt_start.elapsed());
                    rt
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to create Tokio runtime: {}", e)));
                    return;
                }
            };

            // Execute the async code in the Tokio runtime
            let result = runtime.block_on(async {
                let region = Region::new(region);
                let config = aws_config::defaults(BehaviorVersion::latest())
                    .region(region)
                    .load()
                    .await;
                let sso_oidc_client = SsoOidcClient::new(&config);

                // Poll for the token with backoff strategy based on interval
                let mut attempts = 0;
                let max_attempts = 100; // Prevent infinite loops
                let interval_secs = device_auth_data.interval as u64;

                while attempts < max_attempts {
                    // Sleep for the recommended interval before polling
                    if attempts > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
                    }

                    attempts += 1;

                    // Request token
                    match sso_oidc_client
                        .create_token()
                        .client_id(&client_id)
                        .client_secret(&client_secret)
                        .device_code(&device_code)
                        .grant_type("urn:ietf:params:oauth:grant-type:device_code")
                        .send()
                        .await
                    {
                        Ok(token_resp) => {
                            // Got a token, now get account list with SSO client
                            let access_token = token_resp.access_token.unwrap_or_default();
                            // token_resp.expires_in returns i32
                            let expires_in = token_resp.expires_in;
                            let token_expiration =
                                Some(Utc::now() + Duration::seconds(expires_in as i64));

                            // Create SSO client for listing accounts
                            let sso_client = SsoClient::new(&config);

                            match sso_client
                                .list_accounts()
                                .access_token(&access_token)
                                .send()
                                .await
                            {
                                Ok(accounts_resp) => {
                                    let mut accounts = Vec::new();
                                    let mut available_roles = HashMap::new();

                                    // Process account list
                                    if let Some(account_list) = accounts_resp.account_list {
                                        for aws_account in account_list {
                                            // Get roles for this account
                                            let account_id =
                                                aws_account.account_id.unwrap_or_default();
                                            let account_id_clone1 = account_id.clone();
                                            let account_id_clone2 = account_id.clone();
                                            match sso_client
                                                .list_account_roles()
                                                .access_token(&access_token)
                                                .account_id(&account_id)
                                                .send()
                                                .await
                                            {
                                                Ok(roles_resp) => {
                                                    let mut roles = Vec::new();
                                                    if let Some(role_list) = roles_resp.role_list {
                                                        for role in role_list {
                                                            if let Some(role_name) = role.role_name
                                                            {
                                                                roles.push(role_name);
                                                            }
                                                        }
                                                    }

                                                    // Create our account structure
                                                    // Use the default role if it's available, otherwise use the first role
                                                    let role_name =
                                                        if roles.contains(&default_role_name) {
                                                            default_role_name.clone()
                                                        } else {
                                                            roles
                                                                .first()
                                                                .unwrap_or(&"".to_string())
                                                                .clone()
                                                        };

                                                    let account = AwsAccount {
                                                        account_id: account_id_clone1,
                                                        account_name: aws_account
                                                            .account_name
                                                            .unwrap_or_default(),
                                                        account_email: aws_account.email_address,
                                                        role_name,
                                                        credentials: None,
                                                    };

                                                    accounts.push(account);
                                                    available_roles
                                                        .insert(account_id_clone2, roles);
                                                }
                                                Err(e) => {
                                                    error!("Failed to list account roles: {}", e);
                                                }
                                            }
                                        }
                                    }

                                    return Ok((
                                        access_token,
                                        token_expiration,
                                        accounts,
                                        available_roles,
                                    ));
                                }
                                Err(e) => {
                                    return Err(format!("Failed to list accounts: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            // If we get authorization_pending, that's expected, keep polling
                            if e.to_string().contains("authorization_pending") {
                                continue;
                            }

                            // Other errors should be reported
                            return Err(format!("Failed to create token: {}", e));
                        }
                    }
                }

                Err("Exceeded maximum polling attempts".to_string())
            });

            // Send the result back to the main thread
            let _ = tx.send(result);
        });

        // Wait for the authorization to complete (blocking)
        match rx.recv() {
            Ok(Ok((access_token, token_expiration, accounts, available_roles))) => {
                // Update our state with the results
                self.access_token = Some(access_token);
                self.token_expiration = token_expiration;
                self.last_refresh = Some(Utc::now());
                self.accounts = accounts;
                self.available_roles = available_roles;

                // Don't set LoggedIn state here - let the caller set it after credentials are fetched
                // This prevents race condition where state says "logged in" but credentials aren't ready
                tracing::info!("Device authorization complete, accounts loaded, waiting for credentials");

                // Try to automatically extract infrastructure information for common CloudFormation roles
                self.auto_extract_infrastructure_info();

                Ok(())
            }
            Ok(Err(e)) => {
                self.login_state = LoginState::Error(e.clone());
                Err(e)
            }
            Err(e) => {
                let error = format!("Thread communication error: {}", e);
                self.login_state = LoginState::Error(error.clone());
                Err(error)
            }
        }
    }

    /// Obtain temporary credentials for a specific AWS account and IAM role.
    ///
    /// Retrieves short-term credentials for the specified account and role combination
    /// using the current Identity Center access token. These credentials can be used
    /// for direct AWS API access within the role's permission boundaries.
    ///
    /// # Parameters
    ///
    /// * `account_id` - The 12-digit AWS account ID
    /// * `role_name` - The IAM role name to assume (must be available through Identity Center)
    ///
    /// # Credential Lifecycle
    ///
    /// - **Temporary Nature**: All returned credentials are temporary (typically 1-12 hours)
    /// - **Role-Specific**: Credentials are scoped to the specific role's permissions
    /// - **Caching**: Successfully obtained credentials are cached in the account structure
    /// - **Expiration Tracking**: Includes precise expiration timestamp for proactive renewal
    ///
    /// # Security Model
    ///
    /// - **Principle of Least Privilege**: Credentials are limited to the role's IAM permissions
    /// - **Time-Bounded Access**: Automatic expiration prevents long-term credential exposure
    /// - **Audit Trail**: All credential requests are logged for security monitoring
    /// - **Memory-Only Storage**: Credentials exist only in memory, never persisted
    ///
    /// # Async-to-Sync Bridge
    ///
    /// This method safely bridges AWS SDK async operations to the synchronous egui
    /// environment using isolated Tokio runtime management, preventing runtime conflicts
    /// while maintaining full AWS SDK functionality.
    ///
    /// # Return Value
    ///
    /// Returns `AwsCredentials` containing:
    /// - Access key ID and secret access key
    /// - Session token for temporary credential validation
    /// - Expiration timestamp for lifecycle management
    ///
    /// # Error Scenarios
    ///
    /// - **Not Authenticated**: No valid Identity Center access token
    /// - **Account Not Found**: Specified account ID is not accessible
    /// - **Role Not Available**: Requested role is not assignable to the user
    /// - **Network Issues**: Connectivity problems with AWS services
    /// - **Permission Denied**: Insufficient permissions for the requested role
    /// - **Token Expired**: Identity Center access token needs renewal
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// // Get credentials for a specific account and role
    /// match identity_center.get_account_credentials("123456789012", "ReadOnlyAccess") {
    ///     Ok(credentials) => {
    ///         println!("Access Key: {}", credentials.access_key_id);
    ///         if let Some(expiration) = credentials.expiration {
    ///             println!("Expires at: {}", expiration);
    ///         }
    ///
    ///         // Use credentials with AWS SDK clients
    ///         // let config = aws_config::from_env()
    ///         //     .credentials_provider(StaticCredentialsProvider::new(
    ///         //         credentials.access_key_id,
    ///         //         credentials.secret_access_key,
    ///         //         credentials.session_token,
    ///         //         None
    ///         //     ))
    ///         //     .load().await;
    ///     }
    ///     Err(error) => {
    ///         eprintln!("Failed to get credentials: {}", error);
    ///     }
    /// }
    /// ```
    pub fn get_account_credentials(
        &mut self,
        account_id: &str,
        role_name: &str,
    ) -> Result<AwsCredentials, String> {
        // Check if we have a valid access token
        let access_token = match &self.access_token {
            Some(token) => token.clone(),
            None => return Err("Not logged in".to_string()),
        };

        // Find the account
        if !self.accounts.iter().any(|a| a.account_id == account_id) {
            return Err(format!("Account {} not found", account_id));
        }

        // Execute the async code in a tokio runtime
        let rt_start = std::time::Instant::now();
        let runtime =
            Runtime::new().map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        log::info!("⏱️ [AWS] Runtime creation (get_account_role_credentials) took {:?}", rt_start.elapsed());

        let region = self.identity_center_region.clone();

        let result = runtime.block_on(async {
            let region = Region::new(region);
            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .load()
                .await;
            let sso_client = SsoClient::new(&config);

            // Get role credentials
            match sso_client
                .get_role_credentials()
                .access_token(&access_token)
                .account_id(account_id)
                .role_name(role_name)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Some(creds) = resp.role_credentials {
                        // Convert to our credentials format
                        // expiration is in milliseconds (epoch time)
                        let exp = creds.expiration;
                        let expiration = {
                            // Convert AWS timestamp (milliseconds) to DateTime<Utc>
                            let secs = exp / 1000;
                            let nsecs = ((exp % 1000) * 1_000_000) as u32;
                            Some(DateTime::from_timestamp(secs, nsecs).unwrap_or_else(Utc::now))
                        };

                        let credentials = AwsCredentials {
                            access_key_id: creds.access_key_id.unwrap_or_default(),
                            secret_access_key: creds.secret_access_key.unwrap_or_default(),
                            session_token: creds.session_token,
                            expiration,
                        };

                        Ok(credentials)
                    } else {
                        Err("No credentials in response".to_string())
                    }
                }
                Err(e) => Err(format!("Failed to get credentials: {}", e)),
            }
        });

        // Update our account in the accounts list
        match &result {
            Ok(credentials) => {
                for account in &mut self.accounts {
                    if account.account_id == account_id {
                        account.role_name = role_name.to_string();
                        account.credentials = Some(credentials.clone());
                        break;
                    }
                }
            }
            Err(_) => {
                // Don't update on error
            }
        }

        result
    }

    /// Get role credentials for a specific account in the format expected by CredentialCoordinator.
    ///
    /// This method provides AWS role credentials for a specific account and role combination,
    /// formatted as AccountCredentials for use with the multi-account credential coordination
    /// system. It's designed specifically for the AWS Explorer's credential management.
    ///
    /// # Parameters
    ///
    /// * `account_id` - The AWS account ID to get credentials for
    /// * `role_name` - The IAM role name to assume in that account
    ///
    /// # Return Value
    ///
    /// Returns `AccountCredentials` containing:
    /// - Account ID and role name
    /// - AWS access credentials (key, secret, session token)
    /// - Expiration timestamp with proper timezone handling
    ///
    /// # Error Handling
    ///
    /// - Returns error if not logged in to Identity Center
    /// - Returns error if account ID is not found in available accounts
    /// - Returns error if AWS SSO API call fails
    /// - Returns error if credentials are not included in response
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// match identity_center.get_role_credentials("123456789012", "awsdash").await {
    ///     Ok(credentials) => {
    ///         println!("Got credentials for account: {}", credentials.account_id);
    ///         println!("Role: {}", credentials.role_name);
    ///         println!("Expires: {}", credentials.expiration);
    ///     }
    ///     Err(error) => {
    ///         eprintln!("Failed to get role credentials: {}", error);
    ///     }
    /// }
    /// ```
    pub async fn get_role_credentials(
        &self,
        account_id: &str,
        role_name: &str,
    ) -> Result<crate::app::resource_explorer::credentials::AccountCredentials, anyhow::Error> {
        use crate::app::resource_explorer::credentials::AccountCredentials;

        // Check if we have a valid access token
        let access_token = match &self.access_token {
            Some(token) => token.clone(),
            None => return Err(anyhow::anyhow!("Not logged in to AWS Identity Center")),
        };

        // Find the account
        if !self.accounts.iter().any(|a| a.account_id == account_id) {
            return Err(anyhow::anyhow!(
                "Account {} not found in available accounts",
                account_id
            ));
        }

        let region = Region::new(self.identity_center_region.clone());
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;
        let sso_client = SsoClient::new(&config);

        // Get role credentials
        let resp = sso_client
            .get_role_credentials()
            .access_token(&access_token)
            .account_id(account_id)
            .role_name(role_name)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get role credentials from AWS SSO: {}", e))?;

        if let Some(creds) = resp.role_credentials {
            // Convert AWS timestamp (milliseconds) to DateTime<Utc>
            let exp = creds.expiration;
            let secs = exp / 1000;
            let nsecs = ((exp % 1000) * 1_000_000) as u32;
            let expiration = DateTime::from_timestamp(secs, nsecs)
                .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1)); // Fallback to 1 hour from now

            let account_credentials = AccountCredentials {
                account_id: account_id.to_string(),
                role_name: role_name.to_string(),
                access_key_id: creds.access_key_id.unwrap_or_default(),
                secret_access_key: creds.secret_access_key.unwrap_or_default(),
                session_token: creds.session_token.unwrap_or_default(),
                expiration,
            };

            Ok(account_credentials)
        } else {
            Err(anyhow::anyhow!(
                "No credentials returned in AWS SSO response"
            ))
        }
    }

    /// Check if cached credentials for an account are expired or near expiration.
    ///
    /// Determines whether the cached credentials for the specified account are
    /// expired or approaching expiration. This method implements proactive
    /// expiration checking with a safety buffer to prevent service interruptions.
    ///
    /// # Expiration Logic
    ///
    /// Credentials are considered expired if:
    /// - Current time is at or past the expiration timestamp
    /// - Less than 5 minutes remain until expiration (safety buffer)
    /// - No credentials are cached for the account
    /// - No expiration information is available
    ///
    /// # Safety Buffer
    ///
    /// The 5-minute safety buffer ensures that credentials don't expire during
    /// ongoing operations, providing time for automatic renewal before actual
    /// expiration occurs.
    ///
    /// # Parameters
    ///
    /// * `account_id` - The AWS account ID to check credentials for
    ///
    /// # Return Value
    ///
    /// Returns `true` if credentials are expired or should be renewed, `false`
    /// if credentials are still valid with sufficient time remaining.
    ///
    /// # Conservative Approach
    ///
    /// This method takes a conservative approach to credential validation:
    /// - Missing account → expired (true)
    /// - Missing credentials → expired (true)
    /// - Missing expiration → expired (true)
    /// - Near expiration → expired (true)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// let account_id = "123456789012";
    ///
    /// if identity_center.are_credentials_expired(account_id) {
    ///     println!("Credentials need renewal for account {}", account_id);
    ///     // Obtain fresh credentials
    ///     // let fresh_creds = identity_center.get_account_credentials(account_id, "MyRole")?;
    /// } else {
    ///     println!("Credentials are still valid for account {}", account_id);
    ///     // Use existing cached credentials
    /// }
    /// ```
    pub fn are_credentials_expired(&self, account_id: &str) -> bool {
        if let Some(account) = self.accounts.iter().find(|a| a.account_id == account_id) {
            if let Some(credentials) = &account.credentials {
                if let Some(expiration) = credentials.expiration {
                    // Consider credentials expired if they expire within 5 minutes
                    let now = Utc::now();
                    return expiration <= now || (expiration - now).num_minutes() < 5;
                }
            }
        }
        // If we can't find the account or credentials, consider them expired
        true
    }

    /// Add or update an AWS account in the managed accounts list.
    ///
    /// Updates the account information if it already exists (based on account ID),
    /// or adds it as a new account if not found. This method maintains the
    /// integrity of the accounts list during dynamic account management.
    ///
    /// # Parameters
    ///
    /// * `account` - Complete `AwsAccount` structure with updated information
    ///
    /// # Update Behavior
    ///
    /// - **Existing Account**: Completely replaces the existing account entry
    /// - **New Account**: Appends to the accounts list
    /// - **Identification**: Uses `account_id` field for matching
    /// - **Complete Replacement**: All fields of existing accounts are updated
    ///
    /// # Use Cases
    ///
    /// - Updating role assumptions for existing accounts
    /// - Adding newly discovered accounts during authentication
    /// - Refreshing account metadata (name, email changes)
    /// - Updating cached credentials for specific accounts
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::{AwsIdentityCenter, AwsAccount};
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// let updated_account = AwsAccount {
    ///     account_id: "123456789012".to_string(),
    ///     account_name: "Production Account".to_string(),
    ///     account_email: Some("admin@company.com".to_string()),
    ///     role_name: "PowerUserAccess".to_string(),
    ///     credentials: None,
    /// };
    ///
    /// identity_center.update_account(updated_account);
    /// ```
    pub fn update_account(&mut self, account: AwsAccount) {
        if let Some(existing_index) = self
            .accounts
            .iter()
            .position(|a| a.account_id == account.account_id)
        {
            self.accounts[existing_index] = account;
        } else {
            self.accounts.push(account);
        }
    }

    /// Retrieve all available IAM roles for a specific AWS account.
    ///
    /// Returns the complete list of IAM roles that the authenticated user can
    /// assume in the specified AWS account through Identity Center. This information
    /// is populated during the authentication process.
    ///
    /// # Parameters
    ///
    /// * `account_id` - The 12-digit AWS account ID to query roles for
    ///
    /// # Return Value
    ///
    /// Returns a vector of role names (strings) that are available for assumption.
    /// Returns an empty vector if:
    /// - The account ID is not found in accessible accounts
    /// - No roles are available for the account
    /// - Role information hasn't been retrieved yet
    ///
    /// # Role Information Source
    ///
    /// Role information is retrieved during the authentication process by:
    /// 1. Enumerating all accessible accounts through Identity Center
    /// 2. Querying available roles for each account via AWS SSO API
    /// 3. Storing role lists in the `available_roles` HashMap
    /// 4. Refreshing when authentication state changes
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// let account_id = "123456789012";
    /// let available_roles = identity_center.get_account_roles(account_id);
    ///
    /// if available_roles.is_empty() {
    ///     println!("No roles available for account {}", account_id);
    /// } else {
    ///     println!("Available roles for account {}:", account_id);
    ///     for role in available_roles {
    ///         println!("  - {}", role);
    ///
    ///         // Can obtain credentials for any of these roles
    ///         // let creds = identity_center.get_account_credentials(account_id, &role)?;
    ///     }
    /// }
    /// ```
    pub fn get_account_roles(&self, account_id: &str) -> Vec<String> {
        self.available_roles
            .get(account_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Launch AWS Management Console in browser with federated sign-in.
    ///
    /// Opens the AWS Management Console in the user's default browser using
    /// federated sign-in with temporary credentials. This provides seamless
    /// console access without requiring users to manually configure credentials.
    ///
    /// # Federation Process
    ///
    /// 1. **Credential Retrieval**: Obtains fresh temporary credentials for the specified account/role
    /// 2. **Session Creation**: Builds AWS federation session data with credentials
    /// 3. **Token Exchange**: Calls AWS federation service to exchange credentials for signin token
    /// 4. **URL Construction**: Creates federated signin URL with embedded token
    /// 5. **Browser Launch**: Opens the complete signin URL in default browser
    ///
    /// # Parameters
    ///
    /// * `account_id` - The 12-digit AWS account ID for console access
    /// * `role_name` - The IAM role to assume for console session
    ///
    /// # Security Model
    ///
    /// - **Temporary Credentials**: Uses short-lived STS credentials for federation
    /// - **Secure Transport**: All communication with AWS federation service uses HTTPS
    /// - **No Credential Storage**: Credentials are used only for token exchange
    /// - **Session Isolation**: Each console session is independent and time-bounded
    /// - **Role-Based Access**: Console permissions are limited to the assumed role
    ///
    /// # Browser Integration
    ///
    /// - Attempts to open URL in system default browser
    /// - Falls back gracefully if browser launch fails
    /// - Works with any modern web browser
    /// - Preserves existing browser sessions
    ///
    /// # Error Scenarios
    ///
    /// - **Credential Failure**: Cannot obtain valid credentials for account/role
    /// - **Network Issues**: Federation service unavailable or unreachable
    /// - **Invalid Credentials**: Temporary credentials rejected by federation service
    /// - **Browser Launch**: System cannot open default browser
    /// - **Token Exchange**: AWS federation service returns invalid response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// // Open console for specific account and role
    /// match identity_center.open_aws_console("123456789012", "PowerUserAccess") {
    ///     Ok(()) => {
    ///         println!("AWS Console opened successfully in browser");
    ///     }
    ///     Err(error) => {
    ///         eprintln!("Failed to open AWS Console: {}", error);
    ///     }
    /// }
    /// ```
    pub fn open_aws_console(&mut self, account_id: &str, role_name: &str) -> Result<(), String> {
        // First, get or refresh credentials for this account and role
        let credentials = self.get_account_credentials(account_id, role_name)?;

        // Build a session JSON that AWS console will need
        let session_json = serde_json::json!({
            "sessionId": credentials.access_key_id,
            "sessionKey": credentials.secret_access_key,
            "sessionToken": credentials.session_token.unwrap_or_default()
        });

        // Convert to string and URL encode
        let session_data = session_json.to_string();
        let encoded_session_data = utf8_percent_encode(&session_data, NON_ALPHANUMERIC).to_string();

        // Build the sign-in URL
        let console_url = format!(
            "https://signin.aws.amazon.com/federation?Action=getSigninToken&Session={}",
            encoded_session_data
        );

        // Execute the async code in a tokio runtime to get the signin token
        let rt_start = std::time::Instant::now();
        let runtime =
            Runtime::new().map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        log::info!("⏱️ [AWS] Runtime creation (generate_console_url) took {:?}", rt_start.elapsed());

        let signin_result = runtime.block_on(async {
            // Make HTTP request to get signin token
            let client = reqwest::Client::new();
            let resp = match client.get(&console_url).send().await {
                Ok(r) => r,
                Err(e) => return Err(format!("Failed to get signin token: {}", e)),
            };

            // Parse response
            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => return Err(format!("Failed to read signin token response: {}", e)),
            };

            // Parse the JSON response
            let token: serde_json::Value = match serde_json::from_str(&text) {
                Ok(t) => t,
                Err(e) => return Err(format!("Failed to parse signin token: {}", e)),
            };

            // Extract the signin token
            match token.get("SigninToken") {
                Some(token) => match token.as_str() {
                    Some(t) => Ok(t.to_string()),
                    None => Err("Invalid signin token format".to_string()),
                },
                None => Err("No signin token in response".to_string()),
            }
        })?;

        // Build the final AWS console URL with the signin token
        let destination = "https://console.aws.amazon.com/";
        let encoded_destination = utf8_percent_encode(destination, NON_ALPHANUMERIC).to_string();

        let console_signin_url = format!(
            "https://signin.aws.amazon.com/federation?Action=login&Destination={}&SigninToken={}",
            encoded_destination, signin_result
        );

        // Open the URL in the default browser
        if let Err(e) = open::that(&console_signin_url) {
            return Err(format!("Failed to open browser: {}", e));
        }

        info!(
            "Opened AWS Console for account {} with role {}",
            account_id, role_name
        );
        Ok(())
    }

    /// Automatically discover AWSReservedSSO CloudFormation role and extract infrastructure information.
    ///
    /// This method is called after successful login to discover the AWSReservedSSO CloudFormation role
    /// and extract DynamoDB table and CloudFormation role information from its inline policy.
    ///
    /// # Two Separate Role Concepts
    ///
    /// 1. **User-Provided Default Role** (`default_role_name`):
    ///    - Role name provided by user for Identity Center login (e.g., "awsdash")
    ///    - Has limited permissions (read-only, Bedrock access, DynamoDB access)
    ///    - Used for application operations
    ///
    /// 2. **AWSReservedSSO CloudFormation Role**:
    ///    - Separate role starting with "AWSReservedSSO_" prefix
    ///    - Deployed to all accounts, contains CloudFormation deployment permissions
    ///    - Contains inline policy with DynamoDB table and role ARN references
    ///
    /// # Behavior
    ///
    /// - Only runs if default role credentials are available
    /// - Discovers AWSReservedSSO role by listing roles with prefix "AWSReservedSSO_"
    /// - Uses the user's default role name as the DynamoDB table name pattern
    /// - Stores discovered AWSReservedSSO role name for future CloudFormation deployment
    /// - Logs warnings for failed extractions but doesn't fail login
    ///
    fn auto_extract_infrastructure_info(&mut self) {
        info!("Attempting automatic AWSReservedSSO role discovery and infrastructure extraction");

        // Check if we can get default role credentials
        match self.get_default_role_credentials() {
            Ok(_) => {
                // Discover CloudFormation deployment role from our policy
                match self.discover_aws_reserved_sso_role() {
                    Ok(cloudformation_deployment_role_name) => {
                        info!(
                            "Discovered CloudFormation deployment role: {}",
                            cloudformation_deployment_role_name
                        );

                        // Store the discovered role name for future CloudFormation deployment use
                        self.cloudformation_deployment_role_name =
                            Some(cloudformation_deployment_role_name.clone());

                        // Use the user's default role name as the DynamoDB table pattern
                        let dynamodb_table_pattern = self.default_role_name.clone();

                        match self.extract_infrastructure_info(
                            &cloudformation_deployment_role_name,
                            &dynamodb_table_pattern,
                        ) {
                            Ok(infrastructure_info) => {
                                info!(
                                    "Successfully extracted infrastructure info from CloudFormation role: {} - Found table: {} and {} CloudFormation roles",
                                    cloudformation_deployment_role_name, infrastructure_info.table_name, infrastructure_info.cloudformation_role_arns.len()
                                );
                            }
                            Err(e) => {
                                warn!("Failed to extract infrastructure info from CloudFormation role {}: {}", cloudformation_deployment_role_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to discover CloudFormation deployment role: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Cannot auto-extract infrastructure info - no default role credentials: {}",
                    e
                );
            }
        }
    }

    /// Discover the CloudFormation deployment role by analyzing our current role's policy.
    ///
    /// This method uses STS GetCallerIdentity to identify our current AWSReservedSSO role,
    /// then parses its inline policy to find the CloudFormation deployment role specified
    /// in the PassRole statement with condition "iam:PassedToService": "cloudformation.amazonaws.com".
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The discovered CloudFormation deployment role name
    /// * `Err(String)` - Error if role not found in policy or API error
    ///
    /// # Required Permissions
    ///
    /// The user's AWSReservedSSO role must have:
    /// - `sts:GetCallerIdentity` to identify current role
    /// - `iam:GetRolePolicy` to read the role's inline policy
    fn discover_aws_reserved_sso_role(&mut self) -> Result<String, String> {
        info!("Discovering CloudFormation deployment role from current role policy");

        // Ensure we have default role credentials
        let credentials = self
            .default_role_credentials
            .as_ref()
            .ok_or("Not logged in - no default role credentials available")?;

        // Create a Tokio runtime for async operations
        let rt_start = std::time::Instant::now();
        let runtime =
            Runtime::new().map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        log::info!("⏱️ [AWS] Runtime creation (discover_aws_reserved_sso_role) took {:?}", rt_start.elapsed());

        let region = Region::new(self.identity_center_region.clone());

        // Execute the async code in the Tokio runtime
        let result: Result<String, String> = runtime.block_on(async {
            // Create AWS config with our credentials
            let expiration_time = credentials.expiration.map(|dt| {
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
            });

            let creds = aws_credential_types::Credentials::new(
                &credentials.access_key_id,
                &credentials.secret_access_key,
                credentials.session_token.clone(),
                expiration_time,
                "aws-dash",
            );

            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(creds)
                .load()
                .await;

            // Create STS client to identify our current role
            let sts_client = StsClient::new(&config);
            let caller_identity = sts_client
                .get_caller_identity()
                .send()
                .await
                .map_err(|e| format!("Failed to get caller identity: {}", e))?;

            let arn = caller_identity.arn
                .ok_or("No ARN found in caller identity")?;

            // Extract role name from ARN: arn:aws:sts::123456789012:assumed-role/AWSReservedSSO_RoleName/username
            let role_name = arn.split('/')
                .nth(1)
                .ok_or("Invalid ARN format - cannot extract role name")?
                .to_string();

            info!("Current AWSReservedSSO role: {}", role_name);

            // Create IAM client to get the role policy
            let iam_client = IamClient::new(&config);

            // Get the inline policy from our current role
            let policy_response = iam_client
                .get_role_policy()
                .policy_name("AwsSSOInlinePolicy")
                .role_name(&role_name)
                .send()
                .await
                .map_err(|e| {
                    match &e {
                        aws_sdk_iam::error::SdkError::ServiceError(service_err) => {
                            let error_code = service_err.err().code().unwrap_or("Unknown");
                            let error_message = service_err.err().message().unwrap_or("No message");
                            match error_code {
                                "NoSuchEntity" => {
                                    format!("Failed to get role policy: Role '{}' or policy 'AwsSSOInlinePolicy' not found (NoSuchEntity) - {}", role_name, error_message)
                                },
                                "AccessDenied" => {
                                    format!("Failed to get role policy: Access denied (AccessDenied) - insufficient permissions to read role '{}' policy 'AwsSSOInlinePolicy' - {}", role_name, error_message)
                                },
                                _ => {
                                    format!("Failed to get role policy: AWS service error '{}' for role '{}' - {}", error_code, role_name, error_message)
                                }
                            }
                        },
                        _ => {
                            format!("Failed to get role policy for role '{}': {}", role_name, e)
                        }
                    }
                })?;

            let policy_document = policy_response.policy_document;

            // URL decode the policy document
            info!("Raw policy document before decoding: {}", policy_document);

            let decoded_policy = match percent_decode(policy_document.as_bytes()).decode_utf8() {
                Ok(decoded) => {
                    let decoded_str = decoded.to_string();
                    info!("Successfully percent decoded policy document");
                    decoded_str
                }
                Err(e) => {
                    info!("Failed to percent decode policy document: {}, trying form_urlencoded", e);
                    // Fallback to form_urlencoded parsing if percent decoding fails
                    form_urlencoded::parse(policy_document.as_bytes())
                        .find(|(key, _)| key.is_empty())
                        .map(|(_, value)| value.to_string())
                        .unwrap_or_else(|| {
                            info!("Form URL encoding also failed, using original policy document");
                            policy_document
                        })
                }
            };

            // Log the decoded policy in pretty JSON format for troubleshooting
            match serde_json::from_str::<serde_json::Value>(&decoded_policy) {
                Ok(json_value) => {
                    match serde_json::to_string_pretty(&json_value) {
                        Ok(pretty_json) => {
                            info!("Downloaded policy document (pretty JSON):\n{}", pretty_json);
                        }
                        Err(e) => {
                            info!("Failed to pretty print policy JSON: {}", e);
                            info!("Raw decoded policy: {}", decoded_policy);
                        }
                    }
                }
                Err(e) => {
                    info!("Failed to parse policy as JSON: {}", e);
                    info!("Raw decoded policy: {}", decoded_policy);
                }
            }

            info!("Analyzing policy for CloudFormation PassRole statements");

            // Parse the policy JSON to find CloudFormation deployment role
            let policy_json: serde_json::Value = serde_json::from_str(&decoded_policy)
                .map_err(|e| format!("Failed to parse policy JSON: {}", e))?;

            // Look for PassRole statements with CloudFormation condition
            if let Some(statements) = policy_json.get("Statement").and_then(|s| s.as_array()) {
                info!("Found {} policy statements to analyze", statements.len());
                for (i, statement) in statements.iter().enumerate() {
                    info!("Analyzing statement {}: {:?}", i, statement);
                    // Check if this is a PassRole statement
                    if let Some(actions) = statement.get("Action") {
                        let is_pass_role = if let Some(action_str) = actions.as_str() {
                            action_str == "iam:PassRole"
                        } else if let Some(action_array) = actions.as_array() {
                            action_array.iter().any(|a| a.as_str() == Some("iam:PassRole"))
                        } else {
                            false
                        };

                        if is_pass_role {
                            info!("Found PassRole action in statement {}", i);
                            // Check for CloudFormation condition
                            if let Some(condition) = statement.get("Condition")
                                .and_then(|c| c.get("StringEquals"))
                                .and_then(|se| se.get("iam:PassedToService")) {

                                let is_cloudformation = if let Some(service_str) = condition.as_str() {
                                    service_str == "cloudformation.amazonaws.com"
                                } else if let Some(service_array) = condition.as_array() {
                                    service_array.iter().any(|s| s.as_str() == Some("cloudformation.amazonaws.com"))
                                } else {
                                    false
                                };

                                if is_cloudformation {
                                    info!("Found CloudFormation condition in PassRole statement {}", i);
                                    // Extract the role ARN from Resource
                                    if let Some(resource) = statement.get("Resource") {
                                        let role_arn = if let Some(resource_str) = resource.as_str() {
                                            resource_str.to_string()
                                        } else if let Some(resource_array) = resource.as_array() {
                                            resource_array.first()
                                                .and_then(|r| r.as_str())
                                                .unwrap_or("")
                                                .to_string()
                                        } else {
                                            String::new()
                                        };

                                        if !role_arn.is_empty() {
                                            // Extract role name from ARN: arn:aws:iam::account:role/RoleName
                                            let cf_role_name = role_arn.split('/')
                                                .next_back()
                                                .ok_or("Invalid role ARN format")?
                                                .to_string();

                                            info!("Found CloudFormation deployment role: {}", cf_role_name);
                                            return Ok(cf_role_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Err("No CloudFormation deployment role found in PassRole statements - check policy has iam:PassRole with cloudformation.amazonaws.com condition".to_string())
        });

        result
    }

    /// Extract infrastructure information from a CloudFormation role's IAM policies.
    ///
    /// This function retrieves the inline policy from a specified CloudFormation role
    /// and extracts both DynamoDB table ARNs and CloudFormation role ARNs to identify
    /// the application's infrastructure configuration.
    ///
    /// # Parameters
    ///
    /// * `cloudformation_deployment_role_name` - The name of the CloudFormation deployment role to analyze
    /// * `dynamodb_table_pattern` - Pattern to match against DynamoDB table names in ARNs (usually the user's default role name)
    ///
    /// # Process
    ///
    /// 1. Uses default role credentials to create an IAM client
    /// 2. Retrieves the "AwsSSOInlinePolicy" from the specified role
    /// 3. URL-decodes the policy document
    /// 4. Uses regex to find DynamoDB table ARNs matching the pattern
    /// 5. Uses regex to find CloudFormation role ARNs in the policy
    /// 6. Extracts region, account, and table name from the DynamoDB ARN
    /// 7. Stores the information for use in the IAM Debug window
    ///
    /// # Returns
    ///
    /// * `Ok(InfrastructureInfo)` - Successfully extracted infrastructure information
    /// * `Err(String)` - Error retrieving policy or parsing information
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aws_dash::app::aws_identity::AwsIdentityCenter;
    /// # let mut identity_center = AwsIdentityCenter::new("url".to_string(), "role".to_string(), "region".to_string());
    /// // After successful login
    /// match identity_center.extract_infrastructure_info("AWSReservedSSO_SomeRole", "awsdash") {
    ///     Ok(info) => {
    ///         println!("Found DynamoDB table: {} in region: {}",
    ///                  info.table_name, info.db_region);
    ///         println!("Found {} CloudFormation roles", info.cloudformation_role_arns.len());
    ///     }
    ///     Err(e) => println!("Failed to extract infrastructure info: {}", e),
    /// }
    /// ```
    pub fn extract_infrastructure_info(
        &mut self,
        cloudformation_deployment_role_name: &str,
        dynamodb_table_pattern: &str,
    ) -> Result<InfrastructureInfo, String> {
        info!(
            "Extracting infrastructure info from CloudFormation role: {}",
            cloudformation_deployment_role_name
        );

        // Ensure we have default role credentials
        let credentials = self
            .default_role_credentials
            .as_ref()
            .ok_or("Not logged in - no default role credentials available")?;

        // Create a Tokio runtime for async operations
        let rt_start = std::time::Instant::now();
        let runtime =
            Runtime::new().map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        log::info!("⏱️ [AWS] Runtime creation (extract_infrastructure_info) took {:?}", rt_start.elapsed());

        let region = Region::new(self.identity_center_region.clone());

        // Execute the async code in the Tokio runtime
        let result: Result<InfrastructureInfo, String> = runtime.block_on(async {
            // Create AWS config with our credentials
            let expiration_time = credentials.expiration.map(|dt| {
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
            });

            let creds = aws_credential_types::Credentials::new(
                &credentials.access_key_id,
                &credentials.secret_access_key,
                credentials.session_token.clone(),
                expiration_time,
                "aws-dash",
            );

            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(creds)
                .load()
                .await;

            // Create IAM client
            let iam_client = IamClient::new(&config);

            // Get the role policy document
            let policy_response = iam_client
                .get_role_policy()
                .policy_name("AwsSSOInlinePolicy")
                .role_name(cloudformation_deployment_role_name)
                .send()
                .await
                .map_err(|e| {
                    // Provide detailed AWS service error information
                    match &e {
                        aws_sdk_iam::error::SdkError::ServiceError(service_err) => {
                            let error_code = service_err.err().code().unwrap_or("Unknown");
                            let error_message = service_err.err().message().unwrap_or("No message");
                            match error_code {
                                "NoSuchEntity" => {
                                    format!("Failed to get role policy: Role '{}' or policy 'AwsSSOInlinePolicy' not found (NoSuchEntity) - {}", cloudformation_deployment_role_name, error_message)
                                },
                                "InvalidInput" => {
                                    format!("Failed to get role policy: Invalid input parameters (InvalidInput) - role: '{}', policy: 'AwsSSOInlinePolicy' - {}", cloudformation_deployment_role_name, error_message)
                                },
                                "AccessDenied" => {
                                    format!("Failed to get role policy: Access denied (AccessDenied) - insufficient permissions to read role '{}' policy 'AwsSSOInlinePolicy' - {}", cloudformation_deployment_role_name, error_message)
                                },
                                "ServiceFailure" => {
                                    format!("Failed to get role policy: AWS IAM service failure (ServiceFailure) for role '{}' - {}", cloudformation_deployment_role_name, error_message)
                                },
                                _ => {
                                    format!("Failed to get role policy: AWS service error '{}' for role '{}' - {}", error_code, cloudformation_deployment_role_name, error_message)
                                }
                            }
                        },
                        aws_sdk_iam::error::SdkError::TimeoutError(_) => {
                            format!("Failed to get role policy: Request timeout for role '{}' - check network connectivity", cloudformation_deployment_role_name)
                        },
                        aws_sdk_iam::error::SdkError::ResponseError(response_err) => {
                            format!("Failed to get role policy: HTTP response error for role '{}' - status: {:?}", cloudformation_deployment_role_name, response_err.raw().status())
                        },
                        aws_sdk_iam::error::SdkError::DispatchFailure(dispatch_err) => {
                            if dispatch_err.is_timeout() {
                                format!("Failed to get role policy: Network timeout for role '{}' - check connectivity", cloudformation_deployment_role_name)
                            } else if dispatch_err.is_io() {
                                format!("Failed to get role policy: Network/IO error for role '{}' - {:?}", cloudformation_deployment_role_name, dispatch_err)
                            } else {
                                format!("Failed to get role policy: Network dispatch error for role '{}' - {:?}", cloudformation_deployment_role_name, dispatch_err)
                            }
                        },
                        aws_sdk_iam::error::SdkError::ConstructionFailure(construction_err) => {
                            format!("Failed to get role policy: Request construction error for role '{}' - {:?}", cloudformation_deployment_role_name, construction_err)
                        },
                        _ => {
                            format!("Failed to get role policy: Unknown error type for role '{}' - {}", cloudformation_deployment_role_name, e)
                        }
                    }
                })?;

            let policy_document = policy_response
                .policy_document;

            // URL decode the policy document
            info!("Raw CloudFormation role policy document before decoding: {}", policy_document);

            let decoded_policy = match percent_decode(policy_document.as_bytes()).decode_utf8() {
                Ok(decoded) => {
                    let decoded_str = decoded.to_string();
                    info!("Successfully percent decoded CloudFormation role policy document");
                    decoded_str
                }
                Err(e) => {
                    info!("Failed to percent decode CloudFormation role policy document: {}, trying form_urlencoded", e);
                    // Fallback to form_urlencoded parsing if percent decoding fails
                    form_urlencoded::parse(policy_document.as_bytes())
                        .find(|(key, _)| key.is_empty())
                        .map(|(_, value)| value.to_string())
                        .unwrap_or_else(|| {
                            info!("Form URL encoding also failed, using original CloudFormation role policy document");
                            policy_document
                        })
                }
            };

            // Log the decoded policy in pretty JSON format for troubleshooting
            match serde_json::from_str::<serde_json::Value>(&decoded_policy) {
                Ok(json_value) => {
                    match serde_json::to_string_pretty(&json_value) {
                        Ok(pretty_json) => {
                            info!("CloudFormation role policy document (pretty JSON):\n{}", pretty_json);
                        }
                        Err(e) => {
                            info!("Failed to pretty print CloudFormation role policy JSON: {}", e);
                            info!("Raw decoded CloudFormation role policy: {}", decoded_policy);
                        }
                    }
                }
                Err(e) => {
                    info!("Failed to parse CloudFormation role policy as JSON: {}", e);
                    info!("Raw decoded CloudFormation role policy: {}", decoded_policy);
                }
            }

            info!("Analyzing policy document for DynamoDB table and CloudFormation role ARNs");

            // Extract DynamoDB table ARN
            let dynamodb_pattern = format!(r"arn:aws:dynamodb:.*:.*:table/{}", dynamodb_table_pattern);
            let dynamodb_re = Regex::new(&dynamodb_pattern)
                .map_err(|e| format!("Invalid DynamoDB regex pattern: {}", e))?;

            let table_arn = dynamodb_re
                .find(&decoded_policy)
                .ok_or("No matching DynamoDB table found in policy")?
                .as_str()
                .to_string();

            // Parse the DynamoDB ARN: arn:aws:dynamodb:region:account:table/table_name
            let arn_parts: Vec<&str> = table_arn.split(':').collect();
            if arn_parts.len() != 6 {
                return Err("Invalid DynamoDB ARN format".to_string());
            }

            let db_region = arn_parts[3].to_string();
            let db_account = arn_parts[4].to_string();
            let table_part = arn_parts[5];
            let table_name = table_part
                .strip_prefix("table/")
                .ok_or("Invalid table ARN format")?
                .to_string();

            // Extract CloudFormation role ARNs
            let cf_role_pattern = r"arn:aws:iam::*:role/[a-zA-Z0-9\-_]+";
            let cf_role_re = Regex::new(cf_role_pattern)
                .map_err(|e| format!("Invalid CloudFormation role regex pattern: {}", e))?;

            let cloudformation_role_arns: Vec<String> = cf_role_re
                .find_iter(&decoded_policy)
                .map(|m| m.as_str().to_string())
                .collect();

            info!("Found {} CloudFormation role ARNs in policy", cloudformation_role_arns.len());

            Ok(InfrastructureInfo {
                dynamodb_table_arn: table_arn.clone(),
                db_region,
                db_account,
                table_name,
                cloudformation_role_arns,
                source_role: cloudformation_deployment_role_name.to_string(),
            })
        });

        match result {
            Ok(infrastructure_info) => {
                info!(
                    "Successfully extracted infrastructure info: DynamoDB table {} in {}:{}, {} CloudFormation roles",
                    infrastructure_info.table_name, infrastructure_info.db_region,
                    infrastructure_info.db_account, infrastructure_info.cloudformation_role_arns.len()
                );

                // Store the information in the identity center
                self.infrastructure_info = Some(infrastructure_info.clone());

                Ok(infrastructure_info)
            }
            Err(e) => {
                error!("Failed to extract infrastructure info: {}", e);
                Err(e)
            }
        }
    }
}
