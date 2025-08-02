use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_elasticloadbalancing as elb;
use std::sync::Arc;

pub struct ELBService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ELBService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Classic Load Balancers (ELB)
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

        let client = elb::Client::new(&aws_config);
        let response = client.describe_load_balancers().send().await?;

        let mut load_balancers = Vec::new();
        if let Some(lb_descriptions) = response.load_balancer_descriptions {
            for lb in lb_descriptions {
                let lb_json = self.load_balancer_to_json(&lb);
                load_balancers.push(lb_json);
            }
        }

        Ok(load_balancers)
    }

    /// Describe specific Classic Load Balancer
    pub async fn describe_load_balancer(
        &self,
        account_id: &str,
        region: &str,
        load_balancer_name: &str,
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

        let client = elb::Client::new(&aws_config);
        let response = client
            .describe_load_balancers()
            .load_balancer_names(load_balancer_name)
            .send()
            .await?;

        if let Some(lb_descriptions) = response.load_balancer_descriptions {
            if let Some(lb) = lb_descriptions.into_iter().next() {
                return Ok(self.load_balancer_to_json(&lb));
            }
        }

        Err(anyhow::anyhow!(
            "Classic Load Balancer not found: {}",
            load_balancer_name
        ))
    }

    /// Convert Classic Load Balancer to JSON
    fn load_balancer_to_json(&self, lb: &elb::types::LoadBalancerDescription) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        // Core fields
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
                serde_json::Value::String(scheme.clone()),
            );
        }

        if let Some(vpc_id) = &lb.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        // Subnets
        if let Some(subnets) = &lb.subnets {
            if !subnets.is_empty() {
                let subnet_values: Vec<serde_json::Value> = subnets
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect();
                json.insert(
                    "Subnets".to_string(),
                    serde_json::Value::Array(subnet_values),
                );
            }
        }

        // Availability Zones
        if let Some(azs) = &lb.availability_zones {
            if !azs.is_empty() {
                let az_values: Vec<serde_json::Value> = azs
                    .iter()
                    .map(|az| serde_json::Value::String(az.clone()))
                    .collect();
                json.insert(
                    "AvailabilityZones".to_string(),
                    serde_json::Value::Array(az_values),
                );
            }
        }

        // Security Groups
        if let Some(security_groups) = &lb.security_groups {
            if !security_groups.is_empty() {
                let sg_values: Vec<serde_json::Value> = security_groups
                    .iter()
                    .map(|sg| serde_json::Value::String(sg.clone()))
                    .collect();
                json.insert(
                    "SecurityGroups".to_string(),
                    serde_json::Value::Array(sg_values),
                );
            }
        }

        // Health Check
        if let Some(health_check) = &lb.health_check {
            let mut hc_json = serde_json::Map::new();
            hc_json.insert(
                "Target".to_string(),
                serde_json::Value::String(health_check.target.clone()),
            );
            hc_json.insert(
                "Interval".to_string(),
                serde_json::Value::Number(serde_json::Number::from(health_check.interval)),
            );
            hc_json.insert(
                "Timeout".to_string(),
                serde_json::Value::Number(serde_json::Number::from(health_check.timeout)),
            );
            hc_json.insert(
                "HealthyThreshold".to_string(),
                serde_json::Value::Number(serde_json::Number::from(health_check.healthy_threshold)),
            );
            hc_json.insert(
                "UnhealthyThreshold".to_string(),
                serde_json::Value::Number(serde_json::Number::from(
                    health_check.unhealthy_threshold,
                )),
            );
            json.insert(
                "HealthCheck".to_string(),
                serde_json::Value::Object(hc_json),
            );
        }

        // Instances
        if let Some(instances) = &lb.instances {
            if !instances.is_empty() {
                let instance_values: Vec<serde_json::Value> = instances
                    .iter()
                    .filter_map(|instance| {
                        instance
                            .instance_id
                            .as_ref()
                            .map(|id| serde_json::Value::String(id.clone()))
                    })
                    .collect();
                json.insert(
                    "Instances".to_string(),
                    serde_json::Value::Array(instance_values),
                );
            }
        }

        // Listeners
        if let Some(listener_descriptions) = &lb.listener_descriptions {
            if !listener_descriptions.is_empty() {
                let mut listeners = Vec::new();
                for listener_desc in listener_descriptions {
                    if let Some(listener) = &listener_desc.listener {
                        let mut listener_json = serde_json::Map::new();

                        listener_json.insert(
                            "Protocol".to_string(),
                            serde_json::Value::String(listener.protocol.clone()),
                        );
                        listener_json.insert(
                            "LoadBalancerPort".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(
                                listener.load_balancer_port,
                            )),
                        );

                        if let Some(instance_protocol) = &listener.instance_protocol {
                            listener_json.insert(
                                "InstanceProtocol".to_string(),
                                serde_json::Value::String(instance_protocol.clone()),
                            );
                        }
                        listener_json.insert(
                            "InstancePort".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(
                                listener.instance_port,
                            )),
                        );

                        if let Some(ssl_certificate_id) = &listener.ssl_certificate_id {
                            listener_json.insert(
                                "SSLCertificateId".to_string(),
                                serde_json::Value::String(ssl_certificate_id.clone()),
                            );
                        }

                        listeners.push(serde_json::Value::Object(listener_json));
                    }
                }
                json.insert("Listeners".to_string(), serde_json::Value::Array(listeners));
            }
        }

        // Policies
        if let Some(policies) = &lb.policies {
            let mut policies_json = serde_json::Map::new();

            // App Cookie Stickiness Policies
            if let Some(app_policies_list) = &policies.app_cookie_stickiness_policies {
                if !app_policies_list.is_empty() {
                    let app_policies: Vec<serde_json::Value> = app_policies_list
                        .iter()
                        .map(|policy| {
                            let mut policy_json = serde_json::Map::new();
                            if let Some(name) = &policy.policy_name {
                                policy_json.insert(
                                    "PolicyName".to_string(),
                                    serde_json::Value::String(name.clone()),
                                );
                            }
                            if let Some(cookie_name) = &policy.cookie_name {
                                policy_json.insert(
                                    "CookieName".to_string(),
                                    serde_json::Value::String(cookie_name.clone()),
                                );
                            }
                            serde_json::Value::Object(policy_json)
                        })
                        .collect();
                    policies_json.insert(
                        "AppCookieStickinessPolicies".to_string(),
                        serde_json::Value::Array(app_policies),
                    );
                }
            }

            // LB Cookie Stickiness Policies
            if let Some(lb_policies_list) = &policies.lb_cookie_stickiness_policies {
                if !lb_policies_list.is_empty() {
                    let lb_policies: Vec<serde_json::Value> = lb_policies_list
                        .iter()
                        .map(|policy| {
                            let mut policy_json = serde_json::Map::new();
                            if let Some(name) = &policy.policy_name {
                                policy_json.insert(
                                    "PolicyName".to_string(),
                                    serde_json::Value::String(name.clone()),
                                );
                            }
                            if let Some(expiration_period) = policy.cookie_expiration_period {
                                policy_json.insert(
                                    "CookieExpirationPeriod".to_string(),
                                    serde_json::Value::Number(serde_json::Number::from(
                                        expiration_period,
                                    )),
                                );
                            }
                            serde_json::Value::Object(policy_json)
                        })
                        .collect();
                    policies_json.insert(
                        "LBCookieStickinessPolicies".to_string(),
                        serde_json::Value::Array(lb_policies),
                    );
                }
            }

            if !policies_json.is_empty() {
                json.insert(
                    "Policies".to_string(),
                    serde_json::Value::Object(policies_json),
                );
            }
        }

        // Created time
        if let Some(created_time) = &lb.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        // Canonical hosted zone name
        if let Some(zone_name) = &lb.canonical_hosted_zone_name {
            json.insert(
                "CanonicalHostedZoneName".to_string(),
                serde_json::Value::String(zone_name.clone()),
            );
        }

        if let Some(zone_id) = &lb.canonical_hosted_zone_name_id {
            json.insert(
                "CanonicalHostedZoneNameID".to_string(),
                serde_json::Value::String(zone_id.clone()),
            );
        }

        serde_json::Value::Object(json)
    }
}
