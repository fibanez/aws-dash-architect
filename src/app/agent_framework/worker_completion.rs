//! Worker Completion Channel - Async tool result delivery
//!
//! This module provides a channel-based mechanism for delivering worker agent
//! results back to the start_task tool as proper tool results, not user messages.
//!
//! ## Architecture
//!
//! When a TaskManager uses the start_task tool:
//! 1. Tool creates worker agent and blocks waiting for completion
//! 2. Worker executes in background (AgentInstance thread)
//! 3. Worker completes and sends result to this channel
//! 4. Tool unblocks and returns worker result as ToolResult
//! 5. LLM sees: "start_task returned: {worker result}"
//!
//! This ensures worker results appear as tool results in the LLM conversation,
//! not as unexpected user messages.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::agent_framework::AgentId;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Result of a worker agent's execution
#[derive(Debug, Clone)]
pub struct WorkerCompletion {
    /// ID of the worker agent that completed
    pub worker_id: AgentId,

    /// Worker result: Ok(response) or Err(error message)
    /// Contains raw worker output without wrapper text
    pub result: Result<String, String>,

    /// How long the worker took to execute
    pub execution_time: Duration,
}

/// Global registry of pending worker completions
/// Maps worker_id -> (result, condvar for notification)
type CompletionRegistry = Arc<Mutex<HashMap<AgentId, (Option<WorkerCompletion>, Arc<Condvar>)>>>;

static COMPLETION_REGISTRY: once_cell::sync::Lazy<CompletionRegistry> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Register a worker as pending completion
///
/// Called by start_task tool before waiting.
/// Returns a condvar that will be notified when the worker completes.
fn register_pending_worker(worker_id: AgentId) -> Arc<Condvar> {
    let condvar = Arc::new(Condvar::new());
    let mut registry = COMPLETION_REGISTRY.lock().unwrap();
    registry.insert(worker_id, (None, Arc::clone(&condvar)));
    condvar
}

/// Send a worker completion result
///
/// Called by AgentManagerWindow when a worker completes.
/// Notifies any thread waiting for this worker's result.
pub fn send_worker_completion(completion: WorkerCompletion) {
    stood::perf_checkpoint!("awsdash.worker_completion.send.start", &format!("worker_id={}", completion.worker_id));
    let worker_id = completion.worker_id;
    let execution_time_ms = completion.execution_time.as_millis();
    let is_success = completion.result.is_ok();

    let mut registry = COMPLETION_REGISTRY.lock().unwrap();

    if let Some((result_slot, condvar)) = registry.get_mut(&worker_id) {
        // Store the result
        *result_slot = Some(completion);

        // Notify waiting thread
        condvar.notify_one();

        stood::perf_checkpoint!("awsdash.worker_completion.send.notified", &format!("worker_id={}, success={}, execution_time_ms={}", worker_id, is_success, execution_time_ms));
        tracing::info!(
            target: "agent::worker_completion",
            worker_id = %worker_id,
            "Worker completion sent and waiting thread notified"
        );
    } else {
        stood::perf_checkpoint!("awsdash.worker_completion.send.no_waiter", &format!("worker_id={}", worker_id));
        tracing::warn!(
            target: "agent::worker_completion",
            worker_id = %worker_id,
            "Worker completed but no thread is waiting for result"
        );
    }
}

