//! Bookmark System Unit Tests
//!
//! Comprehensive tests for the bookmark management system, covering bookmarks, folders,
//! and hierarchical organization without filesystem dependencies.
//!
//! # Test Coverage
//!
//! - **Bookmark Creation**: Creating bookmarks from Explorer state
//! - **Bookmark State Management**: Applying bookmarks and matching state
//! - **Folder Organization**: Creating and nesting folders
//! - **Bookmark Collection**: Add, remove, reorder operations
//! - **Folder Hierarchy**: Parent-child relationships
//! - **Access Tracking**: Usage counting and last accessed timestamps

use awsdash::app::resource_explorer::bookmarks::{Bookmark, BookmarkCollection, BookmarkFolder};
use awsdash::app::resource_explorer::state::{
    AccountSelection, GroupingMode, RegionSelection, ResourceExplorerState, ResourceTypeSelection,
    TagFilter, TagFilterGroup, TagFilterType,
};

// ============================================================================
// Bookmark Creation and State Management Tests
// ============================================================================

#[test]
fn test_bookmark_captures_explorer_state() {
    let mut state = ResourceExplorerState::default();

    state.add_account(AccountSelection::new(
        "123456789012".to_string(),
        "Production".to_string(),
    ));
    state.add_region(RegionSelection::new(
        "us-east-1".to_string(),
        "US East 1".to_string(),
    ));
    state.add_resource_type(ResourceTypeSelection::new(
        "AWS::EC2::Instance".to_string(),
        "EC2 Instances".to_string(),
        "EC2".to_string(),
    ));
    state.primary_grouping = GroupingMode::ByTag("Environment".to_string());
    state.search_filter = "production".to_string();

    let bookmark = Bookmark::new("My Bookmark".to_string(), &state);

    assert_eq!(bookmark.name, "My Bookmark");
    assert_eq!(bookmark.account_ids, vec!["123456789012".to_string()]);
    assert_eq!(bookmark.region_codes, vec!["us-east-1".to_string()]);
    assert_eq!(
        bookmark.resource_type_ids,
        vec!["AWS::EC2::Instance".to_string()]
    );
    assert_eq!(
        bookmark.grouping,
        GroupingMode::ByTag("Environment".to_string())
    );
    assert_eq!(bookmark.search_filter, "production");
    assert_eq!(bookmark.access_count, 0);
    assert!(bookmark.last_accessed.is_none());
}

#[test]
fn test_bookmark_apply_to_state_updates_grouping() {
    let mut initial_state = ResourceExplorerState::default();
    initial_state.primary_grouping = GroupingMode::ByTag("Team".to_string());
    initial_state.search_filter = "backend".to_string();

    let mut bookmark = Bookmark::new("Test Bookmark".to_string(), &initial_state);

    let mut new_state = ResourceExplorerState::default();
    new_state.primary_grouping = GroupingMode::ByAccount;
    new_state.search_filter = String::new();

    bookmark.apply_to_state(&mut new_state);

    assert_eq!(
        new_state.primary_grouping,
        GroupingMode::ByTag("Team".to_string())
    );
    assert_eq!(new_state.search_filter, "backend");
    assert_eq!(bookmark.access_count, 1);
    assert!(bookmark.last_accessed.is_some());
}

#[test]
fn test_bookmark_matches_state() {
    let mut state = ResourceExplorerState::default();
    state.add_account(AccountSelection::new(
        "123456789012".to_string(),
        "Production".to_string(),
    ));
    state.primary_grouping = GroupingMode::ByRegion;
    state.search_filter = "test".to_string();

    let bookmark = Bookmark::new("Test".to_string(), &state);

    assert!(bookmark.matches_state(&state));

    let mut different_state = ResourceExplorerState::default();
    different_state.primary_grouping = GroupingMode::ByAccount;
    assert!(!bookmark.matches_state(&different_state));
}

#[test]
fn test_bookmark_access_count_increments() {
    let state = ResourceExplorerState::default();
    let mut bookmark = Bookmark::new("Test".to_string(), &state);

    assert_eq!(bookmark.access_count, 0);

    let mut new_state = ResourceExplorerState::default();
    bookmark.apply_to_state(&mut new_state);
    assert_eq!(bookmark.access_count, 1);

    bookmark.apply_to_state(&mut new_state);
    assert_eq!(bookmark.access_count, 2);
}

// ============================================================================
// Bookmark Folder Tests
// ============================================================================

