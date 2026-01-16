//! Virtual File System for Agent Memory
//!
//! Provides an in-memory file system for storing agent data, query results,
//! and generated pages. This reduces LLM context pollution by storing large
//! results in VFS and only returning summaries to the LLM.
//!
//! ## Architecture
//!
//! - TaskManager owns a VFS instance and registers it in the global registry
//! - Workers receive the VFS ID as a string and access VFS via the registry
//! - V8 bindings use thread-local VFS ID to access the correct VFS
//! - File operation tools detect `vfs:` prefix and redirect to VFS
//!
//! ## VFS Structure
//!
//! ```text
//! /scripts/                     # JavaScript code executed
//! /results/                     # Raw query results
//! /workspace/{task_id}/         # Processed data
//! /history/                     # Execution log
//! /final/                       # Final outputs
//! /pages/{page_id}/             # Generated pages
//! ```

#![warn(clippy::all, rust_2018_idioms)]

mod entry;
pub mod registry;

pub use entry::{VfsDirEntry, VfsEntry, VfsMetadata};
pub use registry::{
    deregister_vfs, get_current_vfs_id, register_vfs, set_current_vfs_id, vfs_exists, with_vfs,
    with_vfs_mut,
};

use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Default maximum VFS size: 100MB
pub const DEFAULT_MAX_SIZE: usize = 100 * 1024 * 1024;

/// Virtual File System for agent memory
///
/// Stores files and directories in memory with size limits.
/// Used to store query results, scripts, and generated pages.
#[derive(Debug)]
pub struct VirtualFileSystem {
    /// All files and directories indexed by path
    files: HashMap<PathBuf, VfsEntry>,
    /// Current total size of all file contents
    total_size: usize,
    /// Maximum allowed size
    max_size: usize,
    /// When this VFS was created
    created_at: Instant,
}

