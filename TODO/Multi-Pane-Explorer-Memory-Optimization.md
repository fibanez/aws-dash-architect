# Memory Optimization Implementation Plan

## Goal
Reduce memory usage from ~450MB to ~150MB for 314 resources by:
1. Using automatic JSON serialization (simplify code)
2. Removing `raw_properties` duplication (50% memory reduction)
3. Fixing `cached_tree` to avoid cloning resources (33% additional reduction)

---

## Phase 1: Automatic JSON Serialization (Low Risk, High Value)

### Objective
Replace manual `function_to_json()` with automatic `serde_json::to_value()` to:
- Get AWS SDK fields automatically
- Reduce code from ~130 lines to 1 line per service
- Ensure future SDK updates include new fields

### Changes Required

**1.1. Update Lambda service** (`src/app/resource_explorer/aws_services/lambda.rs`)
```rust
// BEFORE (lines 1090-1207):
fn function_to_json(&self, function: &FunctionConfiguration) -> serde_json::Value {
    let mut json = serde_json::Map::new();
    // ... 130 lines of manual field extraction
}

// AFTER:
fn function_to_json(&self, function: &FunctionConfiguration) -> serde_json::Value {
    serde_json::to_value(function).unwrap_or_else(|e| {
        tracing::error!("Failed to serialize Lambda function: {}", e);
        serde_json::json!({})
    })
}
```

**1.2. Verify AWS SDK types are Serialize**
- Check: `aws_sdk_lambda::types::FunctionConfiguration` derives `Serialize`
- If not: Add manual `Serialize` impl or keep manual conversion

**1.3. Update all other services**
Apply same pattern to:
- S3 (bucket_to_json)
- DynamoDB (table_to_json)
- EC2 (instance_to_json)
- ~100 other services

### Testing
```bash
# Build and verify no compilation errors
cargo build

# Test that all resource types still display correctly
# Run app, query Lambda/S3/EC2, expand resources, verify JSON tree shows all fields
```

### Expected Impact
- Code reduction: ~10,000 lines removed
- Memory: No change (same JSON stored)
- Risk: Low (output format unchanged)

---

## Phase 2: Remove raw_properties (High Risk, High Value)

### Objective
Eliminate duplicate storage of properties by removing `raw_properties` field.

### Changes Required

**2.1. Update ResourceEntry struct** (`src/app/resource_explorer/state.rs`)
```rust
// BEFORE:
pub struct ResourceEntry {
    pub properties: serde_json::Value,     // Merged properties
    pub raw_properties: serde_json::Value, // ← REMOVE THIS
    pub detailed_timestamp: Option<DateTime<Utc>>,
    // ... other fields
}

// AFTER:
pub struct ResourceEntry {
    pub properties: serde_json::Value,     // Complete properties (Phase 1 + Phase 2)
    pub detailed_timestamp: Option<DateTime<Utc>>,
    // ... other fields
}
```

**2.2. Update all normalizers** (~100 files in `src/app/resource_explorer/normalizers/`)
```rust
// BEFORE (lambda.rs line 42-52):
let properties = create_normalized_properties(&raw_response);  // Minimal
Ok(ResourceEntry {
    properties,
    raw_properties: raw_response,  // ← REMOVE
    // ...
})

// AFTER:
let properties = raw_response;  // Store complete AWS response
Ok(ResourceEntry {
    properties,
    // raw_properties removed
    // ...
})
```

**2.3. Remove create_normalized_properties()** (`src/app/resource_explorer/normalizers/mod.rs:943`)
- Delete entire function (lines 943-967)
- Remove all calls to it

**2.4. Update Phase 2 merging** (`src/app/resource_explorer/query_engine.rs:503-522`)
```rust
// BEFORE:
let mut merged = serde_json::Map::new();
// Layer 1: Start with raw_properties
if let Some(raw_obj) = enriched.raw_properties.as_object() {
    for (key, value) in raw_obj {
        merged.insert(key.clone(), value.clone());
    }
}
// Layer 2: Merge detailed on top
if let Some(detailed_obj) = detailed.as_object() {
    for (key, value) in detailed_obj {
        merged.insert(key.clone(), value.clone());
    }
}
enriched.properties = serde_json::Value::Object(merged);

// AFTER:
// properties already has Phase 1 data (full AWS response)
// Just merge detailed fields on top
if let Some(existing) = enriched.properties.as_object_mut() {
    if let Some(detailed_obj) = detailed.as_object() {
        for (key, value) in detailed_obj {
            existing.insert(key.clone(), value.clone());
        }
    }
}
```

