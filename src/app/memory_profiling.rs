//! Memory profiling utilities using dhat
//!
//! This module provides checkpoint functions for tracking memory usage at key points
//! in the application lifecycle. Only active when compiled with --features dhat-heap.
//!
//! # Usage
//!
//! ```ignore
//! use crate::app::memory_profiling::memory_checkpoint;
//!
//! memory_checkpoint("program_start");
//! // ... do work ...
//! memory_checkpoint("after_query_complete");
//! ```
//!
//! # Build with profiling
//!
//! ```bash
//! cargo build --features dhat-heap
//! ```
//!
//! After running, view results at: file://./dhat-heap.json using DHAT viewer

/// Record a memory checkpoint with a label
///
/// When dhat-heap feature is enabled, this logs current memory statistics
/// to help identify memory hotspots throughout the application lifecycle.
///
/// # Arguments
///
/// * `label` - A descriptive label for this checkpoint (e.g., "after_iam_login")
///
/// # Example
///
/// ```ignore
/// memory_checkpoint("program_start");
/// load_resources();
/// memory_checkpoint("after_resources_loaded");
/// ```
#[cfg(feature = "dhat-heap")]
pub fn memory_checkpoint(label: &str) {
    let stats = dhat::HeapStats::get();

    let total_blocks = stats.total_blocks;
    let total_bytes = stats.total_bytes;
    let curr_blocks = stats.curr_blocks;
    let curr_bytes = stats.curr_bytes;
    let max_blocks = stats.max_blocks;
    let max_bytes = stats.max_bytes;

    tracing::info!(
        "[MEMORY CHECKPOINT: {}] total: {} blocks / {:.2} MB, current: {} blocks / {:.2} MB, peak: {} blocks / {:.2} MB",
        label,
        total_blocks,
        total_bytes as f64 / 1_048_576.0,
        curr_blocks,
        curr_bytes as f64 / 1_048_576.0,
        max_blocks,
        max_bytes as f64 / 1_048_576.0
    );
}

/// No-op version when dhat-heap feature is not enabled
#[cfg(not(feature = "dhat-heap"))]
pub fn memory_checkpoint(_label: &str) {
    // No-op in non-profiling builds
}
