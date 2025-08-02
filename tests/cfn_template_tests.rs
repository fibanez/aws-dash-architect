#[cfg(test)]
mod tests {
    use awsdash::app::cfn_template::{
        CloudFormationTemplate, DependsOn, Export, Output, Parameter, Resource,
    };
    use awsdash::app::projects::CloudFormationResource;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_template() {
        let template = CloudFormationTemplate::default();

        assert!(template.aws_template_format_version.is_none());
        assert!(template.description.is_none());
        assert!(template.transform.is_none());
        assert!(template.parameters.is_empty());
        assert!(template.mappings.is_empty());
        assert!(template.conditions.is_empty());
        assert!(template.resources.is_empty());
        assert!(template.outputs.is_empty());
        assert!(template.metadata.is_empty());
        assert!(template.rules.is_empty());
    }

    #[test]
    fn test_template_creation() {
        let mut template = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            description: Some("Test CloudFormation Template".to_string()),
            ..Default::default()
        };

        // Add a parameter
        let parameter = Parameter {
            parameter_type: "String".to_string(),
            description: Some("S3 bucket name".to_string()),
            default: Some(Value::String("my-default-bucket".to_string())),
            allowed_values: None,
            allowed_pattern: None,
            constraint_description: None,
            min_length: Some(3),
            max_length: Some(63),
            min_value: None,
            max_value: None,
            no_echo: None,
        };
        template
            .parameters
            .insert("BucketName".to_string(), parameter);

        // Add a resource
        let mut properties = HashMap::new();
        properties.insert(
            "BucketName".to_string(),
            Value::String("my-bucket".to_string()),
        );

        let resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            properties,
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: None,
            update_policy: None,
        };
        template
            .resources
            .insert("MyS3Bucket".to_string(), resource);

        // Add an output
        let export = Export {
            name: Value::String("MyBucketArn".to_string()),
        };

        let output = Output {
            value: Value::String("arn:aws:s3:::my-bucket".to_string()),
            description: Some("ARN of the S3 bucket".to_string()),
            export: Some(export),
            condition: None,
        };
        template.outputs.insert("BucketArn".to_string(), output);

        // Verify template contents
        assert_eq!(
            template.aws_template_format_version,
            Some("2010-09-09".to_string())
        );
        assert_eq!(
            template.description,
            Some("Test CloudFormation Template".to_string())
        );
        assert_eq!(template.parameters.len(), 1);
        assert_eq!(template.resources.len(), 1);
        assert_eq!(template.outputs.len(), 1);
    }

    #[test]
    fn test_from_legacy_resources() {
        let mut resources = Vec::new();

        // Create legacy resource 1
        let mut properties1 = HashMap::new();
        properties1.insert(
            "BucketName".to_string(),
            serde_json::Value::String("my-bucket-1".to_string()),
        );

        let mut resource1 =
            CloudFormationResource::new("Bucket1".to_string(), "AWS::S3::Bucket".to_string());
        resource1.properties = properties1;
        resources.push(resource1);

        // Create legacy resource 2
        let mut properties2 = HashMap::new();
        properties2.insert(
            "BucketName".to_string(),
            serde_json::Value::String("my-bucket-2".to_string()),
        );

        let mut resource2 =
            CloudFormationResource::new("Bucket2".to_string(), "AWS::S3::Bucket".to_string());
        resource2.properties = properties2;
        resources.push(resource2);

        // Convert to template
        let template = CloudFormationTemplate::from_legacy_resources(resources);

        // Verify conversion
        assert_eq!(
            template.aws_template_format_version,
            Some("2010-09-09".to_string())
        );
        assert_eq!(template.resources.len(), 2);

        let bucket1 = template.resources.get("Bucket1").unwrap();
        assert_eq!(bucket1.resource_type, "AWS::S3::Bucket");
        assert_eq!(
            bucket1.properties.get("BucketName").unwrap(),
            &Value::String("my-bucket-1".to_string())
        );

        let bucket2 = template.resources.get("Bucket2").unwrap();
        assert_eq!(bucket2.resource_type, "AWS::S3::Bucket");
        assert_eq!(
            bucket2.properties.get("BucketName").unwrap(),
            &Value::String("my-bucket-2".to_string())
        );
    }

    #[test]
    fn test_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("template.json");

        let mut template = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            description: Some("Test Template".to_string()),
            ..Default::default()
        };

        // Add a resource
        let mut properties = HashMap::new();
        properties.insert(
            "BucketName".to_string(),
            Value::String("test-bucket".to_string()),
        );

        let resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            properties,
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: None,
            update_policy: None,
        };
        template
            .resources
            .insert("TestBucket".to_string(), resource);

        // Save to file
        let result = template.to_file(&template_path);
        assert!(result.is_ok());

        // Verify file exists and content is correct
        assert!(template_path.exists());

        let content = fs::read_to_string(&template_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed["AwsTemplateFormatVersion"], "2010-09-09");
        assert_eq!(parsed["Description"], "Test Template");
        assert!(parsed["Resources"]["TestBucket"].is_object());
        assert_eq!(parsed["Resources"]["TestBucket"]["Type"], "AWS::S3::Bucket");
    }

    #[test]
    fn test_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("template.json");

        // Create a test template file
        let template_content = r#"{
            "AwsTemplateFormatVersion": "2010-09-09",
            "Description": "Test Template",
            "Resources": {
                "TestBucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "BucketName": "test-bucket"
                    }
                }
            }
        }"#;

        fs::write(&template_path, template_content).unwrap();

        // Load from file
        let loaded_template = CloudFormationTemplate::from_file(&template_path).unwrap();

        // Verify loaded content
        assert_eq!(
            loaded_template.aws_template_format_version,
            Some("2010-09-09".to_string())
        );
        assert_eq!(
            loaded_template.description,
            Some("Test Template".to_string())
        );
        assert_eq!(loaded_template.resources.len(), 1);

        let bucket = loaded_template.resources.get("TestBucket").unwrap();
        assert_eq!(bucket.resource_type, "AWS::S3::Bucket");
        assert_eq!(
            bucket.properties.get("BucketName").unwrap(),
            &Value::String("test-bucket".to_string())
        );
    }

    #[test]
    fn test_yaml_file_support() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("template.yaml");

        // Create a valid YAML file
        let yaml_content = r#"
