//! Event handling for downloads, widget actions, and element activation

use super::super::DashApp;

impl DashApp {
    /// Handle download status updates (download manager removed)
    pub(super) fn handle_downloads(&mut self) {
        // Only update the download status, no auto-download
        // Download manager removed
    }

    /// Check for new validation results and display them in UI (removed)
    pub(super) fn handle_validation_results(&mut self) {
        // CloudFormation manager removed
    }

    /// Compliance validation feature removed
    pub(super) fn handle_validation_task_monitoring(&mut self) {
        // Compliance validation removed
    }

    /// Handle deployment task monitoring for async deployment operations
    pub(super) fn handle_deployment_task_monitoring(&mut self) {
        if let Some(task) = &self.pending_deployment_task {
            if task.is_finished() {
                let _completed_task = self.pending_deployment_task.take().unwrap();
                // Deployment monitoring removed - task is simply cleared
            }
        }
    }

    /// Initialize deployment status notifications when project loads (removed)
    pub(super) fn initialize_deployment_status_notifications(&mut self) {
        // Project management removed
    }

    /// Poll CloudFormation stack status and events for active deployments (removed)
    pub(super) fn poll_deployment_status(&mut self) {
        // Project management removed
    }

    /// Repository sync feature removed
    pub(super) fn update_repository_sync_status(&mut self, _ctx: &eframe::egui::Context) {
        // Guard repository system removed
    }
}
