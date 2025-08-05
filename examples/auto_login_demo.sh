#!/bin/bash

# AWS Dash Automated Login Demo Script
# This demonstrates how to use the automated login feature

echo "AWS Dash Automated Login Demo"
echo "============================="
echo ""

# Method 1: Using command-line arguments
echo "Method 1: Command-line arguments"
echo "--------------------------------"
echo "cargo run -- --auto-login --identity-url https://mycompany.awsapps.com/start/ --region us-east-1 --role DeveloperRole"
echo ""

# Method 2: Using environment variables
echo "Method 2: Environment variables"
echo "------------------------------"
echo "export AWS_DASH_AUTO_LOGIN=true"
echo "export AWS_IDENTITY_CENTER_URL=https://mycompany.awsapps.com/start/"
echo "export AWS_IDENTITY_CENTER_REGION=us-east-1"
echo "export AWS_DEFAULT_ROLE=DeveloperRole"
echo "cargo run"
echo ""

# Method 3: Help command
echo "Method 3: View all options"
echo "-------------------------"
echo "cargo run -- --help"
echo ""

echo "Demo: Running with --help to show available options:"
cargo run -- --help 2>/dev/null || echo "(Build required first)"