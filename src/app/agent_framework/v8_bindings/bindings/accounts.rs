//! Account-related function bindings
//!
//! Provides JavaScript access to AWS account information without
//! exposing credentials, SSO details, or AWS SDK complexity.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use crate::app::aws_identity::AwsIdentityCenter;
use tracing::warn;

/// Global access to AwsIdentityCenter for account lookups
/// This is set by the application when Identity Center is initialized
static GLOBAL_AWS_IDENTITY: RwLock<Option<Arc<Mutex<AwsIdentityCenter>>>> = RwLock::new(None);

/// Set the global AwsIdentityCenter for account lookups (used by V8 bindings)
pub fn set_global_aws_identity(identity: Option<Arc<Mutex<AwsIdentityCenter>>>) {
    match GLOBAL_AWS_IDENTITY.write() {
        Ok(mut guard) => {
            *guard = identity;
        }
        Err(e) => {
            warn!("Failed to update global AwsIdentityCenter for V8 bindings: {}", e);
        }
    }
}

/// Get the global AwsIdentityCenter for account lookups
pub fn get_global_aws_identity() -> Option<Arc<Mutex<AwsIdentityCenter>>> {
    match GLOBAL_AWS_IDENTITY.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            warn!("Failed to read global AwsIdentityCenter for V8 bindings: {}", e);
            None
        }
    }
}

/// Account information exposed to JavaScript
///
/// This structure abstracts away AWS-specific details like SSO URLs,
/// role ARNs, and credential chains. The LLM only sees essential
/// identifying information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// AWS Account ID (12-digit number)
    pub id: String,

    /// Human-readable account name
    pub name: String,

    /// Short alias for the account (e.g., "prod", "dev")
    pub alias: Option<String>,

    /// Account email (if available)
    pub email: Option<String>,
}

/// Register account-related functions into V8 context
pub fn register(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register listAccounts() function
    let list_accounts_fn = v8::Function::new(scope, list_accounts_callback)
        .expect("Failed to create listAccounts function");

    let fn_name = v8::String::new(scope, "listAccounts")
        .expect("Failed to create function name string");
    global.set(scope, fn_name.into(), list_accounts_fn.into());

    Ok(())
}

