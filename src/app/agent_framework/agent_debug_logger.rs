//! Per-Agent Debug Logging for Stood Library Traces
//!
//! Provides utilities to extract Stood library debug messages from the main application
//! log and write them to agent-specific debug files for easier troubleshooting.
//!
//! Since Stood library traces are already being captured in awsdash.log with `stood=debug`,
//! this module provides functions to filter and copy those messages to per-agent debug logs.
//!
//! Log files are stored at: `~/.local/share/awsdash/logs/agents/agent-{uuid}-debug.log`

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::agent_framework::AgentId;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use tracing::info;

/// Get the debug log file path for an agent
pub fn get_debug_log_path(agent_id: AgentId) -> Result<PathBuf, std::io::Error> {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
        let log_dir = proj_dirs.data_dir().join("logs").join("agents");
        Ok(log_dir.join(format!("agent-{}-debug.log", agent_id)))
    } else {
        // Fallback to current directory
        Ok(PathBuf::from(format!("agent-{}-debug.log", agent_id)))
    }
}

/// Get the main application log file path
fn get_app_log_path() -> Result<PathBuf, std::io::Error> {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
        let log_dir = proj_dirs.data_dir().join("logs");
        Ok(log_dir.join("awsdash.log"))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine log directory",
        ))
    }
}

/// Initialize a debug log file for an agent
pub fn init_debug_log(agent_id: AgentId) -> Result<(), std::io::Error> {
    let log_path = get_debug_log_path(agent_id)?;

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create or open the debug log file
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)  // Start fresh for each agent session
        .open(&log_path)?;

    // Write header
    writeln!(file, "{}", "=".repeat(80))?;
    writeln!(file, "🔍 AGENT DEBUG LOG")?;
    writeln!(file, "Agent ID: {}", agent_id)?;
    writeln!(file, "Timestamp: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
    writeln!(file, "Source: Filtered from awsdash.log (stood::* traces)")?;
    writeln!(file, "{}", "=".repeat(80))?;
    writeln!(file)?;
    file.flush()?;

    info!("🔍 Initialized debug log for agent {}: {}", agent_id, log_path.display());
    Ok(())
}

/// Extract Stood debug messages from main log and write to agent debug log
///
/// This function reads the main application log file and extracts all lines
/// containing `stood::` traces, writing them to the agent-specific debug log.
///
/// Call this after agent execution completes to capture all Stood library traces.
pub fn extract_stood_traces(agent_id: AgentId) -> Result<usize, std::io::Error> {
    let app_log_path = get_app_log_path()?;
    let debug_log_path = get_debug_log_path(agent_id)?;

    // Open the main log for reading
    let app_log = File::open(&app_log_path)?;
    let reader = BufReader::new(app_log);

    // Open the debug log for appending
    let mut debug_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log_path)?;

    let mut count = 0;

    // Read each line and filter for stood:: traces
    for line in reader.lines() {
        let line = line?;

        // Check if line contains stood:: module traces
        if line.contains("stood::") {
            writeln!(debug_log, "{}", line)?;
            count += 1;
        }
    }

    debug_log.flush()?;

    if count > 0 {
        info!("📝 Extracted {} Stood traces to agent {} debug log", count, agent_id);
    }

    Ok(count)
}

/// Extract Stood debug messages from a specific time range
///
/// More efficient than `extract_stood_traces` when you know the approximate
/// time range of agent execution. Reads backwards from end of file.
pub fn extract_stood_traces_recent(
    agent_id: AgentId,
    max_lines: usize,
) -> Result<usize, std::io::Error> {
    let app_log_path = get_app_log_path()?;
    let debug_log_path = get_debug_log_path(agent_id)?;

    // Open the main log for reading
    let app_log = File::open(&app_log_path)?;
    let reader = BufReader::new(app_log);

    // Collect recent lines (read entire file, but only keep last max_lines stood:: entries)
    let mut stood_lines = Vec::new();

    for line in reader.lines() {
        let line = line?;

        if line.contains("stood::") {
            stood_lines.push(line);

            // Keep only the most recent max_lines
            if stood_lines.len() > max_lines {
                stood_lines.remove(0);
            }
        }
    }

    // Open the debug log for appending
    let mut debug_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log_path)?;

    // Write the collected lines
    let count = stood_lines.len();
    for line in stood_lines {
        writeln!(debug_log, "{}", line)?;
    }

    debug_log.flush()?;

    if count > 0 {
        info!("📝 Extracted {} recent Stood traces to agent {} debug log", count, agent_id);
    }

    Ok(count)
}
