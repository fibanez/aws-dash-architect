//! SSO Configuration loader for quick login during development.
//!
//! This module loads default SSO configuration from a `sso.json` file in the
//! project root. This is a DEBUG-only feature to speed up testing by pre-filling
//! the login form.
//!
//! # sso.json Format
//!
//! ```json
//! {
//!   "identity_center_url": "https://d-xxxxxxxxxx.awsapps.com/start",
//!   "default_role_name": "awsdash",
//!   "region": "us-east-1"
//! }
//! ```
//!
//! The file should be added to .gitignore to avoid committing credentials.

#![cfg(debug_assertions)]

use serde::Deserialize;
use std::path::Path;
use tracing::{debug, warn};

/// SSO configuration loaded from sso.json.
#[derive(Debug, Clone, Deserialize)]
pub struct SsoConfig {
    /// Full Identity Center URL or short name (e.g., "d-xxxxxxxxxx" or full URL)
    pub identity_center_url: String,

    /// Default role name for login
    pub default_role_name: String,

    /// AWS region for Identity Center
    pub region: String,
}

impl SsoConfig {
    /// Load SSO configuration from sso.json in the current directory.
    ///
    /// Returns None if the file doesn't exist or is invalid.
    pub fn load() -> Option<Self> {
        Self::load_from_path("sso.json")
    }

    /// Load SSO configuration from a specific path.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = path.as_ref();

        if !path.exists() {
            debug!("No sso.json found at {:?}", path);
            return None;
        }

        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<SsoConfig>(&contents) {
                Ok(config) => {
                    debug!(
                        "Loaded SSO config: url={}, role={}, region={}",
                        config.identity_center_url, config.default_role_name, config.region
                    );
                    Some(config)
                }
                Err(e) => {
                    warn!("Failed to parse sso.json: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read sso.json: {}", e);
                None
            }
        }
    }

    /// Extract the short name from the identity_center_url.
    ///
    /// Handles both full URLs and short names:
    /// - "https://d-xxxxxxxxxx.awsapps.com/start" -> "d-xxxxxxxxxx"
    /// - "d-xxxxxxxxxx" -> "d-xxxxxxxxxx"
    pub fn short_name(&self) -> String {
        let url = &self.identity_center_url;

        // If it's a full URL, extract the short name
        if url.starts_with("https://") || url.starts_with("http://") {
            if let Some(start) = url.find("://") {
                let after_protocol = &url[start + 3..];
                if let Some(dot_pos) = after_protocol.find('.') {
                    return after_protocol[..dot_pos].to_string();
                }
            }
        } else if url.contains(".awsapps.com") {
            if let Some(dot_pos) = url.find('.') {
                return url[..dot_pos].to_string();
            }
        }

        // Return as-is if it looks like a short name already
        url.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_name_extraction() {
        let config = SsoConfig {
            identity_center_url: "https://d-1234567890.awsapps.com/start".to_string(),
            default_role_name: "awsdash".to_string(),
            region: "us-east-1".to_string(),
        };
        assert_eq!(config.short_name(), "d-1234567890");

        let config2 = SsoConfig {
            identity_center_url: "d-abcdefghij".to_string(),
            default_role_name: "awsdash".to_string(),
            region: "us-east-1".to_string(),
        };
        assert_eq!(config2.short_name(), "d-abcdefghij");
    }
}
