//! # AWS CloudFormation Resource Specifications Management
//!
//! This module provides comprehensive management of AWS CloudFormation resource specifications,
//! including automated downloading, caching, and parsing of resource definitions from AWS.
//! It serves as the foundation for dynamic form generation and validation throughout the application.
//!
//! ## Core Functionality
//!
//! The module offers several key capabilities:
//!
//! * **Automated Downloads**: Downloads CloudFormation resource specifications for all AWS regions
//! * **Multi-Level Caching**: Implements aggressive caching at multiple levels for optimal performance
//! * **Schema Validation**: Parses schema constraints for enhanced property validation
//! * **Thread Safety**: All operations are thread-safe with proper synchronization
//! * **Resource Discovery**: Discovers and loads resource types, property types, and attributes
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Application Layer                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Resource Forms  │  Property Forms  │  Validation Engine   │
//! ├─────────────────────────────────────────────────────────────┤
//! │                   CFN Resources Module                      │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
//! │  │  Memory Cache   │  │  Definition     │  │  Download   │ │
//! │  │  (HashMap)      │  │  Parsers        │  │  Manager    │ │
//! │  └─────────────────┘  └─────────────────┘  └─────────────┘ │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    Local File System                       │
//! │  ~/.config/awsdash/cfn-resources/{region}/                  │
//! │    ├── CloudFormationResourceSpecification.json            │
//! │    ├── resources/*.json (individual resources)             │
//! │    └── schemas/*.json (validation schemas)                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Performance Optimizations
//!
//! The module implements several performance optimizations:
//!
//! * **Three-Level Caching**:
//!   - Memory cache for immediate access to parsed definitions
//!   - Individual resource files for faster partial loading
//!   - Combined specification files as fallback
//!
//! * **Lazy Loading**: Resource definitions are loaded only when needed
//! * **Background Downloads**: Downloads run in background threads with status reporting
//! * **Incremental Updates**: Only downloads resources when older than 7 days
//!
//! ## Integration Points
//!
//! This module integrates with several other components:
//!
//! * **Form Generation**: Provides resource and property definitions for dynamic UI forms
//! * **Validation Engine**: Supplies schema constraints for real-time validation
//! * **Resource Discovery**: Powers command palettes and resource browsers
//! * **Template Parsing**: Validates resource references and property types
//!
//! ## Example Usage
//!
//! ```rust
//! use crate::app::cfn_resources::{CfnResourcesDownloader, load_resource_types};
//!
//! // Download specifications for a region
//! let downloader = CfnResourcesDownloader::new()?;
//! let rx = CfnResourcesDownloader::download_single_region_async("us-east-1");
//!
//! // Load resource types for form generation
//! let resource_types = load_resource_types("us-east-1")?;
//! for resource_type in resource_types.keys() {
//!     println!("Available resource: {}", resource_type);
//! }
//!
//! // Load property definitions for validation
//! let properties = load_property_definitions("us-east-1", "AWS::EC2::Instance")?;
//! for (prop_name, prop_def) in properties {
//!     if prop_def.required {
//!         println!("Required property: {} ({})", prop_name, prop_def.primitive_type.unwrap_or("Complex".to_string()));
//!     }
//! }
//! ```
//!
//! ## Error Handling
//!
//! The module uses `anyhow::Result` for comprehensive error handling:
//!
//! * Network failures during downloads are handled gracefully with fallback URLs
//! * Missing specifications trigger helpful error messages
//! * Cache corruption is automatically recovered by reloading from disk
//! * Thread synchronization errors are properly propagated

use crate::log_trace;
use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use log::{debug, info, warn};
use once_cell::sync::{Lazy, OnceCell};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;

/// Schema constraints parsed from CloudFormation property schema.
///
/// This struct captures validation constraints that can be applied to CloudFormation
/// resource properties during form generation and validation. These constraints are
/// extracted from the CloudFormation schema files and provide rich validation
/// capabilities beyond basic type checking.
///
/// ## Usage in Form Generation
///
/// ```rust
/// use crate::app::cfn_resources::SchemaConstraints;
///
/// let constraints = SchemaConstraints {
///     enum_values: Some(vec!["t2.micro".to_string(), "t2.small".to_string()]),
///     pattern: Some(r"^t[2-3]\.(micro|small|medium)$".to_string()),
///     min_length: Some(1),
///     max_length: Some(20),
///     ..Default::default()
/// };
///
/// // Use constraints to generate appropriate UI widgets
/// if let Some(enum_vals) = &constraints.enum_values {
///     // Generate dropdown/combo box
/// } else if constraints.pattern.is_some() {
///     // Generate text field with pattern validation
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SchemaConstraints {
    /// Valid enumeration values for the property (generates dropdown UI)
    pub enum_values: Option<Vec<String>>,
    /// Regular expression pattern that must be matched (for text validation)
    pub pattern: Option<String>,
    /// Minimum string length constraint
    pub min_length: Option<usize>,
    /// Maximum string length constraint
    pub max_length: Option<usize>,
    /// Minimum numeric value constraint
    pub min_value: Option<f64>,
    /// Maximum numeric value constraint
    pub max_value: Option<f64>,
    /// Whether array items must be unique
    pub unique_items: Option<bool>,
}
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;
use zip::ZipArchive;

/// Cache for resource type definitions by region.
/// Maps region names to complete resource type specifications.
type ResourceTypeCache = HashMap<String, ResourceTypeMap>;

/// Cache for property definitions by region and resource type.
/// Maps cache keys (format: "region:resource_type") to property definitions.
type PropertyDefinitionCache = HashMap<String, PropertyDefinitionMap>;

/// Cache for attribute definitions by region and resource type.
/// Maps cache keys (format: "region:resource_type") to attribute definitions.
type AttributeDefinitionCache = HashMap<String, AttributeDefinitionMap>;

/// Thread-safe global cache for resource type definitions.
///
/// This cache provides immediate access to resource type specifications without
/// requiring disk I/O on repeated access. The cache is populated lazily as
/// resource types are requested and persists for the lifetime of the application.
///
/// **Thread Safety**: Protected by Mutex, safe for concurrent access from multiple threads.
static RESOURCE_TYPE_CACHE: Lazy<Mutex<ResourceTypeCache>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Thread-safe global cache for property definitions.
///
/// Caches property definitions for individual resource types to avoid
/// re-parsing specification files. Uses composite keys combining region
/// and resource type for efficient lookup.
///
/// **Thread Safety**: Protected by Mutex, safe for concurrent access from multiple threads.
static PROPERTY_CACHE: Lazy<Mutex<PropertyDefinitionCache>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Thread-safe global cache for attribute definitions.
///
/// Caches attribute definitions (return values) for resource types.
/// Attributes define what values can be referenced from a resource using
/// the Fn::GetAtt intrinsic function.
///
/// **Thread Safety**: Protected by Mutex, safe for concurrent access from multiple threads.
static ATTRIBUTE_CACHE: Lazy<Mutex<AttributeDefinitionCache>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Complete list of all AWS regions supported by CloudFormation.
///
/// This list includes all commercial, government, and international AWS regions
/// where CloudFormation resource specifications are available. Used for
/// iterating through regions during bulk downloads and validation.
///
/// **Note**: This list is periodically updated as AWS adds new regions.
pub static AWS_REGIONS: &[&str] = &[
    "us-east-1",      // US East (N. Virginia)
    "us-east-2",      // US East (Ohio)
    "us-west-1",      // US West (N. California)
    "us-west-2",      // US West (Oregon)
    "af-south-1",     // Africa (Cape Town)
    "ap-east-1",      // Asia Pacific (Hong Kong)
    "ap-south-1",     // Asia Pacific (Mumbai)
    "ap-northeast-1", // Asia Pacific (Tokyo)
    "ap-northeast-2", // Asia Pacific (Seoul)
    "ap-northeast-3", // Asia Pacific (Osaka)
    "ap-southeast-1", // Asia Pacific (Singapore)
    "ap-southeast-2", // Asia Pacific (Sydney)
    "ap-southeast-3", // Asia Pacific (Jakarta)
    "ap-southeast-4", // Asia Pacific (Melbourne)
    "ap-south-2",     // Asia Pacific (Hyderabad)
    "ca-central-1",   // Canada (Central)
    "ca-west-1",      // Canada West (Calgary)
    "eu-central-1",   // Europe (Frankfurt)
    "eu-central-2",   // Europe (Zurich)
    "eu-west-1",      // Europe (Ireland)
    "eu-west-2",      // Europe (London)
    "eu-west-3",      // Europe (Paris)
    "eu-north-1",     // Europe (Stockholm)
    "eu-south-1",     // Europe (Milan)
    "eu-south-2",     // Europe (Spain)
    "il-central-1",   // Israel (Tel Aviv)
    "me-central-1",   // Middle East (UAE)
    "me-south-1",     // Middle East (Bahrain)
    "sa-east-1",      // South America (São Paulo)
    "us-gov-east-1",  // AWS GovCloud (US-East)
    "us-gov-west-1",  // AWS GovCloud (US-West)
];

/// Global data directory path for CloudFormation resource specifications.
///
/// This is initialized exactly once during the first access using `OnceCell`,
/// ensuring thread-safe initialization without runtime overhead on subsequent accesses.
/// The directory is typically located at `~/.config/awsdash/cfn-resources/`.
///
/// **Thread Safety**: OnceCell provides thread-safe initialization and access.
static DATA_DIR: OnceCell<PathBuf> = OnceCell::new();