/// Wait for a worker to complete and return its result
///
/// Called by start_task tool after creating the worker.
/// Blocks until the worker completes or timeout is reached.
///
/// # Arguments
///
/// * `worker_id` - ID of the worker to wait for
/// * `timeout` - Maximum time to wait (default: 5 minutes)
///
/// # Returns
///
/// * `Ok(response)` - Worker completed successfully with response text
/// * `Err(error)` - Worker failed with error message or timeout
pub fn wait_for_worker_completion(worker_id: AgentId, timeout: Duration) -> Result<String, String> {
    stood::perf_checkpoint!("awsdash.worker_completion.wait.start", &format!("worker_id={}, timeout_secs={}", worker_id, timeout.as_secs()));
    let _wait_guard = stood::perf_guard!("awsdash.worker_completion.wait", &format!("worker_id={}", worker_id));

    tracing::info!(
        target: "agent::worker_completion",
        worker_id = %worker_id,
        timeout_secs = timeout.as_secs(),
        "Waiting for worker to complete"
    );

    // Register as pending and get condvar
    let condvar = register_pending_worker(worker_id);

    // Wait for completion with timeout
    stood::perf_checkpoint!("awsdash.worker_completion.wait.condvar_wait.start", &format!("worker_id={}", worker_id));
    let result = {
        let registry = COMPLETION_REGISTRY.lock().unwrap();

        // Wait for notification or timeout
        let (mut guard, timeout_result) = condvar
            .wait_timeout_while(registry, timeout, |reg| {
                // Keep waiting while result is None
                reg.get(&worker_id)
                    .and_then(|(result, _)| result.as_ref())
                    .is_none()
            })
            .unwrap();

        // Check if we timed out
        if timeout_result.timed_out() {
            stood::perf_checkpoint!("awsdash.worker_completion.wait.timeout", &format!("worker_id={}", worker_id));
            // Clean up registry entry
            guard.remove(&worker_id);
            return Err(format!(
                "Worker execution timeout after {} seconds",
                timeout.as_secs()
            ));
        }

        stood::perf_checkpoint!("awsdash.worker_completion.wait.condvar_wait.notified", &format!("worker_id={}", worker_id));

        // Extract result and clean up registry
        guard
            .remove(&worker_id)
            .and_then(|(result, _)| result)
            .expect("Result should be present after condvar notification")
    };

    stood::perf_checkpoint!("awsdash.worker_completion.wait.complete", &format!("worker_id={}, success={}, execution_time_ms={}", worker_id, result.result.is_ok(), result.execution_time.as_millis()));

    tracing::info!(
        target: "agent::worker_completion",
        worker_id = %worker_id,
        success = result.result.is_ok(),
        execution_time_ms = result.execution_time.as_millis(),
        "Worker completion received"
    );

    result.result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn test_worker_completion_success() {
        let worker_id = AgentId::new();
        let expected_result = "Task completed with data".to_string();

        // Spawn thread to wait for completion
        let worker_id_clone = worker_id;
        let handle = thread::spawn(move || {
            wait_for_worker_completion(worker_id_clone, Duration::from_secs(10))
        });

        // Give waiting thread time to register
        thread::sleep(Duration::from_millis(100));

        // Send completion
        let completion = WorkerCompletion {
            worker_id,
            result: Ok(expected_result.clone()),
            execution_time: Duration::from_secs(2),
        };
        send_worker_completion(completion);

        // Verify result
        let result = handle.join().unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_result);
    }

    #[test]
    fn test_worker_completion_error() {
        let worker_id = AgentId::new();
        let expected_error = "Worker failed: connection timeout".to_string();

        // Spawn thread to wait for completion
        let worker_id_clone = worker_id;
        let handle = thread::spawn(move || {
            wait_for_worker_completion(worker_id_clone, Duration::from_secs(10))
        });

        // Give waiting thread time to register
        thread::sleep(Duration::from_millis(100));

        // Send error
        let completion = WorkerCompletion {
            worker_id,
            result: Err(expected_error.clone()),
            execution_time: Duration::from_secs(1),
        };
        send_worker_completion(completion);

        // Verify error
        let result = handle.join().unwrap();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), expected_error);
    }

    #[test]
    fn test_worker_completion_timeout() {
        let worker_id = AgentId::new();

        // Wait with short timeout, no completion sent
        let start = Instant::now();
        let result = wait_for_worker_completion(worker_id, Duration::from_millis(500));
        let elapsed = start.elapsed();

        // Should timeout
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timeout"));
        assert!(elapsed >= Duration::from_millis(500));
        assert!(elapsed < Duration::from_secs(1)); // Should not hang
    }
}
