//! File Security Validation
//!
//! Provides security validation for file system operations to prevent
//! path traversal attacks and unauthorized file access.

use std::fmt;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Security errors for file operations
#[derive(Debug, Clone)]
pub enum SecurityError {
    RelativePath(String),
    InvalidPath(String),
    OutsideAllowedDirectory(String),
    SensitiveFile(String),
    FileTooLarge { current: u64, max: u64 },
    InvalidComponents(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::RelativePath(path) => {
                write!(f, "Path must be absolute, not relative: {}", path)
            }
            SecurityError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            SecurityError::OutsideAllowedDirectory(path) => {
                write!(f, "Path is outside allowed directories: {}", path)
            }
            SecurityError::SensitiveFile(path) => {
                write!(f, "Access to sensitive file denied: {}", path)
            }
            SecurityError::FileTooLarge { current, max } => write!(
                f,
                "File size exceeds maximum allowed: {} bytes (max: {} bytes)",
                current, max
            ),
            SecurityError::InvalidComponents(msg) => {
                write!(f, "Path contains invalid components: {}", msg)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// Maximum file size allowed for reading (10MB)
pub const MAX_FILE_SIZE: u64 = 10_000_000;

/// Get allowed skill directories for this system
pub fn get_allowed_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // Anthropic Claude skills directory
        dirs.push(home.join(".claude/skills"));

        // AWS Dash skills directory
        dirs.push(home.join(".awsdash/skills"));

        debug!("Allowed skill directories: {:?}", dirs);
    } else {
        warn!("Could not determine home directory for skill locations");
    }

    dirs
}

/// Check if a path is within any of the allowed directories
fn is_within_allowed_dirs(path: &Path, allowed_dirs: &[PathBuf]) -> bool {
    for allowed_dir in allowed_dirs {
        if path.starts_with(allowed_dir) {
            debug!(
                "Path {:?} is within allowed directory {:?}",
                path, allowed_dir
            );
            return true;
        }
    }

    debug!("Path {:?} is not within any allowed directory", path);
    false
}

/// Check if a path points to a sensitive file that should never be accessed
fn is_sensitive_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    // Sensitive file patterns
    let sensitive_patterns = [
        "/.aws/credentials",
        "/.aws/config",
        "/.ssh/",
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/.env",
        "/credentials",
        "/secrets",
        "/.kube/config",
        "/id_rsa",
        "/id_ed25519",
        "/authorized_keys",
        "/known_hosts",
    ];

    for pattern in &sensitive_patterns {
        if path_str.contains(pattern) {
            warn!("Attempted access to sensitive file: {:?}", path);
            return true;
        }
    }

    false
}

/// Validate that a file path is safe to access
///
/// Performs comprehensive security checks:
/// 1. Must be an absolute path (not relative)
/// 2. Canonicalize to resolve symlinks and .. components
/// 3. Must be within allowed directories
/// 4. Must not be a sensitive system file
///
/// # Arguments
/// * `path` - The path to validate
///
/// # Returns
/// * `Ok(PathBuf)` - Canonical absolute path if safe
/// * `Err(SecurityError)` - Specific security violation
///
/// # Examples
/// ```
/// use agent_framework::tools::file_security::validate_file_path;
///
/// // Valid path within allowed directory
/// let path = validate_file_path("/home/user/.claude/skills/aws-ec2/SKILL.md")?;
///
/// // Invalid: relative path
/// let result = validate_file_path("../secrets/data.txt");
/// assert!(result.is_err());
/// ```
pub fn validate_file_path(path: &str) -> Result<PathBuf, SecurityError> {
    let path_buf = PathBuf::from(path);

    // 1. Must be absolute
    if !path_buf.is_absolute() {
        return Err(SecurityError::RelativePath(path.to_string()));
    }

    // 2. Canonicalize (resolves symlinks, .., ., etc.)
    let canonical = match path_buf.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            // If path doesn't exist yet, try parent directory
            // This allows validation before file creation
            if let Some(parent) = path_buf.parent() {
                if parent.exists() {
                    // Parent exists, path itself doesn't - that's OK for some operations
                    // But we still need to validate the parent
                    let canonical_parent = parent
                        .canonicalize()
                        .map_err(|_| SecurityError::InvalidPath(path.to_string()))?;

                    // Reconstruct the path with canonical parent
                    if let Some(filename) = path_buf.file_name() {
                        canonical_parent.join(filename)
                    } else {
                        return Err(SecurityError::InvalidPath(path.to_string()));
                    }
                } else {
                    return Err(SecurityError::InvalidPath(format!(
                        "{} (parent directory doesn't exist: {})",
                        path, e
                    )));
                }
            } else {
                return Err(SecurityError::InvalidPath(format!("{} ({})", path, e)));
            }
        }
    };

    // 3. Check for path traversal components (should be resolved by canonicalize, but double-check)
    let components = canonical.components().collect::<Vec<_>>();
    for component in &components {
        let component_str = format!("{:?}", component);
        if component_str.contains("..") {
            return Err(SecurityError::InvalidComponents(format!(
                "Path contains traversal: {}",
                path
            )));
        }
    }

    // 4. Check allowed directories
    let allowed_dirs = get_allowed_directories();
    if !is_within_allowed_dirs(&canonical, &allowed_dirs) {
        return Err(SecurityError::OutsideAllowedDirectory(format!(
            "{} (allowed: {:?})",
            canonical.display(),
            allowed_dirs
        )));
    }

    // 5. Check sensitive file blocklist
    if is_sensitive_path(&canonical) {
        return Err(SecurityError::SensitiveFile(
            canonical.display().to_string(),
        ));
    }

    debug!("Path validation successful: {:?}", canonical);
    Ok(canonical)
}