impl VirtualFileSystem {
    /// Create a new VFS with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        let mut vfs = Self {
            files: HashMap::new(),
            total_size: 0,
            max_size,
            created_at: Instant::now(),
        };

        // Create root directories
        let _ = vfs.mkdir("/scripts");
        let _ = vfs.mkdir("/results");
        let _ = vfs.mkdir("/workspace");
        let _ = vfs.mkdir("/history");
        let _ = vfs.mkdir("/final");
        let _ = vfs.mkdir("/pages");

        vfs
    }

    /// Create a new VFS with default 100MB limit
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_MAX_SIZE)
    }

    /// Normalize a path to ensure consistent formatting
    fn normalize_path(path: &str) -> PathBuf {
        let path = path.trim();
        // Ensure path starts with /
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        // Remove trailing slash unless it's the root
        let path = if path.len() > 1 && path.ends_with('/') {
            &path[..path.len() - 1]
        } else {
            &path
        };
        PathBuf::from(path)
    }

    /// Get the parent path of a given path
    fn parent_path(path: &Path) -> Option<PathBuf> {
        path.parent().map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from("/")
            } else {
                p.to_path_buf()
            }
        })
    }

    /// Write a file to the VFS
    ///
    /// Creates parent directories if they don't exist.
    /// Returns an error if the size limit would be exceeded.
    pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<()> {
        let path = Self::normalize_path(path);
        let content_size = content.len();

        // Check if file already exists and get old size
        let old_size = self
            .files
            .get(&path)
            .filter(|e| e.is_file())
            .map(|e| e.size())
            .unwrap_or(0);

        // Calculate new total size
        let size_delta = content_size as isize - old_size as isize;
        let new_total = (self.total_size as isize + size_delta) as usize;

        if new_total > self.max_size {
            bail!(
                "VFS size limit exceeded: current {} + requested {} > limit {}",
                self.total_size,
                content_size,
                self.max_size
            );
        }

        // Create parent directories if needed
        if let Some(parent) = Self::parent_path(&path) {
            if parent != PathBuf::from("/") {
                self.mkdir_recursive(&parent)?;
            }
        }

        // Write the file
        self.files
            .insert(path.clone(), VfsEntry::new_file(content.to_vec()));
        self.total_size = new_total;

        tracing::trace!(
            path = %path.display(),
            size = content_size,
            total_size = self.total_size,
            "VFS: wrote file"
        );

        Ok(())
    }

    /// Read a file from the VFS
    ///
    /// Returns the file content or an error if not found.
    pub fn read_file(&self, path: &str) -> Result<&[u8]> {
        let path = Self::normalize_path(path);

        match self.files.get(&path) {
            Some(entry) if entry.is_file() => Ok(&entry.content),
            Some(_) => Err(anyhow!("Path is a directory: {}", path.display())),
            None => Err(anyhow!("File not found: {}", path.display())),
        }
    }

    /// Check if a path exists in the VFS
    pub fn exists(&self, path: &str) -> bool {
        let path = Self::normalize_path(path);
        self.files.contains_key(&path)
    }

    /// Check if a path is a file
    pub fn is_file(&self, path: &str) -> bool {
        let path = Self::normalize_path(path);
        self.files.get(&path).is_some_and(|e| e.is_file())
    }

    /// Check if a path is a directory
    pub fn is_directory(&self, path: &str) -> bool {
        let path = Self::normalize_path(path);
        self.files.get(&path).is_some_and(|e| e.is_directory())
    }

    /// Create a directory (single level)
    pub fn mkdir(&mut self, path: &str) -> Result<()> {
        let path = Self::normalize_path(path);

        if self.files.contains_key(&path) {
            return Ok(()); // Already exists
        }

        // Check parent exists (except for root-level directories)
        if let Some(parent) = Self::parent_path(&path) {
            if parent != PathBuf::from("/") && !self.files.contains_key(&parent) {
                bail!("Parent directory does not exist: {}", parent.display());
            }
        }

        self.files.insert(path, VfsEntry::new_directory());
        Ok(())
    }

    /// Create a directory and all parent directories
    fn mkdir_recursive(&mut self, path: &Path) -> Result<()> {
        let mut current = PathBuf::from("/");

        for component in path.components().skip(1) {
            // Skip the root
            current.push(component);
            if !self.files.contains_key(&current) {
                self.files
                    .insert(current.clone(), VfsEntry::new_directory());
            }
        }

        Ok(())
    }

    /// List entries in a directory
    pub fn list_dir(&self, path: &str) -> Result<Vec<VfsDirEntry>> {
        let path = Self::normalize_path(path);

        // Check if directory exists
        match self.files.get(&path) {
            Some(entry) if !entry.is_directory() => {
                bail!("Path is not a directory: {}", path.display())
            }
            None if path != PathBuf::from("/") => {
                bail!("Directory not found: {}", path.display())
            }
            _ => {}
        }

        let path_str = path.display().to_string();
        let prefix = if path_str == "/" {
            "/".to_string()
        } else {
            format!("{}/", path_str)
        };

        let mut entries = Vec::new();

        for (entry_path, entry) in &self.files {
            let entry_str = entry_path.display().to_string();

            // Check if this is a direct child of the directory
            if entry_str.starts_with(&prefix) {
                let remainder = &entry_str[prefix.len()..];
                // Only include direct children (no more slashes)
                if !remainder.is_empty() && !remainder.contains('/') {
                    entries.push(VfsDirEntry::new(
                        remainder.to_string(),
                        entry.is_directory(),
                        entry.size(),
                    ));
                }
            }
        }

        // Sort by name for consistent ordering
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(entries)
    }

    /// Delete a file or empty directory
    pub fn delete(&mut self, path: &str) -> Result<()> {
        let path = Self::normalize_path(path);

        match self.files.get(&path) {
            Some(entry) if entry.is_directory() => {
                // Check if directory is empty
                let path_str = path.display().to_string();
                let prefix = format!("{}/", path_str);
                let has_children = self
                    .files
                    .keys()
                    .any(|p| p.display().to_string().starts_with(&prefix));

                if has_children {
                    bail!("Cannot delete non-empty directory: {}", path.display());
                }
            }
            None => bail!("File not found: {}", path.display()),
            _ => {}
        }

        if let Some(entry) = self.files.remove(&path) {
            if entry.is_file() {
                self.total_size -= entry.size();
            }
        }

        Ok(())
    }

    /// Get file/directory metadata
    pub fn stat(&self, path: &str) -> Result<&VfsMetadata> {
        let path = Self::normalize_path(path);

        self.files
            .get(&path)
            .map(|e| &e.metadata)
            .ok_or_else(|| anyhow!("File not found: {}", path.display()))
    }

    /// Get the current total size of all files
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get the maximum allowed size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Get the remaining available space
    pub fn available_space(&self) -> usize {
        self.max_size.saturating_sub(self.total_size)
    }

    /// Get when this VFS was created
    pub fn created_at(&self) -> Instant {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_vfs_has_default_directories() {
        let vfs = VirtualFileSystem::new(1024 * 1024);

        assert!(vfs.is_directory("/scripts"));
        assert!(vfs.is_directory("/results"));
        assert!(vfs.is_directory("/workspace"));
        assert!(vfs.is_directory("/history"));
        assert!(vfs.is_directory("/final"));
        assert!(vfs.is_directory("/pages"));
    }

    #[test]
    fn test_write_and_read_file() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);

        vfs.write_file("/results/test.json", b"{\"count\": 42}")
            .unwrap();

        let content = vfs.read_file("/results/test.json").unwrap();
        assert_eq!(content, b"{\"count\": 42}");
    }

    #[test]
    fn test_size_tracking() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);

        vfs.write_file("/test1.txt", b"hello").unwrap();
        assert_eq!(vfs.total_size(), 5);

        vfs.write_file("/test2.txt", b"world!").unwrap();
        assert_eq!(vfs.total_size(), 11);

        // Overwrite file
        vfs.write_file("/test1.txt", b"hi").unwrap();
        assert_eq!(vfs.total_size(), 8); // 2 + 6
    }

    #[test]
    fn test_size_limit() {
        let mut vfs = VirtualFileSystem::new(10); // 10 bytes max

        vfs.write_file("/test.txt", b"hello").unwrap();
        assert_eq!(vfs.total_size(), 5);

        let result = vfs.write_file("/test2.txt", b"world!!");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("size limit exceeded"));
    }

    #[test]
    fn test_auto_create_parent_directories() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);

        vfs.write_file("/pages/my-dashboard/index.html", b"<html>")
            .unwrap();

        assert!(vfs.is_directory("/pages/my-dashboard"));
        assert!(vfs.is_file("/pages/my-dashboard/index.html"));
    }

    #[test]
    fn test_list_directory() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);

        vfs.write_file("/pages/dashboard/index.html", b"<html>")
            .unwrap();
        vfs.write_file("/pages/dashboard/app.js", b"//js").unwrap();
        vfs.mkdir("/pages/dashboard/assets").unwrap();

        let entries = vfs.list_dir("/pages/dashboard").unwrap();
        assert_eq!(entries.len(), 3);

        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"index.html"));
        assert!(names.contains(&"app.js"));
        assert!(names.contains(&"assets"));
    }

    #[test]
    fn test_delete_file() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);

        vfs.write_file("/test.txt", b"hello").unwrap();
        assert_eq!(vfs.total_size(), 5);

        vfs.delete("/test.txt").unwrap();
        assert!(!vfs.exists("/test.txt"));
        assert_eq!(vfs.total_size(), 0);
    }

    #[test]
    fn test_path_normalization() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);

        vfs.write_file("test.txt", b"no leading slash").unwrap();
        assert!(vfs.exists("/test.txt"));

        vfs.write_file("/trailing/", b"trailing slash").unwrap();
        // This creates a file at /trailing
        assert!(vfs.is_file("/trailing"));
    }
}
