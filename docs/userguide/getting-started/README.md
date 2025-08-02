# Getting Started

Welcome to AWS Dash Architect! This guide will walk you through everything you need to get up and running with the application.

## Prerequisites

⚠️ **AWS Identity Center is Required**: AWS Dash Architect requires AWS Identity Center (formerly AWS SSO) with specific permission sets and roles configured before first use.

## Setup Steps

Follow these steps in order to get AWS Dash Architect working in your environment:

1. **[Setup IAM Identity Center](setup-iam-identity-center.md)** - Configure AWS Identity Center with the required permission sets and deployment roles
2. **[Login into AWS Dash Architect](login-aws-dash-architect.md)** - Complete the first launch and authentication process

## What You'll Need

Before starting the setup process, make sure you have:

- **AWS Organization Admin Access** - To create StackSets and organization-wide roles
- **AWS Identity Center Access** - To create permission sets and assign users
- **AWS Account Access** - To the accounts where you want to deploy CloudFormation templates

## Getting Help

If you encounter issues during setup:

- Check the [Troubleshooting Guide](../troubleshooting.md)
- Review the application logs at `$HOME/.local/share/awsdash/logs/awsdash.log`
- Submit issues on [GitHub](https://github.com/aws/aws-dash-architect/issues)