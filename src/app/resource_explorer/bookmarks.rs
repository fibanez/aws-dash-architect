use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::app::resource_explorer::{GroupingMode, ResourceExplorerState, TagFilterGroup};

/// A folder for organizing bookmarks hierarchically
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookmarkFolder {
    pub id: String, // UUID
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>, // None = Top Folder
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl BookmarkFolder {
    pub fn new(name: String, parent_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            parent_id,
            created_at: now,
            modified_at: now,
        }
    }
}

/// A saved Explorer configuration that can be restored
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String, // UUID
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>, // Emoji or icon identifier
    #[serde(default)]
    pub folder_id: Option<String>, // Folder this bookmark belongs to (None = Top Folder)

    // Explorer state - simplified to store just IDs/names
    pub account_ids: Vec<String>,       // AccountSelection.account_id
    pub region_codes: Vec<String>,      // RegionSelection.region_code
    pub resource_type_ids: Vec<String>, // ResourceTypeSelection.resource_type
    pub grouping: GroupingMode,
    pub tag_filters: TagFilterGroup,
    pub search_filter: String,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub access_count: usize,
    pub last_accessed: Option<DateTime<Utc>>,
}

impl Bookmark {
    /// Create a new bookmark from current Explorer state
    pub fn new(name: String, state: &ResourceExplorerState) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            icon: None,
            folder_id: None, // New bookmarks start in Top Folder
            account_ids: state
                .query_scope
                .accounts
                .iter()
                .map(|a| a.account_id.clone())
                .collect(),
            region_codes: state
                .query_scope
                .regions
                .iter()
                .map(|r| r.region_code.clone())
                .collect(),
            resource_type_ids: state
                .query_scope
                .resource_types
                .iter()
                .map(|rt| rt.resource_type.clone())
                .collect(),
            grouping: state.primary_grouping.clone(),
            tag_filters: state.tag_filter_group.clone(),
            search_filter: state.search_filter.clone(),
            created_at: now,
            modified_at: now,
            access_count: 0,
            last_accessed: None,
        }
    }

    /// Apply this bookmark's state to an Explorer state
    ///
    /// Note: This sets the IDs but doesn't rebuild the full selections.
    /// The caller is responsible for validating and rebuilding selections
    /// from the stored IDs.
    pub fn apply_to_state(&mut self, state: &mut ResourceExplorerState) {
        // Note: We can only set the simple fields here.
        // The query_scope selections need to be rebuilt by the caller
        // from the stored IDs, as they contain additional metadata
        // (display names, colors) that we don't store.

        state.primary_grouping = self.grouping.clone();
        state.tag_filter_group = self.tag_filters.clone();
        state.search_filter = self.search_filter.clone();

        // Update access metadata
        self.access_count += 1;
        self.last_accessed = Some(Utc::now());
        self.modified_at = Utc::now();
    }

    /// Check if this bookmark's core state matches the given Explorer state
    pub fn matches_state(&self, state: &ResourceExplorerState) -> bool {
        // Extract IDs from current state for comparison
        let current_account_ids: Vec<String> = state
            .query_scope
            .accounts
            .iter()
            .map(|a| a.account_id.clone())
            .collect();
        let current_region_codes: Vec<String> = state
            .query_scope
            .regions
            .iter()
            .map(|r| r.region_code.clone())
            .collect();
        let current_resource_type_ids: Vec<String> = state
            .query_scope
            .resource_types
            .iter()
            .map(|rt| rt.resource_type.clone())
            .collect();

        self.account_ids == current_account_ids
            && self.region_codes == current_region_codes
            && self.resource_type_ids == current_resource_type_ids
            && self.grouping == state.primary_grouping
            && self.tag_filters == state.tag_filter_group
            && self.search_filter == state.search_filter
    }
}

/// Collection of bookmarks with version tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkCollection {
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub folders: Vec<BookmarkFolder>,
    pub version: u32, // Schema version for future migrations
}

impl BookmarkCollection {
    pub fn new() -> Self {
        Self {
            bookmarks: Vec::new(),
            folders: Vec::new(),
            version: 1,
        }
    }

    pub fn add(&mut self, bookmark: Bookmark) {
        self.bookmarks.push(bookmark);
    }

    pub fn remove(&mut self, id: &str) -> Option<Bookmark> {
        if let Some(index) = self.bookmarks.iter().position(|b| b.id == id) {
            Some(self.bookmarks.remove(index))
        } else {
            None
        }
    }

