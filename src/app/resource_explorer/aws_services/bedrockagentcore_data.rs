use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_bedrockagentcore as agentcore_data;
use std::sync::Arc;

pub struct BedrockAgentCoreDataService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl BedrockAgentCoreDataService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    // ==================== Data Plane Resources ====================

    /// List Memory Records (child resource - requires parent memory_id)
    pub async fn list_memory_records(
        &self,
        account_id: &str,
        region: &str,
        memory_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = agentcore_data::Client::new(&aws_config);

        let mut records = Vec::new();
        let mut paginator = client
            .list_memory_records()
            .memory_id(memory_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for record in page.memory_record_summaries {
                records.push(self.memory_record_to_json(&record, memory_id));
            }
        }

        Ok(records)
    }

    fn memory_record_to_json(
        &self,
        record: &agentcore_data::types::MemoryRecordSummary,
        parent_memory_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "MemoryRecordId".to_string(),
            serde_json::Value::String(record.memory_record_id.clone()),
        );

        json.insert(
            "ParentMemoryId".to_string(),
            serde_json::Value::String(parent_memory_id.to_string()),
        );

        json.insert(
            "MemoryStrategyId".to_string(),
            serde_json::Value::String(record.memory_strategy_id.clone()),
        );

        // Namespaces as JSON array
        let namespaces: Vec<serde_json::Value> = record
            .namespaces
            .iter()
            .map(|ns| serde_json::Value::String(ns.clone()))
            .collect();
        json.insert("Namespaces".to_string(), serde_json::Value::Array(namespaces));

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                record
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(score) = record.score {
            json.insert(
                "Score".to_string(),
                serde_json::Value::Number(serde_json::Number::from_f64(score).unwrap_or(serde_json::Number::from(0))),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List Events (child resource - requires parent memory_id)
    pub async fn list_events(
        &self,
        account_id: &str,
        region: &str,
        memory_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = agentcore_data::Client::new(&aws_config);

        let mut events = Vec::new();
        let mut paginator = client
            .list_events()
            .memory_id(memory_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for event in page.events {
                events.push(self.event_to_json(&event));
            }
        }

        Ok(events)
    }

    fn event_to_json(
        &self,
        event: &agentcore_data::types::Event,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "EventId".to_string(),
            serde_json::Value::String(event.event_id.clone()),
        );

        json.insert(
            "MemoryId".to_string(),
            serde_json::Value::String(event.memory_id.clone()),
        );

        json.insert(
            "ActorId".to_string(),
            serde_json::Value::String(event.actor_id.clone()),
        );

        json.insert(
            "SessionId".to_string(),
            serde_json::Value::String(event.session_id.clone()),
        );

        json.insert(
            "EventTimestamp".to_string(),
            serde_json::Value::String(
                event
                    .event_timestamp
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    /// List Browser Sessions (child resource - requires parent browser_identifier)
    pub async fn list_browser_sessions(
        &self,
        account_id: &str,
        region: &str,
        browser_identifier: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = agentcore_data::Client::new(&aws_config);

        let mut sessions = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client
                .list_browser_sessions()
                .browser_identifier(browser_identifier);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            for session in response.items {
                sessions.push(self.browser_session_to_json(&session, browser_identifier));
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(sessions)
    }

    fn browser_session_to_json(
        &self,
        session: &agentcore_data::types::BrowserSessionSummary,
        parent_browser_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "SessionId".to_string(),
            serde_json::Value::String(session.session_id.clone()),
        );

        json.insert(
            "ParentBrowserId".to_string(),
            serde_json::Value::String(parent_browser_id.to_string()),
        );

        json.insert(
            "BrowserIdentifier".to_string(),
            serde_json::Value::String(session.browser_identifier.clone()),
        );

        if let Some(name) = &session.name {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(session.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                session
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(last_updated_at) = &session.last_updated_at {
            json.insert(
                "LastUpdatedAt".to_string(),
                serde_json::Value::String(
                    last_updated_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List Code Interpreter Sessions (child resource - requires parent code_interpreter_identifier)
    pub async fn list_code_interpreter_sessions(
        &self,
        account_id: &str,
        region: &str,
        code_interpreter_identifier: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = agentcore_data::Client::new(&aws_config);

        let mut sessions = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client
                .list_code_interpreter_sessions()
                .code_interpreter_identifier(code_interpreter_identifier);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            for session in response.items {
                sessions.push(self.code_interpreter_session_to_json(&session, code_interpreter_identifier));
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(sessions)
    }

    fn code_interpreter_session_to_json(
        &self,
        session: &agentcore_data::types::CodeInterpreterSessionSummary,
        parent_interpreter_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "SessionId".to_string(),
            serde_json::Value::String(session.session_id.clone()),
        );

        json.insert(
            "ParentCodeInterpreterId".to_string(),
            serde_json::Value::String(parent_interpreter_id.to_string()),
        );

        json.insert(
            "CodeInterpreterIdentifier".to_string(),
            serde_json::Value::String(session.code_interpreter_identifier.clone()),
        );

        if let Some(name) = &session.name {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(session.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                session
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(last_updated_at) = &session.last_updated_at {
            json.insert(
                "LastUpdatedAt".to_string(),
                serde_json::Value::String(
                    last_updated_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }
}
