#!/bin/bash

# AWS CloudFormation Templates Integration Test Runner
# This script runs integration tests against real-world AWS CloudFormation templates

set -e

echo "🚀 AWS CloudFormation Templates Integration Test Runner"
echo "======================================================="

# Check if git is available
if ! command -v git &> /dev/null; then
    echo "❌ Error: git is required but not installed"
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo is required but not installed"
    exit 1
fi

# Parse command line arguments
QUICK_MODE=false
PERFORMANCE_MODE=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --performance)
            PERFORMANCE_MODE=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --quick        Run a quick subset of templates for testing"
            echo "  --performance  Run only performance tests with large templates"
            echo "  --verbose, -v  Enable verbose output"
            echo "  --help, -h     Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                    # Run all integration tests"
            echo "  $0 --quick           # Run quick subset"
            echo "  $0 --performance     # Run performance tests only"
            echo "  $0 --verbose         # Run with detailed output"
            exit 0
            ;;
        *)
            echo "❌ Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Set up environment
export RUST_BACKTRACE=1
if [ "$VERBOSE" = true ]; then
    export RUST_LOG=debug
    export AWS_DASH_VERBOSE=true
fi

echo "📋 Configuration:"
echo "  Quick mode: $QUICK_MODE"
echo "  Performance mode: $PERFORMANCE_MODE" 
echo "  Verbose: $VERBOSE"
echo ""

# Build the project first
echo "🔨 Building project..."
cargo build --tests

if [ "$PERFORMANCE_MODE" = true ]; then
    echo "🏃 Running performance tests only..."
    cargo test --test aws_real_world_templates test_performance_with_large_templates -- --ignored --nocapture
elif [ "$QUICK_MODE" = true ]; then
    echo "⚡ Running quick integration test (limited templates)..."
    # For quick mode, we could add an environment variable to limit the number of templates
    QUICK_TEST_MODE=true cargo test --test aws_real_world_templates test_aws_cloudformation_templates_compatibility -- --ignored --nocapture
else
    echo "🧪 Running full integration test suite..."
    echo "⚠️  This may take 10-30 minutes depending on your system and network speed"
    echo ""
    
    # Run all integration tests
    cargo test --test aws_real_world_templates -- --ignored --nocapture
fi

echo ""
echo "✅ Integration tests completed!"
echo ""
echo "📊 Next steps:"
echo "  - Review the test report above for success rates and failure patterns"
echo "  - Check for templates with verification discrepancies"
echo "  - Use findings to prioritize enhancement work"
echo "  - Run with --performance flag to focus on performance analysis"