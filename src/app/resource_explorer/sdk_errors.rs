//! AWS SDK error categorization for retry tracking and user visibility.
//!
//! This module provides structured error categorization for AWS SDK errors,
//! distinguishing between retryable transient errors (throttling, timeouts,
//! network issues) and non-retryable errors (permissions, validation).
//!
//! The AWS SDK handles retries internally with exponential backoff. This module
//! provides visibility into those errors for user feedback without implementing
//! additional application-level retry logic.

use std::time::Duration;

/// Categorized error types for AWS SDK errors
#[derive(Debug, Clone)]
pub enum ErrorCategory {
    /// Request was throttled due to rate limiting
    Throttled {
        service: String,
        error_code: String,
        /// Hint from Retry-After header if available
        retry_after: Option<Duration>,
    },
    /// Request timed out
    Timeout {
        operation: String,
        duration: Option<Duration>,
    },
    /// Network connectivity issues
    NetworkError { message: String },
    /// AWS service temporarily unavailable
    ServiceUnavailable {
        service: String,
        message: String,
    },
    /// Non-retryable error (permissions, validation, etc.)
    NonRetryable {
        code: String,
        message: String,
        is_permission_error: bool,
    },
}

impl ErrorCategory {
    /// Returns true if this error category is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCategory::Throttled { .. }
                | ErrorCategory::Timeout { .. }
                | ErrorCategory::NetworkError { .. }
                | ErrorCategory::ServiceUnavailable { .. }
        )
    }

    /// User-friendly message for status bar display
    pub fn user_message(&self) -> String {
        match self {
            ErrorCategory::Throttled { service, .. } => {
                format!("{} rate limited", service)
            }
            ErrorCategory::Timeout { operation, .. } => {
                format!("{} timeout", operation)
            }
            ErrorCategory::NetworkError { .. } => "Network error".to_string(),
            ErrorCategory::ServiceUnavailable { service, .. } => {
                format!("{} unavailable", service)
            }
            ErrorCategory::NonRetryable { code, .. } => code.clone(),
        }
    }

    /// Short label for compact display
    pub fn short_label(&self) -> &'static str {
        match self {
            ErrorCategory::Throttled { .. } => "throttled",
            ErrorCategory::Timeout { .. } => "timeout",
            ErrorCategory::NetworkError { .. } => "network",
            ErrorCategory::ServiceUnavailable { .. } => "unavailable",
            ErrorCategory::NonRetryable { .. } => "error",
        }
    }
}

/// Analyze an error string and categorize it
///
/// This function examines error messages from anyhow::Error (which wraps SDK errors)
/// and categorizes them based on known AWS error patterns.
pub fn categorize_error(error: &anyhow::Error, service: &str, operation: &str) -> ErrorCategory {
    let error_str = error.to_string();
    let error_debug = format!("{:?}", error);

    // Use the more detailed format if available
    let detail = if error_str.contains("service error") {
        &error_debug
    } else {
        &error_str
    };

    categorize_error_string(detail, service, operation)
}

/// Categorize an error based on its string representation
///
/// This handles the common patterns seen in AWS SDK error messages.
pub fn categorize_error_string(error_str: &str, service: &str, operation: &str) -> ErrorCategory {
    // Throttling errors (most common transient error)
    if error_str.contains("ThrottlingException")
        || error_str.contains("Throttling")
        || error_str.contains("TooManyRequestsException")
        || error_str.contains("RequestLimitExceeded")
        || error_str.contains("ProvisionedThroughputExceededException")
        || error_str.contains("LimitExceededException")
        || error_str.contains("RateExceeded")
    {
        let error_code = extract_error_code(error_str).unwrap_or("Throttling".to_string());
        return ErrorCategory::Throttled {
            service: service.to_string(),
            error_code,
            retry_after: None, // SDK doesn't expose Retry-After easily
        };
    }

    // Timeout errors
    if error_str.contains("TimeoutError")
        || error_str.contains("timeout")
        || error_str.contains("timed out")
        || error_str.contains("deadline exceeded")
    {
        return ErrorCategory::Timeout {
            operation: operation.to_string(),
            duration: None,
        };
    }

    // Network/dispatch errors
    if error_str.contains("DispatchFailure")
        || error_str.contains("connection")
        || error_str.contains("Connection")
        || error_str.contains("network")
        || error_str.contains("Network")
        || error_str.contains("DNS")
        || error_str.contains("socket")
    {
        return ErrorCategory::NetworkError {
            message: truncate_message(error_str, 100),
        };
    }

    // Service unavailable (AWS-side transient errors)
    if error_str.contains("ServiceUnavailable")
        || error_str.contains("InternalServerError")
        || error_str.contains("InternalServerException")
        || error_str.contains("InternalError")
        || error_str.contains("Service Unavailable")
        || error_str.contains("503")
        || error_str.contains("500")
    {
        return ErrorCategory::ServiceUnavailable {
            service: service.to_string(),
            message: truncate_message(error_str, 100),
        };
    }

    // Permission errors (non-retryable)
    let is_permission_error = error_str.contains("AccessDenied")
        || error_str.contains("AccessDeniedException")
        || error_str.contains("UnauthorizedOperation")
        || error_str.contains("UnauthorizedAccess")
        || error_str.contains("AuthFailure")
        || error_str.contains("InvalidClientTokenId")
        || error_str.contains("SignatureDoesNotMatch");

    // Extract error code if possible
    let code = extract_error_code(error_str).unwrap_or_else(|| {
        if is_permission_error {
            "AccessDenied".to_string()
        } else {
            "Error".to_string()
        }
    });

    ErrorCategory::NonRetryable {
        code,
        message: truncate_message(error_str, 200),
        is_permission_error,
    }
}

