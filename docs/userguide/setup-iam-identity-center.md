# Setup IAM Identity Center

Before using AWS Dash Architect, you must configure AWS Identity Center with the required permission sets and deployment roles. This setup provides secure access to AWS resources and enables the application's specialized AI agents to function properly.

## Step 1: Create AWS Dash Permission Set

1. **Navigate to AWS Identity Center**
   - Log into your AWS Management Console
   - Go to AWS Identity Center (successor to AWS Single Sign-On)
   - Select "Permission sets" from the left navigation

2. **Create New Permission Set**
   - Click "Create permission set"
   - Choose "Custom permission set"
   - Name: `awsdash` (or your preferred name)
   - Description: "AWS Dash Architect access with Bedrock and CloudFormation deployment permissions"

3. **Attach AWS Managed Policy**
   - Under "AWS managed policies", search and attach:
     - **ReadOnlyAccess** (Job function category)

4. **Create Inline Policy**
   - Click "Create inline policy"
   - Switch to JSON view and paste the following policy:

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
                "arn:aws:dynamodb:us-east-1:*:table/awsdash"
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
                "DescribeChangeSet",
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
        }
    ]
}
```

   - Name the policy: `AWSDashArchitectInlinePolicy`
   - Click "Create policy"

5. **Complete Permission Set Creation**
   - Review the permission set configuration
   - Click "Create"

## Step 2: Create CloudFormation Deployment Role via StackSets

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

## Step 3: Assign Users to Permission Set

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
- **DynamoDB Access**: Stores application preferences, project data, and cached resource information
- **Bedrock Access**: Powers the specialized AI agents (Planner, Architect, Builder)
- **CloudFormation Access**: Enables template validation, deployment, and stack management
- **IAM PassRole**: Allows CloudFormation to assume the deployment role during stack operations