use anyhow::{Result, Context};
use aws_sdk_ec2 as ec2;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct EC2Service {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl EC2Service {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List EC2 instances (using describe_instances for detailed data)
    pub async fn list_instances(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = ec2::Client::new(&aws_config);
        let mut instances = Vec::new();

        // Use describe_instances for comprehensive instance data
        let mut paginator = client
            .describe_instances()
            .into_paginator()
            .send();

        while let Some(result) = paginator.try_next().await? {
            let reservations = result.reservations.unwrap_or_default();
            for reservation in reservations {
                let reservation_instances = reservation.instances.unwrap_or_default();
                for instance in reservation_instances {
                    let instance_json = self.instance_to_json(&instance);
                    instances.push(instance_json);
                }
            }
        }

        Ok(instances)
    }

    /// List VPCs (using describe_vpcs for detailed data)
    pub async fn list_vpcs(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_vpcs().send().await?;

        let mut vpcs = Vec::new();
        if let Some(vpc_list) = response.vpcs {
            for vpc in vpc_list {
                let vpc_json = self.vpc_to_json(&vpc);
                vpcs.push(vpc_json);
            }
        }

        Ok(vpcs)
    }

    /// List Security Groups (using describe_security_groups for detailed data)
    pub async fn list_security_groups(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_security_groups().send().await?;

        let mut security_groups = Vec::new();
        if let Some(groups) = response.security_groups {
            for group in groups {
                let group_json = self.security_group_to_json(&group);
                security_groups.push(group_json);
            }
        }

        Ok(security_groups)
    }

    /// Describe specific EC2 instance (already detailed from list)
    pub async fn describe_instance(
        &self,
        account_id: &str,
        region: &str,
        instance_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_instances()
            .instance_ids(instance_id)
            .send()
            .await?;

        // Extract the single instance from the response
        if let Some(reservations) = response.reservations {
            for reservation in reservations {
                if let Some(instances) = reservation.instances {
                    for instance in instances {
                        if let Some(id) = &instance.instance_id {
                            if id == instance_id {
                                return Ok(self.instance_to_json(&instance));
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Instance {} not found", instance_id))
    }

    /// Describe specific VPC (already detailed from list)
    pub async fn describe_vpc(
        &self,
        account_id: &str,
        region: &str,
        vpc_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_vpcs()
            .vpc_ids(vpc_id)
            .send()
            .await?;

        if let Some(vpcs) = response.vpcs {
            for vpc in vpcs {
                if let Some(id) = &vpc.vpc_id {
                    if id == vpc_id {
                        return Ok(self.vpc_to_json(&vpc));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("VPC {} not found", vpc_id))
    }

    /// Describe specific Security Group (already detailed from list)
    pub async fn describe_security_group(
        &self,
        account_id: &str,
        region: &str,
        group_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_security_groups()
            .group_ids(group_id)
            .send()
            .await?;

        if let Some(groups) = response.security_groups {
            for group in groups {
                if let Some(id) = &group.group_id {
                    if id == group_id {
                        return Ok(self.security_group_to_json(&group));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Security Group {} not found", group_id))
    }

    // JSON conversion methods
    fn instance_to_json(&self, instance: &ec2::types::Instance) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &instance.instance_id {
            json.insert("InstanceId".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(state) = &instance.state {
            if let Some(name) = &state.name {
                json.insert("State".to_string(), serde_json::Value::String(name.as_str().to_string()));
            }
        }

        if let Some(instance_type) = &instance.instance_type {
            json.insert("InstanceType".to_string(), serde_json::Value::String(instance_type.as_str().to_string()));
        }

        if let Some(vpc_id) = &instance.vpc_id {
            json.insert("VpcId".to_string(), serde_json::Value::String(vpc_id.clone()));
        }

        if let Some(subnet_id) = &instance.subnet_id {
            json.insert("SubnetId".to_string(), serde_json::Value::String(subnet_id.clone()));
        }

        if let Some(private_ip) = &instance.private_ip_address {
            json.insert("PrivateIpAddress".to_string(), serde_json::Value::String(private_ip.clone()));
        }

        if let Some(public_ip) = &instance.public_ip_address {
            json.insert("PublicIpAddress".to_string(), serde_json::Value::String(public_ip.clone()));
        }

        if let Some(launch_time) = &instance.launch_time {
            json.insert("LaunchTime".to_string(), serde_json::Value::String(launch_time.to_string()));
        }

        // Add security groups
        if let Some(ref security_groups) = instance.security_groups {
            if !security_groups.is_empty() {
            let security_groups_json: Vec<serde_json::Value> = security_groups
                .iter()
                .map(|sg| {
                    let mut sg_json = serde_json::Map::new();
                    if let Some(id) = &sg.group_id {
                        sg_json.insert("GroupId".to_string(), serde_json::Value::String(id.clone()));
                    }
                    if let Some(name) = &sg.group_name {
                        sg_json.insert("GroupName".to_string(), serde_json::Value::String(name.clone()));
                    }
                    serde_json::Value::Object(sg_json)
                })
                .collect();
            json.insert("SecurityGroups".to_string(), serde_json::Value::Array(security_groups_json));
        }
        }

        // Add tags
        if let Some(ref tags) = instance.tags {
            if !tags.is_empty() {
            let tags_json: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
        }
        }

        serde_json::Value::Object(json)
    }

    fn vpc_to_json(&self, vpc: &ec2::types::Vpc) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &vpc.vpc_id {
            json.insert("VpcId".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(cidr) = &vpc.cidr_block {
            json.insert("CidrBlock".to_string(), serde_json::Value::String(cidr.clone()));
        }

        if let Some(state) = &vpc.state {
            json.insert("State".to_string(), serde_json::Value::String(state.as_str().to_string()));
        }

        if let Some(is_default) = &vpc.is_default {
            json.insert("IsDefault".to_string(), serde_json::Value::Bool(*is_default));
        }

        // Add CIDR block associations
        if !vpc.cidr_block_association_set.is_empty() {
            let cidr_blocks: Vec<serde_json::Value> = vpc.cidr_block_association_set
                .iter()
                .map(|cb| {
                    let mut cb_json = serde_json::Map::new();
                    if let Some(cidr) = &cb.cidr_block {
                        cb_json.insert("CidrBlock".to_string(), serde_json::Value::String(cidr.clone()));
                    }
                    if let Some(state) = &cb.cidr_block_state {
                        if let Some(state_name) = &state.state {
                            cb_json.insert("State".to_string(), serde_json::Value::String(state_name.as_str().to_string()));
                        }
                    }
                    serde_json::Value::Object(cb_json)
                })
                .collect();
            json.insert("CidrBlockAssociationSet".to_string(), serde_json::Value::Array(cidr_blocks));
        }

        // Add tags
        if !vpc.tags.is_empty() {
            let tags: Vec<serde_json::Value> = vpc.tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Array(tags));
        }

        serde_json::Value::Object(json)
    }

    fn security_group_to_json(&self, group: &ec2::types::SecurityGroup) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &group.group_id {
            json.insert("GroupId".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(name) = &group.group_name {
            json.insert("GroupName".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &group.description {
            json.insert("Description".to_string(), serde_json::Value::String(description.clone()));
        }

        if let Some(vpc_id) = &group.vpc_id {
            json.insert("VpcId".to_string(), serde_json::Value::String(vpc_id.clone()));
        }

        // Add ingress rules
        if !group.ip_permissions.is_empty() {
            let ingress_rules: Vec<serde_json::Value> = group.ip_permissions
                .iter()
                .map(|rule| {
                    let mut rule_json = serde_json::Map::new();
                    if let Some(protocol) = &rule.ip_protocol {
                        rule_json.insert("IpProtocol".to_string(), serde_json::Value::String(protocol.clone()));
                    }
                    if let Some(from_port) = &rule.from_port {
                        rule_json.insert("FromPort".to_string(), serde_json::Value::Number((*from_port).into()));
                    }
                    if let Some(to_port) = &rule.to_port {
                        rule_json.insert("ToPort".to_string(), serde_json::Value::Number((*to_port).into()));
                    }

                    // Add IP ranges
                    if !rule.ip_ranges.is_empty() {
                        let ip_ranges: Vec<serde_json::Value> = rule.ip_ranges
                            .iter()
                            .map(|ip_range| {
                                let mut ip_json = serde_json::Map::new();
                                if let Some(cidr) = &ip_range.cidr_ip {
                                    ip_json.insert("CidrIp".to_string(), serde_json::Value::String(cidr.clone()));
                                }
                                serde_json::Value::Object(ip_json)
                            })
                            .collect();
                        rule_json.insert("IpRanges".to_string(), serde_json::Value::Array(ip_ranges));
                    }

                    serde_json::Value::Object(rule_json)
                })
                .collect();
            json.insert("IpPermissions".to_string(), serde_json::Value::Array(ingress_rules));
        }

        // Add egress rules
        if !group.ip_permissions_egress.is_empty() {
            let egress_rules: Vec<serde_json::Value> = group.ip_permissions_egress
                .iter()
                .map(|rule| {
                    let mut rule_json = serde_json::Map::new();
                    if let Some(protocol) = &rule.ip_protocol {
                        rule_json.insert("IpProtocol".to_string(), serde_json::Value::String(protocol.clone()));
                    }
                    if let Some(from_port) = &rule.from_port {
                        rule_json.insert("FromPort".to_string(), serde_json::Value::Number((*from_port).into()));
                    }
                    if let Some(to_port) = &rule.to_port {
                        rule_json.insert("ToPort".to_string(), serde_json::Value::Number((*to_port).into()));
                    }
                    serde_json::Value::Object(rule_json)
                })
                .collect();
            json.insert("IpPermissionsEgress".to_string(), serde_json::Value::Array(egress_rules));
        }

        // Add tags
        if !group.tags.is_empty() {
            let tags: Vec<serde_json::Value> = group.tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Array(tags));
        }

        serde_json::Value::Object(json)
    }
}