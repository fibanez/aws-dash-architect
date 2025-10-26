# AWS Icon System

Dynamic AWS service icon loading and caching system providing visual resource representation through AWS Architecture Icons with intelligent fallback mechanisms and texture optimization for AWS resources.

## Core Functionality

**Icon Management:**
- Comprehensive mapping of AWS resource types to AWS Architecture Icons
- Dynamic texture loading and caching for egui-based UI rendering
- Service-specific fallback icons for unknown resource types
- Hierarchical fallback system (exact match → service match → generic fallback)
- AWS Architecture Icons compliance with official AWS icon pack usage terms

**Key Features:**
- Lazy texture loading with HashMap-based caching for performance optimization
- Service prefix matching for unmapped resource types (e.g., EC2 instance → EC2 service icon)
- Programmatically generated fallback textures with color-coded service indicators
- Asset path management with organized directory structure (Architecture/Resource icons)
- Integration with Resource Explorer for visual resource representation

**Main Components:**
- **Icon Loading**: Texture loading and caching with egui integration
- **Service Icons**: Mapping of AWS services to icon paths
- **ServiceFallbacks**: Generated fallback textures for service categories
- **IconPathResolver**: Smart matching algorithm for resource-to-icon mapping

**Integration Points:**
- Resource Explorer for resource visualization
- egui texture system for UI rendering
- Asset management system for icon file organization

## Implementation Details

**Key Files:**
- `assets/Icons/` - Official AWS Architecture Icons organized by service category

**Icon Usage Pattern:**
- AWS service icons used throughout the Resource Explorer
- Icons loaded dynamically as needed for resource types
- Service-based icon selection with fallback support

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
1. **Service Match**: Service-based icon matching (EC2 → EC2 service icon)
2. **Service Fallback**: Generated texture with service-specific color coding
3. **Generic Fallback**: Default AWS cloud icon for unknown services

**Icon Resolution Pattern:**
- Icons resolved based on AWS service name
- Service-specific icons loaded from official AWS Architecture Icons
- Fallback textures generated for unknown services

## Developer Notes

**Extension Points for New Services:**

1. **Add New Icon Assets**:
   - Download official AWS Architecture Icons pack
   - Organize into appropriate service subdirectories
   - Use consistent naming convention (Arch_ServiceName_16.png)
   - Update service mappings to reference new asset paths

2. **Service Icon Integration**:
   - Icons organized by AWS service category
   - Service-based matching for resource visualization
   - Fallback generation for new services

**Integration Pattern for UI Components:**
- Icons used in Resource Explorer for resource visualization
- Service-based icon selection
- Dynamic loading with caching for performance

**Service Matching Logic:**
- Maps AWS service names to icon directory structures
- Handles service name variations
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
- **Hierarchical Fallbacks**: Multiple fallback levels ensure icons always available
- **Official AWS Icons**: Uses authentic AWS Architecture Icons for professional appearance
- **Cache-First**: Prioritizes performance through aggressive texture caching
- **Service Grouping**: Logical organization by AWS service categories

**Icon Source:**
- Icons sourced from official AWS Architecture Icons pack (updated quarterly)
- Asset path validation for icon availability
- Fallback texture generation for runtime resilience

**References:**
- [Resource Explorer System](resource-explorer-system.md) - Icon usage in resource visualization
- AWS Architecture Icons: https://aws.amazon.com/architecture/icons/ (official icon source)