**2.5. Update code referencing raw_properties**
Search and replace all occurrences:
```bash
rg "raw_properties" -l | wc -l  # Find all files
```

Common patterns:
```rust
// BEFORE:
resource.raw_properties.get("Arn")

// AFTER:
resource.properties.get("Arn")
```

Files likely affected:
- `tree.rs` (ARN extraction, line 1479, 1651)
- Context menus (copy ARN functionality)
- Any property path extraction logic

### Testing
```bash
# Build
cargo build

# Integration test
cargo test

# Manual verification:
# 1. Query resources
# 2. Verify all properties show in expanded JSON tree
# 3. Verify non-enrichable resources (LogGroups, ApiGateway) show full data
# 4. Verify enrichable resources (Lambda) show merged Phase 1 + Phase 2 data
# 5. Test ARN copy, AWS Console links, CloudTrail integration
```

### Expected Impact
- Memory: **50% reduction** (~450MB → ~225MB)
- Risk: **Medium** (touches many files, but pattern is simple)
- Breaking: Any code expecting `raw_properties` to exist

---

## Phase 3: Fix cached_tree (Medium Risk, Medium Value)

### Objective
Avoid cloning all ResourceEntry objects into the tree by using indices instead.

### Current Problem
```rust
// tree.rs line 225
type_node.add_resource(resource.clone());  // ← Clones entire ResourceEntry

// TreeNode line 16
pub resource_entries: Vec<ResourceEntry>,  // ← Stores clones
```

### Solution: Use Indices

**3.1. Change TreeNode to store indices** (`src/app/resource_explorer/tree.rs`)
```rust
// BEFORE:
pub struct TreeNode {
    pub id: String,
    pub display_name: String,
    pub color: Option<Color32>,
    pub children: Vec<TreeNode>,
    pub resource_entries: Vec<ResourceEntry>,  // ← REMOVE
    pub expanded: bool,
    pub node_type: NodeType,
}

// AFTER:
pub struct TreeNode {
    pub id: String,
    pub display_name: String,
    pub color: Option<Color32>,
    pub children: Vec<TreeNode>,
    pub resource_indices: Vec<usize>,  // ← Indices into state.resources
    pub expanded: bool,
    pub node_type: NodeType,
}
```

**3.2. Update TreeBuilder::build_tree()**
```rust
// BEFORE (line 222-226):
let mut sorted_resources = type_resources.clone();  // Clone
Self::sort_resources_by_name(&mut sorted_resources);
for resource in &sorted_resources {
    type_node.add_resource(resource.clone());  // Clone again
}

// AFTER:
// Build index map: resource_id -> position in resources slice
let resource_index_map: HashMap<String, usize> = resources
    .iter()
    .enumerate()
    .map(|(idx, r)| (r.resource_id.clone(), idx))
    .collect();

// Add indices instead of clones
let mut sorted_resources = type_resources.clone();  // Still need to clone for sorting
Self::sort_resources_by_name(&mut sorted_resources);
for resource in &sorted_resources {
    if let Some(&idx) = resource_index_map.get(&resource.resource_id) {
        type_node.add_resource_index(idx);
    }
}
```

**3.3. Update render methods to accept resources slice**
```rust
// BEFORE:
fn render_node(&mut self, ui: &mut Ui, node: &TreeNode, depth: usize, search_filter: &str)

// AFTER:
fn render_node(
    &mut self,
    ui: &mut Ui,
    node: &TreeNode,
    resources: &[ResourceEntry],  // ← Pass resources slice
    depth: usize,
    search_filter: &str
)

// Access resource by index:
for &resource_idx in &node.resource_indices {
    let resource = &resources[resource_idx];
    self.render_resource_node(ui, resource, search_filter);
}
```

**3.4. Update render_tree_cached to pass resources**
```rust
// BEFORE (line 1272):
if let Some(tree) = self.cached_tree.clone() {
    self.render_node(ui, &tree, 0, search_filter);
}

// AFTER:
if let Some(ref tree) = self.cached_tree {  // Borrow, don't clone
    self.render_node(ui, tree, resources, 0, search_filter);
}
```

