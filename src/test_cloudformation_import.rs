#[cfg(test)]
mod tests {
    use crate::app::cfn_template::CloudFormationTemplate;

    #[test]
    fn test_import_config_template() {
        // Create a test CloudFormation template with DependsOn as a string
        let json_content = r#"{
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "ConfigRecorder": {
                    "Type": "AWS::Config::ConfigurationRecorder",
                    "Properties": {
                        "Name": "default"
                    }
                },
                "DeliveryChannel": {
                    "Type": "AWS::Config::DeliveryChannel",
                    "Properties": {
                        "S3BucketName": "test-bucket"
                    },
                    "DependsOn": "ConfigRecorder"
                }
            }
        }"#;

        // Parse the JSON content
        let template: CloudFormationTemplate =
            serde_json::from_str(json_content).expect("Failed to parse CloudFormation template");

        // Verify resources were parsed correctly
        assert_eq!(template.resources.len(), 2);

        // Check that the DependsOn field was parsed correctly
        let delivery_channel = template.resources.get("DeliveryChannel").unwrap();
        assert!(delivery_channel.depends_on.is_some());

        match &delivery_channel.depends_on {
            Some(crate::app::cfn_template::DependsOn::Single(s)) => {
                assert_eq!(s, "ConfigRecorder");
            }
            _ => panic!("DependsOn should be a single string"),
        }
    }

    #[test]
    fn test_import_template_with_array_depends_on() {
        // Create a test CloudFormation template with DependsOn as an array
        let json_content = r#"{
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "Resource1": {
                    "Type": "AWS::S3::Bucket"
                },
                "Resource2": {
                    "Type": "AWS::S3::Bucket"
                },
                "Resource3": {
                    "Type": "AWS::Config::DeliveryChannel",
                    "DependsOn": ["Resource1", "Resource2"]
                }
            }
        }"#;

        // Parse the JSON content
        let template: CloudFormationTemplate =
            serde_json::from_str(json_content).expect("Failed to parse CloudFormation template");

        // Check that the DependsOn field was parsed correctly as an array
        let resource3 = template.resources.get("Resource3").unwrap();
        assert!(resource3.depends_on.is_some());

        match &resource3.depends_on {
            Some(crate::app::cfn_template::DependsOn::Multiple(v)) => {
                assert_eq!(v.len(), 2);
                assert!(v.contains(&"Resource1".to_string()));
                assert!(v.contains(&"Resource2".to_string()));
            }
            _ => panic!("DependsOn should be an array"),
        }
    }

    #[test]
    fn test_user_config_template() {
        // Test the exact template structure the user is trying to import
        let json_content = r#"{
            "AWSTemplateFormatVersion": "2010-09-09",
            "Description": "AWS Config Test Template",
            "Resources": {
                "ConfigRecorder": {
                    "Type": "AWS::Config::ConfigurationRecorder",
                    "Properties": {
                        "Name": "default",
                        "RecordingGroup": {
                            "AllSupported": true,
                            "IncludeGlobalResourceTypes": true
                        },
                        "RoleARN": {
                            "Fn::GetAtt": ["ConfigRole", "Arn"]
                        }
                    }
                },
                "DeliveryChannel": {
                    "Type": "AWS::Config::DeliveryChannel",
                    "Properties": {
                        "ConfigSnapshotDeliveryProperties": {
                            "DeliveryFrequency": "TwentyFour_Hours"
                        },
                        "S3BucketName": {
                            "Ref": "ConfigBucket"
                        }
                    },
                    "DependsOn": "ConfigRecorder"
                }
            }
        }"#;

        // Parse the JSON content - this should work now with our DependsOn fix
        let template: CloudFormationTemplate =
            serde_json::from_str(json_content).expect("Failed to parse CloudFormation template");

        // Verify the template was parsed correctly
        assert_eq!(
            template.description,
            Some("AWS Config Test Template".to_string())
        );
        assert_eq!(template.resources.len(), 2);

        // Check the DependsOn field
        let delivery_channel = template.resources.get("DeliveryChannel").unwrap();
        assert!(delivery_channel.depends_on.is_some());

        match &delivery_channel.depends_on {
            Some(crate::app::cfn_template::DependsOn::Single(s)) => {
                assert_eq!(s, "ConfigRecorder");
            }
            _ => panic!("DependsOn should be a single string"),
        }
    }
}