    pub fn get(&self, id: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Bookmark> {
        self.bookmarks.iter_mut().find(|b| b.id == id)
    }

    pub fn reorder(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.bookmarks.len() && to_index < self.bookmarks.len() {
            let bookmark = self.bookmarks.remove(from_index);
            self.bookmarks.insert(to_index, bookmark);
        }
    }
}

impl Default for BookmarkCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for bookmarks with auto-save support
pub struct BookmarkManager {
    file_path: PathBuf,
    collection: BookmarkCollection,

    // Auto-save state
    last_session: Bookmark,  // Hidden bookmark for session restoration
    auto_save_dirty: bool,   // Track unsaved auto-save changes
    last_auto_save: Instant, // Debounce auto-save operations

    // Manual bookmark state
    dirty: bool, // Track unsaved manual bookmark changes
}

impl BookmarkManager {
    /// Create a new bookmark manager, loading from disk if available
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("awsdash");

        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        let file_path = config_dir.join("bookmarks.json");

        let (collection, last_session) = if file_path.exists() {
            Self::load_from_file(&file_path)?
        } else {
            (BookmarkCollection::new(), Self::create_empty_auto_save())
        };

        Ok(Self {
            file_path,
            collection,
            last_session,
            auto_save_dirty: false,
            last_auto_save: Instant::now(),
            dirty: false,
        })
    }

    /// Load bookmarks from file
    fn load_from_file(path: &PathBuf) -> Result<(BookmarkCollection, Bookmark)> {
        let contents = fs::read_to_string(path).context("Failed to read bookmarks file")?;

        // Try to deserialize as the full structure with auto-save
        #[derive(Deserialize)]
        struct BookmarkFile {
            collection: BookmarkCollection,
            last_session: Option<Bookmark>,
        }

        let file: BookmarkFile =
            serde_json::from_str(&contents).context("Failed to parse bookmarks JSON")?;

        let last_session = file
            .last_session
            .unwrap_or_else(Self::create_empty_auto_save);

        Ok((file.collection, last_session))
    }

    /// Create an empty auto-save bookmark (default state)
    fn create_empty_auto_save() -> Bookmark {
        let now = Utc::now();
        Bookmark {
            id: "__auto_save__".to_string(),
            name: "Auto Save".to_string(),
            description: None,
            icon: None,
            folder_id: None, // Auto-save not in any folder
            account_ids: Vec::new(),
            region_codes: Vec::new(),
            resource_type_ids: Vec::new(),
            grouping: GroupingMode::ByAccount,
            tag_filters: TagFilterGroup::new(),
            search_filter: String::new(),
            created_at: now,
            modified_at: now,
            access_count: 0,
            last_accessed: None,
        }
    }

    /// Save bookmarks to disk
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty && !self.auto_save_dirty {
            return Ok(());
        }