/// Validate directory path with same security checks as files
pub fn validate_directory_path(path: &str) -> Result<PathBuf, SecurityError> {
    validate_file_path(path)
}

/// Check if a file size is within allowed limits
pub fn validate_file_size(size: u64) -> Result<(), SecurityError> {
    if size > MAX_FILE_SIZE {
        return Err(SecurityError::FileTooLarge {
            current: size,
            max: MAX_FILE_SIZE,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_reject_relative_paths() {
        let result = validate_file_path("../secrets/data.txt");
        assert!(matches!(result, Err(SecurityError::RelativePath(_))));

        let result = validate_file_path("./local/file.md");
        assert!(matches!(result, Err(SecurityError::RelativePath(_))));
    }

    #[test]
    fn test_reject_sensitive_files() {
        // These should fail even if absolute and in allowed dirs
        // (they won't be in allowed dirs, but test the sensitive path check)
        let home = dirs::home_dir().unwrap();

        let sensitive_paths = vec![
            format!("{}/.aws/credentials", home.display()),
            format!("{}/.ssh/id_rsa", home.display()),
            "/etc/passwd".to_string(),
        ];

        for path in sensitive_paths {
            if PathBuf::from(&path).exists() {
                let result = validate_file_path(&path);
                assert!(result.is_err(), "Should reject sensitive path: {}", path);
            }
        }
    }

    #[test]
    fn test_file_size_limits() {
        assert!(validate_file_size(1024).is_ok());
        assert!(validate_file_size(MAX_FILE_SIZE).is_ok());
        assert!(validate_file_size(MAX_FILE_SIZE + 1).is_err());
    }

    #[test]
    fn test_allowed_directories() {
        let dirs = get_allowed_directories();
        assert!(!dirs.is_empty());

        // Should include both Claude and AWS Dash directories
        let has_claude = dirs.iter().any(|d| d.to_string_lossy().contains(".claude"));
        let has_awsdash = dirs
            .iter()
            .any(|d| d.to_string_lossy().contains(".awsdash"));

        assert!(has_claude, "Should include .claude/skills directory");
        assert!(has_awsdash, "Should include .awsdash/skills directory");
    }

    #[test]
    fn test_validate_within_allowed_dir() {
        // Create a test skill directory
        let home = dirs::home_dir().unwrap();
        let test_dir = home.join(".awsdash/skills/test-skill");

        // Create directory if it doesn't exist
        if !test_dir.exists() {
            fs::create_dir_all(&test_dir).ok();
        }

        // Test file path within allowed directory
        let test_file = test_dir.join("SKILL.md");
        let test_path = test_file.to_string_lossy().to_string();

        // If directory exists, validation should succeed (even if file doesn't exist)
        if test_dir.exists() {
            let result = validate_file_path(&test_path);
            // Should either succeed or fail with file not existing, but not security error
            if let Err(e) = result {
                // Should not be OutsideAllowedDirectory or SensitiveFile
                assert!(
                    !matches!(e, SecurityError::OutsideAllowedDirectory(_)),
                    "Should not reject path in allowed directory"
                );
            }
        }

        // Clean up
        fs::remove_dir_all(&test_dir).ok();
    }
}
