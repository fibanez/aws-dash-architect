use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_bedrockagent as bedrockagent;
use std::sync::Arc;

pub struct BedrockAgentService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl BedrockAgentService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Bedrock agents
    pub async fn list_agents(
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut agents = Vec::new();
        let mut paginator = client.list_agents().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for agent in page.agent_summaries {
                agents.push(self.agent_to_json(&agent));
            }
        }

        Ok(agents)
    }

    /// Get detailed information for a specific agent
    pub async fn describe_agent(
        &self,
        account_id: &str,
        region: &str,
        agent_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client.get_agent().agent_id(agent_id).send().await?;

        if let Some(agent_details) = response.agent {
            Ok(self.agent_details_to_json(&agent_details))
        } else {
            Err(anyhow::anyhow!("Agent {} not found", agent_id))
        }
    }

    fn agent_to_json(&self, agent: &bedrockagent::types::AgentSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentId".to_string(),
            serde_json::Value::String(agent.agent_id.clone()),
        );
        json.insert(
            "AgentName".to_string(),
            serde_json::Value::String(agent.agent_name.clone()),
        );

        json.insert(
            "AgentStatus".to_string(),
            serde_json::Value::String(agent.agent_status.as_str().to_string()),
        );

        if let Some(description) = &agent.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(latest_version) = &agent.latest_agent_version {
            json.insert(
                "LatestAgentVersion".to_string(),
                serde_json::Value::String(latest_version.clone()),
            );
        }

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(agent.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn agent_details_to_json(&self, agent: &bedrockagent::types::Agent) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentId".to_string(),
            serde_json::Value::String(agent.agent_id.clone()),
        );
        json.insert(
            "AgentArn".to_string(),
            serde_json::Value::String(agent.agent_arn.clone()),
        );
        json.insert(
            "AgentName".to_string(),
            serde_json::Value::String(agent.agent_name.clone()),
        );

        json.insert(
            "AgentStatus".to_string(),
            serde_json::Value::String(agent.agent_status.as_str().to_string()),
        );

        if let Some(description) = &agent.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "AgentResourceRoleArn".to_string(),
            serde_json::Value::String(agent.agent_resource_role_arn.clone()),
        );

        if let Some(foundation_model) = &agent.foundation_model {
            json.insert(
                "FoundationModel".to_string(),
                serde_json::Value::String(foundation_model.clone()),
            );
        }

        if let Some(instruction) = &agent.instruction {
            json.insert(
                "Instruction".to_string(),
                serde_json::Value::String(instruction.clone()),
            );
        }

        json.insert(
            "IdleSessionTtlInSeconds".to_string(),
            serde_json::Value::Number(agent.idle_session_ttl_in_seconds.into()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(agent.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(agent.updated_at.to_string()),
        );

        if let Some(prepared_at) = &agent.prepared_at {
            json.insert(
                "PreparedAt".to_string(),
                serde_json::Value::String(prepared_at.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List Bedrock knowledge bases
    pub async fn list_knowledge_bases(
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut knowledge_bases = Vec::new();
        let mut paginator = client.list_knowledge_bases().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for kb in page.knowledge_base_summaries {
                knowledge_bases.push(self.knowledge_base_to_json(&kb));
            }
        }

        Ok(knowledge_bases)
    }

    /// Get detailed information for a specific knowledge base
    pub async fn describe_knowledge_base(
        &self,
        account_id: &str,
        region: &str,
        kb_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client
            .get_knowledge_base()
            .knowledge_base_id(kb_id)
            .send()
            .await?;

        if let Some(kb_details) = response.knowledge_base {
            Ok(self.knowledge_base_details_to_json(&kb_details))
        } else {
            Err(anyhow::anyhow!("Knowledge base {} not found", kb_id))
        }
    }

    fn knowledge_base_to_json(
        &self,
        kb: &bedrockagent::types::KnowledgeBaseSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "KnowledgeBaseId".to_string(),
            serde_json::Value::String(kb.knowledge_base_id.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(kb.name.clone()),
        );

        if let Some(description) = &kb.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(kb.status.as_str().to_string()),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(kb.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn knowledge_base_details_to_json(
        &self,
        kb: &bedrockagent::types::KnowledgeBase,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "KnowledgeBaseId".to_string(),
            serde_json::Value::String(kb.knowledge_base_id.clone()),
        );
        json.insert(
            "KnowledgeBaseArn".to_string(),
            serde_json::Value::String(kb.knowledge_base_arn.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(kb.name.clone()),
        );

        if let Some(description) = &kb.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(kb.status.as_str().to_string()),
        );

        json.insert(
            "RoleArn".to_string(),
            serde_json::Value::String(kb.role_arn.clone()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(kb.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(kb.updated_at.to_string()),
        );

        if let Some(failure_reasons) = &kb.failure_reasons {
            if !failure_reasons.is_empty() {
                let reasons_json: Vec<serde_json::Value> = failure_reasons
                    .iter()
                    .map(|r| serde_json::Value::String(r.clone()))
                    .collect();
                json.insert(
                    "FailureReasons".to_string(),
                    serde_json::Value::Array(reasons_json),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// List Bedrock prompts
    pub async fn list_prompts(
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut prompts = Vec::new();
        let mut paginator = client.list_prompts().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for prompt in page.prompt_summaries {
                prompts.push(self.prompt_to_json(&prompt));
            }
        }

        Ok(prompts)
    }

    /// Get detailed information for a specific prompt
    pub async fn describe_prompt(
        &self,
        account_id: &str,
        region: &str,
        prompt_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client.get_prompt().prompt_identifier(prompt_id).send().await?;

        Ok(self.prompt_details_to_json(
            &response.name,
            &response.id,
            &response.arn,
            &response.created_at,
            &response.updated_at,
        ))
    }

    fn prompt_to_json(&self, prompt: &bedrockagent::types::PromptSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "PromptId".to_string(),
            serde_json::Value::String(prompt.id.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(prompt.name.clone()),
        );

        if let Some(description) = &prompt.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(prompt.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(prompt.updated_at.to_string()),
        );

        json.insert(
            "Version".to_string(),
            serde_json::Value::String(prompt.version.clone()),
        );

        serde_json::Value::Object(json)
    }

    fn prompt_details_to_json(
        &self,
        name: &str,
        id: &str,
        arn: &str,
        created_at: &aws_smithy_types::DateTime,
        updated_at: &aws_smithy_types::DateTime,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "PromptId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        json.insert(
            "PromptArn".to_string(),
            serde_json::Value::String(arn.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(name.to_string()),
        );
        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List Bedrock flows
    pub async fn list_flows(
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut flows = Vec::new();
        let mut paginator = client.list_flows().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for flow in page.flow_summaries {
                flows.push(self.flow_to_json(&flow));
            }
        }

        Ok(flows)
    }

    /// Get detailed information for a specific flow
    pub async fn describe_flow(
        &self,
        account_id: &str,
        region: &str,
        flow_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client.get_flow().flow_identifier(flow_id).send().await?;

        Ok(self.flow_details_to_json(
            &response.name,
            &response.id,
            &response.arn,
            &response.status,
            &response.created_at,
            &response.updated_at,
        ))
    }

    fn flow_to_json(&self, flow: &bedrockagent::types::FlowSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "FlowId".to_string(),
            serde_json::Value::String(flow.id.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(flow.name.clone()),
        );

        if let Some(description) = &flow.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(flow.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(flow.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(flow.updated_at.to_string()),
        );

        json.insert(
            "Version".to_string(),
            serde_json::Value::String(flow.version.clone()),
        );

        serde_json::Value::Object(json)
    }

    fn flow_details_to_json(
        &self,
        name: &str,
        id: &str,
        arn: &str,
        status: &bedrockagent::types::FlowStatus,
        created_at: &aws_smithy_types::DateTime,
        updated_at: &aws_smithy_types::DateTime,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "FlowId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        json.insert(
            "FlowArn".to_string(),
            serde_json::Value::String(arn.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(name.to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(status.as_str().to_string()),
        );
        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List Agent Aliases
    pub async fn list_agent_aliases(
        &self,
        account_id: &str,
        region: &str,
        agent_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut aliases = Vec::new();
        let mut paginator = client
            .list_agent_aliases()
            .agent_id(agent_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for alias in page.agent_alias_summaries {
                aliases.push(self.agent_alias_to_json(&alias));
            }
        }

        Ok(aliases)
    }

    /// Get detailed information for a specific agent alias
    pub async fn describe_agent_alias(
        &self,
        account_id: &str,
        region: &str,
        agent_id: &str,
        alias_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client
            .get_agent_alias()
            .agent_id(agent_id)
            .agent_alias_id(alias_id)
            .send()
            .await?;

        if let Some(alias_details) = response.agent_alias {
            Ok(self.agent_alias_details_to_json(&alias_details))
        } else {
            Err(anyhow::anyhow!("Agent alias {} not found", alias_id))
        }
    }

    fn agent_alias_to_json(
        &self,
        alias: &bedrockagent::types::AgentAliasSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentAliasId".to_string(),
            serde_json::Value::String(alias.agent_alias_id.clone()),
        );
        json.insert(
            "AgentAliasName".to_string(),
            serde_json::Value::String(alias.agent_alias_name.clone()),
        );

        if let Some(description) = &alias.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "AgentAliasStatus".to_string(),
            serde_json::Value::String(alias.agent_alias_status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(alias.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(alias.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn agent_alias_details_to_json(
        &self,
        alias: &bedrockagent::types::AgentAlias,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AgentAliasId".to_string(),
            serde_json::Value::String(alias.agent_alias_id.clone()),
        );
        json.insert(
            "AgentAliasArn".to_string(),
            serde_json::Value::String(alias.agent_alias_arn.clone()),
        );
        json.insert(
            "AgentAliasName".to_string(),
            serde_json::Value::String(alias.agent_alias_name.clone()),
        );
        json.insert(
            "AgentId".to_string(),
            serde_json::Value::String(alias.agent_id.clone()),
        );

        if let Some(description) = &alias.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "AgentAliasStatus".to_string(),
            serde_json::Value::String(alias.agent_alias_status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(alias.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(alias.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List Agent Action Groups
    pub async fn list_agent_action_groups(
        &self,
        account_id: &str,
        region: &str,
        agent_id: &str,
        agent_version: &str,
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut action_groups = Vec::new();
        let mut paginator = client
            .list_agent_action_groups()
            .agent_id(agent_id)
            .agent_version(agent_version)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for action_group in page.action_group_summaries {
                action_groups.push(self.action_group_to_json(&action_group));
            }
        }

        Ok(action_groups)
    }

    /// Get detailed information for a specific action group
    pub async fn describe_agent_action_group(
        &self,
        account_id: &str,
        region: &str,
        agent_id: &str,
        agent_version: &str,
        action_group_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client
            .get_agent_action_group()
            .agent_id(agent_id)
            .agent_version(agent_version)
            .action_group_id(action_group_id)
            .send()
            .await?;

        if let Some(action_group_details) = response.agent_action_group {
            Ok(self.action_group_details_to_json(&action_group_details))
        } else {
            Err(anyhow::anyhow!(
                "Action group {} not found",
                action_group_id
            ))
        }
    }

    fn action_group_to_json(
        &self,
        action_group: &bedrockagent::types::ActionGroupSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ActionGroupId".to_string(),
            serde_json::Value::String(action_group.action_group_id.clone()),
        );
        json.insert(
            "ActionGroupName".to_string(),
            serde_json::Value::String(action_group.action_group_name.clone()),
        );

        json.insert(
            "ActionGroupState".to_string(),
            serde_json::Value::String(action_group.action_group_state.as_str().to_string()),
        );

        if let Some(description) = &action_group.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(action_group.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn action_group_details_to_json(
        &self,
        action_group: &bedrockagent::types::AgentActionGroup,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ActionGroupId".to_string(),
            serde_json::Value::String(action_group.action_group_id.clone()),
        );
        json.insert(
            "ActionGroupName".to_string(),
            serde_json::Value::String(action_group.action_group_name.clone()),
        );
        json.insert(
            "AgentId".to_string(),
            serde_json::Value::String(action_group.agent_id.clone()),
        );
        json.insert(
            "AgentVersion".to_string(),
            serde_json::Value::String(action_group.agent_version.clone()),
        );

        json.insert(
            "ActionGroupState".to_string(),
            serde_json::Value::String(action_group.action_group_state.as_str().to_string()),
        );

        if let Some(description) = &action_group.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(action_group.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(action_group.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List Data Sources for a Knowledge Base
    pub async fn list_data_sources(
        &self,
        account_id: &str,
        region: &str,
        knowledge_base_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut data_sources = Vec::new();
        let mut paginator = client
            .list_data_sources()
            .knowledge_base_id(knowledge_base_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for data_source in page.data_source_summaries {
                data_sources.push(self.data_source_to_json(&data_source));
            }
        }

        Ok(data_sources)
    }

    /// Get detailed information for a specific data source
    pub async fn describe_data_source(
        &self,
        account_id: &str,
        region: &str,
        knowledge_base_id: &str,
        data_source_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client
            .get_data_source()
            .knowledge_base_id(knowledge_base_id)
            .data_source_id(data_source_id)
            .send()
            .await?;

        if let Some(data_source_details) = response.data_source {
            Ok(self.data_source_details_to_json(&data_source_details))
        } else {
            Err(anyhow::anyhow!("Data source {} not found", data_source_id))
        }
    }

    fn data_source_to_json(
        &self,
        data_source: &bedrockagent::types::DataSourceSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DataSourceId".to_string(),
            serde_json::Value::String(data_source.data_source_id.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(data_source.name.clone()),
        );

        if let Some(description) = &data_source.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(data_source.status.as_str().to_string()),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(data_source.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn data_source_details_to_json(
        &self,
        data_source: &bedrockagent::types::DataSource,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DataSourceId".to_string(),
            serde_json::Value::String(data_source.data_source_id.clone()),
        );
        json.insert(
            "KnowledgeBaseId".to_string(),
            serde_json::Value::String(data_source.knowledge_base_id.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(data_source.name.clone()),
        );

        if let Some(description) = &data_source.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(data_source.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(data_source.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(data_source.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List Ingestion Jobs for a Data Source
    pub async fn list_ingestion_jobs(
        &self,
        account_id: &str,
        region: &str,
        knowledge_base_id: &str,
        data_source_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut ingestion_jobs = Vec::new();
        let mut paginator = client
            .list_ingestion_jobs()
            .knowledge_base_id(knowledge_base_id)
            .data_source_id(data_source_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for job in page.ingestion_job_summaries {
                ingestion_jobs.push(self.ingestion_job_to_json(&job));
            }
        }

        Ok(ingestion_jobs)
    }

    /// Get detailed information for a specific ingestion job
    pub async fn describe_ingestion_job(
        &self,
        account_id: &str,
        region: &str,
        knowledge_base_id: &str,
        data_source_id: &str,
        ingestion_job_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client
            .get_ingestion_job()
            .knowledge_base_id(knowledge_base_id)
            .data_source_id(data_source_id)
            .ingestion_job_id(ingestion_job_id)
            .send()
            .await?;

        if let Some(ingestion_job_details) = response.ingestion_job {
            Ok(self.ingestion_job_details_to_json(&ingestion_job_details))
        } else {
            Err(anyhow::anyhow!(
                "Ingestion job {} not found",
                ingestion_job_id
            ))
        }
    }

    fn ingestion_job_to_json(
        &self,
        job: &bedrockagent::types::IngestionJobSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "IngestionJobId".to_string(),
            serde_json::Value::String(job.ingestion_job_id.clone()),
        );
        json.insert(
            "DataSourceId".to_string(),
            serde_json::Value::String(job.data_source_id.clone()),
        );
        json.insert(
            "KnowledgeBaseId".to_string(),
            serde_json::Value::String(job.knowledge_base_id.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(job.status.as_str().to_string()),
        );

        json.insert(
            "StartedAt".to_string(),
            serde_json::Value::String(job.started_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(job.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn ingestion_job_details_to_json(
        &self,
        job: &bedrockagent::types::IngestionJob,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "IngestionJobId".to_string(),
            serde_json::Value::String(job.ingestion_job_id.clone()),
        );
        json.insert(
            "DataSourceId".to_string(),
            serde_json::Value::String(job.data_source_id.clone()),
        );
        json.insert(
            "KnowledgeBaseId".to_string(),
            serde_json::Value::String(job.knowledge_base_id.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(job.status.as_str().to_string()),
        );

        json.insert(
            "StartedAt".to_string(),
            serde_json::Value::String(job.started_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(job.updated_at.to_string()),
        );

        if let Some(failure_reasons) = &job.failure_reasons {
            if !failure_reasons.is_empty() {
                let reasons_json: Vec<serde_json::Value> = failure_reasons
                    .iter()
                    .map(|r| serde_json::Value::String(r.clone()))
                    .collect();
                json.insert(
                    "FailureReasons".to_string(),
                    serde_json::Value::Array(reasons_json),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// List Flow Aliases
    pub async fn list_flow_aliases(
        &self,
        account_id: &str,
        region: &str,
        flow_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);

        let mut aliases = Vec::new();
        let mut paginator = client
            .list_flow_aliases()
            .flow_identifier(flow_id)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for alias in page.flow_alias_summaries {
                aliases.push(self.flow_alias_to_json(&alias));
            }
        }

        Ok(aliases)
    }

    /// Get detailed information for a specific flow alias
    pub async fn describe_flow_alias(
        &self,
        account_id: &str,
        region: &str,
        flow_id: &str,
        alias_id: &str,
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

        let client = bedrockagent::Client::new(&aws_config);
        let response = client
            .get_flow_alias()
            .flow_identifier(flow_id)
            .alias_identifier(alias_id)
            .send()
            .await?;

        Ok(self.flow_alias_details_to_json(
            &response.name,
            &response.id,
            &response.arn,
            &response.flow_id,
            &response.created_at,
            &response.updated_at,
        ))
    }

    fn flow_alias_to_json(
        &self,
        alias: &bedrockagent::types::FlowAliasSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Id".to_string(),
            serde_json::Value::String(alias.id.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(alias.name.clone()),
        );
        json.insert(
            "FlowId".to_string(),
            serde_json::Value::String(alias.flow_id.clone()),
        );

        if let Some(description) = &alias.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(alias.created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(alias.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn flow_alias_details_to_json(
        &self,
        name: &str,
        id: &str,
        arn: &str,
        flow_id: &str,
        created_at: &aws_smithy_types::DateTime,
        updated_at: &aws_smithy_types::DateTime,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Id".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(arn.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(name.to_string()),
        );
        json.insert(
            "FlowId".to_string(),
            serde_json::Value::String(flow_id.to_string()),
        );
        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(created_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }
}
