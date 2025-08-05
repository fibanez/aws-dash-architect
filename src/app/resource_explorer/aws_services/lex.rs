use anyhow::{Result, Context};
use aws_sdk_lexmodelsv2 as lex;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct LexService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl LexService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Lex bots (basic list data)
    pub async fn list_bots(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = lex::Client::new(&aws_config);
        
        let mut paginator = client
            .list_bots()
            .into_paginator()
            .send();

        let mut bots = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(bot_summaries) = page.bot_summaries {
                for bot_summary in bot_summaries {
                    let bot_json = self.bot_summary_to_json(&bot_summary);
                    bots.push(bot_json);
                }
            }
        }

        Ok(bots)
    }

    /// Get detailed information for specific Lex bot (for describe functionality)
    pub async fn describe_bot(
        &self,
        account_id: &str,
        region: &str,
        bot_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = lex::Client::new(&aws_config);
        self.describe_bot_internal(&client, bot_id).await
    }

    async fn describe_bot_internal(
        &self,
        client: &lex::Client,
        bot_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_bot()
            .bot_id(bot_id)
            .send()
            .await?;

        let mut bot_details = self.bot_summary_to_json_from_describe(&response);
        
        // Get bot versions for additional details
        if let Ok(versions_response) = client
            .list_bot_versions()
            .bot_id(bot_id)
            .send()
            .await 
        {
            if let Some(bot_version_summaries) = versions_response.bot_version_summaries {
                let versions: Vec<serde_json::Value> = bot_version_summaries
                    .iter()
                    .map(|version| {
                        let mut version_json = serde_json::Map::new();
                        if let Some(bot_version) = &version.bot_version {
                            version_json.insert("BotVersion".to_string(), serde_json::Value::String(bot_version.clone()));
                        }
                        if let Some(bot_status) = &version.bot_status {
                            version_json.insert("BotStatus".to_string(), serde_json::Value::String(bot_status.as_str().to_string()));
                        }
                        if let Some(creation_date_time) = version.creation_date_time {
                            version_json.insert("CreationDateTime".to_string(), serde_json::Value::String(creation_date_time.to_string()));
                        }
                        serde_json::Value::Object(version_json)
                    })
                    .collect();
                
                if let serde_json::Value::Object(ref mut map) = bot_details {
                    map.insert("BotVersions".to_string(), serde_json::Value::Array(versions));
                }
            }
        }

        Ok(bot_details)
    }

    fn bot_summary_to_json(&self, bot_summary: &lex::types::BotSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(bot_id) = &bot_summary.bot_id {
            json.insert("BotId".to_string(), serde_json::Value::String(bot_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(bot_id.clone()));
        }
        
        if let Some(bot_name) = &bot_summary.bot_name {
            json.insert("BotName".to_string(), serde_json::Value::String(bot_name.clone()));
            json.insert("Name".to_string(), serde_json::Value::String(bot_name.clone()));
        }

        if let Some(description) = &bot_summary.description {
            json.insert("Description".to_string(), serde_json::Value::String(description.clone()));
        }

        if let Some(bot_status) = &bot_summary.bot_status {
            json.insert("BotStatus".to_string(), serde_json::Value::String(bot_status.as_str().to_string()));
            json.insert("Status".to_string(), serde_json::Value::String(bot_status.as_str().to_string()));
        }

        if let Some(latest_bot_version) = &bot_summary.latest_bot_version {
            json.insert("LatestBotVersion".to_string(), serde_json::Value::String(latest_bot_version.clone()));
        }

        if let Some(last_updated_date_time) = bot_summary.last_updated_date_time {
            json.insert("LastUpdatedDateTime".to_string(), serde_json::Value::String(last_updated_date_time.to_string()));
        }

        if let Some(bot_type) = &bot_summary.bot_type {
            json.insert("BotType".to_string(), serde_json::Value::String(bot_type.as_str().to_string()));
        }

        serde_json::Value::Object(json)
    }

    fn bot_summary_to_json_from_describe(&self, response: &lex::operation::describe_bot::DescribeBotOutput) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(bot_id) = &response.bot_id {
            json.insert("BotId".to_string(), serde_json::Value::String(bot_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(bot_id.clone()));
        }
        
        if let Some(bot_name) = &response.bot_name {
            json.insert("BotName".to_string(), serde_json::Value::String(bot_name.clone()));
            json.insert("Name".to_string(), serde_json::Value::String(bot_name.clone()));
        }

        if let Some(description) = &response.description {
            json.insert("Description".to_string(), serde_json::Value::String(description.clone()));
        }

        if let Some(role_arn) = &response.role_arn {
            json.insert("RoleArn".to_string(), serde_json::Value::String(role_arn.clone()));
        }

        if let Some(data_privacy) = &response.data_privacy {
            let mut privacy_json = serde_json::Map::new();
            privacy_json.insert("ChildDirected".to_string(), serde_json::Value::Bool(data_privacy.child_directed));
            json.insert("DataPrivacy".to_string(), serde_json::Value::Object(privacy_json));
        }

        if let Some(idle_session_ttl_in_seconds) = response.idle_session_ttl_in_seconds {
            json.insert("IdleSessionTTLInSeconds".to_string(), serde_json::Value::Number(serde_json::Number::from(idle_session_ttl_in_seconds)));
        }

        if let Some(bot_status) = &response.bot_status {
            json.insert("BotStatus".to_string(), serde_json::Value::String(bot_status.as_str().to_string()));
            json.insert("Status".to_string(), serde_json::Value::String(bot_status.as_str().to_string()));
        }

        if let Some(creation_date_time) = response.creation_date_time {
            json.insert("CreationDateTime".to_string(), serde_json::Value::String(creation_date_time.to_string()));
        }

        if let Some(last_updated_date_time) = response.last_updated_date_time {
            json.insert("LastUpdatedDateTime".to_string(), serde_json::Value::String(last_updated_date_time.to_string()));
        }

        if let Some(bot_type) = &response.bot_type {
            json.insert("BotType".to_string(), serde_json::Value::String(bot_type.as_str().to_string()));
        }

        if let Some(bot_members) = &response.bot_members {
            if !bot_members.is_empty() {
                let members_json: Vec<serde_json::Value> = bot_members
                    .iter()
                    .map(|member| {
                        let mut member_json = serde_json::Map::new();
                        member_json.insert("BotMemberId".to_string(), serde_json::Value::String(member.bot_member_id.clone()));
                        member_json.insert("BotMemberName".to_string(), serde_json::Value::String(member.bot_member_name.clone()));
                        member_json.insert("BotMemberAliasId".to_string(), serde_json::Value::String(member.bot_member_alias_id.clone()));
                        member_json.insert("BotMemberAliasName".to_string(), serde_json::Value::String(member.bot_member_alias_name.clone()));
                        member_json.insert("BotMemberVersion".to_string(), serde_json::Value::String(member.bot_member_version.clone()));
                        serde_json::Value::Object(member_json)
                    })
                    .collect();
                json.insert("BotMembers".to_string(), serde_json::Value::Array(members_json));
            }
        }

        serde_json::Value::Object(json)
    }
}