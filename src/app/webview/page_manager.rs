//! Dash Pages Manager - Persistence and organization for user-created pages
//!
//! This module manages the lifecycle of Dash Pages:
//! - Creating temporary pages from agent-generated HTML
//! - Saving pages to persistent storage
//! - Organizing pages in folder hierarchies
//! - Managing concurrent access with file locking

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock as StdRwLock};
use uuid::Uuid;

/// A folder for organizing Dash Pages hierarchically
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageFolder {
    pub id: String,                   // UUID
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,    // None = root level
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl PageFolder {
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

/// A Dash Page - custom HTML/JS application with access to AWS data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashPage {
    pub id: String,                   // UUID
    pub name: String,
    pub description: Option<String>,
    pub folder_id: Option<String>,    // Folder this page belongs to (None = root)

    // File storage
    pub page_path: PathBuf,           // Path to page directory (e.g., ~/.local/share/awsdash/pages/{name})

    // Metadata
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub access_count: usize,
    pub last_accessed: Option<DateTime<Utc>>,
}

impl DashPage {
    /// Create a new page
    pub fn new(name: String, page_path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            folder_id: None,
            page_path,
            created_at: now,
            modified_at: now,
            access_count: 0,
            last_accessed: None,
        }
    }

    /// Record access to this page
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(Utc::now());
        self.modified_at = Utc::now();
    }
}

/// Collection of pages and folders with version tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCollection {
    pub pages: Vec<DashPage>,
    pub folders: Vec<PageFolder>,
    pub version: u32,  // Schema version for future migrations
}

impl PageCollection {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            folders: Vec::new(),
            version: 1,
        }
    }

    pub fn add_page(&mut self, page: DashPage) {
        self.pages.push(page);
    }

    pub fn remove_page(&mut self, id: &str) -> Option<DashPage> {
        if let Some(index) = self.pages.iter().position(|t| t.id == id) {
            Some(self.pages.remove(index))
        } else {
            None
        }
    }

    pub fn get_page(&self, id: &str) -> Option<&DashPage> {
        self.pages.iter().find(|t| t.id == id)
    }

    pub fn get_page_mut(&mut self, id: &str) -> Option<&mut DashPage> {
        self.pages.iter_mut().find(|t| t.id == id)
    }

    pub fn add_folder(&mut self, folder: PageFolder) {
        self.folders.push(folder);
    }

    pub fn remove_folder(&mut self, id: &str) -> Option<PageFolder> {
        if let Some(index) = self.folders.iter().position(|f| f.id == id) {
            Some(self.folders.remove(index))
        } else {
            None
        }
    }

    pub fn get_folder(&self, id: &str) -> Option<&PageFolder> {
        self.folders.iter().find(|f| f.id == id)
    }

    pub fn get_folder_mut(&mut self, id: &str) -> Option<&mut PageFolder> {
        self.folders.iter_mut().find(|f| f.id == id)
    }
}

impl Default for PageCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for Dash Pages with file locking for concurrent access
pub struct PageManager {
    pages_dir: PathBuf,         // ~/.local/share/awsdash/pages
    temp_pages_dir: PathBuf,    // ~/.local/share/awsdash/temp_pages
    manifest_path: PathBuf,     // ~/.local/share/awsdash/dash_pages_manifest.json
    collection: PageCollection,
}

impl PageManager {
    /// Create a new page manager, loading from disk if available
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("awsdash");

        let pages_dir = data_dir.join("pages");
        let temp_pages_dir = data_dir.join("temp_pages");
        let manifest_path = data_dir.join("dash_pages_manifest.json");

        // Create directories if they don't exist
        fs::create_dir_all(&pages_dir)
            .context("Failed to create pages directory")?;
        fs::create_dir_all(&temp_pages_dir)
            .context("Failed to create temp pages directory")?;

        // Load manifest if it exists
        let collection = if manifest_path.exists() {
            Self::load_manifest(&manifest_path)?
        } else {
            PageCollection::new()
        };

        tracing::info!(
            "Page manager initialized: {} pages, {} folders",
            collection.pages.len(),
            collection.folders.len()
        );