#[test]
fn test_folder_creation_top_level() {
    let folder = BookmarkFolder::new("Production".to_string(), None);

    assert_eq!(folder.name, "Production");
    assert!(folder.parent_id.is_none());
    assert!(folder.description.is_none());
    assert!(!folder.id.is_empty());
}

#[test]
fn test_folder_creation_nested() {
    let parent_folder = BookmarkFolder::new("Production".to_string(), None);
    let child_folder = BookmarkFolder::new("Backend".to_string(), Some(parent_folder.id.clone()));

    assert_eq!(child_folder.name, "Backend");
    assert_eq!(child_folder.parent_id, Some(parent_folder.id.clone()));
}

#[test]
fn test_folder_has_unique_ids() {
    let folder1 = BookmarkFolder::new("Folder 1".to_string(), None);
    let folder2 = BookmarkFolder::new("Folder 2".to_string(), None);

    assert_ne!(folder1.id, folder2.id);
}

// ============================================================================
// BookmarkCollection Tests
// ============================================================================

#[test]
fn test_bookmark_collection_add_and_get() {
    let mut collection = BookmarkCollection::new();
    let state = ResourceExplorerState::default();
    let bookmark = Bookmark::new("Test Bookmark".to_string(), &state);
    let id = bookmark.id.clone();

    collection.add(bookmark);

    let retrieved = collection.get(&id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Test Bookmark");
}

#[test]
fn test_bookmark_collection_remove() {
    let mut collection = BookmarkCollection::new();
    let state = ResourceExplorerState::default();
    let bookmark = Bookmark::new("Test Bookmark".to_string(), &state);
    let id = bookmark.id.clone();

    collection.add(bookmark);
    assert!(collection.get(&id).is_some());

    let removed = collection.remove(&id);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "Test Bookmark");
    assert!(collection.get(&id).is_none());
}

#[test]
fn test_bookmark_collection_reorder() {
    let mut collection = BookmarkCollection::new();
    let state = ResourceExplorerState::default();

    let b1 = Bookmark::new("Bookmark 1".to_string(), &state);
    let b2 = Bookmark::new("Bookmark 2".to_string(), &state);
    let b3 = Bookmark::new("Bookmark 3".to_string(), &state);

    let id1 = b1.id.clone();
    let id3 = b3.id.clone();

    collection.add(b1);
    collection.add(b2);
    collection.add(b3);

    // Initial order: 1, 2, 3
    assert_eq!(collection.bookmarks[0].id, id1);
    assert_eq!(collection.bookmarks[2].id, id3);

    // Reorder: move index 0 to index 2
    collection.reorder(0, 2);

    // New order: 2, 3, 1
    assert_eq!(collection.bookmarks[2].id, id1);
}

#[test]
fn test_bookmark_collection_with_folders() {
    let mut collection = BookmarkCollection::new();
    let state = ResourceExplorerState::default();

    let folder = BookmarkFolder::new("Production".to_string(), None);
    let folder_id = folder.id.clone();
    collection.folders.push(folder);

    let mut bookmark = Bookmark::new("My Bookmark".to_string(), &state);
    bookmark.folder_id = Some(folder_id.clone());
    collection.add(bookmark);

    // Verify bookmark is in folder
    let in_folder: Vec<_> = collection
        .bookmarks
        .iter()
        .filter(|b| b.folder_id.as_ref() == Some(&folder_id))
        .collect();
    assert_eq!(in_folder.len(), 1);
}

#[test]
fn test_bookmark_collection_folder_hierarchy() {
    let mut collection = BookmarkCollection::new();

    // Create: Root > Production > Backend
    let prod = BookmarkFolder::new("Production".to_string(), None);
    let backend = BookmarkFolder::new("Backend".to_string(), Some(prod.id.clone()));

    let prod_id = prod.id.clone();
    let backend_id = backend.id.clone();

    collection.folders.push(prod);
    collection.folders.push(backend);

    // Get top-level folders
    let top_level: Vec<_> = collection
        .folders
        .iter()
        .filter(|f| f.parent_id.is_none())
        .collect();
    assert_eq!(top_level.len(), 1);
    assert_eq!(top_level[0].name, "Production");

    // Get subfolders of Production
    let subfolders: Vec<_> = collection
        .folders
        .iter()
        .filter(|f| f.parent_id.as_ref() == Some(&prod_id))
        .collect();
    assert_eq!(subfolders.len(), 1);
    assert_eq!(subfolders[0].id, backend_id);
}

// ============================================================================
// Complex Bookmark State Tests
// ============================================================================

