#!/usr/bin/env python3
"""
Script to check AWS resources and their tags for the agent-pipeline bookmark.
This will help identify which resources should appear in the UI when filtering by Application=agent-pipeline.
"""

import boto3
import json
from datetime import datetime
from botocore.config import Config

# Configuration from bookmark
ACCOUNT_ID = "638876637120"
REGION = "us-east-1"
TAG_KEY = "Application"
TAG_VALUE = "agent-pipeline"

# Create boto3 config with retries
config = Config(
    retries={'max_attempts': 3, 'mode': 'standard'},
    region_name=REGION
)

def get_session():
    """Get boto3 session - assumes credentials are already configured."""
    return boto3.Session(region_name=REGION)

def check_s3_buckets(session):
    """Check S3 buckets and their tags."""
    s3 = session.client('s3', config=config)
    results = []

    try:
        response = s3.list_buckets()
        for bucket in response.get('Buckets', []):
            bucket_name = bucket['Name']
            try:
                # Get bucket location to filter by region
                location = s3.get_bucket_location(Bucket=bucket_name)
                bucket_region = location.get('LocationConstraint') or 'us-east-1'

                # Get tags
                try:
                    tag_response = s3.get_bucket_tagging(Bucket=bucket_name)
                    tags = {t['Key']: t['Value'] for t in tag_response.get('TagSet', [])}
                except s3.exceptions.ClientError:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::S3::Bucket',
                    'id': bucket_name,
                    'region': bucket_region,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
            except Exception as e:
                print(f"  Error checking bucket {bucket_name}: {e}")
    except Exception as e:
        print(f"Error listing S3 buckets: {e}")

    return results

def check_lambda_functions(session):
    """Check Lambda functions and their tags."""
    lambda_client = session.client('lambda', config=config)
    results = []

    try:
        paginator = lambda_client.get_paginator('list_functions')
        for page in paginator.paginate():
            for func in page.get('Functions', []):
                func_name = func['FunctionName']
                func_arn = func['FunctionArn']

                try:
                    tag_response = lambda_client.list_tags(Resource=func_arn)
                    tags = tag_response.get('Tags', {})
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::Lambda::Function',
                    'id': func_name,
                    'arn': func_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing Lambda functions: {e}")

    return results

def check_codebuild_projects(session):
    """Check CodeBuild projects and their tags."""
    codebuild = session.client('codebuild', config=config)
    results = []

    try:
        response = codebuild.list_projects()
        project_names = response.get('projects', [])

        if project_names:
            # Batch get project details (max 100 at a time)
            for i in range(0, len(project_names), 100):
                batch = project_names[i:i+100]
                projects_response = codebuild.batch_get_projects(names=batch)

                for project in projects_response.get('projects', []):
                    project_name = project['name']
                    project_arn = project['arn']
                    tags = {t['key']: t['value'] for t in project.get('tags', [])}

                    has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                    results.append({
                        'type': 'AWS::CodeBuild::Project',
                        'id': project_name,
                        'arn': project_arn,
                        'tags': tags,
                        'has_app_tag': has_app_tag
                    })
    except Exception as e:
        print(f"Error listing CodeBuild projects: {e}")

    return results

def check_codepipeline_pipelines(session):
    """Check CodePipeline pipelines and their tags."""
    codepipeline = session.client('codepipeline', config=config)
    results = []

    try:
        paginator = codepipeline.get_paginator('list_pipelines')
        for page in paginator.paginate():
            for pipeline in page.get('pipelines', []):
                pipeline_name = pipeline['name']
                pipeline_arn = f"arn:aws:codepipeline:{REGION}:{ACCOUNT_ID}:{pipeline_name}"

                try:
                    tag_response = codepipeline.list_tags_for_resource(resourceArn=pipeline_arn)
                    tags = {t['key']: t['value'] for t in tag_response.get('tags', [])}
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::CodePipeline::Pipeline',
                    'id': pipeline_name,
                    'arn': pipeline_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing CodePipeline pipelines: {e}")

    return results

def check_codecommit_repos(session):
    """Check CodeCommit repositories and their tags."""
    codecommit = session.client('codecommit', config=config)
    results = []

    try:
        paginator = codecommit.get_paginator('list_repositories')
        for page in paginator.paginate():
            for repo in page.get('repositories', []):
                repo_name = repo['repositoryName']
                repo_arn = f"arn:aws:codecommit:{REGION}:{ACCOUNT_ID}:{repo_name}"

                try:
                    tag_response = codecommit.list_tags_for_resource(resourceArn=repo_arn)
                    tags = tag_response.get('tags', {})
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::CodeCommit::Repository',
                    'id': repo_name,
                    'arn': repo_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing CodeCommit repositories: {e}")

    return results

