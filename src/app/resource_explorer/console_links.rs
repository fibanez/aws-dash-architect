use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

#[derive(Debug)]
struct ArnParts<'a> {
    service: &'a str,
    _region: &'a str,
    account_id: &'a str,
    resource: &'a str,
}

fn parse_arn(arn: &str) -> Option<ArnParts<'_>> {
    let mut parts = arn.splitn(6, ':');
    if parts.next()? != "arn" {
        return None;
    }
    let _partition = parts.next()?;
    let service = parts.next()?;
    let region = parts.next()?;
    let account_id = parts.next()?;
    let resource = parts.next()?;
    Some(ArnParts {
        service,
        _region: region,
        account_id,
        resource,
    })
}

fn arn_resource_suffix(resource: &str) -> &str {
    resource
        .rsplit(|ch| ch == '/' || ch == ':')
        .next()
        .unwrap_or(resource)
}

fn ecs_cluster_name(arn_or_name: &str) -> Option<&str> {
    if arn_or_name.starts_with("arn:") {
        let parts = parse_arn(arn_or_name)?;
        let resource = parts.resource;
        resource.strip_prefix("cluster/").or_else(|| {
            let trimmed = resource.strip_prefix("cluster/").unwrap_or(resource);
            Some(trimmed)
        })
    } else {
        Some(arn_or_name)
    }
}

fn ecs_task_fields(arn_or_id: &str) -> Option<(String, String)> {
    if !arn_or_id.starts_with("arn:") {
        return None;
    }
    let parts = parse_arn(arn_or_id)?;
    let resource = parts.resource;
    let rest = resource.strip_prefix("task/")?;
    let mut split = rest.splitn(2, '/');
    let cluster = split.next()?.to_string();
    let task_id = split.next()?.to_string();
    Some((cluster, task_id))
}

fn ecs_task_definition_fields(arn_or_id: &str) -> Option<(String, String)> {
    if !arn_or_id.starts_with("arn:") {
        return None;
    }
    let parts = parse_arn(arn_or_id)?;
    let resource = parts.resource;
    let rest = resource.strip_prefix("task-definition/")?;
    let mut split = rest.splitn(2, ':');
    let name = split.next()?.to_string();
    let revision = split.next().unwrap_or("1").to_string();
    Some((name, revision))
}

fn log_group_and_stream(arn: &str) -> Option<(String, Option<String>)> {
    let parts = parse_arn(arn)?;
    if parts.service != "logs" {
        return None;
    }
    let resource = parts.resource;
    if let Some(rest) = resource.strip_prefix("log-group:") {
        if let Some((group, stream)) = rest.split_once(":log-stream:") {
            return Some((group.to_string(), Some(stream.to_string())));
        }
        return Some((rest.to_string(), None));
    }
    None
}

