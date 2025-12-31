# Setup IAM Identity Center

Before using AWS Dash Architect, you must configure AWS Identity Center with the required permission sets and deployment roles. This setup provides secure access to AWS resources and enables the application's specialized AI agents to function properly.

## Step 1: Select a Role Home Account

Before creating your permission set, you must select a **Role Home Account**. This is the AWS account where AWS Dash will store its operational data and telemetry.

**What is a Role Home Account?**

Each AWS Dash permission set requires a designated Role Home Account that serves as the central location for:

- **Persistent Data**: DynamoDB table for storing application state, preferences, and cached resource data
- **Agent Logging**: CloudWatch logs and X-Ray traces for AI agent observability and evaluation
- **Bedrock Models**: The AI models used by agents must be enabled in this account

**Multi-Team Deployments**

You can create multiple AWS Dash permission sets, typically one per team or application:

| Permission Set | Role Home Account | Purpose |
|----------------|-------------------|---------|
| `awsdash-platform` | 111122223333 | Platform engineering team |
| `awsdash-devops` | 444455556666 | DevOps team |
| `awsdash-security` | 777788889999 | Security team |

Each team's data remains isolated in their designated Role Home Account.

**Requirements for the Role Home Account:**

1. **Bedrock Model Access**: Enable the foundation models you want agents to use (Claude Sonnet, Claude Haiku, etc.)
2. **X-Ray Destination**: Configure X-Ray to send traces to CloudWatch Logs (one-time setup, see Step 1.5)
3. **DynamoDB Table**: The `awsdash` table will be created automatically on first use

**Record your Role Home Account ID** - you will need it for the inline policy below.

## Step 2: Create AWS Dash Permission Set

1. **Navigate to AWS Identity Center**
   - Log into your AWS Management Console
   - Go to AWS Identity Center (successor to AWS Single Sign-On)
   - Select "Permission sets" from the left navigation

2. **Create New Permission Set**
   - Click "Create permission set"
   - Choose "Custom permission set"
   - Name: `awsdash` (or your preferred name, e.g., `awsdash-platform`)
   - Description: "AWS Dash Architect access with Bedrock and CloudFormation deployment permissions"

3. **Attach AWS Managed Policy**
   - Under "AWS managed policies", search and attach:
     - **ReadOnlyAccess** (Job function category)

