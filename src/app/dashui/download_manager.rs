use crate::app::cfn_resources::{CfnResourcesDownloader, DownloadStatus};
use eframe::egui;
use std::sync::mpsc::Receiver;

#[derive(Default)]
pub struct DownloadManager {
    pub download_status: Option<DownloadStatus>,
    pub download_receiver: Option<Receiver<DownloadStatus>>,
    pub resource_types_loaded: bool,
    pub auto_download_attempted: bool,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_download(&mut self) {
        if self.download_receiver.is_none() {
            let receiver = CfnResourcesDownloader::download_all_regions_async();
            self.download_receiver = Some(receiver);
        }
    }

    pub fn try_auto_download(&mut self) {
        // No longer automatically starts downloads at startup
        self.auto_download_attempted = true;
    }

    pub fn download_for_regions(&mut self, regions: Vec<String>) {
        if self.download_receiver.is_none() && !regions.is_empty() {
            let receiver =
                crate::app::cfn_resources::CfnResourcesDownloader::download_regions_async(regions);
            self.download_receiver = Some(receiver);
        }
    }

    pub fn update_download_status(&mut self) {
        if let Some(receiver) = &self.download_receiver {
            // Check for new status updates (non-blocking)
            match receiver.try_recv() {
                Ok(status) => {
                    self.download_status = Some(status.clone());
                    // Check if download is completed
                    if status.completed {
                        if status.error.is_none() {
                            self.resource_types_loaded = true;
                        } else {
                            println!("Download completed with error: {:?}", status.error);
                        }
                        self.download_receiver = None; // No longer need the receiver
                    }
                }
                Err(e) => {
                    // Only log TryRecvError::Disconnected as it indicates a problem
                    if let std::sync::mpsc::TryRecvError::Disconnected = e {
                        println!("Download channel disconnected unexpectedly");
                    }
                }
            }
        }
    }

    pub fn show_download_progress(&mut self, ui: &mut egui::Ui) {
        if let Some(status) = &self.download_status {
            if !status.completed {
                ui.heading("Downloading CloudFormation Resources");

                // Use a spinner instead of a progress bar
                ui.horizontal(|ui| {
                    ui.spinner();

                    // Show different message based on the download phase
                    let phase_message = match status.phase {
                        crate::app::cfn_resources::DownloadPhase::Specification => {
                            "Downloading resource specifications"
                        }
                        crate::app::cfn_resources::DownloadPhase::ResourceTypes => {
                            "Processing individual resource types"
                        }
                        crate::app::cfn_resources::DownloadPhase::Schemas => {
                            "Downloading and extracting resource provider schemas"
                        }
                        crate::app::cfn_resources::DownloadPhase::Complete => "Completing download",
                    };

                    ui.label(format!(
                        "Region {} of {}: {} for {}",
                        status.current_region, status.total_regions, phase_message, status.region
                    ));
                });
            } else if let Some(error) = &status.error {
                ui.heading("Download Error");
                ui.colored_label(egui::Color32::RED, error);

                // Add a retry button
                if ui.button("Retry Download").clicked() {
                    self.start_download();
                }
            }
        } else {
            ui.heading("CloudFormation Resources");

            // Add a manual download button
            if ui.button("Download Resource Specifications").clicked() {
                self.start_download();
            }
        }
    }
}