fn service_console_url(resource_type: &str, region: &str) -> Option<String> {
    let service = resource_type.split("::").nth(1)?.to_ascii_lowercase();
    let encoded_region = encode(region);
    let url = match service.as_str() {
        "ec2" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}",
            encoded_region
        ),
        "vpc" => format!(
            "https://console.aws.amazon.com/vpc/home?region={}",
            encoded_region
        ),
        "s3" => format!(
            "https://s3.console.aws.amazon.com/s3/home?region={}",
            encoded_region
        ),
        "lambda" => format!(
            "https://console.aws.amazon.com/lambda/home?region={}",
            encoded_region
        ),
        "dynamodb" => format!(
            "https://console.aws.amazon.com/dynamodbv2/home?region={}",
            encoded_region
        ),
        "rds" => format!(
            "https://console.aws.amazon.com/rds/home?region={}",
            encoded_region
        ),
        "eks" => format!(
            "https://console.aws.amazon.com/eks/home?region={}",
            encoded_region
        ),
        "ecs" => format!(
            "https://console.aws.amazon.com/ecs/home?region={}",
            encoded_region
        ),
        "cloudformation" => format!(
            "https://console.aws.amazon.com/cloudformation/home?region={}",
            encoded_region
        ),
        "iam" => "https://console.aws.amazon.com/iam/home#/".to_string(),
        "apigateway" | "apigatewayv2" => format!(
            "https://console.aws.amazon.com/apigateway/main/apis?region={}",
            encoded_region
        ),
        "stepfunctions" => format!(
            "https://console.aws.amazon.com/states/home?region={}",
            encoded_region
        ),
        "logs" | "cloudwatch" => format!(
            "https://console.aws.amazon.com/cloudwatch/home?region={}",
            encoded_region
        ),
        "events" => format!(
            "https://console.aws.amazon.com/events/home?region={}",
            encoded_region
        ),
        "kinesis" => format!(
            "https://console.aws.amazon.com/kinesis/home?region={}",
            encoded_region
        ),
        "kinesisfirehose" => format!(
            "https://console.aws.amazon.com/firehose/home?region={}",
            encoded_region
        ),
        "sqs" => format!(
            "https://console.aws.amazon.com/sqs/v2/home?region={}",
            encoded_region
        ),
        "sns" => format!(
            "https://console.aws.amazon.com/sns/v3/home?region={}",
            encoded_region
        ),
        "ssm" => format!(
            "https://console.aws.amazon.com/systems-manager/home?region={}",
            encoded_region
        ),
        "kms" => format!(
            "https://console.aws.amazon.com/kms/home?region={}",
            encoded_region
        ),
        "glue" => format!(
            "https://console.aws.amazon.com/glue/home?region={}",
            encoded_region
        ),
        "athena" => format!(
            "https://console.aws.amazon.com/athena/home?region={}",
            encoded_region
        ),
        "codebuild" => format!(
            "https://console.aws.amazon.com/codesuite/codebuild/home?region={}",
            encoded_region
        ),
        "codecommit" => format!(
            "https://console.aws.amazon.com/codesuite/codecommit/home?region={}",
            encoded_region
        ),
        "codepipeline" => format!(
            "https://console.aws.amazon.com/codesuite/codepipeline/home?region={}",
            encoded_region
        ),
        "ecr" => format!(
            "https://console.aws.amazon.com/ecr/repositories?region={}",
            encoded_region
        ),
        "secretsmanager" => format!(
            "https://console.aws.amazon.com/secretsmanager/home?region={}",
            encoded_region
        ),
        "route53" => "https://console.aws.amazon.com/route53/v2/hostedzones".to_string(),
        "cloudfront" => "https://console.aws.amazon.com/cloudfront/v3/home".to_string(),
        "globalaccelerator" => format!(
            "https://console.aws.amazon.com/globalaccelerator/home?region={}",
            encoded_region
        ),
        "organizations" => "https://console.aws.amazon.com/organizations/v2/home".to_string(),
        "shield" => format!(
            "https://console.aws.amazon.com/shield/home?region={}",
            encoded_region
        ),
        "wafv2" => format!(
            "https://console.aws.amazon.com/wafv2/home?region={}",
            encoded_region
        ),
        "elasticloadbalancing" | "elasticloadbalancingv2" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}#LoadBalancers:",
            encoded_region
        ),
        "cloudtrail" => format!(
            "https://console.aws.amazon.com/cloudtrail/home?region={}",
            encoded_region
        ),
        "config" => format!(
            "https://console.aws.amazon.com/config/home?region={}",
            encoded_region
        ),
        "accessanalyzer" => format!(
            "https://console.aws.amazon.com/access-analyzer/home?region={}",
            encoded_region
        ),
        "guardduty" => format!(
            "https://console.aws.amazon.com/guardduty/home?region={}",
            encoded_region
        ),
        "securityhub" => format!(
            "https://console.aws.amazon.com/securityhub/home?region={}",
            encoded_region
        ),
        "acm" | "certificatemanager" => format!(
            "https://console.aws.amazon.com/acm/home?region={}",
            encoded_region
        ),
        "acmpca" => format!(
            "https://console.aws.amazon.com/acm-pca/home?region={}",
            encoded_region
        ),
        "autoscaling" => format!(
            "https://console.aws.amazon.com/ec2autoscaling/home?region={}",
            encoded_region
        ),
        "appsync" => format!(
            "https://console.aws.amazon.com/appsync/home?region={}",
            encoded_region
        ),
        "amazonmq" => format!(
            "https://console.aws.amazon.com/amazonmq/home?region={}",
            encoded_region
        ),
        "msk" => format!(
            "https://console.aws.amazon.com/msk/home?region={}",
            encoded_region
        ),
        "lakeformation" => format!(
            "https://console.aws.amazon.com/lakeformation/home?region={}",
            encoded_region
        ),
        "iot" => format!(
            "https://console.aws.amazon.com/iot/home?region={}",
            encoded_region
        ),
        "greengrassv2" => format!(
            "https://console.aws.amazon.com/greengrass/v2/home?region={}",
            encoded_region
        ),
        "cognito" => format!(
            "https://console.aws.amazon.com/cognito/home?region={}",
            encoded_region
        ),
        "batch" => format!(
            "https://console.aws.amazon.com/batch/home?region={}",
            encoded_region
        ),
        "quicksight" => format!(
            "https://console.aws.amazon.com/quicksight/home?region={}",
            encoded_region
        ),
        "macie" => format!(
            "https://console.aws.amazon.com/macie/home?region={}",
            encoded_region
        ),
        "inspector" => format!(
            "https://console.aws.amazon.com/inspector/home?region={}",
            encoded_region
        ),
        "timestream" => format!(
            "https://console.aws.amazon.com/timestream/home?region={}",
            encoded_region
        ),
        "documentdb" => format!(
            "https://console.aws.amazon.com/docdb/home?region={}",
            encoded_region
        ),
        "transfer" => format!(
            "https://console.aws.amazon.com/transfer/home?region={}",
            encoded_region
        ),
        "fsx" => format!(
            "https://console.aws.amazon.com/fsx/home?region={}",
            encoded_region
        ),
        "workspaces" => format!(
            "https://console.aws.amazon.com/workspaces/home?region={}",
            encoded_region
        ),
        "apprunner" => format!(
            "https://console.aws.amazon.com/apprunner/home?region={}",
            encoded_region
        ),
        "connect" => format!(
            "https://console.aws.amazon.com/connect/home?region={}",
            encoded_region
        ),
        "amplify" => format!(
            "https://console.aws.amazon.com/amplify/home?region={}",
            encoded_region
        ),
        "lex" => format!(
            "https://console.aws.amazon.com/lexv2/home?region={}",
            encoded_region
        ),
        "rekognition" => format!(
            "https://console.aws.amazon.com/rekognition/home?region={}",
            encoded_region
        ),
        "polly" => format!(
            "https://console.aws.amazon.com/polly/home?region={}",
            encoded_region
        ),
        "bedrock" => format!(
            "https://console.aws.amazon.com/bedrock/home?region={}",
            encoded_region
        ),
        "sagemaker" => format!(
            "https://console.aws.amazon.com/sagemaker/home?region={}",
            encoded_region
        ),
        "redshift" => format!(
            "https://console.aws.amazon.com/redshiftv2/home?region={}",
            encoded_region
        ),
        "opensearchservice" => format!(
            "https://console.aws.amazon.com/opensearch/home?region={}",
            encoded_region
        ),
        "neptune" => format!(
            "https://console.aws.amazon.com/neptune/home?region={}",
            encoded_region
        ),
        "elasticache" => format!(
            "https://console.aws.amazon.com/elasticache/home?region={}",
            encoded_region
        ),
        "efs" => format!(
            "https://console.aws.amazon.com/efs/home?region={}",
            encoded_region
        ),
        "backup" => format!(
            "https://console.aws.amazon.com/backup/home?region={}",
            encoded_region
        ),
        "databrew" => format!(
            "https://console.aws.amazon.com/databrew/home?region={}",
            encoded_region
        ),
        "detective" => format!(
            "https://console.aws.amazon.com/detective/home?region={}",
            encoded_region
        ),
        "xray" => format!(
            "https://console.aws.amazon.com/xray/home?region={}",
            encoded_region
        ),
        _ => return None,
    };
    Some(url)
}