/// Status information for download operations.
///
/// Provides real-time feedback during CloudFormation specification downloads,
/// enabling progress tracking and error reporting in the UI. Used with
/// `mpsc::Receiver` channels to communicate download progress from background threads.
///
/// ## Usage Example
///
/// ```rust
/// let rx = CfnResourcesDownloader::download_single_region_async("us-east-1");
/// while let Ok(status) = rx.recv() {
///     println!("Downloading {}: {} ({}/{})",
///              status.region,
///              status.phase,
///              status.current_region,
///              status.total_regions);
///
///     if status.completed {
///         if let Some(error) = status.error {
///             eprintln!("Download failed: {}", error);
///         } else {
///             println!("Download completed successfully");
///         }
///         break;
///     }
/// }
/// ```
#[derive(Clone)]
pub struct DownloadStatus {
    /// Name of the AWS region currently being processed
    pub region: String,
    /// Total number of regions in the download operation
    pub total_regions: usize,
    /// Current region number (1-based index)
    pub current_region: usize,
    /// Whether the entire download operation has completed
    pub completed: bool,
    /// Error message if the download failed
    pub error: Option<String>,
    /// Current phase of the download process
    pub phase: DownloadPhase,
}

/// Phases of the CloudFormation specification download process.
///
/// Each download operation progresses through multiple phases, allowing
/// for granular progress reporting and early cancellation if needed.
#[derive(Clone, PartialEq)]
pub enum DownloadPhase {
    /// Downloading the main CloudFormation resource specification file
    Specification,
    /// Downloading individual resource type specification files
    ResourceTypes,
    /// Downloading JSON schema files for validation
    Schemas,
    /// All download phases completed successfully
    Complete,
}

impl DownloadStatus {
    pub fn new() -> Self {
        Self {
            region: String::new(),
            total_regions: AWS_REGIONS.len(),
            current_region: 0,
            completed: false,
            error: None,
            phase: DownloadPhase::Specification,
        }
    }
}

impl Default for DownloadStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// CloudFormation resource specification downloader and manager.
///
/// This is the primary interface for downloading and managing CloudFormation
/// resource specifications from AWS. It handles the complete download process
/// including retry logic, file management, and progress reporting.
///
/// ## Download Strategy
///
/// The downloader implements a multi-layered approach:
///
/// 1. **Combined Specifications**: Downloads the main specification file containing all resources
/// 2. **Individual Resources**: Downloads separate files for each resource type (faster partial loading)
/// 3. **Schema Files**: Downloads JSON schema files for enhanced validation
/// 4. **Incremental Updates**: Only downloads files older than 7 days unless forced
/// 5. **Fallback URLs**: Tries multiple AWS endpoints if primary URLs fail
///
/// ## Thread Safety
///
/// The downloader can be safely used from multiple threads. File operations use
/// atomic writes and the underlying HTTP client is thread-safe.
///
/// ## Example Usage
///
/// ```rust
/// use crate::app::cfn_resources::CfnResourcesDownloader;
///
/// // Create a new downloader
/// let downloader = CfnResourcesDownloader::new()?;
///
/// // Download specifications for a specific region
/// downloader.download_region("us-east-1", false)?;
///
/// // Download asynchronously with progress tracking
/// let rx = CfnResourcesDownloader::download_single_region_async("us-west-2");
/// // ... handle progress updates
/// ```
pub struct CfnResourcesDownloader {
    /// HTTP client for downloading specifications from AWS
    client: Client,
    /// Local directory where specifications are stored
    data_dir: PathBuf,
}

impl CfnResourcesDownloader {
    /// Creates a new downloader instance with HTTP client and data directory setup.
    ///
    /// This method initializes the data directory (typically `~/.config/awsdash/cfn-resources/`)
    /// and creates a configured HTTP client with appropriate timeout settings.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The project directories cannot be determined (missing home directory)
    /// - Directory creation fails due to permissions
    /// - HTTP client initialization fails
    ///
    /// ## Example
    ///
    /// ```rust
    /// use crate::app::cfn_resources::CfnResourcesDownloader;
    ///
    /// match CfnResourcesDownloader::new() {
    ///     Ok(downloader) => {
    ///         // Ready to download specifications
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to initialize downloader: {}", e);
    ///     }
    /// }
    /// ```
    pub fn new() -> Result<Self> {
        // Initialize the data directory
        Self::init_data_dir()?;

        // Get the data directory (will be available since we initialized it)
        let data_dir = DATA_DIR.get().expect("DATA_DIR not initialized").clone();

        // Create the HTTP client
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Print the data directory to help with debugging
        eprintln!("CFN Resources data directory: {:?}", data_dir);

        Ok(Self { client, data_dir })
    }

    /// Initialize the data directory (only done once)
    fn init_data_dir() -> Result<()> {
        // If DATA_DIR is already set, return early
        if DATA_DIR.get().is_some() {
            return Ok(());
        }

        // Try to initialize the data directory
        if let Some(proj_dirs) = ProjectDirs::from("com", "", "awsdash") {
            let data_dir = proj_dirs.config_dir().join("cfn-resources");
            info!("Creating CFN resources directory at: {:?}", data_dir);
            eprintln!("Creating CFN resources directory at: {:?}", data_dir);

            // Make sure parent directories exist
            if let Some(parent) = data_dir.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    let err_msg = format!("Failed to create parent directory: {}", e);
                    eprintln!("{}", err_msg);
                    info!("{}", err_msg);
                    return Err(anyhow!(err_msg));
                }
            }

            if let Err(e) = fs::create_dir_all(&data_dir) {
                let err_msg = format!("Failed to create data directory: {}", e);
                eprintln!("{}", err_msg);
                info!("{}", err_msg);
                return Err(anyhow!(err_msg));
            }

            let success_msg = format!("Successfully created data directory: {:?}", data_dir);
            eprintln!("{}", success_msg);
            info!("{}", success_msg);

