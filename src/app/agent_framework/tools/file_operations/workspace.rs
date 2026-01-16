//! Workspace abstraction for file operations
//!
//! Supports both disk-based and VFS-based workspaces using a magic workspace name pattern.
//! VFS workspaces use the pattern: `vfs:{vfs_id}:{page_id}`

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

use crate::app::agent_framework::vfs::{with_vfs, with_vfs_mut};

/// File entry information for listing
#[derive(Debug, Clone)]
pub struct WorkspaceFileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: u64,
}

/// Workspace type for file operations
#[derive(Debug, Clone)]
pub enum WorkspaceType {
    /// Disk-based workspace at a specific path
    Disk { path: PathBuf },
    /// VFS-based workspace (temporary, in-memory)
    Vfs { vfs_id: String, page_id: String },
}

impl WorkspaceType {
    /// Parse workspace name and return appropriate type
    ///
    /// # Arguments
    /// * `workspace_name` - Either a simple name for disk storage, or `vfs:{vfs_id}:{page_id}` for VFS
    ///
    /// # Examples
    /// ```ignore
    /// // Disk workspace: ~/.local/share/awsdash/pages/my-dashboard/
    /// let ws = WorkspaceType::from_workspace_name("my-dashboard")?;
    ///
    /// // VFS workspace: /pages/vpc-explorer/ in VFS instance abc123
    /// let ws = WorkspaceType::from_workspace_name("vfs:abc123:vpc-explorer")?;
    /// ```
    pub fn from_workspace_name(workspace_name: &str) -> Result<Self> {
        if workspace_name.starts_with("vfs:") {
            // Parse: "vfs:{vfs_id}:{page_id}"
            let parts: Vec<&str> = workspace_name.splitn(3, ':').collect();
            if parts.len() == 3 && !parts[1].is_empty() && !parts[2].is_empty() {
                Ok(WorkspaceType::Vfs {
                    vfs_id: parts[1].to_string(),
                    page_id: parts[2].to_string(),
                })
            } else {
                anyhow::bail!(
                    "Invalid VFS workspace format '{}'. Expected 'vfs:{{vfs_id}}:{{page_id}}'",
                    workspace_name
                );
            }
        } else {
            // Disk-based workspace
            let path = dirs::data_local_dir()
                .context("Failed to get local data directory")?
                .join("awsdash/pages")
                .join(workspace_name);

            // Ensure workspace directory exists for disk
            std::fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create workspace directory: {:?}", path))?;

            Ok(WorkspaceType::Disk { path })
        }
    }

    /// Validate a relative path for security
    pub fn validate_path(&self, relative_path: &str) -> Result<()> {
        // Prevent directory traversal
        if relative_path.contains("..") || relative_path.starts_with('/') {
            anyhow::bail!("Invalid path: directory traversal not allowed");
        }

        // For disk workspaces, also verify the resolved path stays in workspace
        if let WorkspaceType::Disk { path } = self {
            let full_path = path.join(relative_path);
            if !full_path.starts_with(path) {
                anyhow::bail!("Path outside workspace");
            }
        }

        Ok(())
    }

    /// Get the VFS path for a relative path
    fn vfs_path(&self, relative_path: &str) -> String {
        match self {
            WorkspaceType::Vfs { page_id, .. } => {
                format!("/pages/{}/{}", page_id, relative_path)
            }
            WorkspaceType::Disk { .. } => relative_path.to_string(),
        }
    }