        // Serialize both collection and auto-save
        #[derive(Serialize)]
        struct BookmarkFile<'a> {
            collection: &'a BookmarkCollection,
            last_session: &'a Bookmark,
        }

        let file = BookmarkFile {
            collection: &self.collection,
            last_session: &self.last_session,
        };

        let json = serde_json::to_string_pretty(&file).context("Failed to serialize bookmarks")?;

        // Atomic write with temp file
        let temp_path = self.file_path.with_extension("json.tmp");
        fs::write(&temp_path, json).context("Failed to write temp bookmarks file")?;

        fs::rename(&temp_path, &self.file_path).context("Failed to rename temp bookmarks file")?;

        self.dirty = false;
        self.auto_save_dirty = false;
        Ok(())
    }

    /// Update the auto-save bookmark with current state
    /// Debounced: Only saves if 2 seconds elapsed since last save
    pub fn update_auto_save(&mut self, state: &ResourceExplorerState) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_auto_save) < Duration::from_secs(2) {
            // Mark dirty but don't save yet
            self.auto_save_dirty = true;
            return Ok(());
        }

        // Update last_session bookmark
        self.last_session = Bookmark::new("Auto Save".to_string(), state);
        self.last_session.id = "__auto_save__".to_string(); // Fixed ID

        // Save to disk
        self.save()?;
        self.auto_save_dirty = false;
        self.last_auto_save = now;

        tracing::debug!("Auto-saved Explorer state");
        Ok(())
    }

    /// Get the auto-save bookmark for restoration
    pub fn get_auto_save(&self) -> &Bookmark {
        &self.last_session
    }

    /// Force save auto-save if dirty (called on app exit)
    pub fn flush_auto_save(&mut self) -> Result<()> {
        if self.auto_save_dirty {
            self.save()?;
            self.auto_save_dirty = false;
            tracing::info!("Flushed pending auto-save on exit");
        }
        Ok(())
    }

    /// Add a user bookmark
    pub fn add_bookmark(&mut self, bookmark: Bookmark) {
        self.collection.add(bookmark);
        self.dirty = true;
    }

    /// Remove a user bookmark
    pub fn remove_bookmark(&mut self, id: &str) -> Option<Bookmark> {
        let result = self.collection.remove(id);
        if result.is_some() {
            self.dirty = true;
        }
        result
    }

    /// Get all user bookmarks
    pub fn get_bookmarks(&self) -> &[Bookmark] {
        &self.collection.bookmarks
    }

    /// Get a bookmark by ID
    pub fn get_bookmark(&self, id: &str) -> Option<&Bookmark> {
        self.collection.bookmarks.iter().find(|b| b.id == id)
    }

    /// Get mutable reference to a bookmark
    pub fn get_bookmark_mut(&mut self, id: &str) -> Option<&mut Bookmark> {
        self.dirty = true;
        self.collection.get_mut(id)
    }

    /// Reorder bookmarks
    pub fn reorder(&mut self, from_index: usize, to_index: usize) {
        self.collection.reorder(from_index, to_index);
        self.dirty = true;
    }

    /// Export bookmarks to a file
    pub fn export_to_file(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.collection)
            .context("Failed to serialize bookmarks")?;
        fs::write(path, json).context("Failed to write export file")?;
        Ok(())
    }

    /// Import bookmarks from a file
    pub fn import_from_file(&mut self, path: &PathBuf) -> Result<usize> {
        let contents = fs::read_to_string(path).context("Failed to read import file")?;
        let imported: BookmarkCollection =
            serde_json::from_str(&contents).context("Failed to parse import file")?;

        let count = imported.bookmarks.len();
        for bookmark in imported.bookmarks {
            self.add_bookmark(bookmark);
        }

        Ok(count)
    }

    /// Find bookmark matching current state
    pub fn find_matching_bookmark(&self, state: &ResourceExplorerState) -> Option<&Bookmark> {
        self.collection
            .bookmarks
            .iter()
            .find(|b| b.matches_state(state))
    }

    // ========================================================================
    // Folder Management Methods
    // ========================================================================

    /// Add a new folder
    pub fn add_folder(&mut self, folder: BookmarkFolder) {
        self.collection.folders.push(folder);
        self.dirty = true;
    }

    /// Remove a folder (only if it's empty - no bookmarks or subfolders)
    pub fn remove_folder(&mut self, id: &str) -> Result<Option<BookmarkFolder>> {
        // Check if folder has bookmarks
        let has_bookmarks = self
            .collection
            .bookmarks
            .iter()
            .any(|b| b.folder_id.as_ref() == Some(&id.to_string()));

        if has_bookmarks {
            return Err(anyhow::anyhow!("Cannot delete folder with bookmarks"));
        }

        // Check if folder has subfolders
        let has_subfolders = self
            .collection
            .folders
            .iter()
            .any(|f| f.parent_id.as_ref() == Some(&id.to_string()));

        if has_subfolders {
            return Err(anyhow::anyhow!("Cannot delete folder with subfolders"));
        }

        // Remove folder
        if let Some(index) = self.collection.folders.iter().position(|f| f.id == id) {
            let folder = self.collection.folders.remove(index);
            self.dirty = true;
            Ok(Some(folder))
        } else {
            Ok(None)
        }
    }

    /// Move a bookmark to a different folder
    pub fn move_bookmark_to_folder(&mut self, bookmark_id: &str, folder_id: Option<String>) {
        if let Some(bookmark) = self
            .collection
            .bookmarks
            .iter_mut()
            .find(|b| b.id == bookmark_id)
        {
            bookmark.folder_id = folder_id;
            bookmark.modified_at = chrono::Utc::now();
            self.dirty = true;
        }
    }

    /// Get a folder by ID
    pub fn get_folder(&self, id: &str) -> Option<&BookmarkFolder> {
        self.collection.folders.iter().find(|f| f.id == id)
    }

    /// Get mutable reference to a folder
    pub fn get_folder_mut(&mut self, id: &str) -> Option<&mut BookmarkFolder> {
        self.dirty = true;
        self.collection.folders.iter_mut().find(|f| f.id == id)
    }

    /// Get all bookmarks in a specific folder (or Top Folder if folder_id is None)
    pub fn get_bookmarks_in_folder(&self, folder_id: Option<&String>) -> Vec<&Bookmark> {
        self.collection
            .bookmarks
            .iter()
            .filter(|b| b.folder_id.as_ref() == folder_id)
            .collect()
    }

    /// Get all subfolders of a specific folder (or Top Folder if parent_id is None)
    pub fn get_subfolders(&self, parent_id: Option<&String>) -> Vec<&BookmarkFolder> {
        self.collection
            .folders
            .iter()
            .filter(|f| f.parent_id.as_ref() == parent_id)
            .collect()
    }

    /// Get the full folder path (breadcrumbs) for a folder
    pub fn get_folder_path(&self, folder_id: Option<&String>) -> Vec<&BookmarkFolder> {
        let mut path = Vec::new();
        let mut current_id = folder_id;

        while let Some(id) = current_id {
            if let Some(folder) = self.get_folder(id) {
                path.push(folder);
                current_id = folder.parent_id.as_ref();
            } else {
                break;
            }
        }

        path.reverse(); // Return path from Top Folder to current folder
        path
    }

    /// Get all folders
    pub fn get_all_folders(&self) -> &[BookmarkFolder] {
        &self.collection.folders
    }

    /// Move a folder to a different parent folder
    /// Returns error if the move would create a circular reference
    pub fn move_folder_to_parent(
        &mut self,
        folder_id: &str,
        new_parent_id: Option<String>,
    ) -> Result<()> {
        // Prevent moving a folder into itself
        if Some(folder_id.to_string()) == new_parent_id {
            return Err(anyhow::anyhow!("Cannot move folder into itself"));
        }

        // Prevent circular references: check if new parent is a descendant of this folder
        if let Some(ref parent_id) = new_parent_id {
            if self.is_descendant(parent_id, folder_id) {
                return Err(anyhow::anyhow!(
                    "Cannot move folder into its own descendant (would create circular reference)"
                ));
            }
        }

        // Move the folder
        if let Some(folder) = self
            .collection
            .folders
            .iter_mut()
            .find(|f| f.id == folder_id)
        {
            folder.parent_id = new_parent_id;
            folder.modified_at = chrono::Utc::now();
            self.dirty = true;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Folder not found"))
        }
    }

    /// Check if `potential_descendant` is a descendant of `ancestor_id`
    pub fn is_descendant(&self, potential_descendant: &str, ancestor_id: &str) -> bool {
        let mut current_id = Some(potential_descendant.to_string());

        while let Some(id) = current_id {
            if id == ancestor_id {
                return true;
            }
            // Get parent of current folder
            current_id = self
                .collection
                .folders
                .iter()
                .find(|f| f.id == id)
                .and_then(|f| f.parent_id.clone());
        }

        false
    }
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create bookmark manager: {}", e);
            Self {
                file_path: PathBuf::from("bookmarks.json"),
                collection: BookmarkCollection::new(),
                last_session: Self::create_empty_auto_save(),
                auto_save_dirty: false,
                last_auto_save: Instant::now(),
                dirty: false,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_creation() {
        let state = ResourceExplorerState::default();
        let bookmark = Bookmark::new("Test".to_string(), &state);

        assert_eq!(bookmark.name, "Test");
        assert_eq!(bookmark.account_ids.len(), 0); // Default state has no accounts
        assert_eq!(bookmark.access_count, 0);
        assert!(bookmark.last_accessed.is_none());
    }

    #[test]
    fn test_bookmark_collection() {
        let mut collection = BookmarkCollection::new();
        assert_eq!(collection.bookmarks.len(), 0);

        let state = ResourceExplorerState::default();
        let bookmark = Bookmark::new("Test".to_string(), &state);
        let id = bookmark.id.clone();

        collection.add(bookmark);
        assert_eq!(collection.bookmarks.len(), 1);

        let retrieved = collection.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test");

        let removed = collection.remove(&id);
        assert!(removed.is_some());
        assert_eq!(collection.bookmarks.len(), 0);
    }

    #[test]
    fn test_bookmark_apply_to_state() {
        let state = ResourceExplorerState::default();
        let mut bookmark = Bookmark::new("Test".to_string(), &state);
        assert_eq!(bookmark.access_count, 0);

        let mut new_state = ResourceExplorerState::default();
        bookmark.apply_to_state(&mut new_state);

        assert_eq!(bookmark.access_count, 1);
        assert!(bookmark.last_accessed.is_some());
    }
}