        Ok(Self {
            pages_dir,
            temp_pages_dir,
            manifest_path,
            collection,
        })
    }

    /// Load manifest from disk
    fn load_manifest(path: &PathBuf) -> Result<PageCollection> {
        let contents = fs::read_to_string(path)
            .context("Failed to read manifest file")?;

        let collection: PageCollection = serde_json::from_str(&contents)
            .context("Failed to parse manifest JSON")?;

        Ok(collection)
    }

    /// Save manifest to disk with atomic write
    fn save_manifest(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.collection)
            .context("Failed to serialize manifest")?;

        // Write to temp file first
        let temp_path = self.manifest_path.with_extension("json.tmp");
        fs::write(&temp_path, json)
            .context("Failed to write temp manifest file")?;

        // Atomic rename
        fs::rename(&temp_path, &self.manifest_path)
            .context("Failed to rename temp manifest file")?;

        Ok(())
    }

    /// Create a temporary page from HTML source
    ///
    /// Temporary pages are stored in temp_pages/ and not added to the manifest.
    /// They can be saved later via save_page().
    pub fn create_temp_page(&mut self, html_source: String, page_id: Option<String>) -> Result<DashPage> {
        let id = page_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let temp_file = self.temp_pages_dir.join(format!("{}.html", id));

        // Write HTML to temp file
        fs::write(&temp_file, html_source)
            .context("Failed to write temp page file")?;

        tracing::info!("Created temp page: {}", id);

        // Create page struct (not saved to manifest yet)
        Ok(DashPage {
            id: id.clone(),
            name: format!("Page {}", id),  // Default name, will be renamed on save
            description: None,
            folder_id: None,
            page_path: temp_file,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            access_count: 0,
            last_accessed: None,
        })
    }

    /// Save a temporary page to persistent storage
    ///
    /// Copies page from temp_pages/{id}.html to pages/{name}/
    /// Updates manifest with file locking to handle concurrent saves.
    pub fn save_page(
        &mut self,
        page_id: &str,
        name: String,
        description: Option<String>,
        folder_id: Option<String>,
    ) -> Result<DashPage> {
        let temp_file = self.temp_pages_dir.join(format!("{}.html", page_id));

        if !temp_file.exists() {
            anyhow::bail!("Temp page not found: {}", page_id);
        }

        // Create page directory with sanitized name
        let safe_name = Self::sanitize_name(&name);
        let page_dir = self.pages_dir.join(&safe_name);
        fs::create_dir_all(&page_dir)
            .context("Failed to create page directory")?;

        // Copy HTML to page directory as index.html
        let index_path = page_dir.join("index.html");
        fs::copy(&temp_file, &index_path)
            .context("Failed to copy page to persistent storage")?;

        // Remove temp file
        fs::remove_file(&temp_file)
            .context("Failed to remove temp file")?;

        // Create page struct
        let page = DashPage {
            id: page_id.to_string(),
            name,
            description,
            folder_id,
            page_path: page_dir,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            access_count: 0,
            last_accessed: None,
        };

        // Add to manifest with file locking
        self.add_page_to_manifest(page.clone())?;

        tracing::info!("Saved page: {} ({})", page.name, page.id);

        Ok(page)
    }

    /// Add page to manifest with file locking for concurrent access
    ///
    /// This implements the critical file locking pattern to handle multiple
    /// webview processes saving pages simultaneously:
    /// 1. Acquire exclusive lock
    /// 2. Re-read manifest from disk (get latest state)
    /// 3. Merge in new page
    /// 4. Atomic write (temp file + rename)
    /// 5. Lock auto-released on drop
    fn add_page_to_manifest(&mut self, page: DashPage) -> Result<()> {
        let lock_file_path = self.manifest_path.with_extension("lock");
        let lock_file = fs::File::create(&lock_file_path)
            .context("Failed to create lock file")?;

        // Acquire exclusive lock (blocks other webview processes)
        lock_file.lock_exclusive()
            .context("Failed to acquire manifest lock")?;

        tracing::debug!("Acquired manifest lock for page: {}", page.id);

        // Re-read manifest from disk to get latest state
        let mut current = if self.manifest_path.exists() {
            Self::load_manifest(&self.manifest_path)?
        } else {
            PageCollection::new()
        };

        // Check if page already exists (update instead of add)
        if let Some(existing) = current.get_page_mut(&page.id) {
            *existing = page;
            tracing::debug!("Updated existing page in manifest");
        } else {
            current.add_page(page);
            tracing::debug!("Added new page to manifest");
        }

        // Atomic write
        let json = serde_json::to_string_pretty(&current)
            .context("Failed to serialize manifest")?;

        let temp_path = self.manifest_path.with_extension("json.tmp");
        fs::write(&temp_path, json)
            .context("Failed to write temp manifest file")?;

        fs::rename(&temp_path, &self.manifest_path)
            .context("Failed to rename temp manifest file")?;

        // Update our in-memory copy
        self.collection = current;

        tracing::debug!("Released manifest lock");

        // Lock is auto-released when lock_file goes out of scope
        Ok(())
    }

    /// Delete a page from persistent storage
    pub fn delete_page(&mut self, page_id: &str) -> Result<()> {
        let page = self.collection.remove_page(page_id)
            .context("Page not found")?;

        // Remove page directory
        if page.page_path.exists() {
            fs::remove_dir_all(&page.page_path)
                .context("Failed to remove page directory")?;
        }

        // Save updated manifest
        self.save_manifest()?;

        tracing::info!("Deleted page: {} ({})", page.name, page.id);

        Ok(())
    }

    /// Get a reference to the page collection
    pub fn collection(&self) -> &PageCollection {
        &self.collection
    }

    /// Get a mutable reference to the page collection
    pub fn collection_mut(&mut self) -> &mut PageCollection {
        &mut self.collection
    }

    /// Sanitize a name for use as a directory name
    fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect::<String>()
            .to_lowercase()
    }
}

/// Global page manager singleton
static GLOBAL_PAGE_MANAGER: OnceLock<StdRwLock<PageManager>> = OnceLock::new();

/// Get or initialize the global page manager
pub fn get_page_manager() -> &'static StdRwLock<PageManager> {
    GLOBAL_PAGE_MANAGER.get_or_init(|| {
        let manager = PageManager::new()
            .expect("Failed to initialize page manager");
        StdRwLock::new(manager)
    })
}
