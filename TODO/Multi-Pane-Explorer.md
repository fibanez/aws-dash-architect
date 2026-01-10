# Multi-Pane Explorer Implementation Plan

## Overview

Transform the Resource Explorer from a single monolithic window to a multi-instance, multi-pane system where:
- Each menu click opens a NEW Explorer window instance
- Each window starts with a single pane (left), with a button to toggle the right pane
- **NO TABS** - architecture is ExplorerManager → ExplorerInstance → ExplorerPane (direct)
- All instances share the same cache and bookmarks via modular query engine
- Each pane maintains independent state (filters, searches, selections, grouping)
- Query engine is decoupled from UI, enabling use by Agents and future WebView

## Current State

### What's Already Built ✅ (User Completed)

**Multi-Pane UI Structure**:
- ✅ `ExplorerInstance` - **tabs already removed**, direct Window → Panes (instance.rs:17-34)
- ✅ `FocusableWindow` trait - **already implemented** (instance.rs:159-218)
- ✅ `ExplorerManager` - manages multiple window instances, no auto-creation (manager.rs:96-103)
- ✅ `ExplorerPane` - independent state wrapper per pane (pane.rs)
- ✅ `PaneRenderer` - extracted rendering logic with unique widget IDs (pane_renderer.rs)
- ✅ Shared cache infrastructure (`SharedResourceCache` with zstd compression)
- ✅ Shared bookmark manager (Arc-wrapped for V8 integration)

**DashApp Integration**:
- ✅ `DashApp` uses `ExplorerManager` (mod.rs:117)
- ✅ `handle_explorer_windows()` method implemented (window_rendering.rs:232-291)
- ✅ Called from main render loop (mod.rs:366)
- ✅ Menu handler creates new windows (rendering.rs:75-85)
- ✅ AWS client set on login/cleared on logout (window_rendering.rs:110, 161)

**Pane Query Triggering**:
- ✅ `trigger_query_if_ready()` exists in pane.rs (pane.rs:259-328)
- ⚠️  Currently calls `window.rs::spawn_parallel_query` (line 321) - **needs adapter change**

### What Needs to Be Built ❌

- ❌ **M1: Modular Query Engine** - Extract `spawn_parallel_query` from window.rs
- ❌ **M2: UI Adapter** - Connect query engine to pane state updates
- ❌ **M3: Query Engine Integration** - Add `query_engine` field to `ExplorerSharedContext`
- ❌ **M4: Cleanup** - Delete `tab.rs`, remove from `mod.rs` exports
- ❌ **M5: Wire Adapter** - Change pane.rs:321 from window.rs call to adapter call
- ❌ **M7: Agent Adapter** - Enable agent queries with JSON-friendly progress

## User Requirements (Clarified)