            // Store the data directory in the OnceCell
            match DATA_DIR.set(data_dir) {
                Ok(_) => {
                    let dir = DATA_DIR.get().unwrap();
                    let msg = format!("Using data directory: {:?}", dir);
                    eprintln!("{}", msg);
                    info!("{}", msg);
                    Ok(())
                }
                Err(_) => {
                    // This can only happen if another thread initialized DATA_DIR first
                    // In that case, we can still proceed using the value set by the other thread
                    Ok(())
                }
            }
        } else {
            let err_msg = "Could not determine project directories";
            eprintln!("{}", err_msg);
            info!("{}", err_msg);
            Err(anyhow!(err_msg))
        }
    }

    /// Get the data directory path
    pub fn get_data_dir() -> Result<PathBuf> {
        // If not initialized yet, initialize it
        if DATA_DIR.get().is_none() {
            Self::init_data_dir()?;
        }

        // Now we should have a value
        match DATA_DIR.get() {
            Some(dir) => Ok(dir.clone()),
            None => Err(anyhow!("Data directory not initialized")),
        }
    }

    /// Check if the data directory has been initialized
    pub fn is_data_dir_initialized() -> bool {
        DATA_DIR.get().is_some()
    }

    /// Downloads CloudFormation specifications for all AWS regions asynchronously.
    ///
    /// Starts a background thread that downloads specifications for all supported
    /// AWS regions. Progress is reported through the returned channel, allowing
    /// the UI to display real-time download status.
    ///
    /// ## Returns
    ///
    /// Returns an `mpsc::Receiver<DownloadStatus>` that provides status updates:
    /// - Current region being processed
    /// - Download phase (Specification, ResourceTypes, Schemas, Complete)
    /// - Progress counters (current/total regions)
    /// - Error information if downloads fail
    ///
    /// ## Performance Notes
    ///
    /// This downloads specifications for 30+ regions and can take several minutes
    /// on slower connections. Consider using `download_single_region_async()` for
    /// better user experience if only specific regions are needed.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use crate::app::cfn_resources::{CfnResourcesDownloader, DownloadPhase};
    ///
    /// let rx = CfnResourcesDownloader::download_all_regions_async();
    ///
    /// while let Ok(status) = rx.recv() {
    ///     match status.phase {
    ///         DownloadPhase::Specification => {
    ///             println!("Downloading main spec for {}", status.region);
    ///         }
    ///         DownloadPhase::Complete => {
    ///             if status.completed {
    ///                 println!("All regions downloaded successfully!");
    ///                 break;
    ///             }
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn download_all_regions_async() -> mpsc::Receiver<DownloadStatus> {
        Self::download_regions_async(AWS_REGIONS.iter().map(|r| r.to_string()).collect())
    }

    /// Download CloudFormation resource specifications for specific regions (background thread version)
    pub fn download_regions_async(regions: Vec<String>) -> mpsc::Receiver<DownloadStatus> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut status = DownloadStatus::new();

            // Create the downloader
            match CfnResourcesDownloader::new() {
                Ok(downloader) => {
                    status.total_regions = regions.len();

                    for (i, region) in regions.iter().enumerate() {
                        status.current_region = i + 1;
                        status.region = region.clone();
                        status.phase = DownloadPhase::Specification;
                        tx.send(status.clone()).unwrap_or_default();

                        if let Err(e) =
                            downloader.download_region_with_status(region, false, |phase| {
                                // Update the status with the current phase
                                let mut phase_status = status.clone();
                                phase_status.phase = phase;
                                tx.send(phase_status).unwrap_or_default();
                            })
                        {
                            eprintln!(
                                "Failed to download specifications for region {}: {}",
                                region, e
                            );
                            status.error = Some(format!("Failed on {}: {}", region, e));
                            tx.send(status.clone()).unwrap_or_default();
                        }
                    }

                    status.completed = true;
                    status.phase = DownloadPhase::Complete;
                    tx.send(status).unwrap_or_default();
                }
                Err(e) => {
                    eprintln!("Failed to create downloader: {}", e);
                    status.error = Some(format!("Failed to initialize: {}", e));
                    status.completed = true;
                    tx.send(status).unwrap_or_default();
                }
            }
        });

        rx
    }

    /// Download CloudFormation resource specifications for a single region (background thread version)
    pub fn download_single_region_async(region: &'static str) -> mpsc::Receiver<DownloadStatus> {
        let (tx, rx) = mpsc::channel();

        eprintln!("Creating download thread for region: {}", region);

        thread::spawn(move || {
            eprintln!("Download thread started for region: {}", region);
            let mut status = DownloadStatus::new();
            status.total_regions = 1;
            status.current_region = 1;
            status.region = region.to_string();

            // Send initial status
            info!("Starting download for region: {}", region);
            tx.send(status.clone()).unwrap_or_default();

            // Create the downloader
            match CfnResourcesDownloader::new() {
                Ok(downloader) => {
                    info!(
                        "Downloader created successfully, starting download for {}",
                        region
                    );
                    if let Err(e) = downloader.download_region(region, false) {
                        let error_msg = format!("Failed on {}: {}", region, e);
                        eprintln!("{}", error_msg);
                        info!("{}", error_msg);
                        status.error = Some(error_msg);
                    } else {
                        info!("Successfully downloaded specifications for {}", region);
                    }

                    status.completed = true;
                    tx.send(status).unwrap_or_default();
                }
                Err(e) => {
                    let error_msg = format!("Failed to initialize downloader: {}", e);
                    eprintln!("{}", error_msg);
                    info!("{}", error_msg);
                    status.error = Some(error_msg);
                    status.completed = true;
                    tx.send(status).unwrap_or_default();
                }
            }
        });

        eprintln!("Returning receiver for region: {}", region);
        rx
    }

    /// Download CloudFormation resource specifications for a specific region with status callback
    pub fn download_region_with_status<F>(
        &self,
        region: &str,
        force_refresh: bool,
        mut status_callback: F,
    ) -> Result<()>
    where
        F: FnMut(DownloadPhase),
    {
        info!("Processing region: {}", region);

        // Call the original download_region method with status callback
        self.download_region_internal(region, force_refresh, &mut status_callback)
    }

    /// Download CloudFormation resource specifications for a specific region
    pub fn download_region(&self, region: &str, force_refresh: bool) -> Result<()> {
        // Call the internal method with a no-op status callback
        self.download_region_internal(region, force_refresh, &mut |_| {})
    }

    /// Internal implementation of download_region that accepts a status callback
    fn download_region_internal<F>(
        &self,
        region: &str,
        force_refresh: bool,
        status_callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(DownloadPhase),
    {
        info!("Processing region: {}", region);

        // Create region directory
        let region_dir = self.data_dir.join(region);
        fs::create_dir_all(&region_dir)
            .with_context(|| format!("Failed to create directory for region {}", region))?;

        // Check if specifications already exist and are recent enough
        let spec_file = region_dir.join("CloudFormationResourceSpecification.json");
        let schema_dir = region_dir.join("schemas");

        if !force_refresh && spec_file.exists() {
            if let Ok(metadata) = fs::metadata(&spec_file) {
                if let Ok(modified) = metadata.modified() {
                    // Only download if file is older than 7 days
                    if modified
                        .elapsed()
                        .map(|e| e.as_secs() < 7 * 24 * 60 * 60)
                        .unwrap_or(false)
                    {
                        info!(
                            "Skipping download for {} (file is less than 7 days old)",
                            region
                        );

                        // Even if we skip the main spec, check if we need to download the schemas
                        if !schema_dir.exists()
                            || schema_dir
                                .read_dir()
                                .map(|d| d.count() == 0)
                                .unwrap_or(true)
                        {
                            // Update status to schemas phase
                            status_callback(DownloadPhase::Schemas);

                            // Schemas don't exist or directory is empty, download them
                            if let Err(e) = self.download_provider_schemas(region, &region_dir) {
                                warn!("Could not download provider schemas for {}: {}", region, e);
                            }
                        }

                        // Mark as complete
                        status_callback(DownloadPhase::Complete);
                        return Ok(());
                    }
                }
            }
        }

        // Status: Downloading main specification
        status_callback(DownloadPhase::Specification);

        // URL for the combined specification file
        let combined_url = format!(
            "https://cfn-resource-specifications-{}-prod.s3.amazonaws.com/latest/CloudFormationResourceSpecification.json",
            region
        );

        // Alternative URL formats to try if the main one fails
        let alt_urls = vec![
            format!("https://d3teyb21fexa9r.cloudfront.net/latest/CloudFormationResourceSpecification-{}.json", region),
            format!("https://cfn-resource-specifications-{}-prod.s3.{}.amazonaws.com/latest/CloudFormationResourceSpecification.json", region, region),
        ];

        // Download combined specification file
        info!("Downloading combined specification for {}", region);
        match self.download_file_with_alternatives(&combined_url, &alt_urls, &spec_file) {
            Ok(()) => {
                info!(
                    "Successfully downloaded combined specification for {}",
                    region
                );

                // Status: Downloading resource types
                status_callback(DownloadPhase::ResourceTypes);

                // Try to download individual resource type files
                match self.download_individual_specs(region, &region_dir) {
                    Ok(()) => info!(
                        "Successfully downloaded individual specifications for {}",
                        region
                    ),
                    Err(e) => warn!(
                        "Could not download individual specifications for {}: {}",
                        region, e
                    ),
                }

                // Status: Downloading schemas
                status_callback(DownloadPhase::Schemas);

                // Also download CloudFormation Resource Provider schemas
                match self.download_provider_schemas(region, &region_dir) {
                    Ok(()) => info!("Successfully downloaded provider schemas for {}", region),
                    Err(e) => warn!("Could not download provider schemas for {}: {}", region, e),
                }

                // Status: Completed
                status_callback(DownloadPhase::Complete);

                Ok(())
            }
            Err(e) => Err(anyhow!(
                "Failed to download specifications for {}: {}",
                region,
                e
            )),
        }
    }

    /// Download a file with alternatives if the main URL fails
    fn download_file_with_alternatives(
        &self,
        url: &str,
        alt_urls: &[String],
        output_path: &Path,
    ) -> Result<()> {
        // Try the main URL first
        match self.download_file(url, output_path) {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("Primary URL failed ({}), trying alternatives", e);

                // Try alternative URLs
                for alt_url in alt_urls {
                    match self.download_file(alt_url, output_path) {
                        Ok(()) => return Ok(()),
                        Err(e) => debug!("Alternative URL failed: {}", e),
                    }

                    // Add a small delay between requests
                    std::thread::sleep(Duration::from_millis(100));
                }

                // If we got here, all URLs failed
                Err(anyhow!("All download attempts failed"))
            }
        }
    }

    /// Download a file from a URL
    fn download_file(&self, url: &str, output_path: &Path) -> Result<()> {
        // Send GET request to the URL
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("Failed to send request to {}", url))?;

        // Check if the request was successful
        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }

        // Get the response body as bytes
        let content = response.bytes().context("Failed to get response body")?;

        // Write the bytes to the output file
        fs::write(output_path, content)
            .with_context(|| format!("Failed to write to file {:?}", output_path))?;

        Ok(())
    }

    /// Download and unzip CloudFormation Resource Provider schemas
    fn download_provider_schemas(&self, region: &str, region_dir: &Path) -> Result<()> {
        info!(
            "Downloading CloudFormation Resource Provider schemas for {}",
            region
        );

        // Create a directory for schemas
        let schemas_dir = region_dir.join("schemas");
        fs::create_dir_all(&schemas_dir).context("Failed to create schemas directory")?;

        // URL for the CloudFormation Resource Provider schemas zip file
        let schemas_url = format!(
            "https://schema.cloudformation.{}.amazonaws.com/CloudformationSchema.zip",
            region
        );

        // Alternative URLs for schemas
        let alt_schemas_urls = vec![
            format!("https://cfn-resource-specifications-{}-prod.s3.amazonaws.com/latest/CloudFormationSchema.zip", region),
            format!("https://d3teyb21fexa9r.cloudfront.net/latest/CloudformationSchema-{}.zip", region),
        ];

        // Temporary zip file path
        let zip_file_path = region_dir.join("CloudformationSchema.zip");

        // Download the schemas zip file
        match self.download_file_with_alternatives(&schemas_url, &alt_schemas_urls, &zip_file_path)
        {
            Ok(()) => {
                info!("Successfully downloaded schemas zip for {}", region);

                // Unzip the schemas
                match self.unzip_schemas(&zip_file_path, &schemas_dir) {
                    Ok(()) => {
                        info!("Successfully unzipped schemas for {}", region);

                        // Remove the zip file after extraction
                        if let Err(e) = fs::remove_file(&zip_file_path) {
                            warn!("Could not remove temporary zip file: {}", e);
                        }

                        Ok(())
                    }
                    Err(e) => Err(anyhow!("Failed to unzip schemas for {}: {}", region, e)),
                }
            }
            Err(e) => Err(anyhow!("Failed to download schemas for {}: {}", region, e)),
        }
    }

    /// Unzip CloudFormation Resource Provider schemas
    fn unzip_schemas(&self, zip_path: &Path, output_dir: &Path) -> Result<()> {
        // Open the zip file
        let file = fs::File::open(zip_path).context("Failed to open zip file")?;
        let mut archive = ZipArchive::new(file).context("Failed to parse zip file")?;

        // Extract each file in the archive
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("Failed to access zip entry")?;
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir.join(path),
                None => continue,
            };

            // Create parent directories if needed
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).context("Failed to create directory for schema file")?;
            }

            // If it's a directory, create it and continue
            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath).context("Failed to create directory from zip")?;
                continue;
            }

            // Create the output file
            let mut outfile = fs::File::create(&outpath)
                .with_context(|| format!("Failed to create file {}", outpath.display()))?;

            // Copy the contents
            io::copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to write to {}", outpath.display()))?;
        }

        Ok(())
    }

    /// Download individual resource type specifications
    fn download_individual_specs(&self, region: &str, region_dir: &Path) -> Result<()> {
        // Create directory for individual resource type files
        let resources_dir = region_dir.join("resources");
        fs::create_dir_all(&resources_dir).context("Failed to create resources directory")?;

        // URL for the resource types index file
        let resource_index_url = format!(
            "https://cfn-resource-specifications-{}-prod.s3.amazonaws.com/latest/resource-types.json",
            region
        );

        // Alternative URLs for resource types index
        let alt_index_urls = vec![
            format!("https://d3teyb21fexa9r.cloudfront.net/latest/resource-types-{}.json", region),
            format!("https://cfn-resource-specifications-{}-prod.s3.{}.amazonaws.com/latest/resource-types.json", region, region),
        ];

        // Try to download the resource types index
        info!("Downloading resource types index for {}", region);
        let index_path = resources_dir.join("resource-types.json");

        match self.download_file_with_alternatives(
            &resource_index_url,
            &alt_index_urls,
            &index_path,
        ) {
            Ok(()) => {
                info!(
                    "Successfully downloaded resource types index for {}",
                    region
                );

                // Read and parse the index file
                let index_content = fs::read_to_string(&index_path).with_context(|| {
                    format!("Failed to read resource types index for {}", region)
                })?;

                let resource_urls: Value =
                    serde_json::from_str(&index_content).with_context(|| {
                        format!("Failed to parse resource types index for {}", region)
                    })?;

                if let Some(resource_types) = resource_urls.as_object() {
                    info!(
                        "Found {} resource types for {}",
                        resource_types.len(),
                        region
                    );

                    // Download each resource type file
                    for (resource_name, resource_url) in resource_types {
                        if let Some(url) = resource_url.as_str() {
                            // Create a safe filename from the resource name
                            let filename = resource_name.replace("::", "-") + ".json";
                            let resource_path = resources_dir.join(&filename);

                            // Skip if file already exists and is recent
                            if resource_path.exists() {
                                if let Ok(metadata) = fs::metadata(&resource_path) {
                                    if let Ok(modified) = metadata.modified() {
                                        if modified
                                            .elapsed()
                                            .map(|e| e.as_secs() < 7 * 24 * 60 * 60)
                                            .unwrap_or(false)
                                        {
                                            debug!("Skipping download for {} (file is less than 7 days old)", resource_name);
                                            continue;
                                        }
                                    }
                                }
                            }

                            match self.download_file(url, &resource_path) {
                                Ok(()) => debug!("Downloaded {}", resource_name),
                                Err(e) => warn!("Failed to download {}: {}", resource_name, e),
                            }

                            // Add a small delay between downloads
                            std::thread::sleep(Duration::from_millis(50));
                        }
                    }
                } else {
                    return Err(anyhow!(
                        "Resource types index for {} is not a valid object",
                        region
                    ));
                }

                Ok(())
            }
            Err(e) => {
                warn!(
                    "Could not download resource types index for {}: {}",
                    region, e
                );
                Err(e)
            }
        }
    }

    /// Check if resource specifications are available for a region
    pub fn has_specs_for_region(region: &str) -> bool {
        match Self::get_data_dir() {
            Ok(data_dir) => {
                let spec_file = data_dir
                    .join(region)
                    .join("CloudFormationResourceSpecification.json");
                spec_file.exists()
            }
            Err(_) => false,
        }
    }

    /// Get all resource types for a region (excluding property types)
    pub fn get_resource_types(region: &str) -> Result<Vec<String>> {
        let data_dir = Self::get_data_dir()?;
        let spec_file = data_dir
            .join(region)
            .join("CloudFormationResourceSpecification.json");

        if !spec_file.exists() {
            return Err(anyhow!(
                "Specifications for region {} not downloaded",
                region
            ));
        }

        let content = fs::read_to_string(spec_file)
            .with_context(|| format!("Failed to read specification file for {}", region))?;

        let spec: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse specification file for {}", region))?;

        let mut resource_types = Vec::new();

        // Only add resource types (not property types)
        if let Some(resource_types_obj) = spec["ResourceTypes"].as_object() {
            resource_types.extend(resource_types_obj.keys().cloned());
        } else {
            return Err(anyhow!("Invalid ResourceTypes format for {}", region));
        }

        // Sort the list
        resource_types.sort();

        Ok(resource_types)
    }

    /// Get properties for a specific property type in a region
    pub fn get_property_type_properties(region: &str, property_type: &str) -> Result<Value> {
        use std::time::Instant;
        use tracing::{debug, trace};

        let start_time = Instant::now();
        trace!(
            "Getting properties for property type: {} in region: {}",
            property_type,
            region
        );

        let data_dir = Self::get_data_dir()?;
        trace!("Data dir retrieved in: {:?}", start_time.elapsed());

        // Try to get from individual resource file first (some property types might be in resource files)
        let resource_file = data_dir
            .join(region)
            .join("resources")
            .join(format!("{}.json", property_type.replace("::", "-")));

        if resource_file.exists() {
            trace!("Individual property type file exists: {:?}", resource_file);
            let read_start = Instant::now();
            let content = fs::read_to_string(&resource_file).with_context(|| {
                format!("Failed to read property type file for {}", property_type)
            })?;
            trace!("File read in: {:?}", read_start.elapsed());

            let parse_start = Instant::now();
            let property_data: Value = serde_json::from_str(&content).with_context(|| {
                format!("Failed to parse property type file for {}", property_type)
            })?;
            trace!("JSON parsed in: {:?}", parse_start.elapsed());

            debug!(
                "Property type properties loaded from individual file in: {:?}",
                start_time.elapsed()
            );
            return Ok(property_data);
        }

        trace!("Individual property type file not found, falling back to combined specification");
        // Fallback to the combined specification file
        let spec_file = data_dir
            .join(region)
            .join("CloudFormationResourceSpecification.json");

        if !spec_file.exists() {
            return Err(anyhow!(
                "Specifications for region {} not downloaded",
                region
            ));
        }

        let read_start = Instant::now();
        let content = fs::read_to_string(&spec_file)
            .with_context(|| format!("Failed to read specification file for {}", region))?;
        trace!("Combined spec file read in: {:?}", read_start.elapsed());

        let parse_start = Instant::now();
        let spec: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse specification file for {}", region))?;
        trace!("Combined spec JSON parsed in: {:?}", parse_start.elapsed());

        let lookup_start = Instant::now();
        if let Some(property_data) = spec["PropertyTypes"].get(property_type) {
            trace!(
                "Property type found in combined spec in: {:?}",
                lookup_start.elapsed()
            );
            debug!(
                "Property type properties loaded from combined file in: {:?}",
                start_time.elapsed()
            );
            Ok(property_data.clone())
        } else {
            trace!("Property type not found in: {:?}", lookup_start.elapsed());
            Err(anyhow!(
                "Property type {} not found in region {}",
                property_type,
                region
            ))
        }
    }

    /// Get properties for a specific resource type in a region
    pub fn get_resource_properties(region: &str, resource_type: &str) -> Result<Value> {
        use std::time::Instant;
        use tracing::{debug, trace};

        let start_time = Instant::now();
        trace!(
            "Getting properties for resource type: {} in region: {}",
            resource_type,
            region
        );

        let data_dir = Self::get_data_dir()?;
        trace!("Data dir retrieved in: {:?}", start_time.elapsed());

        // Try to get from individual resource file first
        let resource_file = data_dir
            .join(region)
            .join("resources")
            .join(format!("{}.json", resource_type.replace("::", "-")));

        if resource_file.exists() {
            trace!("Individual resource file exists: {:?}", resource_file);
            let read_start = Instant::now();
            let content = fs::read_to_string(&resource_file)
                .with_context(|| format!("Failed to read resource file for {}", resource_type))?;
            trace!("File read in: {:?}", read_start.elapsed());

            let parse_start = Instant::now();
            let resource_data: Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse resource file for {}", resource_type))?;
            trace!("JSON parsed in: {:?}", parse_start.elapsed());

            debug!(
                "Resource properties loaded from individual file in: {:?}",
                start_time.elapsed()
            );
            return Ok(resource_data);
        }

        trace!("Individual resource file not found, falling back to combined specification");
        // Fallback to the combined specification file
        let spec_file = data_dir
            .join(region)
            .join("CloudFormationResourceSpecification.json");

        if !spec_file.exists() {
            return Err(anyhow!(
                "Specifications for region {} not downloaded",
                region
            ));
        }

        let read_start = Instant::now();
        let content = fs::read_to_string(&spec_file)
            .with_context(|| format!("Failed to read specification file for {}", region))?;
        trace!("Combined spec file read in: {:?}", read_start.elapsed());

        let parse_start = Instant::now();
        let spec: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse specification file for {}", region))?;
        trace!("Combined spec JSON parsed in: {:?}", parse_start.elapsed());

        let lookup_start = Instant::now();
        if let Some(resource_data) = spec["ResourceTypes"].get(resource_type) {
            trace!(
                "Resource type found in combined spec in: {:?}",
                lookup_start.elapsed()
            );
            debug!(
                "Resource properties loaded from combined file in: {:?}",
                start_time.elapsed()
            );
            Ok(resource_data.clone())
        } else {
            trace!("Resource type not found in: {:?}", lookup_start.elapsed());
            Err(anyhow!(
                "Resource type {} not found in region {}",
                resource_type,
                region
            ))
        }
    }
}