4. **Create Inline Policy**
   - Click "Create inline policy"
   - Switch to JSON view and paste the following policy
   - **Important**: Replace `<RoleHomeAccount>` with your Role Home Account ID

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Sid": "DynamoDBAccess",
            "Effect": "Allow",
            "Action": [
                "dynamodb:*"
            ],
            "Resource": [
                "arn:aws:dynamodb:us-east-1:<RoleHomeAccount>:table/awsdash"
            ]
        },
        {
            "Sid": "BedrockAccess",
            "Effect": "Allow",
            "Action": [
                "bedrock:*",
                "bedrock:ListFoundationModels",
                "bedrock:InvokeModel",
                "bedrock:InvokeModelWithResponseStream"
            ],
            "Resource": "*"
        },
        {
            "Sid": "PassCloudFormationDeploymentRole",
            "Effect": "Allow",
            "Action": [
                "iam:PassRole"
            ],
            "Resource": "arn:aws:iam::*:role/CloudFormation-Deployment-Role",
            "Condition": {
                "StringEquals": {
                    "iam:PassedToService": "cloudformation.amazonaws.com"
                }
            }
        },
        {
            "Sid": "CloudFormationStackOperations",
            "Effect": "Allow",
            "Action": [
                "cloudformation:CreateStack",
                "cloudformation:UpdateStack",
                "cloudformation:DeleteStack",
                "cloudformation:DescribeStacks",
                "cloudformation:DescribeStackEvents",
                "cloudformation:DescribeStackResources",
                "cloudformation:GetStackPolicy",
                "cloudformation:GetTemplate",
                "cloudformation:ListStackResources",
                "cloudformation:ValidateTemplate",
                "cloudformation:CreateChangeSet",
                "cloudformation:DeleteChangeSet",
                "cloudformation:DescribeChangeSet",
                "cloudformation:ExecuteChangeSet",
                "cloudformation:ListChangeSets",
                "cloudformation:SetStackPolicy",
                "cloudformation:TagResource",
                "cloudformation:UntagResource",
                "cloudformation:ListStacks"
            ],
            "Resource": "*"
        },
        {
            "Sid": "ViewCloudFormationRole",
            "Effect": "Allow",
            "Action": [
                "iam:GetRole",
                "iam:GetRolePolicy",
                "iam:ListRolePolicies",
                "iam:ListAttachedRolePolicies"
            ],
            "Resource": "arn:aws:iam::*:role/CloudFormation-Deployment-Role"
        },
        {
            "Sid": "AgentCoreLogGroupManagement",
            "Effect": "Allow",
            "Action": [
                "logs:DescribeLogStreams",
                "logs:CreateLogGroup"
            ],
            "Resource": "arn:aws:logs:*:<RoleHomeAccount>:log-group:/aws/bedrock-agentcore/runtimes/*"
        },
        {
            "Sid": "AgentCoreDescribeLogGroups",
            "Effect": "Allow",
            "Action": "logs:DescribeLogGroups",
            "Resource": "arn:aws:logs:*:<RoleHomeAccount>:log-group:*"
        },
        {
            "Sid": "AgentCoreLogStreamWrite",
            "Effect": "Allow",
            "Action": [
                "logs:CreateLogStream",
                "logs:PutLogEvents"
            ],
            "Resource": "arn:aws:logs:*:<RoleHomeAccount>:log-group:/aws/bedrock-agentcore/runtimes/*:log-stream:*"
        },
        {
            "Sid": "XRayTraceWrite",
            "Effect": "Allow",
            "Action": [
                "xray:PutTraceSegments",
                "xray:PutTelemetryRecords",
                "xray:GetSamplingRules",
                "xray:GetSamplingTargets"
            ],
            "Resource": "*"
        },
        {
            "Sid": "AgentCoreMetrics",
            "Effect": "Allow",
            "Action": "cloudwatch:PutMetricData",
            "Resource": "*",
            "Condition": {
                "StringEquals": {
                    "cloudwatch:namespace": "bedrock-agentcore"
                }
            }
        }
    ]
}
```

   - Name the policy: `AWSDashArchitectInlinePolicy`
   - Click "Create policy"

### Step 2.5: Enable Agent Logging in Role Home Account (Optional)

If you plan to use **Agent Logging** (CloudWatch Gen AI Observability), the permissions above already include the necessary CloudWatch Logs, X-Ray, and CloudWatch Metrics access. You just need to complete the one-time setup in your **Role Home Account**.

**One-time Role Home Account Setup (required for Agent Logging):**

1. **Enable Transaction Search** in CloudWatch Console:
   - Navigate to CloudWatch → Settings → Transaction Search
   - Enable the Transaction Search feature

2. **Set X-Ray Destination** to CloudWatch Logs (run this command while authenticated to your Role Home Account):
   ```bash
   # Ensure you are authenticated to your Role Home Account before running
   aws xray update-trace-segment-destination --destination CloudWatchLogs
   ```

   This step is critical - it tells X-Ray to route trace spans to CloudWatch Logs. X-Ray automatically creates and manages the `aws/spans` log group in your Role Home Account.

**How Spans Flow (automatic once configured):**
```
AWS Dash agent → X-Ray OTLP endpoint → X-Ray → aws/spans log group (Role Home Account) → GenAI Dashboard
```

**PII Redaction (Optional):**

Agent Logging captures full prompts and responses for debugging. To enable automatic PII masking:
- See [AWS CloudWatch Data Protection](https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/protect-sensitive-log-data-types.html) for log data masking configuration.

5. **Complete Permission Set Creation**
   - Review the permission set configuration
   - Click "Create"

## Step 3: Create CloudFormation Deployment Role via StackSets

The `CloudFormation-Deployment-Role` is required in each AWS account where you want to deploy CloudFormation templates using AWS Dash Architect.

1. **Navigate to CloudFormation StackSets** (from your Organization's management/payer account)
   - Go to CloudFormation → StackSets
   - Click "Create StackSet"

2. **Upload CloudFormation Template**
   Create a new file `cloudformation-deployment-role.yaml` with the following content:

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: 'CloudFormation Deployment Role for AWS Dash Architect'

Resources:
  CloudFormationDeploymentRole:
    Type: AWS::IAM::Role
    Properties:
      RoleName: CloudFormation-Deployment-Role
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              Service: cloudformation.amazonaws.com
            Action: sts:AssumeRole
      ManagedPolicyArns:
        - arn:aws:iam::aws:policy/PowerUserAccess
      Policies:
        - PolicyName: IAMFullAccess
          PolicyDocument:
            Version: '2012-10-17'
            Statement:
              - Effect: Allow
                Action: 'iam:*'
                Resource: '*'

Outputs:
  RoleArn:
    Description: 'ARN of the CloudFormation Deployment Role'
    Value: !GetAtt CloudFormationDeploymentRole.Arn
    Export:
      Name: !Sub '${AWS::StackName}-CloudFormationDeploymentRoleArn'
```

