# Download Manager

Asynchronous CloudFormation resource specification download system providing background downloads with real-time progress tracking and multi-region support for AWS resource definitions.

## Core Functionality

**Download Management:**
- Asynchronous background downloads using spawned threads with mpsc channels
- Real-time progress reporting with phase-based status updates
- Multi-region support for downloading AWS CloudFormation specifications across 30+ regions
- Automatic retry capabilities with user-initiated retry options
- Graceful error handling with detailed error reporting

**Download Phases:**
- **Specification**: Downloads main CloudFormation resource specification file
- **ResourceTypes**: Downloads individual resource type specification files  
- **Schemas**: Downloads JSON schema files for validation
- **Complete**: All download phases finished successfully

**Key Features:**
- Non-blocking UI updates using `try_recv()` for status polling
- Manual download initiation (no longer auto-starts at application startup)
- Region-specific downloads for targeted resource specification updates
- Visual progress indicators with spinner and phase descriptions
- Error recovery with retry functionality

**Main Components:**
- **DownloadManager**: UI coordinator managing download state and progress display
- **CfnResourcesDownloader**: Core download engine handling file operations and AWS API calls
- **DownloadStatus**: Progress tracking structure with region counters and phase information
- **ProgressDisplay**: UI components for showing download progress and errors

**Integration Points:**
- CloudFormation resource system for resource specification storage
- Application data directory for file caching (`~/.config/awsdash/cfn-resources/`)
- egui UI system for progress display and user interaction
- Background thread system for non-blocking downloads

## Implementation Details

**Key Files:**
- `src/app/dashui/download_manager.rs` - UI management and progress display
- `src/app/cfn_resources.rs` - Core download engine and specification management

**Download Architecture:**
```rust
// Background thread with channel communication
let (tx, rx) = mpsc::channel();
thread::spawn(move || {
    // Download with progress callbacks
    downloader.download_region_with_status(region, false, |phase| {
        tx.send(DownloadStatus { phase, ... }).unwrap_or_default();
    });
});
```

**Status Structure:**
```rust
pub struct DownloadStatus {
    pub region: String,
    pub total_regions: usize,
    pub current_region: usize,
    pub completed: bool,
    pub error: Option<String>,
    pub phase: DownloadPhase,
}
```

**File Organization:**
- Region-specific directories: `~/.config/awsdash/cfn-resources/{region}/`
- Main specification: `CloudFormationResourceSpecification.json`
- Individual resources: `resources/*.json`
- Validation schemas: `schemas/*.json`

**Progress Display Logic:**
- Visual spinner during active downloads using `egui::Spinner`
- Phase-specific messages for better user understanding
- Region counter display (`Region X of Y`)
- Error display with retry buttons
- Manual download buttons when no download is active

## Developer Notes

**Extension Points for New Download Types:**

1. **Add New DownloadPhase**:
   ```rust
   #[derive(Clone, PartialEq)]
   pub enum DownloadPhase {
       Specification, ResourceTypes, Schemas, Complete,
       NewPhase, // Add new phase here
   }
   ```

2. **Implement Phase-Specific Downloads**:
   ```rust
   // In CfnResourcesDownloader::download_region_internal()
   status_callback(DownloadPhase::NewPhase);
   // Implement new download logic
   ```

3. **Update Progress Display**:
   ```rust
   // In DownloadManager::show_download_progress()
   DownloadPhase::NewPhase => "New phase description"
   ```

**Integration Pattern for New Download Sources:**
- Follow async pattern with mpsc channels for progress reporting
- Use `try_recv()` for non-blocking status updates in UI thread
- Implement proper error handling with user-friendly messages
- Provide retry mechanisms for failed downloads

**Background Thread Management:**
```rust
// Start download
let receiver = CfnResourcesDownloader::download_regions_async(regions);
self.download_receiver = Some(receiver);

// Poll for updates (non-blocking)
if let Ok(status) = receiver.try_recv() {
    self.download_status = Some(status);
}
```

**Architectural Decisions:**
- **No Auto-Start**: Downloads no longer start automatically to reduce startup time
- **Manual Control**: Users explicitly initiate downloads when needed  
- **Thread-Safe Communication**: Uses mpsc channels for safe cross-thread communication
- **Non-Blocking UI**: UI remains responsive during downloads using `try_recv()`
- **Graceful Degradation**: Application functions without downloaded specifications

**Performance Considerations:**
- Downloads run in background threads to avoid blocking UI
- Progress updates throttled to avoid overwhelming UI with status messages
- Large download operations (all regions) can take several minutes
- Individual region downloads typically complete in 30-60 seconds
- File caching reduces redundant downloads (7-day freshness check)

**Error Handling Strategy:**
- Network failures display user-friendly error messages
- Retry functionality available for failed downloads
- Graceful handling of interrupted downloads
- Fallback URLs attempted for AWS specification files

**References:**
- [CloudFormation System](cloudformation-system.md) - Resource specification usage
- [User Interface](user-interface.md) - Progress display integration
- [Performance Optimization](performance-optimization.md) - Background processing patterns