/// Raw resource type definitions as JSON values.
/// Maps resource type names (e.g., "AWS::EC2::Instance") to their complete specification.
pub type ResourceTypeMap = HashMap<String, Value>;

/// Parsed resource definitions with structured data.
/// Maps resource type names to parsed `ResourceDefinition` structs.
pub type ResourceDefinitionMap = HashMap<String, ResourceDefinition>;

/// Property definitions for resource types.
/// Maps property names to their detailed specifications including constraints.
pub type PropertyDefinitionMap = HashMap<String, PropertyDefinition>;

/// Attribute definitions for resource types.
/// Maps attribute names to their type information for `Fn::GetAtt` references.
pub type AttributeDefinitionMap = HashMap<String, AttributeDefinition>;

/// Complete definition of a CloudFormation resource type.
///
/// Contains all information needed to generate forms, perform validation,
/// and provide documentation for a specific resource type.
///
/// ## Usage in Form Generation
///
/// ```rust
/// use crate::app::cfn_resources::ResourceDefinition;
///
/// fn generate_resource_form(resource_def: &ResourceDefinition) {
///     // Display documentation to help users
///     if !resource_def.documentation.is_empty() {
///         println!("Help: {}", resource_def.documentation);
///     }
///
///     // Generate form fields for each property
///     for (prop_name, prop_def) in &resource_def.properties {
///         let label = if prop_def.required {
///             format!("{}*", prop_name)
///         } else {
///             prop_name.clone()
///         };
///         // Create appropriate UI widget based on property type
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ResourceDefinition {
    /// Human-readable documentation for the resource type
    pub documentation: String,
    /// URL to AWS documentation (typically the same as documentation field)
    pub documentation_url: String,
    /// All properties that can be set on this resource type
    pub properties: PropertyDefinitionMap,
    /// All attributes that can be retrieved via Fn::GetAtt
    pub attributes: AttributeDefinitionMap,
}

