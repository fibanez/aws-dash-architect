//! VFS Global Registry
//!
//! Provides global access to VFS instances via string IDs.
//! TaskManager registers its VFS here, workers access it via ID.

#![warn(clippy::all, rust_2018_idioms)]

use super::VirtualFileSystem;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// Global registry of VFS instances
static VFS_REGISTRY: Lazy<RwLock<HashMap<String, VirtualFileSystem>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// Thread-local storage for the current VFS ID
thread_local! {
    static CURRENT_VFS_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Register a new VFS instance in the global registry
///
/// Returns the generated VFS ID that can be used to access the VFS.
pub fn register_vfs(vfs: VirtualFileSystem) -> String {
    let id = Uuid::new_v4().to_string();
    let mut registry = VFS_REGISTRY.write().expect("VFS registry poisoned");
    registry.insert(id.clone(), vfs);
    tracing::debug!(vfs_id = %id, "Registered VFS in global registry");
    id
}

/// Deregister a VFS instance from the global registry
///
/// This should be called when a TaskManager terminates to free memory.
pub fn deregister_vfs(vfs_id: &str) {
    let mut registry = VFS_REGISTRY.write().expect("VFS registry poisoned");
    if registry.remove(vfs_id).is_some() {
        tracing::debug!(vfs_id = %vfs_id, "Deregistered VFS from global registry");
    } else {
        tracing::warn!(vfs_id = %vfs_id, "Attempted to deregister non-existent VFS");
    }
}

/// Execute a closure with read access to a VFS
///
/// Returns None if the VFS ID doesn't exist.
pub fn with_vfs<F, R>(vfs_id: &str, f: F) -> Option<R>
where
    F: FnOnce(&VirtualFileSystem) -> R,
{
    let registry = VFS_REGISTRY.read().expect("VFS registry poisoned");
    registry.get(vfs_id).map(f)
}

/// Execute a closure with mutable access to a VFS
///
/// Returns None if the VFS ID doesn't exist.
pub fn with_vfs_mut<F, R>(vfs_id: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut VirtualFileSystem) -> R,
{
    let mut registry = VFS_REGISTRY.write().expect("VFS registry poisoned");
    registry.get_mut(vfs_id).map(f)
}

/// Set the current thread's VFS ID
///
/// This should be called before executing V8 code so that
/// V8 bindings can access the correct VFS.
pub fn set_current_vfs_id(id: Option<String>) {
    CURRENT_VFS_ID.with(|cell| {
        *cell.borrow_mut() = id;
    });
}

/// Get the current thread's VFS ID
///
/// Returns None if no VFS ID has been set for this thread.
pub fn get_current_vfs_id() -> Option<String> {
    CURRENT_VFS_ID.with(|cell| cell.borrow().clone())
}

/// Check if a VFS exists in the registry
pub fn vfs_exists(vfs_id: &str) -> bool {
    let registry = VFS_REGISTRY.read().expect("VFS registry poisoned");
    registry.contains_key(vfs_id)
}

/// Get the number of registered VFS instances (for debugging)
#[allow(dead_code)]
pub fn registry_size() -> usize {
    let registry = VFS_REGISTRY.read().expect("VFS registry poisoned");
    registry.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_deregister() {
        let vfs = VirtualFileSystem::new(1024 * 1024); // 1MB
        let id = register_vfs(vfs);

        assert!(vfs_exists(&id));

        deregister_vfs(&id);
        assert!(!vfs_exists(&id));
    }

    #[test]
    fn test_with_vfs() {
        let mut vfs = VirtualFileSystem::new(1024 * 1024);
        vfs.write_file("/test.txt", b"hello").unwrap();
        let id = register_vfs(vfs);

        let result = with_vfs(&id, |vfs| vfs.read_file("/test.txt").map(|c| c.to_vec()));

        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), b"hello");

        deregister_vfs(&id);
    }

    #[test]
    fn test_with_vfs_mut() {
        let vfs = VirtualFileSystem::new(1024 * 1024);
        let id = register_vfs(vfs);

        with_vfs_mut(&id, |vfs| {
            vfs.write_file("/test.txt", b"hello").unwrap();
        });

        let content = with_vfs(&id, |vfs| vfs.read_file("/test.txt").map(|c| c.to_vec()));
        assert_eq!(content.unwrap().unwrap(), b"hello");

        deregister_vfs(&id);
    }

    #[test]
    fn test_thread_local_vfs_id() {
        assert!(get_current_vfs_id().is_none());

        set_current_vfs_id(Some("test-id".to_string()));
        assert_eq!(get_current_vfs_id(), Some("test-id".to_string()));

        set_current_vfs_id(None);
        assert!(get_current_vfs_id().is_none());
    }

    #[test]
    fn test_nonexistent_vfs() {
        let result = with_vfs("nonexistent", |_| 42);
        assert!(result.is_none());
    }
}
