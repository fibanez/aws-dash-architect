use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_wafv2 as wafv2;
use std::sync::Arc;

pub struct WafV2Service {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl WafV2Service {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List WAFv2 Web ACLs
    pub async fn list_web_acls(
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

        let client = wafv2::Client::new(&aws_config);

        let mut web_acls = Vec::new();

        // List CloudFront (GLOBAL) Web ACLs
        if region == "us-east-1" {
            // CloudFront Web ACLs are only available in us-east-1
            let response = client
                .list_web_acls()
                .scope(wafv2::types::Scope::Cloudfront)
                .send()
                .await?;

            if let Some(web_acl_list) = response.web_acls {
                for web_acl in web_acl_list {
                    if let Ok(web_acl_details) = self
                        .get_web_acl_internal(&client, &web_acl, wafv2::types::Scope::Cloudfront)
                        .await
                    {
                        web_acls.push(web_acl_details);
                    } else {
                        // Fallback to basic web ACL info if get fails
                        let web_acl_json = self.web_acl_summary_to_json(&web_acl, "CLOUDFRONT");
                        web_acls.push(web_acl_json);
                    }
                }
            }
        }

        // List Regional Web ACLs
        let response = client
            .list_web_acls()
            .scope(wafv2::types::Scope::Regional)
            .send()
            .await?;

        if let Some(web_acl_list) = response.web_acls {
            for web_acl in web_acl_list {
                if let Ok(web_acl_details) = self
                    .get_web_acl_internal(&client, &web_acl, wafv2::types::Scope::Regional)
                    .await
                {
                    web_acls.push(web_acl_details);
                } else {
                    // Fallback to basic web ACL info if get fails
                    let web_acl_json = self.web_acl_summary_to_json(&web_acl, "REGIONAL");
                    web_acls.push(web_acl_json);
                }
            }
        }

        Ok(web_acls)
    }

    /// Get detailed information for specific WAFv2 Web ACL
    pub async fn get_web_acl(
        &self,
        account_id: &str,
        region: &str,
        web_acl_id: &str,
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

        let client = wafv2::Client::new(&aws_config);

        // Try both CloudFront and Regional scopes since we don't know which one from the ID
        if let Ok(result) = self
            .get_web_acl_by_id_internal(&client, web_acl_id, wafv2::types::Scope::Regional)
            .await
        {
            return Ok(result);
        }

        if region == "us-east-1" {
            if let Ok(result) = self
                .get_web_acl_by_id_internal(&client, web_acl_id, wafv2::types::Scope::Cloudfront)
                .await
            {
                return Ok(result);
            }
        }

        Err(anyhow::anyhow!(
            "Web ACL {} not found in any scope",
            web_acl_id
        ))
    }

    async fn get_web_acl_internal(
        &self,
        client: &wafv2::Client,
        web_acl_summary: &wafv2::types::WebAclSummary,
        scope: wafv2::types::Scope,
    ) -> Result<serde_json::Value> {
        if let (Some(name), Some(id)) = (&web_acl_summary.name, &web_acl_summary.id) {
            let response = client
                .get_web_acl()
                .scope(scope.clone())
                .id(id)
                .name(name)
                .send()
                .await?;

            if let Some(web_acl) = response.web_acl {
                Ok(self.web_acl_detail_to_json(&web_acl, &scope))
            } else {
                Ok(self.web_acl_summary_to_json(web_acl_summary, &format!("{:?}", scope)))
            }
        } else {
            Ok(self.web_acl_summary_to_json(web_acl_summary, &format!("{:?}", scope)))
        }
    }

    async fn get_web_acl_by_id_internal(
        &self,
        _client: &wafv2::Client,
        _web_acl_id: &str,
        _scope: wafv2::types::Scope,
    ) -> Result<serde_json::Value> {
        // We need both ID and Name for the get_web_acl call, but we only have ID
        // This is a limitation of the WAF API - we would need to list all Web ACLs
        // and find the matching one by ID to get the name
        Err(anyhow::anyhow!(
            "Cannot get Web ACL by ID only - need both ID and name"
        ))
    }

