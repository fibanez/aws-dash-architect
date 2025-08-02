use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_ec2 as ec2;
use std::sync::Arc;

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

        let client = ec2::Client::new(&aws_config);
        let mut instances = Vec::new();

        // Use describe_instances for comprehensive instance data
        let mut paginator = client.describe_instances().into_paginator().send();

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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_vpcs().vpc_ids(vpc_id).send().await?;

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

    /// Describe specific EBS Volume
    pub async fn describe_volume(
        &self,
        account_id: &str,
        region: &str,
        volume_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_volumes()
            .volume_ids(volume_id)
            .send()
            .await?;

        if let Some(volumes) = response.volumes {
            for volume in volumes {
                if let Some(id) = &volume.volume_id {
                    if id == volume_id {
                        return Ok(self.volume_to_json(&volume));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Volume {} not found", volume_id))
    }

    /// Describe specific EBS Snapshot
    pub async fn describe_snapshot(
        &self,
        account_id: &str,
        region: &str,
        snapshot_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_snapshots()
            .snapshot_ids(snapshot_id)
            .send()
            .await?;

        if let Some(snapshots) = response.snapshots {
            for snapshot in snapshots {
                if let Some(id) = &snapshot.snapshot_id {
                    if id == snapshot_id {
                        return Ok(self.snapshot_to_json(&snapshot));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Snapshot {} not found", snapshot_id))
    }

    /// Describe specific AMI Image
    pub async fn describe_image(
        &self,
        account_id: &str,
        region: &str,
        image_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_images().image_ids(image_id).send().await?;

        if let Some(images) = response.images {
            for image in images {
                if let Some(id) = &image.image_id {
                    if id == image_id {
                        return Ok(self.ami_to_json(&image));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("AMI {} not found", image_id))
    }

    /// Describe specific Subnet
    pub async fn describe_subnet(
        &self,
        account_id: &str,
        region: &str,
        subnet_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_subnets()
            .subnet_ids(subnet_id)
            .send()
            .await?;

        if let Some(subnets) = response.subnets {
            for subnet in subnets {
                if let Some(id) = &subnet.subnet_id {
                    if id == subnet_id {
                        return Ok(self.subnet_to_json(&subnet));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Subnet {} not found", subnet_id))
    }

    /// Describe specific Route Table
    pub async fn describe_route_table(
        &self,
        account_id: &str,
        region: &str,
        route_table_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_route_tables()
            .route_table_ids(route_table_id)
            .send()
            .await?;

        if let Some(route_tables) = response.route_tables {
            for rt in route_tables {
                if let Some(id) = &rt.route_table_id {
                    if id == route_table_id {
                        return Ok(self.route_table_to_json(&rt));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Route Table {} not found", route_table_id))
    }

    /// Describe specific NAT Gateway
    pub async fn describe_nat_gateway(
        &self,
        account_id: &str,
        region: &str,
        nat_gateway_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_nat_gateways()
            .nat_gateway_ids(nat_gateway_id)
            .send()
            .await?;

        if let Some(nat_gateways) = response.nat_gateways {
            for ng in nat_gateways {
                if let Some(id) = &ng.nat_gateway_id {
                    if id == nat_gateway_id {
                        return Ok(self.nat_gateway_to_json(&ng));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("NAT Gateway {} not found", nat_gateway_id))
    }

    /// Describe specific Network Interface
    pub async fn describe_network_interface(
        &self,
        account_id: &str,
        region: &str,
        network_interface_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_network_interfaces()
            .network_interface_ids(network_interface_id)
            .send()
            .await?;

        if let Some(network_interfaces) = response.network_interfaces {
            for ni in network_interfaces {
                if let Some(id) = &ni.network_interface_id {
                    if id == network_interface_id {
                        return Ok(self.network_interface_to_json(&ni));
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Network Interface {} not found",
            network_interface_id
        ))
    }

    /// Describe specific VPC Endpoint
    pub async fn describe_vpc_endpoint(
        &self,
        account_id: &str,
        region: &str,
        vpc_endpoint_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_vpc_endpoints()
            .vpc_endpoint_ids(vpc_endpoint_id)
            .send()
            .await?;

        if let Some(vpc_endpoints) = response.vpc_endpoints {
            for ve in vpc_endpoints {
                if let Some(id) = &ve.vpc_endpoint_id {
                    if id == vpc_endpoint_id {
                        return Ok(self.vpc_endpoint_to_json(&ve));
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "VPC Endpoint {} not found",
            vpc_endpoint_id
        ))
    }

    /// Describe specific Network ACL
    pub async fn describe_network_acl(
        &self,
        account_id: &str,
        region: &str,
        network_acl_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_network_acls()
            .network_acl_ids(network_acl_id)
            .send()
            .await?;

        if let Some(network_acls) = response.network_acls {
            for acl in network_acls {
                if let Some(id) = &acl.network_acl_id {
                    if id == network_acl_id {
                        return Ok(self.network_acl_to_json(&acl));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Network ACL {} not found", network_acl_id))
    }

    /// Describe specific Key Pair
    pub async fn describe_key_pair(
        &self,
        account_id: &str,
        region: &str,
        key_name: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_key_pairs()
            .key_names(key_name)
            .send()
            .await?;

        if let Some(key_pairs) = response.key_pairs {
            for kp in key_pairs {
                if let Some(name) = &kp.key_name {
                    if name == key_name {
                        return Ok(self.key_pair_to_json(&kp));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Key Pair {} not found", key_name))
    }

    /// Describe specific Internet Gateway
    pub async fn describe_internet_gateway(
        &self,
        account_id: &str,
        region: &str,
        internet_gateway_id: &str,
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_internet_gateways()
            .internet_gateway_ids(internet_gateway_id)
            .send()
            .await?;

        if let Some(internet_gateways) = response.internet_gateways {
            for ig in internet_gateways {
                if let Some(id) = &ig.internet_gateway_id {
                    if id == internet_gateway_id {
                        return Ok(self.internet_gateway_to_json(&ig));
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Internet Gateway {} not found",
            internet_gateway_id
        ))
    }

    // JSON conversion methods
    fn instance_to_json(&self, instance: &ec2::types::Instance) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &instance.instance_id {
            json.insert(
                "InstanceId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(state) = &instance.state {
            if let Some(name) = &state.name {
                json.insert(
                    "State".to_string(),
                    serde_json::Value::String(name.as_str().to_string()),
                );
            }
        }

        if let Some(instance_type) = &instance.instance_type {
            json.insert(
                "InstanceType".to_string(),
                serde_json::Value::String(instance_type.as_str().to_string()),
            );
        }

        if let Some(vpc_id) = &instance.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(subnet_id) = &instance.subnet_id {
            json.insert(
                "SubnetId".to_string(),
                serde_json::Value::String(subnet_id.clone()),
            );
        }

        if let Some(private_ip) = &instance.private_ip_address {
            json.insert(
                "PrivateIpAddress".to_string(),
                serde_json::Value::String(private_ip.clone()),
            );
        }

        if let Some(public_ip) = &instance.public_ip_address {
            json.insert(
                "PublicIpAddress".to_string(),
                serde_json::Value::String(public_ip.clone()),
            );
        }

        if let Some(launch_time) = &instance.launch_time {
            json.insert(
                "LaunchTime".to_string(),
                serde_json::Value::String(launch_time.to_string()),
            );
        }

        // Add security groups
        if let Some(ref security_groups) = instance.security_groups {
            if !security_groups.is_empty() {
                let security_groups_json: Vec<serde_json::Value> = security_groups
                    .iter()
                    .map(|sg| {
                        let mut sg_json = serde_json::Map::new();
                        if let Some(id) = &sg.group_id {
                            sg_json.insert(
                                "GroupId".to_string(),
                                serde_json::Value::String(id.clone()),
                            );
                        }
                        if let Some(name) = &sg.group_name {
                            sg_json.insert(
                                "GroupName".to_string(),
                                serde_json::Value::String(name.clone()),
                            );
                        }
                        serde_json::Value::Object(sg_json)
                    })
                    .collect();
                json.insert(
                    "SecurityGroups".to_string(),
                    serde_json::Value::Array(security_groups_json),
                );
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
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
            json.insert(
                "CidrBlock".to_string(),
                serde_json::Value::String(cidr.clone()),
            );
        }

        if let Some(state) = &vpc.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(is_default) = &vpc.is_default {
            json.insert(
                "IsDefault".to_string(),
                serde_json::Value::Bool(*is_default),
            );
        }

        // Add CIDR block associations
        if let Some(ref cidr_block_associations) = vpc.cidr_block_association_set {
            if !cidr_block_associations.is_empty() {
                let cidr_blocks: Vec<serde_json::Value> = cidr_block_associations
                    .iter()
                    .map(|cb| {
                        let mut cb_json = serde_json::Map::new();
                        if let Some(cidr) = &cb.cidr_block {
                            cb_json.insert(
                                "CidrBlock".to_string(),
                                serde_json::Value::String(cidr.clone()),
                            );
                        }
                        if let Some(state) = &cb.cidr_block_state {
                            if let Some(state_name) = &state.state {
                                cb_json.insert(
                                    "State".to_string(),
                                    serde_json::Value::String(state_name.as_str().to_string()),
                                );
                            }
                        }
                        serde_json::Value::Object(cb_json)
                    })
                    .collect();
                json.insert(
                    "CidrBlockAssociationSet".to_string(),
                    serde_json::Value::Array(cidr_blocks),
                );
            }
        }

        // Add tags
        if let Some(ref tags) = vpc.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    fn security_group_to_json(&self, group: &ec2::types::SecurityGroup) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &group.group_id {
            json.insert("GroupId".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(name) = &group.group_name {
            json.insert(
                "GroupName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(description) = &group.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(vpc_id) = &group.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        // Add ingress rules
        if let Some(ref ip_permissions) = group.ip_permissions {
            if !ip_permissions.is_empty() {
                let ingress_rules: Vec<serde_json::Value> = ip_permissions
                    .iter()
                    .map(|rule| {
                        let mut rule_json = serde_json::Map::new();
                        if let Some(protocol) = &rule.ip_protocol {
                            rule_json.insert(
                                "IpProtocol".to_string(),
                                serde_json::Value::String(protocol.clone()),
                            );
                        }
                        if let Some(from_port) = &rule.from_port {
                            rule_json.insert(
                                "FromPort".to_string(),
                                serde_json::Value::Number((*from_port).into()),
                            );
                        }
                        if let Some(to_port) = &rule.to_port {
                            rule_json.insert(
                                "ToPort".to_string(),
                                serde_json::Value::Number((*to_port).into()),
                            );
                        }

                        // Add IP ranges
                        if let Some(ref ip_ranges) = rule.ip_ranges {
                            if !ip_ranges.is_empty() {
                                let ip_ranges_json: Vec<serde_json::Value> = ip_ranges
                                    .iter()
                                    .map(|ip_range| {
                                        let mut ip_json = serde_json::Map::new();
                                        if let Some(cidr) = &ip_range.cidr_ip {
                                            ip_json.insert(
                                                "CidrIp".to_string(),
                                                serde_json::Value::String(cidr.clone()),
                                            );
                                        }
                                        serde_json::Value::Object(ip_json)
                                    })
                                    .collect();
                                rule_json.insert(
                                    "IpRanges".to_string(),
                                    serde_json::Value::Array(ip_ranges_json),
                                );
                            }
                        }

                        serde_json::Value::Object(rule_json)
                    })
                    .collect();
                json.insert(
                    "IpPermissions".to_string(),
                    serde_json::Value::Array(ingress_rules),
                );
            }
        }

        // Add egress rules
        if let Some(ref ip_permissions_egress) = group.ip_permissions_egress {
            if !ip_permissions_egress.is_empty() {
                let egress_rules: Vec<serde_json::Value> = ip_permissions_egress
                    .iter()
                    .map(|rule| {
                        let mut rule_json = serde_json::Map::new();
                        if let Some(protocol) = &rule.ip_protocol {
                            rule_json.insert(
                                "IpProtocol".to_string(),
                                serde_json::Value::String(protocol.clone()),
                            );
                        }
                        if let Some(from_port) = &rule.from_port {
                            rule_json.insert(
                                "FromPort".to_string(),
                                serde_json::Value::Number((*from_port).into()),
                            );
                        }
                        if let Some(to_port) = &rule.to_port {
                            rule_json.insert(
                                "ToPort".to_string(),
                                serde_json::Value::Number((*to_port).into()),
                            );
                        }
                        serde_json::Value::Object(rule_json)
                    })
                    .collect();
                json.insert(
                    "IpPermissionsEgress".to_string(),
                    serde_json::Value::Array(egress_rules),
                );
            }
        }

        // Add tags
        if let Some(ref tags) = group.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    /// List EBS volumes
    pub async fn list_volumes(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_volumes().send().await?;

        let mut volumes = Vec::new();
        if let Some(volume_list) = response.volumes {
            for volume in volume_list {
                let volume_json = self.volume_to_json(&volume);
                volumes.push(volume_json);
            }
        }

        Ok(volumes)
    }

    /// List EBS snapshots
    pub async fn list_snapshots(
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_snapshots()
            .owner_ids("self") // Only own snapshots to avoid too much data
            .send()
            .await?;

        let mut snapshots = Vec::new();
        if let Some(snapshot_list) = response.snapshots {
            for snapshot in snapshot_list {
                let snapshot_json = self.snapshot_to_json(&snapshot);
                snapshots.push(snapshot_json);
            }
        }

        Ok(snapshots)
    }

    /// List AMIs
    pub async fn list_amis(
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

        let client = ec2::Client::new(&aws_config);
        let response = client
            .describe_images()
            .owners("self") // Only own AMIs to avoid too much data
            .send()
            .await?;

        let mut amis = Vec::new();
        if let Some(image_list) = response.images {
            for image in image_list {
                let image_json = self.ami_to_json(&image);
                amis.push(image_json);
            }
        }

        Ok(amis)
    }

    /// List subnets
    pub async fn list_subnets(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_subnets().send().await?;

        let mut subnets = Vec::new();
        if let Some(subnet_list) = response.subnets {
            for subnet in subnet_list {
                let subnet_json = self.subnet_to_json(&subnet);
                subnets.push(subnet_json);
            }
        }

        Ok(subnets)
    }

    /// List internet gateways
    pub async fn list_internet_gateways(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_internet_gateways().send().await?;

        let mut igws = Vec::new();
        if let Some(igw_list) = response.internet_gateways {
            for igw in igw_list {
                let igw_json = self.internet_gateway_to_json(&igw);
                igws.push(igw_json);
            }
        }

        Ok(igws)
    }

    /// List route tables
    pub async fn list_route_tables(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_route_tables().send().await?;

        let mut route_tables = Vec::new();
        if let Some(route_table_list) = response.route_tables {
            for route_table in route_table_list {
                let route_table_json = self.route_table_to_json(&route_table);
                route_tables.push(route_table_json);
            }
        }

        Ok(route_tables)
    }

    /// List NAT gateways
    pub async fn list_nat_gateways(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_nat_gateways().send().await?;

        let mut nat_gateways = Vec::new();
        if let Some(nat_gateway_list) = response.nat_gateways {
            for nat_gateway in nat_gateway_list {
                let nat_gateway_json = self.nat_gateway_to_json(&nat_gateway);
                nat_gateways.push(nat_gateway_json);
            }
        }

        Ok(nat_gateways)
    }

    /// List network interfaces
    pub async fn list_network_interfaces(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_network_interfaces().send().await?;

        let mut network_interfaces = Vec::new();
        if let Some(eni_list) = response.network_interfaces {
            for eni in eni_list {
                let eni_json = self.network_interface_to_json(&eni);
                network_interfaces.push(eni_json);
            }
        }

        Ok(network_interfaces)
    }

    /// List VPC endpoints
    pub async fn list_vpc_endpoints(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_vpc_endpoints().send().await?;

        let mut vpc_endpoints = Vec::new();
        if let Some(endpoint_list) = response.vpc_endpoints {
            for endpoint in endpoint_list {
                let endpoint_json = self.vpc_endpoint_to_json(&endpoint);
                vpc_endpoints.push(endpoint_json);
            }
        }

        Ok(vpc_endpoints)
    }

    /// List network ACLs
    pub async fn list_network_acls(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_network_acls().send().await?;

        let mut network_acls = Vec::new();
        if let Some(acl_list) = response.network_acls {
            for acl in acl_list {
                let acl_json = self.network_acl_to_json(&acl);
                network_acls.push(acl_json);
            }
        }

        Ok(network_acls)
    }

    /// List key pairs
    pub async fn list_key_pairs(
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

        let client = ec2::Client::new(&aws_config);
        let response = client.describe_key_pairs().send().await?;

        let mut key_pairs = Vec::new();
        if let Some(kp_list) = response.key_pairs {
            for kp in kp_list {
                let kp_json = self.key_pair_to_json(&kp);
                key_pairs.push(kp_json);
            }
        }

        Ok(key_pairs)
    }

    /// Convert EBS volume to JSON format
    fn volume_to_json(&self, volume: &ec2::types::Volume) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(volume_id) = &volume.volume_id {
            json.insert(
                "VolumeId".to_string(),
                serde_json::Value::String(volume_id.clone()),
            );
        }

        if let Some(size) = volume.size {
            json.insert("Size".to_string(), serde_json::Value::Number(size.into()));
        }

        if let Some(volume_type) = &volume.volume_type {
            json.insert(
                "VolumeType".to_string(),
                serde_json::Value::String(volume_type.as_str().to_string()),
            );
        }

        if let Some(state) = &volume.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(create_time) = volume.create_time {
            json.insert(
                "CreateTime".to_string(),
                serde_json::Value::String(create_time.to_string()),
            );
        }

        if let Some(availability_zone) = &volume.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(encrypted) = volume.encrypted {
            json.insert("Encrypted".to_string(), serde_json::Value::Bool(encrypted));
        }

        if let Some(iops) = volume.iops {
            json.insert("Iops".to_string(), serde_json::Value::Number(iops.into()));
        }

        if let Some(attachments) = &volume.attachments {
            if !attachments.is_empty() {
                let attachments_json: Vec<serde_json::Value> = attachments
                    .iter()
                    .map(|attachment| {
                        let mut attach_json = serde_json::Map::new();
                        if let Some(instance_id) = &attachment.instance_id {
                            attach_json.insert(
                                "InstanceId".to_string(),
                                serde_json::Value::String(instance_id.clone()),
                            );
                        }
                        if let Some(device) = &attachment.device {
                            attach_json.insert(
                                "Device".to_string(),
                                serde_json::Value::String(device.clone()),
                            );
                        }
                        if let Some(state) = &attachment.state {
                            attach_json.insert(
                                "State".to_string(),
                                serde_json::Value::String(state.as_str().to_string()),
                            );
                        }
                        serde_json::Value::Object(attach_json)
                    })
                    .collect();
                json.insert(
                    "Attachments".to_string(),
                    serde_json::Value::Array(attachments_json),
                );
            }
        }

        if let Some(tags) = &volume.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        // Try to extract name from tags
        if let Some(tags) = &volume.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use volume ID
        if !json.contains_key("Name") {
            if let Some(volume_id) = &volume.volume_id {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(volume_id.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert EBS snapshot to JSON format
    fn snapshot_to_json(&self, snapshot: &ec2::types::Snapshot) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(snapshot_id) = &snapshot.snapshot_id {
            json.insert(
                "SnapshotId".to_string(),
                serde_json::Value::String(snapshot_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(snapshot_id.clone()),
            );
        }

        if let Some(description) = &snapshot.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(volume_id) = &snapshot.volume_id {
            json.insert(
                "VolumeId".to_string(),
                serde_json::Value::String(volume_id.clone()),
            );
        }

        if let Some(state) = &snapshot.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(start_time) = snapshot.start_time {
            json.insert(
                "StartTime".to_string(),
                serde_json::Value::String(start_time.to_string()),
            );
        }

        if let Some(progress) = &snapshot.progress {
            json.insert(
                "Progress".to_string(),
                serde_json::Value::String(progress.clone()),
            );
        }

        if let Some(volume_size) = snapshot.volume_size {
            json.insert(
                "VolumeSize".to_string(),
                serde_json::Value::Number(volume_size.into()),
            );
        }

        if let Some(encrypted) = snapshot.encrypted {
            json.insert("Encrypted".to_string(), serde_json::Value::Bool(encrypted));
        }

        if let Some(tags) = &snapshot.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert AMI to JSON format
    fn ami_to_json(&self, image: &ec2::types::Image) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(image_id) = &image.image_id {
            json.insert(
                "ImageId".to_string(),
                serde_json::Value::String(image_id.clone()),
            );
        }

        if let Some(name) = &image.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        } else if let Some(image_id) = &image.image_id {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(image_id.clone()),
            );
        }

        if let Some(description) = &image.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(state) = &image.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(creation_date) = &image.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.clone()),
            );
        }

        if let Some(architecture) = &image.architecture {
            json.insert(
                "Architecture".to_string(),
                serde_json::Value::String(architecture.as_str().to_string()),
            );
        }

        if let Some(virtualization_type) = &image.virtualization_type {
            json.insert(
                "VirtualizationType".to_string(),
                serde_json::Value::String(virtualization_type.as_str().to_string()),
            );
        }

        if let Some(public) = image.public {
            json.insert("Public".to_string(), serde_json::Value::Bool(public));
        }

        if let Some(tags) = &image.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert subnet to JSON format
    fn subnet_to_json(&self, subnet: &ec2::types::Subnet) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(subnet_id) = &subnet.subnet_id {
            json.insert(
                "SubnetId".to_string(),
                serde_json::Value::String(subnet_id.clone()),
            );
        }

        if let Some(vpc_id) = &subnet.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(cidr_block) = &subnet.cidr_block {
            json.insert(
                "CidrBlock".to_string(),
                serde_json::Value::String(cidr_block.clone()),
            );
        }

        if let Some(availability_zone) = &subnet.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(state) = &subnet.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(available_ip_address_count) = subnet.available_ip_address_count {
            json.insert(
                "AvailableIpAddressCount".to_string(),
                serde_json::Value::Number(available_ip_address_count.into()),
            );
        }

        if let Some(map_public_ip_on_launch) = subnet.map_public_ip_on_launch {
            json.insert(
                "MapPublicIpOnLaunch".to_string(),
                serde_json::Value::Bool(map_public_ip_on_launch),
            );
        }

        if let Some(tags) = &subnet.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        // Try to extract name from tags
        if let Some(tags) = &subnet.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use subnet ID
        if !json.contains_key("Name") {
            if let Some(subnet_id) = &subnet.subnet_id {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(subnet_id.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert internet gateway to JSON format
    fn internet_gateway_to_json(&self, igw: &ec2::types::InternetGateway) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(igw_id) = &igw.internet_gateway_id {
            json.insert(
                "InternetGatewayId".to_string(),
                serde_json::Value::String(igw_id.clone()),
            );
        }

        if let Some(attachments) = &igw.attachments {
            if !attachments.is_empty() {
                let attachments_json: Vec<serde_json::Value> = attachments
                    .iter()
                    .map(|attachment| {
                        let mut attach_json = serde_json::Map::new();
                        if let Some(vpc_id) = &attachment.vpc_id {
                            attach_json.insert(
                                "VpcId".to_string(),
                                serde_json::Value::String(vpc_id.clone()),
                            );
                        }
                        if let Some(state) = &attachment.state {
                            attach_json.insert(
                                "State".to_string(),
                                serde_json::Value::String(state.as_str().to_string()),
                            );
                        }
                        serde_json::Value::Object(attach_json)
                    })
                    .collect();
                json.insert(
                    "Attachments".to_string(),
                    serde_json::Value::Array(attachments_json),
                );
            }
        }

        if let Some(tags) = &igw.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        // Try to extract name from tags
        if let Some(tags) = &igw.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use IGW ID
        if !json.contains_key("Name") {
            if let Some(igw_id) = &igw.internet_gateway_id {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(igw_id.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert route table to JSON format
    fn route_table_to_json(&self, route_table: &ec2::types::RouteTable) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(route_table_id) = &route_table.route_table_id {
            json.insert(
                "RouteTableId".to_string(),
                serde_json::Value::String(route_table_id.clone()),
            );
        }

        if let Some(vpc_id) = &route_table.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        // Add routes array
        if let Some(routes) = &route_table.routes {
            let routes_json: Vec<serde_json::Value> = routes
                .iter()
                .map(|route| {
                    let mut route_json = serde_json::Map::new();

                    if let Some(destination_cidr_block) = &route.destination_cidr_block {
                        route_json.insert(
                            "DestinationCidrBlock".to_string(),
                            serde_json::Value::String(destination_cidr_block.clone()),
                        );
                    }

                    if let Some(destination_ipv6_cidr_block) = &route.destination_ipv6_cidr_block {
                        route_json.insert(
                            "DestinationIpv6CidrBlock".to_string(),
                            serde_json::Value::String(destination_ipv6_cidr_block.clone()),
                        );
                    }

                    if let Some(gateway_id) = &route.gateway_id {
                        route_json.insert(
                            "GatewayId".to_string(),
                            serde_json::Value::String(gateway_id.clone()),
                        );
                    }

                    if let Some(instance_id) = &route.instance_id {
                        route_json.insert(
                            "InstanceId".to_string(),
                            serde_json::Value::String(instance_id.clone()),
                        );
                    }

                    if let Some(nat_gateway_id) = &route.nat_gateway_id {
                        route_json.insert(
                            "NatGatewayId".to_string(),
                            serde_json::Value::String(nat_gateway_id.clone()),
                        );
                    }

                    if let Some(network_interface_id) = &route.network_interface_id {
                        route_json.insert(
                            "NetworkInterfaceId".to_string(),
                            serde_json::Value::String(network_interface_id.clone()),
                        );
                    }

                    if let Some(vpc_peering_connection_id) = &route.vpc_peering_connection_id {
                        route_json.insert(
                            "VpcPeeringConnectionId".to_string(),
                            serde_json::Value::String(vpc_peering_connection_id.clone()),
                        );
                    }

                    if let Some(state) = &route.state {
                        route_json.insert(
                            "State".to_string(),
                            serde_json::Value::String(state.as_str().to_string()),
                        );
                    }

                    if let Some(origin) = &route.origin {
                        route_json.insert(
                            "Origin".to_string(),
                            serde_json::Value::String(origin.as_str().to_string()),
                        );
                    }

                    serde_json::Value::Object(route_json)
                })
                .collect();
            json.insert("Routes".to_string(), serde_json::Value::Array(routes_json));
        }

        // Add associations array
        if let Some(associations) = &route_table.associations {
            let associations_json: Vec<serde_json::Value> = associations
                .iter()
                .map(|association| {
                    let mut assoc_json = serde_json::Map::new();

                    if let Some(route_table_association_id) =
                        &association.route_table_association_id
                    {
                        assoc_json.insert(
                            "RouteTableAssociationId".to_string(),
                            serde_json::Value::String(route_table_association_id.clone()),
                        );
                    }

                    if let Some(route_table_id) = &association.route_table_id {
                        assoc_json.insert(
                            "RouteTableId".to_string(),
                            serde_json::Value::String(route_table_id.clone()),
                        );
                    }

                    if let Some(subnet_id) = &association.subnet_id {
                        assoc_json.insert(
                            "SubnetId".to_string(),
                            serde_json::Value::String(subnet_id.clone()),
                        );
                    }

                    if let Some(gateway_id) = &association.gateway_id {
                        assoc_json.insert(
                            "GatewayId".to_string(),
                            serde_json::Value::String(gateway_id.clone()),
                        );
                    }

                    if let Some(main) = association.main {
                        assoc_json.insert("Main".to_string(), serde_json::Value::Bool(main));
                    }

                    if let Some(association_state) = &association.association_state {
                        if let Some(state) = &association_state.state {
                            assoc_json.insert(
                                "AssociationState".to_string(),
                                serde_json::Value::String(state.as_str().to_string()),
                            );
                        }
                    }

                    serde_json::Value::Object(assoc_json)
                })
                .collect();
            json.insert(
                "Associations".to_string(),
                serde_json::Value::Array(associations_json),
            );
        }

        // Add owner ID
        if let Some(owner_id) = &route_table.owner_id {
            json.insert(
                "OwnerId".to_string(),
                serde_json::Value::String(owner_id.clone()),
            );
        }

        // Extract tags and handle Name tag specially
        if let Some(tags) = &route_table.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use route table ID
        if !json.contains_key("Name") {
            if let Some(route_table_id) = &route_table.route_table_id {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(route_table_id.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert NAT gateway to JSON format
    fn nat_gateway_to_json(&self, nat_gateway: &ec2::types::NatGateway) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(nat_gateway_id) = &nat_gateway.nat_gateway_id {
            json.insert(
                "NatGatewayId".to_string(),
                serde_json::Value::String(nat_gateway_id.clone()),
            );
        }

        if let Some(vpc_id) = &nat_gateway.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(subnet_id) = &nat_gateway.subnet_id {
            json.insert(
                "SubnetId".to_string(),
                serde_json::Value::String(subnet_id.clone()),
            );
        }

        if let Some(state) = &nat_gateway.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(connectivity_type) = &nat_gateway.connectivity_type {
            json.insert(
                "ConnectivityType".to_string(),
                serde_json::Value::String(connectivity_type.as_str().to_string()),
            );
        }

        if let Some(create_time) = nat_gateway.create_time {
            json.insert(
                "CreateTime".to_string(),
                serde_json::Value::String(create_time.to_string()),
            );
        }

        if let Some(delete_time) = nat_gateway.delete_time {
            json.insert(
                "DeleteTime".to_string(),
                serde_json::Value::String(delete_time.to_string()),
            );
        }

        if let Some(failure_code) = &nat_gateway.failure_code {
            json.insert(
                "FailureCode".to_string(),
                serde_json::Value::String(failure_code.clone()),
            );
        }

        if let Some(failure_message) = &nat_gateway.failure_message {
            json.insert(
                "FailureMessage".to_string(),
                serde_json::Value::String(failure_message.clone()),
            );
        }

        // Add NAT gateway addresses (EIP allocations)
        if let Some(nat_gateway_addresses) = &nat_gateway.nat_gateway_addresses {
            let addresses_json: Vec<serde_json::Value> = nat_gateway_addresses
                .iter()
                .map(|address| {
                    let mut addr_json = serde_json::Map::new();

                    if let Some(allocation_id) = &address.allocation_id {
                        addr_json.insert(
                            "AllocationId".to_string(),
                            serde_json::Value::String(allocation_id.clone()),
                        );
                    }

                    if let Some(network_interface_id) = &address.network_interface_id {
                        addr_json.insert(
                            "NetworkInterfaceId".to_string(),
                            serde_json::Value::String(network_interface_id.clone()),
                        );
                    }

                    if let Some(private_ip) = &address.private_ip {
                        addr_json.insert(
                            "PrivateIp".to_string(),
                            serde_json::Value::String(private_ip.clone()),
                        );
                    }

                    if let Some(public_ip) = &address.public_ip {
                        addr_json.insert(
                            "PublicIp".to_string(),
                            serde_json::Value::String(public_ip.clone()),
                        );
                    }

                    if let Some(association_id) = &address.association_id {
                        addr_json.insert(
                            "AssociationId".to_string(),
                            serde_json::Value::String(association_id.clone()),
                        );
                    }

                    if let Some(is_primary) = address.is_primary {
                        addr_json
                            .insert("IsPrimary".to_string(), serde_json::Value::Bool(is_primary));
                    }

                    if let Some(failure_message) = &address.failure_message {
                        addr_json.insert(
                            "FailureMessage".to_string(),
                            serde_json::Value::String(failure_message.clone()),
                        );
                    }

                    if let Some(status) = &address.status {
                        addr_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(status.as_str().to_string()),
                        );
                    }

                    serde_json::Value::Object(addr_json)
                })
                .collect();
            json.insert(
                "NatGatewayAddresses".to_string(),
                serde_json::Value::Array(addresses_json),
            );
        }

        // Extract tags and handle Name tag specially
        if let Some(tags) = &nat_gateway.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use NAT gateway ID
        if !json.contains_key("Name") {
            if let Some(nat_gateway_id) = &nat_gateway.nat_gateway_id {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(nat_gateway_id.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert network interface to JSON format
    fn network_interface_to_json(&self, eni: &ec2::types::NetworkInterface) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(network_interface_id) = &eni.network_interface_id {
            json.insert(
                "NetworkInterfaceId".to_string(),
                serde_json::Value::String(network_interface_id.clone()),
            );
        }

        if let Some(subnet_id) = &eni.subnet_id {
            json.insert(
                "SubnetId".to_string(),
                serde_json::Value::String(subnet_id.clone()),
            );
        }

        if let Some(vpc_id) = &eni.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(availability_zone) = &eni.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(description) = &eni.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(interface_type) = &eni.interface_type {
            json.insert(
                "InterfaceType".to_string(),
                serde_json::Value::String(interface_type.as_str().to_string()),
            );
        }

        if let Some(owner_id) = &eni.owner_id {
            json.insert(
                "OwnerId".to_string(),
                serde_json::Value::String(owner_id.clone()),
            );
        }

        if let Some(private_dns_name) = &eni.private_dns_name {
            json.insert(
                "PrivateDnsName".to_string(),
                serde_json::Value::String(private_dns_name.clone()),
            );
        }

        if let Some(private_ip_address) = &eni.private_ip_address {
            json.insert(
                "PrivateIpAddress".to_string(),
                serde_json::Value::String(private_ip_address.clone()),
            );
        }

        if let Some(source_dest_check) = eni.source_dest_check {
            json.insert(
                "SourceDestCheck".to_string(),
                serde_json::Value::Bool(source_dest_check),
            );
        }

        if let Some(status) = &eni.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(mac_address) = &eni.mac_address {
            json.insert(
                "MacAddress".to_string(),
                serde_json::Value::String(mac_address.clone()),
            );
        }

        // Add private IP addresses
        if let Some(private_ip_addresses) = &eni.private_ip_addresses {
            let private_ips_json: Vec<serde_json::Value> = private_ip_addresses
                .iter()
                .map(|ip| {
                    let mut ip_json = serde_json::Map::new();

                    if let Some(private_ip_address) = &ip.private_ip_address {
                        ip_json.insert(
                            "PrivateIpAddress".to_string(),
                            serde_json::Value::String(private_ip_address.clone()),
                        );
                    }

                    if let Some(private_dns_name) = &ip.private_dns_name {
                        ip_json.insert(
                            "PrivateDnsName".to_string(),
                            serde_json::Value::String(private_dns_name.clone()),
                        );
                    }

                    if let Some(primary) = ip.primary {
                        ip_json.insert("Primary".to_string(), serde_json::Value::Bool(primary));
                    }

                    if let Some(association) = &ip.association {
                        let mut assoc_json = serde_json::Map::new();

                        if let Some(public_ip) = &association.public_ip {
                            assoc_json.insert(
                                "PublicIp".to_string(),
                                serde_json::Value::String(public_ip.clone()),
                            );
                        }

                        if let Some(public_dns_name) = &association.public_dns_name {
                            assoc_json.insert(
                                "PublicDnsName".to_string(),
                                serde_json::Value::String(public_dns_name.clone()),
                            );
                        }

                        if let Some(allocation_id) = &association.allocation_id {
                            assoc_json.insert(
                                "AllocationId".to_string(),
                                serde_json::Value::String(allocation_id.clone()),
                            );
                        }

                        if let Some(association_id) = &association.association_id {
                            assoc_json.insert(
                                "AssociationId".to_string(),
                                serde_json::Value::String(association_id.clone()),
                            );
                        }

                        if let Some(ip_owner_id) = &association.ip_owner_id {
                            assoc_json.insert(
                                "IpOwnerId".to_string(),
                                serde_json::Value::String(ip_owner_id.clone()),
                            );
                        }

                        ip_json.insert(
                            "Association".to_string(),
                            serde_json::Value::Object(assoc_json),
                        );
                    }

                    serde_json::Value::Object(ip_json)
                })
                .collect();
            json.insert(
                "PrivateIpAddresses".to_string(),
                serde_json::Value::Array(private_ips_json),
            );
        }

        // Add security groups
        if let Some(groups) = &eni.groups {
            let groups_json: Vec<serde_json::Value> = groups
                .iter()
                .map(|group| {
                    let mut group_json = serde_json::Map::new();

                    if let Some(group_id) = &group.group_id {
                        group_json.insert(
                            "GroupId".to_string(),
                            serde_json::Value::String(group_id.clone()),
                        );
                    }

                    if let Some(group_name) = &group.group_name {
                        group_json.insert(
                            "GroupName".to_string(),
                            serde_json::Value::String(group_name.clone()),
                        );
                    }

                    serde_json::Value::Object(group_json)
                })
                .collect();
            json.insert("Groups".to_string(), serde_json::Value::Array(groups_json));
        }

        // Add attachment information
        if let Some(attachment) = &eni.attachment {
            let mut attachment_json = serde_json::Map::new();

            if let Some(attachment_id) = &attachment.attachment_id {
                attachment_json.insert(
                    "AttachmentId".to_string(),
                    serde_json::Value::String(attachment_id.clone()),
                );
            }

            if let Some(instance_id) = &attachment.instance_id {
                attachment_json.insert(
                    "InstanceId".to_string(),
                    serde_json::Value::String(instance_id.clone()),
                );
            }

            if let Some(instance_owner_id) = &attachment.instance_owner_id {
                attachment_json.insert(
                    "InstanceOwnerId".to_string(),
                    serde_json::Value::String(instance_owner_id.clone()),
                );
            }

            if let Some(device_index) = attachment.device_index {
                attachment_json.insert(
                    "DeviceIndex".to_string(),
                    serde_json::Value::Number(device_index.into()),
                );
            }

            if let Some(status) = &attachment.status {
                attachment_json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(status.as_str().to_string()),
                );
            }

            if let Some(attach_time) = attachment.attach_time {
                attachment_json.insert(
                    "AttachTime".to_string(),
                    serde_json::Value::String(attach_time.to_string()),
                );
            }

            if let Some(delete_on_termination) = attachment.delete_on_termination {
                attachment_json.insert(
                    "DeleteOnTermination".to_string(),
                    serde_json::Value::Bool(delete_on_termination),
                );
            }

            json.insert(
                "Attachment".to_string(),
                serde_json::Value::Object(attachment_json),
            );
        }

        // Extract tags and handle Name tag specially
        if let Some(tags) = &eni.tag_set {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use network interface ID or description
        if !json.contains_key("Name") {
            if let Some(description) = &eni.description {
                if !description.is_empty() {
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(description.clone()),
                    );
                }
            }

            // If still no name, use the network interface ID
            if !json.contains_key("Name") {
                if let Some(network_interface_id) = &eni.network_interface_id {
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(network_interface_id.clone()),
                    );
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert VPC endpoint to JSON format
    fn vpc_endpoint_to_json(&self, endpoint: &ec2::types::VpcEndpoint) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(vpc_endpoint_id) = &endpoint.vpc_endpoint_id {
            json.insert(
                "VpcEndpointId".to_string(),
                serde_json::Value::String(vpc_endpoint_id.clone()),
            );
        }

        if let Some(vpc_id) = &endpoint.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(service_name) = &endpoint.service_name {
            json.insert(
                "ServiceName".to_string(),
                serde_json::Value::String(service_name.clone()),
            );
        }

        if let Some(state) = &endpoint.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(vpc_endpoint_type) = &endpoint.vpc_endpoint_type {
            json.insert(
                "VpcEndpointType".to_string(),
                serde_json::Value::String(vpc_endpoint_type.as_str().to_string()),
            );
        }

        if let Some(policy_document) = &endpoint.policy_document {
            json.insert(
                "PolicyDocument".to_string(),
                serde_json::Value::String(policy_document.clone()),
            );
        }

        if let Some(creation_timestamp) = endpoint.creation_timestamp {
            json.insert(
                "CreationTimestamp".to_string(),
                serde_json::Value::String(creation_timestamp.to_string()),
            );
        }

        if let Some(owner_id) = &endpoint.owner_id {
            json.insert(
                "OwnerId".to_string(),
                serde_json::Value::String(owner_id.clone()),
            );
        }

        if let Some(private_dns_enabled) = endpoint.private_dns_enabled {
            json.insert(
                "PrivateDnsEnabled".to_string(),
                serde_json::Value::Bool(private_dns_enabled),
            );
        }

        if let Some(requester_managed) = endpoint.requester_managed {
            json.insert(
                "RequesterManaged".to_string(),
                serde_json::Value::Bool(requester_managed),
            );
        }

        // Add route table IDs
        if let Some(route_table_ids) = &endpoint.route_table_ids {
            let route_table_ids_json: Vec<serde_json::Value> = route_table_ids
                .iter()
                .map(|id| serde_json::Value::String(id.clone()))
                .collect();
            json.insert(
                "RouteTableIds".to_string(),
                serde_json::Value::Array(route_table_ids_json),
            );
        }

        // Add subnet IDs
        if let Some(subnet_ids) = &endpoint.subnet_ids {
            let subnet_ids_json: Vec<serde_json::Value> = subnet_ids
                .iter()
                .map(|id| serde_json::Value::String(id.clone()))
                .collect();
            json.insert(
                "SubnetIds".to_string(),
                serde_json::Value::Array(subnet_ids_json),
            );
        }

        // Add security group IDs
        if let Some(groups) = &endpoint.groups {
            let groups_json: Vec<serde_json::Value> = groups
                .iter()
                .map(|group| {
                    let mut group_json = serde_json::Map::new();

                    if let Some(group_id) = &group.group_id {
                        group_json.insert(
                            "GroupId".to_string(),
                            serde_json::Value::String(group_id.clone()),
                        );
                    }

                    if let Some(group_name) = &group.group_name {
                        group_json.insert(
                            "GroupName".to_string(),
                            serde_json::Value::String(group_name.clone()),
                        );
                    }

                    serde_json::Value::Object(group_json)
                })
                .collect();
            json.insert("Groups".to_string(), serde_json::Value::Array(groups_json));
        }

        // Add DNS entries
        if let Some(dns_entries) = &endpoint.dns_entries {
            let dns_entries_json: Vec<serde_json::Value> = dns_entries
                .iter()
                .map(|dns| {
                    let mut dns_json = serde_json::Map::new();

                    if let Some(dns_name) = &dns.dns_name {
                        dns_json.insert(
                            "DnsName".to_string(),
                            serde_json::Value::String(dns_name.clone()),
                        );
                    }

                    if let Some(hosted_zone_id) = &dns.hosted_zone_id {
                        dns_json.insert(
                            "HostedZoneId".to_string(),
                            serde_json::Value::String(hosted_zone_id.clone()),
                        );
                    }

                    serde_json::Value::Object(dns_json)
                })
                .collect();
            json.insert(
                "DnsEntries".to_string(),
                serde_json::Value::Array(dns_entries_json),
            );
        }

        // Add network interface IDs
        if let Some(network_interface_ids) = &endpoint.network_interface_ids {
            let network_interface_ids_json: Vec<serde_json::Value> = network_interface_ids
                .iter()
                .map(|id| serde_json::Value::String(id.clone()))
                .collect();
            json.insert(
                "NetworkInterfaceIds".to_string(),
                serde_json::Value::Array(network_interface_ids_json),
            );
        }

        // Extract tags and handle Name tag specially
        if let Some(tags) = &endpoint.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use service name and VPC endpoint ID
        if !json.contains_key("Name") {
            if let Some(service_name) = &endpoint.service_name {
                if let Some(vpc_endpoint_id) = &endpoint.vpc_endpoint_id {
                    let name = format!(
                        "{} ({})",
                        service_name.split('.').next_back().unwrap_or(service_name),
                        vpc_endpoint_id
                    );
                    json.insert("Name".to_string(), serde_json::Value::String(name));
                }
            } else if let Some(vpc_endpoint_id) = &endpoint.vpc_endpoint_id {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(vpc_endpoint_id.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert network ACL to JSON format
    fn network_acl_to_json(&self, acl: &ec2::types::NetworkAcl) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(network_acl_id) = &acl.network_acl_id {
            json.insert(
                "NetworkAclId".to_string(),
                serde_json::Value::String(network_acl_id.clone()),
            );
        }

        if let Some(vpc_id) = &acl.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(is_default) = acl.is_default {
            json.insert("IsDefault".to_string(), serde_json::Value::Bool(is_default));
        }

        if let Some(owner_id) = &acl.owner_id {
            json.insert(
                "OwnerId".to_string(),
                serde_json::Value::String(owner_id.clone()),
            );
        }

        // Add entries (rules)
        if let Some(entries) = &acl.entries {
            let entries_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|entry| {
                    let mut entry_json = serde_json::Map::new();

                    if let Some(rule_number) = entry.rule_number {
                        entry_json.insert(
                            "RuleNumber".to_string(),
                            serde_json::Value::Number(rule_number.into()),
                        );
                    }

                    if let Some(protocol) = &entry.protocol {
                        entry_json.insert(
                            "Protocol".to_string(),
                            serde_json::Value::String(protocol.clone()),
                        );
                    }

                    if let Some(rule_action) = &entry.rule_action {
                        entry_json.insert(
                            "RuleAction".to_string(),
                            serde_json::Value::String(rule_action.as_str().to_string()),
                        );
                    }

                    if let Some(cidr_block) = &entry.cidr_block {
                        entry_json.insert(
                            "CidrBlock".to_string(),
                            serde_json::Value::String(cidr_block.clone()),
                        );
                    }

                    if let Some(ipv6_cidr_block) = &entry.ipv6_cidr_block {
                        entry_json.insert(
                            "Ipv6CidrBlock".to_string(),
                            serde_json::Value::String(ipv6_cidr_block.clone()),
                        );
                    }

                    if let Some(port_range) = &entry.port_range {
                        let mut port_range_json = serde_json::Map::new();

                        if let Some(from) = port_range.from {
                            port_range_json
                                .insert("From".to_string(), serde_json::Value::Number(from.into()));
                        }

                        if let Some(to) = port_range.to {
                            port_range_json
                                .insert("To".to_string(), serde_json::Value::Number(to.into()));
                        }

                        entry_json.insert(
                            "PortRange".to_string(),
                            serde_json::Value::Object(port_range_json),
                        );
                    }

                    if let Some(icmp_type_code) = &entry.icmp_type_code {
                        let mut icmp_json = serde_json::Map::new();

                        if let Some(icmp_type) = icmp_type_code.r#type {
                            icmp_json.insert(
                                "Type".to_string(),
                                serde_json::Value::Number(icmp_type.into()),
                            );
                        }

                        if let Some(code) = icmp_type_code.code {
                            icmp_json
                                .insert("Code".to_string(), serde_json::Value::Number(code.into()));
                        }

                        entry_json.insert(
                            "IcmpTypeCode".to_string(),
                            serde_json::Value::Object(icmp_json),
                        );
                    }

                    serde_json::Value::Object(entry_json)
                })
                .collect();
            json.insert(
                "Entries".to_string(),
                serde_json::Value::Array(entries_json),
            );
        }

        // Add associations
        if let Some(associations) = &acl.associations {
            let associations_json: Vec<serde_json::Value> = associations
                .iter()
                .map(|association| {
                    let mut assoc_json = serde_json::Map::new();

                    if let Some(network_acl_association_id) =
                        &association.network_acl_association_id
                    {
                        assoc_json.insert(
                            "NetworkAclAssociationId".to_string(),
                            serde_json::Value::String(network_acl_association_id.clone()),
                        );
                    }

                    if let Some(network_acl_id) = &association.network_acl_id {
                        assoc_json.insert(
                            "NetworkAclId".to_string(),
                            serde_json::Value::String(network_acl_id.clone()),
                        );
                    }

                    if let Some(subnet_id) = &association.subnet_id {
                        assoc_json.insert(
                            "SubnetId".to_string(),
                            serde_json::Value::String(subnet_id.clone()),
                        );
                    }

                    serde_json::Value::Object(assoc_json)
                })
                .collect();
            json.insert(
                "Associations".to_string(),
                serde_json::Value::Array(associations_json),
            );
        }

        // Extract tags and handle Name tag specially
        if let Some(tags) = &acl.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        // If no name found, use network ACL ID with default indicator
        if !json.contains_key("Name") {
            if let Some(network_acl_id) = &acl.network_acl_id {
                let is_default = acl.is_default.unwrap_or(false);
                let name = if is_default {
                    format!("{} (default)", network_acl_id)
                } else {
                    network_acl_id.clone()
                };
                json.insert("Name".to_string(), serde_json::Value::String(name));
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert key pair to JSON format
    fn key_pair_to_json(&self, kp: &ec2::types::KeyPairInfo) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(key_name) = &kp.key_name {
            json.insert(
                "KeyName".to_string(),
                serde_json::Value::String(key_name.clone()),
            );
            // Use key name as the display name
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(key_name.clone()),
            );
        }

        if let Some(key_fingerprint) = &kp.key_fingerprint {
            json.insert(
                "KeyFingerprint".to_string(),
                serde_json::Value::String(key_fingerprint.clone()),
            );
        }

        if let Some(key_type) = &kp.key_type {
            json.insert(
                "KeyType".to_string(),
                serde_json::Value::String(key_type.as_str().to_string()),
            );
        }

        if let Some(key_pair_id) = &kp.key_pair_id {
            json.insert(
                "KeyPairId".to_string(),
                serde_json::Value::String(key_pair_id.clone()),
            );
        }

        if let Some(create_time) = kp.create_time {
            json.insert(
                "CreateTime".to_string(),
                serde_json::Value::String(create_time.to_string()),
            );
        }

        if let Some(public_key) = &kp.public_key {
            json.insert(
                "PublicKey".to_string(),
                serde_json::Value::String(public_key.clone()),
            );
        }

        // Extract tags and handle Name tag specially (overriding key name if present)
        if let Some(tags) = &kp.tags {
            for tag in tags {
                if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                    if key == "Name" {
                        json.insert("Name".to_string(), serde_json::Value::String(value.clone()));
                        break;
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }
}
