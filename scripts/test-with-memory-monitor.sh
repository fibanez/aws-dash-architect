#!/usr/bin/env bash
# Test Runner with Memory Monitoring
# Wrapper script that runs tests with memory monitoring

set -e

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
MEMORY_THRESHOLD=${MEMORY_THRESHOLD:-95}
CHECK_INTERVAL=${CHECK_INTERVAL:-2}

show_usage() {
    echo "Test Runner with Memory Monitoring"
    echo ""
    echo "Usage:"
    echo "  $0 [test_chunk] [options]"
    echo ""
    echo "Test chunks (same as test-chunks.sh):"
    echo "  core         Run core tests (frozen, API contracts, unit tests)"
    echo "  cfn          Run CloudFormation logic tests"
    echo "  ui           Run UI component tests"
    echo "  projects     Run project management tests"
    echo "  integration  Run integration tests (long-running)"
    echo "  docs         Run documentation tests"
    echo "  fast         Run chunks 1-4 (excludes integration tests)"
    echo "  all          Run all test chunks including integration"
    echo ""
    echo "Memory monitoring options:"
    echo "  --threshold N    Set memory threshold percentage (default: 95)"
    echo "  --interval N     Set check interval in seconds (default: 2)"
    echo "  --no-monitor     Run tests without memory monitoring"
    echo ""
    echo "Environment variables:"
    echo "  MEMORY_THRESHOLD    Memory threshold percentage"
    echo "  CHECK_INTERVAL      Check interval in seconds"
    echo ""
    echo "Examples:"
    echo "  $0 fast                           # Run fast tests with 95% memory threshold"
    echo "  $0 all --threshold 90             # Run all tests with 90% memory threshold"
    echo "  $0 integration --interval 1       # Run integration tests, check every 1 second"
    echo "  $0 core --no-monitor              # Run core tests without memory monitoring"
    echo ""
    echo "Note: Tests continue running even if individual test chunks fail."
    echo "      Only memory threshold breaches will stop the test suite."
    echo ""
    echo "Logs are saved to: \$HOME/.local/share/awsdash/logs/memory-monitor.log"
}

# Parse arguments
TEST_CHUNK=""
USE_MONITOR=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --threshold)
            MEMORY_THRESHOLD="$2"
            shift 2
            ;;
        --interval)
            CHECK_INTERVAL="$2"
            shift 2
            ;;
        --no-monitor)
            USE_MONITOR=false
            shift
            ;;
        --help|-h)
            show_usage
            exit 0
            ;;
        core|cfn|ui|projects|integration|docs|fast|all)
            TEST_CHUNK="$1"
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_usage
            exit 1
            ;;
    esac
done

# Default to fast if no test chunk specified
if [ -z "$TEST_CHUNK" ]; then
    TEST_CHUNK="fast"
fi

# Export environment variables for memory monitor
export MEMORY_THRESHOLD
export CHECK_INTERVAL

echo -e "${BLUE}🧪 AWS Dash Test Runner with Memory Monitoring${NC}"
echo "=============================================="
echo "Test chunk: $TEST_CHUNK"
echo "Memory threshold: ${MEMORY_THRESHOLD}%"
echo "Check interval: ${CHECK_INTERVAL}s"
echo "Memory monitoring: $([ "$USE_MONITOR" = true ] && echo "ENABLED" || echo "DISABLED")"
echo ""

# Construct the test command
TEST_CMD="$SCRIPT_DIR/test-chunks.sh $TEST_CHUNK"

if [ "$USE_MONITOR" = true ]; then
    echo -e "${GREEN}🔍 Starting tests with memory monitoring...${NC}"
    echo ""
    
    # Run with memory monitoring
    "$SCRIPT_DIR/memory-monitor.sh" --monitor "$TEST_CMD" "Test Chunk: $TEST_CHUNK"
else
    echo -e "${YELLOW}⚠️  Running tests WITHOUT memory monitoring${NC}"
    echo ""
    
    # Run tests directly
    $TEST_CMD
fi

echo ""
echo -e "${GREEN}✅ Test execution completed!${NC}"

# Show final memory status
echo ""
echo -e "${BLUE}Final memory status:${NC}"
"$SCRIPT_DIR/memory-monitor.sh" --check