    fn web_acl_summary_to_json(
        &self,
        web_acl: &wafv2::types::WebAclSummary,
        scope: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &web_acl.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(id) = &web_acl.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(description) = &web_acl.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(lock_token) = &web_acl.lock_token {
            json.insert(
                "LockToken".to_string(),
                serde_json::Value::String(lock_token.clone()),
            );
        }

        if let Some(arn) = &web_acl.arn {
            json.insert("ARN".to_string(), serde_json::Value::String(arn.clone()));
        }

        json.insert(
            "Scope".to_string(),
            serde_json::Value::String(scope.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn web_acl_detail_to_json(
        &self,
        web_acl: &wafv2::types::WebAcl,
        scope: &wafv2::types::Scope,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(web_acl.name.clone()),
        );
        json.insert(
            "Id".to_string(),
            serde_json::Value::String(web_acl.id.clone()),
        );
        json.insert(
            "ARN".to_string(),
            serde_json::Value::String(web_acl.arn.clone()),
        );
        json.insert(
            "Scope".to_string(),
            serde_json::Value::String(format!("{:?}", scope)),
        );

        if let Some(description) = &web_acl.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(default_action) = &web_acl.default_action {
            let mut default_action_json = serde_json::Map::new();
            if default_action.block.is_some() {
                default_action_json.insert(
                    "Type".to_string(),
                    serde_json::Value::String("BLOCK".to_string()),
                );
            } else if default_action.allow.is_some() {
                default_action_json.insert(
                    "Type".to_string(),
                    serde_json::Value::String("ALLOW".to_string()),
                );
            }
            json.insert(
                "DefaultAction".to_string(),
                serde_json::Value::Object(default_action_json),
            );
        }

        if let Some(rules) = &web_acl.rules {
            let rules_array: Vec<serde_json::Value> = rules
                .iter()
                .map(|rule| {
                    let mut rule_json = serde_json::Map::new();
                    rule_json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(rule.name.clone()),
                    );
                    rule_json.insert(
                        "Priority".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(rule.priority)),
                    );

                    if let Some(action) = &rule.action {
                        let mut action_json = serde_json::Map::new();
                        if action.block.is_some() {
                            action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("BLOCK".to_string()),
                            );
                        } else if action.allow.is_some() {
                            action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("ALLOW".to_string()),
                            );
                        } else if action.count.is_some() {
                            action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("COUNT".to_string()),
                            );
                        } else if action.captcha.is_some() {
                            action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("CAPTCHA".to_string()),
                            );
                        } else if action.challenge.is_some() {
                            action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("CHALLENGE".to_string()),
                            );
                        }
                        rule_json
                            .insert("Action".to_string(), serde_json::Value::Object(action_json));
                    }

