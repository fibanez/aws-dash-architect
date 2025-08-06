use anyhow::{Result, Context};
use aws_sdk_detective as detective;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct DetectiveService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl DetectiveService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Detective behavior graphs
    pub async fn list_graphs(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = detective::Client::new(&aws_config);
        let mut graphs = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_graphs();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(graph_list) = response.graph_list {
                for graph in graph_list {
                    let graph_json = self.graph_to_json(&graph);
                    graphs.push(graph_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(graphs)
    }

    /// Get detailed information for specific Detective graph
    pub async fn describe_graph(
        &self,
        account_id: &str,
        region: &str,
        graph_arn: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = detective::Client::new(&aws_config);
        self.describe_graph_internal(&client, graph_arn).await
    }

    async fn describe_graph_internal(
        &self,
        client: &detective::Client,
        graph_arn: &str,
    ) -> Result<serde_json::Value> {
        // Detective doesn't have a specific describe_graph API, so we'll get the graph info from list_graphs
        let response = client.list_graphs().send().await?;

        if let Some(graph_list) = response.graph_list {
            for graph in graph_list {
                if let Some(arn) = &graph.arn {
                    if arn == graph_arn {
                        return Ok(self.graph_detail_to_json(&graph));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Detective graph {} not found", graph_arn))
    }

    fn graph_to_json(&self, graph: &detective::types::Graph) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(arn) = &graph.arn {
            json.insert("GraphArn".to_string(), serde_json::Value::String(arn.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(arn.clone()));
            
            // Extract graph name from ARN
            if let Some(graph_name) = arn.split('/').next_back() {
                json.insert("GraphName".to_string(), serde_json::Value::String(graph_name.to_string()));
                json.insert("Name".to_string(), serde_json::Value::String(format!("Detective-{}", graph_name)));
            }
        }

        if let Some(created_time) = graph.created_time {
            json.insert("CreatedTime".to_string(), serde_json::Value::String(created_time.to_string()));
        }

        // Status is always ACTIVE if the graph exists
        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        // Set default name if not available
        if !json.contains_key("Name") {
            json.insert("Name".to_string(), serde_json::Value::String("Detective Graph".to_string()));
        }

        serde_json::Value::Object(json)
    }

    fn graph_detail_to_json(&self, graph: &detective::types::Graph) -> serde_json::Value {
        // For detailed view, we use the same conversion as the summary
        // since Detective API doesn't provide much more detail in a single call
        self.graph_to_json(graph)
    }

    /// List members of a Detective graph
    pub async fn list_members(
        &self,
        account_id: &str,
        region: &str,
        graph_arn: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = detective::Client::new(&aws_config);
        let mut members = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_members().graph_arn(graph_arn);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(member_details) = response.member_details {
                for member in member_details {
                    let member_json = self.member_to_json(&member);
                    members.push(member_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(members)
    }

    fn member_to_json(&self, member: &detective::types::MemberDetail) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(account_id) = &member.account_id {
            json.insert("AccountId".to_string(), serde_json::Value::String(account_id.clone()));
        }

        if let Some(email_address) = &member.email_address {
            json.insert("EmailAddress".to_string(), serde_json::Value::String(email_address.clone()));
        }

        if let Some(graph_arn) = &member.graph_arn {
            json.insert("GraphArn".to_string(), serde_json::Value::String(graph_arn.clone()));
        }

        if let Some(status) = &member.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        if let Some(disabled_reason) = &member.disabled_reason {
            json.insert("DisabledReason".to_string(), serde_json::Value::String(disabled_reason.as_str().to_string()));
        }

        if let Some(invited_time) = member.invited_time {
            json.insert("InvitedTime".to_string(), serde_json::Value::String(invited_time.to_string()));
        }

        if let Some(updated_time) = member.updated_time {
            json.insert("UpdatedTime".to_string(), serde_json::Value::String(updated_time.to_string()));
        }

        serde_json::Value::Object(json)
    }
}