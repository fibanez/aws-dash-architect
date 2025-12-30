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

        // InstanceTenancy (e.g., "default", "dedicated", "host")
        if let Some(instance_tenancy) = &vpc.instance_tenancy {
            json.insert(
                "InstanceTenancy".to_string(),
                serde_json::Value::String(instance_tenancy.as_str().to_string()),
            );
        }

        // DhcpOptionsId
        if let Some(dhcp_options_id) = &vpc.dhcp_options_id {
            json.insert(
                "DhcpOptionsId".to_string(),
                serde_json::Value::String(dhcp_options_id.clone()),
            );
        }

        // OwnerId (AWS account ID)
        if let Some(owner_id) = &vpc.owner_id {
            json.insert(
                "OwnerId".to_string(),
                serde_json::Value::String(owner_id.clone()),
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

    /// List Transit Gateways
    pub async fn list_transit_gateways(
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
        let mut paginator = client.describe_transit_gateways().into_paginator().send();

        let mut transit_gateways = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(tgw_list) = page.transit_gateways {
                for tgw in tgw_list {
                    let tgw_json = self.transit_gateway_to_json(&tgw);
                    transit_gateways.push(tgw_json);
                }
            }
        }

        Ok(transit_gateways)
    }

    /// List VPC Peering Connections
    pub async fn list_vpc_peering_connections(
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
        let mut paginator = client
            .describe_vpc_peering_connections()
            .into_paginator()
            .send();

        let mut peering_connections = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(pc_list) = page.vpc_peering_connections {
                for pc in pc_list {
                    let pc_json = self.vpc_peering_connection_to_json(&pc);
                    peering_connections.push(pc_json);
                }
            }
        }

        Ok(peering_connections)
    }

    /// List VPC Flow Logs
    pub async fn list_flow_logs(
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
        let mut paginator = client.describe_flow_logs().into_paginator().send();

        let mut flow_logs = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(fl_list) = page.flow_logs {
                for fl in fl_list {
                    let fl_json = self.flow_log_to_json(&fl);
                    flow_logs.push(fl_json);
                }
            }
        }

        Ok(flow_logs)
    }

    /// List EBS Volume Attachments
    pub async fn list_volume_attachments(
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

        // Get all volumes to extract attachment information
        let response = client.describe_volumes().send().await?;

        let mut attachments = Vec::new();
        if let Some(volumes) = response.volumes {
            for volume in volumes {
                if let Some(volume_attachments) = &volume.attachments {
                    for attachment in volume_attachments {
                        let attachment_json = self.volume_attachment_to_json(&volume, attachment);
                        attachments.push(attachment_json);
                    }
                }
            }
        }

        Ok(attachments)
    }

    /// List Elastic IP addresses
    pub async fn list_elastic_ips(
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
        let response = client.describe_addresses().send().await?;

        let mut addresses = Vec::new();
        if let Some(address_list) = response.addresses {
            for address in address_list {
                let address_json = self.elastic_ip_to_json(&address);
                addresses.push(address_json);
            }
        }

        Ok(addresses)
    }

    /// List EC2 Launch Templates
    pub async fn list_launch_templates(
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
        let mut paginator = client
            .describe_launch_templates()
            .into_paginator()
            .send();

        let mut templates = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(template_list) = page.launch_templates {
                for template in template_list {
                    let template_json = self.launch_template_to_json(&template);
                    templates.push(template_json);
                }
            }
        }

        Ok(templates)
    }

    /// List EC2 Placement Groups
    pub async fn list_placement_groups(
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
        let response = client.describe_placement_groups().send().await?;

        let mut groups = Vec::new();
        if let Some(group_list) = response.placement_groups {
            for group in group_list {
                let group_json = self.placement_group_to_json(&group);
                groups.push(group_json);
            }
        }

        Ok(groups)
    }

    /// List EC2 Reserved Instances
    pub async fn list_reserved_instances(
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
        let response = client.describe_reserved_instances().send().await?;

        let mut reserved_instances = Vec::new();
        if let Some(instance_list) = response.reserved_instances {
            for instance in instance_list {
                let instance_json = self.reserved_instance_to_json(&instance);
                reserved_instances.push(instance_json);
            }
        }

        Ok(reserved_instances)
    }

    /// List EC2 Spot Instance Requests
    pub async fn list_spot_instance_requests(
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
        let mut paginator = client
            .describe_spot_instance_requests()
            .into_paginator()
            .send();

        let mut requests = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(request_list) = page.spot_instance_requests {
                for request in request_list {
                    let request_json = self.spot_instance_request_to_json(&request);
                    requests.push(request_json);
                }
            }
        }

        Ok(requests)
    }

    /// List DHCP Options Sets
    pub async fn list_dhcp_options(
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
        let mut paginator = client
            .describe_dhcp_options()
            .into_paginator()
            .send();

        let mut options_sets = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(options_list) = page.dhcp_options {
                for options in options_list {
                    let options_json = self.dhcp_options_to_json(&options);
                    options_sets.push(options_json);
                }
            }
        }

        Ok(options_sets)
    }

    /// List Egress-Only Internet Gateways
    pub async fn list_egress_only_internet_gateways(
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
        let mut paginator = client
            .describe_egress_only_internet_gateways()
            .into_paginator()
            .send();

        let mut gateways = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(gateway_list) = page.egress_only_internet_gateways {
                for gateway in gateway_list {
                    let gateway_json = self.egress_only_internet_gateway_to_json(&gateway);
                    gateways.push(gateway_json);
                }
            }
        }

        Ok(gateways)
    }

    /// List VPN Connections
    pub async fn list_vpn_connections(
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
        let response = client.describe_vpn_connections().send().await?;

        let mut connections = Vec::new();
        if let Some(connection_list) = response.vpn_connections {
            for connection in connection_list {
                let connection_json = self.vpn_connection_to_json(&connection);
                connections.push(connection_json);
            }
        }

        Ok(connections)
    }

    /// List VPN Gateways
    pub async fn list_vpn_gateways(
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
        let response = client.describe_vpn_gateways().send().await?;

        let mut gateways = Vec::new();
        if let Some(gateway_list) = response.vpn_gateways {
            for gateway in gateway_list {
                let gateway_json = self.vpn_gateway_to_json(&gateway);
                gateways.push(gateway_json);
            }
        }

        Ok(gateways)
    }

    /// List Customer Gateways
    pub async fn list_customer_gateways(
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
        let response = client.describe_customer_gateways().send().await?;

        let mut gateways = Vec::new();
        if let Some(gateway_list) = response.customer_gateways {
            for gateway in gateway_list {
                let gateway_json = self.customer_gateway_to_json(&gateway);
                gateways.push(gateway_json);
            }
        }

        Ok(gateways)
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

    /// Convert Transit Gateway to JSON format
    fn transit_gateway_to_json(&self, tgw: &ec2::types::TransitGateway) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(tgw_id) = &tgw.transit_gateway_id {
            json.insert(
                "TransitGatewayId".to_string(),
                serde_json::Value::String(tgw_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(tgw_id.clone()),
            );
        }

        if let Some(state) = &tgw.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(owner_id) = &tgw.owner_id {
            json.insert(
                "OwnerId".to_string(),
                serde_json::Value::String(owner_id.clone()),
            );
        }

        if let Some(creation_time) = tgw.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(description) = &tgw.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        // Handle tags and extract Name if available
        if let Some(tags) = &tgw.tags {
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

                // Extract Name tag for display
                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert VPC Peering Connection to JSON format
    fn vpc_peering_connection_to_json(
        &self,
        pc: &ec2::types::VpcPeeringConnection,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(pc_id) = &pc.vpc_peering_connection_id {
            json.insert(
                "VpcPeeringConnectionId".to_string(),
                serde_json::Value::String(pc_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(pc_id.clone()),
            );
        }

        if let Some(status) = &pc.status {
            if let Some(code) = &status.code {
                json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(code.as_str().to_string()),
                );
            }
            if let Some(message) = &status.message {
                json.insert(
                    "StatusMessage".to_string(),
                    serde_json::Value::String(message.clone()),
                );
            }
        }

        // Accepter VPC Info
        if let Some(accepter_vpc_info) = &pc.accepter_vpc_info {
            if let Some(vpc_id) = &accepter_vpc_info.vpc_id {
                json.insert(
                    "AccepterVpcId".to_string(),
                    serde_json::Value::String(vpc_id.clone()),
                );
            }
            if let Some(owner_id) = &accepter_vpc_info.owner_id {
                json.insert(
                    "AccepterOwnerId".to_string(),
                    serde_json::Value::String(owner_id.clone()),
                );
            }
        }

        // Requester VPC Info
        if let Some(requester_vpc_info) = &pc.requester_vpc_info {
            if let Some(vpc_id) = &requester_vpc_info.vpc_id {
                json.insert(
                    "RequesterVpcId".to_string(),
                    serde_json::Value::String(vpc_id.clone()),
                );
            }
            if let Some(owner_id) = &requester_vpc_info.owner_id {
                json.insert(
                    "RequesterOwnerId".to_string(),
                    serde_json::Value::String(owner_id.clone()),
                );
            }
        }

        // Handle tags
        if let Some(tags) = &pc.tags {
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

                // Extract Name tag for display
                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert VPC Flow Log to JSON format
    fn flow_log_to_json(&self, fl: &ec2::types::FlowLog) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(flow_log_id) = &fl.flow_log_id {
            json.insert(
                "FlowLogId".to_string(),
                serde_json::Value::String(flow_log_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(flow_log_id.clone()),
            );
        }

        if let Some(resource_id) = &fl.resource_id {
            json.insert(
                "AttachedResourceId".to_string(),
                serde_json::Value::String(resource_id.clone()),
            );
        }

        // Note: FlowLog resource_type is inferred from resource_id format
        // VPC flow logs have vpc-xxx format, subnet have subnet-xxx, etc.
        if let Some(resource_id) = &fl.resource_id {
            if resource_id.starts_with("vpc-") {
                json.insert(
                    "ResourceType".to_string(),
                    serde_json::Value::String("VPC".to_string()),
                );
            } else if resource_id.starts_with("subnet-") {
                json.insert(
                    "ResourceType".to_string(),
                    serde_json::Value::String("Subnet".to_string()),
                );
            } else if resource_id.starts_with("eni-") {
                json.insert(
                    "ResourceType".to_string(),
                    serde_json::Value::String("NetworkInterface".to_string()),
                );
            } else {
                json.insert(
                    "ResourceType".to_string(),
                    serde_json::Value::String("Unknown".to_string()),
                );
            }
        }

        if let Some(flow_log_status) = &fl.flow_log_status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(flow_log_status.as_str().to_string()),
            );
        }

        if let Some(traffic_type) = &fl.traffic_type {
            json.insert(
                "TrafficType".to_string(),
                serde_json::Value::String(traffic_type.as_str().to_string()),
            );
        }

        if let Some(log_destination_type) = &fl.log_destination_type {
            json.insert(
                "LogDestinationType".to_string(),
                serde_json::Value::String(log_destination_type.as_str().to_string()),
            );
        }

        if let Some(log_destination) = &fl.log_destination {
            json.insert(
                "LogDestination".to_string(),
                serde_json::Value::String(log_destination.clone()),
            );
        }

        if let Some(creation_time) = fl.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        // Handle tags
        if let Some(tags) = &fl.tags {
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

                // Extract Name tag for display
                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert EBS Volume Attachment to JSON format
    fn volume_attachment_to_json(
        &self,
        volume: &ec2::types::Volume,
        attachment: &ec2::types::VolumeAttachment,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        // Create a unique ID for the attachment combining volume and instance IDs
        let volume_id = volume.volume_id.as_deref().unwrap_or("unknown-volume");
        let instance_id = attachment
            .instance_id
            .as_deref()
            .unwrap_or("unknown-instance");
        let attachment_id = format!("{}:{}", volume_id, instance_id);

        json.insert(
            "AttachmentId".to_string(),
            serde_json::Value::String(attachment_id.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(attachment_id),
        );

        if let Some(volume_id) = &volume.volume_id {
            json.insert(
                "VolumeId".to_string(),
                serde_json::Value::String(volume_id.clone()),
            );
        }

        if let Some(instance_id) = &attachment.instance_id {
            json.insert(
                "InstanceId".to_string(),
                serde_json::Value::String(instance_id.clone()),
            );
        }

        if let Some(device) = &attachment.device {
            json.insert(
                "Device".to_string(),
                serde_json::Value::String(device.clone()),
            );
        }

        if let Some(state) = &attachment.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(attach_time) = attachment.attach_time {
            json.insert(
                "AttachTime".to_string(),
                serde_json::Value::String(attach_time.to_string()),
            );
        }

        json.insert(
            "DeleteOnTermination".to_string(),
            serde_json::Value::Bool(attachment.delete_on_termination.unwrap_or(false)),
        );

        // Include volume information for context
        if let Some(size) = volume.size {
            json.insert(
                "VolumeSize".to_string(),
                serde_json::Value::Number(size.into()),
            );
        }

        if let Some(volume_type) = &volume.volume_type {
            json.insert(
                "VolumeType".to_string(),
                serde_json::Value::String(volume_type.as_str().to_string()),
            );
        }

        if let Some(availability_zone) = &volume.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        // Generate a display name
        let display_name = format!("{} → {}", volume_id, instance_id);
        json.insert("Name".to_string(), serde_json::Value::String(display_name));

        serde_json::Value::Object(json)
    }

    /// Convert Elastic IP to JSON format
    fn elastic_ip_to_json(&self, address: &ec2::types::Address) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(allocation_id) = &address.allocation_id {
            json.insert(
                "AllocationId".to_string(),
                serde_json::Value::String(allocation_id.clone()),
            );
        }

        if let Some(public_ip) = &address.public_ip {
            json.insert(
                "PublicIp".to_string(),
                serde_json::Value::String(public_ip.clone()),
            );
        }

        if let Some(association_id) = &address.association_id {
            json.insert(
                "AssociationId".to_string(),
                serde_json::Value::String(association_id.clone()),
            );
        }

        if let Some(domain) = &address.domain {
            json.insert(
                "Domain".to_string(),
                serde_json::Value::String(domain.as_str().to_string()),
            );
        }

        if let Some(instance_id) = &address.instance_id {
            json.insert(
                "InstanceId".to_string(),
                serde_json::Value::String(instance_id.clone()),
            );
        }

        if let Some(network_interface_id) = &address.network_interface_id {
            json.insert(
                "NetworkInterfaceId".to_string(),
                serde_json::Value::String(network_interface_id.clone()),
            );
        }

        if let Some(network_interface_owner_id) = &address.network_interface_owner_id {
            json.insert(
                "NetworkInterfaceOwnerId".to_string(),
                serde_json::Value::String(network_interface_owner_id.clone()),
            );
        }

        if let Some(private_ip) = &address.private_ip_address {
            json.insert(
                "PrivateIpAddress".to_string(),
                serde_json::Value::String(private_ip.clone()),
            );
        }

        if let Some(public_ipv4_pool) = &address.public_ipv4_pool {
            json.insert(
                "PublicIpv4Pool".to_string(),
                serde_json::Value::String(public_ipv4_pool.clone()),
            );
        }

        if let Some(border_group) = &address.network_border_group {
            json.insert(
                "NetworkBorderGroup".to_string(),
                serde_json::Value::String(border_group.clone()),
            );
        }

        if let Some(tags) = &address.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert Launch Template to JSON format
    fn launch_template_to_json(
        &self,
        template: &ec2::types::LaunchTemplate,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(template_id) = &template.launch_template_id {
            json.insert(
                "LaunchTemplateId".to_string(),
                serde_json::Value::String(template_id.clone()),
            );
        }

        if let Some(template_name) = &template.launch_template_name {
            json.insert(
                "LaunchTemplateName".to_string(),
                serde_json::Value::String(template_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(template_name.clone()),
            );
        }

        if let Some(created_by) = &template.created_by {
            json.insert(
                "CreatedBy".to_string(),
                serde_json::Value::String(created_by.clone()),
            );
        }

        if let Some(create_time) = template.create_time {
            json.insert(
                "CreateTime".to_string(),
                serde_json::Value::String(create_time.to_string()),
            );
        }

        if let Some(default_version) = template.default_version_number {
            json.insert(
                "DefaultVersionNumber".to_string(),
                serde_json::Value::Number(default_version.into()),
            );
        }

        if let Some(latest_version) = template.latest_version_number {
            json.insert(
                "LatestVersionNumber".to_string(),
                serde_json::Value::Number(latest_version.into()),
            );
        }

        if let Some(tags) = &template.tags {
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

                if !json.contains_key("Name") {
                    for tag in tags {
                        if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                            if key == "Name" {
                                json.insert(
                                    "Name".to_string(),
                                    serde_json::Value::String(value.clone()),
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert Placement Group to JSON format
    fn placement_group_to_json(
        &self,
        group: &ec2::types::PlacementGroup,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(group_id) = &group.group_id {
            json.insert(
                "GroupId".to_string(),
                serde_json::Value::String(group_id.clone()),
            );
        }

        if let Some(group_name) = &group.group_name {
            json.insert(
                "GroupName".to_string(),
                serde_json::Value::String(group_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(group_name.clone()),
            );
        }

        if let Some(strategy) = &group.strategy {
            json.insert(
                "Strategy".to_string(),
                serde_json::Value::String(strategy.as_str().to_string()),
            );
        }

        if let Some(state) = &group.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(partition_count) = group.partition_count {
            json.insert(
                "PartitionCount".to_string(),
                serde_json::Value::Number(partition_count.into()),
            );
        }

        if let Some(spread_level) = &group.spread_level {
            json.insert(
                "SpreadLevel".to_string(),
                serde_json::Value::String(spread_level.as_str().to_string()),
            );
        }

        if let Some(tags) = &group.tags {
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

                if !json.contains_key("Name") {
                    for tag in tags {
                        if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                            if key == "Name" {
                                json.insert(
                                    "Name".to_string(),
                                    serde_json::Value::String(value.clone()),
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert Reserved Instance to JSON format
    fn reserved_instance_to_json(
        &self,
        instance: &ec2::types::ReservedInstances,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(reserved_id) = &instance.reserved_instances_id {
            json.insert(
                "ReservedInstancesId".to_string(),
                serde_json::Value::String(reserved_id.clone()),
            );
        }

        if let Some(instance_type) = &instance.instance_type {
            json.insert(
                "InstanceType".to_string(),
                serde_json::Value::String(instance_type.as_str().to_string()),
            );
        }

        if let Some(state) = &instance.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(availability_zone) = &instance.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(start) = instance.start {
            json.insert(
                "Start".to_string(),
                serde_json::Value::String(start.to_string()),
            );
        }

        if let Some(end) = instance.end {
            json.insert(
                "End".to_string(),
                serde_json::Value::String(end.to_string()),
            );
        }

        if let Some(instance_count) = instance.instance_count {
            json.insert(
                "InstanceCount".to_string(),
                serde_json::Value::Number(instance_count.into()),
            );
        }

        if let Some(offering_type) = &instance.offering_type {
            json.insert(
                "OfferingType".to_string(),
                serde_json::Value::String(offering_type.as_str().to_string()),
            );
        }

        if let Some(offering_class) = &instance.offering_class {
            json.insert(
                "OfferingClass".to_string(),
                serde_json::Value::String(offering_class.as_str().to_string()),
            );
        }

        if let Some(product_description) = &instance.product_description {
            json.insert(
                "ProductDescription".to_string(),
                serde_json::Value::String(product_description.as_str().to_string()),
            );
        }

        if let Some(tags) = &instance.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert Spot Instance Request to JSON format
    fn spot_instance_request_to_json(
        &self,
        request: &ec2::types::SpotInstanceRequest,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(request_id) = &request.spot_instance_request_id {
            json.insert(
                "SpotInstanceRequestId".to_string(),
                serde_json::Value::String(request_id.clone()),
            );
        }

        if let Some(state) = &request.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(spot_price) = &request.spot_price {
            json.insert(
                "SpotPrice".to_string(),
                serde_json::Value::String(spot_price.clone()),
            );
        }

        if let Some(instance_id) = &request.instance_id {
            json.insert(
                "InstanceId".to_string(),
                serde_json::Value::String(instance_id.clone()),
            );
        }

        if let Some(create_time) = request.create_time {
            json.insert(
                "CreateTime".to_string(),
                serde_json::Value::String(create_time.to_string()),
            );
        }

        if let Some(status) = &request.status {
            if let Some(code) = &status.code {
                json.insert(
                    "StatusCode".to_string(),
                    serde_json::Value::String(code.clone()),
                );
            }
            if let Some(message) = &status.message {
                json.insert(
                    "StatusMessage".to_string(),
                    serde_json::Value::String(message.clone()),
                );
            }
        }

        if let Some(tags) = &request.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert DHCP Options Set to JSON format
    fn dhcp_options_to_json(&self, options: &ec2::types::DhcpOptions) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(options_id) = &options.dhcp_options_id {
            json.insert(
                "DhcpOptionsId".to_string(),
                serde_json::Value::String(options_id.clone()),
            );
        }

        if let Some(configurations) = &options.dhcp_configurations {
            let configs_json: Vec<serde_json::Value> = configurations
                .iter()
                .map(|config| {
                    let mut config_json = serde_json::Map::new();
                    if let Some(key) = &config.key {
                        config_json
                            .insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(values) = &config.values {
                        let values_json: Vec<serde_json::Value> = values
                            .iter()
                            .filter_map(|value| {
                                value.value.as_ref().map(|v| {
                                    serde_json::Value::String(v.clone())
                                })
                            })
                            .collect();
                        config_json.insert(
                            "Values".to_string(),
                            serde_json::Value::Array(values_json),
                        );
                    }
                    serde_json::Value::Object(config_json)
                })
                .collect();
            json.insert(
                "DhcpConfigurations".to_string(),
                serde_json::Value::Array(configs_json),
            );
        }

        if let Some(tags) = &options.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert Egress-Only Internet Gateway to JSON format
    fn egress_only_internet_gateway_to_json(
        &self,
        gateway: &ec2::types::EgressOnlyInternetGateway,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(gateway_id) = &gateway.egress_only_internet_gateway_id {
            json.insert(
                "EgressOnlyInternetGatewayId".to_string(),
                serde_json::Value::String(gateway_id.clone()),
            );
        }

        if let Some(attachments) = &gateway.attachments {
            let attachments_json: Vec<serde_json::Value> = attachments
                .iter()
                .map(|attachment| {
                    let mut attachment_json = serde_json::Map::new();
                    if let Some(state) = &attachment.state {
                        attachment_json.insert(
                            "State".to_string(),
                            serde_json::Value::String(state.as_str().to_string()),
                        );
                    }
                    if let Some(vpc_id) = &attachment.vpc_id {
                        attachment_json.insert(
                            "VpcId".to_string(),
                            serde_json::Value::String(vpc_id.clone()),
                        );
                    }
                    serde_json::Value::Object(attachment_json)
                })
                .collect();
            json.insert(
                "Attachments".to_string(),
                serde_json::Value::Array(attachments_json),
            );
        }

        if let Some(tags) = &gateway.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert VPN Connection to JSON format
    fn vpn_connection_to_json(
        &self,
        connection: &ec2::types::VpnConnection,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(connection_id) = &connection.vpn_connection_id {
            json.insert(
                "VpnConnectionId".to_string(),
                serde_json::Value::String(connection_id.clone()),
            );
        }

        if let Some(state) = &connection.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(connection_type) = &connection.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(connection_type.as_str().to_string()),
            );
        }

        if let Some(customer_gateway_id) = &connection.customer_gateway_id {
            json.insert(
                "CustomerGatewayId".to_string(),
                serde_json::Value::String(customer_gateway_id.clone()),
            );
        }

        if let Some(vpn_gateway_id) = &connection.vpn_gateway_id {
            json.insert(
                "VpnGatewayId".to_string(),
                serde_json::Value::String(vpn_gateway_id.clone()),
            );
        }

        if let Some(transit_gateway_id) = &connection.transit_gateway_id {
            json.insert(
                "TransitGatewayId".to_string(),
                serde_json::Value::String(transit_gateway_id.clone()),
            );
        }

        if let Some(tags) = &connection.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert VPN Gateway to JSON format
    fn vpn_gateway_to_json(&self, gateway: &ec2::types::VpnGateway) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(gateway_id) = &gateway.vpn_gateway_id {
            json.insert(
                "VpnGatewayId".to_string(),
                serde_json::Value::String(gateway_id.clone()),
            );
        }

        if let Some(state) = &gateway.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(gateway_type) = &gateway.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(gateway_type.as_str().to_string()),
            );
        }

        if let Some(amazon_side_asn) = gateway.amazon_side_asn {
            json.insert(
                "AmazonSideAsn".to_string(),
                serde_json::Value::Number(amazon_side_asn.into()),
            );
        }

        if let Some(tags) = &gateway.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }

    /// Convert Customer Gateway to JSON format
    fn customer_gateway_to_json(
        &self,
        gateway: &ec2::types::CustomerGateway,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(gateway_id) = &gateway.customer_gateway_id {
            json.insert(
                "CustomerGatewayId".to_string(),
                serde_json::Value::String(gateway_id.clone()),
            );
        }

        if let Some(state) = &gateway.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(gateway_type) = &gateway.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(gateway_type.as_str().to_string()),
            );
        }

        if let Some(bgp_asn) = &gateway.bgp_asn {
            json.insert(
                "BgpAsn".to_string(),
                serde_json::Value::String(bgp_asn.clone()),
            );
        }

        if let Some(ip_address) = &gateway.ip_address {
            json.insert(
                "IpAddress".to_string(),
                serde_json::Value::String(ip_address.clone()),
            );
        }

        if let Some(tags) = &gateway.tags {
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

                for tag in tags {
                    if let (Some(key), Some(value)) = (&tag.key, &tag.value) {
                        if key == "Name" {
                            json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                            break;
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(json)
    }
}
