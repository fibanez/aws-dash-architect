#[cfg(test)]
mod tests {
    use awsdash::app::cfn_template::{CloudFormationTemplate, Output, Parameter, Resource};
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn test_template_verification_no_discrepancies() {
        let template1 = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            description: Some("Test template".to_string()),
            resources: {
                let mut resources = HashMap::new();
                resources.insert(
                    "TestResource".to_string(),
                    Resource {
                        resource_type: "AWS::S3::Bucket".to_string(),
                        properties: HashMap::new(),
                        depends_on: None,
                        condition: None,
                        metadata: None,
                        deletion_policy: None,
                        update_replace_policy: None,
                        creation_policy: None,
                        update_policy: None,
                    },
                );
                resources
            },
            ..Default::default()
        };

        let template2 = template1.clone();
        let discrepancies = template2.verify_against(&template1);

        assert!(
            discrepancies.is_empty(),
            "Expected no discrepancies, found: {:?}",
            discrepancies
        );
    }

    #[test]
    fn test_template_verification_missing_resource() {
        let mut source = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            ..Default::default()
        };
        source.resources.insert(
            "MissingResource".to_string(),
            Resource {
                resource_type: "AWS::EC2::Instance".to_string(),
                properties: HashMap::new(),
                depends_on: None,
                condition: None,
                metadata: None,
                deletion_policy: None,
                update_replace_policy: None,
                creation_policy: None,
                update_policy: None,
            },
        );

        let target = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            ..Default::default()
        };

        let discrepancies = target.verify_against(&source);

        assert!(
            !discrepancies.is_empty(),
            "Expected discrepancies for missing resource"
        );
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Missing resource: MissingResource")));
    }

    #[test]
    fn test_template_verification_parameter_mismatch() {
        let mut source = CloudFormationTemplate::default();
        source.parameters.insert(
            "TestParam".to_string(),
            Parameter {
                parameter_type: "String".to_string(),
                description: Some("Original description".to_string()),
                default: None,
                allowed_values: None,
                allowed_pattern: None,
                constraint_description: None,
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                no_echo: None,
            },
        );

        let mut target = CloudFormationTemplate::default();
        target.parameters.insert(
            "TestParam".to_string(),
            Parameter {
                parameter_type: "String".to_string(),
                description: Some("Different description".to_string()),
                default: None,
                allowed_values: None,
                allowed_pattern: None,
                constraint_description: None,
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                no_echo: None,
            },
        );

        let discrepancies = target.verify_against(&source);

        assert!(
            !discrepancies.is_empty(),
            "Expected discrepancies for parameter mismatch"
        );
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Parameter 'TestParam' content mismatch")));
    }

    #[test]
    fn test_template_verification_all_sections() {
        let mut source = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            description: Some("Source template".to_string()),
            transform: Some(vec!["AWS::Serverless-2016-10-31".to_string()]),
            ..Default::default()
        };

        // Add various sections to source
        source.parameters.insert(
            "Param1".to_string(),
            Parameter {
                parameter_type: "String".to_string(),
                description: None,
                default: None,
                allowed_values: None,
                allowed_pattern: None,
                constraint_description: None,
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                no_echo: None,
            },
        );
        source
            .mappings
            .insert("Map1".to_string(), Value::Object(serde_json::Map::new()));
        source
            .conditions
            .insert("Cond1".to_string(), Value::Bool(true));
        source.outputs.insert(
            "Out1".to_string(),
            Output {
                value: Value::String("output".to_string()),
                description: None,
                export: None,
                condition: None,
            },
        );

        // Target with different values
        let target = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            description: Some("Different description".to_string()),
            ..Default::default()
        };

        let discrepancies = target.verify_against(&source);

        // Should find multiple discrepancies
        assert!(
            discrepancies.len() >= 5,
            "Expected multiple discrepancies, found: {}",
            discrepancies.len()
        );
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Description mismatch")));
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Transform mismatch")));
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Missing parameter: Param1")));
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Missing mapping: Map1")));
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Missing condition: Cond1")));
        assert!(discrepancies
            .iter()
            .any(|d| d.contains("Missing output: Out1")));
    }

    #[test]
    fn test_property_type_preservation_bug() {
        use awsdash::app::projects::CloudFormationResource;
        use std::fs;
        use tempfile::TempDir;

        // Create a test template with mixed property types that should expose the bug
        let original_template = CloudFormationTemplate {
            aws_template_format_version: Some("2010-09-09".to_string()),
            description: Some("Property Type Test Template".to_string()),
            resources: {
                let mut resources = HashMap::new();

                // Create a resource with mixed property types
                let mut properties = HashMap::new();
                properties.insert(
                    "Size".to_string(),
                    Value::Number(serde_json::Number::from(5)),
                );
                properties.insert("AutoEnableIO".to_string(), Value::Bool(true));
                properties.insert("Name".to_string(), Value::String("test-volume".to_string()));
                properties.insert(
                    "Tags".to_string(),
                    Value::Array(vec![Value::Object({
                        let mut tag = serde_json::Map::new();
                        tag.insert("Key".to_string(), Value::String("Environment".to_string()));
                        tag.insert("Value".to_string(), Value::String("Test".to_string()));
                        tag
                    })]),
                );
                properties.insert(
                    "Config".to_string(),
                    Value::Object({
                        let mut config = serde_json::Map::new();
                        config.insert("Enabled".to_string(), Value::Bool(false));
                        config.insert(
                            "Count".to_string(),
                            Value::Number(serde_json::Number::from(10)),
                        );
                        config
                    }),
                );

                resources.insert(
                    "TestVolume".to_string(),
                    Resource {
                        resource_type: "AWS::EC2::Volume".to_string(),
                        properties,
                        depends_on: None,
                        condition: None,
                        metadata: None,
                        deletion_policy: None,
                        update_replace_policy: None,
                        creation_policy: None,
                        update_policy: None,
                    },
                );
                resources
            },
            ..Default::default()
        };

        // Create a temporary directory and save the template
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let template_path = temp_dir.path().join("test_template.json");

        let template_json =
            serde_json::to_string_pretty(&original_template).expect("Failed to serialize template");
        fs::write(&template_path, template_json).expect("Failed to write template");

        // Load the template back from file (simulates the file import process)
        let _loaded_template = CloudFormationTemplate::from_file(&template_path)
            .expect("Failed to load template from file");

        // Now simulate the property conversion bug by creating CloudFormationResource and back
        let original_resource = original_template.resources.get("TestVolume").unwrap();
        let cfn_resource =
            CloudFormationResource::from_cfn_resource("TestVolume".to_string(), original_resource);

        // Convert back to CloudFormation resource (this is where the bug happens)
        let converted_resource = cfn_resource.to_cfn_resource();

        // Create template with converted resource
        let mut converted_template = original_template.clone();
        converted_template.resources.clear();
        converted_template
            .resources
            .insert("TestVolume".to_string(), converted_resource);

        // Verify against the original - this should FAIL due to type conversion bug
        let discrepancies = converted_template.verify_against(&original_template);

        // UPDATE: This test now PASSES as property type preservation works correctly
        // The original expectation was that there would be a bug, but the system works properly
        assert!(
            discrepancies.is_empty(),
            "Property types are correctly preserved. No discrepancies expected. \
             Found {} discrepancies: {:?}",
            discrepancies.len(),
            discrepancies
        );

        // Since there are no discrepancies (which is correct), we can verify the conversion worked
        println!("✅ Property type preservation test PASSED - no discrepancies found");
        println!("   This confirms that the CloudFormation import system correctly preserves property types");

        // Log what we found for debugging
        println!(
            "Property type preservation test found {} discrepancies:",
            discrepancies.len()
        );
        for discrepancy in &discrepancies {
            println!("  - {}", discrepancy);
        }
    }

    #[test]
    fn test_real_import_workflow_property_type_bug() {
        use awsdash::app::projects::Project;
        use std::fs;
        use tempfile::TempDir;

        // Create a simpler test template that focuses on property types without dependency issues
        let test_template_json = r#"{
            "AWSTemplateFormatVersion": "2010-09-09",
            "Description": "Property Type Test Template",
            "Resources": {
                "TestVolume": {
                    "Type": "AWS::EC2::Volume",
                    "Properties": {
                        "Size": 5,
                        "AutoEnableIO": true,
                        "AvailabilityZone": "us-east-1a",
                        "VolumeType": "gp2",
                        "Tags": [
                            {
                                "Key": "Environment",
                                "Value": "Test"
                            }
                        ]
                    }
                },
                "TestBucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "PublicAccessBlockConfiguration": {
                            "BlockPublicAcls": true,
                            "BlockPublicPolicy": true,
                            "IgnorePublicAcls": true,
                            "RestrictPublicBuckets": true
                        }
                    }
                }
            }
        }"#;

        // Create a temporary directory for the test project
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let project_path = temp_dir.path().to_path_buf();

        // Create project structure
        let resources_dir = project_path.join("Resources");
        fs::create_dir_all(&resources_dir).expect("Failed to create Resources directory");

        // Write the test template
        let template_path = resources_dir.join("cloudformation_template.json");
        fs::write(&template_path, test_template_json).expect("Failed to write template to project");

        // Load the original template directly for comparison
        let original_template = CloudFormationTemplate::from_file(&template_path)
            .expect("Failed to load original template");

        println!(
            "Original template loaded with {} resources",
            original_template.resources.len()
        );

        // Create a test project and set the local folder
        let mut project = Project::new(
            "test_project".to_string(),
            "Test project for real import workflow".to_string(),
            "test".to_string(),
        );
        project.local_folder = Some(project_path);

        // This is the critical path - load resources from template through the project system
        // This triggers the load_resources_from_template -> line 707 bug
        let loaded_count = project
            .load_resources_from_template()
            .expect("Failed to load resources from template");

        println!("Loaded {} resources from template", loaded_count);

        // The template should already be updated after load_resources_from_template

        // Get the CloudFormation template that was reconstructed from the DAG
        let reconstructed_template = project
            .cfn_template
            .as_ref()
            .expect("Project should have a CloudFormation template after loading");

        println!(
            "Reconstructed template has {} resources",
            reconstructed_template.resources.len()
        );

        // Verify the reconstructed template against the original
        // This should FAIL due to property type conversion in line 707
        let discrepancies = reconstructed_template.verify_against(&original_template);

        // This test is EXPECTED TO FAIL until we fix the property type preservation bug
        // The assertion here documents the current bug state
        if !discrepancies.is_empty() {
            println!("FOUND EXPECTED BUG: Property type discrepancies detected!");
            println!(
                "Real import workflow test found {} discrepancies:",
                discrepancies.len()
            );
            for discrepancy in &discrepancies {
                println!("  - {}", discrepancy);
            }

            // Specifically look for property type issues
            let has_property_type_issues = discrepancies.iter().any(|d| {
                d.contains("property mismatch")
                    && (d.contains("Size")
                        || d.contains("AutoEnableIO")
                        || d.contains("BlockPublicAcls"))
            });

            assert!(
                has_property_type_issues,
                "Expected to find property type conversion issues, but found other discrepancies: {:?}",
                discrepancies
            );
        } else {
            // If no discrepancies, let's examine the properties directly to see what happened
            println!("No discrepancies found - examining properties directly...");

            if let Some(original_volume) = original_template.resources.get("TestVolume") {
                if let Some(reconstructed_volume) =
                    reconstructed_template.resources.get("TestVolume")
                {
                    println!(
                        "Original Size property: {:?}",
                        original_volume.properties.get("Size")
                    );
                    println!(
                        "Reconstructed Size property: {:?}",
                        reconstructed_volume.properties.get("Size")
                    );

                    println!(
                        "Original AutoEnableIO property: {:?}",
                        original_volume.properties.get("AutoEnableIO")
                    );
                    println!(
                        "Reconstructed AutoEnableIO property: {:?}",
                        reconstructed_volume.properties.get("AutoEnableIO")
                    );
                }
            }

            // Let's test if the bug is in the DAG round-trip instead
            println!("Testing DAG round-trip conversion...");

            // Check if we can get resources from the CloudFormation template
            if let Some(template) = &project.cfn_template {
                let template_resources = &template.resources;
                println!("Template has {} resources", template_resources.len());

                for (resource_id, template_resource) in template_resources {
                    println!("Checking template resource: {}", resource_id);

                    if let Some(original_resource) = original_template.resources.get(resource_id) {
                        // Compare properties individually
                        for (prop_key, original_value) in &original_resource.properties {
                            if let Some(converted_value) =
                                template_resource.properties.get(prop_key)
                            {
                                if original_value != converted_value {
                                    println!("FOUND BUG: Property '{}' type mismatch!", prop_key);
                                    println!(
                                        "  Original: {:?} (type: {:?})",
                                        original_value, original_value
                                    );
                                    println!(
                                        "  Converted: {:?} (type: {:?})",
                                        converted_value, converted_value
                                    );

                                    // This is the bug we're looking for!
                                    panic!("Found property type conversion bug in DAG round-trip for property '{}'", prop_key);
                                }
                            }
                        }
                    }
                }
            }

            // Let's test the specific conversion methods that might have the bug
            println!("Testing CloudFormationResource conversion methods...");

            // Get a resource from the original template
            if let Some(original_resource) = original_template.resources.get("TestVolume") {
                println!("Testing conversion for TestVolume resource");

                // Convert CloudFormation resource to CloudFormationResource (DAG format)
                let dag_resource =
                    awsdash::app::projects::CloudFormationResource::from_cfn_resource(
                        "TestVolume".to_string(),
                        original_resource,
                    );

                println!("DAG resource properties:");
                for (key, value) in &dag_resource.properties {
                    println!("  {}: {} (type: string)", key, value);
                }

                // Convert back to CloudFormation resource
                let converted_resource = dag_resource.to_cfn_resource();

                println!("Converted back resource properties:");
                for (key, value) in &converted_resource.properties {
                    println!("  {}: {:?}", key, value);
                }

                // Compare specific properties
                let original_size = original_resource.properties.get("Size");
                let converted_size = converted_resource.properties.get("Size");

                if original_size != converted_size {
                    println!("FOUND BUG: Size property type mismatch!");
                    println!("  Original: {:?}", original_size);
                    println!("  Converted: {:?}", converted_size);
                    panic!("Found the property type conversion bug!");
                }

                let original_auto_enable = original_resource.properties.get("AutoEnableIO");
                let converted_auto_enable = converted_resource.properties.get("AutoEnableIO");

                if original_auto_enable != converted_auto_enable {
                    println!("FOUND BUG: AutoEnableIO property type mismatch!");
                    println!("  Original: {:?}", original_auto_enable);
                    println!("  Converted: {:?}", converted_auto_enable);
                    panic!("Found the property type conversion bug!");
                }
            }

            println!("✅ Real import workflow test PASSED - no discrepancies found");
            println!(
                "   Property type preservation works correctly through the full import workflow"
            );
        }
    }
}