/// Definition of a CloudFormation resource property.
///
/// Contains comprehensive information about a single property of a resource type,
/// including type information, validation constraints, and AWS-specific metadata.
/// This is the primary data structure used for form generation and validation.
///
/// ## Type System
///
/// CloudFormation uses a hierarchical type system:
/// - **Primitive Types**: String, Integer, Boolean, Double, Json, etc.
/// - **Complex Types**: Custom objects defined in PropertyTypes
/// - **Collection Types**: List and Map with typed elements
///
/// ## Example Usage
///
/// ```rust
/// use crate::app::cfn_resources::PropertyDefinition;
///
/// fn create_form_widget(prop_def: &PropertyDefinition) -> Widget {
///     match (&prop_def.primitive_type, &prop_def.enum_values) {
///         (Some(_), Some(enum_vals)) => {
///             // Create dropdown with enum values
///             create_dropdown(enum_vals)
///         }
///         (Some(prim_type), None) if prim_type == "Boolean" => {
///             // Create checkbox
///             create_checkbox()
///         }
///         (Some(prim_type), None) if prim_type == "Integer" => {
///             // Create number input with constraints
///             create_number_input(prop_def.min_value, prop_def.max_value)
///         }
///         _ => {
///             // Create text input with pattern validation
///             create_text_input(prop_def.pattern.as_ref())
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PropertyDefinition {
    /// Human-readable description of the property's purpose
    pub documentation: String,
    /// Whether this property must be specified (affects UI marking and validation)
    pub required: bool,
    /// CloudFormation primitive type (String, Integer, Boolean, etc.)
    pub primitive_type: Option<String>,
    /// CloudFormation complex type name (for nested objects)
    pub type_name: Option<String>,
    /// Element type for List and Map types
    pub item_type: Option<String>,
    /// AWS update behavior: Mutable, Immutable, or Conditional
    pub update_type: String,

    // Enhanced schema validation fields from CloudFormation schemas
    /// Valid enumeration values (generates dropdown UI controls)
    pub enum_values: Option<Vec<String>>,
    /// Regular expression pattern for string validation
    pub pattern: Option<String>,
    /// Minimum string length constraint
    pub min_length: Option<usize>,
    /// Maximum string length constraint
    pub max_length: Option<usize>,
    /// Minimum numeric value constraint
    pub min_value: Option<f64>,
    /// Maximum numeric value constraint
    pub max_value: Option<f64>,
    /// Whether array elements must be unique
    pub unique_items: Option<bool>,
}

/// Definition of a CloudFormation resource attribute.
///
/// Attributes are read-only values that can be retrieved from a resource
/// using the `Fn::GetAtt` intrinsic function. They represent computed
/// or runtime values like ARNs, URLs, or auto-generated identifiers.
///
/// ## Example Usage
///
/// ```rust
/// // In a CloudFormation template:
/// // "MyInstanceArn": { "Fn::GetAtt": ["MyInstance", "Arn"] }
///
/// use crate::app::cfn_resources::AttributeDefinition;
///
/// fn validate_getatt_reference(attr_def: &AttributeDefinition, resource_type: &str, attr_name: &str) -> bool {
///     // Validate that the attribute exists and check its type
///     match (&attr_def.primitive_type, &attr_def.type_name) {
///         (Some(prim_type), _) => {
///             println!("Attribute {}.{} returns {}", resource_type, attr_name, prim_type);
///             true
///         }
///         (None, Some(type_name)) => {
///             println!("Attribute {}.{} returns complex type {}", resource_type, attr_name, type_name);
///             true
///         }
///         _ => false
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AttributeDefinition {
    /// CloudFormation primitive type of the attribute value
    pub primitive_type: Option<String>,
    /// CloudFormation complex type name if the attribute returns an object
    pub type_name: Option<String>,
}

/// Loads all CloudFormation resource types for a specified region.
///
/// This is the primary function for discovering available resource types.
/// It first checks the memory cache for immediate response, then loads from
/// disk if needed. The result includes all resource types (but not property types)
/// available in the specified region.
///
/// ## Caching Strategy
///
/// 1. **Memory Cache**: Check thread-safe in-memory cache first
/// 2. **Disk Cache**: Load from local specification files if not cached
/// 3. **Cache Population**: Store results in memory for future access
///
/// ## Parameters
///
/// * `region` - AWS region identifier (e.g., "us-east-1", "eu-west-1")
///
/// ## Returns
///
/// Returns a `ResourceTypeMap` containing resource type names mapped to their
/// complete JSON specifications. Resource types follow the pattern
/// `AWS::Service::Resource` (e.g., "AWS::EC2::Instance").
///
/// ## Errors
///
/// Returns an error if:
/// - Specifications for the region haven't been downloaded
/// - File system access fails
/// - JSON parsing fails due to corrupted files
///
/// ## Example
///
/// ```rust
/// use crate::app::cfn_resources::load_resource_types;
///
/// match load_resource_types("us-east-1") {
///     Ok(resource_types) => {
///         println!("Found {} resource types", resource_types.len());
///
///         // List all EC2 resources
///         for resource_name in resource_types.keys() {
///             if resource_name.starts_with("AWS::EC2::") {
///                 println!("EC2 Resource: {}", resource_name);
///             }
///         }
///     }
///     Err(e) => {
///         eprintln!("Failed to load resource types: {}", e);
///     }
/// }
/// ```
pub fn load_resource_types(region: &str) -> Result<ResourceTypeMap> {
    // Check cache first
    {
        let cache = RESOURCE_TYPE_CACHE.lock().unwrap();
        if let Some(cached_types) = cache.get(region) {
            log_trace!("Using cached resource types for region: {}", region);
            return Ok(cached_types.clone());
        }
    }

    debug!(
        "Cache miss for resource types in region: {}, loading from disk",
        region
    );

    // Get data directory with minimal initialization
    let data_dir = match DATA_DIR.get() {
        Some(dir) => dir.clone(),
        None => CfnResourcesDownloader::get_data_dir()?,
    };
    let spec_file = data_dir
        .join(region)
        .join("CloudFormationResourceSpecification.json");

    if !spec_file.exists() {
        return Err(anyhow!(
            "Specifications for region {} not downloaded",
            region
        ));
    }

    let content = fs::read_to_string(&spec_file)
        .with_context(|| format!("Failed to read specification file for {}", region))?;

    let spec: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse specification file for {}", region))?;

    if let Some(resource_types) = spec["ResourceTypes"].as_object() {
        let mut resource_map = ResourceTypeMap::new();

        for (resource_name, resource_data) in resource_types {
            resource_map.insert(resource_name.clone(), resource_data.clone());
        }

        // Store in cache
        {
            let mut cache = RESOURCE_TYPE_CACHE.lock().unwrap();
            cache.insert(region.to_string(), resource_map.clone());
            log_trace!("Cached resource types for region: {}", region);
        }

        Ok(resource_map)
    } else {
        Err(anyhow!("Invalid specification format for {}", region))
    }
}

/// Load resource type definitions from a specification file
pub fn load_resource_definitions(region: &str) -> Result<ResourceDefinitionMap> {
    // Get data directory with minimal initialization
    let data_dir = match DATA_DIR.get() {
        Some(dir) => dir.clone(),
        None => CfnResourcesDownloader::get_data_dir()?,
    };
    let spec_file = data_dir
        .join(region)
        .join("CloudFormationResourceSpecification.json");

    if !spec_file.exists() {
        return Err(anyhow!(
            "Specifications for region {} not downloaded",
            region
        ));
    }

    let content = fs::read_to_string(&spec_file)
        .with_context(|| format!("Failed to read specification file for {}", region))?;

    let spec: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse specification file for {}", region))?;

    if let Some(resource_types) = spec["ResourceTypes"].as_object() {
        let mut resource_defs = ResourceDefinitionMap::new();

        for (resource_name, resource_data) in resource_types {
            let documentation = resource_data
                .get("Documentation")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            // Extract documentation URL
            let documentation_url = if documentation.is_empty() {
                String::new()
            } else {
                documentation.clone()
            };

            let properties = load_property_definitions_from_value(resource_data);
            let attributes = load_attribute_definitions_from_value(resource_data);

            resource_defs.insert(
                resource_name.clone(),
                ResourceDefinition {
                    documentation,
                    documentation_url,
                    properties,
                    attributes,
                },
            );
        }

        Ok(resource_defs)
    } else {
        Err(anyhow!("Invalid specification format for {}", region))
    }
}