/// Extract AWS error code from error message if present
fn extract_error_code(error_str: &str) -> Option<String> {
    // Common patterns:
    // "ThrottlingException: Rate exceeded"
    // "service error: AccessDeniedException"
    // Error { code: "ValidationException", ...}

    // Pattern 1: ErrorName: message
    if let Some(pos) = error_str.find(':') {
        let prefix = error_str[..pos].trim();
        // Check if it looks like an error code (CamelCase or ends with Exception/Error)
        if prefix.ends_with("Exception")
            || prefix.ends_with("Error")
            || prefix.chars().next().is_some_and(|c| c.is_uppercase())
        {
            // Get just the error name, not full path
            let code = prefix.rsplit("::").next().unwrap_or(prefix);
            if !code.is_empty() && code.len() < 50 {
                return Some(code.to_string());
            }
        }
    }

    // Pattern 2: code: "ErrorName" in debug output
    if let Some(start) = error_str.find("code:") {
        let after_code = &error_str[start + 5..];
        // Find quoted string
        if let Some(quote_start) = after_code.find('"') {
            let after_quote = &after_code[quote_start + 1..];
            if let Some(quote_end) = after_quote.find('"') {
                let code = &after_quote[..quote_end];
                if !code.is_empty() && code.len() < 50 {
                    return Some(code.to_string());
                }
            }
        }
    }

    None
}

/// Truncate a message to max length, adding ellipsis if truncated
fn truncate_message(msg: &str, max_len: usize) -> String {
    if msg.len() <= max_len {
        msg.to_string()
    } else {
        format!("{}...", &msg[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_throttling() {
        let error = "ThrottlingException: Rate exceeded";
        let cat = categorize_error_string(error, "S3", "ListBuckets");
        assert!(matches!(cat, ErrorCategory::Throttled { .. }));
        assert!(cat.is_retryable());
    }

    #[test]
    fn test_categorize_too_many_requests() {
        let error = "TooManyRequestsException: Request rate too high";
        let cat = categorize_error_string(error, "Lambda", "ListFunctions");
        assert!(matches!(cat, ErrorCategory::Throttled { .. }));
    }

    #[test]
    fn test_categorize_timeout() {
        let error = "TimeoutError: request timed out after 30s";
        let cat = categorize_error_string(error, "EC2", "DescribeInstances");
        assert!(matches!(cat, ErrorCategory::Timeout { .. }));
        assert!(cat.is_retryable());
    }

    #[test]
    fn test_categorize_network_error() {
        let error = "DispatchFailure: connection refused";
        let cat = categorize_error_string(error, "IAM", "ListRoles");
        assert!(matches!(cat, ErrorCategory::NetworkError { .. }));
        assert!(cat.is_retryable());
    }

    #[test]
    fn test_categorize_service_unavailable() {
        let error = "ServiceUnavailable: The service is currently unavailable";
        let cat = categorize_error_string(error, "DynamoDB", "Scan");
        assert!(matches!(cat, ErrorCategory::ServiceUnavailable { .. }));
        assert!(cat.is_retryable());
    }

    #[test]
    fn test_categorize_access_denied() {
        let error = "AccessDeniedException: User is not authorized";
        let cat = categorize_error_string(error, "S3", "ListBuckets");
        assert!(matches!(
            cat,
            ErrorCategory::NonRetryable {
                is_permission_error: true,
                ..
            }
        ));
        assert!(!cat.is_retryable());
    }

    #[test]
    fn test_categorize_generic_error() {
        let error = "ValidationException: Invalid parameter";
        let cat = categorize_error_string(error, "Lambda", "CreateFunction");
        assert!(matches!(
            cat,
            ErrorCategory::NonRetryable {
                is_permission_error: false,
                ..
            }
        ));
        assert!(!cat.is_retryable());
    }

    #[test]
    fn test_extract_error_code() {
        assert_eq!(
            extract_error_code("ThrottlingException: Rate exceeded"),
            Some("ThrottlingException".to_string())
        );
        assert_eq!(
            extract_error_code("code: \"ValidationException\""),
            Some("ValidationException".to_string())
        );
    }

    #[test]
    fn test_user_message() {
        let throttled = ErrorCategory::Throttled {
            service: "Lambda".to_string(),
            error_code: "ThrottlingException".to_string(),
            retry_after: None,
        };
        assert_eq!(throttled.user_message(), "Lambda rate limited");

        let timeout = ErrorCategory::Timeout {
            operation: "ListFunctions".to_string(),
            duration: None,
        };
        assert_eq!(timeout.user_message(), "ListFunctions timeout");
    }
}
