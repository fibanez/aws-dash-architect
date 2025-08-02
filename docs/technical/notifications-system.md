# Notifications System

Comprehensive notification management system providing user feedback through multiple notification types with automatic expiration, persistent deployment status tracking, and detailed error reporting.

## Core Functionality

**Notification Types:**
- **Error**: Critical issues that require user attention (never auto-expire)
- **Warning**: Important alerts that auto-expire after 30 seconds
- **Info**: General information messages that auto-expire after 10 seconds
- **Success**: Positive feedback messages that auto-expire after 5 seconds
- **DeploymentStatus**: Persistent notifications for CloudFormation deployment tracking

**Key Features:**
- Automatic expiration handling with type-specific timeouts
- Status bar integration with clickable error/warning indicators
- Detailed notification window with error codes and contextual details
- Clipboard export functionality for error reporting
- Deployment status tracking with real-time polling indicators
- Source-based categorization (e.g., "CloudFormation Validation", "Compliance Check")
- Color-coded visual system with type-specific icons

**Main Components:**
- **NotificationManager**: Central management of all notifications with HashMap storage
- **Notification**: Core data structure with metadata, errors, and expiration logic
- **NotificationDetailsWindow**: Modal window for detailed error inspection
- **Status Bar Integration**: Real-time indicators in application status bar

**Integration Points:**
- CloudFormation Manager for deployment status notifications
- Status bar system for persistent visual feedback
- Clipboard system for error report sharing
- Window management system for notification details display

## Implementation Details

**Key Files:**
- `src/app/notifications/mod.rs` - Core notification types, manager, and status bar rendering
- `src/app/notifications/error_window.rs` - Detailed notification display window

**Notification Structure:**
```rust
pub struct Notification {
    pub id: String,
    pub title: String,
    pub notification_type: NotificationType,
    pub errors: Vec<NotificationError>,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
    pub dismissible: bool,
    pub source: String,
    pub deployment_data: Option<DeploymentNotificationData>,
}
```

**Expiration Timeouts:**
- **Error**: No expiration (persistent until dismissed)
- **Warning**: 30 seconds auto-expiration
- **Info**: 10 seconds auto-expiration  
- **Success**: 5 seconds auto-expiration
- **DeploymentStatus**: No expiration (persistent, updated in-place)

**Status Bar Rendering:**
- Real-time spinner for active deployments (`⟳` icon with egui::Spinner)
- Color-coded indicators: Red (errors), Orange (warnings), Blue (info/deployment)
- Clickable counters that open detailed notification windows
- Automatic cleanup of expired notifications during render

**Deployment Status Integration:**
- Environment-specific notification IDs (`deployment_status_{environment_name}`)
- In-place updates for existing deployment notifications
- Polling state management with visual indicators
- Persistent across application sessions

## Developer Notes

**Extension Points for Custom Notification Types:**

1. **Add New NotificationType**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum NotificationType {
       Error, Warning, Info, Success, DeploymentStatus,
       CustomType, // Add new type here
   }
   ```

2. **Implement Type-Specific Behavior**:
   ```rust
   // In Notification::get_color() and get_icon()
   NotificationType::CustomType => {
       // Define color and icon
   }
   ```

3. **Add Constructor Method**:
   ```rust
   impl Notification {
       pub fn new_custom_type(id: String, title: String, message: String, source: String) -> Self {
           // Custom expiration and dismissibility logic
       }
   }
   ```

**Source-Based Action Integration:**
- Add new source types in `NotificationDetailsWindow::show_notification_details()`
- Implement source-specific action buttons (e.g., "Fix Template", "View Report")
- Hook into relevant application systems for contextual actions

**Deployment Status Pattern:**
```rust
// Update deployment status
manager.update_deployment_status(
    environment_name,
    stack_name,
    deployment_id,
    message,
    is_polling
);

// Check deployment status
if let Some(status) = manager.get_deployment_status(environment_name) {
    // Handle existing deployment notification
}
```

**Architectural Decisions:**
- **HashMap Storage**: Fast lookup by notification ID for updates and dismissal
- **Instant-Based Expiration**: Precise timing without requiring background tasks
- **Clone-on-Display**: Notifications cloned for display to avoid borrow checker issues
- **Source Attribution**: Enables contextual actions and better user understanding
- **Persistent Deployment Status**: DeploymentStatus notifications never expire automatically

**Performance Considerations:**
- Expired notifications cleaned during status bar render (no background tasks)
- Deployment notification data collected once per render to avoid borrow conflicts
- Clipboard operations use efficient string building
- Status bar updates only when notifications exist

**References:**
- [CloudFormation Manager](cloudformation-manager.md) - Deployment status integration
- [Window Focus System](window-focus-system.md) - Notification detail window integration
- [User Interface](user-interface.md) - Status bar integration patterns