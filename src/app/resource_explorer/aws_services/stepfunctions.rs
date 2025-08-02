use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_sfn as sfn;
use std::sync::Arc;

pub struct StepFunctionsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl StepFunctionsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Step Functions State Machines
    pub async fn list_state_machines(
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

        let client = sfn::Client::new(&aws_config);
        let mut paginator = client.list_state_machines().into_paginator().send();

        let mut state_machines = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            for state_machine in page.state_machines {
                // Get detailed state machine information
                if let Ok(sm_details) = self
                    .describe_state_machine_internal(&client, &state_machine.state_machine_arn)
                    .await
                {
                    state_machines.push(sm_details);
                } else {
                    // Fallback to basic state machine info if describe fails
                    let sm_json = self.state_machine_list_item_to_json(&state_machine);
                    state_machines.push(sm_json);
                }
            }
        }

        Ok(state_machines)
    }

    /// Get detailed information for specific state machine
    pub async fn describe_state_machine(
        &self,
        account_id: &str,
        region: &str,
        state_machine_arn: &str,
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

        let client = sfn::Client::new(&aws_config);
        self.describe_state_machine_internal(&client, state_machine_arn)
            .await
    }

    async fn describe_state_machine_internal(
        &self,
        client: &sfn::Client,
        state_machine_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_state_machine()
            .state_machine_arn(state_machine_arn)
            .send()
            .await?;

        Ok(self.state_machine_description_to_json(&response))
    }

    fn state_machine_list_item_to_json(
        &self,
        state_machine: &sfn::types::StateMachineListItem,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "StateMachineArn".to_string(),
            serde_json::Value::String(state_machine.state_machine_arn.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(state_machine.state_machine_arn.clone()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(state_machine.name.clone()),
        );

        json.insert(
            "Type".to_string(),
            serde_json::Value::String(state_machine.r#type.as_str().to_string()),
        );

        json.insert(
            "CreationDate".to_string(),
            serde_json::Value::String(state_machine.creation_date.to_string()),
        );

        // Add default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn state_machine_description_to_json(
        &self,
        response: &sfn::operation::describe_state_machine::DescribeStateMachineOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "StateMachineArn".to_string(),
            serde_json::Value::String(response.state_machine_arn.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(response.state_machine_arn.clone()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        if let Some(status) = &response.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        json.insert(
            "Definition".to_string(),
            serde_json::Value::String(response.definition.clone()),
        );

        json.insert(
            "RoleArn".to_string(),
            serde_json::Value::String(response.role_arn.clone()),
        );

        json.insert(
            "Type".to_string(),
            serde_json::Value::String(response.r#type.as_str().to_string()),
        );

        json.insert(
            "CreationDate".to_string(),
            serde_json::Value::String(response.creation_date.to_string()),
        );

        if let Some(logging_configuration) = &response.logging_configuration {
            let mut logging_json = serde_json::Map::new();
            if let Some(level) = &logging_configuration.level {
                logging_json.insert(
                    "Level".to_string(),
                    serde_json::Value::String(level.as_str().to_string()),
                );
            }
            logging_json.insert(
                "IncludeExecutionData".to_string(),
                serde_json::Value::Bool(logging_configuration.include_execution_data),
            );
            if let Some(destinations) = &logging_configuration.destinations {
                if !destinations.is_empty() {
                    let dest_json: Vec<serde_json::Value> = destinations
                        .iter()
                        .map(|dest| {
                            let mut dest_obj = serde_json::Map::new();
                            if let Some(cloud_watch_logs_log_group) =
                                &dest.cloud_watch_logs_log_group
                            {
                                if let Some(log_group_arn) =
                                    &cloud_watch_logs_log_group.log_group_arn
                                {
                                    dest_obj.insert(
                                        "LogGroupArn".to_string(),
                                        serde_json::Value::String(log_group_arn.clone()),
                                    );
                                }
                            }
                            serde_json::Value::Object(dest_obj)
                        })
                        .collect();
                    logging_json.insert(
                        "Destinations".to_string(),
                        serde_json::Value::Array(dest_json),
                    );
                }
            }
            json.insert(
                "LoggingConfiguration".to_string(),
                serde_json::Value::Object(logging_json),
            );
        }

        if let Some(tracing_configuration) = &response.tracing_configuration {
            let mut tracing_json = serde_json::Map::new();
            tracing_json.insert(
                "Enabled".to_string(),
                serde_json::Value::Bool(tracing_configuration.enabled),
            );
            json.insert(
                "TracingConfiguration".to_string(),
                serde_json::Value::Object(tracing_json),
            );
        }

        serde_json::Value::Object(json)
    }
}