**3.5. Handle child resource attachment**
Need to update `attach_child_resources()` to work with indices.

### Testing
```bash
# Build
cargo build

# Extensive UI testing required:
# 1. Verify tree structure displays correctly
# 2. Test all grouping modes (by account, region, resource type)
# 3. Test search filtering
# 4. Test resource expansion/collapse
# 5. Test child resource attachment
# 6. Test multi-pane with different queries
# 7. Test bookmark loading
```

### Expected Impact
- Memory: **33% reduction** of remaining (~225MB → ~150MB)
- Risk: **Medium-High** (changes tree rendering logic, complex testing)
- Performance: Slightly faster rendering (no clone needed)

---

## Phase 4: Verification and Cleanup

**4.1. Memory profiling**
```bash
# Run app with 314 resources
# Use system monitor to verify memory usage
# Target: ~150-200MB total (down from 1015MB)
```

**4.2. Add memory tracking**
```rust
// In ResourceEntry
pub fn memory_size(&self) -> usize {
    std::mem::size_of::<Self>()
        + self.properties.to_string().len()
        // No raw_properties anymore
        + self.tags.capacity() * std::mem::size_of::<ResourceTag>()
        // ... other fields
}
```

**4.3. Update documentation**
- Update CLAUDE.md with new memory architecture
- Document that `properties` contains full AWS data (not normalized)
- Remove references to `raw_properties`

---

## Risk Mitigation

**Git Strategy**:
```bash
# Phase 1 commit
git commit -m "refactor: use automatic JSON serialization for AWS resources"

# Phase 2 commit (can revert independently)
git commit -m "refactor: remove raw_properties duplication"

# Phase 3 commit (can revert independently)
git commit -m "refactor: use indices in cached_tree to avoid cloning"
```

**Rollback Plan**:
- Each phase is independently revertable
- Phase 1 is safest (pure refactor)
- Phase 2 has highest impact but medium risk
- Phase 3 is most complex but optional

**Testing Checkpoints**:
- After each phase: Full cargo test suite
- After Phase 2 & 3: Manual UI testing with multiple scenarios
- Before merging: Load test with 1000+ resources

---

## Implementation Order

1. ✅ **Phase 1 first** - Easy win, reduces code, enables auto SDK updates
2. ✅ **Phase 2 second** - Biggest memory impact, relatively straightforward
3. ⚠️ **Phase 3 optional** - Additional optimization if Phase 2 isn't enough

## Estimated Timeline

- Phase 1: 2-4 hours (mostly search/replace across services)
- Phase 2: 4-6 hours (careful refactoring, testing)
- Phase 3: 6-8 hours (complex refactoring, extensive testing)

**Total: 12-18 hours for all phases**

---

## Progress Tracking

### Phase 1: Automatic JSON Serialization - SKIPPED
- [x] Verify AWS SDK types implement Serialize - **FAILED: AWS SDK types don't implement Serialize**
- Skipped remaining steps - manual JSON conversion required

### Phase 2: Remove raw_properties - COMPLETED
- [x] Update ResourceEntry struct
- [x] Update all normalizers (~107 files)
- [x] Remove create_normalized_properties()
- [x] Update Phase 2 merging logic
- [x] Update tree.rs property references (global replace)
- [x] Fix duplicate field compilation errors
- [x] Build and test - SUCCESS (only unused import warnings)
- [ ] Manual UI verification - PENDING
- [ ] Commit - PENDING

### Phase 3: Fix cached_tree - COMPLETED
- [x] Update TreeNode struct (resource_entries → resource_indices)
- [x] Create resource_index_map in build_tree()
- [x] Update all add_resource() calls to add_resource_index()
- [x] Update attach_child_resources to use indices
- [x] Update build_tag_hierarchy_tree to use indices
- [x] Update build_property_hierarchy_tree to use indices
- [x] Update render_node signature to accept resources slice
- [x] Update render_tree_cached to borrow (not clone) tree
- [x] Fix resource_entries iteration to use resource_indices
- [x] Remove unused render_tree legacy method
- [x] Build and test - SUCCESS
- [ ] Extensive UI testing - PENDING
- [ ] Commit - PENDING

### Phase 4: Verification
- [ ] Memory profiling
- [ ] Update documentation
- [ ] Final testing