/// Load property definitions from a resource data value
pub fn load_property_definitions_from_value(resource_data: &Value) -> PropertyDefinitionMap {
    let mut properties = PropertyDefinitionMap::new();

    if let Some(props_obj) = resource_data.get("Properties").and_then(|p| p.as_object()) {
        for (prop_name, prop_data) in props_obj {
            let documentation = prop_data
                .get("Documentation")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let required = prop_data
                .get("Required")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);

            let primitive_type = prop_data
                .get("PrimitiveType")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());

            let type_name = prop_data
                .get("Type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());

            let item_type = prop_data
                .get("ItemType")
                .and_then(|i| i.as_str())
                .map(|s| s.to_string());

            let update_type = prop_data
                .get("UpdateType")
                .and_then(|u| u.as_str())
                .unwrap_or("Mutable")
                .to_string();

            properties.insert(
                prop_name.clone(),
                PropertyDefinition {
                    documentation,
                    required,
                    primitive_type,
                    type_name,
                    item_type,
                    update_type,
                    // Initialize enhanced schema validation fields to None for now
                    enum_values: None,
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    min_value: None,
                    max_value: None,
                    unique_items: None,
                },
            );
        }
    }

    properties
}

/// Load attribute definitions from a resource data value
pub fn load_attribute_definitions_from_value(resource_data: &Value) -> AttributeDefinitionMap {
    let mut attributes = AttributeDefinitionMap::new();

    if let Some(attrs_obj) = resource_data.get("Attributes").and_then(|a| a.as_object()) {
        for (attr_name, attr_data) in attrs_obj {
            let primitive_type = attr_data
                .get("PrimitiveType")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());

            let type_name = attr_data
                .get("Type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());

            attributes.insert(
                attr_name.clone(),
                AttributeDefinition {
                    primitive_type,
                    type_name,
                },
            );
        }
    }

    attributes
}

/// Load property type definitions from a specification file
pub fn load_property_type_definitions(
    region: &str,
    property_type: &str,
) -> Result<PropertyDefinitionMap> {
    use std::time::Instant;
    use tracing::{debug, trace};

    let start_time = Instant::now();
    trace!(
        "Loading property type definitions for: {} in region: {}",
        property_type,
        region
    );

    // Check cache first with a different prefix to avoid collision with resource properties
    let cache_key = format!("PT:{}:{}", region, property_type);
    {
        let cache = PROPERTY_CACHE.lock().unwrap();
        if let Some(cached_props) = cache.get(&cache_key) {
            debug!(
                "Using cached property type definitions for {}:{}",
                region, property_type
            );
            return Ok(cached_props.clone());
        }
    }

    debug!(
        "Cache miss for property type definitions {}:{}, loading from disk",
        region, property_type
    );

    // Get data directory with minimal initialization
    let dir_start = Instant::now();
    let data_dir = match DATA_DIR.get() {
        Some(dir) => dir.clone(),
        None => CfnResourcesDownloader::get_data_dir()?,
    };
    trace!("Data dir retrieved in: {:?}", dir_start.elapsed());

    // Try to get from individual property type file first
    let property_file = data_dir
        .join(region)
        .join("resources")
        .join(format!("{}.json", property_type.replace("::", "-")));

    if property_file.exists() {
        trace!("Individual property type file exists: {:?}", property_file);
        let read_start = Instant::now();
        let content = fs::read_to_string(&property_file)
            .with_context(|| format!("Failed to read property type file for {}", property_type))?;
        trace!("File read in: {:?}", read_start.elapsed());

        let parse_start = Instant::now();
        let property_data: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse property type file for {}", property_type))?;
        trace!("JSON parsed in: {:?}", parse_start.elapsed());

        let extract_start = Instant::now();
        let result = load_property_definitions_from_value(&property_data);
        trace!("Properties extracted in: {:?}", extract_start.elapsed());

        // Store in cache
        {
            let mut cache = PROPERTY_CACHE.lock().unwrap();
            cache.insert(cache_key, result.clone());
            debug!(
                "Cached property type definitions for {}:{}",
                region, property_type
            );
        }

        debug!(
            "Property type definitions loaded from individual file in: {:?}",
            start_time.elapsed()
        );
        return Ok(result);
    }

    trace!("Individual property type file not found, falling back to combined specification");
    // Fallback to the combined specification file
    let spec_file = data_dir
        .join(region)
        .join("CloudFormationResourceSpecification.json");

    if !spec_file.exists() {
        return Err(anyhow!(
            "Specifications for region {} not downloaded",
            region
        ));
    }

    let read_start = Instant::now();
    let content = fs::read_to_string(&spec_file)
        .with_context(|| format!("Failed to read specification file for {}", region))?;
    trace!("Combined spec file read in: {:?}", read_start.elapsed());

    let parse_start = Instant::now();
    let spec: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse specification file for {}", region))?;
    trace!("Combined spec JSON parsed in: {:?}", parse_start.elapsed());

    let lookup_start = Instant::now();
    if let Some(property_data) = spec["PropertyTypes"].get(property_type) {
        trace!(
            "Property type found in combined spec in: {:?}",
            lookup_start.elapsed()
        );

        let extract_start = Instant::now();
        let result = load_property_definitions_from_value(property_data);
        trace!("Properties extracted in: {:?}", extract_start.elapsed());

        // Store in cache
        {
            let mut cache = PROPERTY_CACHE.lock().unwrap();
            cache.insert(cache_key, result.clone());
            debug!(
                "Cached property type definitions for {}:{}",
                region, property_type
            );
        }

        debug!(
            "Property type definitions loaded from combined file in: {:?}",
            start_time.elapsed()
        );
        Ok(result)
    } else {
        trace!("Property type not found in: {:?}", lookup_start.elapsed());
        Err(anyhow!(
            "Property type {} not found in region {}",
            property_type,
            region
        ))
    }
}

/// Loads property definitions for a specific resource type in a region.
///
/// This function provides the detailed property specifications needed for
/// form generation and validation. It implements a sophisticated caching
/// and fallback strategy for optimal performance.
///
/// ## Multi-Level Loading Strategy
///
/// 1. **Memory Cache**: Check for cached property definitions first
/// 2. **Individual Files**: Look for dedicated resource type files (fastest)
/// 3. **Combined Specification**: Fall back to the main specification file
/// 4. **Cache Population**: Store results for future access
///
/// ## Parameters
///
/// * `region` - AWS region identifier
/// * `resource_type` - Full resource type name (e.g., "AWS::EC2::Instance")
///
/// ## Returns
///
/// Returns a `PropertyDefinitionMap` with property names mapped to their
/// complete definitions including type information, constraints, and metadata.
///
/// ## Performance Notes
///
/// This function is heavily optimized for form generation scenarios:
/// - Memory cache provides sub-millisecond access for repeated calls
/// - Individual resource files reduce parsing overhead for single resources
/// - Combined fallback ensures compatibility with all download strategies
///
/// ## Example
///
/// ```rust
/// use crate::app::cfn_resources::load_property_definitions;
///
/// match load_property_definitions("us-east-1", "AWS::EC2::Instance") {
///     Ok(properties) => {
///         // Generate form fields for required properties first
///         let mut required_props: Vec<_> = properties.iter()
///             .filter(|(_, prop_def)| prop_def.required)
///             .collect();
///         required_props.sort_by_key(|(name, _)| *name);
///
///         for (prop_name, prop_def) in required_props {
///             println!("Required: {} ({})", prop_name,
///                      prop_def.primitive_type.as_deref().unwrap_or("Complex"));
///         }
///     }
///     Err(e) => {
///         eprintln!("Failed to load properties: {}", e);
///     }
/// }
/// ```
pub fn load_property_definitions(
    region: &str,
    resource_type: &str,
) -> Result<PropertyDefinitionMap> {
    use std::time::Instant;
    use tracing::{debug, trace};

    let start_time = Instant::now();
    trace!(
        "Loading property definitions for resource type: {} in region: {}",
        resource_type,
        region
    );

    // Check cache first
    let cache_key = format!("{}:{}", region, resource_type);
    {
        let cache = PROPERTY_CACHE.lock().unwrap();
        if let Some(cached_props) = cache.get(&cache_key) {
            debug!(
                "Using cached property definitions for {}:{}",
                region, resource_type
            );
            return Ok(cached_props.clone());
        }
    }

    debug!(
        "Cache miss for property definitions {}:{}, loading from disk",
        region, resource_type
    );

    // Get data directory with minimal initialization
    let dir_start = Instant::now();
    let data_dir = match DATA_DIR.get() {
        Some(dir) => dir.clone(),
        None => CfnResourcesDownloader::get_data_dir()?,
    };
    trace!("Data dir retrieved in: {:?}", dir_start.elapsed());

    // Try to get from individual resource file first
    let resource_file = data_dir
        .join(region)
        .join("resources")
        .join(format!("{}.json", resource_type.replace("::", "-")));

    if resource_file.exists() {
        trace!("Individual resource file exists: {:?}", resource_file);
        let read_start = Instant::now();
        let content = fs::read_to_string(&resource_file)
            .with_context(|| format!("Failed to read resource file for {}", resource_type))?;
        trace!("File read in: {:?}", read_start.elapsed());

        let parse_start = Instant::now();
        let resource_data: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse resource file for {}", resource_type))?;
        trace!("JSON parsed in: {:?}", parse_start.elapsed());

        let extract_start = Instant::now();
        let result = load_property_definitions_from_value(&resource_data);
        trace!("Properties extracted in: {:?}", extract_start.elapsed());

        // Store in cache
        {
            let mut cache = PROPERTY_CACHE.lock().unwrap();
            cache.insert(cache_key, result.clone());
            debug!(
                "Cached property definitions for {}:{}",
                region, resource_type
            );
        }

        debug!(
            "Property definitions loaded from individual file in: {:?}",
            start_time.elapsed()
        );
        return Ok(result);
    }

    trace!("Individual resource file not found, falling back to combined specification");
    // Fallback to the combined specification file
    let spec_file = data_dir
        .join(region)
        .join("CloudFormationResourceSpecification.json");

    if !spec_file.exists() {
        return Err(anyhow!(
            "Specifications for region {} not downloaded",
            region
        ));
    }

    let read_start = Instant::now();
    let content = fs::read_to_string(&spec_file)
        .with_context(|| format!("Failed to read specification file for {}", region))?;
    trace!("Combined spec file read in: {:?}", read_start.elapsed());

    let parse_start = Instant::now();
    let spec: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse specification file for {}", region))?;
    trace!("Combined spec JSON parsed in: {:?}", parse_start.elapsed());

    let lookup_start = Instant::now();
    if let Some(resource_data) = spec["ResourceTypes"].get(resource_type) {
        trace!(
            "Resource type found in combined spec in: {:?}",
            lookup_start.elapsed()
        );

        let extract_start = Instant::now();
        let result = load_property_definitions_from_value(resource_data);
        trace!("Properties extracted in: {:?}", extract_start.elapsed());

        // Store in cache
        {
            let mut cache = PROPERTY_CACHE.lock().unwrap();
            cache.insert(cache_key, result.clone());
            debug!(
                "Cached property definitions for {}:{}",
                region, resource_type
            );
        }

        debug!(
            "Property definitions loaded from combined file in: {:?}",
            start_time.elapsed()
        );
        Ok(result)
    } else {
        trace!("Resource type not found in: {:?}", lookup_start.elapsed());
        Err(anyhow!(
            "Resource type {} not found in region {}",
            resource_type,
            region
        ))
    }
}

