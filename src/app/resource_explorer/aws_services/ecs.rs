use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_ecs as ecs;
use std::sync::Arc;

pub struct ECSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ECSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List ECS clusters
    pub async fn list_clusters(
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

        let client = ecs::Client::new(&aws_config);
        let mut paginator = client.list_clusters().into_paginator().send();

        let mut clusters = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(cluster_arns) = page.cluster_arns {
                if !cluster_arns.is_empty() {
                    // Describe clusters to get detailed information
                    if let Ok(detailed_clusters) = self
                        .describe_clusters_internal(&client, &cluster_arns)
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

        Ok(clusters)
    }

    /// Get detailed information for specific ECS cluster
    pub async fn describe_cluster(
        &self,
        account_id: &str,
        region: &str,
        cluster_name: &str,
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

        let client = ecs::Client::new(&aws_config);
        let detailed_clusters = self
            .describe_clusters_internal(&client, &[cluster_name.to_string()])
            .await?;

        if let Some(cluster) = detailed_clusters.first() {
            Ok(cluster.clone())
        } else {
            Err(anyhow::anyhow!("Cluster {} not found", cluster_name))
        }
    }

    async fn describe_clusters_internal(
        &self,
        client: &ecs::Client,
        cluster_arns: &[String],
    ) -> Result<Vec<serde_json::Value>> {
        let response = client
            .describe_clusters()
            .set_clusters(Some(cluster_arns.to_vec()))
            .send()
            .await?;

        let mut clusters = Vec::new();
        if let Some(cluster_list) = response.clusters {
            for cluster in cluster_list {
                let cluster_json = self.cluster_to_json(&cluster);
                clusters.push(cluster_json);
            }
        }

        Ok(clusters)
    }

    fn cluster_to_json(&self, cluster: &ecs::types::Cluster) -> serde_json::Value {
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

        if let Some(_configuration) = &cluster.configuration {
            let mut config_json = serde_json::Map::new();
            // Note: ECS Configuration fields need manual conversion due to AWS SDK serialization
            config_json.insert(
                "HasConfiguration".to_string(),
                serde_json::Value::Bool(true),
            );
            json.insert(
                "Configuration".to_string(),
                serde_json::Value::Object(config_json),
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

        if let Some(statistics) = &cluster.statistics {
            let stats_json: Vec<serde_json::Value> = statistics
                .iter()
                .map(|stat| {
                    let mut stat_json = serde_json::Map::new();
                    if let Some(name) = &stat.name {
                        stat_json
                            .insert("Name".to_string(), serde_json::Value::String(name.clone()));
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

        serde_json::Value::Object(json)
    }

    /// Describe specific ECS Service
    pub async fn describe_service(
        &self,
        account_id: &str,
        region: &str,
        service_arn: &str,
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

        let response = client
            .describe_services()
            .cluster(cluster)
            .services(service_arn)
            .send()
            .await?;

        if let Some(services) = response.services {
            if let Some(service) = services.into_iter().next() {
                return Ok(self.service_to_json(&service));
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

        let response = client
            .describe_tasks()
            .cluster(cluster)
            .tasks(task_arn)
            .send()
            .await?;

        if let Some(tasks) = response.tasks {
            if let Some(task) = tasks.into_iter().next() {
                return Ok(self.task_to_json(&task));
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
        let response = client
            .describe_task_definition()
            .task_definition(task_definition_arn)
            .send()
            .await?;

        if let Some(task_definition) = response.task_definition {
            return Ok(self.task_definition_to_json(&task_definition));
        }

        Err(anyhow::anyhow!(
            "ECS Task Definition not found: {}",
            task_definition_arn
        ))
    }

    /// List ECS services
    pub async fn list_services(
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

        let client = ecs::Client::new(&aws_config);
        let mut services = Vec::new();

        // First get all clusters to list services from
        let clusters_response = client.list_clusters().send().await?;

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
                                .describe_services_internal(&client, &cluster_arn, &service_arns)
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

        Ok(services)
    }

    /// List ECS tasks
    pub async fn list_tasks(
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

        let client = ecs::Client::new(&aws_config);
        let mut tasks = Vec::new();

        // First get all clusters to list tasks from
        let clusters_response = client.list_clusters().send().await?;

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
                                .describe_tasks_internal(&client, &cluster_arn, &task_arns)
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

        Ok(tasks)
    }

    /// List ECS task definitions
    pub async fn list_task_definitions(
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
                    if let Ok(task_def_response) = client
                        .describe_task_definition()
                        .task_definition(&family)
                        .send()
                        .await
                    {
                        if let Some(task_definition) = task_def_response.task_definition {
                            let task_def_json = self.task_definition_to_json(&task_definition);
                            task_definitions.push(task_def_json);
                        }
                    }
                }
            }
        }

        Ok(task_definitions)
    }

    async fn describe_services_internal(
        &self,
        client: &ecs::Client,
        cluster_arn: &str,
        service_arns: &[String],
    ) -> Result<Vec<serde_json::Value>> {
        let response = client
            .describe_services()
            .cluster(cluster_arn)
            .set_services(Some(service_arns.to_vec()))
            .send()
            .await?;

        let mut services = Vec::new();
        if let Some(service_list) = response.services {
            for service in service_list {
                let service_json = self.service_to_json(&service);
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
    ) -> Result<Vec<serde_json::Value>> {
        let response = client
            .describe_tasks()
            .cluster(cluster_arn)
            .set_tasks(Some(task_arns.to_vec()))
            .send()
            .await?;

        let mut tasks = Vec::new();
        if let Some(task_list) = response.tasks {
            for task in task_list {
                let task_json = self.task_to_json(&task);
                tasks.push(task_json);
            }
        }

        Ok(tasks)
    }

    fn service_to_json(&self, service: &ecs::types::Service) -> serde_json::Value {
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

        serde_json::Value::Object(json)
    }

    fn task_to_json(&self, task: &ecs::types::Task) -> serde_json::Value {
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

        serde_json::Value::Object(json)
    }

    fn task_definition_to_json(
        &self,
        task_definition: &ecs::types::TaskDefinition,
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

            let containers: Vec<serde_json::Value> = container_definitions
                .iter()
                .map(|container| {
                    let mut container_json = serde_json::Map::new();
                    if let Some(name) = &container.name {
                        container_json
                            .insert("Name".to_string(), serde_json::Value::String(name.clone()));
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
                    serde_json::Value::Object(container_json)
                })
                .collect();
            json.insert(
                "ContainerDefinitions".to_string(),
                serde_json::Value::Array(containers),
            );
        }

        serde_json::Value::Object(json)
    }
}