1. **Initial State**: Single pane (left) at startup, user adds right pane via "Show Split" button
2. **Menu Behavior**: Each "AWS Explorer" menu click creates a NEW window (doesn't toggle existing)
3. **Focus System**: YES - implement `FocusableWindow` trait for consistent behavior
4. **Persistence**: NO - always start fresh (no saving pane/tab state across restarts)

---

## Implementation Strategy

**Approach**: Build modular query architecture FIRST, then build multi-pane UI on top of it from the start. This avoids building it twice and ensures proper decoupling.

**Testing**: Each milestone is independently testable via human integration tests.

**Key Change from Original Plan**: We're NOT building tabs - user wants Window → Panes only (no tab layer).

---

## Implementation Plan (7 Milestones)

### Milestone 1: Build Modular Query Engine (Core)

**Goal**: Extract query execution logic into standalone, UI-independent query engine with callback-based progress system.

**Human Test**: Run query engine unit tests; verify callbacks fire in correct order with correct data.

**Files**:
- **New**: `src/app/resource_explorer/query_engine.rs`
- **Reference**: `window.rs:3476-3700` (spawn_parallel_query)

#### Tasks

##### M1.T1: Create Query Progress Types

Create progress event enum:
```rust
/// Progress updates during query execution
#[derive(Clone, Debug)]
pub enum QueryProgress {
    Phase1Started { total_queries: usize },
    Phase1QueryCompleted {
        query_key: String,
        resource_count: usize,
        pending: usize,
        total: usize,
    },
    Phase1QueryFailed {
        query_key: String,
        error: String,
        pending: usize,
        total: usize,
    },
    Phase1Completed {
        total_resources: usize,
        failed_queries: Vec<String>,
    },
    Completed {
        resources: Vec<ResourceEntry>,
        failed_queries: Vec<String>,
    },
    Failed { error: String },
}

pub type ProgressCallback = Arc<dyn Fn(QueryProgress) + Send + Sync>;
```

**Reference**: Based on state updates in `window.rs:3615-3680`

##### M1.T2: Create ResourceQueryEngine Struct

```rust
pub struct ResourceQueryEngine {
    aws_client: Arc<AWSResourceClient>,
    cache: Arc<SharedResourceCache>,
}

impl ResourceQueryEngine {
    pub fn new(
        aws_client: Arc<AWSResourceClient>,
        cache: Arc<SharedResourceCache>,
    ) -> Self {
        Self { aws_client, cache }
    }

    /// Execute a query with progress callbacks
    /// Returns a handle for cancellation/status checks
    pub fn execute_query(
        &self,
        scope: QueryScope,
        progress_callback: Option<ProgressCallback>,
    ) -> QueryHandle {
        // Implementation in M1.T3
    }
}
```

##### M1.T3: Extract Query Key Building Logic

**Reference**: `window.rs:3483-3514`

```rust
fn build_query_keys(&self, scope: &QueryScope) -> Vec<String> {
    let global_registry = super::GlobalServiceRegistry::new();
    let mut queries_to_track = Vec::new();

    for account in &scope.accounts {
        for resource_type in &scope.resource_types {
            if global_registry.is_global(&resource_type.resource_type) {
                // Global service: one query per account
                let query_key = ResourceExplorerState::make_query_key(
                    &account.account_id,
                    "Global",
                    &resource_type.resource_type,
                );
                if !queries_to_track.contains(&query_key) {
                    queries_to_track.push(query_key);
                }
            } else {
                // Regional service: one query per account × region
                for region in &scope.regions {
                    let query_key = ResourceExplorerState::make_query_key(
                        &account.account_id,
                        &region.region_code,
                        &resource_type.resource_type,
                    );
                    queries_to_track.push(query_key);
                }
            }
        }
    }

    queries_to_track
}
```

##### M1.T4: Implement Query Execution with Callbacks

**Reference**: `window.rs:3536-3700`

```rust
pub fn execute_query(
    &self,
    scope: QueryScope,
    progress_callback: Option<ProgressCallback>,
) -> QueryHandle {
    let query_id = format!("query_{}", Uuid::new_v4());
    let queries_to_track = self.build_query_keys(&scope);

    // Fire Phase1Started callback
    if let Some(ref callback) = progress_callback {
        callback(QueryProgress::Phase1Started {
            total_queries: queries_to_track.len(),
        });
    }

    let aws_client = self.aws_client.clone();
    let cache = self.cache.clone();
    let callback_clone = progress_callback.clone();
    let queries_clone = queries_to_track.clone();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async {
            let (result_sender, mut result_receiver) =
                tokio::sync::mpsc::channel(1000);

            // Launch parallel query
            let query_future = aws_client.query_aws_resources_parallel(
                &scope,
                result_sender,
                None,
                cache.clone(),
            );

            let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
            let all_resources_clone = all_resources.clone();
            let queries_tracker = Arc::new(tokio::sync::Mutex::new(queries_clone));
            let failed_queries = Arc::new(tokio::sync::Mutex::new(Vec::new()));

            // Process results and fire callbacks
            let result_processing = async {
                while let Some(result) = result_receiver.recv().await {
                    let query_key = ResourceExplorerState::make_query_key(
                        &result.account_id,
                        &result.region,
                        &result.resource_type,
                    );

                    match result.resources {
                        Ok(resources) => {
                            let resource_count = resources.len();
                            let mut all_res = all_resources_clone.lock().await;
                            all_res.extend(resources);

                            let mut tracker = queries_tracker.lock().await;
                            tracker.retain(|q| q != &query_key);
                            let pending = tracker.len();
                            drop(tracker);

                            // Fire Phase1QueryCompleted callback
                            if let Some(ref cb) = callback_clone {
                                cb(QueryProgress::Phase1QueryCompleted {
                                    query_key: query_key.clone(),
                                    resource_count,
                                    pending,
                                    total: queries_to_track.len(),
                                });
                            }
                        }
                        Err(e) => {
                            let mut tracker = queries_tracker.lock().await;
                            tracker.retain(|q| q != &query_key);
                            let pending = tracker.len();
                            drop(tracker);

                            let mut failed = failed_queries.lock().await;
                            failed.push(query_key.clone());
                            drop(failed);

                            // Fire Phase1QueryFailed callback
                            if let Some(ref cb) = callback_clone {
                                cb(QueryProgress::Phase1QueryFailed {
                                    query_key: query_key.clone(),
                                    error: e.to_string(),
                                    pending,
                                    total: queries_to_track.len(),
                                });
                            }
                        }
                    }
                }

                // Fire Phase1Completed callback
                let final_resources = all_resources_clone.lock().await.clone();
                let final_failed = failed_queries.lock().await.clone();

                if let Some(ref cb) = callback_clone {
                    cb(QueryProgress::Phase1Completed {
                        total_resources: final_resources.len(),
                        failed_queries: final_failed.clone(),
                    });
                }

                Ok::<_, anyhow::Error>((final_resources, final_failed))
            };

            // Wait for completion
            let ((_query_result, processing_result)) = tokio::join!(
                query_future,
                result_processing,
            );

            match processing_result {
                Ok((resources, failed)) => {
                    if let Some(ref cb) = callback_clone {
                        cb(QueryProgress::Completed {
                            resources,
                            failed_queries: failed,
                        });
                    }
                }
                Err(e) => {
                    if let Some(ref cb) = callback_clone {
                        cb(QueryProgress::Failed {
                            error: e.to_string(),
                        });
                    }
                }
            }
        });
    });

    QueryHandle { query_id }
}
```

##### M1.T5: Create QueryHandle for Cancellation

```rust
pub struct QueryHandle {
    query_id: String,
    // Future: Add cancellation token
}

impl QueryHandle {
    pub fn query_id(&self) -> &str {
        &self.query_id
    }
}
```

##### M1.T6: Write Unit Tests

**File**: `tests/query_engine_tests.rs`

Test cases:
- Query execution fires callbacks in correct order
- Failed queries trigger Phase1QueryFailed callbacks
- Final resources match input
- Query keys built correctly (global vs regional services)

---

### Milestone 2: Build UI Adapter for Query Engine

**Goal**: Create adapter that connects query engine to pane state updates and UI repaints, maintaining exact original behavior.

**Human Test**: Create test harness that mocks ResourceExplorerState and egui::Context; verify adapter updates state correctly.

**Files**:
- **New**: `src/app/resource_explorer/ui_query_adapter.rs`
- **Reference**: `window.rs:3434-3474` (trigger_query_if_ready)
- **Reference**: `window.rs:3615-3680` (state update retry logic)

#### Tasks

##### M2.T1: Create UIQueryAdapter Struct

```rust
pub struct UIQueryAdapter {
    query_engine: Arc<ResourceQueryEngine>,
}

impl UIQueryAdapter {
    pub fn new(query_engine: Arc<ResourceQueryEngine>) -> Self {
        Self { query_engine }
    }

    /// Execute a query with UI-specific callbacks
    ///
    /// Based on:
    /// - Query triggering: window.rs:3434-3474
    /// - State updates: window.rs:3615-3680
    /// - UI repaints: window.rs:3470
    pub fn execute_for_pane(
        &self,
        state: Arc<RwLock<ResourceExplorerState>>,
        scope: QueryScope,
        ctx: egui::Context,
    ) -> QueryHandle {
        // Implementation in M2.T2
    }
}
```

##### M2.T2: Implement State Update Logic with Retry

**Reference**: `window.rs:3615-3680` (10-retry loop for lock contention)

```rust
pub fn execute_for_pane(
    &self,
    state: Arc<RwLock<ResourceExplorerState>>,
    scope: QueryScope,
    ctx: egui::Context,
) -> QueryHandle {
    // Mark state as loading (window.rs:3459-3467)
    let cache_key = if let Ok(mut loading_state) = state.try_write() {
        loading_state.start_loading_task("parallel_query")
    } else {
        format!(
            "parallel_query_fallback_{}",
            chrono::Utc::now().timestamp_millis()
        )
    };

    // Request UI repaint (window.rs:3470)
    ctx.request_repaint_after(std::time::Duration::from_millis(50));

    let state_clone = state.clone();
    let ctx_clone = ctx.clone();
    let cache_key_clone = cache_key.clone();

    // Create progress callback that updates state
    let progress_callback: ProgressCallback = Arc::new(move |progress| {
        match progress {
            QueryProgress::Phase1Started { total_queries } => {
                // Initialize Phase 1 tracking (window.rs:3515-3531)
                for attempt in 0..10 {
                    if let Ok(mut state) = state_clone.try_write() {
                        // Note: queries_to_track populated by query engine internally
                        // State tracking initialized separately
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                ctx_clone.request_repaint();
            }

            QueryProgress::Phase1QueryCompleted {
                query_key,
                resource_count,
                ..
            } => {
                // Update state with retry logic (window.rs:3615-3643)
                for attempt in 0..10 {
                    if let Ok(mut state) = state_clone.try_write() {
                        state.mark_phase1_query_completed(&query_key);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                ctx_clone.request_repaint();
            }

            QueryProgress::Phase1QueryFailed { query_key, .. } => {
                // Mark failed with retry logic (window.rs:3652-3678)
                for attempt in 0..10 {
                    if let Ok(mut state) = state_clone.try_write() {
                        state.mark_phase1_query_failed(&query_key);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                ctx_clone.request_repaint();
            }

            QueryProgress::Completed { resources, .. } => {
                // Update final resources
                if let Ok(mut state) = state_clone.try_write() {
                    state.resources = resources;
                    state.loading_tasks.remove(&cache_key_clone);
                }
                ctx_clone.request_repaint();
            }

            QueryProgress::Failed { error } => {
                if let Ok(mut state) = state_clone.try_write() {
                    state.loading_tasks.remove(&cache_key_clone);
                    state.reset_phase1_state();
                }
                ctx_clone.request_repaint();
            }

            _ => {
                ctx_clone.request_repaint();
            }
        }
    });

    self.query_engine.execute_query(scope, Some(progress_callback))
}
```

##### M2.T3: Add Phase 1 Tracking Initialization

**Problem**: Query engine doesn't know about state.phase1_pending_queries tracking.

**Solution**: Adapter must initialize Phase 1 tracking separately:

```rust
// In execute_for_pane, before calling query_engine.execute_query:
let queries_to_track = build_query_keys_for_scope(&scope);  // Helper function

for attempt in 0..10 {
    if let Ok(mut state) = state.try_write() {
        state.start_phase1_tracking(queries_to_track.clone());
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(5));
}
```

##### M2.T4: Write Integration Tests

**File**: `tests/ui_query_adapter_tests.rs`

Test cases:
- State updates correctly during query execution
- UI repaints requested at correct times
- Lock contention retry logic works
- Failed queries tracked in state
- Multiple simultaneous queries don't conflict

---

### Milestone 3: Integrate Query Engine with Shared Context

**Goal**: Add query engine to ExplorerSharedContext; update when AWS client changes.

**Human Test**: Login to AWS, verify query engine created with correct AWS client; change credentials, verify engine updates.

**Files**:
- **Modified**: `src/app/resource_explorer/instances/manager.rs`

#### Tasks

##### M3.T1: Add Query Engine to ExplorerSharedContext

```rust
pub struct ExplorerSharedContext {
    pub cache: Arc<SharedResourceCache>,
    pub bookmarks: Arc<StdRwLock<BookmarkManager>>,
    pub aws_identity_center: Option<Arc<StdRwLock<AwsIdentityCenter>>>,
    pub aws_client: Option<Arc<AWSResourceClient>>,

    // NEW: Query engine for executing resource queries
    pub query_engine: Arc<ResourceQueryEngine>,
}

impl ExplorerSharedContext {
    pub fn new() -> Self {
        let cache = super::cache::shared_cache();

        // Create placeholder query engine (will be updated when AWS client is set)
        let placeholder_client = Arc::new(AWSResourceClient::new(/* placeholder */));
        let query_engine = Arc::new(ResourceQueryEngine::new(
            placeholder_client,
            cache.clone(),
        ));

        Self {
            cache,
            bookmarks: Arc::new(StdRwLock::new(BookmarkManager::new())),
            aws_identity_center: None,
            aws_client: None,
            query_engine,
        }
    }

    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client.clone();

        // Update query engine with new AWS client
        if let Some(client) = aws_client {
            self.query_engine = Arc::new(ResourceQueryEngine::new(
                client,
                self.cache.clone(),
            ));
            tracing::info!("Query engine updated with new AWS client");
        }
    }
}
```

##### M3.T2: Update ExplorerManager to Use set_aws_client

```rust
impl ExplorerManager {
    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.shared_context.set_aws_client(aws_client);
        // All existing panes will get the new query engine via shared_context reference
    }
}
```

---

### Milestone 4: Cleanup Tabs (Already Mostly Done ✅)

**Status**: ✅ **95% COMPLETE** - User already removed tabs from instance.rs and implemented everything. Just need cleanup.

**Goal**: Delete `tab.rs` file and clean up exports. All functionality already working.

**Human Test**: Verify app still compiles and runs after cleanup.

**Files**:
- **Delete**: `src/app/resource_explorer/instances/tab.rs`
- **Modified**: `src/app/resource_explorer/instances/mod.rs` (remove tab exports)

#### Tasks

##### M4.T1: Delete tab.rs File

```bash
rm src/app/resource_explorer/instances/tab.rs
```

##### M4.T2: Remove Tab Exports from mod.rs

**Current** (mod.rs:14, 20):
```rust
pub mod tab;
pub use tab::ExplorerTab;
```

**New**: Remove these lines entirely.

---

### Milestone 5 (OLD - SKIP THIS)

**Current Structure**: ExplorerManager → Instance → Tab → Pane
**New Structure**: ExplorerManager → Instance → Pane (direct)

```rust
pub struct ExplorerInstance {
    pub id: Uuid,

    // Direct panes (no tabs)
    pub left_pane: ExplorerPane,
    pub right_pane: Option<ExplorerPane>,
    pub show_right_pane: bool,

    pub is_open: bool,
    pub title: String,
    instance_number: usize,
    window_id_static: Option<&'static str>,
}

impl ExplorerInstance {
    pub fn new(instance_number: usize) -> Self {
        let title = if instance_number == 1 {
            "Explorer".to_string()
        } else {
            format!("Explorer {}", instance_number)
        };

        Self {
            id: Uuid::new_v4(),
            left_pane: ExplorerPane::new(),
            right_pane: None,
            show_right_pane: false,
            is_open: true,
            title,
            instance_number,
            window_id_static: None,
        }
    }

    pub fn toggle_right_pane(&mut self) {
        self.show_right_pane = !self.show_right_pane;
        if self.show_right_pane && self.right_pane.is_none() {
            self.right_pane = Some(ExplorerPane::new());
        }
    }
}
```

##### M4.T2: Add SharedContext to Pane Rendering

Update render signatures to pass shared_context:

```rust
// instance.rs
pub fn render(
    &mut self,
    ui: &mut Ui,
    shared_context: &ExplorerSharedContext,
) -> Vec<PaneAction> {
    let mut actions = Vec::new();

    // Split pane toggle button
    ui.horizontal(|ui| {
        let button_text = if self.show_right_pane {
            "Hide Split"
        } else {
            "Show Split"
        };
        if ui.button(button_text).clicked() {
            self.toggle_right_pane();
        }
    });
    ui.separator();

    if self.show_right_pane {
        // Two-column layout
        ui.columns(2, |columns| {
            actions.extend(self.left_pane.render(&mut columns[0], shared_context));

            // Right pane with visual separator
            egui::Frame::new()
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_gray(100)))
                .show(&mut columns[1], |ui| {
                    if let Some(ref mut right_pane) = self.right_pane {
                        actions.extend(right_pane.render(ui, shared_context));
                    }
                });
        });
    } else {
        actions.extend(self.left_pane.render(ui, shared_context));
    }

    actions
}

// pane.rs
pub fn render(
    &mut self,
    ui: &mut Ui,
    shared_context: &ExplorerSharedContext,
) -> Vec<PaneAction> {
    // Try to acquire write lock on state (non-blocking)
    if let Ok(mut state) = self.state.try_write() {
        // Pass pane ID and shared context to renderer
        self.renderer.render_with_id(ui, &mut state, self.id, shared_context)
    } else {
        // State locked, show loading
        ui.centered_and_justified(|ui| {
            ui.spinner();
            ui.label("Loading...");
        });
        Vec::new()
    }
}
```

##### M4.T3: Implement FocusableWindow Trait

```rust
use crate::app::dashui::window_focus::FocusableWindow;

impl FocusableWindow for ExplorerInstance {
    type ShowParams = ExplorerSharedContext;

    fn window_id(&self) -> &'static str {
        // Cache the window ID as a static string (leaked once per window)
        if let Some(cached_id) = self.window_id_static {
            cached_id
        } else {
            // Memory leak: ~40 bytes per window (acceptable)
            Box::leak(format!("explorer_instance_{}", self.id).into_boxed_str())
        }
    }

    fn window_title(&self) -> String {
        self.title.clone()
    }

    fn is_open(&self) -> bool {
        self.is_open
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        shared_context: Self::ShowParams,
        bring_to_front: bool,
    ) {
        let mut is_open = self.is_open;

        let mut window = egui::Window::new(self.window_title())
            .id(egui::Id::new(self.window_id()))
            .default_size([1200.0, 800.0])
            .resizable(true)
            .open(&mut is_open);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            let _actions = self.render(ui, &shared_context);
            // Actions will be collected via take_pending_actions()
        });

        // Render dialogs for all panes (outside main window)
        self.left_pane.render_dialogs(ctx, &shared_context);
        if let Some(ref mut right_pane) = self.right_pane {
            right_pane.render_dialogs(ctx, &shared_context);
        }

        self.is_open = is_open;
    }
}
```

##### M4.T4: Fix ExplorerManager Initialization

Remove auto-instance creation:

```rust
pub fn new() -> Self {
    Self {
        shared_context: ExplorerSharedContext::new(),
        instances: Vec::new(),  // Empty, not pre-populated
        focused_instance_id: None,
        next_instance_number: 1,
    }
}

pub fn open_new_window(&mut self) -> Uuid {
    let instance_id = Uuid::new_v4();
    let instance = ExplorerInstance::new(self.next_instance_number);
    self.next_instance_number += 1;

    self.instances.push(instance);
    self.focused_instance_id = Some(instance_id);

    instance_id
}

pub fn close_all_windows(&mut self) {
    self.instances.clear();
    self.focused_instance_id = None;
}
```

---

### Milestone 5: Wire Panes to Use Query Engine

**Goal**: Connect pane trigger_query_if_ready to UIQueryAdapter; wire up unified selection dialog to trigger queries.

**Human Test**: FULL INTEGRATION - Open Explorer window, click Select button, choose account/region/resource types, click Apply, verify loading spinner shows, verify resources load, verify status bar updates.

**Files**:
- **Modified**: `src/app/resource_explorer/instances/pane.rs`
- **Modified**: `src/app/resource_explorer/instances/pane_renderer.rs`

#### Tasks

##### M5.T1: Wire trigger_query_if_ready to UIQueryAdapter

**Reference**: Original logic in `window.rs:3434-3474`

```rust
// In pane.rs
fn trigger_query_if_ready(
    &self,
    ctx: &Context,
    shared_context: &super::manager::ExplorerSharedContext,
) {
    tracing::info!("Pane {}: trigger_query_if_ready called", self.id);

    // Check conditions
    if let Ok(state) = self.state.try_read() {
        if state.query_scope.is_empty() {
            tracing::warn!("Pane {}: Query scope is empty, not triggering", self.id);
            return;
        }

        if state.is_loading() {
            tracing::warn!("Pane {}: Already loading, not triggering", self.id);
            return;
        }

        tracing::info!(
            "Pane {}: Triggering query for {} account(s) × {} region(s) × {} resource type(s)",
            self.id,
            state.query_scope.accounts.len(),
            state.query_scope.regions.len(),
            state.query_scope.resource_types.len()
        );
    } else {
        tracing::error!("Pane {}: Failed to acquire state read lock", self.id);
        return;
    }

    // Get query scope
    let scope = if let Ok(state) = self.state.try_read() {
        state.query_scope.clone()
    } else {
        return;
    };

    // Get query engine and create UI adapter
    let query_engine = shared_context.query_engine.clone();
    let ui_adapter = UIQueryAdapter::new(query_engine);

    // Execute query via UI adapter
    let _handle = ui_adapter.execute_for_pane(
        self.state.clone(),
        scope,
        ctx.clone(),
    );
}
```

##### M5.T2: Wire Unified Selection Dialog Apply Button

**Reference**: `window.rs:1193-1251` (Apply button handling)

In `pane.rs::render_dialogs()`:

```rust
pub fn render_dialogs(
    &mut self,
    ctx: &Context,
    shared_context: &super::manager::ExplorerSharedContext,
) {
    // Unified selection dialog
    if let Ok(mut state) = self.state.try_write() {
        if state.show_unified_selection_dialog {
            // Get available accounts, regions, resource types...
            // Show dialog...

            if let Some((accounts, regions, resources)) =
                self.fuzzy_dialog.show_unified_selection_dialog(
                    ctx,
                    &mut state.show_unified_selection_dialog,
                    &available_accounts,
                    &available_regions,
                    &available_resource_types,
                    &current_accounts,
                    &current_regions,
                    &current_resources,
                )
            {
                // Update selections
                state.query_scope.accounts.clear();
                for account in accounts {
                    state.add_account(account);
                }

                state.query_scope.regions.clear();
                for region in regions {
                    state.add_region(region);
                }

                state.query_scope.resource_types.clear();
                for resource in resources {
                    state.add_resource_type(resource);
                }

                // Drop write lock before triggering query
                drop(state);

                // Trigger query after selection changes
                self.trigger_query_if_ready(ctx, shared_context);
            }
        }
    }
}
```

##### M5.T3: Verify Status Bar Rendering

**Reference**: `window.rs:448-597` (status bar rendering)

Ensure status bar in pane_renderer.rs polls state correctly:

```rust
fn render_status_bar(ui: &mut Ui, state: &ResourceExplorerState) {
    ui.horizontal(|ui| {
        // Check for Phase 1 progress
        if state.is_phase1_in_progress() {
            ui.spinner();
            let (pending, total, failed, queries) = state.get_phase1_progress();
            let completed = total - pending;

            ui.label(format!("Querying: {}/{}", completed, total));

            if !failed.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    format!("[{} Failed]", failed.len())
                );
            }
        } else {
            ui.label("Ready");
        }
    });
}
```

---

### Milestone 6: DashApp Integration (Already Done ✅)

**Status**: ✅ **COMPLETE** - User already integrated everything.

**Goal**: ~~Replace ResourceExplorer with ExplorerManager~~ **DONE** - All integration complete.

**Human Test**: Once M1-M5 complete, test full app: login → click menu → new window → query → verify results.

**Files**:
- **Modified**: `src/app/dashui/app/mod.rs`
- **Modified**: `src/app/dashui/app/window_rendering.rs`
- **Modified**: `src/app/dashui/app/rendering.rs`

#### Tasks

##### M6.T1: Replace ResourceExplorer with ExplorerManager

```rust
// In src/app/dashui/app/mod.rs
pub struct DashApp {
    // ... other fields ...

    #[serde(skip)]
    pub explorer_manager: ExplorerManager,  // Changed from resource_explorer
}

impl Default for DashApp {
    fn default() -> Self {
        Self {
            // ... other fields ...
            explorer_manager: ExplorerManager::new(),
        }
    }
}
```

##### M6.T2: Update AWS Login/Logout Handlers

**File**: `src/app/dashui/app/window_rendering.rs`

```rust
// On login
if let Ok(identity_center) = aws_identity.lock() {
    let default_role = identity_center.default_role_name.clone();
    let credential_coordinator = Arc::new(
        crate::app::resource_explorer::credentials::CredentialCoordinator::new(
            aws_identity.clone(),
            default_role,
        ),
    );
    let aws_client = Arc::new(
        crate::app::resource_explorer::AWSResourceClient::new(credential_coordinator)
    );

    // Set AWS client in ExplorerManager (updates query engine)
    self.explorer_manager.set_aws_client(Some(aws_client.clone()));

    // Set global AWS client for bridge tools
    set_global_aws_client(Some(aws_client));

    // Set global bookmark manager
    set_global_bookmark_manager(Some(self.explorer_manager.get_bookmark_manager()));

    tracing::info!("ExplorerManager AWS client created and set");
}

// On logout
self.explorer_manager.set_aws_client(None);
self.explorer_manager.close_all_windows();
set_global_aws_client(None);
set_global_bookmark_manager(None);
set_global_explorer_state(None);
```

##### M6.T3: Update Menu Handler to Create New Windows

**File**: `src/app/dashui/app/rendering.rs`

```rust
menu::MenuAction::AWSExplorer => {
    if self.is_aws_logged_in() {
        // Create new Explorer window instance
        let instance_id = self.explorer_manager.open_new_window();

        // Bring new window to front via focus system
        let window_id_str = format!("explorer_instance_{}", instance_id);
        self.window_focus_manager.bring_to_front(window_id_str);

        tracing::info!("New AWS Explorer window created from menu: {}", instance_id);
    } else {
        self.show_login_required_notification("AWS Explorer");
    }
}
```

##### M6.T4: Add handle_explorer_windows to Main Render Loop

**File**: `src/app/dashui/app/window_rendering.rs`

```rust
pub(super) fn handle_explorer_windows(&mut self, ctx: &egui::Context) {
    // Get instance IDs to avoid borrow conflicts
    let instance_ids: Vec<Uuid> = self.explorer_manager.instances
        .iter()
        .map(|i| i.id())
        .collect();

    for instance_id in instance_ids {
        if let Some(instance) = self.explorer_manager.get_instance_mut(instance_id) {
            let window_id_str = format!("explorer_instance_{}", instance_id);
            let bring_to_front = self.window_focus_manager.should_bring_to_front(&window_id_str);

            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(&window_id_str);
            }

            let shared_ctx = self.explorer_manager.shared_context.clone();
            FocusableWindow::show_with_focus(instance, ctx, shared_ctx, bring_to_front);
        }
    }

    // Collect and process pending actions
    let actions = self.explorer_manager.take_pending_actions();
    for action in actions {
        match action {
            ResourceExplorerAction::OpenCloudWatchLogs { account_id, region, log_group_name } => {
                self.open_cloudwatch_logs_window(account_id, region, log_group_name);
            }
            ResourceExplorerAction::OpenCloudTrailEvents { account_id, region, resource_arn } => {
                self.open_cloudtrail_events_window(account_id, region, resource_arn);
            }
            ResourceExplorerAction::OpenAWSConsole { url } => {
                if let Err(e) = webbrowser::open(&url) {
                    tracing::error!("Failed to open AWS Console: {}", e);
                }
            }
        }
    }
}
```

Update main loop in `src/app/dashui/app/mod.rs`:

```rust
// Replace:
self.handle_resource_explorer_window(ctx);

// With:
self.handle_explorer_windows(ctx);
```

##### M6.T5: Add Pending Actions Collection

**File**: `src/app/resource_explorer/instances/manager.rs`

```rust
impl ExplorerManager {
    pub fn take_pending_actions(&mut self) -> Vec<ResourceExplorerAction> {
        let mut actions = Vec::new();
        for instance in &mut self.instances {
            actions.extend(instance.take_pending_actions());
        }
        actions
    }
}
```

**File**: `src/app/resource_explorer/instances/instance.rs`

```rust
impl ExplorerInstance {
    pub fn take_pending_actions(&mut self) -> Vec<ResourceExplorerAction> {
        let mut actions = Vec::new();

        // Collect from left pane
        actions.extend(self.left_pane.take_pending_actions());

        // Collect from right pane if it exists
        if let Some(ref mut right_pane) = self.right_pane {
            actions.extend(right_pane.take_pending_actions());
        }

        actions
    }
}
```

**File**: `src/app/resource_explorer/instances/pane.rs`

```rust
impl ExplorerPane {
    pub fn take_pending_actions(&mut self) -> Vec<ResourceExplorerAction> {
        if let Ok(mut state) = self.state.try_write() {
            std::mem::take(&mut state.pending_actions)
        } else {
            Vec::new()
        }
    }
}
```

---

### Milestone 7: Build Agent Adapter (Future-Proofing)

**Goal**: Create agent adapter for query engine with agent-friendly progress format; add WebView adapter placeholder.

**Human Test**: Run agent script that queries resources; verify progress updates are agent-friendly (no egui types).

**Files**:
- **New**: `src/app/resource_explorer/agent_query_adapter.rs`
- **New**: `src/app/resource_explorer/webview_query_adapter.rs`

#### Tasks

##### M7.T1: Create Agent Adapter

```rust
pub struct AgentQueryAdapter {
    query_engine: Arc<ResourceQueryEngine>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentQueryProgress {
    pub query_id: String,
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub failed_count: usize,
    pub message: String,
}

pub type AgentProgressCallback = Box<dyn Fn(AgentQueryProgress) + Send + Sync>;

impl AgentQueryAdapter {
    pub fn new(query_engine: Arc<ResourceQueryEngine>) -> Self {
        Self { query_engine }
    }

    pub fn execute_for_agent(
        &self,
        scope: QueryScope,
        progress_callback: Option<AgentProgressCallback>,
    ) -> QueryHandle {
        let query_id = format!("agent_query_{}", Uuid::new_v4());
        let query_id_clone = query_id.clone();

        // Translate engine callbacks to agent format
        let engine_callback: ProgressCallback = Arc::new(move |progress| {
            if let Some(ref agent_cb) = progress_callback {
                let agent_progress = match progress {
                    QueryProgress::Phase1Started { total_queries } => {
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Phase1".to_string(),
                            current: 0,
                            total: total_queries,
                            failed_count: 0,
                            message: format!("Starting query ({} queries)", total_queries),
                        }
                    }
                    QueryProgress::Completed { resources, failed_queries } => {
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Completed".to_string(),
                            current: resources.len(),
                            total: resources.len(),
                            failed_count: failed_queries.len(),
                            message: format!(
                                "Query complete ({} resources, {} failed)",
                                resources.len(),
                                failed_queries.len()
                            ),
                        }
                    }
                    // ... other variants ...
                    _ => return,
                };

                agent_cb(agent_progress);
            }
        });

        self.query_engine.execute_query(scope, Some(engine_callback))
    }
}
```

##### M7.T2: Create WebView Adapter Placeholder

```rust
pub struct WebViewQueryAdapter {
    query_engine: Arc<ResourceQueryEngine>,
}

impl WebViewQueryAdapter {
    pub fn new(query_engine: Arc<ResourceQueryEngine>) -> Self {
        Self { query_engine }
    }

    // TODO: Implement when WebView requirements are known
}
```

---

## Critical Integration Points

**Files Modified by Milestone**:

**M1**: `src/app/resource_explorer/query_engine.rs` (new)
**M2**: `src/app/resource_explorer/ui_query_adapter.rs` (new)
**M3**: `src/app/resource_explorer/instances/manager.rs`
**M4**:
- `src/app/resource_explorer/instances/instance.rs` (remove tabs, add FocusableWindow)
- `src/app/resource_explorer/instances/pane.rs` (update render signatures)
- Delete `src/app/resource_explorer/instances/tab.rs`
**M5**:
- `src/app/resource_explorer/instances/pane.rs` (wire query engine)
- `src/app/resource_explorer/instances/pane_renderer.rs` (verify status bar)
**M6**:
- `src/app/dashui/app/mod.rs` (replace ResourceExplorer)
- `src/app/dashui/app/window_rendering.rs` (login/logout, render loop)
- `src/app/dashui/app/rendering.rs` (menu handler)
**M7**:
- `src/app/resource_explorer/agent_query_adapter.rs` (new)
- `src/app/resource_explorer/webview_query_adapter.rs` (new placeholder)

**Reference Files** (original logic to preserve):
- `window.rs:3434-3474` - Query triggering conditions
- `window.rs:3476-3700` - Query execution with channels
- `window.rs:3483-3514` - Query key building (global vs regional)
- `window.rs:3515-3531` - Phase 1 tracking initialization
- `window.rs:3615-3680` - State updates with retry logic (10 retries, 5ms sleep)
- `window.rs:448-597` - Status bar rendering (Phase 1/2 progress)

---

## Key Design Decisions

### 1. **No Persistence**
- **Decision**: Explorer state (windows, tabs, panes, filters) is NOT saved across app restarts
- **Rationale**: User preference, reduces complexity
- **Exception**: Bookmarks ARE persisted (via `BookmarkManager`)

### 2. **Global State from First Window**
- **Decision**: First window's first tab's left pane provides global state for V8 bindings
- **Rationale**: Agent scripts need a reference point; arbitrary choice is simpler than "focused pane"
- **Alternative**: Could track focused pane and update global state on focus change (adds complexity)

### 3. **Shared Cache and Bookmarks**
- **Decision**: Cache and bookmarks are singletons, shared across ALL panes
- **Rationale**: Reduces memory, improves performance, consistent user experience
- **Tradeoff**: Panes cannot have different cache TTLs or bookmark sets

### 4. **Lazy Right Pane Creation**
- **Decision**: Right pane is only created when user clicks "Show Split" for the first time
- **Rationale**: Saves memory for users who don't use split view
- **Tradeoff**: Slight complexity in `Option<ExplorerPane>` handling

### 5. **FocusableWindow Trait Integration**
- **Decision**: Explorer instances implement `FocusableWindow` trait
- **Rationale**: Consistent with other windows, enables bring-to-front via WindowFocusManager
- **Benefit**: Standard window behavior (minimize, close, focus)

### 6. **Menu Always Creates New Window**
- **Decision**: Menu/command palette actions create NEW windows, not toggle existing
- **Rationale**: User preference, matches typical multi-document interface (MDI) behavior
- **Alternative**: Could implement "focus or create" pattern (adds complexity)

### 7. **Action-Based UI Architecture**
- **Decision**: Panes return `Vec<PaneAction>` instead of directly mutating state
- **Rationale**: Cleaner separation of concerns, easiek testing, avoids borrow conflicts
- **Pattern**: Used in existing PaneRenderer (see `pane_renderer.rs`)

---

## Risk Analysis

### High Risk
1. **Global state for V8 bindings** - If first window is closed but others exist, global state is cleared but shouldn't be
   - **Mitigation**: Update global state to point to remaining window's state (see M6.T3)

2. **Pending actions queue** - Actions might be lost if window closes mid-frame
   - **Mitigation**: Collect actions BEFORE processing window close (done in M5.T6)

### Medium Risk
1. **Performance with many windows** - 50+ panes might cause frame rate issues
   - **Mitigation**: Profile and optimize rendering, consider lazy rendering of non-visible tabs

2. **Cache eviction during split pane rendering** - Both panes might query same resource, causing double fetch
   - **Mitigation**: Cache is checked before fetch, second pane will hit cache (already implemented)

### Low Risk
1. **Tab rename conflicts** - Two users (or agent + user) might rename same tab simultaneously
   - **Mitigation**: No concurrent access (single-threaded UI), not a real risk

2. **Window focus thrashing** - Opening many windows rapidly might cause focus issues
   - **Mitigation**: WindowFocusManager already handles this (see `src/ui/focus.rs`)

---

## Testing Strategy

### Unit Tests
- `ExplorerManager`: window lifecycle (open, close, focus)
- `ExplorerInstance`: tab lifecycle (add, close, rename)
- `ExplorerTab`: pane lifecycle (toggle split)
- `ExplorerPane`: state isolation (filters, searches don't leak)

### Integration Tests
- Multi-window creation and closure
- Shared cache verification (query in one pane, hit in another)
- Shared bookmarks verification
- Pending actions from multiple panes
- Global state updates on window lifecycle events

### UI Tests (egui_kittest)
- Tab bar rendering (add, close, rename buttons)
- Split pane toggle button
- Window title updates on tab rename
- Focus behavior (bring to front on creation)

### Manual Testing
- Open 10 windows with various tab/pane configurations
- Test extreme cases (100 tabs, 50 panes)
- Verify no memory leaks (close all windows, check process memory)
- Test with real AWS queries (not mocked)

---

## Migration Checklist

### Phase 1: Preparation
- [ ] Backup current working tree
- [ ] Review all `instances/` module code
- [ ] Verify tests pass before changes (`./scripts/test-chunks.sh fast`)

### Phase 2: Core Integration
- [ ] M1: Replace ResourceExplorer with ExplorerManager in DashApp
- [ ] M2: Update menu/command palette to create new windows
- [ ] M3: Implement FocusableWindow trait and update render loop
- [ ] Run tests after each milestone

### Phase 3: Lifecycle Management
- [ ] M4: Wire up tab/pane lifecycle (add, close, rename, toggle)
- [ ] M5: Ensure pending actions work across all instances
- [ ] Run tests after each milestone

### Phase 4: Global State
- [ ] M6: Update global state management for V8 bindings
- [ ] Test agent framework integration

### Phase 5: Testing and Documentation
- [ ] M7: Write integration tests
- [ ] M7: Update technical documentation
- [ ] M7: Manual testing checklist
- [ ] M7: Performance testing

### Phase 6: Cleanup
- [ ] Remove old `ResourceExplorer` wrapper (if fully replaced)
- [ ] Remove old `ResourceExplorerWindow` (if fully replaced)
- [ ] Update MERGE_PLAN.md
- [ ] Run full test suite (`./scripts/test-chunks.sh all`)

---

## Questions for Clarification

1. **Global State Selection**: Should global state update when user focuses a different window, or always point to first window's first pane?
   - **Current Plan**: Always point to first window's first pane (simpler)
   - **Alternative**: Update on focus change (more intuitive for agents?)

2. **Window Close Behavior**: If user closes the last tab in a window, should the window close too?
   - **Current Plan**: No, window stays open with no tabs (user must close window explicitly)
   - **Alternative**: Auto-close window when last tab is closed

3. **Initial Tab Name**: What should the default tab name be?
   - **Current Plan**: "New Tab" or "Explorer 1", "Explorer 2", etc.
   - **Alternative**: Based on active selection (e.g., "EC2 Instances" if that's the filter)

4. **Maximum Windows/Tabs**: Should there be a limit on number of windows or tabs?
   - **Current Plan**: No limit (user can open unlimited)
   - **Alternative**: Set a reasonable limit (e.g., 20 windows, 10 tabs per window)

5. **Pane Synchronization**: Should there be a "sync panes" button to copy filters from left to right pane?
   - **Current Plan**: No, panes are completely independent
   - **Alternative**: Add "Sync Filters" button for convenience

---

## Sample Code Snippets

### Creating a New Window (M2.T1)
```rust
// src/app/dashui/app/rendering.rs
menu::MenuAction::AWSExplorer => {
    if self.is_aws_logged_in() {
        let instance_id = self.explorer_manager.open_new_window();
        let window_id = egui::Id::new(format!("explorer_instance_{}", instance_id));
        self.window_focus_manager.bring_to_front(window_id);
        tracing::info!("Created Explorer window {}", instance_id);
    } else {
        self.show_login_required_notification("AWS Explorer");
    }
}
```

### Rendering All Windows (M3.T3)
```rust
// src/app/dashui/app/window_rendering.rs
pub(super) fn handle_explorer_windows(&mut self, ctx: &egui::Context) {
    let instance_ids: Vec<Uuid> = self.explorer_manager.get_all_instance_ids().collect();

    for instance_id in instance_ids {
        let window_id = egui::Id::new(format!("explorer_instance_{}", instance_id));
        let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);

        if bring_to_front {
            self.window_focus_manager.clear_bring_to_front(window_id);
        }

        if let Some(instance) = self.explorer_manager.get_instance_mut(instance_id) {
            let action = FocusableWindow::show_with_focus(instance, ctx, (), bring_to_front);

            if let WindowAction::Terminate = action {
                self.explorer_manager.close_window(instance_id);
            }
        }
    }

    // Process pending actions
    let actions = self.explorer_manager.take_pending_actions();
    for action in actions {
        match action {
            ResourceExplorerAction::OpenCloudWatchLogs { account_id, region, log_group_name } => {
                self.open_cloudwatch_logs_window(account_id, region, log_group_name);
            }
            ResourceExplorerAction::OpenCloudTrailEvents { account_id, region, resource_arn } => {
                self.open_cloudtrail_events_window(account_id, region, resource_arn);
            }
            ResourceExplorerAction::OpenAWSConsole { url } => {
                if let Err(e) = webbrowser::open(&url) {
                    tracing::error!("Failed to open AWS Console: {}", e);
                }
            }
        }
    }
}
```

### Tab Rendering with Rename (M4.T3)
```rust
// src/app/resource_explorer/instances/instance.rs
pub fn render(&mut self, ui: &mut egui::Ui) {
    // Tab bar
    ui.horizontal(|ui| {
        let mut tab_to_close = None;

        for (idx, tab) in self.tabs.iter_mut().enumerate() {
            let is_active = Some(idx) == self.active_tab_index;

            if tab.is_renaming {
                let response = ui.text_edit_singleline(&mut tab.name);
                if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    tab.is_renaming = false;
                }
            } else {
                let response = ui.selectable_label(is_active, &tab.name);
                if response.clicked() {
                    self.active_tab_index = Some(idx);
                }
                if response.double_clicked() {
                    tab.is_renaming = true;
                }
            }

            if ui.small_button("x").clicked() {
                tab_to_close = Some(idx);
            }
        }

        if ui.small_button("+").clicked() {
            self.add_tab("New Tab".to_string());
        }

        if let Some(idx) = tab_to_close {
            self.close_tab(idx);
        }
    });

    ui.separator();

    // Active tab content
    if let Some(tab) = self.get_active_tab_mut() {
        tab.render_content(ui, &self.shared_context);
    }
}
```

### Split Pane Toggle (M4.T6)
```rust
// src/app/resource_explorer/instances/tab.rs
pub fn render_content(&mut self, ui: &mut egui::Ui, shared_context: &ExplorerSharedContext) {
    ui.horizontal(|ui| {
        let button_text = if self.show_right_pane { "Hide Split" } else { "Show Split" };
        if ui.button(button_text).clicked() {
            self.toggle_right_pane();
        }
    });

    ui.separator();

    if self.show_right_pane {
        ui.columns(2, |columns| {
            self.left_pane.render(&mut columns[0], shared_context);
            if let Some(right_pane) = &mut self.right_pane {
                right_pane.render(&mut columns[1], shared_context);
            }
        });
    } else {
        self.left_pane.render(ui, shared_context);
    }
}
```

---

## Success Criteria

- [ ] Menu button creates new Explorer windows (not toggle)
- [ ] Each window starts with one pane (left), user can add right pane
- [ ] Windows support multiple tabs
- [ ] Tabs can be renamed (double-click)
- [ ] Tabs can be closed (x button)
- [ ] Right pane can be toggled (Show Split / Hide Split button)
- [ ] All panes share the same cache (query once, hit cache in all panes)
- [ ] All panes share the same bookmarks (create in one, see in all)
- [ ] Each pane has independent state (filters, searches, selections, grouping)
- [ ] Pending actions work from any pane (CloudWatch, CloudTrail, Console)
- [ ] Global state for V8 bindings is updated correctly on window lifecycle
- [ ] FocusableWindow trait is implemented (bring to front works)
- [ ] No persistence (always start fresh on app restart)
- [ ] All tests pass (`./scripts/test-chunks.sh fast`)
- [ ] No memory leaks (verified via manual testing)
- [ ] Documentation is updated (`docs/technical/resource-explorer-system.md`)

---

## Timeline Estimate

**Milestone 1**: 2-3 hours (straightforward replacement)
**Milestone 2**: 1-2 hours (update menu handlers)
**Milestone 3**: 3-4 hours (FocusableWindow trait, render loop refactor)
**Milestone 4**: 2-3 hours (tab lifecycle, UI polish)
**Milestone 5**: 1-2 hours (pending actions, verify existing code)
**Milestone 6**: 1-2 hours (global state, V8 bindings)
**Milestone 7**: 4-6 hours (tests, documentation, manual testing)

**Total**: 14-22 hours over 2-3 sessions

---

---

## Milestone 8: Modular Query Architecture Refactoring

**Goal**: Extract the query engine from UI coupling to enable usage by UI, Agents, and WebView with progress callbacks.

### Background

**Current Problem**: The query system is tightly embedded in `ResourceExplorerWindow` (window.rs:3434+), making it difficult to reuse in agent framework or webview contexts.

**Original Query Flow** (commit `47fccf6` - Pre-Multi-Pane):
1. User clicks "Apply Selection" in unified dialog (dialogs.rs:941-952)
2. Dialog returns `Some((accounts, regions, resources))`
3. Window receives results and updates `state.query_scope` (window.rs:1193-1251)
4. Window calls `self.trigger_query_if_ready(&state, ctx)` (window.rs:1251)
5. `trigger_query_if_ready()` checks conditions (window.rs:3434-3474):
   - Query scope not empty
   - Not already loading
   - Marks state as loading via `state.start_loading_task("parallel_query")`
   - Requests UI repaint: `ctx.request_repaint_after(Duration::from_millis(50))`
6. Calls `Self::spawn_parallel_query(state_arc, scope, cache, aws_client, cache_key)` (window.rs:3476+)
7. `spawn_parallel_query()`:
   - Builds query keys: "account_id:region:resource_type" (window.rs:3483-3514)
   - Initializes Phase 1 tracking: `state.start_phase1_tracking(queries_to_track)` (window.rs:3515-3531)
   - Spawns thread with tokio runtime (window.rs:3536+)
   - Creates result/progress channels (window.rs:3581-3584)
   - Calls `aws_client.query_aws_resources_parallel()` (window.rs:3590-3595)
   - Processes results via retry loops: `state.try_write()` with 10 retries (window.rs:3615-3643)
   - Marks queries completed: `state.mark_phase1_query_completed(&query_key)` (window.rs:3621)
   - Marks queries failed: `state.mark_phase1_query_failed(&query_key)` (window.rs:3657)
8. UI polls state every frame (window.rs:448-597):
   - Checks `state.is_phase1_in_progress()` (window.rs:465)
   - Gets progress: `state.get_phase1_progress()` → `(pending, total, failed, queries)` (window.rs:471)
   - Shows spinner, percentage, service names (window.rs:571-596)
   - Displays failed queries after Phase 1 completes (window.rs:476-512)

**Current State** (Multi-Pane Architecture):
- Removed tabs, now: ExplorerManager → ExplorerInstance → ExplorerPane
- Each pane has `trigger_query_if_ready()` in pane.rs:259-328
- Status bars at pane level render progress (pane_renderer.rs:259+)
- Still calls `window.rs::spawn_parallel_query()` (now public)

**Requirements for Modular Architecture**:
1. Extract query engine to separate module (not coupled to Window or Pane)
2. Support multiple consumers: UI (panes), Agents, WebView
3. Progress callbacks for each consumer type (UI needs repaint, Agents need updates, WebView needs events)
4. Maintain all current functionality:
   - Phase 1 tracking (resource listing with pending/total/failed counts)
   - Phase 1.5 tracking (tag analysis)
   - Phase 2 tracking (enrichment)
   - Retry logic with lock contention handling
   - Cache integration (Moka cache for GET hits)
   - Error categorization for failed queries
5. UI status bar must work exactly as before (no functionality loss)

### Tasks

#### M8.T1: Design Query Engine Interface

**Goal**: Define the core query engine trait and progress callback system.

**File**: Create `src/app/resource_explorer/query_engine.rs`

**Design**:
```rust
/// Progress updates during query execution
#[derive(Clone, Debug)]
pub enum QueryProgress {
    /// Phase 1 started with expected query count
    Phase1Started { total_queries: usize },

    /// Phase 1 query completed successfully
    Phase1QueryCompleted {
        query_key: String,
        resource_count: usize,
        pending: usize,
        total: usize,
    },

    /// Phase 1 query failed
    Phase1QueryFailed {
        query_key: String,
        error: String,
        pending: usize,
        total: usize,
    },

    /// Phase 1 completed
    Phase1Completed {
        total_resources: usize,
        failed_queries: Vec<String>,
    },

    /// Phase 1.5 (tag analysis) progress
    Phase1_5Progress {
        stage: String,
        current: usize,
        total: usize,
    },

    /// Phase 2 (enrichment) progress
    Phase2Progress {
        service: String,
        current: usize,
        total: usize,
    },

    /// Query execution completed
    Completed {
        resources: Vec<ResourceEntry>,
        failed_queries: Vec<String>,
    },

    /// Query execution failed
    Failed { error: String },
}

/// Callback for query progress updates
pub type ProgressCallback = Arc<dyn Fn(QueryProgress) + Send + Sync>;

/// Core query engine - independent of UI
pub struct ResourceQueryEngine {
    aws_client: Arc<AWSResourceClient>,
    cache: Arc<SharedResourceCache>,
}

impl ResourceQueryEngine {
    pub fn new(
        aws_client: Arc<AWSResourceClient>,
        cache: Arc<SharedResourceCache>,
    ) -> Self {
        Self { aws_client, cache }
    }

    /// Execute a query with progress callbacks
    ///
    /// Based on: window.rs:3476-3700 (spawn_parallel_query)
    ///
    /// Returns a handle to the query execution for cancellation/status checks
    pub fn execute_query(
        &self,
        scope: QueryScope,
        progress_callback: Option<ProgressCallback>,
    ) -> QueryHandle {
        // Implementation will mirror spawn_parallel_query but with callbacks
        todo!("Extract from window.rs:3476+")
    }
}

/// Handle to a running query
pub struct QueryHandle {
    query_id: String,
    // Internal state for cancellation/status
}

impl QueryHandle {
    pub fn is_complete(&self) -> bool {
        todo!()
    }

    pub fn cancel(&self) {
        todo!()
    }
}
```

**Integration Points**:
- Reference: `window.rs:3476-3700` for query execution logic
- Reference: `aws_client.rs` for `query_aws_resources_parallel()`
- Reference: `cache.rs` for cache integration

**Testing**:
- Unit tests: Query engine can run without UI
- Integration tests: Query execution matches original behavior
- Callback tests: Progress events fire at correct times

---

#### M8.T2: Extract Query Execution Logic

**Goal**: Move `spawn_parallel_query` logic into `ResourceQueryEngine.execute_query()`.

**File**: `src/app/resource_explorer/query_engine.rs`

**Implementation Steps**:

1. **Copy query key building logic** (window.rs:3483-3514):
   ```rust
   fn build_query_keys(&self, scope: &QueryScope) -> Vec<String> {
       let global_registry = super::GlobalServiceRegistry::new();
       let mut queries_to_track = Vec::new();

       for account in &scope.accounts {
           for resource_type in &scope.resource_types {
               if global_registry.is_global(&resource_type.resource_type) {
                   // Global service: one query per account
                   let query_key = make_query_key(
                       &account.account_id,
                       "Global",
                       &resource_type.resource_type,
                   );
                   if !queries_to_track.contains(&query_key) {
                       queries_to_track.push(query_key);
                   }
               } else {
                   // Regional service: one query per account × region
                   for region in &scope.regions {
                       let query_key = make_query_key(
                           &account.account_id,
                           &region.region_code,
                           &resource_type.resource_type,
                       );
                       queries_to_track.push(query_key);
                   }
               }
           }
       }

       queries_to_track
   }
   ```

2. **Implement async query execution** (window.rs:3536-3700):
   ```rust
   pub fn execute_query(
       &self,
       scope: QueryScope,
       progress_callback: Option<ProgressCallback>,
   ) -> QueryHandle {
       let query_id = format!("query_{}", Uuid::new_v4());
       let queries_to_track = self.build_query_keys(&scope);

       // Fire Phase1Started callback
       if let Some(ref callback) = progress_callback {
           callback(QueryProgress::Phase1Started {
               total_queries: queries_to_track.len(),
           });
       }

       let aws_client = self.aws_client.clone();
       let cache = self.cache.clone();
       let callback_clone = progress_callback.clone();

       std::thread::spawn(move || {
           let runtime = tokio::runtime::Runtime::new().unwrap();

           runtime.block_on(async {
               let (result_sender, mut result_receiver) =
                   tokio::sync::mpsc::channel(1000);
               let (progress_sender, mut progress_receiver) =
                   tokio::sync::mpsc::channel(100);

               // Launch parallel query
               let query_future = aws_client.query_aws_resources_parallel(
                   &scope,
                   result_sender,
                   Some(progress_sender),
                   cache.clone(),
               );

               let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
               let all_resources_clone = all_resources.clone();
               let queries_tracker = Arc::new(tokio::sync::Mutex::new(
                   queries_to_track.clone()
               ));
               let failed_queries = Arc::new(tokio::sync::Mutex::new(Vec::new()));

               // Process results and fire callbacks
               let result_processing = async {
                   while let Some(result) = result_receiver.recv().await {
                       let query_key = make_query_key(
                           &result.account_id,
                           &result.region,
                           &result.resource_type,
                       );

                       match result.resources {
                           Ok(resources) => {
                               let resource_count = resources.len();
                               {
                                   let mut all_res = all_resources_clone.lock().await;
                                   all_res.extend(resources);

                                   let mut tracker = queries_tracker.lock().await;
                                   tracker.retain(|q| q != &query_key);
                                   let pending = tracker.len();

                                   // Fire Phase1QueryCompleted callback
                                   if let Some(ref cb) = callback_clone {
                                       cb(QueryProgress::Phase1QueryCompleted {
                                           query_key: query_key.clone(),
                                           resource_count,
                                           pending,
                                           total: queries_to_track.len(),
                                       });
                                   }
                               }
                           }
                           Err(e) => {
                               {
                                   let mut tracker = queries_tracker.lock().await;
                                   tracker.retain(|q| q != &query_key);
                                   let pending = tracker.len();

                                   let mut failed = failed_queries.lock().await;
                                   failed.push(query_key.clone());

                                   // Fire Phase1QueryFailed callback
                                   if let Some(ref cb) = callback_clone {
                                       cb(QueryProgress::Phase1QueryFailed {
                                           query_key: query_key.clone(),
                                           error: e.to_string(),
                                           pending,
                                           total: queries_to_track.len(),
                                       });
                                   }
                               }
                           }
                       }
                   }

                   // Fire Phase1Completed callback
                   let final_resources = all_resources_clone.lock().await.clone();
                   let final_failed = failed_queries.lock().await.clone();

                   if let Some(ref cb) = callback_clone {
                       cb(QueryProgress::Phase1Completed {
                           total_resources: final_resources.len(),
                           failed_queries: final_failed.clone(),
                       });
                   }

                   Ok::<_, anyhow::Error>((final_resources, final_failed))
               };

               // Wait for query and result processing to complete
               let ((query_result, processing_result)) = tokio::join!(
                   query_future,
                   result_processing,
               );

               match processing_result {
                   Ok((resources, failed)) => {
                       if let Some(ref cb) = callback_clone {
                           cb(QueryProgress::Completed {
                               resources,
                               failed_queries: failed,
                           });
                       }
                   }
                   Err(e) => {
                       if let Some(ref cb) = callback_clone {
                           cb(QueryProgress::Failed {
                               error: e.to_string(),
                           });
                       }
                   }
               }
           });
       });

       QueryHandle { query_id }
   }
   ```

**Reference Implementation**: `window.rs:3536-3700`

**Testing**:
- Query execution without callbacks works
- Callbacks fire in correct order: Phase1Started → Phase1QueryCompleted (multiple) → Phase1Completed → Completed
- Failed queries trigger Phase1QueryFailed callbacks
- Resource counts match original implementation

---

#### M8.T3: Create UI Adapter for Query Engine

**Goal**: Build an adapter that connects `ResourceQueryEngine` to pane state updates and UI repaints.

**File**: Create `src/app/resource_explorer/ui_query_adapter.rs`

**Design**:
```rust
/// Adapter that connects ResourceQueryEngine to UI state updates
pub struct UIQueryAdapter {
    query_engine: Arc<ResourceQueryEngine>,
}

impl UIQueryAdapter {
    pub fn new(query_engine: Arc<ResourceQueryEngine>) -> Self {
        Self { query_engine }
    }

    /// Execute a query with UI-specific callbacks
    ///
    /// This adapter:
    /// 1. Updates pane state via Arc<RwLock<ResourceExplorerState>> with retry logic
    /// 2. Requests UI repaint via egui::Context
    /// 3. Maintains backward compatibility with existing status bar rendering
    ///
    /// Based on:
    /// - Query triggering: window.rs:3434-3474 (trigger_query_if_ready)
    /// - State updates: window.rs:3615-3680 (mark_phase1_query_completed/failed)
    /// - UI polling: window.rs:464-597 (status bar rendering)
    pub fn execute_for_pane(
        &self,
        state: Arc<RwLock<ResourceExplorerState>>,
        scope: QueryScope,
        ctx: egui::Context,
    ) -> QueryHandle {
        // Mark state as loading (window.rs:3459-3467)
        let cache_key = if let Ok(mut loading_state) = state.try_write() {
            loading_state.start_loading_task("parallel_query")
        } else {
            format!(
                "parallel_query_fallback_{}",
                chrono::Utc::now().timestamp_millis()
            )
        };

        // Request UI repaint (window.rs:3470)
        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        let state_clone = state.clone();
        let ctx_clone = ctx.clone();
        let cache_key_clone = cache_key.clone();

        // Create progress callback that updates state and requests repaints
        let progress_callback: ProgressCallback = Arc::new(move |progress| {
            match progress {
                QueryProgress::Phase1Started { total_queries } => {
                    // Initialize Phase 1 tracking (window.rs:3515-3531)
                    let queries_to_track: Vec<String> = vec![]; // Populated by query engine
                    for attempt in 0..10 {
                        if let Ok(mut state) = state_clone.try_write() {
                            state.start_phase1_tracking(queries_to_track.clone());
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    ctx_clone.request_repaint();
                }

                QueryProgress::Phase1QueryCompleted {
                    query_key,
                    resource_count,
                    pending,
                    total,
                } => {
                    // Update state with retry logic (window.rs:3615-3643)
                    for attempt in 0..10 {
                        if let Ok(mut state) = state_clone.try_write() {
                            // Note: resources are added separately, not in callback
                            state.mark_phase1_query_completed(&query_key);
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    ctx_clone.request_repaint();
                }

                QueryProgress::Phase1QueryFailed {
                    query_key,
                    error,
                    pending,
                    total,
                } => {
                    // Mark failed with retry logic (window.rs:3652-3678)
                    for attempt in 0..10 {
                        if let Ok(mut state) = state_clone.try_write() {
                            state.mark_phase1_query_failed(&query_key);
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    ctx_clone.request_repaint();
                }

                QueryProgress::Phase1Completed {
                    total_resources,
                    failed_queries,
                } => {
                    ctx_clone.request_repaint();
                }

                QueryProgress::Completed {
                    resources,
                    failed_queries,
                } => {
                    // Update final resources
                    if let Ok(mut state) = state_clone.try_write() {
                        state.resources = resources;
                        state.loading_tasks.remove(&cache_key_clone);
                    }
                    ctx_clone.request_repaint();
                }

                QueryProgress::Failed { error } => {
                    if let Ok(mut state) = state_clone.try_write() {
                        state.loading_tasks.remove(&cache_key_clone);
                        state.reset_phase1_state();
                    }
                    ctx_clone.request_repaint();
                }

                _ => {
                    ctx_clone.request_repaint();
                }
            }
        });

        self.query_engine.execute_query(scope, Some(progress_callback))
    }
}
```

**Integration Points**:
- Reference: `window.rs:3459-3467` for loading task management
- Reference: `window.rs:3615-3680` for state update retry loops
- Reference: `window.rs:3470` for UI repaint requests

**Testing**:
- Pane state updates correctly during query execution
- UI repaints are requested at appropriate times
- Status bar rendering works exactly as before (window.rs:464-597)
- Failed queries are tracked in state

---

#### M8.T4: Update Pane to Use UI Adapter

**Goal**: Replace direct `spawn_parallel_query()` call with `UIQueryAdapter.execute_for_pane()`.

**File**: `src/app/resource_explorer/instances/pane.rs`

**Changes**:

**Current** (pane.rs:259-328):
```rust
fn trigger_query_if_ready(
    &self,
    ctx: &Context,
    shared_context: &super::manager::ExplorerSharedContext,
) {
    // ... condition checks ...

    // Spawn parallel query (using window.rs implementation)
    crate::app::resource_explorer::window::ResourceExplorerWindow::spawn_parallel_query(
        state_arc,
        scope,
        cache,
        aws_client,
        cache_key,
    );
}
```

**New**:
```rust
fn trigger_query_if_ready(
    &self,
    ctx: &Context,
    shared_context: &super::manager::ExplorerSharedContext,
) {
    tracing::info!("Pane {}: trigger_query_if_ready called", self.id);

    // Check if we have selections and not already loading
    if let Ok(state) = self.state.try_read() {
        if state.query_scope.is_empty() {
            tracing::warn!("Pane {}: Query scope is empty, not triggering", self.id);
            return;
        }

        if state.is_loading() {
            tracing::warn!("Pane {}: Already loading, not triggering", self.id);
            return;
        }

        tracing::info!(
            "Pane {}: Triggering query for {} account(s) × {} region(s) × {} resource type(s)",
            self.id,
            state.query_scope.accounts.len(),
            state.query_scope.regions.len(),
            state.query_scope.resource_types.len()
        );
    } else {
        tracing::error!("Pane {}: Failed to acquire state read lock", self.id);
        return;
    }

    // Get query engine from shared context
    let query_engine = shared_context.query_engine.clone();
    let ui_adapter = UIQueryAdapter::new(query_engine);

    // Get query scope
    let scope = if let Ok(state) = self.state.try_read() {
        state.query_scope.clone()
    } else {
        return;
    };

    // Execute query via UI adapter (handles state updates and repaints)
    let _handle = ui_adapter.execute_for_pane(
        self.state.clone(),
        scope,
        ctx.clone(),
    );

    // Store handle if we need cancellation support later
}
```

**Update ExplorerSharedContext** (manager.rs):
```rust
pub struct ExplorerSharedContext {
    pub cache: Arc<SharedResourceCache>,
    pub bookmarks: Arc<StdRwLock<BookmarkManager>>,
    pub aws_identity_center: Option<Arc<StdRwLock<AwsIdentityCenter>>>,
    pub aws_client: Option<Arc<AWSResourceClient>>,

    // NEW: Query engine for executing resource queries
    pub query_engine: Arc<ResourceQueryEngine>,
}

impl ExplorerSharedContext {
    pub fn new() -> Self {
        let cache = super::cache::shared_cache();

        Self {
            cache: cache.clone(),
            bookmarks: Arc::new(StdRwLock::new(BookmarkManager::new())),
            aws_identity_center: None,
            aws_client: None,
            query_engine: Arc::new(ResourceQueryEngine::new(
                // Will be updated when AWS client is set
                Arc::new(AWSResourceClient::new(/* placeholder */)),
                cache,
            )),
        }
    }

    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client.clone();

        // Update query engine with new AWS client
        if let Some(client) = aws_client {
            self.query_engine = Arc::new(ResourceQueryEngine::new(
                client,
                self.cache.clone(),
            ));
        }
    }
}
```

**Testing**:
- Pane queries execute using new adapter
- Status bar updates correctly during query
- Failed queries are tracked
- Multiple panes can query simultaneously without conflicts

---

#### M8.T5: Create Agent Adapter for Query Engine

**Goal**: Build an adapter for agent framework to execute queries with agent-specific callbacks.

**File**: Create `src/app/resource_explorer/agent_query_adapter.rs`

**Design**:
```rust
/// Adapter that connects ResourceQueryEngine to agent callbacks
pub struct AgentQueryAdapter {
    query_engine: Arc<ResourceQueryEngine>,
}

/// Agent-friendly progress updates (no UI types)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentQueryProgress {
    pub query_id: String,
    pub phase: String, // "Phase1", "Phase1.5", "Phase2", "Completed"
    pub current: usize,
    pub total: usize,
    pub failed_count: usize,
    pub message: String, // Human-readable status
}

pub type AgentProgressCallback = Box<dyn Fn(AgentQueryProgress) + Send + Sync>;

impl AgentQueryAdapter {
    pub fn new(query_engine: Arc<ResourceQueryEngine>) -> Self {
        Self { query_engine }
    }

    /// Execute a query with agent-specific callbacks
    ///
    /// Unlike UI adapter, this:
    /// 1. Does NOT update pane state (agents don't have state)
    /// 2. Does NOT request UI repaints
    /// 3. Returns results via callback instead of state mutation
    /// 4. Provides simplified progress updates suitable for agent tools
    pub fn execute_for_agent(
        &self,
        scope: QueryScope,
        progress_callback: Option<AgentProgressCallback>,
    ) -> QueryHandle {
        let query_id = format!("agent_query_{}", Uuid::new_v4());
        let query_id_clone = query_id.clone();

        // Create progress callback that translates to agent format
        let engine_callback: ProgressCallback = Arc::new(move |progress| {
            if let Some(ref agent_cb) = progress_callback {
                let agent_progress = match progress {
                    QueryProgress::Phase1Started { total_queries } => {
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Phase1".to_string(),
                            current: 0,
                            total: total_queries,
                            failed_count: 0,
                            message: format!("Starting resource query ({} queries)", total_queries),
                        }
                    }

                    QueryProgress::Phase1QueryCompleted { pending, total, .. } => {
                        let current = total - pending;
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Phase1".to_string(),
                            current,
                            total,
                            failed_count: 0,
                            message: format!("Querying resources ({}/{})", current, total),
                        }
                    }

                    QueryProgress::Phase1QueryFailed { pending, total, .. } => {
                        let current = total - pending;
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Phase1".to_string(),
                            current,
                            total,
                            failed_count: 1,
                            message: format!("Query failed ({}/{})", current, total),
                        }
                    }

                    QueryProgress::Phase1Completed { total_resources, failed_queries } => {
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Phase1".to_string(),
                            current: total_resources,
                            total: total_resources,
                            failed_count: failed_queries.len(),
                            message: format!(
                                "Resource listing complete ({} resources, {} failed)",
                                total_resources,
                                failed_queries.len()
                            ),
                        }
                    }

                    QueryProgress::Completed { resources, failed_queries } => {
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Completed".to_string(),
                            current: resources.len(),
                            total: resources.len(),
                            failed_count: failed_queries.len(),
                            message: format!(
                                "Query complete ({} resources, {} failed queries)",
                                resources.len(),
                                failed_queries.len()
                            ),
                        }
                    }

                    QueryProgress::Failed { error } => {
                        AgentQueryProgress {
                            query_id: query_id_clone.clone(),
                            phase: "Failed".to_string(),
                            current: 0,
                            total: 0,
                            failed_count: 1,
                            message: format!("Query failed: {}", error),
                        }
                    }

                    _ => return, // Ignore other progress types for agents
                };

                agent_cb(agent_progress);
            }
        });

        self.query_engine.execute_query(scope, Some(engine_callback))
    }
}
```

**Integration Points**:
- Agent tools will use this adapter instead of accessing pane state
- V8 bindings can expose `execute_for_agent()` to JavaScript
- Results are returned via callback, not stored in state

**Testing**:
- Agents can execute queries without UI
- Progress callbacks fire with correct agent-friendly format
- Multiple agent queries can run simultaneously
- No UI dependencies in agent adapter

---

#### M8.T6: Integrate Query Engine with Shared Context

**Goal**: Add query engine to `ExplorerSharedContext` and update on AWS client changes.

**File**: `src/app/resource_explorer/instances/manager.rs`

**Changes** (see M8.T4 for struct changes):

**Update AWS client setter**:
```rust
impl ExplorerManager {
    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        // Update shared context
        self.shared_context.set_aws_client(aws_client);

        // All existing panes will get the new query engine via shared_context reference
        tracing::info!("Query engine updated with new AWS client");
    }
}
```

**Testing**:
- Query engine is created with AWS client on login
- All panes get the same query engine instance
- Query engine updates when AWS client changes

---

#### M8.T7: Add WebView Adapter (Future - Placeholder)

**Goal**: Provide placeholder for future WebView integration.

**File**: Create `src/app/resource_explorer/webview_query_adapter.rs`

**Design** (placeholder for future implementation):
```rust
/// Adapter that connects ResourceQueryEngine to WebView callbacks
///
/// This is a placeholder for future WebView integration.
/// WebView will likely need:
/// 1. JSON-serializable progress updates
/// 2. Event-based callbacks (not direct function calls)
/// 3. Query cancellation support
/// 4. Resource streaming (not batch updates)
pub struct WebViewQueryAdapter {
    query_engine: Arc<ResourceQueryEngine>,
}

impl WebViewQueryAdapter {
    pub fn new(query_engine: Arc<ResourceQueryEngine>) -> Self {
        Self { query_engine }
    }

    // TODO: Implement when WebView requirements are known
    // Expected signature:
    // pub fn execute_for_webview(
    //     &self,
    //     scope: QueryScope,
    //     event_sender: WebViewEventSender,
    // ) -> QueryHandle { ... }
}
```

---

#### M8.T8: Testing and Validation

**Goal**: Ensure modular architecture maintains all original functionality.

**Test Files**: Create/update test files:
- `tests/query_engine_tests.rs` - Unit tests for query engine
- `tests/ui_query_adapter_tests.rs` - Integration tests for UI adapter
- `tests/agent_query_adapter_tests.rs` - Integration tests for agent adapter

**Test Cases**:

1. **Query Engine Tests**:
   - [ ] Query execution without callbacks works
   - [ ] Query execution with callbacks works
   - [ ] Failed queries trigger correct callbacks
   - [ ] Query key building matches original (window.rs:3483-3514)
   - [ ] Phase 1 tracking matches original behavior
   - [ ] Cache integration works (GET hits)

2. **UI Adapter Tests**:
   - [ ] Pane state updates correctly during query
   - [ ] UI repaints are requested at correct times
   - [ ] Status bar rendering works (window.rs:464-597)
   - [ ] Failed queries tracked in state
   - [ ] Multiple panes can query simultaneously
   - [ ] Lock contention retry logic works

3. **Agent Adapter Tests**:
   - [ ] Agent queries execute without UI
   - [ ] Progress callbacks fire with agent format
   - [ ] Multiple agent queries run simultaneously
   - [ ] No UI dependencies in agent adapter

4. **Integration Tests**:
   - [ ] UI adapter produces same results as original window.rs
   - [ ] Agent adapter produces same results as UI adapter (different format)
   - [ ] Query engine shared across panes (cache hits)
   - [ ] AWS client updates propagate to query engine

**Manual Testing Checklist**:
- [ ] Open Explorer window, execute query, verify status bar updates
- [ ] Execute query in multiple panes simultaneously, verify no conflicts
- [ ] Execute query via agent tool, verify progress updates
- [ ] Change AWS credentials, verify queries use new credentials
- [ ] Compare query timing logs before/after refactor (should match)

**Performance Testing**:
- [ ] Query execution time matches original (±5%)
- [ ] Memory usage matches original (±10%)
- [ ] No additional lock contention (measure try_write failures)
- [ ] Cache hit rate matches original

**References for Validation**:
- Original query timing: Check `query_timing.log` for Phase 1/2 durations
- Original state updates: Compare `awsdash.log` for state transition logs
- Original UI behavior: Verify status bar updates at same frequency

---

### Integration Points

**Files Modified**:
1. **New**: `src/app/resource_explorer/query_engine.rs` - Core query engine
2. **New**: `src/app/resource_explorer/ui_query_adapter.rs` - UI adapter
3. **New**: `src/app/resource_explorer/agent_query_adapter.rs` - Agent adapter
4. **New**: `src/app/resource_explorer/webview_query_adapter.rs` - WebView placeholder
5. **Modified**: `src/app/resource_explorer/instances/pane.rs` - Use UI adapter
6. **Modified**: `src/app/resource_explorer/instances/manager.rs` - Add query engine to shared context
7. **New**: `tests/query_engine_tests.rs` - Unit tests
8. **New**: `tests/ui_query_adapter_tests.rs` - Integration tests
9. **New**: `tests/agent_query_adapter_tests.rs` - Integration tests

**References to Original Code**:
- Query triggering: `window.rs:3434-3474` (trigger_query_if_ready)
- Query execution: `window.rs:3476-3700` (spawn_parallel_query)
- Query key building: `window.rs:3483-3514`
- Phase 1 tracking init: `window.rs:3515-3531`
- State updates with retry: `window.rs:3615-3680`
- UI status bar rendering: `window.rs:448-597`
- Phase 1 progress: `window.rs:464-517`
- Phase 2 progress: `window.rs:550-564`

---

### Design Decisions

1. **Callback-based architecture**: Query engine fires callbacks instead of directly mutating state
   - **Rationale**: Enables multiple consumers (UI, Agents, WebView) without coupling
   - **Tradeoff**: Slightly more complex than direct state mutation

2. **Separate adapters per consumer type**: UIQueryAdapter, AgentQueryAdapter, WebViewQueryAdapter
   - **Rationale**: Each consumer has different requirements (UI needs repaints, Agents need JSON)
   - **Tradeoff**: More code, but cleaner separation of concerns

3. **Query engine is stateless**: Does not store query results or progress
   - **Rationale**: Results/progress flow through callbacks to consumer's storage
   - **Tradeoff**: Consumers must manage their own state (already the case)

4. **Maintain retry logic in adapters**: Lock contention retry loops stay in UIQueryAdapter
   - **Rationale**: Retry logic is UI-specific (tokio async + try_write)
   - **Tradeoff**: Query engine doesn't handle retries, adapters must implement

5. **Reuse existing state tracking**: `ResourceExplorerState` keeps Phase 1/2 tracking fields
   - **Rationale**: UI status bar already polls these fields (window.rs:464+)
   - **Tradeoff**: State struct stays coupled to UI needs (acceptable for now)

6. **Query engine in shared context**: All panes/agents access same engine instance
   - **Rationale**: Ensures consistent behavior, shares AWS client and cache
   - **Tradeoff**: Query engine must be thread-safe (already is via Arc)

---

### Success Criteria

- [ ] Query engine can execute queries independently of UI
- [ ] UI adapter maintains exact original behavior (status bar, state updates)
- [ ] Agent adapter provides agent-friendly progress format
- [ ] Multiple panes can query simultaneously without conflicts
- [ ] Agents can execute queries without accessing pane state
- [ ] All tests pass (unit, integration, manual)
- [ ] Query timing logs match original (±5%)
- [ ] No performance regression (timing, memory)
- [ ] Documentation updated (see M8.T9)

---

### Timeline Estimate

**M8.T1**: 2-3 hours (design query engine interface)
**M8.T2**: 4-5 hours (extract query execution logic)
**M8.T3**: 3-4 hours (create UI adapter)
**M8.T4**: 1-2 hours (update pane to use UI adapter)
**M8.T5**: 2-3 hours (create agent adapter)
**M8.T6**: 1 hour (integrate with shared context)
**M8.T7**: 30 minutes (WebView placeholder)
**M8.T8**: 4-6 hours (testing and validation)

**Total**: 18-26 hours over 3-4 sessions

---

## Next Steps

1. Review this plan with user to ensure alignment
2. Answer clarifying questions (see "Questions for Clarification" section)
3. Begin implementation with Milestone 1 (low risk, foundational)
4. Test after each milestone to catch issues early
5. Iterate based on feedback and testing results

---

**Plan created**: 2026-01-04
**Updated**: 2026-01-05 (added Milestone 8: Modular Query Architecture)
**Target completion**: TBD (based on user schedule)
**Implementation approach**: Incremental, test-driven, milestone-based
