# AWS Icon System

Dynamic AWS service icon loading and caching system providing visual resource representation through AWS Architecture Icons with intelligent fallback mechanisms and texture optimization for CloudFormation resources.

## Core Functionality

**Icon Management:**
- Comprehensive mapping of 500+ CloudFormation resource types to AWS Architecture Icons
- Dynamic texture loading and caching for egui-based UI rendering
- Service-specific fallback icons for unknown resource types
- Hierarchical fallback system (exact match → service match → generic fallback)
- AWS Architecture Icons compliance with official AWS icon pack usage terms

**Key Features:**
- Lazy texture loading with HashMap-based caching for performance optimization
- Service prefix matching for unmapped resource types (e.g., AWS::EC2::* → EC2 service icon)
- Programmatically generated fallback textures with color-coded service indicators
- Asset path management with organized directory structure (Architecture/Resource icons)
- Integration with CloudFormation scene graph for visual resource representation

**Main Components:**
- **AwsIconManager**: Texture loading and caching coordinator with egui integration
- **RESOURCE_ICONS**: Static HashMap mapping 500+ CloudFormation types to icon paths
- **ServiceFallbacks**: Generated fallback textures for service categories
- **IconPathResolver**: Smart matching algorithm for resource-to-icon mapping

**Integration Points:**
- CloudFormation Scene Graph for node visualization
- egui texture system for UI rendering
- Asset management system for icon file organization
- Resource type system for CloudFormation template processing

## Implementation Details

**Key Files:**
- `src/app/dashui/aws_icon_manager.rs` - Texture loading, caching, and egui integration
- `src/app/cfn_resource_icons.rs` - Resource-to-icon mapping with 500+ CloudFormation types
- `assets/Icons/` - Official AWS Architecture Icons organized by service category

**Icon Mapping Structure:**
```rust
pub static RESOURCE_ICONS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("AWS::EC2::Instance", "assets/Icons/.../Arch_Amazon-EC2_16.png");
    map.insert("AWS::S3::Bucket", "assets/Icons/.../Arch_Amazon-Simple-Storage-Service_16.png");
    // 500+ additional mappings
});
```

**Asset Directory Organization:**
- `Architecture-Service-Icons_02072025/` - Main AWS service icons (16x16px)
- `Resource-Icons_02072025/` - Specific resource type icons (48x48px)
- `Architecture-Group-Icons_02072025/` - Service group icons (32x32px)
- Service-specific subdirectories (Arch_Compute/, Arch_Database/, etc.)

**Texture Caching System:**
- HashMap-based cache keyed by icon file path
- Lazy loading on first access to minimize startup time
- Texture handles maintained for egui rendering pipeline
- Fallback texture generation for missing icons

**Fallback Hierarchy:**
1. **Exact Match**: Direct resource type lookup in RESOURCE_ICONS map
2. **Service Match**: Service prefix matching (AWS::EC2::* → EC2 service icon)  
3. **Service Fallback**: Generated texture with service-specific color coding
4. **Generic Fallback**: Default AWS cloud icon for unknown services

**Icon Resolution Algorithm:**
```rust
pub fn get_icon_for_resource(resource_type: &str) -> &'static str {
    // 1. Try exact match
    if let Some(icon_path) = RESOURCE_ICONS.get(resource_type) {
        return icon_path;
    }
    
    // 2. Try service prefix match
    let service_prefix = resource_type.split("::").take(2).collect::<Vec<_>>().join("::");
    // Additional matching logic...
}
```

## Developer Notes

**Extension Points for New Resource Types:**

1. **Add Direct Resource Mapping**:
   ```rust
   // In cfn_resource_icons.rs RESOURCE_ICONS map
   map.insert("AWS::NewService::NewResource", 
              "assets/Icons/Architecture-Service-Icons_02072025/Arch_NewService/16/Arch_AWS-NewService_16.png");
   ```

2. **Add Service Fallback**:
   ```rust
   // In AwsIconManager::create_service_fallbacks()
   self.service_fallbacks.insert(
       "NewService".to_string(),
       self.create_service_fallback_texture(ctx, "NewService", Color32::from_rgb(r, g, b))
   );
   ```

3. **Add New Icon Assets**:
   - Download official AWS Architecture Icons pack
   - Organize into appropriate service subdirectories
   - Use consistent naming convention (Arch_ServiceName_16.png)
   - Update resource mappings to reference new asset paths

**Integration Pattern for New UI Components:**
```rust
// Get texture for resource visualization
let texture = icon_manager.get_texture_for_resource(ctx, "AWS::EC2::Instance");

// Use in egui rendering
ui.image(texture, egui::Vec2::new(16.0, 16.0));
```

**Service Extraction Logic:**
- Extracts service name from CloudFormation resource type format
- Maps AWS service names to icon directory structures
- Handles special cases for service name variations
- Provides graceful degradation for unknown services

**Asset Management Considerations:**
- Icons organized by AWS-provided directory structure
- Consistent sizing (16px for services, 48px for resources)
- PNG format with transparency support
- Organized by service category for efficient browsing

**Performance Optimizations:**
- Lazy HashMap initialization using `once_cell::sync::Lazy`
- Texture caching prevents redundant file I/O operations
- Service prefix matching reduces exact mapping requirements
- Fallback texture generation minimizes asset dependencies

**Architectural Decisions:**
- **Static Mapping**: Compile-time resource-to-icon associations for performance
- **Hierarchical Fallbacks**: Multiple fallback levels ensure icons always available
- **Official AWS Icons**: Uses authentic AWS Architecture Icons for professional appearance
- **Cache-First**: Prioritizes performance through aggressive texture caching
- **Service Grouping**: Logical organization by AWS service categories

**Icon Generation Workflow:**
- Icons sourced from official AWS Architecture Icons pack (updated quarterly)
- Automated mapping generation available through Python script
- Asset path validation during build process
- Fallback texture generation for runtime resilience

**References:**
- [CloudFormation System](cloudformation-system.md) - Resource type integration
- [CloudFormation Scene Graph](cloudformation-system.md#visualization-system) - Icon usage in visualization
- [Performance Optimization](performance-optimization.md) - Texture caching strategies
- AWS Architecture Icons: https://aws.amazon.com/architecture/icons/ (official icon source)