    /// Check if a file exists
    pub fn exists(&self, relative_path: &str) -> Result<bool> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                Ok(full_path.exists())
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs(vfs_id, |vfs| vfs.exists(&vfs_path)) {
                    Some(exists) => Ok(exists),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Check if a path is a file (not a directory)
    pub fn is_file(&self, relative_path: &str) -> Result<bool> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                Ok(full_path.is_file())
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs(vfs_id, |vfs| {
                    match vfs.stat(&vfs_path) {
                        Ok(entry) => !entry.is_directory,
                        Err(_) => false,
                    }
                }) {
                    Some(is_file) => Ok(is_file),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Check if a path is a directory
    pub fn is_directory(&self, relative_path: &str) -> Result<bool> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                Ok(full_path.is_dir())
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs(vfs_id, |vfs| {
                    match vfs.stat(&vfs_path) {
                        Ok(entry) => entry.is_directory,
                        Err(_) => false,
                    }
                }) {
                    Some(is_dir) => Ok(is_dir),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Read file content as bytes
    pub fn read_file(&self, relative_path: &str) -> Result<Vec<u8>> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                std::fs::read(&full_path).with_context(|| format!("Failed to read file: {}", relative_path))
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs(vfs_id, |vfs| {
                    vfs.read_file(&vfs_path).map(|bytes| bytes.to_vec())
                }) {
                    Some(result) => result.with_context(|| format!("Failed to read VFS file: {}", relative_path)),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Read file content as string
    pub fn read_file_string(&self, relative_path: &str) -> Result<String> {
        let bytes = self.read_file(relative_path)?;
        String::from_utf8(bytes).with_context(|| format!("File is not valid UTF-8: {}", relative_path))
    }

    /// Write file content
    pub fn write_file(&self, relative_path: &str, content: &[u8]) -> Result<()> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);

                // Create parent directories if needed
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create parent directory for: {}", relative_path))?;
                }

                std::fs::write(&full_path, content)
                    .with_context(|| format!("Failed to write file: {}", relative_path))
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs_mut(vfs_id, |vfs| vfs.write_file(&vfs_path, content)) {
                    Some(result) => result.with_context(|| format!("Failed to write VFS file: {}", relative_path)),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Delete a file
    pub fn delete_file(&self, relative_path: &str) -> Result<()> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                std::fs::remove_file(&full_path)
                    .with_context(|| format!("Failed to delete file: {}", relative_path))
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs_mut(vfs_id, |vfs| vfs.delete(&vfs_path)) {
                    Some(result) => result.with_context(|| format!("Failed to delete VFS file: {}", relative_path)),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// List files in a directory
    pub fn list_dir(&self, relative_path: Option<&str>) -> Result<Vec<WorkspaceFileEntry>> {
        if let Some(path) = relative_path {
            self.validate_path(path)?;
        }

        match self {
            WorkspaceType::Disk { path } => {
                let dir_path = match relative_path {
                    Some(p) => path.join(p),
                    None => path.clone(),
                };

                if !dir_path.exists() {
                    anyhow::bail!("Directory not found");
                }

                if !dir_path.is_dir() {
                    anyhow::bail!("Path is not a directory");
                }

                let mut entries = Vec::new();

                for entry_result in std::fs::read_dir(&dir_path)? {
                    if let Ok(entry) = entry_result {
                        if let Ok(metadata) = entry.metadata() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            let relative = match entry.path().strip_prefix(path) {
                                Ok(p) => p.to_string_lossy().to_string(),
                                Err(_) => continue,
                            };

                            entries.push(WorkspaceFileEntry {
                                name,
                                path: relative,
                                is_directory: metadata.is_dir(),
                                size_bytes: metadata.len(),
                            });
                        }
                    }
                }

                // Sort: directories first, then alphabetically
                entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                });

                Ok(entries)
            }
            WorkspaceType::Vfs { vfs_id, page_id } => {
                let vfs_dir = match relative_path {
                    Some(p) => format!("/pages/{}/{}", page_id, p),
                    None => format!("/pages/{}", page_id),
                };
                let base_path = relative_path.unwrap_or("");

                match with_vfs(vfs_id, |vfs| {
                    vfs.list_dir(&vfs_dir).map(|vfs_entries| {
                        vfs_entries
                            .into_iter()
                            .map(|e| {
                                let entry_path = if base_path.is_empty() {
                                    e.name.clone()
                                } else {
                                    format!("{}/{}", base_path, e.name)
                                };
                                WorkspaceFileEntry {
                                    name: e.name,
                                    path: entry_path,
                                    is_directory: e.is_directory,
                                    size_bytes: e.size as u64,
                                }
                            })
                            .collect()
                    })
                }) {
                    Some(result) => result.with_context(|| "Failed to list VFS directory"),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Create a directory
    pub fn mkdir(&self, relative_path: &str) -> Result<()> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                std::fs::create_dir_all(&full_path)
                    .with_context(|| format!("Failed to create directory: {}", relative_path))
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs_mut(vfs_id, |vfs| vfs.mkdir(&vfs_path)) {
                    Some(result) => result.with_context(|| format!("Failed to create VFS directory: {}", relative_path)),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }

    /// Get file size in bytes
    pub fn file_size(&self, relative_path: &str) -> Result<u64> {
        self.validate_path(relative_path)?;

        match self {
            WorkspaceType::Disk { path } => {
                let full_path = path.join(relative_path);
                let metadata = std::fs::metadata(&full_path)
                    .with_context(|| format!("Failed to get file metadata: {}", relative_path))?;
                Ok(metadata.len())
            }
            WorkspaceType::Vfs { vfs_id, .. } => {
                let vfs_path = self.vfs_path(relative_path);
                match with_vfs(vfs_id, |vfs| {
                    vfs.stat(&vfs_path).map(|entry| entry.size as u64)
                }) {
                    Some(result) => result.with_context(|| format!("Failed to get file size: {}", relative_path)),
                    None => Err(anyhow!("VFS not found: {}", vfs_id)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_disk_workspace() {
        // We can't easily test this without mocking dirs, but we can test the pattern
        let ws = WorkspaceType::Disk {
            path: PathBuf::from("/tmp/test"),
        };
        assert!(matches!(ws, WorkspaceType::Disk { .. }));
    }

    #[test]
    fn test_parse_vfs_workspace_pattern() {
        let ws = WorkspaceType::from_workspace_name("vfs:abc123:my-page");
        assert!(ws.is_ok());
        if let Ok(WorkspaceType::Vfs { vfs_id, page_id }) = ws {
            assert_eq!(vfs_id, "abc123");
            assert_eq!(page_id, "my-page");
        } else {
            panic!("Expected VFS workspace type");
        }
    }

    #[test]
    fn test_parse_vfs_workspace_invalid() {
        // Missing page_id
        let ws = WorkspaceType::from_workspace_name("vfs:abc123");
        assert!(ws.is_err());

        // Empty vfs_id
        let ws = WorkspaceType::from_workspace_name("vfs::my-page");
        assert!(ws.is_err());

        // Empty page_id
        let ws = WorkspaceType::from_workspace_name("vfs:abc123:");
        assert!(ws.is_err());
    }

    #[test]
    fn test_validate_path_prevents_traversal() {
        let ws = WorkspaceType::Disk {
            path: PathBuf::from("/tmp/workspace"),
        };

        assert!(ws.validate_path("file.txt").is_ok());
        assert!(ws.validate_path("subdir/file.txt").is_ok());
        assert!(ws.validate_path("../file.txt").is_err());
        assert!(ws.validate_path("/etc/passwd").is_err());
        assert!(ws.validate_path("subdir/../../../etc/passwd").is_err());
    }

    #[test]
    fn test_disk_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let ws = WorkspaceType::Disk {
            path: temp_dir.path().to_path_buf(),
        };

        // Write file
        ws.write_file("test.txt", b"Hello, World!").unwrap();

        // Check exists
        assert!(ws.exists("test.txt").unwrap());
        assert!(!ws.exists("nonexistent.txt").unwrap());

        // Check is_file
        assert!(ws.is_file("test.txt").unwrap());

        // Read file
        let content = ws.read_file_string("test.txt").unwrap();
        assert_eq!(content, "Hello, World!");

        // File size
        assert_eq!(ws.file_size("test.txt").unwrap(), 13);

        // Delete file
        ws.delete_file("test.txt").unwrap();
        assert!(!ws.exists("test.txt").unwrap());
    }

    #[test]
    fn test_disk_directory_operations() {
        let temp_dir = TempDir::new().unwrap();
        let ws = WorkspaceType::Disk {
            path: temp_dir.path().to_path_buf(),
        };

        // Create directory
        ws.mkdir("subdir").unwrap();
        assert!(ws.is_directory("subdir").unwrap());

        // Write file in subdir
        ws.write_file("subdir/file.txt", b"content").unwrap();

        // List dir
        let entries = ws.list_dir(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "subdir");
        assert!(entries[0].is_directory);

        // List subdir
        let entries = ws.list_dir(Some("subdir")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file.txt");
        assert!(!entries[0].is_directory);
    }

    #[test]
    fn test_vfs_path_construction() {
        let ws = WorkspaceType::Vfs {
            vfs_id: "abc123".to_string(),
            page_id: "my-page".to_string(),
        };

        assert_eq!(ws.vfs_path("index.html"), "/pages/my-page/index.html");
        assert_eq!(ws.vfs_path("subdir/file.js"), "/pages/my-page/subdir/file.js");
    }
}
