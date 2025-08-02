use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_elasticloadbalancingv2 as elbv2;
use std::sync::Arc;

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
        let response = client.describe_load_balancers().send().await?;

        let mut load_balancers = Vec::new();
        if let Some(lb_list) = response.load_balancers {
            for lb in lb_list {
                let lb_json = self.load_balancer_to_json(&lb);
                load_balancers.push(lb_json);
            }
        }

        Ok(load_balancers)
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
                return Ok(self.load_balancer_to_json(&lb));
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
    fn load_balancer_to_json(&self, lb: &elbv2::types::LoadBalancer) -> serde_json::Value {
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
}
