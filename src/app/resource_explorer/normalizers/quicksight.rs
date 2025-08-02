use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for QuickSight Data Source resources
pub struct QuickSightDataSourceNormalizer;

impl ResourceNormalizer for QuickSightDataSourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let data_source_id = raw_response
            .get("DataSourceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&data_source_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut tags = Vec::new();

        // Add data source type as a tag
        if let Some(data_source_type) = raw_response.get("Type").and_then(|v| v.as_str()) {
            tags.push(ResourceTag {
                key: "DataSourceType".to_string(),
                value: data_source_type.to_string(),
            });
        }

        Ok(ResourceEntry {
            resource_id: data_source_id.clone(),
            resource_type: "AWS::QuickSight::DataSource".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            display_name: name,
            status,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            query_timestamp,
            account_color: egui::Color32::GRAY,
            region_color: egui::Color32::GRAY,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract relationships from data source parameters
        if let Some(detailed_props) = &entry.detailed_properties {
            if let Some(params) = detailed_props.get("DataSourceParameters") {
                // RDS Instance relationship
                if let Some(instance_id) = params.get("RdsInstanceId").and_then(|v| v.as_str()) {
                    if let Some(_rds_instance) = all_resources.iter().find(|r| {
                        r.resource_type == "AWS::RDS::DBInstance"
                            && r.resource_id == instance_id
                            && r.account_id == entry.account_id
                            && r.region == entry.region
                    }) {
                        relationships.push(ResourceRelationship {
                            target_resource_id: instance_id.to_string(),
                            target_resource_type: "AWS::RDS::DBInstance".to_string(),
                            relationship_type: RelationshipType::Uses,
                        });
                    }
                }

                // Redshift Cluster relationship (derived from host)
                if let Some(redshift_host) = params.get("RedshiftHost").and_then(|v| v.as_str()) {
                    if let Some(cluster_name) = redshift_host.split('.').next() {
                        if let Some(_redshift_cluster) = all_resources.iter().find(|r| {
                            r.resource_type == "AWS::Redshift::Cluster"
                                && r.resource_id == cluster_name
                                && r.account_id == entry.account_id
                                && r.region == entry.region
                        }) {
                            relationships.push(ResourceRelationship {
                                target_resource_id: cluster_name.to_string(),
                                target_resource_type: "AWS::Redshift::Cluster".to_string(),
                                relationship_type: RelationshipType::Uses,
                            });
                        }
                    }
                }

                // S3 Bucket relationship
                if let Some(s3_location) = params
                    .get("S3ManifestFileLocation")
                    .and_then(|v| v.as_str())
                {
                    if let Some(_s3_bucket) = all_resources.iter().find(|r| {
                        r.resource_type == "AWS::S3::Bucket"
                            && r.resource_id == s3_location
                            && r.account_id == entry.account_id
                    }) {
                        relationships.push(ResourceRelationship {
                            target_resource_id: s3_location.to_string(),
                            target_resource_type: "AWS::S3::Bucket".to_string(),
                            relationship_type: RelationshipType::Uses,
                        });
                    }
                }
            }

            // VPC Connection relationship
            if let Some(vpc_props) = detailed_props.get("VpcConnectionProperties") {
                if let Some(vpc_conn_arn) =
                    vpc_props.get("VpcConnectionArn").and_then(|v| v.as_str())
                {
                    // Extract VPC ID from connection ARN if possible
                    if let Some(vpc_id) = vpc_conn_arn.split('/').next_back() {
                        if let Some(_vpc) = all_resources.iter().find(|r| {
                            r.resource_type == "AWS::EC2::VPC"
                                && r.resource_id == vpc_id
                                && r.account_id == entry.account_id
                                && r.region == entry.region
                        }) {
                            relationships.push(ResourceRelationship {
                                target_resource_id: vpc_id.to_string(),
                                target_resource_type: "AWS::EC2::VPC".to_string(),
                                relationship_type: RelationshipType::Uses,
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::QuickSight::DataSource"
    }
}

/// Normalizer for QuickSight Dashboard resources
pub struct QuickSightDashboardNormalizer;

impl ResourceNormalizer for QuickSightDashboardNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let dashboard_id = raw_response
            .get("DashboardId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&dashboard_id)
            .to_string();

        let status = raw_response
            .get("Version")
            .and_then(|v| v.get("Status"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut tags = Vec::new();

        // Add version number as a tag
        if let Some(version_num) = raw_response
            .get("Version")
            .and_then(|v| v.get("VersionNumber"))
            .and_then(|v| v.as_i64())
        {
            tags.push(ResourceTag {
                key: "VersionNumber".to_string(),
                value: version_num.to_string(),
            });
        }

        Ok(ResourceEntry {
            resource_id: dashboard_id.clone(),
            resource_type: "AWS::QuickSight::Dashboard".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            display_name: name,
            status,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            query_timestamp,
            account_color: egui::Color32::GRAY,
            region_color: egui::Color32::GRAY,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract relationships from data set ARNs referenced in dashboard version
        if let Some(detailed_props) = &entry.detailed_properties {
            if let Some(version) = detailed_props.get("Version") {
                if let Some(data_set_arns) = version
                    .get("DataSetArnsReferenced")
                    .and_then(|v| v.as_array())
                {
                    for data_set_arn in data_set_arns {
                        if let Some(arn_str) = data_set_arn.as_str() {
                            // Extract data set ID from ARN
                            if let Some(data_set_id) = arn_str.split('/').next_back() {
                                if let Some(_data_set) = all_resources.iter().find(|r| {
                                    r.resource_type == "AWS::QuickSight::DataSet"
                                        && r.resource_id == data_set_id
                                        && r.account_id == entry.account_id
                                        && r.region == entry.region
                                }) {
                                    relationships.push(ResourceRelationship {
                                        target_resource_id: data_set_id.to_string(),
                                        target_resource_type: "AWS::QuickSight::DataSet"
                                            .to_string(),
                                        relationship_type: RelationshipType::Uses,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::QuickSight::Dashboard"
    }
}

/// Normalizer for QuickSight Data Set resources
pub struct QuickSightDataSetNormalizer;

impl ResourceNormalizer for QuickSightDataSetNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let data_set_id = raw_response
            .get("DataSetId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&data_set_id)
            .to_string();

        let status = raw_response
            .get("ImportMode")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut tags = Vec::new();

        // Add import mode as a tag
        if let Some(import_mode) = raw_response.get("ImportMode").and_then(|v| v.as_str()) {
            tags.push(ResourceTag {
                key: "ImportMode".to_string(),
                value: import_mode.to_string(),
            });
        }

        // Add consumed SPICE capacity as a tag
        if let Some(spice_capacity) = raw_response
            .get("ConsumedSpiceCapacityInBytes")
            .and_then(|v| v.as_i64())
        {
            tags.push(ResourceTag {
                key: "ConsumedSpiceCapacityInBytes".to_string(),
                value: spice_capacity.to_string(),
            });
        }

        Ok(ResourceEntry {
            resource_id: data_set_id.clone(),
            resource_type: "AWS::QuickSight::DataSet".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            display_name: name,
            status,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            query_timestamp,
            account_color: egui::Color32::GRAY,
            region_color: egui::Color32::GRAY,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract relationships from physical table map
        if let Some(detailed_props) = &entry.detailed_properties {
            if let Some(physical_tables) = detailed_props
                .get("PhysicalTableMap")
                .and_then(|v| v.as_object())
            {
                for (_, table) in physical_tables {
                    if let Some(data_source_arn) =
                        table.get("DataSourceArn").and_then(|v| v.as_str())
                    {
                        // Extract data source ID from ARN
                        if let Some(data_source_id) = data_source_arn.split('/').next_back() {
                            if let Some(_data_source) = all_resources.iter().find(|r| {
                                r.resource_type == "AWS::QuickSight::DataSource"
                                    && r.resource_id == data_source_id
                                    && r.account_id == entry.account_id
                                    && r.region == entry.region
                            }) {
                                relationships.push(ResourceRelationship {
                                    target_resource_id: data_source_id.to_string(),
                                    target_resource_type: "AWS::QuickSight::DataSource".to_string(),
                                    relationship_type: RelationshipType::Uses,
                                });
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::QuickSight::DataSet"
    }
}