AWSTemplateFormatVersion: '2010-09-09'
Description: Test YAML Template
Resources:
  MyBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: test-bucket
"#;
        fs::write(&template_path, yaml_content).unwrap();

        // Try to load YAML file - it should work now
        let result = CloudFormationTemplate::from_file(&template_path);
        assert!(result.is_ok());

        let template = result.unwrap();
        assert_eq!(template.description, Some("Test YAML Template".to_string()));
        assert_eq!(template.resources.len(), 1);
    }

    #[test]
    fn test_invalid_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("template.json");

        // Create an invalid JSON file
        fs::write(&template_path, "{ invalid json }").unwrap();

        // Try to load invalid JSON
        let result = CloudFormationTemplate::from_file(&template_path);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to parse JSON"));
    }

    #[test]
    fn test_resource_with_dependencies() {
        let mut template = CloudFormationTemplate::default();

        // Add a resource with dependencies
        let mut properties = HashMap::new();
        properties.insert(
            "BucketName".to_string(),
            Value::String("test-bucket".to_string()),
        );

        let resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            properties,
            depends_on: Some(DependsOn::Multiple(vec![
                "MyRole".to_string(),
                "MyPolicy".to_string(),
            ])),
            condition: Some("CreateBucket".to_string()),
            metadata: None,
            deletion_policy: Some("Retain".to_string()),
            update_replace_policy: Some("Delete".to_string()),
            creation_policy: None,
            update_policy: None,
        };
        template
            .resources
            .insert("TestBucket".to_string(), resource);

        // Verify the resource
        let bucket = template.resources.get("TestBucket").unwrap();
        assert!(bucket.depends_on.is_some());

        match bucket.depends_on.as_ref().unwrap() {
            DependsOn::Multiple(deps) => {
                assert_eq!(deps.len(), 2);
                assert!(deps.contains(&"MyRole".to_string()));
                assert!(deps.contains(&"MyPolicy".to_string()));
            }
            _ => panic!("Expected DependsOn::Multiple"),
        }

        assert_eq!(bucket.condition, Some("CreateBucket".to_string()));
        assert_eq!(bucket.deletion_policy, Some("Retain".to_string()));
        assert_eq!(bucket.update_replace_policy, Some("Delete".to_string()));
    }

    #[test]
    fn test_resource_with_creation_and_update_policies() {
        let mut template = CloudFormationTemplate::default();

        // Add a resource with CreationPolicy and UpdatePolicy
        let mut properties = HashMap::new();
        properties.insert(
            "LaunchConfigurationName".to_string(),
            Value::String("my-launch-config".to_string()),
        );

        let creation_policy = serde_json::json!({
            "ResourceSignal": {
                "Count": 2,
                "Timeout": "PT5M"
            }
        });

        let update_policy = serde_json::json!({
            "AutoScalingRollingUpdate": {
                "MaxBatchSize": 1,
                "MinInstancesInService": 1,
                "PauseTime": "PT5M"
            }
        });

        let resource = Resource {
            resource_type: "AWS::AutoScaling::AutoScalingGroup".to_string(),
            properties,
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: Some(creation_policy.clone()),
            update_policy: Some(update_policy.clone()),
        };
        template
            .resources
            .insert("MyAutoScalingGroup".to_string(), resource);

        // Verify the resource
        let asg = template.resources.get("MyAutoScalingGroup").unwrap();
        assert_eq!(asg.creation_policy, Some(creation_policy));
        assert_eq!(asg.update_policy, Some(update_policy));
    }

    #[test]
    fn test_resource_serialization_with_all_attributes() {
        let mut properties = HashMap::new();
        properties.insert(
            "BucketName".to_string(),
            Value::String("test-bucket".to_string()),
        );

        let metadata = serde_json::json!({
            "Designer": {
                "id": "abc123"
            }
        });

        let resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            properties: properties.clone(),
            depends_on: Some(DependsOn::Single("MyRole".to_string())),
            condition: Some("CreateBucket".to_string()),
            metadata: Some(metadata.clone()),
            deletion_policy: Some("Retain".to_string()),
            update_replace_policy: Some("Delete".to_string()),
            creation_policy: None,
            update_policy: None,
        };

        // Serialize to JSON
        let json = serde_json::to_value(&resource).unwrap();

        // Verify all fields are correctly serialized
        assert_eq!(json["Type"], "AWS::S3::Bucket");
        assert_eq!(json["Properties"]["BucketName"], "test-bucket");
        assert_eq!(json["DependsOn"], "MyRole");
        assert_eq!(json["Condition"], "CreateBucket");
        assert_eq!(json["Metadata"], metadata);
        assert_eq!(json["DeletionPolicy"], "Retain");
        assert_eq!(json["UpdateReplacePolicy"], "Delete");
        assert!(!json.as_object().unwrap().contains_key("CreationPolicy"));
        assert!(!json.as_object().unwrap().contains_key("UpdatePolicy"));
    }
}
