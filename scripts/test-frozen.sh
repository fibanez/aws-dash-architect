#!/bin/bash

# Script to run frozen functionality tests
set -e

echo "Running AWS Dash Frozen Functionality Tests"
echo "=========================================="

# Create necessary directories
mkdir -p tests/fixtures tests/snapshots

# Run contract tests first (these are the most important)
echo -e "\n1. Running API Contract Tests..."
cargo test test_api_contract -j 1 -- --nocapture

# Run snapshot tests
echo -e "\n2. Running Snapshot Tests..."
cargo test test_aws_identity test_projects test_cfn_dag test_cfn_resources test_bedrock_client -j 1 -- --nocapture

# Check for snapshot changes
echo -e "\n3. Checking for Snapshot Changes..."
if cargo insta test | grep -q "snapshots remaining"; then
    echo "WARNING: Snapshot changes detected!"
    echo "Run 'cargo insta review' to review changes"
    exit 1
else
    echo "All snapshots match!"
fi

# Run golden file tests
echo -e "\n4. Running Golden File Tests..."
cargo test golden -j 1 -- --nocapture

# Summary
echo -e "\n=========================================="
echo "Frozen Functionality Test Summary:"
echo "✓ API contracts verified"
echo "✓ Data structures unchanged" 
echo "✓ File formats stable"
echo -e "\nAll frozen functionality tests passed!"