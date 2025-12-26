use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_bedrockagentcorecontrol as agentcore;
use std::sync::Arc;

pub struct BedrockAgentCoreControlService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl BedrockAgentCoreControlService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    // ==================== Core Resources ====================

    /// List Agent Runtimes
    pub async fn list_agent_runtimes(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut runtimes = Vec::new();
        let mut paginator = client.list_agent_runtimes().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for runtime in page.agent_runtimes {
                runtimes.push(self.agent_runtime_to_json(&runtime));
            }
        }

        Ok(runtimes)
    }

    /// Get detailed information for a specific agent runtime
    pub async fn describe_agent_runtime(
        &self,
        account_id: &str,
        region: &str,
        runtime_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_agent_runtime()
            .agent_runtime_id(runtime_id)
            .send()
            .await?;

        Ok(self.agent_runtime_details_to_json(&response))
    }

    fn agent_runtime_to_json(&self, runtime: &agentcore::types::AgentRuntime) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentRuntimeId".to_string(),
            serde_json::Value::String(runtime.agent_runtime_id.clone()),
        );

        json.insert(
            "AgentRuntimeArn".to_string(),
            serde_json::Value::String(runtime.agent_runtime_arn.clone()),
        );

        json.insert(
            "AgentRuntimeName".to_string(),
            serde_json::Value::String(runtime.agent_runtime_name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(runtime.status.as_str().to_string()),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                runtime
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "Description".to_string(),
            serde_json::Value::String(runtime.description.clone()),
        );

        json.insert(
            "AgentRuntimeVersion".to_string(),
            serde_json::Value::String(runtime.agent_runtime_version.clone()),
        );

        serde_json::Value::Object(json)
    }

    fn agent_runtime_details_to_json(
        &self,
        response: &agentcore::operation::get_agent_runtime::GetAgentRuntimeOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentRuntimeId".to_string(),
            serde_json::Value::String(response.agent_runtime_id.clone()),
        );

        json.insert(
            "AgentRuntimeArn".to_string(),
            serde_json::Value::String(response.agent_runtime_arn.clone()),
        );

        json.insert(
            "AgentRuntimeName".to_string(),
            serde_json::Value::String(response.agent_runtime_name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                response
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    // ==================== AgentRuntimeEndpoint ====================

    /// List Agent Runtime Endpoints
    pub async fn list_agent_runtime_endpoints(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut endpoints = Vec::new();
        let mut paginator = client
            .list_agent_runtime_endpoints()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for endpoint in page.runtime_endpoints {
                endpoints.push(self.agent_runtime_endpoint_to_json(&endpoint));
            }
        }

        Ok(endpoints)
    }

    /// Get detailed information for a specific agent runtime endpoint
    pub async fn describe_agent_runtime_endpoint(
        &self,
        account_id: &str,
        region: &str,
        endpoint_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_agent_runtime_endpoint()
            .agent_runtime_id(endpoint_id)
            .send()
            .await?;

        Ok(self.agent_runtime_endpoint_details_to_json(&response))
    }

    fn agent_runtime_endpoint_to_json(
        &self,
        endpoint: &agentcore::types::AgentRuntimeEndpoint,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "EndpointId".to_string(),
            serde_json::Value::String(endpoint.id.clone()),
        );

        json.insert(
            "AgentRuntimeEndpointArn".to_string(),
            serde_json::Value::String(endpoint.agent_runtime_endpoint_arn.clone()),
        );

        json.insert(
            "EndpointName".to_string(),
            serde_json::Value::String(endpoint.name.clone()),
        );

        json.insert(
            "AgentRuntimeArn".to_string(),
            serde_json::Value::String(endpoint.agent_runtime_arn.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(endpoint.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                endpoint
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                endpoint
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(desc) = &endpoint.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }

        if let Some(live_version) = &endpoint.live_version {
            json.insert(
                "LiveVersion".to_string(),
                serde_json::Value::String(live_version.clone()),
            );
        }

        if let Some(target_version) = &endpoint.target_version {
            json.insert(
                "TargetVersion".to_string(),
                serde_json::Value::String(target_version.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn agent_runtime_endpoint_details_to_json(
        &self,
        response: &agentcore::operation::get_agent_runtime_endpoint::GetAgentRuntimeEndpointOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "EndpointId".to_string(),
            serde_json::Value::String(response.id.clone()),
        );

        json.insert(
            "AgentRuntimeEndpointArn".to_string(),
            serde_json::Value::String(response.agent_runtime_endpoint_arn.clone()),
        );

        json.insert(
            "EndpointName".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        json.insert(
            "AgentRuntimeArn".to_string(),
            serde_json::Value::String(response.agent_runtime_arn.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        if let Some(desc) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                response
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(live_version) = &response.live_version {
            json.insert(
                "LiveVersion".to_string(),
                serde_json::Value::String(live_version.clone()),
            );
        }

        if let Some(target_version) = &response.target_version {
            json.insert(
                "TargetVersion".to_string(),
                serde_json::Value::String(target_version.clone()),
            );
        }

        if let Some(failure_reason) = &response.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(failure_reason.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    // ==================== Memory ====================

    /// List Memories
    pub async fn list_memories(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut memories = Vec::new();
        let mut paginator = client.list_memories().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for memory in page.memories {
                memories.push(self.memory_to_json(&memory));
            }
        }

        Ok(memories)
    }

    /// Get detailed information for a specific memory
    pub async fn describe_memory(
        &self,
        account_id: &str,
        region: &str,
        memory_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client.get_memory().memory_id(memory_id).send().await?;

        Ok(self.memory_details_to_json(&response))
    }

    fn memory_to_json(&self, memory: &agentcore::types::MemorySummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &memory.id {
            json.insert(
                "MemoryId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(arn) = &memory.arn {
            json.insert(
                "MemoryArn".to_string(),
                serde_json::Value::String(arn.clone()),
            );
        }

        if let Some(status) = &memory.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                memory
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(
                memory
                    .updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    fn memory_details_to_json(
        &self,
        response: &agentcore::operation::get_memory::GetMemoryOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(memory) = &response.memory {
            json.insert(
                "MemoryId".to_string(),
                serde_json::Value::String(memory.id.clone()),
            );

            json.insert(
                "MemoryArn".to_string(),
                serde_json::Value::String(memory.arn.clone()),
            );

            json.insert(
                "MemoryName".to_string(),
                serde_json::Value::String(memory.name.clone()),
            );

            json.insert(
                "Status".to_string(),
                serde_json::Value::String(memory.status.as_str().to_string()),
            );

            if let Some(description) = &memory.description {
                json.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description.clone()),
                );
            }

            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(
                    memory
                        .created_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );

            json.insert(
                "UpdatedAt".to_string(),
                serde_json::Value::String(
                    memory
                        .updated_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }

    // ==================== Gateway ====================

    /// List Gateways
    pub async fn list_gateways(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut gateways = Vec::new();
        let mut paginator = client.list_gateways().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for gateway in page.items {
                gateways.push(self.gateway_to_json(&gateway));
            }
        }

        Ok(gateways)
    }

    /// Get detailed information for a specific gateway
    pub async fn describe_gateway(
        &self,
        account_id: &str,
        region: &str,
        gateway_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_gateway()
            .gateway_identifier(gateway_id)
            .send()
            .await?;

        Ok(self.gateway_details_to_json(&response))
    }

    fn gateway_to_json(&self, gateway: &agentcore::types::GatewaySummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "GatewayId".to_string(),
            serde_json::Value::String(gateway.gateway_id.clone()),
        );

        json.insert(
            "GatewayName".to_string(),
            serde_json::Value::String(gateway.name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(gateway.status.as_str().to_string()),
        );

        if let Some(desc) = &gateway.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                gateway
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(
                gateway
                    .updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "AuthorizerType".to_string(),
            serde_json::Value::String(gateway.authorizer_type.as_str().to_string()),
        );

        json.insert(
            "ProtocolType".to_string(),
            serde_json::Value::String(gateway.protocol_type.as_str().to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn gateway_details_to_json(
        &self,
        response: &agentcore::operation::get_gateway::GetGatewayOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "GatewayId".to_string(),
            serde_json::Value::String(response.gateway_id.clone()),
        );

        json.insert(
            "GatewayArn".to_string(),
            serde_json::Value::String(response.gateway_arn.clone()),
        );

        json.insert(
            "GatewayName".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                response
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(
                response
                    .updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    // ==================== Browser ====================

    /// List Browsers
    pub async fn list_browsers(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut browsers = Vec::new();
        let mut paginator = client.list_browsers().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for browser in page.browser_summaries {
                browsers.push(self.browser_to_json(&browser));
            }
        }

        Ok(browsers)
    }

    /// Get detailed information for a specific browser
    pub async fn describe_browser(
        &self,
        account_id: &str,
        region: &str,
        browser_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client.get_browser().browser_id(browser_id).send().await?;

        Ok(self.browser_details_to_json(&response))
    }

    fn browser_to_json(&self, browser: &agentcore::types::BrowserSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "BrowserId".to_string(),
            serde_json::Value::String(browser.browser_id.clone()),
        );

        json.insert(
            "BrowserArn".to_string(),
            serde_json::Value::String(browser.browser_arn.clone()),
        );

        if let Some(name) = &browser.name {
            json.insert(
                "BrowserName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(browser.status.as_str().to_string()),
        );

        if let Some(desc) = &browser.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                browser
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(last_updated_at) = &browser.last_updated_at {
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

    fn browser_details_to_json(
        &self,
        response: &agentcore::operation::get_browser::GetBrowserOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "BrowserId".to_string(),
            serde_json::Value::String(response.browser_id.clone()),
        );

        json.insert(
            "BrowserArn".to_string(),
            serde_json::Value::String(response.browser_arn.clone()),
        );

        json.insert(
            "BrowserName".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                response
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(failure_reason) = &response.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(failure_reason.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    // ==================== Additional Control Plane Resources ====================

    /// List Code Interpreters
    pub async fn list_code_interpreters(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut interpreters = Vec::new();
        let mut paginator = client.list_code_interpreters().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for interpreter in page.code_interpreter_summaries {
                interpreters.push(self.code_interpreter_summary_to_json(&interpreter));
            }
        }

        Ok(interpreters)
    }

    /// Get detailed information for a specific code interpreter
    pub async fn describe_code_interpreter(
        &self,
        account_id: &str,
        region: &str,
        interpreter_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_code_interpreter()
            .code_interpreter_id(interpreter_id)
            .send()
            .await?;

        Ok(self.code_interpreter_details_to_json(&response))
    }

    fn code_interpreter_summary_to_json(
        &self,
        interpreter: &agentcore::types::CodeInterpreterSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "CodeInterpreterId".to_string(),
            serde_json::Value::String(interpreter.code_interpreter_id.clone()),
        );

        json.insert(
            "CodeInterpreterArn".to_string(),
            serde_json::Value::String(interpreter.code_interpreter_arn.clone()),
        );

        if let Some(name) = &interpreter.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &interpreter.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(interpreter.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                interpreter
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(last_updated_at) = &interpreter.last_updated_at {
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

    fn code_interpreter_details_to_json(
        &self,
        response: &agentcore::operation::get_code_interpreter::GetCodeInterpreterOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "CodeInterpreterId".to_string(),
            serde_json::Value::String(response.code_interpreter_id.clone()),
        );

        json.insert(
            "CodeInterpreterArn".to_string(),
            serde_json::Value::String(response.code_interpreter_arn.clone()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(execution_role_arn) = &response.execution_role_arn {
            json.insert(
                "ExecutionRoleArn".to_string(),
                serde_json::Value::String(execution_role_arn.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        if let Some(failure_reason) = &response.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(failure_reason.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                response
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    /// List API Key Credential Providers
    pub async fn list_api_key_credential_providers(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut providers = Vec::new();
        let mut paginator = client
            .list_api_key_credential_providers()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for provider in page.credential_providers {
                providers.push(self.api_key_credential_provider_to_json(&provider));
            }
        }

        Ok(providers)
    }

    /// Get detailed information for a specific API key credential provider
    pub async fn describe_api_key_credential_provider(
        &self,
        account_id: &str,
        region: &str,
        provider_name: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_api_key_credential_provider()
            .name(provider_name)
            .send()
            .await?;

        Ok(self.api_key_credential_provider_details_to_json(&response))
    }

    fn api_key_credential_provider_to_json(
        &self,
        provider: &agentcore::types::ApiKeyCredentialProviderItem,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(provider.name.clone()),
        );

        json.insert(
            "CredentialProviderArn".to_string(),
            serde_json::Value::String(provider.credential_provider_arn.clone()),
        );

        json.insert(
            "CreatedTime".to_string(),
            serde_json::Value::String(
                provider
                    .created_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedTime".to_string(),
            serde_json::Value::String(
                provider
                    .last_updated_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    fn api_key_credential_provider_details_to_json(
        &self,
        response: &agentcore::operation::get_api_key_credential_provider::GetApiKeyCredentialProviderOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        json.insert(
            "CredentialProviderArn".to_string(),
            serde_json::Value::String(response.credential_provider_arn.clone()),
        );

        json.insert(
            "CreatedTime".to_string(),
            serde_json::Value::String(
                response
                    .created_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedTime".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    /// List OAuth2 Credential Providers
    pub async fn list_oauth2_credential_providers(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut providers = Vec::new();
        let mut paginator = client
            .list_oauth2_credential_providers()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for provider in page.credential_providers {
                providers.push(self.oauth2_credential_provider_to_json(&provider));
            }
        }

        Ok(providers)
    }

    /// Get detailed information for a specific OAuth2 credential provider
    pub async fn describe_oauth2_credential_provider(
        &self,
        account_id: &str,
        region: &str,
        provider_name: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_oauth2_credential_provider()
            .name(provider_name)
            .send()
            .await?;

        Ok(self.oauth2_credential_provider_details_to_json(&response))
    }

    fn oauth2_credential_provider_to_json(
        &self,
        provider: &agentcore::types::Oauth2CredentialProviderItem,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(provider.name.clone()),
        );

        json.insert(
            "CredentialProviderVendor".to_string(),
            serde_json::Value::String(provider.credential_provider_vendor.as_str().to_string()),
        );

        json.insert(
            "CredentialProviderArn".to_string(),
            serde_json::Value::String(provider.credential_provider_arn.clone()),
        );

        json.insert(
            "CreatedTime".to_string(),
            serde_json::Value::String(
                provider
                    .created_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedTime".to_string(),
            serde_json::Value::String(
                provider
                    .last_updated_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    fn oauth2_credential_provider_details_to_json(
        &self,
        response: &agentcore::operation::get_oauth2_credential_provider::GetOauth2CredentialProviderOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        json.insert(
            "CredentialProviderArn".to_string(),
            serde_json::Value::String(response.credential_provider_arn.clone()),
        );

        json.insert(
            "CredentialProviderVendor".to_string(),
            serde_json::Value::String(response.credential_provider_vendor.as_str().to_string()),
        );

        if let Some(callback_url) = &response.callback_url {
            json.insert(
                "CallbackUrl".to_string(),
                serde_json::Value::String(callback_url.clone()),
            );
        }

        json.insert(
            "CreatedTime".to_string(),
            serde_json::Value::String(
                response
                    .created_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedTime".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    /// List Workload Identities
    pub async fn list_workload_identities(
        &self,
        account_id: &str,
        region: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut identities = Vec::new();
        let mut paginator = client.list_workload_identities().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for identity in page.workload_identities {
                identities.push(self.workload_identity_to_json(&identity));
            }
        }

        Ok(identities)
    }

    /// Get detailed information for a specific workload identity
    pub async fn describe_workload_identity(
        &self,
        account_id: &str,
        region: &str,
        identity_name: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_workload_identity()
            .name(identity_name)
            .send()
            .await?;

        Ok(self.workload_identity_details_to_json(&response))
    }

    fn workload_identity_to_json(
        &self,
        identity: &agentcore::types::WorkloadIdentityType,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(identity.name.clone()),
        );

        json.insert(
            "WorkloadIdentityArn".to_string(),
            serde_json::Value::String(identity.workload_identity_arn.clone()),
        );

        serde_json::Value::Object(json)
    }

    fn workload_identity_details_to_json(
        &self,
        response: &agentcore::operation::get_workload_identity::GetWorkloadIdentityOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        json.insert(
            "WorkloadIdentityArn".to_string(),
            serde_json::Value::String(response.workload_identity_arn.clone()),
        );

        json.insert(
            "CreatedTime".to_string(),
            serde_json::Value::String(
                response
                    .created_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "LastUpdatedTime".to_string(),
            serde_json::Value::String(
                response
                    .last_updated_time
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    /// List Agent Runtime Versions (child resource - requires parent runtime ID)
    pub async fn list_agent_runtime_versions(
        &self,
        account_id: &str,
        region: &str,
        runtime_id: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut versions = Vec::new();
        let mut paginator = client
            .list_agent_runtime_versions()
            .agent_runtime_id(runtime_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for version in page.agent_runtimes {
                versions.push(self.agent_runtime_version_to_json(&version, runtime_id));
            }
        }

        Ok(versions)
    }

    fn agent_runtime_version_to_json(
        &self,
        version: &agentcore::types::AgentRuntime,
        parent_runtime_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentRuntimeId".to_string(),
            serde_json::Value::String(version.agent_runtime_id.clone()),
        );

        json.insert(
            "ParentRuntimeId".to_string(),
            serde_json::Value::String(parent_runtime_id.to_string()),
        );

        json.insert(
            "AgentRuntimeArn".to_string(),
            serde_json::Value::String(version.agent_runtime_arn.clone()),
        );

        json.insert(
            "AgentRuntimeVersion".to_string(),
            serde_json::Value::String(version.agent_runtime_version.clone()),
        );

        json.insert(
            "AgentRuntimeName".to_string(),
            serde_json::Value::String(version.agent_runtime_name.clone()),
        );

        json.insert(
            "Description".to_string(),
            serde_json::Value::String(version.description.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(version.status.as_str().to_string()),
        );

        json.insert(
            "LastUpdatedAt".to_string(),
            serde_json::Value::String(
                version
                    .last_updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    /// List Gateway Targets (child resource - requires parent gateway ID)
    pub async fn list_gateway_targets(
        &self,
        account_id: &str,
        region: &str,
        gateway_id: &str,
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

        let client = agentcore::Client::new(&aws_config);

        let mut targets = Vec::new();
        let mut paginator = client
            .list_gateway_targets()
            .gateway_identifier(gateway_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for target in page.items {
                targets.push(self.gateway_target_to_json(&target, gateway_id));
            }
        }

        Ok(targets)
    }

    /// Get detailed information for a specific gateway target
    pub async fn describe_gateway_target(
        &self,
        account_id: &str,
        region: &str,
        gateway_id: &str,
        target_id: &str,
    ) -> Result<serde_json::Value> {
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

        let client = agentcore::Client::new(&aws_config);
        let response = client
            .get_gateway_target()
            .gateway_identifier(gateway_id)
            .target_id(target_id)
            .send()
            .await?;

        Ok(self.gateway_target_details_to_json(&response, gateway_id))
    }

    fn gateway_target_to_json(
        &self,
        target: &agentcore::types::TargetSummary,
        parent_gateway_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "TargetId".to_string(),
            serde_json::Value::String(target.target_id.clone()),
        );

        json.insert(
            "ParentGatewayId".to_string(),
            serde_json::Value::String(parent_gateway_id.to_string()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(target.name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(target.status.as_str().to_string()),
        );

        if let Some(description) = &target.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                target
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(
                target
                    .updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        serde_json::Value::Object(json)
    }

    fn gateway_target_details_to_json(
        &self,
        response: &agentcore::operation::get_gateway_target::GetGatewayTargetOutput,
        parent_gateway_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "TargetId".to_string(),
            serde_json::Value::String(response.target_id.clone()),
        );

        json.insert(
            "ParentGatewayId".to_string(),
            serde_json::Value::String(parent_gateway_id.to_string()),
        );

        json.insert(
            "GatewayArn".to_string(),
            serde_json::Value::String(response.gateway_arn.clone()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                response
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(
                response
                    .updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        if let Some(last_synchronized_at) = &response.last_synchronized_at {
            json.insert(
                "LastSynchronizedAt".to_string(),
                serde_json::Value::String(
                    last_synchronized_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        serde_json::Value::Object(json)
    }
}