pub fn build_console_destination(
    resource_type: &str,
    resource_id: &str,
    region: &str,
    resource_arn: Option<&str>,
) -> String {
    match resource_type {
        "AWS::EC2::Instance" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}#InstanceDetails:instanceId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::Image" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}#ImageDetails:imageId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::Volume" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}#VolumeDetails:volumeId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::Snapshot" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}#SnapshotDetails:snapshotId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::SecurityGroup" => format!(
            "https://console.aws.amazon.com/ec2/home?region={}#SecurityGroup:groupId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::VPC" => format!(
            "https://console.aws.amazon.com/vpc/home?region={}#vpcs:VpcId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::Subnet" => format!(
            "https://console.aws.amazon.com/vpc/home?region={}#SubnetDetails:subnetId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::RouteTable" => format!(
            "https://console.aws.amazon.com/vpc/home?region={}#RouteTableDetails:RouteTableId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EC2::VPCEndpoint" => format!(
            "https://console.aws.amazon.com/vpc/home?region={}#EndpointDetails:vpcEndpointId={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::S3::Bucket" => format!(
            "https://s3.console.aws.amazon.com/s3/buckets/{}?region={}&tab=objects",
            encode(resource_id),
            encode(region)
        ),
        "AWS::Lambda::Function" => format!(
            "https://console.aws.amazon.com/lambda/home?region={}#/functions/{}?tab=code",
            encode(region),
            encode(resource_id)
        ),
        "AWS::Lambda::LayerVersion" => {
            let (name, version) = if let Some(arn) = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            }) {
                if let Some(parts) = parse_arn(arn) {
                    let resource = parts.resource;
                    if let Some(rest) = resource.strip_prefix("layer:") {
                        let mut split = rest.splitn(2, ':');
                        (
                            split.next().unwrap_or(resource_id).to_string(),
                            split.next().unwrap_or("1").to_string(),
                        )
                    } else {
                        (resource_id.to_string(), "1".to_string())
                    }
                } else {
                    (resource_id.to_string(), "1".to_string())
                }
            } else {
                (resource_id.to_string(), "1".to_string())
            };
            format!(
                "https://console.aws.amazon.com/lambda/home?region={}#/layers/{}/versions/{}?tab=versions",
                encode(region),
                encode(&name),
                encode(&version)
            )
        }
        "AWS::DynamoDB::Table" => format!(
            "https://console.aws.amazon.com/dynamodbv2/home?region={}#item-explorer?initialTagKey=&maximize=true&table={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::RDS::DBInstance" => format!(
            "https://console.aws.amazon.com/rds/home?region={}#database:id={};is-cluster=false",
            encode(region),
            encode(resource_id)
        ),
        "AWS::RDS::DBCluster" => format!(
            "https://console.aws.amazon.com/rds/home?region={}#database:id={};is-cluster=true",
            encode(region),
            encode(resource_id)
        ),
        "AWS::RDS::DBSnapshot" => format!(
            "https://console.aws.amazon.com/rds/home?region={}#db-snapshot:id={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::RDS::DBSubnetGroup" => format!(
            "https://console.aws.amazon.com/rds/home?region={}#db-subnet-group:id={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::RDS::DBParameterGroup" => format!(
            "https://console.aws.amazon.com/rds/home?region={}#parameter-group-details:parameter-group-name={}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::EKS::Cluster" => format!(
            "https://console.aws.amazon.com/eks/home?region={}#/clusters/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::ECS::Cluster" => {
            let name = ecs_cluster_name(resource_id)
                .or_else(|| resource_arn.and_then(ecs_cluster_name))
                .unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/ecs/clusters/{}/services?region={}",
                encode(name),
                encode(region)
            )
        }
        "AWS::ECS::Task" => {
            if let Some(arn) = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            }) {
                if let Some((cluster, task_id)) = ecs_task_fields(arn) {
                    return format!(
                        "https://console.aws.amazon.com/ecs/clusters/{}/tasks/{}/configuration?region={}",
                        encode(&cluster),
                        encode(&task_id),
                        encode(region)
                    );
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::ECS::TaskDefinition" => {
            if let Some(arn) = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            }) {
                if let Some((name, revision)) = ecs_task_definition_fields(arn) {
                    return format!(
                        "https://console.aws.amazon.com/ecs/task-definitions/{}/{}/containers?region={}",
                        encode(&name),
                        encode(&revision),
                        encode(region)
                    );
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::CloudFormation::Stack" => format!(
            "https://console.aws.amazon.com/cloudformation/home?region={}#/stacks/stackinfo?stackId={}",
            encode(region),
            encode(resource_arn.unwrap_or(resource_id))
        ),
        "AWS::IAM::Role" => format!(
            "https://console.aws.amazon.com/iam/home#/roles/{}",
            encode(resource_id)
        ),
        "AWS::IAM::User" => format!(
            "https://console.aws.amazon.com/iam/home#/users/{}",
            encode(resource_id)
        ),
        "AWS::IAM::Policy" => {
            let arn = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/iam/home#/policies/details/{}?section=permissions",
                encode(arn)
            )
        }
        "AWS::ApiGateway::RestApi" => format!(
            "https://console.aws.amazon.com/apigateway/main/apis/{}/resources?api={}&region={}",
            encode(resource_id),
            encode(resource_id),
            encode(region)
        ),
        "AWS::ApiGatewayV2::Api" => format!(
            "https://console.aws.amazon.com/apigateway/main/develop/routes?api={}&region={}",
            encode(resource_id),
            encode(region)
        ),
        "AWS::StepFunctions::StateMachine" => {
            let arn = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/states/home?region={}#/statemachines/view/{}",
                encode(region),
                encode(arn)
            )
        }
        "AWS::Logs::LogGroup" => {
            if let Some(arn) = resource_arn {
                if let Some((group, _)) = log_group_and_stream(arn) {
                    return format!(
                        "https://console.aws.amazon.com/cloudwatch/home?region={}#logsV2:log-groups/log-group/{}",
                        encode(region),
                        encode(&group)
                    );
                }
            }
            format!(
                "https://console.aws.amazon.com/cloudwatch/home?region={}#logsV2:log-groups/log-group/{}",
                encode(region),
                encode(resource_id)
            )
        }
        "AWS::Logs::LogStream" => {
            if let Some(arn) = resource_arn {
                if let Some((group, stream)) = log_group_and_stream(arn) {
                    if let Some(stream) = stream {
                        return format!(
                            "https://console.aws.amazon.com/cloudwatch/home?region={}#logsV2:log-groups/log-group/{}/log-events/{}",
                            encode(region),
                            encode(&group),
                            encode(&stream)
                        );
                    }
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::Events::EventBus" => format!(
            "https://console.aws.amazon.com/events/home?region={}#/eventbus/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::Events::Rule" => {
            if let Some(arn) = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            }) {
                if let Some(parts) = parse_arn(arn) {
                    if let Some(rest) = parts.resource.strip_prefix("rule/") {
                        let mut split = rest.splitn(2, '/');
                        let first = split.next().unwrap_or_default();
                        let second = split.next();
                        let (event_bus, rule) = if let Some(rule) = second {
                            (first, rule)
                        } else {
                            ("default", first)
                        };
                        return format!(
                            "https://console.aws.amazon.com/events/home?region={}#/eventbus/{}/rules/{}",
                            encode(region),
                            encode(event_bus),
                            encode(rule)
                        );
                    }
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::Kinesis::Stream" => {
            let name = resource_arn
                .and_then(parse_arn)
                .map(|parts| arn_resource_suffix(parts.resource).to_string())
                .unwrap_or_else(|| resource_id.to_string());
            format!(
                "https://console.aws.amazon.com/kinesis/home?region={}#/streams/details/{}/monitoring",
                encode(region),
                encode(&name)
            )
        }
        "AWS::KinesisFirehose::DeliveryStream" => {
            let name = resource_arn
                .and_then(parse_arn)
                .map(|parts| arn_resource_suffix(parts.resource).to_string())
                .unwrap_or_else(|| resource_id.to_string());
            format!(
                "https://console.aws.amazon.com/firehose/home?region={}#/details/{}/monitoring",
                encode(region),
                encode(&name)
            )
        }
        "AWS::SQS::Queue" => {
            let arn = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            });
            if let Some(arn) = arn {
                if let Some(parts) = parse_arn(arn) {
                    let name = arn_resource_suffix(parts.resource);
                    return format!(
                        "https://console.aws.amazon.com/sqs/v2/home?region={}#/queues/https%3A%2F%2Fsqs.{}.amazonaws.com%2F{}%2F{}",
                        encode(region),
                        encode(region),
                        encode(parts.account_id),
                        encode(name)
                    );
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::SNS::Topic" => {
            let arn = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/sns/v3/home?region={}#/topic/{}",
                encode(region),
                encode(arn)
            )
        }
        "AWS::SSM::Parameter" => format!(
            "https://console.aws.amazon.com/systems-manager/parameters/{}/description?region={}&tab=Table",
            encode(resource_id),
            encode(region)
        ),
        "AWS::KMS::Key" => format!(
            "https://console.aws.amazon.com/kms/home?region={}#/kms/keys/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::Glue::Database" => format!(
            "https://console.aws.amazon.com/glue/home?region={}#/v2/data-catalog/databases/view/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::Glue::Crawler" => format!(
            "https://console.aws.amazon.com/glue/home?region={}#/v2/data-catalog/crawlers/view/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::Glue::Job" => format!(
            "https://console.aws.amazon.com/gluestudio/home?region={}#/editor/job/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::Athena::WorkGroup" => format!(
            "https://console.aws.amazon.com/athena/home?region={}#/workgroups/details/{}",
            encode(region),
            encode(resource_id)
        ),
        "AWS::CodeBuild::Project" => {
            let arn = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            });
            if let Some(arn) = arn {
                if let Some(parts) = parse_arn(arn) {
                    let name = arn_resource_suffix(parts.resource);
                    return format!(
                        "https://console.aws.amazon.com/codesuite/codebuild/{}/projects/{}/history?region={}",
                        encode(parts.account_id),
                        encode(name),
                        encode(region)
                    );
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::CodeCommit::Repository" => format!(
            "https://console.aws.amazon.com/codesuite/codecommit/repositories/{}/browse?region={}",
            encode(resource_id),
            encode(region)
        ),
        "AWS::CodePipeline::Pipeline" => format!(
            "https://console.aws.amazon.com/codesuite/codepipeline/pipelines/{}/view?region={}",
            encode(resource_id),
            encode(region)
        ),
        "AWS::ECR::Repository" => {
            let arn = resource_arn.or_else(|| {
                if resource_id.starts_with("arn:") {
                    Some(resource_id)
                } else {
                    None
                }
            });
            if let Some(arn) = arn {
                if let Some(parts) = parse_arn(arn) {
                    let name = arn_resource_suffix(parts.resource);
                    return format!(
                        "https://console.aws.amazon.com/ecr/repositories/private/{}/{}?region={}",
                        encode(parts.account_id),
                        encode(name),
                        encode(region)
                    );
                }
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
        "AWS::SecretsManager::Secret" => format!(
            "https://console.aws.amazon.com/secretsmanager/secret?name={}&region={}",
            encode(resource_id),
            encode(region)
        ),
        _ => {
            if let Some(url) = service_console_url(resource_type, region) {
                return url;
            }
            let query = resource_arn.unwrap_or(resource_id);
            format!(
                "https://console.aws.amazon.com/resource-groups/home?region={}#/resources?search={}",
                encode(region),
                encode(query)
            )
        }
    }
}
