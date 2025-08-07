#!/usr/bin/env bash
# Memory Monitor Script for AWS Dash Testing
# Monitors memory usage during test execution and stops tests if memory exceeds threshold

set -e

# Configuration
MEMORY_THRESHOLD=${MEMORY_THRESHOLD:-95}  # Default 95% threshold
CHECK_INTERVAL=${CHECK_INTERVAL:-2}       # Check every 2 seconds
LOG_FILE=${LOG_FILE:-"memory-monitor.log"}
MEMORY_LOG_DIR="${HOME}/.local/share/awsdash/logs"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create log directory if it doesn't exist
mkdir -p "$MEMORY_LOG_DIR"
FULL_LOG_PATH="$MEMORY_LOG_DIR/$LOG_FILE"

# Function to get memory usage percentage
get_memory_usage() {
    local mem_info=$(free | grep '^Mem:')
    local total=$(echo $mem_info | awk '{print $2}')
    local used=$(echo $mem_info | awk '{print $3}')
    local usage_percent=$((used * 100 / total))
    echo $usage_percent
}

# Function to get detailed memory info
get_detailed_memory_info() {
    echo "=== Memory Info $(date) ==="
    free -h
    echo ""
    echo "=== Top Memory Consumers ==="
    ps aux --sort=-%mem | head -10
    echo ""
    echo "=== System Load ==="
    uptime
    echo ""
}

# Function to log memory usage
log_memory_usage() {
    local usage=$1
    local test_context=$2
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] Memory: ${usage}% | Context: $test_context" >> "$FULL_LOG_PATH"
}

# Function to handle memory threshold breach
handle_memory_breach() {
    local usage=$1
    local test_context=$2
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    echo -e "${RED}🚨 MEMORY THRESHOLD BREACHED! ${usage}% > ${MEMORY_THRESHOLD}%${NC}"
    echo "[$timestamp] MEMORY BREACH: ${usage}% > ${MEMORY_THRESHOLD}% | Context: $test_context" >> "$FULL_LOG_PATH"
    
    # Log detailed memory info
    get_detailed_memory_info >> "$FULL_LOG_PATH"
    
    # Kill test processes
    echo "Attempting to stop test processes..."
    pkill -f "cargo test" || true
    pkill -f "rustc" || true
    
    echo -e "${RED}Tests stopped due to memory usage exceeding ${MEMORY_THRESHOLD}%${NC}"
    echo -e "${BLUE}Check log file: $FULL_LOG_PATH${NC}"
    
    exit 1
}

# Function to monitor memory during command execution
monitor_memory_with_command() {
    local cmd="$1"
    local test_name="$2"
    
    echo -e "${GREEN}🔍 Starting memory monitor for: $test_name${NC}"
    echo "Memory threshold: ${MEMORY_THRESHOLD}%"
    echo "Check interval: ${CHECK_INTERVAL}s"
    echo "Log file: $FULL_LOG_PATH"
    echo ""
    
    # Initialize log
    echo "=== Memory Monitor Started $(date) ===" >> "$FULL_LOG_PATH"
    echo "Command: $cmd" >> "$FULL_LOG_PATH"
    echo "Test: $test_name" >> "$FULL_LOG_PATH"
    echo "Threshold: ${MEMORY_THRESHOLD}%" >> "$FULL_LOG_PATH"
    echo "" >> "$FULL_LOG_PATH"
    
    # Start the command in background
    $cmd &
    local cmd_pid=$!
    
    # Monitor memory while command runs
    while kill -0 $cmd_pid 2>/dev/null; do
        local memory_usage=$(get_memory_usage)
        log_memory_usage "$memory_usage" "$test_name"
        
        echo -e "${BLUE}[$(date '+%H:%M:%S')] Memory: ${YELLOW}${memory_usage}%${NC} | Test: $test_name"
        
        if [ "$memory_usage" -gt "$MEMORY_THRESHOLD" ]; then
            handle_memory_breach "$memory_usage" "$test_name"
        fi
        
        sleep $CHECK_INTERVAL
    done
    
    # Wait for command to complete and get exit code
    wait $cmd_pid
    local exit_code=$?
    
    local final_memory=$(get_memory_usage)
    if [ "$exit_code" -eq 0 ]; then
        echo -e "${GREEN}✅ Test completed successfully. Final memory usage: ${final_memory}%${NC}"
    else
        echo -e "${YELLOW}⚠️  Test completed with failures (exit code: $exit_code). Final memory usage: ${final_memory}%${NC}"
    fi
    echo "=== Test Completed $(date) | Exit Code: $exit_code | Final Memory: ${final_memory}% ===" >> "$FULL_LOG_PATH"
    echo "" >> "$FULL_LOG_PATH"
    
    # Always return 0 to continue testing even if individual tests fail
    # Only stop for memory threshold breaches (handled in handle_memory_breach)
    return 0
}

# Function to show usage
show_usage() {
    echo "Memory Monitor Script for AWS Dash Testing"
    echo ""
    echo "Usage:"
    echo "  $0 --monitor \"command\" \"test_name\""
    echo "  $0 --check"
    echo "  $0 --help"
    echo ""
    echo "Options:"
    echo "  --monitor   Monitor memory usage while running a command"
    echo "  --check     Check current memory usage and exit"
    echo "  --help      Show this help message"
    echo ""
    echo "Environment variables:"
    echo "  MEMORY_THRESHOLD    Memory threshold percentage (default: 95)"
    echo "  CHECK_INTERVAL      Check interval in seconds (default: 2)"
    echo "  LOG_FILE           Log file name (default: memory-monitor.log)"
    echo ""
    echo "Examples:"
    echo "  $0 --monitor \"./scripts/test-chunks.sh all\" \"Full Test Suite\""
    echo "  $0 --monitor \"cargo test --test aws_real_world_templates -j 1\" \"Integration Tests\""
    echo "  MEMORY_THRESHOLD=90 $0 --monitor \"cargo build\" \"Build Process\""
    echo ""
    echo "Log location: $MEMORY_LOG_DIR/$LOG_FILE"
}

# Function to just check current memory
check_memory() {
    local usage=$(get_memory_usage)
    echo -e "${BLUE}Current memory usage: ${YELLOW}${usage}%${NC}"
    
    if [ "$usage" -gt "$MEMORY_THRESHOLD" ]; then
        echo -e "${RED}⚠️  Memory usage is above threshold (${MEMORY_THRESHOLD}%)${NC}"
        get_detailed_memory_info
    else
        echo -e "${GREEN}✅ Memory usage is within threshold (${MEMORY_THRESHOLD}%)${NC}"
    fi
}

# Main script logic
case "${1:-}" in
    "--monitor")
        if [ -z "$2" ] || [ -z "$3" ]; then
            echo -e "${RED}Error: --monitor requires command and test name${NC}"
            echo "Usage: $0 --monitor \"command\" \"test_name\""
            exit 1
        fi
        monitor_memory_with_command "$2" "$3"
        ;;
    "--check")
        check_memory
        ;;
    "--help"|"-h"|"")
        show_usage
        ;;
    *)
        echo -e "${RED}Unknown option: $1${NC}"
        show_usage
        exit 1
        ;;
esac