#[test]
fn test_bookmark_with_tag_filters() {
    let mut state = ResourceExplorerState::default();

    let mut group = TagFilterGroup::new();
    group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    group.add_filter(TagFilter::new("Team".to_string(), TagFilterType::Exists));

    state.tag_filter_group = group.clone();

    let bookmark = Bookmark::new("Filtered Resources".to_string(), &state);

    assert_eq!(bookmark.tag_filters, group);
}

#[test]
fn test_bookmark_with_tag_hierarchy_grouping() {
    let mut state = ResourceExplorerState::default();
    state.primary_grouping = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
        "Project".to_string(),
    ]);

    let bookmark = Bookmark::new("Hierarchical View".to_string(), &state);

    assert_eq!(
        bookmark.grouping,
        GroupingMode::ByTagHierarchy(vec![
            "Environment".to_string(),
            "Team".to_string(),
            "Project".to_string()
        ])
    );
}

#[test]
fn test_bookmark_empty_state() {
    let state = ResourceExplorerState::default();
    let bookmark = Bookmark::new("Empty State".to_string(), &state);

    assert_eq!(bookmark.account_ids.len(), 0);
    assert_eq!(bookmark.region_codes.len(), 0);
    assert_eq!(bookmark.resource_type_ids.len(), 0);
    assert_eq!(bookmark.grouping, GroupingMode::ByAccount);
    assert_eq!(bookmark.search_filter, "");
}

// ============================================================================
// Real-World Scenarios
// ============================================================================

#[test]
fn test_multi_folder_organization() {
    let mut collection = BookmarkCollection::new();
    let state = ResourceExplorerState::default();

    // Create folder structure:
    // Production
    // ├── Backend
    // └── Frontend
    // Staging

    let prod = BookmarkFolder::new("Production".to_string(), None);
    let staging = BookmarkFolder::new("Staging".to_string(), None);
    let backend = BookmarkFolder::new("Backend".to_string(), Some(prod.id.clone()));
    let frontend = BookmarkFolder::new("Frontend".to_string(), Some(prod.id.clone()));

    let prod_id = prod.id.clone();
    let backend_id = backend.id.clone();

    collection.folders.push(prod);
    collection.folders.push(staging);
    collection.folders.push(backend);
    collection.folders.push(frontend);

    // Add bookmarks
    let mut b1 = Bookmark::new("API Instances".to_string(), &state);
    let mut b2 = Bookmark::new("Database Instances".to_string(), &state);
    let b3 = Bookmark::new("Staging Resources".to_string(), &state);

    b1.folder_id = Some(backend_id.clone());
    b2.folder_id = Some(backend_id.clone());
    // b3 stays in Top Folder

    collection.add(b1);
    collection.add(b2);
    collection.add(b3);

    // Verify organization
    let in_backend: Vec<_> = collection
        .bookmarks
        .iter()
        .filter(|b| b.folder_id.as_ref() == Some(&backend_id))
        .collect();
    assert_eq!(in_backend.len(), 2);

    let in_top: Vec<_> = collection
        .bookmarks
        .iter()
        .filter(|b| b.folder_id.is_none())
        .collect();
    assert_eq!(in_top.len(), 1);

    // Get Production's children
    let prod_children: Vec<_> = collection
        .folders
        .iter()
        .filter(|f| f.parent_id.as_ref() == Some(&prod_id))
        .collect();
    assert_eq!(prod_children.len(), 2);
}

#[test]
fn test_bookmark_serialization_roundtrip() {
    let mut state = ResourceExplorerState::default();
    state.add_account(AccountSelection::new(
        "123456789012".to_string(),
        "Production".to_string(),
    ));
    state.search_filter = "test-search".to_string();

    let bookmark = Bookmark::new("Test".to_string(), &state);

    // Serialize
    let json = serde_json::to_string(&bookmark).unwrap();

    // Deserialize
    let restored: Bookmark = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.name, bookmark.name);
    assert_eq!(restored.account_ids, bookmark.account_ids);
    assert_eq!(restored.search_filter, bookmark.search_filter);
}

#[test]
fn test_bookmark_collection_serialization() {
    let mut collection = BookmarkCollection::new();
    let state = ResourceExplorerState::default();

    collection.add(Bookmark::new("Bookmark 1".to_string(), &state));
    collection.add(Bookmark::new("Bookmark 2".to_string(), &state));
    collection
        .folders
        .push(BookmarkFolder::new("Folder 1".to_string(), None));

    // Serialize
    let json = serde_json::to_string(&collection).unwrap();

    // Deserialize
    let restored: BookmarkCollection = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.bookmarks.len(), 2);
    assert_eq!(restored.folders.len(), 1);
    assert_eq!(restored.version, 1);
}