/// Load attribute definitions from a specification file
pub fn load_attribute_definitions(
    region: &str,
    resource_type: &str,
) -> Result<AttributeDefinitionMap> {
    use std::time::Instant;
    use tracing::{debug, trace};

    let start_time = Instant::now();
    trace!(
        "Loading attribute definitions for resource type: {} in region: {}",
        resource_type,
        region
    );

    // Check cache first
    let cache_key = format!("{}:{}", region, resource_type);
    {
        let cache = ATTRIBUTE_CACHE.lock().unwrap();
        if let Some(cached_attrs) = cache.get(&cache_key) {
            debug!(
                "Using cached attribute definitions for {}:{}",
                region, resource_type
            );
            return Ok(cached_attrs.clone());
        }
    }

    debug!(
        "Cache miss for attribute definitions {}:{}, loading from disk",
        region, resource_type
    );

    // Get data directory with minimal initialization
    let dir_start = Instant::now();
    let data_dir = match DATA_DIR.get() {
        Some(dir) => dir.clone(),
        None => CfnResourcesDownloader::get_data_dir()?,
    };
    trace!("Data dir retrieved in: {:?}", dir_start.elapsed());

    // Try to get from individual resource file first
    let resource_file = data_dir
        .join(region)
        .join("resources")
        .join(format!("{}.json", resource_type.replace("::", "-")));

    if resource_file.exists() {
        trace!("Individual resource file exists: {:?}", resource_file);
        let read_start = Instant::now();
        let content = fs::read_to_string(&resource_file)
            .with_context(|| format!("Failed to read resource file for {}", resource_type))?;
        trace!("File read in: {:?}", read_start.elapsed());

        let parse_start = Instant::now();
        let resource_data: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse resource file for {}", resource_type))?;
        trace!("JSON parsed in: {:?}", parse_start.elapsed());

        let extract_start = Instant::now();
        let result = load_attribute_definitions_from_value(&resource_data);
        trace!("Attributes extracted in: {:?}", extract_start.elapsed());

        // Store in cache
        {
            let mut cache = ATTRIBUTE_CACHE.lock().unwrap();
            cache.insert(cache_key, result.clone());
            debug!(
                "Cached attribute definitions for {}:{}",
                region, resource_type
            );
        }

        debug!(
            "Attribute definitions loaded from individual file in: {:?}",
            start_time.elapsed()
        );
        return Ok(result);
    }

    trace!("Individual resource file not found, falling back to combined specification");
    // Fallback to the combined specification file
    let spec_file = data_dir
        .join(region)
        .join("CloudFormationResourceSpecification.json");

    if !spec_file.exists() {
        return Err(anyhow!(
            "Specifications for region {} not downloaded",
            region
        ));
    }

    let read_start = Instant::now();
    let content = fs::read_to_string(&spec_file)
        .with_context(|| format!("Failed to read specification file for {}", region))?;
    trace!("Combined spec file read in: {:?}", read_start.elapsed());

    let parse_start = Instant::now();
    let spec: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse specification file for {}", region))?;
    trace!("Combined spec JSON parsed in: {:?}", parse_start.elapsed());

    let lookup_start = Instant::now();
    if let Some(resource_data) = spec["ResourceTypes"].get(resource_type) {
        trace!(
            "Resource type found in combined spec in: {:?}",
            lookup_start.elapsed()
        );

        let extract_start = Instant::now();
        let result = load_attribute_definitions_from_value(resource_data);
        trace!("Attributes extracted in: {:?}", extract_start.elapsed());

        // Store in cache
        {
            let mut cache = ATTRIBUTE_CACHE.lock().unwrap();
            cache.insert(cache_key, result.clone());
            debug!(
                "Cached attribute definitions for {}:{}",
                region, resource_type
            );
        }

        debug!(
            "Attribute definitions loaded from combined file in: {:?}",
            start_time.elapsed()
        );
        Ok(result)
    } else {
        trace!("Resource type not found in: {:?}", lookup_start.elapsed());
        Err(anyhow!(
            "Resource type {} not found in region {}",
            resource_type,
            region
        ))
    }
}

/// Determine if a type is a resource type or a property type
///
/// A resource type follows the pattern AWS::Service::Resource
/// A property type follows the pattern AWS::Service::Resource.PropertyName
pub fn is_resource_type(type_name: &str, _region: &str) -> Result<bool> {
    // If the type name contains a dot after the resource part, it's a property type
    // Otherwise, it's a resource type
    if type_name.contains('.') {
        // It's a property type (e.g., AWS::EC2::Instance.NetworkInterface)
        Ok(false)
    } else {
        // It's a resource type (e.g., AWS::EC2::Instance)
        Ok(true)
    }
}

// Helper to parse CloudFormation resource properties
pub fn parse_resource_properties(resource_data: &Value) -> Result<Vec<(String, bool, String)>> {
    let mut properties = Vec::new();

    if let Some(props_obj) = resource_data.get("Properties").and_then(|p| p.as_object()) {
        for (prop_name, prop_data) in props_obj {
            let required = prop_data
                .get("Required")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);

            let prop_type = match (
                prop_data.get("PrimitiveType").and_then(|p| p.as_str()),
                prop_data.get("Type").and_then(|t| t.as_str()),
                prop_data.get("ItemType").and_then(|i| i.as_str()),
            ) {
                (Some(primitive), _, _) => primitive.to_string(),
                (_, Some("List"), Some(item_type)) => format!("List<{}>", item_type),
                (_, Some("Map"), Some(item_type)) => format!("Map<{}>", item_type),
                (_, Some(type_name), _) => type_name.to_string(),
                _ => "Unknown".to_string(),
            };

            properties.push((prop_name.clone(), required, prop_type));
        }
    }

    // Sort by required (required first) then by name
    properties.sort_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.cmp(&b.0),
    });

    Ok(properties)
}

