use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_cloudformation as cfn;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct CloudFormationService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudFormationService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CloudFormation stacks with optional detailed information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic stack info quickly.
    ///   If true (Phase 2), includes resources, events, policy, and drift status.
    pub async fn list_stacks(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("CloudFormation", "list_stacks", Some(region));

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
                let mut stack_json = self.stack_to_json(&stack);

                // Only fetch additional details if requested (Phase 2)
                if include_details {
                    if let Some(stack_name) = &stack.stack_name {
                        if let serde_json::Value::Object(ref mut details) = stack_json {
                            // Get stack resources
                            report_status(
                                "CloudFormation",
                                "list_stack_resources",
                                Some(stack_name),
                            );
                            match self
                                .list_stack_resources_internal(&client, stack_name)
                                .await
                            {
                                Ok(resources) => {
                                    details.insert("Resources".to_string(), resources);
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Could not get stack resources for {}: {}",
                                        stack_name,
                                        e
                                    );
                                }
                            }

                            // Get stack events (recent)
                            report_status(
                                "CloudFormation",
                                "describe_stack_events",
                                Some(stack_name),
                            );
                            match self
                                .list_stack_events_internal(&client, stack_name, Some(10))
                                .await
                            {
                                Ok(events) => {
                                    details.insert("RecentEvents".to_string(), events);
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Could not get stack events for {}: {}",
                                        stack_name,
                                        e
                                    );
                                }
                            }

                            // Get stack policy
                            report_status("CloudFormation", "get_stack_policy", Some(stack_name));
                            match self.get_stack_policy_internal(&client, stack_name).await {
                                Ok(policy) => {
                                    details.insert("StackPolicy".to_string(), policy);
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Could not get stack policy for {}: {}",
                                        stack_name,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }

                stacks.push(stack_json);
            }
        }

        report_status_done("CloudFormation", "list_stacks", Some(region));
        Ok(stacks)
    }

    /// Get detailed information for a single CloudFormation stack (Phase 2 enrichment)
    ///
    /// This function fetches detailed information for a single stack,
    /// including resources, events, policy, and drift status.
    /// Used for incremental detail fetching after the initial fast list.
    pub async fn get_stack_details(
        &self,
        account_id: &str,
        region: &str,
        stack_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("CloudFormation", "get_stack_details", Some(stack_name));

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
        let mut details = serde_json::Map::new();

        // Get stack resources
        report_status("CloudFormation", "list_stack_resources", Some(stack_name));
        match self
            .list_stack_resources_internal(&client, stack_name)
            .await
        {
            Ok(resources) => {
                details.insert("Resources".to_string(), resources);
            }
            Err(e) => {
                tracing::debug!("Could not get stack resources for {}: {}", stack_name, e);
            }
        }

        // Get stack events (recent 10)
        report_status("CloudFormation", "describe_stack_events", Some(stack_name));
        match self
            .list_stack_events_internal(&client, stack_name, Some(10))
            .await
        {
            Ok(events) => {
                details.insert("RecentEvents".to_string(), events);
            }
            Err(e) => {
                tracing::debug!("Could not get stack events for {}: {}", stack_name, e);
            }
        }

        // Get stack policy
        report_status("CloudFormation", "get_stack_policy", Some(stack_name));
        match self.get_stack_policy_internal(&client, stack_name).await {
            Ok(policy) => {
                details.insert("StackPolicy".to_string(), policy);
            }
            Err(e) => {
                tracing::debug!("Could not get stack policy for {}: {}", stack_name, e);
            }
        }

        report_status_done("CloudFormation", "get_stack_details", Some(stack_name));
        Ok(serde_json::Value::Object(details))
    }

    /// Get detailed stack information
    pub async fn describe_stack(
        &self,
        account_id: &str,
        region: &str,
        stack_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("CloudFormation", "describe_stack", Some(stack_name));

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
        let response = timeout(
            Duration::from_secs(10),
            client.describe_stacks().stack_name(stack_name).send(),
        )
        .await
        .with_context(|| "describe_stacks timed out")?
        .with_context(|| format!("Failed to describe stack {}", stack_name))?;

        if let Some(stacks) = response.stacks {
            if let Some(stack) = stacks.into_iter().next() {
                // Get stack resources
                report_status("CloudFormation", "list_stack_resources", Some(stack_name));
                let resources_response = self
                    .list_stack_resources_internal(&client, stack_name)
                    .await;

                // Get stack events (last 10)
                report_status("CloudFormation", "describe_stack_events", Some(stack_name));
                let events_response = self
                    .list_stack_events_internal(&client, stack_name, Some(10))
                    .await;

                // Get stack template
                report_status("CloudFormation", "get_template", Some(stack_name));
                let template_response = timeout(
                    Duration::from_secs(10),
                    client.get_template().stack_name(stack_name).send(),
                )
                .await;

                // Get stack policy
                report_status("CloudFormation", "get_stack_policy", Some(stack_name));
                let policy_response = self.get_stack_policy_internal(&client, stack_name).await;

                let mut stack_details = self.stack_to_json(&stack);

                // Add resources
                if let Ok(resources) = resources_response {
                    stack_details
                        .as_object_mut()
                        .unwrap()
                        .insert("Resources".to_string(), resources);
                }

                // Add recent events
                if let Ok(events) = events_response {
                    stack_details
                        .as_object_mut()
                        .unwrap()
                        .insert("RecentEvents".to_string(), events);
                }

                // Add template body
                if let Ok(Ok(template)) = template_response {
                    if let Some(template_body) = template.template_body {
                        stack_details.as_object_mut().unwrap().insert(
                            "TemplateBody".to_string(),
                            serde_json::Value::String(template_body),
                        );
                    }
                }

                // Add stack policy
                if let Ok(policy) = policy_response {
                    stack_details
                        .as_object_mut()
                        .unwrap()
                        .insert("StackPolicy".to_string(), policy);
                }

                report_status_done("CloudFormation", "describe_stack", Some(stack_name));
                return Ok(stack_details);
            }
        }

        report_status_done("CloudFormation", "describe_stack", Some(stack_name));
        Err(anyhow::anyhow!("Stack not found: {}", stack_name))
    }

    /// List stack events with pagination
    pub async fn list_stack_events(
        &self,
        account_id: &str,
        region: &str,
        stack_name: &str,
        limit: Option<usize>,
    ) -> Result<serde_json::Value> {
        report_status("CloudFormation", "list_stack_events", Some(stack_name));

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
        let result = self
            .list_stack_events_internal(&client, stack_name, limit)
            .await;

        report_status_done("CloudFormation", "list_stack_events", Some(stack_name));
        result
    }

    async fn list_stack_events_internal(
        &self,
        client: &cfn::Client,
        stack_name: &str,
        limit: Option<usize>,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.describe_stack_events().stack_name(stack_name).send(),
        )
        .await
        .with_context(|| "describe_stack_events timed out")?
        .with_context(|| format!("Failed to get events for stack {}", stack_name))?;

        let mut events_json = Vec::new();

        if let Some(stack_events) = response.stack_events {
            let events_iter: Box<dyn Iterator<Item = _>> = match limit {
                Some(n) => Box::new(stack_events.into_iter().take(n)),
                None => Box::new(stack_events.into_iter()),
            };

            for event in events_iter {
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

                if let Some(physical_id) = &event.physical_resource_id {
                    event_json.insert(
                        "PhysicalResourceId".to_string(),
                        serde_json::Value::String(physical_id.clone()),
                    );
                }

                if let Some(resource_type) = &event.resource_type {
                    event_json.insert(
                        "ResourceType".to_string(),
                        serde_json::Value::String(resource_type.clone()),
                    );
                }

                if let Some(timestamp) = event.timestamp {
                    event_json.insert(
                        "Timestamp".to_string(),
                        serde_json::Value::String(timestamp.to_string()),
                    );
                }

                if let Some(status) = &event.resource_status {
                    event_json.insert(
                        "ResourceStatus".to_string(),
                        serde_json::Value::String(status.as_str().to_string()),
                    );
                }

                if let Some(status_reason) = &event.resource_status_reason {
                    event_json.insert(
                        "ResourceStatusReason".to_string(),
                        serde_json::Value::String(status_reason.clone()),
                    );
                }

                events_json.push(serde_json::Value::Object(event_json));
            }
        }

        Ok(serde_json::Value::Array(events_json))
    }

    /// List stack resources
    pub async fn list_stack_resources(
        &self,
        account_id: &str,
        region: &str,
        stack_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("CloudFormation", "list_stack_resources", Some(stack_name));

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
        let result = self
            .list_stack_resources_internal(&client, stack_name)
            .await;

        report_status_done("CloudFormation", "list_stack_resources", Some(stack_name));
        result
    }

    async fn list_stack_resources_internal(
        &self,
        client: &cfn::Client,
        stack_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_stack_resources().stack_name(stack_name).send(),
        )
        .await
        .with_context(|| "list_stack_resources timed out")?
        .with_context(|| format!("Failed to list resources for stack {}", stack_name))?;

        let mut resources_json = Vec::new();

        if let Some(resource_summaries) = response.stack_resource_summaries {
            for resource in resource_summaries {
                let mut resource_json = serde_json::Map::new();

                if let Some(logical_id) = &resource.logical_resource_id {
                    resource_json.insert(
                        "LogicalResourceId".to_string(),
                        serde_json::Value::String(logical_id.clone()),
                    );
                }

                if let Some(physical_id) = &resource.physical_resource_id {
                    resource_json.insert(
                        "PhysicalResourceId".to_string(),
                        serde_json::Value::String(physical_id.clone()),
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

                if let Some(status_reason) = &resource.resource_status_reason {
                    resource_json.insert(
                        "ResourceStatusReason".to_string(),
                        serde_json::Value::String(status_reason.clone()),
                    );
                }

                if let Some(last_updated) = resource.last_updated_timestamp {
                    resource_json.insert(
                        "LastUpdatedTimestamp".to_string(),
                        serde_json::Value::String(last_updated.to_string()),
                    );
                }

                if let Some(drift_info) = &resource.drift_information {
                    let mut drift_json = serde_json::Map::new();
                    if let Some(status) = &drift_info.stack_resource_drift_status {
                        drift_json.insert(
                            "StackResourceDriftStatus".to_string(),
                            serde_json::Value::String(status.as_str().to_string()),
                        );
                    }
                    if let Some(last_check) = drift_info.last_check_timestamp {
                        drift_json.insert(
                            "LastCheckTimestamp".to_string(),
                            serde_json::Value::String(last_check.to_string()),
                        );
                    }
                    resource_json.insert(
                        "DriftInformation".to_string(),
                        serde_json::Value::Object(drift_json),
                    );
                }

                resources_json.push(serde_json::Value::Object(resource_json));
            }
        }

        Ok(serde_json::Value::Array(resources_json))
    }

    /// Get stack policy
    pub async fn get_stack_policy(
        &self,
        account_id: &str,
        region: &str,
        stack_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("CloudFormation", "get_stack_policy", Some(stack_name));

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
        let result = self.get_stack_policy_internal(&client, stack_name).await;

        report_status_done("CloudFormation", "get_stack_policy", Some(stack_name));
        result
    }

    async fn get_stack_policy_internal(
        &self,
        client: &cfn::Client,
        stack_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.get_stack_policy().stack_name(stack_name).send(),
        )
        .await
        .with_context(|| "get_stack_policy timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();

                if let Some(policy_body) = result.stack_policy_body {
                    // Try to parse the policy as JSON
                    if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&policy_body)
                    {
                        json.insert("StackPolicyBody".to_string(), policy_json);
                    } else {
                        json.insert(
                            "StackPolicyBody".to_string(),
                            serde_json::Value::String(policy_body),
                        );
                    }
                } else {
                    json.insert("StackPolicyBody".to_string(), serde_json::Value::Null);
                    json.insert(
                        "Note".to_string(),
                        serde_json::Value::String("No stack policy configured".to_string()),
                    );
                }

                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("StackNotFoundException") {
                    Ok(serde_json::json!({
                        "StackPolicyBody": null,
                        "Note": "Stack not found"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get stack policy: {}", e))
                }
            }
        }
    }

    /// Describe stack drift detection status
    pub async fn describe_stack_drift_detection_status(
        &self,
        account_id: &str,
        region: &str,
        stack_drift_detection_id: &str,
    ) -> Result<serde_json::Value> {
        report_status(
            "CloudFormation",
            "describe_stack_drift_detection_status",
            Some(stack_drift_detection_id),
        );

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

        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_stack_drift_detection_status()
                .stack_drift_detection_id(stack_drift_detection_id)
                .send(),
        )
        .await
        .with_context(|| "describe_stack_drift_detection_status timed out")?
        .with_context(|| {
            format!(
                "Failed to get drift detection status for {}",
                stack_drift_detection_id
            )
        })?;

        let mut json = serde_json::Map::new();

        if let Some(stack_id) = response.stack_id {
            json.insert("StackId".to_string(), serde_json::Value::String(stack_id));
        }

        if let Some(stack_drift_detection_id) = response.stack_drift_detection_id {
            json.insert(
                "StackDriftDetectionId".to_string(),
                serde_json::Value::String(stack_drift_detection_id),
            );
        }

        if let Some(stack_drift_status) = response.stack_drift_status {
            json.insert(
                "StackDriftStatus".to_string(),
                serde_json::Value::String(stack_drift_status.as_str().to_string()),
            );
        }

        if let Some(detection_status) = response.detection_status {
            json.insert(
                "DetectionStatus".to_string(),
                serde_json::Value::String(detection_status.as_str().to_string()),
            );
        }

        if let Some(detection_status_reason) = response.detection_status_reason {
            json.insert(
                "DetectionStatusReason".to_string(),
                serde_json::Value::String(detection_status_reason),
            );
        }

        if let Some(drifted_stack_resource_count) = response.drifted_stack_resource_count {
            json.insert(
                "DriftedStackResourceCount".to_string(),
                serde_json::Value::Number(drifted_stack_resource_count.into()),
            );
        }

        if let Some(timestamp) = response.timestamp {
            json.insert(
                "Timestamp".to_string(),
                serde_json::Value::String(timestamp.to_string()),
            );
        }

        report_status_done(
            "CloudFormation",
            "describe_stack_drift_detection_status",
            Some(stack_drift_detection_id),
        );
        Ok(serde_json::Value::Object(json))
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
                    if let Some(export_name) = &output.export_name {
                        output_json.insert(
                            "ExportName".to_string(),
                            serde_json::Value::String(export_name.clone()),
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
                    if let Some(resolved_value) = &param.resolved_value {
                        param_json.insert(
                            "ResolvedValue".to_string(),
                            serde_json::Value::String(resolved_value.clone()),
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

        // Enable termination protection status
        if let Some(enable_termination_protection) = stack.enable_termination_protection {
            stack_map.insert(
                "EnableTerminationProtection".to_string(),
                serde_json::Value::Bool(enable_termination_protection),
            );
        }

        // Parent stack info
        if let Some(parent_id) = &stack.parent_id {
            stack_map.insert(
                "ParentId".to_string(),
                serde_json::Value::String(parent_id.clone()),
            );
        }

        if let Some(root_id) = &stack.root_id {
            stack_map.insert(
                "RootId".to_string(),
                serde_json::Value::String(root_id.clone()),
            );
        }

        serde_json::Value::Object(stack_map)
    }
}