/// Callback for listAccounts() JavaScript function
fn list_accounts_callback(
    scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Get account data from application state
    let accounts = match get_accounts_from_app() {
        Ok(accounts) => accounts,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to get accounts: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Serialize to JSON string
    let json_str = match serde_json::to_string(&accounts) {
        Ok(json) => json,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to serialize accounts: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Create V8 string from JSON
    let v8_str = match v8::String::new(scope, &json_str) {
        Some(s) => s,
        None => {
            let msg = v8::String::new(scope, "Failed to create V8 string").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON in V8 to create JavaScript array
    let v8_value = match v8::json::parse(scope, v8_str) {
        Some(v) => v,
        None => {
            let msg = v8::String::new(scope, "Failed to parse JSON in V8").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(v8_value);
}

/// Get account information from application state
///
/// Accesses the cached AwsIdentityCenter data without making API calls.
/// Returns the list of accounts that the user has access to through Identity Center.
fn get_accounts_from_app() -> Result<Vec<AccountInfo>> {
    // Get the global AwsIdentityCenter (cached data - no API calls)
    let identity = get_global_aws_identity()
        .ok_or_else(|| anyhow::anyhow!("AwsIdentityCenter not initialized"))?;

    let identity_guard = identity.lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock AwsIdentityCenter: {}", e))?;

    // Convert AwsAccount to AccountInfo (only expose essential fields to JavaScript)
    let accounts: Vec<AccountInfo> = identity_guard
        .accounts
        .iter()
        .map(|aws_account| AccountInfo {
            id: aws_account.account_id.clone(),
            name: aws_account.account_name.clone(),
            alias: None,  // AwsAccount doesn't have alias field
            email: aws_account.account_email.clone(),
        })
        .collect();

    Ok(accounts)
}

/// Get LLM documentation for account functions
pub fn get_documentation() -> String {
    r#"
### listAccounts()

List all configured AWS accounts available in the system.

**Signature:**
```typescript
function listAccounts(): AccountInfo[]
```

**Description:**
Returns an array of AWS account objects. No credentials or configuration needed -
authentication and account discovery are handled internally by the system.

**Return value structure:**
```json
[
  {
    "id": "123456789012",
    "name": "Production Account",
    "alias": "prod",
    "email": "aws-prod@example.com"
  },
  {
    "id": "987654321098",
    "name": "Development Account",
    "alias": "dev",
    "email": "aws-dev@example.com"
  }
]
```

**Field descriptions:**
- `id` (string): AWS Account ID (12-digit number as string)
- `name` (string): Human-readable account name
- `alias` (string | null): Short alias for the account (e.g., "prod", "dev")
- `email` (string | null): Account email address if available

**Example usage:**
```javascript
// Get all accounts
const accounts = listAccounts();
console.log(`Found ${accounts.length} accounts`);

// Find specific account by alias
const prodAccount = accounts.find(a => a.alias === 'prod');
if (prodAccount) {
  console.log(`Production account ID: ${prodAccount.id}`);
}

// Filter and map
const accountNames = accounts
  .filter(a => a.alias !== null)
  .map(a => `${a.alias}: ${a.name}`)
  .join('\n');

// Get all account IDs
const accountIds = accounts.map(a => a.id);

// Check if specific account exists
const hasDevAccount = accounts.some(a => a.alias === 'dev');
```

**Edge cases:**
- Returns empty array `[]` if no accounts are configured
- `alias` field may be `null` for accounts without aliases
- `email` field may be `null` if not available
- Never returns `null` or `undefined` - always returns an array

**Error handling:**
```javascript
const accounts = listAccounts();

if (accounts.length === 0) {
  console.error("No AWS accounts configured");
  return null;
}

// Safe access with null checks
const prodAccount = accounts.find(a => a.alias === 'prod');
if (!prodAccount) {
  console.error("Production account not found");
  return null;
}
```
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
    use crate::app::aws_identity::{AwsAccount, LoginState};
    use std::pin::pin;

    /// Create a test AwsIdentityCenter and set it globally for tests
    fn setup_test_identity() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://test.awsapps.com/start".to_string(),
            "test-role".to_string(),
            "us-east-1".to_string(),
        );

        // Add test accounts
        identity_center.accounts = vec![
            AwsAccount {
                account_id: "123456789012".to_string(),
                account_name: "Test Production Account".to_string(),
                account_email: Some("prod@test.com".to_string()),
                role_name: "test-role".to_string(),
                credentials: None,
            },
            AwsAccount {
                account_id: "987654321098".to_string(),
                account_name: "Test Development Account".to_string(),
                account_email: Some("dev@test.com".to_string()),
                role_name: "test-role".to_string(),
                credentials: None,
            },
        ];
        identity_center.login_state = LoginState::LoggedIn;

        // Set globally for tests
        set_global_aws_identity(Some(Arc::new(Mutex::new(identity_center))));
    }

    #[test]
    fn test_get_accounts_from_app() {
        setup_test_identity();
        let accounts = get_accounts_from_app().unwrap();

        assert!(!accounts.is_empty());
        assert!(accounts.len() >= 2);

        // Verify structure
        let account = &accounts[0];
        assert!(!account.id.is_empty());
        assert!(!account.name.is_empty());
    }

    #[test]
    fn test_list_accounts_binding() {
        setup_test_identity();
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Register the binding
        register(scope).unwrap();

        // Execute JavaScript that calls listAccounts()
        let code = v8::String::new(scope, "listAccounts()").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();

        // Should return an array
        assert!(result.is_array());

        // Convert to JavaScript array
        let array = v8::Local::<v8::Array>::try_from(result).unwrap();
        assert!(array.length() > 0);
    }

    #[test]
    fn test_list_accounts_javascript_access() {
        setup_test_identity();
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        register(scope).unwrap();

        // Test JavaScript can access account properties
        let code = r#"
            const accounts = listAccounts();
            const firstAccount = accounts[0];
            JSON.stringify({
                count: accounts.length,
                firstId: firstAccount.id,
                firstName: firstAccount.name,
                hasAlias: firstAccount.alias !== null
            })
        "#;

        let code_str = v8::String::new(scope, code).unwrap();
        let script = v8::Script::compile(scope, code_str, None).unwrap();
        let result = script.run(scope).unwrap();

        let result_str = result.to_string(scope).unwrap();
        let result_json = result_str.to_rust_string_lossy(scope);

        // Verify JavaScript could access properties
        assert!(result_json.contains("count"));
        assert!(result_json.contains("firstId"));
        assert!(result_json.contains("firstName"));
    }

    #[test]
    fn test_list_accounts_filtering() {
        setup_test_identity();
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        register(scope).unwrap();

        // Test JavaScript can filter and map accounts
        let code = r#"
            const accounts = listAccounts();
            const prodAccounts = accounts.filter(a => a.alias === 'prod');
            const accountIds = accounts.map(a => a.id);
            JSON.stringify({
                totalAccounts: accounts.length,
                prodCount: prodAccounts.length,
                firstId: accountIds[0]
            })
        "#;

        let code_str = v8::String::new(scope, code).unwrap();
        let script = v8::Script::compile(scope, code_str, None).unwrap();
        let result = script.run(scope).unwrap();

        let result_str = result.to_string(scope).unwrap();
        let result_json = result_str.to_rust_string_lossy(scope);

        // Verify operations worked
        assert!(result_json.contains("totalAccounts"));
        assert!(result_json.contains("prodCount"));
        assert!(result_json.contains("firstId"));
    }

    #[test]
    fn test_documentation_format() {
        let docs = get_documentation();

        // Verify required documentation elements
        assert!(docs.contains("listAccounts()"));
        assert!(docs.contains("function listAccounts()"));
        assert!(docs.contains("Return value structure:"));
        assert!(docs.contains("```json"));
        assert!(docs.contains("Field descriptions:"));
        assert!(docs.contains("Example usage:"));
        assert!(docs.contains("Edge cases:"));
        assert!(docs.contains("Error handling:"));
    }
}
