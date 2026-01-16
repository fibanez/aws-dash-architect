//! VFS Entry and Metadata Types
//!
//! Defines the data structures for files stored in the Virtual File System.

#![warn(clippy::all, rust_2018_idioms)]

use std::time::Instant;

/// Metadata for a VFS entry
#[derive(Debug, Clone)]
pub struct VfsMetadata {
    /// Size of the content in bytes
    pub size: usize,
    /// When the entry was created
    pub created_at: Instant,
    /// When the entry was last modified
    pub modified_at: Instant,
    /// Whether this is a directory
    pub is_directory: bool,
}

impl VfsMetadata {
    /// Create metadata for a file with the given size
    pub fn new_file(size: usize) -> Self {
        let now = Instant::now();
        Self {
            size,
            created_at: now,
            modified_at: now,
            is_directory: false,
        }
    }

    /// Create metadata for a directory
    pub fn new_directory() -> Self {
        let now = Instant::now();
        Self {
            size: 0,
            created_at: now,
            modified_at: now,
            is_directory: true,
        }
    }
}

/// A file or directory entry in the VFS
#[derive(Debug, Clone)]
pub struct VfsEntry {
    /// Content of the file (empty for directories)
    pub content: Vec<u8>,
    /// Metadata about this entry
    pub metadata: VfsMetadata,
}

impl VfsEntry {
    /// Create a new file entry with the given content
    pub fn new_file(content: Vec<u8>) -> Self {
        let size = content.len();
        Self {
            content,
            metadata: VfsMetadata::new_file(size),
        }
    }

    /// Create a new directory entry
    pub fn new_directory() -> Self {
        Self {
            content: Vec::new(),
            metadata: VfsMetadata::new_directory(),
        }
    }

    /// Check if this entry is a directory
    pub fn is_directory(&self) -> bool {
        self.metadata.is_directory
    }

    /// Check if this entry is a file
    pub fn is_file(&self) -> bool {
        !self.metadata.is_directory
    }

    /// Get the size of this entry
    pub fn size(&self) -> usize {
        self.metadata.size
    }

    /// Update the content of this file entry
    #[allow(dead_code)]
    pub fn update_content(&mut self, content: Vec<u8>) {
        self.metadata.size = content.len();
        self.metadata.modified_at = Instant::now();
        self.content = content;
    }
}

/// Information about a directory entry for listing
#[derive(Debug, Clone)]
pub struct VfsDirEntry {
    /// Name of the entry (just the filename, not full path)
    pub name: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// Size in bytes (0 for directories)
    pub size: usize,
}

impl VfsDirEntry {
    /// Create a new directory entry info
    pub fn new(name: String, is_directory: bool, size: usize) -> Self {
        Self {
            name,
            is_directory,
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_file_entry() {
        let content = b"Hello, World!".to_vec();
        let entry = VfsEntry::new_file(content.clone());

        assert!(entry.is_file());
        assert!(!entry.is_directory());
        assert_eq!(entry.size(), 13);
        assert_eq!(entry.content, content);
    }

    #[test]
    fn test_new_directory_entry() {
        let entry = VfsEntry::new_directory();

        assert!(entry.is_directory());
        assert!(!entry.is_file());
        assert_eq!(entry.size(), 0);
        assert!(entry.content.is_empty());
    }

    #[test]
    fn test_dir_entry_info() {
        let file_info = VfsDirEntry::new("test.txt".to_string(), false, 100);
        assert_eq!(file_info.name, "test.txt");
        assert!(!file_info.is_directory);
        assert_eq!(file_info.size, 100);

        let dir_info = VfsDirEntry::new("subdir".to_string(), true, 0);
        assert_eq!(dir_info.name, "subdir");
        assert!(dir_info.is_directory);
    }
}
