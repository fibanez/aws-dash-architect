use awsdash::app::cfn_dag::ResourceDag;
use awsdash::app::cfn_template::{CloudFormationTemplate, DependsOn, Resource};
use awsdash::app::projects::CloudFormationResource;
use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod dependency_validation_tests {
    use super::*;

    fn create_test_cfn_resource(resource_id: &str, resource_type: &str) -> CloudFormationResource {
        CloudFormationResource {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            properties: HashMap::new(),
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: None,
            update_policy: None,
        }
    }

    fn create_test_template() -> CloudFormationTemplate {
        let mut template = CloudFormationTemplate::default();

        // Add some test resources with dependencies
        let mut s3_bucket = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            ..Default::default()
        };
        s3_bucket.properties.insert(
            "BucketName".to_string(),
            Value::String("test-bucket".to_string()),
        );

        let mut iam_role = Resource {
            resource_type: "AWS::IAM::Role".to_string(),
            ..Default::default()
        };
        iam_role.properties.insert(
            "RoleName".to_string(),
            Value::String("test-role".to_string()),
        );

        let mut lambda_function = Resource {
            resource_type: "AWS::Lambda::Function".to_string(),
            depends_on: Some(DependsOn::Multiple(vec![
                "MyBucket".to_string(),
                "MyRole".to_string(),
            ])),
            ..Default::default()
        };

        // Add Ref to IAM role in Lambda properties
        let mut lambda_props = HashMap::new();
        lambda_props.insert(
            "FunctionName".to_string(),
            Value::String("test-function".to_string()),
        );
        lambda_props.insert(
            "Role".to_string(),
            Value::Object({
                let mut ref_obj = serde_json::Map::new();
                ref_obj.insert("Ref".to_string(), Value::String("MyRole".to_string()));
                ref_obj
            }),
        );

        // Add GetAtt to S3 bucket in Lambda environment
        lambda_props.insert(
            "Environment".to_string(),
            Value::Object({
                let mut env_obj = serde_json::Map::new();
                env_obj.insert(
                    "Variables".to_string(),
                    Value::Object({
                        let mut vars = serde_json::Map::new();
                        vars.insert(
                            "BUCKET_ARN".to_string(),
                            Value::Object({
                                let mut getatt = serde_json::Map::new();
                                getatt.insert(
                                    "Fn::GetAtt".to_string(),
                                    Value::Array(vec![
                                        Value::String("MyBucket".to_string()),
                                        Value::String("Arn".to_string()),
                                    ]),
                                );
                                getatt
                            }),
                        );
                        vars
                    }),
                );
                env_obj
            }),
        );

        lambda_function.properties = lambda_props;

        template.resources.insert("MyBucket".to_string(), s3_bucket);
        template.resources.insert("MyRole".to_string(), iam_role);
        template
            .resources
            .insert("MyFunction".to_string(), lambda_function);

        // Add a parameter for testing
        template.parameters.insert(
            "Environment".to_string(),
            serde_json::from_str(r#"{"Type": "String", "Default": "dev"}"#).unwrap(),
        );

        template
    }

    #[test]
    fn test_validate_dependencies_success() {
        let template = create_test_template();
        let errors = template.validate_dependencies();

        // Should have no errors since all dependencies exist
        assert!(
            errors.is_empty(),
            "Expected no dependency errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_missing_depends_on_reference() {
        let mut template = create_test_template();

        // Add a resource with DependsOn pointing to non-existent resource
        let bad_resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            depends_on: Some(DependsOn::Single("NonExistentResource".to_string())),
            ..Default::default()
        };
        template
            .resources
            .insert("BadResource".to_string(), bad_resource);

        let errors = template.validate_dependencies();

        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("DependsOn reference to non-existent resource")));
        assert!(errors.iter().any(|e| e.contains("NonExistentResource")));
    }

    #[test]
    fn test_validate_self_dependency() {
        let mut template = create_test_template();

        // Add a resource that depends on itself
        let self_dep_resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            depends_on: Some(DependsOn::Single("SelfDepResource".to_string())),
            ..Default::default()
        };
        template
            .resources
            .insert("SelfDepResource".to_string(), self_dep_resource);

        let errors = template.validate_dependencies();

        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("cannot depend on itself")));
    }

    #[test]
    fn test_validate_missing_ref_target() {
        let mut template = create_test_template();

        // Add a resource with Ref to non-existent resource
        let mut bad_resource = Resource {
            resource_type: "AWS::Lambda::Function".to_string(),
            ..Default::default()
        };
        bad_resource.properties.insert(
            "Role".to_string(),
            Value::Object({
                let mut ref_obj = serde_json::Map::new();
                ref_obj.insert(
                    "Ref".to_string(),
                    Value::String("NonExistentRole".to_string()),
                );
                ref_obj
            }),
        );
        template
            .resources
            .insert("BadFunction".to_string(), bad_resource);

        let errors = template.validate_dependencies();

        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("Ref to non-existent resource or parameter")));
        assert!(errors.iter().any(|e| e.contains("NonExistentRole")));
    }

    #[test]
    fn test_validate_missing_getatt_target() {
        let mut template = create_test_template();

        // Add a resource with GetAtt to non-existent resource
        let mut bad_resource = Resource {
            resource_type: "AWS::Lambda::Function".to_string(),
            ..Default::default()
        };
        bad_resource.properties.insert(
            "Environment".to_string(),
            Value::Object({
                let mut env_obj = serde_json::Map::new();
                env_obj.insert(
                    "Variables".to_string(),
                    Value::Object({
                        let mut vars = serde_json::Map::new();
                        vars.insert(
                            "ARN".to_string(),
                            Value::Object({
                                let mut getatt = serde_json::Map::new();
                                getatt.insert(
                                    "Fn::GetAtt".to_string(),
                                    Value::Array(vec![
                                        Value::String("NonExistentResource".to_string()),
                                        Value::String("Arn".to_string()),
                                    ]),
                                );
                                getatt
                            }),
                        );
                        vars
                    }),
                );
                env_obj
            }),
        );
        template
            .resources
            .insert("BadFunction2".to_string(), bad_resource);

        let errors = template.validate_dependencies();

        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("Fn::GetAtt reference to non-existent resource")));
        assert!(errors.iter().any(|e| e.contains("NonExistentResource")));
    }

    #[test]
    fn test_validate_missing_condition_reference() {
        let mut template = create_test_template();

        // Add a resource with condition reference to non-existent condition
        let conditional_resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            condition: Some("NonExistentCondition".to_string()),
            ..Default::default()
        };
        template
            .resources
            .insert("ConditionalResource".to_string(), conditional_resource);

        let errors = template.validate_dependencies();

        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("references non-existent condition")));
        assert!(errors.iter().any(|e| e.contains("NonExistentCondition")));
    }

    #[test]
    fn test_detect_circular_dependencies() {
        let mut template = CloudFormationTemplate::default();

        // Create circular dependency: A -> B -> C -> A
        let resource_a = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            depends_on: Some(DependsOn::Single("ResourceC".to_string())),
            ..Default::default()
        };

        let resource_b = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            depends_on: Some(DependsOn::Single("ResourceA".to_string())),
            ..Default::default()
        };

        let resource_c = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            depends_on: Some(DependsOn::Single("ResourceB".to_string())),
            ..Default::default()
        };

        template
            .resources
            .insert("ResourceA".to_string(), resource_a);
        template
            .resources
            .insert("ResourceB".to_string(), resource_b);
        template
            .resources
            .insert("ResourceC".to_string(), resource_c);

        let errors = template.detect_circular_dependencies();

        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("Circular dependency detected")));
    }

    #[test]
    fn test_extract_implicit_dependencies() {
        let template = create_test_template();
        let lambda_resource = template.resources.get("MyFunction").unwrap();

        let implicit_deps = template.extract_implicit_dependencies(lambda_resource);

        // Should find dependencies from Ref and GetAtt
        assert!(implicit_deps.contains(&"MyRole".to_string()));
        assert!(implicit_deps.contains(&"MyBucket".to_string()));
    }

    #[test]
    fn test_smart_dependency_resolution_in_order() {
        let _template = create_test_template();
        let mut dag = ResourceDag::new();

        // Convert CloudFormation resources to DAG resources in correct order
        let bucket_resource = create_test_cfn_resource("MyBucket", "AWS::S3::Bucket");
        let role_resource = create_test_cfn_resource("MyRole", "AWS::IAM::Role");
        let function_resource = create_test_cfn_resource("MyFunction", "AWS::Lambda::Function");

        // Add resources in correct dependency order
        let bucket_result = dag.add_resource_smart(bucket_resource, vec![]);
        assert!(bucket_result.is_ok());
        assert_eq!(bucket_result.unwrap(), 1); // 1 resource added

        let role_result = dag.add_resource_smart(role_resource, vec![]);
        assert!(role_result.is_ok());
        assert_eq!(role_result.unwrap(), 1); // 1 resource added

        let function_result = dag.add_resource_smart(
            function_resource,
            vec!["MyBucket".to_string(), "MyRole".to_string()],
        );
        assert!(function_result.is_ok());
        assert_eq!(function_result.unwrap(), 1); // 1 resource added

        // Verify all resources are in the DAG
        assert_eq!(dag.get_resources().len(), 3);
        assert_eq!(dag.get_deferred_count(), 0);
    }

    #[test]
    fn test_smart_dependency_resolution_out_of_order() {
        let _template = create_test_template();
        let mut dag = ResourceDag::new();

        // Convert CloudFormation resources to DAG resources
        let bucket_resource = create_test_cfn_resource("MyBucket", "AWS::S3::Bucket");
        let role_resource = create_test_cfn_resource("MyRole", "AWS::IAM::Role");
        let function_resource = create_test_cfn_resource("MyFunction", "AWS::Lambda::Function");

        // Add resources in WRONG dependency order (function first)
        let function_result = dag.add_resource_smart(
            function_resource,
            vec!["MyBucket".to_string(), "MyRole".to_string()],
        );
        assert!(function_result.is_ok());
        assert_eq!(function_result.unwrap(), 0); // 0 resources added (deferred)
        assert_eq!(dag.get_deferred_count(), 1); // 1 resource deferred

        // Add bucket - should process bucket and try function
        let bucket_result = dag.add_resource_smart(bucket_resource, vec![]);
        assert!(bucket_result.is_ok());
        assert_eq!(bucket_result.unwrap(), 1); // 1 resource added (bucket only, function still needs role)
        assert_eq!(dag.get_deferred_count(), 1); // function still deferred

        // Add role - should process role and function
        let role_result = dag.add_resource_smart(role_resource, vec![]);
        assert!(role_result.is_ok());
        assert_eq!(role_result.unwrap(), 2); // 2 resources added (role + function from queue)
        assert_eq!(dag.get_deferred_count(), 0); // no more deferred

        // Verify all resources are in the DAG
        assert_eq!(dag.get_resources().len(), 3);

        // Verify dependencies are correct
        let function_deps = dag.get_dependencies("MyFunction");
        assert!(function_deps.contains(&"MyBucket".to_string()));
        assert!(function_deps.contains(&"MyRole".to_string()));
    }

    #[test]
    fn test_smart_dependency_resolution_with_template() {
        let template = create_test_template();
        let mut dag = ResourceDag::new();

        // Create resources in random order
        let resources = vec![
            create_test_cfn_resource("MyFunction", "AWS::Lambda::Function"),
            create_test_cfn_resource("MyBucket", "AWS::S3::Bucket"),
            create_test_cfn_resource("MyRole", "AWS::IAM::Role"),
        ];

        // Add all resources using template-based resolution
        let result = dag.add_resources_from_template(&template, resources);
        assert!(result.is_ok());

        let (added_count, warnings) = result.unwrap();
        assert_eq!(added_count, 3); // All 3 resources should be added
        assert!(warnings.is_empty()); // No warnings expected for valid template

        // Verify all resources are in the DAG
        assert_eq!(dag.get_resources().len(), 3);
        assert_eq!(dag.get_deferred_count(), 0);

        // Verify dependencies are extracted correctly
        let function_deps = dag.get_dependencies("MyFunction");
        assert!(function_deps.contains(&"MyBucket".to_string()));
        assert!(function_deps.contains(&"MyRole".to_string()));
    }

    #[test]
    fn test_smart_dependency_resolution_unresolvable_dependencies() {
        let _template = CloudFormationTemplate::default();
        let mut dag = ResourceDag::new();

        // Create a resource with dependency on non-existent resource
        let orphan_resource = create_test_cfn_resource("OrphanResource", "AWS::S3::Bucket");

        // Test using add_resource_smart directly with unresolvable dependencies
        let result = dag.add_resource_smart(
            orphan_resource.clone(),
            vec!["NonExistentResource".to_string()],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // 0 resources added (deferred)
        assert_eq!(dag.get_deferred_count(), 1); // 1 resource deferred

        // Verify deferred resource details
        let deferred_ids = dag.get_deferred_resource_ids();
        assert_eq!(deferred_ids.len(), 1);
        assert_eq!(deferred_ids[0], "OrphanResource");

        // Try to process deferred queue manually - should not resolve
        let processed = dag.process_deferred_queue();
        assert!(processed.is_ok());
        assert_eq!(processed.unwrap(), 0); // 0 resources processed
        assert_eq!(dag.get_deferred_count(), 1); // still 1 deferred

        // Test template-based resolution with valid template (should work fine)
        let valid_template = create_test_template();
        let mut dag2 = ResourceDag::new();
        let valid_resources = vec![
            create_test_cfn_resource("MyBucket", "AWS::S3::Bucket"),
            create_test_cfn_resource("MyRole", "AWS::IAM::Role"),
            create_test_cfn_resource("MyFunction", "AWS::Lambda::Function"),
        ];

        let result2 = dag2.add_resources_from_template(&valid_template, valid_resources);
        assert!(result2.is_ok());

        let (added_count2, warnings2) = result2.unwrap();
        assert_eq!(added_count2, 3); // All 3 resources should be added
        assert!(warnings2.is_empty()); // No warnings for valid template
        assert_eq!(dag2.get_deferred_count(), 0); // No deferred resources
    }

    #[test]
    fn test_smart_dependency_resolution_circular_dependencies() {
        let mut dag = ResourceDag::new();

        // Create resources with circular dependency: A -> B -> A
        let resource_a = create_test_cfn_resource("ResourceA", "AWS::S3::Bucket");
        let resource_b = create_test_cfn_resource("ResourceB", "AWS::S3::Bucket");

        // Add A depending on B
        let result_a = dag.add_resource_smart(resource_a, vec!["ResourceB".to_string()]);
        assert!(result_a.is_ok());
        assert_eq!(result_a.unwrap(), 0); // Deferred due to missing dependency

        // Add B depending on A - should detect cycle when trying to process queue
        let result_b = dag.add_resource_smart(resource_b, vec!["ResourceA".to_string()]);
        assert!(result_b.is_ok());
        assert_eq!(result_b.unwrap(), 0); // Deferred due to missing dependency

        // Both resources should be deferred with unresolvable circular dependency
        assert_eq!(dag.get_deferred_count(), 2);
        assert_eq!(dag.get_resources().len(), 0);
    }

    #[test]
    fn test_deferred_queue_operations() {
        let mut dag = ResourceDag::new();

        // Add a resource that will be deferred
        let resource = create_test_cfn_resource("TestResource", "AWS::S3::Bucket");

        let result = dag.add_resource_smart(resource, vec!["MissingDependency".to_string()]);
        assert!(result.is_ok());
        assert_eq!(dag.get_deferred_count(), 1);

        // Test deferred resource IDs
        let deferred_ids = dag.get_deferred_resource_ids();
        assert_eq!(deferred_ids.len(), 1);
        assert_eq!(deferred_ids[0], "TestResource");
    }

    #[test]
    fn test_aws_pseudo_parameters() {
        let mut template = CloudFormationTemplate::default();

        // Add a resource with AWS pseudo parameter references
        let mut resource = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            ..Default::default()
        };
        resource.properties.insert(
            "BucketName".to_string(),
            Value::Object({
                let mut ref_obj = serde_json::Map::new();
                ref_obj.insert("Ref".to_string(), Value::String("AWS::Region".to_string()));
                ref_obj
            }),
        );
        template
            .resources
            .insert("TestBucket".to_string(), resource);

        let errors = template.validate_dependencies();

        // Should not report errors for AWS pseudo parameters
        assert!(
            errors.is_empty(),
            "AWS pseudo parameters should not generate errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_dag_integration_with_template_validation() {
        let template = create_test_template();
        let mut dag = ResourceDag::new();

        // Convert CloudFormation resources to DAG resources
        for (resource_id, cfn_resource) in &template.resources {
            let dag_resource = CloudFormationResource {
                resource_id: resource_id.clone(),
                resource_type: cfn_resource.resource_type.clone(),
                properties: cfn_resource
                    .properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                depends_on: None,
                condition: None,
                metadata: None,
                deletion_policy: None,
                update_replace_policy: None,
                creation_policy: None,
                update_policy: None,
            };

            // Use the enhanced add method
            if let Err(e) = dag.add_resource_with_template_validation(dag_resource, &template) {
                // Only the first resource should succeed (no dependencies)
                // Subsequent resources depend on earlier ones not yet added
                println!("Expected error for resource {}: {}", resource_id, e);
            }
        }

        // Validate the DAG against template
        let validation_errors = dag.validate_against_template(&template);
        println!("DAG validation errors: {:?}", validation_errors);

        // Some errors are expected due to resource addition order
        // In real usage, resources would be added in dependency order
    }

    #[test]
    fn test_topological_sort_deployment_order() {
        let mut dag = ResourceDag::new();

        // Add resources in any order
        let bucket_resource = CloudFormationResource {
            resource_id: "MyBucket".to_string(),
            resource_type: "AWS::S3::Bucket".to_string(),
            properties: HashMap::new(),
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: None,
            update_policy: None,
        };

        let role_resource = CloudFormationResource {
            resource_id: "MyRole".to_string(),
            resource_type: "AWS::IAM::Role".to_string(),
            properties: HashMap::new(),
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: None,
            update_policy: None,
        };

        let function_resource = CloudFormationResource {
            resource_id: "MyFunction".to_string(),
            resource_type: "AWS::Lambda::Function".to_string(),
            properties: HashMap::new(),
            depends_on: None,
            condition: None,
            metadata: None,
            deletion_policy: None,
            update_replace_policy: None,
            creation_policy: None,
            update_policy: None,
        };

        // Add in dependency order
        dag.add_resource(bucket_resource, vec![]).unwrap();
        dag.add_resource(role_resource, vec![]).unwrap();
        dag.add_resource(
            function_resource,
            vec!["MyBucket".to_string(), "MyRole".to_string()],
        )
        .unwrap();

        let deployment_order = dag.get_deployment_order().unwrap();

        // Function should come after bucket and role
        let bucket_pos = deployment_order
            .iter()
            .position(|r| r == "MyBucket")
            .unwrap();
        let role_pos = deployment_order.iter().position(|r| r == "MyRole").unwrap();
        let function_pos = deployment_order
            .iter()
            .position(|r| r == "MyFunction")
            .unwrap();

        assert!(bucket_pos < function_pos);
        assert!(role_pos < function_pos);
    }

    #[test]
    fn test_complex_nested_ref_getatt() {
        let mut template = CloudFormationTemplate::default();

        // Create a resource with deeply nested Ref and GetAtt
        let mut complex_resource = Resource {
            resource_type: "AWS::CloudFormation::Stack".to_string(),
            ..Default::default()
        };

        // Complex nested structure with multiple levels
        complex_resource.properties.insert(
            "Parameters".to_string(),
            Value::Object({
                let mut params = serde_json::Map::new();
                params.insert(
                    "BucketArn".to_string(),
                    Value::Object({
                        let mut fn_sub = serde_json::Map::new();
                        fn_sub.insert(
                            "Fn::Sub".to_string(),
                            Value::Array(vec![
                                Value::String("arn:aws:s3:::${BucketName}/*".to_string()),
                                Value::Object({
                                    let mut sub_vars = serde_json::Map::new();
                                    sub_vars.insert(
                                        "BucketName".to_string(),
                                        Value::Object({
                                            let mut ref_obj = serde_json::Map::new();
                                            ref_obj.insert(
                                                "Ref".to_string(),
                                                Value::String("TestBucket".to_string()),
                                            );
                                            ref_obj
                                        }),
                                    );
                                    sub_vars
                                }),
                            ]),
                        );
                        fn_sub
                    }),
                );
                params.insert(
                    "RoleArn".to_string(),
                    Value::Object({
                        let mut getatt = serde_json::Map::new();
                        getatt.insert(
                            "Fn::GetAtt".to_string(),
                            Value::Array(vec![
                                Value::String("TestRole".to_string()),
                                Value::String("Arn".to_string()),
                            ]),
                        );
                        getatt
                    }),
                );
                params
            }),
        );

        // Add referenced resources
        let bucket = Resource {
            resource_type: "AWS::S3::Bucket".to_string(),
            ..Default::default()
        };

        let role = Resource {
            resource_type: "AWS::IAM::Role".to_string(),
            ..Default::default()
        };

        template
            .resources
            .insert("ComplexStack".to_string(), complex_resource);
        template.resources.insert("TestBucket".to_string(), bucket);
        template.resources.insert("TestRole".to_string(), role);

        let errors = template.validate_dependencies();
        assert!(
            errors.is_empty(),
            "Complex nested structure should validate correctly: {:?}",
            errors
        );

        let complex_cfn_resource = template.resources.get("ComplexStack").unwrap();
        let deps = template.extract_implicit_dependencies(complex_cfn_resource);

        // Should extract both dependencies
        assert!(deps.contains(&"TestBucket".to_string()));
        assert!(deps.contains(&"TestRole".to_string()));
    }
}