def check_sns_topics(session):
    """Check SNS topics and their tags."""
    sns = session.client('sns', config=config)
    results = []

    try:
        paginator = sns.get_paginator('list_topics')
        for page in paginator.paginate():
            for topic in page.get('Topics', []):
                topic_arn = topic['TopicArn']
                topic_name = topic_arn.split(':')[-1]

                try:
                    tag_response = sns.list_tags_for_resource(ResourceArn=topic_arn)
                    tags = {t['Key']: t['Value'] for t in tag_response.get('Tags', [])}
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::SNS::Topic',
                    'id': topic_name,
                    'arn': topic_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing SNS topics: {e}")

    return results

def check_ssm_parameters(session):
    """Check SSM parameters and their tags."""
    ssm = session.client('ssm', config=config)
    results = []

    try:
        paginator = ssm.get_paginator('describe_parameters')
        for page in paginator.paginate():
            for param in page.get('Parameters', []):
                param_name = param['Name']
                param_arn = f"arn:aws:ssm:{REGION}:{ACCOUNT_ID}:parameter{param_name}"

                try:
                    tag_response = ssm.list_tags_for_resource(
                        ResourceType='Parameter',
                        ResourceId=param_name
                    )
                    tags = {t['Key']: t['Value'] for t in tag_response.get('TagList', [])}
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::SSM::Parameter',
                    'id': param_name,
                    'arn': param_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing SSM parameters: {e}")

    return results

def check_logs_log_groups(session):
    """Check CloudWatch Log Groups and their tags."""
    logs = session.client('logs', config=config)
    results = []

    try:
        paginator = logs.get_paginator('describe_log_groups')
        for page in paginator.paginate():
            for lg in page.get('logGroups', []):
                lg_name = lg['logGroupName']
                lg_arn = lg.get('arn', f"arn:aws:logs:{REGION}:{ACCOUNT_ID}:log-group:{lg_name}")

                try:
                    tag_response = logs.list_tags_for_resource(resourceArn=lg_arn)
                    tags = tag_response.get('tags', {})
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::Logs::LogGroup',
                    'id': lg_name,
                    'arn': lg_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing CloudWatch Log Groups: {e}")

    return results

def check_cognito_user_pools(session):
    """Check Cognito User Pools and their tags."""
    cognito = session.client('cognito-idp', config=config)
    results = []

    try:
        paginator = cognito.get_paginator('list_user_pools')
        for page in paginator.paginate(MaxResults=60):
            for pool in page.get('UserPools', []):
                pool_id = pool['Id']
                pool_name = pool['Name']
                pool_arn = f"arn:aws:cognito-idp:{REGION}:{ACCOUNT_ID}:userpool/{pool_id}"

                try:
                    # Get detailed info including tags
                    detail = cognito.describe_user_pool(UserPoolId=pool_id)
                    tags = detail.get('UserPool', {}).get('UserPoolTags', {})
                except Exception:
                    tags = {}

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::Cognito::UserPool',
                    'id': pool_id,
                    'name': pool_name,
                    'arn': pool_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing Cognito User Pools: {e}")

    return results

def check_cognito_identity_pools(session):
    """Check Cognito Identity Pools and their tags."""
    cognito_identity = session.client('cognito-identity', config=config)
    results = []

    try:
        response = cognito_identity.list_identity_pools(MaxResults=60)
        for pool in response.get('IdentityPools', []):
            pool_id = pool['IdentityPoolId']
            pool_name = pool['IdentityPoolName']
            pool_arn = f"arn:aws:cognito-identity:{REGION}:{ACCOUNT_ID}:identitypool/{pool_id}"

            try:
                tag_response = cognito_identity.list_tags_for_resource(ResourceArn=pool_arn)
                tags = tag_response.get('Tags', {})
            except Exception:
                tags = {}

            has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
            results.append({
                'type': 'AWS::Cognito::IdentityPool',
                'id': pool_id,
                'name': pool_name,
                'arn': pool_arn,
                'tags': tags,
                'has_app_tag': has_app_tag
            })
    except Exception as e:
        print(f"Error listing Cognito Identity Pools: {e}")

    return results

