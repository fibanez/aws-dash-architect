//! Agent Cancellation Management
//!
//! Provides cancellation token infrastructure for stopping running agents created via create_task tool.
//! Integrates with the Bridge UI Stop button to provide real agent cancellation, not just UI state reset.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Manages cancellation tokens for active agents
#[derive(Debug, Clone)]
pub struct AgentCancellationManager {
    /// Maps agent_id to cancellation token
    active_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl AgentCancellationManager {
    /// Create a new cancellation manager
    pub fn new() -> Self {
        Self {
            active_tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new cancellation token for an agent
    pub fn create_token(&self, agent_id: String) -> CancellationToken {
        let token = CancellationToken::new();

        // Store the token for later cancellation
        match self.active_tokens.lock() {
            Ok(mut tokens) => {
                tokens.insert(agent_id.clone(), token.clone());
                debug!("🎯 Created cancellation token for agent: {}", agent_id);
            }
            Err(e) => {
                warn!(
                    "Failed to store cancellation token for agent {}: {}",
                    agent_id, e
                );
            }
        }

        token
    }

    /// Cancel a specific agent by ID
    pub fn cancel_agent(&self, agent_id: &str) -> bool {
        match self.active_tokens.lock() {
            Ok(mut tokens) => {
                if let Some(token) = tokens.remove(agent_id) {
                    info!("🛑 Cancelling agent: {}", agent_id);
                    token.cancel();
                    true
                } else {
                    debug!("No cancellation token found for agent: {}", agent_id);
                    false
                }
            }
            Err(e) => {
                warn!("Failed to cancel agent {}: {}", agent_id, e);
                false
            }
        }
    }

    /// Cancel all active agents
    pub fn cancel_all(&self) -> usize {
        match self.active_tokens.lock() {
            Ok(mut tokens) => {
                let count = tokens.len();
                if count > 0 {
                    info!("🛑 Cancelling {} active agents", count);
                    for (agent_id, token) in tokens.drain() {
                        debug!("🛑 Cancelling agent: {}", agent_id);
                        token.cancel();
                    }
                    count
                } else {
                    debug!("No active agents to cancel");
                    0
                }
            }
            Err(e) => {
                warn!("Failed to cancel all agents: {}", e);
                0
            }
        }
    }

    /// Remove a token when an agent completes normally
    pub fn remove_token(&self, agent_id: &str) {
        match self.active_tokens.lock() {
            Ok(mut tokens) => {
                if tokens.remove(agent_id).is_some() {
                    debug!(
                        "✅ Removed cancellation token for completed agent: {}",
                        agent_id
                    );
                } else {
                    debug!(
                        "No cancellation token found to remove for agent: {}",
                        agent_id
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Failed to remove cancellation token for agent {}: {}",
                    agent_id, e
                );
            }
        }
    }

    /// Get the number of active agents
    pub fn active_count(&self) -> usize {
        self.active_tokens
            .lock()
            .map(|tokens| tokens.len())
            .unwrap_or(0)
    }

    /// Check if a specific agent has an active token
    pub fn has_active_token(&self, agent_id: &str) -> bool {
        self.active_tokens
            .lock()
            .map(|tokens| tokens.contains_key(agent_id))
            .unwrap_or(false)
    }

    /// Get all active agent IDs
    pub fn get_active_agent_ids(&self) -> Vec<String> {
        self.active_tokens
            .lock()
            .map(|tokens| tokens.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for AgentCancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_cancellation_manager() {
        let manager = AgentCancellationManager::new();

        // Test token creation
        let agent_id = "test-agent-123".to_string();
        let token = manager.create_token(agent_id.clone());

        assert_eq!(manager.active_count(), 1);
        assert!(manager.has_active_token(&agent_id));
        assert!(!token.is_cancelled());

        // Test individual cancellation
        assert!(manager.cancel_agent(&agent_id));
        assert!(token.is_cancelled());
        assert_eq!(manager.active_count(), 0);

        // Test cancelling non-existent agent
        assert!(!manager.cancel_agent("non-existent"));
    }

    #[tokio::test]
    async fn test_cancel_all() {
        let manager = AgentCancellationManager::new();

        // Create multiple tokens
        let token1 = manager.create_token("agent1".to_string());
        let token2 = manager.create_token("agent2".to_string());
        let token3 = manager.create_token("agent3".to_string());

        assert_eq!(manager.active_count(), 3);
        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());
        assert!(!token3.is_cancelled());

        // Cancel all
        let cancelled_count = manager.cancel_all();
        assert_eq!(cancelled_count, 3);
        assert_eq!(manager.active_count(), 0);

        // All tokens should be cancelled
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
        assert!(token3.is_cancelled());
    }

    #[tokio::test]
    async fn test_token_usage_pattern() {
        let manager = AgentCancellationManager::new();
        let agent_id = "test-agent".to_string();
        let token = manager.create_token(agent_id.clone());

        // Simulate agent work with cancellation check
        let work_result = tokio::select! {
            _ = async {
                for i in 0..10 {
                    tokio::select! {
                        _ = sleep(Duration::from_millis(100)) => {
                            // Work continues
                        }
                        _ = token.cancelled() => {
                            return "cancelled";
                        }
                    }
                }
                "completed"
            } => "work_result",
            _ = async {
                sleep(Duration::from_millis(250)).await;
                manager.cancel_agent(&agent_id);
            } => "cancel_result"
        };

        // Work should be cancelled before completion
        assert!(token.is_cancelled());
        assert_eq!(manager.active_count(), 0);
    }
}
