# Changelog

## [Unreleased] - 0.1.3

### December 2025
- **Agent Middleware System**: Extensible conversation middleware with logging, token tracking, and auto-analysis layers plus cancellation support
- **AWS Resource Expansion**: Added 27 new resource types for CloudWatch, CloudWatch Logs, EC2, and Lambda services
- **CLI Verification Framework**: Property comparison tool for validating SDK responses against AWS CLI output
- **Two-Phase Resource Loading**: Background enrichment for security details across 24 resource types with non-blocking UI updates
- **Cross-Platform Distribution**: GitHub Actions release workflow with Linux AppImage, macOS app bundle, and Windows builds
- **Unified Querying System**: Cross-service resource search with unified query API across 30+ AWS services
- **Agent Model Selection**: Type-safe model selection with AgentModel enum and markdown rendering for LLM responses

### November 2025
- **Multi-Agent Task Orchestration**: Implemented task orchestration system for coordinating multiple specialized agents
- **Login UX Improvements**: Fixed credential race condition and improved button layout with centered 4-column design
- **Documentation Cleanup**: Removed broken relative links and updated reference documentation structure
- **Performance**: Removed per-frame UI render logging that was causing excessive log output

- **Agent Framework V2**
  - Implemented simplified agent system using Stood library directly with lazy initialization and background execution
  - Integrated V8 JavaScript runtime with AWS API bindings for intelligent infrastructure operations

- **V8 JavaScript Execution Engine**
  - Complete V8 runtime integration with timeout enforcement, memory limits, and security sandboxing
  - Implemented V8 bindings for AWS operations: listAccounts(), listRegions(), queryResources()
  - Added CloudWatch Logs V8 binding for log stream queries and event retrieval
  - Implemented CloudWatch Logs client foundation with log group/stream discovery
  - Added CloudWatch Logs viewer window with real-time log event streaming
  - Integrated logs viewer with Agent Framework for automated log analysis

- **Advanced Resource Organization**
  - Developed bookmark folder system with hierarchical organization and drag-drop support
  - Implemented comprehensive tag filtering with presence/absence, value matching, and logical operators
  - Added tag hierarchy builder widget with visual grouping and auto-discovery
  - Created property-based filtering system with dynamic resource grouping

- **Resource Explorer Enhancements**
  - Expanded service coverage with service-specific tag implementations (ECS, EKS, CloudFront, KMS, SNS)

### October 2025
- **Agent Framework Rename**: Renamed "Bridge" component to "Agent Framework" for clarity
  - Internal naming: "Agent Framework" for code, comments, and internal documentation
  - User-facing naming: "Agent" or "Agent Control" for UI elements
  - Module renamed: `bridge` → `agent_framework`
  - Main agent: `BridgeAgent` → Agent V1 (removed) → Agent V2
  - UI window: `ControlBridgeWindow` → `AgentControlWindow`
  - Window title: "🚢 Control Bridge" → "🤖 Agent"
  - All documentation and references updated
  - Prepares codebase for future "Agent Harness" component

- **Major Architecture Refactoring**: Removed CloudFormation designer to focus on Agent Framework + Explorer
  - Deleted 66 files (43 source + 23 tests), removing 41,756+ lines of code
  - Removed CloudFormation template system, DAG analysis, and deployment manager
  - Removed Project Management system (multi-environment support)
  - Removed Git Repository Management and integration
  - Removed Bedrock Client and standalone Chat Window
  - Removed Compliance/Guard validation system
  - Removed all CloudFormation UI windows (scene graph, resource editors, property forms)
  - Created minimal `aws_regions` module for Agent Framework tools
  - Updated module documentation to reflect Agent Framework + Explorer architecture
  - Clean compilation with zero warnings

- **UI Simplification**: Streamlined user interface for resource exploration
  - Removed "No Project Loaded" screen, replaced with welcome message
  - Updated command palette: removed Project, CloudFormation, Graph View, Show Resources commands
  - Command palette now shows: Login (L), AWS Explorer (E), Agent (B), Quit (Q)
  - Updated help window with current keyboard shortcuts
  - Removed F1 reference and outdated CloudFormation getting started steps

### August 2025
- **DAG System Removal**: Complete architectural simplification by removing DAG persistence system
- **Documentation Migration**: Migrated all technical documentation from VimWiki to Markdown format  
- **Application Branding**: Added dash-icon.png and comprehensive README user guide
- **AWS Resource Icons**: Added complete AWS icon asset library with proper attribution

### Late July 2025
- **UI Navigation Fixes**: Fixed Project submenu and command palette organization
- **Login Experience**: Added spinner feedback during AWS Identity Center authentication
- **Performance Optimization**: Enhanced build system with memory-safe parallel testing
- **CloudFormation UI**: Replaced emoji spinners with proper egui widgets

### Early July 2025  
- **AWS Service Integration**: Achieved 100% AWS service coverage (Phase 2 complete)
- **Security Services**: Implemented WAFv2, GuardDuty, ACM, and Certificate Manager
- **Database Services**: Added Neptune, OpenSearch, ElastiCache support
- **Content Delivery**: Integrated CloudFront and comprehensive CDN management
- **Business Intelligence**: Added QuickSight and Batch processing services
- **Identity Management**: Implemented complete Cognito services

### Late June 2025
- **CloudFormation Deployment**: Successfully achieved end-to-end CloudFormation deployment
- **AWS Explorer Enhancement**: Fixed infinite render loops and performance issues  
- **Service Describe Coverage**: Completed 100% describe functionality across all AWS services
- **Enhanced Monitoring**: Implemented CloudWatch, Systems Manager, and Backup services

### Mid June 2025
- **CloudFormation Manager**: Complete parameter management with Parameter Store integration
- **Secrets Manager**: Full AWS Secrets Manager integration with dynamic references
- **Template Validation**: Real-time CloudFormation template validation using AWS APIs
- **AWS Resource Querying**: Parallel querying with real-time tree updates
- **Fuzzy Search System**: Character-level highlighting and advanced filtering
- **JSON Tree Viewer**: Expandable resource property inspection

### Early June 2025
- **AWS Explorer Foundation**: Comprehensive AWS resource discovery and management
- **UI Testing Framework**: Established egui_kittest testing infrastructure
- **Window Management**: Focusable window trait system with raised window support
- **Keyboard Navigation**: Vimium-like hint mode and space bar command palette

### Late May 2025
- **AWS Identity Center**: OAuth 2.0 device flow authentication with multi-account support
- **Core Architecture**: Trait-based window system and command palette foundation
- **CloudFormation Engine**: Template processing with dependency graph system
- **Project Management**: Multi-environment organization with file persistence