def check_api_gateway_rest_apis(session):
    """Check API Gateway REST APIs and their tags."""
    apigateway = session.client('apigateway', config=config)
    results = []

    try:
        paginator = apigateway.get_paginator('get_rest_apis')
        for page in paginator.paginate():
            for api in page.get('items', []):
                api_id = api['id']
                api_name = api.get('name', api_id)
                api_arn = f"arn:aws:apigateway:{REGION}::/restapis/{api_id}"
                tags = api.get('tags', {})

                has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
                results.append({
                    'type': 'AWS::ApiGateway::RestApi',
                    'id': api_id,
                    'name': api_name,
                    'arn': api_arn,
                    'tags': tags,
                    'has_app_tag': has_app_tag
                })
    except Exception as e:
        print(f"Error listing API Gateway REST APIs: {e}")

    return results

def check_api_gateway_v2_apis(session):
    """Check API Gateway V2 APIs and their tags."""
    apigatewayv2 = session.client('apigatewayv2', config=config)
    results = []

    try:
        response = apigatewayv2.get_apis()
        for api in response.get('Items', []):
            api_id = api['ApiId']
            api_name = api.get('Name', api_id)
            api_arn = f"arn:aws:apigateway:{REGION}::/apis/{api_id}"
            tags = api.get('Tags', {})

            has_app_tag = tags.get(TAG_KEY) == TAG_VALUE
            results.append({
                'type': 'AWS::ApiGatewayV2::Api',
                'id': api_id,
                'name': api_name,
                'arn': api_arn,
                'tags': tags,
                'has_app_tag': has_app_tag
            })
    except Exception as e:
        print(f"Error listing API Gateway V2 APIs: {e}")

    return results

def main():
    print(f"Checking AWS resources in account {ACCOUNT_ID}, region {REGION}")
    print(f"Looking for tag: {TAG_KEY}={TAG_VALUE}")
    print("=" * 80)

    session = get_session()

    all_results = []

    # Check each resource type
    checkers = [
        ("S3 Buckets", check_s3_buckets),
        ("Lambda Functions", check_lambda_functions),
        ("CodeBuild Projects", check_codebuild_projects),
        ("CodePipeline Pipelines", check_codepipeline_pipelines),
        ("CodeCommit Repositories", check_codecommit_repos),
        ("SNS Topics", check_sns_topics),
        ("SSM Parameters", check_ssm_parameters),
        ("CloudWatch Log Groups", check_logs_log_groups),
        ("Cognito User Pools", check_cognito_user_pools),
        ("Cognito Identity Pools", check_cognito_identity_pools),
        ("API Gateway REST APIs", check_api_gateway_rest_apis),
        ("API Gateway V2 APIs", check_api_gateway_v2_apis),
    ]

    for name, checker in checkers:
        print(f"\nChecking {name}...")
        results = checker(session)
        all_results.extend(results)

        matching = [r for r in results if r['has_app_tag']]
        print(f"  Found {len(results)} total, {len(matching)} with Application=agent-pipeline tag")

    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY: Resources with Application=agent-pipeline tag")
    print("=" * 80)

    matching_resources = [r for r in all_results if r['has_app_tag']]

    # Group by type
    by_type = {}
    for r in matching_resources:
        t = r['type']
        if t not in by_type:
            by_type[t] = []
        by_type[t].append(r)

    for resource_type in sorted(by_type.keys()):
        resources = by_type[resource_type]
        print(f"\n{resource_type} ({len(resources)}):")
        for r in resources:
            name = r.get('name', r['id'])
            print(f"  - {name} (id: {r['id']})")
            if r['tags']:
                app_tag = r['tags'].get(TAG_KEY, 'N/A')
                print(f"    Tags: Application={app_tag}")

    print("\n" + "=" * 80)
    print(f"TOTAL: {len(matching_resources)} resources should appear in UI with Application=agent-pipeline filter")
    print("=" * 80)

    # Save detailed results to JSON
    output_file = "/tmp/tag_check_results.json"
    with open(output_file, 'w') as f:
        json.dump({
            'timestamp': datetime.now().isoformat(),
            'account': ACCOUNT_ID,
            'region': REGION,
            'tag_filter': f"{TAG_KEY}={TAG_VALUE}",
            'matching_count': len(matching_resources),
            'total_count': len(all_results),
            'matching_resources': matching_resources,
            'all_resources': all_results
        }, f, indent=2, default=str)

    print(f"\nDetailed results saved to: {output_file}")

if __name__ == "__main__":
    main()
