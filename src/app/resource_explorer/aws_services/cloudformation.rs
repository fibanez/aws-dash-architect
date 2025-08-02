use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudformation as cfn;
use std::sync::Arc;

pub struct CloudFormationService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudFormationService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CloudFormation stacks
    pub async fn list_stacks(
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

        let client = cfn::Client::new(&aws_config);
        let mut stacks = Vec::new();

        // Use describe_stacks for comprehensive stack data
        let mut paginator = client.describe_stacks().into_paginator().send();

        while let Some(result) = paginator.try_next().await? {
            let stack_list = result.stacks.unwrap_or_default();
            for stack in stack_list {
                let stack_json = self.stack_to_json(&stack);
                stacks.push(stack_json);
            }
        }

        Ok(stacks)
    }

    /// Get detailed stack information
    pub async fn describe_stack(
        &self,
        account_id: &str,
        region: &str,
        stack_name: &str,
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

        let client = cfn::Client::new(&aws_config);

        // Get stack details
        let response = client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await?;

        if let Some(stacks) = response.stacks {
            if let Some(stack) = stacks.into_iter().next() {
                // Get stack resources
                let resources_response = client
                    .list_stack_resources()
                    .stack_name(stack_name)
                    .send()
                    .await;

                // Get stack events (last 10)
                let events_response = client
                    .describe_stack_events()
                    .stack_name(stack_name)
                    .send()
                    .await;

                // Get stack template
                let template_response = client.get_template().stack_name(stack_name).send().await;

                let mut stack_details = self.stack_to_json(&stack);

                // Add resources
                if let Ok(resources) = resources_response {
                    if let Some(resource_summaries) = resources.stack_resource_summaries {
                        // Manual conversion for AWS SDK type
                        let resources_json: Vec<serde_json::Value> = resource_summaries
                            .iter()
                            .map(|resource| {
                                let mut resource_json = serde_json::Map::new();
                                if let Some(logical_id) = &resource.logical_resource_id {
                                    resource_json.insert(
                                        "LogicalResourceId".to_string(),
                                        serde_json::Value::String(logical_id.clone()),
                                    );
                                }
                                if let Some(resource_type) = &resource.resource_type {
                                    resource_json.insert(
                                        "ResourceType".to_string(),
                                        serde_json::Value::String(resource_type.clone()),
                                    );
                                }
                                if let Some(status) = &resource.resource_status {
                                    resource_json.insert(
                                        "ResourceStatus".to_string(),
                                        serde_json::Value::String(status.as_str().to_string()),
                                    );
                                }
                                serde_json::Value::Object(resource_json)
                            })
                            .collect();
                        stack_details.as_object_mut().unwrap().insert(
                            "Resources".to_string(),
                            serde_json::Value::Array(resources_json),
                        );
                    }
                }

                // Add recent events
                if let Ok(events) = events_response {
                    if let Some(stack_events) = events.stack_events {
                        // Manual conversion for AWS SDK type
                        let events_json: Vec<serde_json::Value> = stack_events
                            .into_iter()
                            .take(10)
                            .map(|event| {
                                let mut event_json = serde_json::Map::new();
                                if let Some(event_id) = &event.event_id {
                                    event_json.insert(
                                        "EventId".to_string(),
                                        serde_json::Value::String(event_id.clone()),
                                    );
                                }
                                if let Some(logical_id) = &event.logical_resource_id {
                                    event_json.insert(
                                        "LogicalResourceId".to_string(),
                                        serde_json::Value::String(logical_id.clone()),
                                    );
                                }
                                if let Some(status) = &event.resource_status {
                                    event_json.insert(
                                        "ResourceStatus".to_string(),
                                        serde_json::Value::String(status.as_str().to_string()),
                                    );
                                }
                                serde_json::Value::Object(event_json)
                            })
                            .collect();
                        stack_details.as_object_mut().unwrap().insert(
                            "RecentEvents".to_string(),
                            serde_json::Value::Array(events_json),
                        );
                    }
                }

                // Add template body
                if let Ok(template) = template_response {
                    if let Some(template_body) = template.template_body {
                        stack_details.as_object_mut().unwrap().insert(
                            "TemplateBody".to_string(),
                            serde_json::Value::String(template_body),
                        );
                    }
                }

                return Ok(stack_details);
            }
        }

        Err(anyhow::anyhow!("Stack not found: {}", stack_name))
    }

    /// Convert stack to JSON format
    fn stack_to_json(&self, stack: &cfn::types::Stack) -> serde_json::Value {
        let mut stack_map = serde_json::Map::new();

        if let Some(stack_id) = &stack.stack_id {
            stack_map.insert(
                "StackId".to_string(),
                serde_json::Value::String(stack_id.clone()),
            );
        }

        if let Some(stack_name) = &stack.stack_name {
            stack_map.insert(
                "StackName".to_string(),
                serde_json::Value::String(stack_name.clone()),
            );
            stack_map.insert(
                "Name".to_string(),
                serde_json::Value::String(stack_name.clone()),
            );
        }

        if let Some(description) = &stack.description {
            stack_map.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(creation_time) = stack.creation_time {
            stack_map.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(last_updated_time) = stack.last_updated_time {
            stack_map.insert(
                "LastUpdatedTime".to_string(),
                serde_json::Value::String(last_updated_time.to_string()),
            );
        }

        if let Some(stack_status) = &stack.stack_status {
            stack_map.insert(
                "StackStatus".to_string(),
                serde_json::Value::String(stack_status.as_str().to_string()),
            );
            stack_map.insert(
                "Status".to_string(),
                serde_json::Value::String(stack_status.as_str().to_string()),
            );
        }

        if let Some(stack_status_reason) = &stack.stack_status_reason {
            stack_map.insert(
                "StackStatusReason".to_string(),
                serde_json::Value::String(stack_status_reason.clone()),
            );
        }

        if let Some(disable_rollback) = stack.disable_rollback {
            stack_map.insert(
                "DisableRollback".to_string(),
                serde_json::Value::Bool(disable_rollback),
            );
        }

        if let Some(timeout_in_minutes) = stack.timeout_in_minutes {
            stack_map.insert(
                "TimeoutInMinutes".to_string(),
                serde_json::Value::Number(timeout_in_minutes.into()),
            );
        }

        if let Some(capabilities) = &stack.capabilities {
            // Manual conversion for AWS SDK enum type
            let capabilities_json: Vec<serde_json::Value> = capabilities
                .iter()
                .map(|cap| serde_json::Value::String(cap.as_str().to_string()))
                .collect();
            stack_map.insert(
                "Capabilities".to_string(),
                serde_json::Value::Array(capabilities_json),
            );
        }

        if let Some(outputs) = &stack.outputs {
            // Manual conversion for AWS SDK type
            let outputs_json: Vec<serde_json::Value> = outputs
                .iter()
                .map(|output| {
                    let mut output_json = serde_json::Map::new();
                    if let Some(key) = &output.output_key {
                        output_json.insert(
                            "OutputKey".to_string(),
                            serde_json::Value::String(key.clone()),
                        );
                    }
                    if let Some(value) = &output.output_value {
                        output_json.insert(
                            "OutputValue".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    if let Some(description) = &output.description {
                        output_json.insert(
                            "Description".to_string(),
                            serde_json::Value::String(description.clone()),
                        );
                    }
                    serde_json::Value::Object(output_json)
                })
                .collect();
            stack_map.insert(
                "Outputs".to_string(),
                serde_json::Value::Array(outputs_json),
            );
        }

        if let Some(parameters) = &stack.parameters {
            // Manual conversion for AWS SDK type
            let parameters_json: Vec<serde_json::Value> = parameters
                .iter()
                .map(|param| {
                    let mut param_json = serde_json::Map::new();
                    if let Some(key) = &param.parameter_key {
                        param_json.insert(
                            "ParameterKey".to_string(),
                            serde_json::Value::String(key.clone()),
                        );
                    }
                    if let Some(value) = &param.parameter_value {
                        param_json.insert(
                            "ParameterValue".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(param_json)
                })
                .collect();
            stack_map.insert(
                "Parameters".to_string(),
                serde_json::Value::Array(parameters_json),
            );
        }

        if let Some(tags) = &stack.tags {
            // Manual conversion for AWS SDK type
            let tags_json: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            stack_map.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
        }

        if let Some(role_arn) = &stack.role_arn {
            stack_map.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(drift_information) = &stack.drift_information {
            // Manual conversion for AWS SDK type
            let mut drift_json = serde_json::Map::new();
            if let Some(status) = &drift_information.stack_drift_status {
                drift_json.insert(
                    "StackDriftStatus".to_string(),
                    serde_json::Value::String(status.as_str().to_string()),
                );
            }
            if let Some(last_check_timestamp) = drift_information.last_check_timestamp {
                drift_json.insert(
                    "LastCheckTimestamp".to_string(),
                    serde_json::Value::String(last_check_timestamp.to_string()),
                );
            }
            stack_map.insert(
                "DriftInformation".to_string(),
                serde_json::Value::Object(drift_json),
            );
        }

        serde_json::Value::Object(stack_map)
    }
}
