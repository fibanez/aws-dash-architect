#!/usr/bin/env bash
# Safe Test Runner - Comprehensive memory protection for cargo tests
# Combines all protection mechanisms to prevent system crashes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

show_usage() {
    echo "Safe Test Runner - Comprehensive memory protection for cargo tests"
    echo ""
    echo "Usage: $0 [test_chunk] [options]"
    echo ""
    echo "Test chunks:"
    echo "  core         Run core tests"
    echo "  cfn          Run CloudFormation tests"
    echo "  ui           Run UI tests"
    echo "  projects     Run project tests"
    echo "  integration  Run integration tests"
    echo "  docs         Run documentation tests"
    echo "  fast         Run fast test suite (core, cfn, ui, projects, docs)"
    echo "  all          Run all tests including integration"
    echo ""
    echo "Protection options:"
    echo "  --no-guard              Skip memory guard protection"
    echo "  --memory-emergency N    Memory emergency threshold (default: 95%)"
    echo "  --memory-warning N      Memory warning threshold (default: 85%)"
    echo "  --help                  Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 core                           # Run core tests with full protection"
    echo "  $0 integration --memory-emergency 90   # Run with custom emergency threshold"
    echo "  $0 fast --no-guard                # Run fast tests without memory guard"
}

# Default values
USE_GUARD=true
MEMORY_EMERGENCY=95
MEMORY_WARNING=85
TEST_CHUNK=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-guard)
            USE_GUARD=false
            shift
            ;;
        --memory-emergency)
            MEMORY_EMERGENCY="$2"
            shift 2
            ;;
        --memory-warning)
            MEMORY_WARNING="$2"
            shift 2
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

echo -e "${BLUE}🛡️  Safe Test Runner - Memory Protected Cargo Testing${NC}"
echo "====================================================="
echo "Test chunk: $TEST_CHUNK"
echo "Memory guard: $([ "$USE_GUARD" = true ] && echo "ENABLED" || echo "DISABLED")"
[ "$USE_GUARD" = true ] && echo "Emergency threshold: ${MEMORY_EMERGENCY}%"
[ "$USE_GUARD" = true ] && echo "Warning threshold: ${MEMORY_WARNING}%"
echo ""

# Pre-flight checks
echo -e "${YELLOW}🔍 Pre-flight checks...${NC}"

# Check available memory
available_memory_gb=$(free -g | awk '/^Mem:/{print $7}')
total_memory_gb=$(free -g | awk '/^Mem:/{print $2}')
echo "Available memory: ${available_memory_gb}GB / ${total_memory_gb}GB"

if [ "$available_memory_gb" -lt 2 ]; then
    echo -e "${YELLOW}⚠️  Warning: Low available memory (< 2GB). Consider freeing up memory before testing.${NC}"
fi

# Check cargo configuration
if [ -f .cargo/config.toml ]; then
    echo "✓ Local cargo config found (jobs limited to 1)"
else
    echo -e "${YELLOW}⚠️  No local cargo config found. Consider creating .cargo/config.toml${NC}"
fi

# Check if sccache is available
if command -v sccache >/dev/null 2>&1; then
    echo "✓ sccache available for compilation caching"
else
    echo -e "${YELLOW}⚠️  sccache not found. Consider installing for faster compilation${NC}"
fi

echo ""

# Set up environment for maximum memory safety
export CARGO_BUILD_JOBS=1
export RUST_BACKTRACE=0
export CARGO_NET_GIT_FETCH_WITH_CLI=true

# Set conservative ulimits
ulimit -v 6291456 2>/dev/null || echo "⚠️  Could not set virtual memory limit (requires privileged access)"
ulimit -m 4194304 2>/dev/null || echo "⚠️  Could not set resident memory limit (requires privileged access)"

# Construct the test command
TEST_CMD="$SCRIPT_DIR/test-chunks.sh $TEST_CHUNK"

if [ "$USE_GUARD" = true ]; then
    echo -e "${GREEN}🛡️  Running with memory guard protection...${NC}"
    echo ""
    
    # Run with memory guard
    "$SCRIPT_DIR/memory-guard.sh" \
        --memory-emergency "$MEMORY_EMERGENCY" \
        --memory-warning "$MEMORY_WARNING" \
        --interval 3 \
        -- $TEST_CMD
else
    echo -e "${YELLOW}⚠️  Running WITHOUT memory guard protection${NC}"
    echo ""
    
    # Run tests directly
    $TEST_CMD
fi

# Post-test cleanup
echo ""
echo -e "${BLUE}🧹 Post-test cleanup...${NC}"

# Kill any remaining cargo/rust processes
pkill -f "cargo.*test" 2>/dev/null || true
pkill -f "rustc" 2>/dev/null || true

# Show final memory status
echo ""
echo -e "${GREEN}📊 Final system status:${NC}"
free -h | grep -E '^(Mem|Swap):'

echo ""
echo -e "${GREEN}✅ Safe test execution completed!${NC}"