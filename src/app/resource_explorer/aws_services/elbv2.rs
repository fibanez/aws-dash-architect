use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_elasticloadbalancingv2 as elbv2;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct ELBv2Service {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ELBv2Service {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Application/Network Load Balancers (ELBv2)
    pub async fn list_load_balancers(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("ELBv2", "list_load_balancers", Some(region));

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

        let client = elbv2::Client::new(&aws_config);
        let mut paginator = client.describe_load_balancers().into_paginator().send();

        let mut load_balancers = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(lb_list) = page.load_balancers {
                for lb in lb_list {
                    let mut lb_json = self.load_balancer_to_json(&lb, include_details);

                    // Fetch additional details if requested
                    if include_details {
                        if let Some(arn) = &lb.load_balancer_arn {
                            // Get load balancer attributes
                            if let Ok(attrs) = self
                                .describe_load_balancer_attributes_internal(&client, arn)
                                .await
                            {
                                if let Some(obj) = lb_json.as_object_mut() {
                                    obj.insert("Attributes".to_string(), attrs);
                                }
                            }

                            // Get listeners
                            if let Ok(listeners) =
                                self.describe_listeners_internal(&client, arn).await
                            {
                                if let Some(obj) = lb_json.as_object_mut() {
                                    obj.insert("Listeners".to_string(), listeners);
                                }
                            }

                            // Get tags
                            if let Ok(tags) = self.describe_tags_internal(&client, arn).await {
                                if let Some(obj) = lb_json.as_object_mut() {
                                    obj.insert("Tags".to_string(), tags);
                                }
                            }
                        }
                    }

                    load_balancers.push(lb_json);
                }
            }
        }

        report_status_done("ELBv2", "list_load_balancers", Some(region));
        Ok(load_balancers)
    }

    /// Get detailed information for a single load balancer (Phase 2 enrichment)
    pub async fn get_load_balancer_details(
        &self,
        account_id: &str,
        region: &str,
        load_balancer_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status(
            "ELBv2",
            "get_load_balancer_details",
            Some(load_balancer_arn),
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

        let client = elbv2::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Get load balancer attributes (access logs, deletion protection, etc.)
        report_status(
            "ELBv2",
            "describe_load_balancer_attributes",
            Some(load_balancer_arn),
        );
        if let Ok(attrs) = self
            .describe_load_balancer_attributes_internal(&client, load_balancer_arn)
            .await
        {
            details.insert("Attributes".to_string(), attrs);
        }

        // Get listeners
        report_status("ELBv2", "describe_listeners", Some(load_balancer_arn));
        if let Ok(listeners) = self
            .describe_listeners_internal(&client, load_balancer_arn)
            .await
        {
            // Also get rules for each listener
            let mut listeners_with_rules = Vec::new();
            if let Some(listeners_array) = listeners.as_array() {
                for listener in listeners_array {
                    let mut listener_obj = listener.clone();
                    if let Some(listener_arn) = listener.get("ListenerArn").and_then(|v| v.as_str())
                    {
                        if let Ok(rules) = self.describe_rules_internal(&client, listener_arn).await
                        {
                            if let Some(obj) = listener_obj.as_object_mut() {
                                obj.insert("Rules".to_string(), rules);
                            }
                        }
                    }
                    listeners_with_rules.push(listener_obj);
                }
            }
            details.insert(
                "Listeners".to_string(),
                serde_json::Value::Array(listeners_with_rules),
            );
        }

        // Get associated target groups and their health
        report_status("ELBv2", "describe_target_groups", Some(load_balancer_arn));
        if let Ok(target_groups) = self
            .describe_target_groups_for_lb_internal(&client, load_balancer_arn)
            .await
        {
            // Get health for each target group
            let mut tg_with_health = Vec::new();
            if let Some(tg_array) = target_groups.as_array() {
                for tg in tg_array {
                    let mut tg_obj = tg.clone();
                    if let Some(tg_arn) = tg.get("TargetGroupArn").and_then(|v| v.as_str()) {
                        if let Ok(health) =
                            self.describe_target_health_internal(&client, tg_arn).await
                        {
                            if let Some(obj) = tg_obj.as_object_mut() {
                                obj.insert("TargetHealth".to_string(), health);
                            }
                        }
                    }
                    tg_with_health.push(tg_obj);
                }
            }
            details.insert(
                "TargetGroups".to_string(),
                serde_json::Value::Array(tg_with_health),
            );
        }

        // Get tags
        report_status("ELBv2", "describe_tags", Some(load_balancer_arn));
        if let Ok(tags) = self
            .describe_tags_internal(&client, load_balancer_arn)
            .await
        {
            details.insert("Tags".to_string(), tags);
        }

        report_status_done(
            "ELBv2",
            "get_load_balancer_details",
            Some(load_balancer_arn),
        );
        Ok(serde_json::Value::Object(details))
    }

    /// List Target Groups
    pub async fn list_target_groups(
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

        let client = elbv2::Client::new(&aws_config);
        let response = client.describe_target_groups().send().await?;

        let mut target_groups = Vec::new();
        if let Some(tg_list) = response.target_groups {
            for tg in tg_list {
                let tg_json = self.target_group_to_json(&tg);
                target_groups.push(tg_json);
            }
        }

        Ok(target_groups)
    }

    /// Describe specific Application/Network Load Balancer
    pub async fn describe_load_balancer(
        &self,
        account_id: &str,
        region: &str,
        load_balancer_arn: &str,
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

        let client = elbv2::Client::new(&aws_config);
        let response = client
            .describe_load_balancers()
            .load_balancer_arns(load_balancer_arn)
            .send()
            .await?;

        if let Some(load_balancers) = response.load_balancers {
            if let Some(lb) = load_balancers.into_iter().next() {
                return Ok(self.load_balancer_to_json(&lb, false));
            }
        }

        Err(anyhow::anyhow!(
            "Application/Network Load Balancer not found: {}",
            load_balancer_arn
        ))
    }

    /// Describe specific Target Group
    pub async fn describe_target_group(
        &self,
        account_id: &str,
        region: &str,
        target_group_arn: &str,
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

        let client = elbv2::Client::new(&aws_config);
        let response = client
            .describe_target_groups()
            .target_group_arns(target_group_arn)
            .send()
            .await?;

        if let Some(target_groups) = response.target_groups {
            if let Some(tg) = target_groups.into_iter().next() {
                return Ok(self.target_group_to_json(&tg));
            }
        }

        Err(anyhow::anyhow!(
            "Target Group not found: {}",
            target_group_arn
        ))
    }

    /// Convert Application/Network Load Balancer to JSON
    fn load_balancer_to_json(
        &self,
        lb: &elbv2::types::LoadBalancer,
        _include_details: bool,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        // Core fields
        if let Some(arn) = &lb.load_balancer_arn {
            json.insert(
                "LoadBalancerArn".to_string(),
                serde_json::Value::String(arn.clone()),
            );
        }

        if let Some(name) = &lb.load_balancer_name {
            json.insert(
                "LoadBalancerName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(dns_name) = &lb.dns_name {
            json.insert(
                "DNSName".to_string(),
                serde_json::Value::String(dns_name.clone()),
            );
        }

        if let Some(scheme) = &lb.scheme {
            json.insert(
                "Scheme".to_string(),
                serde_json::Value::String(scheme.as_str().to_string()),
            );
        }

        if let Some(vpc_id) = &lb.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(lb_type) = &lb.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(lb_type.as_str().to_string()),
            );
        }

        if let Some(state) = &lb.state {
            let mut state_json = serde_json::Map::new();
            if let Some(code) = &state.code {
                state_json.insert(
                    "Code".to_string(),
                    serde_json::Value::String(code.as_str().to_string()),
                );
            }
            if let Some(reason) = &state.reason {
                state_json.insert(
                    "Reason".to_string(),
                    serde_json::Value::String(reason.clone()),
                );
            }
            json.insert("State".to_string(), serde_json::Value::Object(state_json));
        }

        // Availability Zones
        if let Some(availability_zones) = &lb.availability_zones {
            if !availability_zones.is_empty() {
                let azs: Vec<serde_json::Value> = availability_zones
                    .iter()
                    .map(|az| {
                        let mut az_json = serde_json::Map::new();
                        if let Some(zone_name) = &az.zone_name {
                            az_json.insert(
                                "ZoneName".to_string(),
                                serde_json::Value::String(zone_name.clone()),
                            );
                        }
                        // Note: zone_id field not available in this SDK version, skip for now
                        if let Some(subnet_id) = &az.subnet_id {
                            az_json.insert(
                                "SubnetId".to_string(),
                                serde_json::Value::String(subnet_id.clone()),
                            );
                        }
                        // Note: Load balancer addresses field might not be available in this SDK version
                        // Skip for now to fix compilation
                        serde_json::Value::Object(az_json)
                    })
                    .collect();
                json.insert(
                    "AvailabilityZones".to_string(),
                    serde_json::Value::Array(azs),
                );
            }
        }

        // Security Groups
        if let Some(security_groups) = &lb.security_groups {
            if !security_groups.is_empty() {
                let sgs: Vec<serde_json::Value> = security_groups
                    .iter()
                    .map(|sg| serde_json::Value::String(sg.clone()))
                    .collect();
                json.insert("SecurityGroups".to_string(), serde_json::Value::Array(sgs));
            }
        }

        // IP Address Type
        if let Some(ip_address_type) = &lb.ip_address_type {
            json.insert(
                "IpAddressType".to_string(),
                serde_json::Value::String(ip_address_type.as_str().to_string()),
            );
        }

        // Customer Owned Ip v4 Pool
        if let Some(pool) = &lb.customer_owned_ipv4_pool {
            json.insert(
                "CustomerOwnedIpv4Pool".to_string(),
                serde_json::Value::String(pool.clone()),
            );
        }

        // Created time
        if let Some(created_time) = &lb.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        // Canonical hosted zone
        if let Some(zone_id) = &lb.canonical_hosted_zone_id {
            json.insert(
                "CanonicalHostedZoneId".to_string(),
                serde_json::Value::String(zone_id.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    /// Convert Target Group to JSON
    fn target_group_to_json(&self, tg: &elbv2::types::TargetGroup) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        // Core fields
        if let Some(arn) = &tg.target_group_arn {
            json.insert(
                "TargetGroupArn".to_string(),
                serde_json::Value::String(arn.clone()),
            );
        }

        if let Some(name) = &tg.target_group_name {
            json.insert(
                "TargetGroupName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(protocol) = &tg.protocol {
            json.insert(
                "Protocol".to_string(),
                serde_json::Value::String(protocol.as_str().to_string()),
            );
        }

        if let Some(port) = tg.port {
            json.insert(
                "Port".to_string(),
                serde_json::Value::Number(serde_json::Number::from(port)),
            );
        }

        if let Some(vpc_id) = &tg.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(health_check_protocol) = &tg.health_check_protocol {
            json.insert(
                "HealthCheckProtocol".to_string(),
                serde_json::Value::String(health_check_protocol.as_str().to_string()),
            );
        }

        if let Some(health_check_port) = &tg.health_check_port {
            json.insert(
                "HealthCheckPort".to_string(),
                serde_json::Value::String(health_check_port.clone()),
            );
        }

        json.insert(
            "HealthCheckEnabled".to_string(),
            serde_json::Value::Bool(tg.health_check_enabled.unwrap_or(false)),
        );

        if let Some(health_check_interval_seconds) = tg.health_check_interval_seconds {
            json.insert(
                "HealthCheckIntervalSeconds".to_string(),
                serde_json::Value::Number(serde_json::Number::from(health_check_interval_seconds)),
            );
        }

        if let Some(health_check_timeout_seconds) = tg.health_check_timeout_seconds {
            json.insert(
                "HealthCheckTimeoutSeconds".to_string(),
                serde_json::Value::Number(serde_json::Number::from(health_check_timeout_seconds)),
            );
        }

        if let Some(healthy_threshold_count) = tg.healthy_threshold_count {
            json.insert(
                "HealthyThresholdCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(healthy_threshold_count)),
            );
        }

        if let Some(unhealthy_threshold_count) = tg.unhealthy_threshold_count {
            json.insert(
                "UnhealthyThresholdCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(unhealthy_threshold_count)),
            );
        }

        if let Some(health_check_path) = &tg.health_check_path {
            json.insert(
                "HealthCheckPath".to_string(),
                serde_json::Value::String(health_check_path.clone()),
            );
        }

        if let Some(matcher) = &tg.matcher {
            let mut matcher_json = serde_json::Map::new();
            if let Some(http_code) = &matcher.http_code {
                matcher_json.insert(
                    "HttpCode".to_string(),
                    serde_json::Value::String(http_code.clone()),
                );
            }
            if let Some(grpc_code) = &matcher.grpc_code {
                matcher_json.insert(
                    "GrpcCode".to_string(),
                    serde_json::Value::String(grpc_code.clone()),
                );
            }
            json.insert(
                "Matcher".to_string(),
                serde_json::Value::Object(matcher_json),
            );
        }

        // Load Balancer ARNs
        if let Some(load_balancer_arns) = &tg.load_balancer_arns {
            if !load_balancer_arns.is_empty() {
                let lb_arns: Vec<serde_json::Value> = load_balancer_arns
                    .iter()
                    .map(|arn| serde_json::Value::String(arn.clone()))
                    .collect();
                json.insert(
                    "LoadBalancerArns".to_string(),
                    serde_json::Value::Array(lb_arns),
                );
            }
        }

        if let Some(target_type) = &tg.target_type {
            json.insert(
                "TargetType".to_string(),
                serde_json::Value::String(target_type.as_str().to_string()),
            );
        }

        if let Some(protocol_version) = &tg.protocol_version {
            json.insert(
                "ProtocolVersion".to_string(),
                serde_json::Value::String(protocol_version.clone()),
            );
        }

        if let Some(ip_address_type) = &tg.ip_address_type {
            json.insert(
                "IpAddressType".to_string(),
                serde_json::Value::String(ip_address_type.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    // ============= Internal Helper Functions for Detail Fetching =============

    /// Internal: Get load balancer attributes (access logs, deletion protection, etc.)
    async fn describe_load_balancer_attributes_internal(
        &self,
        client: &elbv2::Client,
        load_balancer_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_load_balancer_attributes()
                .load_balancer_arn(load_balancer_arn)
                .send(),
        )
        .await
        .with_context(|| "describe_load_balancer_attributes timed out")?
        .with_context(|| "Failed to describe load balancer attributes")?;

        let mut attrs_json = serde_json::Map::new();
        if let Some(attrs) = response.attributes {
            for attr in attrs {
                if let (Some(key), Some(value)) = (&attr.key, &attr.value) {
                    attrs_json.insert(key.clone(), serde_json::Value::String(value.clone()));
                }
            }
        }

        Ok(serde_json::Value::Object(attrs_json))
    }

    /// Internal: Get listeners for a load balancer
    async fn describe_listeners_internal(
        &self,
        client: &elbv2::Client,
        load_balancer_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_listeners()
                .load_balancer_arn(load_balancer_arn)
                .send(),
        )
        .await
        .with_context(|| "describe_listeners timed out")?
        .with_context(|| "Failed to describe listeners")?;

        let mut listeners = Vec::new();
        if let Some(listener_list) = response.listeners {
            for listener in listener_list {
                let mut l_json = serde_json::Map::new();

                if let Some(arn) = &listener.listener_arn {
                    l_json.insert(
                        "ListenerArn".to_string(),
                        serde_json::Value::String(arn.clone()),
                    );
                }

                if let Some(port) = listener.port {
                    l_json.insert("Port".to_string(), serde_json::Value::Number(port.into()));
                }

                if let Some(protocol) = &listener.protocol {
                    l_json.insert(
                        "Protocol".to_string(),
                        serde_json::Value::String(protocol.as_str().to_string()),
                    );
                }

                if let Some(ssl_policy) = &listener.ssl_policy {
                    l_json.insert(
                        "SslPolicy".to_string(),
                        serde_json::Value::String(ssl_policy.clone()),
                    );
                }

                // Default actions
                if let Some(actions) = &listener.default_actions {
                    let actions_json: Vec<serde_json::Value> = actions
                        .iter()
                        .map(|action| self.action_to_json(action))
                        .collect();
                    l_json.insert(
                        "DefaultActions".to_string(),
                        serde_json::Value::Array(actions_json),
                    );
                }

                // Certificates
                if let Some(certs) = &listener.certificates {
                    let certs_json: Vec<serde_json::Value> = certs
                        .iter()
                        .map(|cert| {
                            let mut c = serde_json::Map::new();
                            if let Some(arn) = &cert.certificate_arn {
                                c.insert(
                                    "CertificateArn".to_string(),
                                    serde_json::Value::String(arn.clone()),
                                );
                            }
                            if let Some(is_default) = cert.is_default {
                                c.insert(
                                    "IsDefault".to_string(),
                                    serde_json::Value::Bool(is_default),
                                );
                            }
                            serde_json::Value::Object(c)
                        })
                        .collect();
                    l_json.insert(
                        "Certificates".to_string(),
                        serde_json::Value::Array(certs_json),
                    );
                }

                listeners.push(serde_json::Value::Object(l_json));
            }
        }

        Ok(serde_json::Value::Array(listeners))
    }

    /// Internal: Get rules for a listener
    async fn describe_rules_internal(
        &self,
        client: &elbv2::Client,
        listener_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.describe_rules().listener_arn(listener_arn).send(),
        )
        .await
        .with_context(|| "describe_rules timed out")?
        .with_context(|| "Failed to describe rules")?;

        let mut rules = Vec::new();
        if let Some(rule_list) = response.rules {
            for rule in rule_list {
                let mut r_json = serde_json::Map::new();

                if let Some(arn) = &rule.rule_arn {
                    r_json.insert(
                        "RuleArn".to_string(),
                        serde_json::Value::String(arn.clone()),
                    );
                }

                if let Some(priority) = &rule.priority {
                    r_json.insert(
                        "Priority".to_string(),
                        serde_json::Value::String(priority.clone()),
                    );
                }

                if let Some(is_default) = rule.is_default {
                    r_json.insert("IsDefault".to_string(), serde_json::Value::Bool(is_default));
                }

                // Conditions
                if let Some(conditions) = &rule.conditions {
                    let conds_json: Vec<serde_json::Value> = conditions
                        .iter()
                        .map(|cond| {
                            let mut c = serde_json::Map::new();
                            if let Some(field) = &cond.field {
                                c.insert(
                                    "Field".to_string(),
                                    serde_json::Value::String(field.clone()),
                                );
                            }
                            if let Some(values) = &cond.values {
                                let vals: Vec<serde_json::Value> = values
                                    .iter()
                                    .map(|v| serde_json::Value::String(v.clone()))
                                    .collect();
                                c.insert("Values".to_string(), serde_json::Value::Array(vals));
                            }
                            // Host header config
                            if let Some(host_config) = &cond.host_header_config {
                                if let Some(vals) = &host_config.values {
                                    let host_vals: Vec<serde_json::Value> = vals
                                        .iter()
                                        .map(|v| serde_json::Value::String(v.clone()))
                                        .collect();
                                    c.insert(
                                        "HostHeaderValues".to_string(),
                                        serde_json::Value::Array(host_vals),
                                    );
                                }
                            }
                            // Path pattern config
                            if let Some(path_config) = &cond.path_pattern_config {
                                if let Some(vals) = &path_config.values {
                                    let path_vals: Vec<serde_json::Value> = vals
                                        .iter()
                                        .map(|v| serde_json::Value::String(v.clone()))
                                        .collect();
                                    c.insert(
                                        "PathPatternValues".to_string(),
                                        serde_json::Value::Array(path_vals),
                                    );
                                }
                            }
                            serde_json::Value::Object(c)
                        })
                        .collect();
                    r_json.insert(
                        "Conditions".to_string(),
                        serde_json::Value::Array(conds_json),
                    );
                }

                // Actions
                if let Some(actions) = &rule.actions {
                    let actions_json: Vec<serde_json::Value> = actions
                        .iter()
                        .map(|action| self.action_to_json(action))
                        .collect();
                    r_json.insert(
                        "Actions".to_string(),
                        serde_json::Value::Array(actions_json),
                    );
                }

                rules.push(serde_json::Value::Object(r_json));
            }
        }

        Ok(serde_json::Value::Array(rules))
    }

    /// Internal: Get target groups for a load balancer
    async fn describe_target_groups_for_lb_internal(
        &self,
        client: &elbv2::Client,
        load_balancer_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_target_groups()
                .load_balancer_arn(load_balancer_arn)
                .send(),
        )
        .await
        .with_context(|| "describe_target_groups timed out")?
        .with_context(|| "Failed to describe target groups")?;

        let mut target_groups = Vec::new();
        if let Some(tg_list) = response.target_groups {
            for tg in tg_list {
                target_groups.push(self.target_group_to_json(&tg));
            }
        }

        Ok(serde_json::Value::Array(target_groups))
    }

    /// Internal: Get target health for a target group
    async fn describe_target_health_internal(
        &self,
        client: &elbv2::Client,
        target_group_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_target_health()
                .target_group_arn(target_group_arn)
                .send(),
        )
        .await
        .with_context(|| "describe_target_health timed out")?
        .with_context(|| "Failed to describe target health")?;

        let mut targets = Vec::new();
        if let Some(health_descriptions) = response.target_health_descriptions {
            for desc in health_descriptions {
                let mut t_json = serde_json::Map::new();

                // Target info
                if let Some(target) = &desc.target {
                    if let Some(id) = &target.id {
                        t_json.insert(
                            "TargetId".to_string(),
                            serde_json::Value::String(id.clone()),
                        );
                    }
                    if let Some(port) = target.port {
                        t_json.insert(
                            "TargetPort".to_string(),
                            serde_json::Value::Number(port.into()),
                        );
                    }
                    if let Some(az) = &target.availability_zone {
                        t_json.insert(
                            "AvailabilityZone".to_string(),
                            serde_json::Value::String(az.clone()),
                        );
                    }
                }

                // Health status
                if let Some(health) = &desc.target_health {
                    let mut h_json = serde_json::Map::new();
                    if let Some(state) = &health.state {
                        h_json.insert(
                            "State".to_string(),
                            serde_json::Value::String(state.as_str().to_string()),
                        );
                    }
                    if let Some(reason) = &health.reason {
                        h_json.insert(
                            "Reason".to_string(),
                            serde_json::Value::String(reason.as_str().to_string()),
                        );
                    }
                    if let Some(description) = &health.description {
                        h_json.insert(
                            "Description".to_string(),
                            serde_json::Value::String(description.clone()),
                        );
                    }
                    t_json.insert("Health".to_string(), serde_json::Value::Object(h_json));
                }

                // Health check port
                if let Some(hc_port) = &desc.health_check_port {
                    t_json.insert(
                        "HealthCheckPort".to_string(),
                        serde_json::Value::String(hc_port.clone()),
                    );
                }

                targets.push(serde_json::Value::Object(t_json));
            }
        }

        Ok(serde_json::Value::Array(targets))
    }

    /// Internal: Get tags for a resource
    async fn describe_tags_internal(
        &self,
        client: &elbv2::Client,
        resource_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.describe_tags().resource_arns(resource_arn).send(),
        )
        .await
        .with_context(|| "describe_tags timed out")?
        .with_context(|| "Failed to describe tags")?;

        let mut tags_json = serde_json::Map::new();
        if let Some(tag_descriptions) = response.tag_descriptions {
            for desc in tag_descriptions {
                if let Some(tags) = &desc.tags {
                    for tag in tags {
                        if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                            tags_json.insert(key.clone(), serde_json::Value::String(value.clone()));
                        }
                    }
                }
            }
        }

        Ok(serde_json::Value::Object(tags_json))
    }

    /// Helper: Convert action to JSON
    fn action_to_json(&self, action: &elbv2::types::Action) -> serde_json::Value {
        let mut a_json = serde_json::Map::new();

        if let Some(action_type) = &action.r#type {
            a_json.insert(
                "Type".to_string(),
                serde_json::Value::String(action_type.as_str().to_string()),
            );
        }

        if let Some(target_group_arn) = &action.target_group_arn {
            a_json.insert(
                "TargetGroupArn".to_string(),
                serde_json::Value::String(target_group_arn.clone()),
            );
        }

        if let Some(order) = action.order {
            a_json.insert("Order".to_string(), serde_json::Value::Number(order.into()));
        }

        // Forward config
        if let Some(forward_config) = &action.forward_config {
            let mut fc_json = serde_json::Map::new();
            if let Some(tgs) = &forward_config.target_groups {
                let tgs_json: Vec<serde_json::Value> = tgs
                    .iter()
                    .map(|tg| {
                        let mut tg_json = serde_json::Map::new();
                        if let Some(arn) = &tg.target_group_arn {
                            tg_json.insert(
                                "TargetGroupArn".to_string(),
                                serde_json::Value::String(arn.clone()),
                            );
                        }
                        if let Some(weight) = tg.weight {
                            tg_json.insert(
                                "Weight".to_string(),
                                serde_json::Value::Number(weight.into()),
                            );
                        }
                        serde_json::Value::Object(tg_json)
                    })
                    .collect();
                fc_json.insert(
                    "TargetGroups".to_string(),
                    serde_json::Value::Array(tgs_json),
                );
            }
            a_json.insert(
                "ForwardConfig".to_string(),
                serde_json::Value::Object(fc_json),
            );
        }

        // Redirect config
        if let Some(redirect_config) = &action.redirect_config {
            let mut rc_json = serde_json::Map::new();
            if let Some(protocol) = &redirect_config.protocol {
                rc_json.insert(
                    "Protocol".to_string(),
                    serde_json::Value::String(protocol.clone()),
                );
            }
            if let Some(port) = &redirect_config.port {
                rc_json.insert("Port".to_string(), serde_json::Value::String(port.clone()));
            }
            if let Some(host) = &redirect_config.host {
                rc_json.insert("Host".to_string(), serde_json::Value::String(host.clone()));
            }
            if let Some(path) = &redirect_config.path {
                rc_json.insert("Path".to_string(), serde_json::Value::String(path.clone()));
            }
            if let Some(query) = &redirect_config.query {
                rc_json.insert(
                    "Query".to_string(),
                    serde_json::Value::String(query.clone()),
                );
            }
            if let Some(status_code) = &redirect_config.status_code {
                rc_json.insert(
                    "StatusCode".to_string(),
                    serde_json::Value::String(status_code.as_str().to_string()),
                );
            }
            a_json.insert(
                "RedirectConfig".to_string(),
                serde_json::Value::Object(rc_json),
            );
        }

        // Fixed response config
        if let Some(fixed_response) = &action.fixed_response_config {
            let mut fr_json = serde_json::Map::new();
            if let Some(content_type) = &fixed_response.content_type {
                fr_json.insert(
                    "ContentType".to_string(),
                    serde_json::Value::String(content_type.clone()),
                );
            }
            if let Some(message_body) = &fixed_response.message_body {
                fr_json.insert(
                    "MessageBody".to_string(),
                    serde_json::Value::String(message_body.clone()),
                );
            }
            if let Some(status_code) = &fixed_response.status_code {
                fr_json.insert(
                    "StatusCode".to_string(),
                    serde_json::Value::String(status_code.clone()),
                );
            }
            a_json.insert(
                "FixedResponseConfig".to_string(),
                serde_json::Value::Object(fr_json),
            );
        }

        serde_json::Value::Object(a_json)
    }
}
