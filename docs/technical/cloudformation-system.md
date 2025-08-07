# CloudFormation System

Template parsing, dependency management, and resource visualization system providing smart dependency resolution, interactive visualizations, and seamless template import/export workflows.

## Core Functionality

**Key Features:**
- Multi-format template parsing (JSON/YAML auto-detection) with complete attribute preservation
- Smart dependency resolution with deferred processing and circular dependency detection
- Interactive scene graph visualization with AWS service styling and real-time updates
- Three-level caching system for AWS resource specifications (memory → files → download)
- Template import/export with zero data loss and deployment compatibility
- Advanced intrinsic function processing with smart editor selection

**Main Components:**
- **Template Manager**: Multi-format parsing, validation, and complete CloudFormation section support
- **Resource Specifications System**: AWS specification downloads, caching, and schema validation
- **Dependency Graph (DAG)**: Smart resolution, topological sorting, and cycle detection with emergency recovery
- **Intrinsic Functions Processor**: Function classification, parsing, and editor selection logic
- **Scene Graph Renderer**: Interactive visualizations with egui native containers and AWS service styling
- **Multi-Section Support**: Resources, Parameters, Outputs, Mappings, Conditions, Metadata handling

**Integration Points:**
- Window Focus System for keyboard navigation and multi-window interaction
- Project Management System for template organization and environment workflows
- AWS Service Integration for real-time specification updates and validation
- File System Integration with auto-save, recovery, and cross-platform path handling
- CloudFormation Guard System for compliance validation and violation reporting

## Implementation Details

**Key Files:**
- `src/app/cfn_template.rs` - Template parsing, validation, and multi-format support
- `src/app/cfn_resources.rs` - AWS resource specification system with three-level caching
- `src/app/cfn_dag.rs` - Dependency graph management with smart resolution and cycle detection
- `src/app/cfn_intrinsic_functions.rs` - CloudFormation intrinsic function processing and classification
- `src/app/dashui/cloudformation_scene_graph.rs` - Interactive scene graph visualization
- `src/app/dashui/template_sections_window.rs` - Multi-section template management interface

**Template Processing Pipeline:**
```
Template File → Auto-detect Format → Parse Sections → Build DAG → Validate → Visualize
```

**Resource Specification Caching:**
- **Level 1**: Memory cache for active resources (immediate access)
- **Level 2**: Individual JSON files for resource types (fast partial loading)
- **Level 3**: Background downloads from AWS CloudFormation APIs with progress reporting

**Smart Dependency Resolution:**
```rust
// Deferred processing for out-of-order resource dependencies
impl ResourceDag {
    pub fn add_resource_smart(&mut self, name: String, resource: CloudFormationResource) -> Result<(), String> {
        if self.can_add_immediately(&resource) {
            self.add_resource_direct(name, resource)?;
        } else {
            self.deferred_queue.push((name, resource)); // Process later
        }
        self.process_deferred_queue()?;
        Ok(())
    }
}
```

**Configuration Requirements:**
- AWS CloudFormation specification URLs for multi-region support
- Local cache directory for resource specification storage (7-day expiration)
- Memory limits require `-j 7` flag for testing due to large concurrent processing

## Developer Notes

**Extension Points for Adding New AWS Services:**

1. **Create Service Module** in template parsing:
   ```rust
   // Add support for new AWS resource types
   match resource_type.as_str() {
       "AWS::NewService::Resource" => {
           validate_new_service_properties(&properties)?;
           extract_new_service_dependencies(&properties)
       }
   }
   ```

2. **Add Resource Specification Support**:
   ```rust
   // Extend specification cache to handle new service
   pub fn get_resource_specification(&self, resource_type: &str) -> Result<ResourceSpec> {
       // Three-level cache lookup with background updates
   }
   ```

3. **Extend Visualization**: Add AWS service colors and icons for new resource types

4. **Update Intrinsic Function Processing**: Handle service-specific function patterns

**Template Import/Export Workflow:**
- **Import**: JSON/YAML auto-detection → Parse sections → Build dependency graph → Validate references
- **Export**: Generate deployment order → Serialize sections → Preserve original formatting
- **Round-trip**: Ensure zero data loss through complete attribute preservation

**Architectural Decisions:**
- **Smart Resolution**: Handles out-of-order resource dependencies during template import
- **Three-Level Caching**: Balances performance with fresh AWS specification data
- **egui Scene Integration**: Native rendering performance with interactive node manipulation
- **Emergency Recovery**: Bypass validation for corrupted states to maintain application stability

**Performance Considerations:**
- Lazy loading of resource specifications reduces memory usage
- Background specification updates prevent UI blocking
- Chunked processing for large templates maintains responsiveness
- Efficient graph algorithms handle 1000+ resource templates

**References:**
- [Resource Explorer System](resource-explorer-system.md) - Shared AWS client functionality
- [Project Management](project-management.md) - Template organization and environment workflows
- [UI Testing Framework](ui-testing-framework.md) - Testing CloudFormation components