3. **Deploy via StackSets**
   - Upload the template
   - Configure deployment options:
     - **Deployment targets**: All organizational units or specific accounts
     - **Regions**: Primary regions where you'll deploy resources
   - Deploy the StackSet

## Step 4: Assign Users to Permission Set

1. **Navigate to AWS Identity Center → Permission sets**
2. **Select your `awsdash` permission set**
3. **Click "Assign users or groups"**
4. **Select users/groups** who should have access to AWS Dash Architect
5. **Select AWS accounts** where they should have access
6. **Complete assignment**

## Security Considerations and Role Restrictions

### Default Configuration (Full Access)
The above configuration provides comprehensive access suitable for development and testing environments. This setup allows users to:
- Read all AWS resources across assigned accounts
- Deploy CloudFormation templates with broad permissions
- Access Amazon Bedrock models (Claude Haiku and Sonnet)
- Manage the AWS Dash application data

### Restricting Permissions for Production Use

**For organizations with strict security policies, consider these restrictions:**

1. **Multiple Permission Sets by Team**
   ```
   awsdash-developers    # Full access for development teams
   awsdash-architects    # Architecture design only (no deployment)
   awsdash-readonly      # View-only access for auditing
   ```

2. **Account-Specific Deployment Roles**
   - Create different `CloudFormation-Deployment-Role-Dev`, `CloudFormation-Deployment-Role-Prod`
   - Restrict permissions based on environment:
     - **Dev**: Full PowerUserAccess + IAM
     - **Staging**: Limited resource creation
     - **Prod**: Approval-gated deployments only

3. **Service-Specific Restrictions**
   - Modify the CloudFormation deployment role to restrict specific AWS services
   - Example: Remove access to sensitive services like IAM in production

4. **Regional Restrictions**
   - Limit DynamoDB access to specific regions
   - Restrict CloudFormation operations to approved regions

5. **Bedrock Model Restrictions**
   - Limit access to specific foundation models
   - Implement cost controls through IAM conditions

### Who Should Create What

- **AWS Organization Admins**: Create StackSets and organization-wide roles
- **Security Teams**: Define permission boundaries and create restricted permission sets
- **Team Leads**: Assign users to appropriate permission sets based on roles
- **Individual Users**: No setup required once assigned to permission sets

### Why These Permissions Are Needed

- **ReadOnlyAccess**: Enables cross-account resource discovery and architecture visualization
- **DynamoDB Access** (Role Home Account): Stores application preferences, project data, and cached resource information
- **Bedrock Access** (Role Home Account): Powers the specialized AI agents - models must be enabled in this account
- **CloudFormation Access**: Enables template validation, deployment, and stack management
- **IAM PassRole**: Allows CloudFormation to assume the deployment role during stack operations
- **CloudWatch Logs Access** (Role Home Account): Enables Agent Logging for monitoring and evaluating AI agent behavior
- **X-Ray Access** (Role Home Account): Sends traces to CloudWatch Gen AI Observability dashboard for analysis
- **CloudWatch Metrics Access** (Role Home Account): Publishes agent performance metrics to bedrock-agentcore namespace