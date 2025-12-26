use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_ecs as ecs;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct ECSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ECSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List ECS clusters with optional detailed information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic cluster info quickly.
    ///   If true (Phase 2), includes settings, capacity providers, and container instances.
    pub async fn list_clusters(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("ECS", "list_clusters", Some(region));

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

        let client = ecs::Client::new(&aws_config);
        let mut paginator = client.list_clusters().into_paginator().send();

        let mut clusters = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(cluster_arns) = page.cluster_arns {
                if !cluster_arns.is_empty() {
                    // Describe clusters to get detailed information
                    if let Ok(detailed_clusters) = self
                        .describe_clusters_internal(&client, &cluster_arns, include_details)
                        .await
                    {
                        clusters.extend(detailed_clusters);
                    } else {
                        // Fallback to basic cluster info if describe fails
                        for cluster_arn in cluster_arns {
                            let mut basic_cluster = serde_json::Map::new();
                            basic_cluster.insert(
                                "ClusterArn".to_string(),
                                serde_json::Value::String(cluster_arn.clone()),
                            );
                            // Extract cluster name from ARN
                            let cluster_name =
                                cluster_arn.split('/').next_back().unwrap_or(&cluster_arn);
                            basic_cluster.insert(
                                "Name".to_string(),
                                serde_json::Value::String(cluster_name.to_string()),
                            );
                            clusters.push(serde_json::Value::Object(basic_cluster));
                        }
                    }
                }
            }
        }

        report_status_done("ECS", "list_clusters", Some(region));
        Ok(clusters)
    }

    /// Get detailed information for a single ECS cluster (Phase 2 enrichment)
    pub async fn get_cluster_details(
        &self,
        account_id: &str,
        region: &str,
        cluster_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "get_cluster_details", Some(cluster_name));

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

        let client = ecs::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Get container instances for this cluster
        report_status("ECS", "list_container_instances", Some(cluster_name));
        match self
            .list_container_instances_internal(&client, cluster_name)
            .await
        {
            Ok(instances) => {
                details.insert("ContainerInstances".to_string(), instances);
            }
            Err(e) => {
                tracing::debug!(
                    "Could not get container instances for cluster {}: {}",
                    cluster_name,
                    e
                );
            }
        }

        // Get cluster settings with include parameters
        report_status("ECS", "describe_clusters", Some(cluster_name));
        match self
            .describe_clusters_internal(&client, &[cluster_name.to_string()], true)
            .await
        {
            Ok(cluster_details) => {
                if let Some(cluster) = cluster_details.first() {
                    if let Some(obj) = cluster.as_object() {
                        // Merge cluster details into our details map
                        for (key, value) in obj {
                            if key != "Name" && key != "ClusterArn" {
                                details.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Could not get cluster details for {}: {}", cluster_name, e);
            }
        }

        report_status_done("ECS", "get_cluster_details", Some(cluster_name));
        Ok(serde_json::Value::Object(details))
    }

    /// Get detailed information for specific ECS cluster
    pub async fn describe_cluster(
        &self,
        account_id: &str,
        region: &str,
        cluster_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "describe_cluster", Some(cluster_name));

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

        let client = ecs::Client::new(&aws_config);
        let detailed_clusters = self
            .describe_clusters_internal(&client, &[cluster_name.to_string()], true)
            .await?;

        report_status_done("ECS", "describe_cluster", Some(cluster_name));

        if let Some(cluster) = detailed_clusters.first() {
            Ok(cluster.clone())
        } else {
            Err(anyhow::anyhow!("Cluster {} not found", cluster_name))
        }
    }

    /// List ECS services with optional detailed information
    pub async fn list_services(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("ECS", "list_services", Some(region));

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

        let client = ecs::Client::new(&aws_config);
        let mut services = Vec::new();

        // First get all clusters to list services from
        let clusters_response = timeout(Duration::from_secs(10), client.list_clusters().send())
            .await
            .with_context(|| "list_clusters timed out")?
            .with_context(|| "Failed to list clusters")?;

        if let Some(cluster_arns) = clusters_response.cluster_arns {
            for cluster_arn in cluster_arns {
                // List services for each cluster
                let mut paginator = client
                    .list_services()
                    .cluster(&cluster_arn)
                    .into_paginator()
                    .send();

                while let Some(page) = paginator.next().await {
                    let page = page?;
                    if let Some(service_arns) = page.service_arns {
                        if !service_arns.is_empty() {
                            // Describe services to get detailed information
                            if let Ok(detailed_services) = self
                                .describe_services_internal(
                                    &client,
                                    &cluster_arn,
                                    &service_arns,
                                    include_details,
                                )
                                .await
                            {
                                services.extend(detailed_services);
                            } else {
                                // Fallback to basic service info if describe fails
                                for service_arn in service_arns {
                                    let mut basic_service = serde_json::Map::new();
                                    basic_service.insert(
                                        "ServiceArn".to_string(),
                                        serde_json::Value::String(service_arn.clone()),
                                    );
                                    basic_service.insert(
                                        "ClusterArn".to_string(),
                                        serde_json::Value::String(cluster_arn.clone()),
                                    );
                                    // Extract service name from ARN
                                    let service_name =
                                        service_arn.split('/').next_back().unwrap_or(&service_arn);
                                    basic_service.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(service_name.to_string()),
                                    );
                                    services.push(serde_json::Value::Object(basic_service));
                                }
                            }
                        }
                    }
                }
            }
        }

        report_status_done("ECS", "list_services", Some(region));
        Ok(services)
    }

    /// Get detailed information for a single ECS service (Phase 2 enrichment)
    pub async fn get_service_details(
        &self,
        account_id: &str,
        region: &str,
        service_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "get_service_details", Some(service_arn));

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

        let client = ecs::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Extract cluster name from service ARN if present
        let cluster = if service_arn.contains("cluster/") {
            service_arn
                .split("cluster/")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .unwrap_or("default")
        } else {
            "default"
        };

        // Get service with full details including tags
        report_status("ECS", "describe_services", Some(service_arn));
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_services()
                .cluster(cluster)
                .services(service_arn)
                .include(ecs::types::ServiceField::Tags)
                .send(),
        )
        .await
        .with_context(|| "describe_services timed out")?;

        if let Ok(result) = response {
            if let Some(services) = result.services {
                if let Some(service) = services.first() {
                    // Add load balancers
                    if let Some(load_balancers) = &service.load_balancers {
                        let lbs: Vec<serde_json::Value> = load_balancers
                            .iter()
                            .map(|lb| {
                                let mut lb_json = serde_json::Map::new();
                                if let Some(target_group_arn) = &lb.target_group_arn {
                                    lb_json.insert(
                                        "TargetGroupArn".to_string(),
                                        serde_json::Value::String(target_group_arn.clone()),
                                    );
                                }
                                if let Some(container_name) = &lb.container_name {
                                    lb_json.insert(
                                        "ContainerName".to_string(),
                                        serde_json::Value::String(container_name.clone()),
                                    );
                                }
                                if let Some(container_port) = lb.container_port {
                                    lb_json.insert(
                                        "ContainerPort".to_string(),
                                        serde_json::Value::Number(container_port.into()),
                                    );
                                }
                                serde_json::Value::Object(lb_json)
                            })
                            .collect();
                        details.insert("LoadBalancers".to_string(), serde_json::Value::Array(lbs));
                    }

                    // Add deployment configuration
                    if let Some(deployment_config) = &service.deployment_configuration {
                        let mut dc_json = serde_json::Map::new();
                        if let Some(max_percent) = deployment_config.maximum_percent {
                            dc_json.insert(
                                "MaximumPercent".to_string(),
                                serde_json::Value::Number(max_percent.into()),
                            );
                        }
                        if let Some(min_healthy) = deployment_config.minimum_healthy_percent {
                            dc_json.insert(
                                "MinimumHealthyPercent".to_string(),
                                serde_json::Value::Number(min_healthy.into()),
                            );
                        }
                        details.insert(
                            "DeploymentConfiguration".to_string(),
                            serde_json::Value::Object(dc_json),
                        );
                    }

                    // Add deployments
                    if let Some(deployments) = &service.deployments {
                        let deps: Vec<serde_json::Value> = deployments
                            .iter()
                            .map(|dep| {
                                let mut dep_json = serde_json::Map::new();
                                if let Some(id) = &dep.id {
                                    dep_json.insert(
                                        "Id".to_string(),
                                        serde_json::Value::String(id.clone()),
                                    );
                                }
                                if let Some(status) = &dep.status {
                                    dep_json.insert(
                                        "Status".to_string(),
                                        serde_json::Value::String(status.clone()),
                                    );
                                }
                                dep_json.insert(
                                    "DesiredCount".to_string(),
                                    serde_json::Value::Number(dep.desired_count.into()),
                                );
                                dep_json.insert(
                                    "RunningCount".to_string(),
                                    serde_json::Value::Number(dep.running_count.into()),
                                );
                                serde_json::Value::Object(dep_json)
                            })
                            .collect();
                        details.insert("Deployments".to_string(), serde_json::Value::Array(deps));
                    }

                    // Add events (last 10)
                    if let Some(events) = &service.events {
                        let evts: Vec<serde_json::Value> = events
                            .iter()
                            .take(10)
                            .map(|evt| {
                                let mut evt_json = serde_json::Map::new();
                                if let Some(id) = &evt.id {
                                    evt_json.insert(
                                        "Id".to_string(),
                                        serde_json::Value::String(id.clone()),
                                    );
                                }
                                if let Some(message) = &evt.message {
                                    evt_json.insert(
                                        "Message".to_string(),
                                        serde_json::Value::String(message.clone()),
                                    );
                                }
                                if let Some(created_at) = evt.created_at {
                                    evt_json.insert(
                                        "CreatedAt".to_string(),
                                        serde_json::Value::String(created_at.to_string()),
                                    );
                                }
                                serde_json::Value::Object(evt_json)
                            })
                            .collect();
                        details.insert("RecentEvents".to_string(), serde_json::Value::Array(evts));
                    }

                    // Add tags
                    if let Some(tags) = &service.tags {
                        let tags_json: serde_json::Map<String, serde_json::Value> = tags
                            .iter()
                            .filter_map(|tag| {
                                tag.key.as_ref().map(|k| {
                                    (
                                        k.clone(),
                                        serde_json::Value::String(
                                            tag.value.clone().unwrap_or_default(),
                                        ),
                                    )
                                })
                            })
                            .collect();
                        details.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
                    }
                }
            }
        }

        report_status_done("ECS", "get_service_details", Some(service_arn));
        Ok(serde_json::Value::Object(details))
    }

    /// List ECS Fargate Services (Fargate launch type only)
    pub async fn list_fargate_services(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let all_services = self.list_services(account_id, region, false).await?;

        // Filter for Fargate services only
        let fargate_services: Vec<serde_json::Value> = all_services
            .into_iter()
            .filter(|service| {
                service
                    .get("LaunchType")
                    .and_then(|lt| lt.as_str())
                    .map(|lt| lt == "FARGATE")
                    .unwrap_or(false)
            })
            .collect();

        Ok(fargate_services)
    }

    /// List ECS Fargate Tasks (Fargate launch type only)
    pub async fn list_fargate_tasks(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let all_tasks = self.list_tasks(account_id, region, false).await?;

        // Filter for Fargate tasks only
        let fargate_tasks: Vec<serde_json::Value> = all_tasks
            .into_iter()
            .filter(|task| {
                task.get("LaunchType")
                    .and_then(|lt| lt.as_str())
                    .map(|lt| lt == "FARGATE")
                    .unwrap_or(false)
            })
            .collect();

        Ok(fargate_tasks)
    }

    /// List ECS tasks with optional detailed information
    pub async fn list_tasks(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("ECS", "list_tasks", Some(region));

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

        let client = ecs::Client::new(&aws_config);
        let mut tasks = Vec::new();

        // First get all clusters to list tasks from
        let clusters_response = timeout(Duration::from_secs(10), client.list_clusters().send())
            .await
            .with_context(|| "list_clusters timed out")?
            .with_context(|| "Failed to list clusters")?;

        if let Some(cluster_arns) = clusters_response.cluster_arns {
            for cluster_arn in cluster_arns {
                // List tasks for each cluster
                let mut paginator = client
                    .list_tasks()
                    .cluster(&cluster_arn)
                    .into_paginator()
                    .send();

                while let Some(page) = paginator.next().await {
                    let page = page?;
                    if let Some(task_arns) = page.task_arns {
                        if !task_arns.is_empty() {
                            // Describe tasks to get detailed information
                            if let Ok(detailed_tasks) = self
                                .describe_tasks_internal(
                                    &client,
                                    &cluster_arn,
                                    &task_arns,
                                    include_details,
                                )
                                .await
                            {
                                tasks.extend(detailed_tasks);
                            } else {
                                // Fallback to basic task info if describe fails
                                for task_arn in task_arns {
                                    let mut basic_task = serde_json::Map::new();
                                    basic_task.insert(
                                        "TaskArn".to_string(),
                                        serde_json::Value::String(task_arn.clone()),
                                    );
                                    basic_task.insert(
                                        "ClusterArn".to_string(),
                                        serde_json::Value::String(cluster_arn.clone()),
                                    );
                                    // Extract task ID from ARN
                                    let task_id =
                                        task_arn.split('/').next_back().unwrap_or(&task_arn);
                                    basic_task.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(task_id.to_string()),
                                    );
                                    tasks.push(serde_json::Value::Object(basic_task));
                                }
                            }
                        }
                    }
                }
            }
        }

        report_status_done("ECS", "list_tasks", Some(region));
        Ok(tasks)
    }

    /// List ECS task definitions with optional detailed information
    pub async fn list_task_definitions(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("ECS", "list_task_definitions", Some(region));

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

        let client = ecs::Client::new(&aws_config);
        let mut task_definitions = Vec::new();

        // List task definition families first
        let mut paginator = client
            .list_task_definition_families()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(families) = page.families {
                for family in families {
                    // Get latest task definition for each family
                    report_status("ECS", "describe_task_definition", Some(&family));
                    if let Ok(Ok(response)) = timeout(
                        Duration::from_secs(10),
                        client
                            .describe_task_definition()
                            .task_definition(&family)
                            .send(),
                    )
                    .await
                    {
                        if let Some(task_definition) = response.task_definition {
                            let task_def_json =
                                self.task_definition_to_json(&task_definition, include_details);
                            task_definitions.push(task_def_json);
                        }
                    }
                }
            }
        }

        report_status_done("ECS", "list_task_definitions", Some(region));
        Ok(task_definitions)
    }

    /// List container instances for a cluster
    pub async fn list_container_instances(
        &self,
        account_id: &str,
        region: &str,
        cluster_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "list_container_instances", Some(cluster_name));

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

        let client = ecs::Client::new(&aws_config);
        let result = self
            .list_container_instances_internal(&client, cluster_name)
            .await;

        report_status_done("ECS", "list_container_instances", Some(cluster_name));
        result
    }

    async fn list_container_instances_internal(
        &self,
        client: &ecs::Client,
        cluster_name: &str,
    ) -> Result<serde_json::Value> {
        // List container instance ARNs
        let list_response = timeout(
            Duration::from_secs(10),
            client
                .list_container_instances()
                .cluster(cluster_name)
                .send(),
        )
        .await
        .with_context(|| "list_container_instances timed out")?
        .with_context(|| format!("Failed to list container instances for {}", cluster_name))?;

        let mut instances_json = Vec::new();

        if let Some(instance_arns) = list_response.container_instance_arns {
            if !instance_arns.is_empty() {
                // Describe the container instances
                let describe_response = timeout(
                    Duration::from_secs(10),
                    client
                        .describe_container_instances()
                        .cluster(cluster_name)
                        .set_container_instances(Some(instance_arns))
                        .send(),
                )
                .await
                .with_context(|| "describe_container_instances timed out")?
                .with_context(|| {
                    format!(
                        "Failed to describe container instances for {}",
                        cluster_name
                    )
                })?;

                if let Some(instances) = describe_response.container_instances {
                    for instance in instances {
                        let mut instance_json = serde_json::Map::new();

                        if let Some(arn) = &instance.container_instance_arn {
                            instance_json.insert(
                                "ContainerInstanceArn".to_string(),
                                serde_json::Value::String(arn.clone()),
                            );
                        }

                        if let Some(ec2_instance_id) = &instance.ec2_instance_id {
                            instance_json.insert(
                                "Ec2InstanceId".to_string(),
                                serde_json::Value::String(ec2_instance_id.clone()),
                            );
                        }

                        if let Some(status) = &instance.status {
                            instance_json.insert(
                                "Status".to_string(),
                                serde_json::Value::String(status.clone()),
                            );
                        }

                        instance_json.insert(
                            "AgentConnected".to_string(),
                            serde_json::Value::Bool(instance.agent_connected),
                        );

                        instance_json.insert(
                            "RunningTasksCount".to_string(),
                            serde_json::Value::Number(instance.running_tasks_count.into()),
                        );

                        instance_json.insert(
                            "PendingTasksCount".to_string(),
                            serde_json::Value::Number(instance.pending_tasks_count.into()),
                        );

                        // Add registered resources
                        if let Some(resources) = &instance.registered_resources {
                            let res_json: Vec<serde_json::Value> = resources
                                .iter()
                                .filter_map(|res| {
                                    res.name.as_ref().map(|name| {
                                        let mut r = serde_json::Map::new();
                                        r.insert(
                                            "Name".to_string(),
                                            serde_json::Value::String(name.clone()),
                                        );
                                        r.insert(
                                            "IntegerValue".to_string(),
                                            serde_json::Value::Number(res.integer_value.into()),
                                        );
                                        serde_json::Value::Object(r)
                                    })
                                })
                                .collect();
                            instance_json.insert(
                                "RegisteredResources".to_string(),
                                serde_json::Value::Array(res_json),
                            );
                        }

                        instances_json.push(serde_json::Value::Object(instance_json));
                    }
                }
            }
        }

        Ok(serde_json::Value::Array(instances_json))
    }

    /// Describe specific ECS Service
    pub async fn describe_service(
        &self,
        account_id: &str,
        region: &str,
        service_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "describe_service", Some(service_arn));

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

        let client = ecs::Client::new(&aws_config);

        // Extract cluster name from service ARN if present
        let cluster = if service_arn.contains("cluster/") {
            service_arn
                .split("cluster/")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .unwrap_or("default")
        } else {
            "default"
        };

        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_services()
                .cluster(cluster)
                .services(service_arn)
                .include(ecs::types::ServiceField::Tags)
                .send(),
        )
        .await
        .with_context(|| "describe_services timed out")?
        .with_context(|| format!("Failed to describe service {}", service_arn))?;

        report_status_done("ECS", "describe_service", Some(service_arn));

        if let Some(services) = response.services {
            if let Some(service) = services.into_iter().next() {
                return Ok(self.service_to_json(&service, true));
            }
        }

        Err(anyhow::anyhow!("ECS Service not found: {}", service_arn))
    }

    /// Describe specific ECS Task
    pub async fn describe_task(
        &self,
        account_id: &str,
        region: &str,
        task_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "describe_task", Some(task_arn));

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

        let client = ecs::Client::new(&aws_config);

        // Extract cluster name from task ARN if present
        let cluster = if task_arn.contains("cluster/") {
            task_arn
                .split("cluster/")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .unwrap_or("default")
        } else {
            "default"
        };

        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_tasks()
                .cluster(cluster)
                .tasks(task_arn)
                .include(ecs::types::TaskField::Tags)
                .send(),
        )
        .await
        .with_context(|| "describe_tasks timed out")?
        .with_context(|| format!("Failed to describe task {}", task_arn))?;

        report_status_done("ECS", "describe_task", Some(task_arn));

        if let Some(tasks) = response.tasks {
            if let Some(task) = tasks.into_iter().next() {
                return Ok(self.task_to_json(&task, true));
            }
        }

        Err(anyhow::anyhow!("ECS Task not found: {}", task_arn))
    }

    /// Describe specific ECS Task Definition
    pub async fn describe_task_definition(
        &self,
        account_id: &str,
        region: &str,
        task_definition_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("ECS", "describe_task_definition", Some(task_definition_arn));

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

        let client = ecs::Client::new(&aws_config);
        let response = timeout(
            Duration::from_secs(10),
            client
                .describe_task_definition()
                .task_definition(task_definition_arn)
                .include(ecs::types::TaskDefinitionField::Tags)
                .send(),
        )
        .await
        .with_context(|| "describe_task_definition timed out")?
        .with_context(|| format!("Failed to describe task definition {}", task_definition_arn))?;

        report_status_done("ECS", "describe_task_definition", Some(task_definition_arn));

        if let Some(task_definition) = response.task_definition {
            return Ok(self.task_definition_to_json(&task_definition, true));
        }

        Err(anyhow::anyhow!(
            "ECS Task Definition not found: {}",
            task_definition_arn
        ))
    }

    async fn describe_clusters_internal(
        &self,
        client: &ecs::Client,
        cluster_arns: &[String],
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        let mut request = client
            .describe_clusters()
            .set_clusters(Some(cluster_arns.to_vec()));

        if include_details {
            request = request
                .include(ecs::types::ClusterField::Settings)
                .include(ecs::types::ClusterField::Statistics)
                .include(ecs::types::ClusterField::Tags)
                .include(ecs::types::ClusterField::Configurations);
        }

        let response = timeout(Duration::from_secs(10), request.send())
            .await
            .with_context(|| "describe_clusters timed out")?
            .with_context(|| "Failed to describe clusters")?;

        let mut clusters = Vec::new();
        if let Some(cluster_list) = response.clusters {
            for cluster in cluster_list {
                let cluster_json = self.cluster_to_json(&cluster, include_details);
                clusters.push(cluster_json);
            }
        }

        Ok(clusters)
    }

    async fn describe_services_internal(
        &self,
        client: &ecs::Client,
        cluster_arn: &str,
        service_arns: &[String],
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        let mut request = client
            .describe_services()
            .cluster(cluster_arn)
            .set_services(Some(service_arns.to_vec()));

        if include_details {
            request = request.include(ecs::types::ServiceField::Tags);
        }

        let response = timeout(Duration::from_secs(10), request.send())
            .await
            .with_context(|| "describe_services timed out")?
            .with_context(|| "Failed to describe services")?;

        let mut services = Vec::new();
        if let Some(service_list) = response.services {
            for service in service_list {
                let service_json = self.service_to_json(&service, include_details);
                services.push(service_json);
            }
        }

        Ok(services)
    }

    async fn describe_tasks_internal(
        &self,
        client: &ecs::Client,
        cluster_arn: &str,
        task_arns: &[String],
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        let mut request = client
            .describe_tasks()
            .cluster(cluster_arn)
            .set_tasks(Some(task_arns.to_vec()));

        if include_details {
            request = request.include(ecs::types::TaskField::Tags);
        }

        let response = timeout(Duration::from_secs(10), request.send())
            .await
            .with_context(|| "describe_tasks timed out")?
            .with_context(|| "Failed to describe tasks")?;

        let mut tasks = Vec::new();
        if let Some(task_list) = response.tasks {
            for task in task_list {
                let task_json = self.task_to_json(&task, include_details);
                tasks.push(task_json);
            }
        }

        Ok(tasks)
    }

    fn cluster_to_json(
        &self,
        cluster: &ecs::types::Cluster,
        include_details: bool,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cluster_arn) = &cluster.cluster_arn {
            json.insert(
                "ClusterArn".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );
        }

        if let Some(cluster_name) = &cluster.cluster_name {
            json.insert(
                "ClusterName".to_string(),
                serde_json::Value::String(cluster_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(cluster_name.clone()),
            );
        }

        if let Some(status) = &cluster.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        json.insert(
            "RegisteredContainerInstancesCount".to_string(),
            serde_json::Value::Number(cluster.registered_container_instances_count.into()),
        );

        json.insert(
            "RunningTasksCount".to_string(),
            serde_json::Value::Number(cluster.running_tasks_count.into()),
        );

        json.insert(
            "PendingTasksCount".to_string(),
            serde_json::Value::Number(cluster.pending_tasks_count.into()),
        );

        json.insert(
            "ActiveServicesCount".to_string(),
            serde_json::Value::Number(cluster.active_services_count.into()),
        );

        // Only include detailed info if requested
        if include_details {
            if let Some(_configuration) = &cluster.configuration {
                let mut config_json = serde_json::Map::new();
                config_json.insert(
                    "HasConfiguration".to_string(),
                    serde_json::Value::Bool(true),
                );
                json.insert(
                    "Configuration".to_string(),
                    serde_json::Value::Object(config_json),
                );
            }

            if let Some(statistics) = &cluster.statistics {
                let stats_json: Vec<serde_json::Value> = statistics
                    .iter()
                    .map(|stat| {
                        let mut stat_json = serde_json::Map::new();
                        if let Some(name) = &stat.name {
                            stat_json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(name.clone()),
                            );
                        }
                        if let Some(value) = &stat.value {
                            stat_json.insert(
                                "Value".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                        }
                        serde_json::Value::Object(stat_json)
                    })
                    .collect();
                json.insert(
                    "Statistics".to_string(),
                    serde_json::Value::Array(stats_json),
                );
            }

            if let Some(capacity_providers) = &cluster.capacity_providers {
                let providers_json: Vec<serde_json::Value> = capacity_providers
                    .iter()
                    .map(|provider| serde_json::Value::String(provider.clone()))
                    .collect();
                json.insert(
                    "CapacityProviders".to_string(),
                    serde_json::Value::Array(providers_json),
                );
            }

            if let Some(default_capacity_provider_strategy) =
                &cluster.default_capacity_provider_strategy
            {
                let strategy_json: Vec<serde_json::Value> = default_capacity_provider_strategy
                    .iter()
                    .map(|strategy| {
                        let mut strategy_json = serde_json::Map::new();
                        strategy_json.insert(
                            "CapacityProvider".to_string(),
                            serde_json::Value::String(strategy.capacity_provider.clone()),
                        );
                        strategy_json.insert(
                            "Weight".to_string(),
                            serde_json::Value::Number(strategy.weight.into()),
                        );
                        strategy_json.insert(
                            "Base".to_string(),
                            serde_json::Value::Number(strategy.base.into()),
                        );
                        serde_json::Value::Object(strategy_json)
                    })
                    .collect();
                json.insert(
                    "DefaultCapacityProviderStrategy".to_string(),
                    serde_json::Value::Array(strategy_json),
                );
            }

            if let Some(settings) = &cluster.settings {
                let settings_json: Vec<serde_json::Value> = settings
                    .iter()
                    .map(|setting| {
                        let mut s = serde_json::Map::new();
                        if let Some(name) = &setting.name {
                            s.insert(
                                "Name".to_string(),
                                serde_json::Value::String(name.as_str().to_string()),
                            );
                        }
                        if let Some(value) = &setting.value {
                            s.insert(
                                "Value".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                        }
                        serde_json::Value::Object(s)
                    })
                    .collect();
                json.insert(
                    "Settings".to_string(),
                    serde_json::Value::Array(settings_json),
                );
            }

            if let Some(tags) = &cluster.tags {
                let tags_json: serde_json::Map<String, serde_json::Value> = tags
                    .iter()
                    .filter_map(|tag| {
                        tag.key.as_ref().map(|k| {
                            (
                                k.clone(),
                                serde_json::Value::String(tag.value.clone().unwrap_or_default()),
                            )
                        })
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    fn service_to_json(
        &self,
        service: &ecs::types::Service,
        include_details: bool,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(service_arn) = &service.service_arn {
            json.insert(
                "ServiceArn".to_string(),
                serde_json::Value::String(service_arn.clone()),
            );
        }

        if let Some(service_name) = &service.service_name {
            json.insert(
                "ServiceName".to_string(),
                serde_json::Value::String(service_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(service_name.clone()),
            );
        }

        if let Some(cluster_arn) = &service.cluster_arn {
            json.insert(
                "ClusterArn".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );
        }

        if let Some(task_definition) = &service.task_definition {
            json.insert(
                "TaskDefinition".to_string(),
                serde_json::Value::String(task_definition.clone()),
            );
        }

        if let Some(status) = &service.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        json.insert(
            "DesiredCount".to_string(),
            serde_json::Value::Number(service.desired_count.into()),
        );
        json.insert(
            "RunningCount".to_string(),
            serde_json::Value::Number(service.running_count.into()),
        );
        json.insert(
            "PendingCount".to_string(),
            serde_json::Value::Number(service.pending_count.into()),
        );

        if let Some(launch_type) = &service.launch_type {
            json.insert(
                "LaunchType".to_string(),
                serde_json::Value::String(launch_type.as_str().to_string()),
            );
        }

        if let Some(platform_version) = &service.platform_version {
            json.insert(
                "PlatformVersion".to_string(),
                serde_json::Value::String(platform_version.clone()),
            );
        }

        // Only include detailed info if requested
        if include_details {
            if let Some(load_balancers) = &service.load_balancers {
                let lbs: Vec<serde_json::Value> = load_balancers
                    .iter()
                    .map(|lb| {
                        let mut lb_json = serde_json::Map::new();
                        if let Some(target_group_arn) = &lb.target_group_arn {
                            lb_json.insert(
                                "TargetGroupArn".to_string(),
                                serde_json::Value::String(target_group_arn.clone()),
                            );
                        }
                        if let Some(container_name) = &lb.container_name {
                            lb_json.insert(
                                "ContainerName".to_string(),
                                serde_json::Value::String(container_name.clone()),
                            );
                        }
                        if let Some(container_port) = lb.container_port {
                            lb_json.insert(
                                "ContainerPort".to_string(),
                                serde_json::Value::Number(container_port.into()),
                            );
                        }
                        serde_json::Value::Object(lb_json)
                    })
                    .collect();
                json.insert("LoadBalancers".to_string(), serde_json::Value::Array(lbs));
            }

            if let Some(deployment_config) = &service.deployment_configuration {
                let mut dc_json = serde_json::Map::new();
                if let Some(max_percent) = deployment_config.maximum_percent {
                    dc_json.insert(
                        "MaximumPercent".to_string(),
                        serde_json::Value::Number(max_percent.into()),
                    );
                }
                if let Some(min_healthy) = deployment_config.minimum_healthy_percent {
                    dc_json.insert(
                        "MinimumHealthyPercent".to_string(),
                        serde_json::Value::Number(min_healthy.into()),
                    );
                }
                json.insert(
                    "DeploymentConfiguration".to_string(),
                    serde_json::Value::Object(dc_json),
                );
            }

            if let Some(tags) = &service.tags {
                let tags_json: serde_json::Map<String, serde_json::Value> = tags
                    .iter()
                    .filter_map(|tag| {
                        tag.key.as_ref().map(|k| {
                            (
                                k.clone(),
                                serde_json::Value::String(tag.value.clone().unwrap_or_default()),
                            )
                        })
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    fn task_to_json(&self, task: &ecs::types::Task, include_details: bool) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(task_arn) = &task.task_arn {
            json.insert(
                "TaskArn".to_string(),
                serde_json::Value::String(task_arn.clone()),
            );
            // Extract task ID from ARN for display name
            let task_id = task_arn.split('/').next_back().unwrap_or(task_arn);
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(task_id.to_string()),
            );
        }

        if let Some(cluster_arn) = &task.cluster_arn {
            json.insert(
                "ClusterArn".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );
        }

        if let Some(task_definition_arn) = &task.task_definition_arn {
            json.insert(
                "TaskDefinitionArn".to_string(),
                serde_json::Value::String(task_definition_arn.clone()),
            );
        }

        if let Some(last_status) = &task.last_status {
            json.insert(
                "LastStatus".to_string(),
                serde_json::Value::String(last_status.clone()),
            );
        }

        if let Some(desired_status) = &task.desired_status {
            json.insert(
                "DesiredStatus".to_string(),
                serde_json::Value::String(desired_status.clone()),
            );
        }

        if let Some(launch_type) = &task.launch_type {
            json.insert(
                "LaunchType".to_string(),
                serde_json::Value::String(launch_type.as_str().to_string()),
            );
        }

        if let Some(platform_version) = &task.platform_version {
            json.insert(
                "PlatformVersion".to_string(),
                serde_json::Value::String(platform_version.clone()),
            );
        }

        if let Some(cpu) = &task.cpu {
            json.insert("Cpu".to_string(), serde_json::Value::String(cpu.clone()));
        }

        if let Some(memory) = &task.memory {
            json.insert(
                "Memory".to_string(),
                serde_json::Value::String(memory.clone()),
            );
        }

        // Only include detailed info if requested
        if include_details {
            if let Some(containers) = &task.containers {
                let containers_json: Vec<serde_json::Value> = containers
                    .iter()
                    .map(|c| {
                        let mut cj = serde_json::Map::new();
                        if let Some(name) = &c.name {
                            cj.insert("Name".to_string(), serde_json::Value::String(name.clone()));
                        }
                        if let Some(last_status) = &c.last_status {
                            cj.insert(
                                "LastStatus".to_string(),
                                serde_json::Value::String(last_status.clone()),
                            );
                        }
                        if let Some(exit_code) = c.exit_code {
                            cj.insert(
                                "ExitCode".to_string(),
                                serde_json::Value::Number(exit_code.into()),
                            );
                        }
                        serde_json::Value::Object(cj)
                    })
                    .collect();
                json.insert(
                    "Containers".to_string(),
                    serde_json::Value::Array(containers_json),
                );
            }

            if let Some(tags) = &task.tags {
                let tags_json: serde_json::Map<String, serde_json::Value> = tags
                    .iter()
                    .filter_map(|tag| {
                        tag.key.as_ref().map(|k| {
                            (
                                k.clone(),
                                serde_json::Value::String(tag.value.clone().unwrap_or_default()),
                            )
                        })
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }

    fn task_definition_to_json(
        &self,
        task_definition: &ecs::types::TaskDefinition,
        include_details: bool,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(task_definition_arn) = &task_definition.task_definition_arn {
            json.insert(
                "TaskDefinitionArn".to_string(),
                serde_json::Value::String(task_definition_arn.clone()),
            );
        }

        if let Some(family) = &task_definition.family {
            json.insert(
                "Family".to_string(),
                serde_json::Value::String(family.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(family.clone()),
            );
        }

        json.insert(
            "Revision".to_string(),
            serde_json::Value::Number(task_definition.revision.into()),
        );

        if let Some(status) = &task_definition.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(cpu) = &task_definition.cpu {
            json.insert("Cpu".to_string(), serde_json::Value::String(cpu.clone()));
        }

        if let Some(memory) = &task_definition.memory {
            json.insert(
                "Memory".to_string(),
                serde_json::Value::String(memory.clone()),
            );
        }

        if let Some(network_mode) = &task_definition.network_mode {
            json.insert(
                "NetworkMode".to_string(),
                serde_json::Value::String(network_mode.as_str().to_string()),
            );
        }

        if let Some(requires_compatibilities) = &task_definition.requires_compatibilities {
            let compatibilities: Vec<serde_json::Value> = requires_compatibilities
                .iter()
                .map(|c| serde_json::Value::String(c.as_str().to_string()))
                .collect();
            json.insert(
                "RequiresCompatibilities".to_string(),
                serde_json::Value::Array(compatibilities),
            );
        }

        if let Some(container_definitions) = &task_definition.container_definitions {
            json.insert(
                "ContainerDefinitionsCount".to_string(),
                serde_json::Value::Number(container_definitions.len().into()),
            );

            // Only include full container definitions if requested
            if include_details {
                let containers: Vec<serde_json::Value> = container_definitions
                    .iter()
                    .map(|container| {
                        let mut container_json = serde_json::Map::new();
                        if let Some(name) = &container.name {
                            container_json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(name.clone()),
                            );
                        }
                        if let Some(image) = &container.image {
                            container_json.insert(
                                "Image".to_string(),
                                serde_json::Value::String(image.clone()),
                            );
                        }
                        if let Some(memory) = container.memory {
                            container_json.insert(
                                "Memory".to_string(),
                                serde_json::Value::Number(memory.into()),
                            );
                        }
                        container_json.insert(
                            "Cpu".to_string(),
                            serde_json::Value::Number(container.cpu.into()),
                        );
                        if let Some(essential) = container.essential {
                            container_json.insert(
                                "Essential".to_string(),
                                serde_json::Value::Bool(essential),
                            );
                        }

                        // Add port mappings
                        if let Some(port_mappings) = &container.port_mappings {
                            let ports: Vec<serde_json::Value> = port_mappings
                                .iter()
                                .map(|pm| {
                                    let mut p = serde_json::Map::new();
                                    if let Some(container_port) = pm.container_port {
                                        p.insert(
                                            "ContainerPort".to_string(),
                                            serde_json::Value::Number(container_port.into()),
                                        );
                                    }
                                    if let Some(host_port) = pm.host_port {
                                        p.insert(
                                            "HostPort".to_string(),
                                            serde_json::Value::Number(host_port.into()),
                                        );
                                    }
                                    if let Some(protocol) = &pm.protocol {
                                        p.insert(
                                            "Protocol".to_string(),
                                            serde_json::Value::String(
                                                protocol.as_str().to_string(),
                                            ),
                                        );
                                    }
                                    serde_json::Value::Object(p)
                                })
                                .collect();
                            container_json.insert(
                                "PortMappings".to_string(),
                                serde_json::Value::Array(ports),
                            );
                        }

                        serde_json::Value::Object(container_json)
                    })
                    .collect();
                json.insert(
                    "ContainerDefinitions".to_string(),
                    serde_json::Value::Array(containers),
                );
            }
        }

        serde_json::Value::Object(json)
    }
}