                    if let Some(override_action) = &rule.override_action {
                        let mut override_action_json = serde_json::Map::new();
                        if override_action.count.is_some() {
                            override_action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("COUNT".to_string()),
                            );
                        } else if override_action.none.is_some() {
                            override_action_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String("NONE".to_string()),
                            );
                        }
                        rule_json.insert(
                            "OverrideAction".to_string(),
                            serde_json::Value::Object(override_action_json),
                        );
                    }

                    if let Some(visibility_config) = &rule.visibility_config {
                        let mut visibility_json = serde_json::Map::new();
                        visibility_json.insert(
                            "SampledRequestsEnabled".to_string(),
                            serde_json::Value::Bool(visibility_config.sampled_requests_enabled),
                        );
                        visibility_json.insert(
                            "CloudWatchMetricsEnabled".to_string(),
                            serde_json::Value::Bool(visibility_config.cloud_watch_metrics_enabled),
                        );
                        visibility_json.insert(
                            "MetricName".to_string(),
                            serde_json::Value::String(visibility_config.metric_name.clone()),
                        );
                        rule_json.insert(
                            "VisibilityConfig".to_string(),
                            serde_json::Value::Object(visibility_json),
                        );
                    }

                    serde_json::Value::Object(rule_json)
                })
                .collect();
            json.insert("Rules".to_string(), serde_json::Value::Array(rules_array));
        }

        if let Some(visibility_config) = &web_acl.visibility_config {
            let mut visibility_json = serde_json::Map::new();
            visibility_json.insert(
                "SampledRequestsEnabled".to_string(),
                serde_json::Value::Bool(visibility_config.sampled_requests_enabled),
            );
            visibility_json.insert(
                "CloudWatchMetricsEnabled".to_string(),
                serde_json::Value::Bool(visibility_config.cloud_watch_metrics_enabled),
            );
            visibility_json.insert(
                "MetricName".to_string(),
                serde_json::Value::String(visibility_config.metric_name.clone()),
            );
            json.insert(
                "VisibilityConfig".to_string(),
                serde_json::Value::Object(visibility_json),
            );
        }

        json.insert(
            "Capacity".to_string(),
            serde_json::Value::Number(serde_json::Number::from(web_acl.capacity)),
        );

        if let Some(pre_process_firewall_manager_rule_groups) =
            &web_acl.pre_process_firewall_manager_rule_groups
        {
            let firewall_groups: Vec<serde_json::Value> = pre_process_firewall_manager_rule_groups
                .iter()
                .map(|group| {
                    let mut group_json = serde_json::Map::new();
                    group_json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(group.name.clone()),
                    );
                    group_json.insert(
                        "Priority".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(group.priority)),
                    );
                    if let Some(firewall_manager_statement) = &group.firewall_manager_statement {
                        if let Some(managed_rule_group_statement) =
                            &firewall_manager_statement.managed_rule_group_statement
                        {
                            group_json.insert(
                                "VendorName".to_string(),
                                serde_json::Value::String(
                                    managed_rule_group_statement.vendor_name.clone(),
                                ),
                            );
                            group_json.insert(
                                "GroupName".to_string(),
                                serde_json::Value::String(
                                    managed_rule_group_statement.name.clone(),
                                ),
                            );
                        }
                    }
                    serde_json::Value::Object(group_json)
                })
                .collect();
            json.insert(
                "PreProcessFirewallManagerRuleGroups".to_string(),
                serde_json::Value::Array(firewall_groups),
            );
        }

        if let Some(post_process_firewall_manager_rule_groups) =
            &web_acl.post_process_firewall_manager_rule_groups
        {
            let firewall_groups: Vec<serde_json::Value> = post_process_firewall_manager_rule_groups
                .iter()
                .map(|group| {
                    let mut group_json = serde_json::Map::new();
                    group_json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(group.name.clone()),
                    );
                    group_json.insert(
                        "Priority".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(group.priority)),
                    );
                    if let Some(firewall_manager_statement) = &group.firewall_manager_statement {
                        if let Some(managed_rule_group_statement) =
                            &firewall_manager_statement.managed_rule_group_statement
                        {
                            group_json.insert(
                                "VendorName".to_string(),
                                serde_json::Value::String(
                                    managed_rule_group_statement.vendor_name.clone(),
                                ),
                            );
                            group_json.insert(
                                "GroupName".to_string(),
                                serde_json::Value::String(
                                    managed_rule_group_statement.name.clone(),
                                ),
                            );
                        }
                    }
                    serde_json::Value::Object(group_json)
                })
                .collect();
            json.insert(
                "PostProcessFirewallManagerRuleGroups".to_string(),
                serde_json::Value::Array(firewall_groups),
            );
        }

        json.insert(
            "ManagedByFirewallManager".to_string(),
            serde_json::Value::Bool(web_acl.managed_by_firewall_manager),
        );

        if let Some(label_namespace) = &web_acl.label_namespace {
            json.insert(
                "LabelNamespace".to_string(),
                serde_json::Value::String(label_namespace.clone()),
            );
        }

        if let Some(custom_response_bodies) = &web_acl.custom_response_bodies {
            let response_bodies: Vec<serde_json::Value> = custom_response_bodies
                .iter()
                .map(|(key, body)| {
                    let mut body_json = serde_json::Map::new();
                    body_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    body_json.insert(
                        "ContentType".to_string(),
                        serde_json::Value::String(format!("{:?}", body.content_type)),
                    );
                    body_json.insert(
                        "Content".to_string(),
                        serde_json::Value::String(body.content.clone()),
                    );
                    serde_json::Value::Object(body_json)
                })
                .collect();
            json.insert(
                "CustomResponseBodies".to_string(),
                serde_json::Value::Array(response_bodies),
            );
        }

        serde_json::Value::Object(json)
    }
}
