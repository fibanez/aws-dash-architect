# Project Management

Project organization system providing multi-environment CloudFormation infrastructure management with resource tracking, dependency graphs, and seamless template import/export workflows.

## Core Functionality

**Key Features:**
- Multi-environment project organization with Dev/Prod environment defaults
- Smart dependency resolution with deferred processing and cycle detection
- Complete CloudFormation template import/export with zero data loss
- Resource tracking with type-safe property preservation and round-trip fidelity
- Git repository integration for version control and collaboration
- Project-based file organization with automatic directory structure creation

**Main Components:**
- **Project Container**: Multi-environment infrastructure organization with metadata management
- **Environment Management**: Account and region isolation with extensible configuration
- **Resource Tracking**: Dependency graph (DAG) with smart resolution and emergency recovery
- **Template Integration**: Bidirectional CloudFormation compatibility with format preservation
- **File Operations**: Auto-save, recovery mechanisms, and cross-platform path handling

**Integration Points:**
- CloudFormation System for template parsing and dependency validation
- Window Focus System for keyboard navigation and multi-window management
- Command Palette System for project operations (New/Open/Edit workflows)
- File Picker System for template import and project directory selection

## Implementation Details

**Key Files:**
- `src/app/projects.rs` - Core project management logic and data structures
- `src/app/cfn_dag.rs` - Resource dependency graph with smart resolution algorithms
- `src/app/dashui/project_command_palette.rs` - Project selection and management interface
- `src/app/dashui/menu.rs` - Menu bar project status integration and real-time updates

**Three-Tier Project Structure:**
```
Project (Infrastructure Container)
    ↓
Environment (AWS Account + Region Configuration)  
    ↓
CloudFormationResource (Individual Infrastructure Components)
```

**Smart Dependency Resolution:**
```rust
impl ResourceDag {
    pub fn add_resource_smart(&mut self, name: String, resource: CloudFormationResource) -> Result<(), String> {
        if self.can_add_immediately(&resource) {
            self.add_resource_direct(name, resource)?;
        } else {
            self.deferred_queue.push((name, resource)); // Process later when dependencies available
        }
        self.process_deferred_queue()?;
        Ok(())
    }
}
```

**File Organization:**
- **Modern Layout**: `Project.json` + `Resources/cloudformation_template.json`
- **Legacy Migration**: Automatic consolidation from individual resource files
- **Cross-Platform**: `PathBuf` usage with absolute path storage for reliability

**Configuration Requirements:**
- Project directory with read/write permissions for metadata and template storage
- Git repository URL (optional) for version control integration
- AWS accounts and regions configuration for multi-environment deployment

## Developer Notes

**Extension Points for Project Workflow Customization:**

1. **Add New Environment Types**:
   ```rust
   // Extend beyond Dev/Prod defaults
   fn create_custom_environments() -> Vec<Environment> {
       vec![
           Environment { name: "Staging".to_string(), ... },
           Environment { name: "Production".to_string(), ... },
           Environment { name: "DR".to_string(), ... }, // Disaster Recovery
       ]
   }
   ```

2. **Custom Validation Rules**:
   ```rust
   // Add project-specific validation logic
   impl Project {
       pub fn validate_custom_constraints(&self) -> Result<(), ValidationError> {
           // Implement organization-specific validation rules
       }
   }
   ```

3. **Template Processing Hooks**:
   ```rust
   // Add pre/post processing for import/export operations
   pub trait TemplateProcessor {
       fn pre_import(&self, template: &mut CloudFormationTemplate) -> Result<()>;
       fn post_export(&self, template: &CloudFormationTemplate) -> Result<()>;
   }
   ```

**Template Import/Export Workflow:**
- **Import**: JSON/YAML auto-detection → Parse sections → Build DAG → Validate references → Create project
- **Export**: Generate deployment order → Serialize sections → Preserve formatting → Validate round-trip
- **Migration**: Legacy individual files → Consolidated template → Archive old structure

**Architectural Decisions:**
- **Smart Resolution**: Handles out-of-order resource dependencies during template import
- **Zero Data Loss**: Complete attribute preservation through `serde_json::Value` storage
- **Multi-Environment**: Separates concerns between infrastructure definition and deployment targets
- **Emergency Recovery**: Multiple fallback mechanisms for corrupted project states

**Performance Considerations:**
- Linear scaling with resource count (handles 1000+ resources efficiently)
- Lazy loading for large projects with on-demand DAG construction
- Memory-efficient storage with Arc-based sharing for immutable data
- Background processing for long-running template operations

**References:**
- [CloudFormation System](cloudformation-system.md) - Template processing and dependency validation
- [Command Palette System](command-palette-system.md) - Project management workflow integration
- [File Picker System](file-picker-system.md) - Template import and directory selection