/// Parses validation constraints from CloudFormation property schema.
///
/// Extracts comprehensive validation rules from JSON schema definitions,
/// enabling rich form validation and appropriate UI widget selection.
/// This function processes both basic constraints (string length, numeric ranges)
/// and advanced constraints (patterns, enumerations, uniqueness rules).
///
/// ## Supported Constraint Types
///
/// * **Enumerations**: `enum` field → dropdown/radio button UI
/// * **Pattern Validation**: `pattern` field → regex validation
/// * **String Constraints**: `minLength`, `maxLength` → input validation
/// * **Numeric Constraints**: `minimum`, `maximum` → number input ranges
/// * **Array Constraints**: `uniqueItems` → duplicate detection
///
/// ## Parameters
///
/// * `property_schema` - JSON schema object from CloudFormation specification
///
/// ## Returns
///
/// Returns a `SchemaConstraints` struct with all applicable constraints populated.
/// Fields are `None` if the corresponding constraint is not present in the schema.
///
/// ## Example Usage
///
/// ```rust
/// use crate::app::cfn_resources::parse_schema_constraints;
/// use serde_json::json;
///
/// let schema = json!({
///     "type": "string",
///     "enum": ["t2.micro", "t2.small", "t2.medium"],
///     "pattern": "^t[2-3]\\.(micro|small|medium)$",
///     "minLength": 3,
///     "maxLength": 20
/// });
///
/// let constraints = parse_schema_constraints(&schema);
///
/// // Use constraints for form generation
/// if let Some(enum_values) = &constraints.enum_values {
///     // Generate dropdown with predefined values
///     create_dropdown_widget(enum_values);
/// } else if let Some(pattern) = &constraints.pattern {
///     // Generate text input with pattern validation
///     create_validated_text_input(pattern);
/// }
/// ```
///
/// ## Integration with Form Generation
///
/// The parsed constraints directly drive UI widget selection:
/// - Enum values → ComboBox or Radio buttons
/// - Patterns → Text inputs with real-time validation
/// - Numeric ranges → Slider or spin box controls
/// - Length limits → Character counters and input validation
pub fn parse_schema_constraints(property_schema: &Value) -> SchemaConstraints {
    let mut constraints = SchemaConstraints::default();

    // Parse enum values
    if let Some(enum_array) = property_schema.get("enum") {
        if let Some(array) = enum_array.as_array() {
            let values: Vec<String> = array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !values.is_empty() {
                constraints.enum_values = Some(values);
            }
        }
    }

    // Parse pattern (regex)
    if let Some(pattern_value) = property_schema.get("pattern") {
        if let Some(pattern_str) = pattern_value.as_str() {
            constraints.pattern = Some(pattern_str.to_string());
        }
    }

    // Parse string length constraints
    if let Some(min_len) = property_schema.get("minLength") {
        if let Some(min_len_num) = min_len.as_u64() {
            constraints.min_length = Some(min_len_num as usize);
        }
    }

    if let Some(max_len) = property_schema.get("maxLength") {
        if let Some(max_len_num) = max_len.as_u64() {
            constraints.max_length = Some(max_len_num as usize);
        }
    }

    // Parse numeric value constraints
    if let Some(min_val) = property_schema.get("minimum") {
        if let Some(min_val_num) = min_val.as_f64() {
            constraints.min_value = Some(min_val_num);
        }
    }

    if let Some(max_val) = property_schema.get("maximum") {
        if let Some(max_val_num) = max_val.as_f64() {
            constraints.max_value = Some(max_val_num);
        }
    }

    // Parse array constraints
    if let Some(unique_items_val) = property_schema.get("uniqueItems") {
        if let Some(unique_bool) = unique_items_val.as_bool() {
            constraints.unique_items = Some(unique_bool);
        }
    }

    constraints
}

/// Loads property definitions enhanced with validation constraints from schema files.
///
/// This function extends `load_property_definitions()` by attempting to merge
/// validation constraints from CloudFormation JSON schema files. It provides
/// the richest possible property definitions for advanced form generation
/// and validation scenarios.
///
/// ## Enhancement Process
///
/// 1. **Load Base Definitions**: Get standard property definitions
/// 2. **Locate Schema Files**: Search for matching JSON schema files
/// 3. **Parse Constraints**: Extract validation rules from schemas
/// 4. **Merge Information**: Combine base definitions with schema constraints
/// 5. **Fallback Gracefully**: Return base definitions if schema enhancement fails
///
/// ## Benefits of Schema Enhancement
///
/// * **Rich Validation**: Enum values, patterns, ranges, and format constraints
/// * **Better UX**: Appropriate UI widgets (dropdowns vs text inputs)
/// * **Real-time Feedback**: Client-side validation before template submission
/// * **Documentation**: Additional property hints and valid value examples
///
/// ## Performance Considerations
///
/// Schema enhancement adds some overhead compared to basic property loading:
/// - Additional file system lookups for schema files
/// - Extra JSON parsing for constraint extraction
/// - Graceful degradation ensures fallback to basic definitions
///
/// ## Example Usage
///
/// ```rust
/// use crate::app::cfn_resources::load_property_definitions_with_constraints;
///
/// match load_property_definitions_with_constraints("us-east-1", "AWS::EC2::Instance") {
///     Ok(properties) => {
///         for (prop_name, prop_def) in properties {
///             println!("Property: {}", prop_name);
///
///             // Check for enhanced constraints
///             if let Some(enum_vals) = &prop_def.enum_values {
///                 println!("  Valid values: {:?}", enum_vals);
///             }
///             if let Some(pattern) = &prop_def.pattern {
///                 println!("  Pattern: {}", pattern);
///             }
///             if prop_def.min_length.is_some() || prop_def.max_length.is_some() {
///                 println!("  Length: {:?}-{:?}", prop_def.min_length, prop_def.max_length);
///             }
///         }
///     }
///     Err(e) => {
///         eprintln!("Failed to load enhanced properties: {}", e);
///     }
/// }
/// ```
pub fn load_property_definitions_with_constraints(
    region: &str,
    resource_type: &str,
) -> Result<PropertyDefinitionMap> {
    // First load the basic property definitions
    let properties = load_property_definitions(region, resource_type)?;

    // Try to enhance with schema constraints from CloudFormation schema files
    if let Ok(enhanced_properties) =
        enhance_properties_with_schema_constraints(region, resource_type, &properties)
    {
        return Ok(enhanced_properties);
    }

    // Fall back to basic properties if schema enhancement fails
    Ok(properties)
}

/// Enhance property definitions with schema constraints
fn enhance_properties_with_schema_constraints(
    _region: &str,
    resource_type: &str,
    base_properties: &PropertyDefinitionMap,
) -> Result<PropertyDefinitionMap> {
    // Try to load schema constraints from CloudFormation schema files
    if let Ok(schema_constraints) = load_schema_constraints_for_resource(resource_type) {
        let mut enhanced_properties = base_properties.clone();

        // Apply schema constraints to matching properties
        for (property_name, property_def) in enhanced_properties.iter_mut() {
            if let Some(constraints) = schema_constraints.get(property_name) {
                property_def.enum_values = constraints.enum_values.clone();
                property_def.pattern = constraints.pattern.clone();
                property_def.min_length = constraints.min_length;
                property_def.max_length = constraints.max_length;
                property_def.min_value = constraints.min_value;
                property_def.max_value = constraints.max_value;
                property_def.unique_items = constraints.unique_items;
            }
        }

        return Ok(enhanced_properties);
    }

    log::debug!(
        "Schema constraint enhancement not available for {}",
        resource_type
    );
    Ok(base_properties.clone())
}

/// Load schema constraints for a resource type from CloudFormation schema files
fn load_schema_constraints_for_resource(
    resource_type: &str,
) -> Result<HashMap<String, SchemaConstraints>> {
    // Get data directory
    let data_dir = match DATA_DIR.get() {
        Some(dir) => dir.clone(),
        None => CfnResourcesDownloader::get_data_dir()?,
    };

    // Try to find schema file for the resource type
    // Schema files are typically stored in schemas/ directory with .json extension
    let schema_filename = format!("{}.json", resource_type.replace("::", "-"));

    // Look in all regions for the schema file (schemas are often region-agnostic)
    for region in AWS_REGIONS {
        let schema_path = data_dir.join(region).join("schemas").join(&schema_filename);

        if schema_path.exists() {
            return parse_schema_constraints_from_file(&schema_path);
        }
    }

    // If no individual schema file found, try to extract from combined schemas
    for region in AWS_REGIONS {
        let schemas_dir = data_dir.join(region).join("schemas");
        if schemas_dir.exists() {
            if let Ok(constraints) =
                parse_schema_from_schemas_directory(&schemas_dir, resource_type)
            {
                return Ok(constraints);
            }
        }
    }

    Err(anyhow!(
        "No schema constraints found for resource type: {}",
        resource_type
    ))
}

/// Parse schema constraints from a specific schema file
fn parse_schema_constraints_from_file(
    schema_path: &Path,
) -> Result<HashMap<String, SchemaConstraints>> {
    let content = fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema file: {:?}", schema_path))?;

    let schema: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse schema file: {:?}", schema_path))?;

    let mut constraints = HashMap::new();

    // Parse properties from the schema
    if let Some(properties) = schema.get("properties") {
        if let Some(resource_props) = properties.get("Properties") {
            if let Some(props_schema) = resource_props.get("properties") {
                if let Some(props_obj) = props_schema.as_object() {
                    for (prop_name, prop_schema) in props_obj {
                        let schema_constraints = parse_schema_constraints(prop_schema);

                        if schema_constraints.enum_values.is_some()
                            || schema_constraints.pattern.is_some()
                            || schema_constraints.min_length.is_some()
                            || schema_constraints.max_length.is_some()
                            || schema_constraints.min_value.is_some()
                            || schema_constraints.max_value.is_some()
                            || schema_constraints.unique_items.is_some()
                        {
                            constraints.insert(prop_name.clone(), schema_constraints);
                        }
                    }
                }
            }
        }
    }

    Ok(constraints)
}

/// Parse schema constraints from a schemas directory by searching for the resource type
fn parse_schema_from_schemas_directory(
    schemas_dir: &Path,
    resource_type: &str,
) -> Result<HashMap<String, SchemaConstraints>> {
    // Try different possible filename patterns for the resource type
    let possible_names = vec![
        format!("{}.json", resource_type.replace("::", "-")),
        format!("{}.json", resource_type.replace("::", ".")),
        format!("{}.json", resource_type.to_lowercase().replace("::", "-")),
        format!("{}.json", resource_type.to_lowercase().replace("::", ".")),
    ];

    for filename in possible_names {
        let schema_path = schemas_dir.join(&filename);
        if schema_path.exists() {
            if let Ok(constraints) = parse_schema_constraints_from_file(&schema_path) {
                return Ok(constraints);
            }
        }
    }

    // If no specific file found, try to search through all schema files
    if let Ok(entries) = fs::read_dir(schemas_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".json")
                    && filename.contains(&resource_type.replace("::", "-"))
                {
                    let schema_path = entry.path();
                    if let Ok(constraints) = parse_schema_constraints_from_file(&schema_path) {
                        return Ok(constraints);
                    }
                }
            }
        }
    }

    Err(anyhow!(
        "Schema constraints not found in directory for resource type: {}",
        resource_type
    ))
}
