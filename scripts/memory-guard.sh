#!/usr/bin/env bash
# Memory Guard - System-level protection against memory exhaustion during cargo tests
# This script provides additional protection layers to prevent system crashes

set -e

# Configuration
MEMORY_EMERGENCY_THRESHOLD=${MEMORY_EMERGENCY_THRESHOLD:-95}
MEMORY_WARNING_THRESHOLD=${MEMORY_WARNING_THRESHOLD:-85}
SWAP_EMERGENCY_THRESHOLD=${SWAP_EMERGENCY_THRESHOLD:-90}
CHECK_INTERVAL=${CHECK_INTERVAL:-3}

# Colors
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

show_usage() {
    echo "Memory Guard - System-level protection against memory exhaustion"
    echo ""
    echo "Usage: $0 [options] -- <command>"
    echo ""
    echo "Options:"
    echo "  --memory-emergency N    Set memory emergency threshold (default: 95%)"
    echo "  --memory-warning N      Set memory warning threshold (default: 85%)"
    echo "  --swap-emergency N      Set swap emergency threshold (default: 90%)"
    echo "  --interval N            Set check interval in seconds (default: 3)"
    echo "  --help                  Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 -- cargo test --test guard_test"
    echo "  $0 --memory-emergency 90 -- ./scripts/test-chunks.sh core"
    echo "  $0 --interval 1 -- cargo build"
    echo ""
    echo "Environment Variables:"
    echo "  MEMORY_EMERGENCY_THRESHOLD    Memory emergency threshold percentage"
    echo "  MEMORY_WARNING_THRESHOLD      Memory warning threshold percentage"
    echo "  SWAP_EMERGENCY_THRESHOLD      Swap emergency threshold percentage"
    echo "  CHECK_INTERVAL                Check interval in seconds"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --memory-emergency)
            MEMORY_EMERGENCY_THRESHOLD="$2"
            shift 2
            ;;
        --memory-warning)
            MEMORY_WARNING_THRESHOLD="$2"
            shift 2
            ;;
        --swap-emergency)
            SWAP_EMERGENCY_THRESHOLD="$2"
            shift 2
            ;;
        --interval)
            CHECK_INTERVAL="$2"
            shift 2
            ;;
        --help|-h)
            show_usage
            exit 0
            ;;
        --)
            shift
            break
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_usage
            exit 1
            ;;
    esac
done

if [ $# -eq 0 ]; then
    echo -e "${RED}Error: No command specified${NC}"
    show_usage
    exit 1
fi

echo -e "${GREEN}🛡️  Memory Guard - Protecting against memory exhaustion${NC}"
echo "=============================================="
echo "Memory emergency threshold: ${MEMORY_EMERGENCY_THRESHOLD}%"
echo "Memory warning threshold: ${MEMORY_WARNING_THRESHOLD}%"
echo "Swap emergency threshold: ${SWAP_EMERGENCY_THRESHOLD}%"
echo "Check interval: ${CHECK_INTERVAL}s"
echo "Command: $*"
echo ""

# Set up trap to clean up background processes
cleanup() {
    if [ ! -z "$monitor_pid" ]; then
        kill $monitor_pid 2>/dev/null || true
        wait $monitor_pid 2>/dev/null || true
    fi
    if [ ! -z "$command_pid" ]; then
        kill $command_pid 2>/dev/null || true
        wait $command_pid 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

# Start the command in background
echo -e "${GREEN}🚀 Starting command...${NC}"
"$@" &
command_pid=$!

# Memory monitoring function
monitor_memory() {
    local last_warning_time=0
    
    while kill -0 $command_pid 2>/dev/null; do
        # Get memory and swap usage
        local memory_info=$(free | grep -E '^(Mem|Swap):')
        local memory_usage=$(echo "$memory_info" | awk '/^Mem:/{printf("%.1f"), $3/$2 * 100}')
        local swap_usage=$(echo "$memory_info" | awk '/^Swap:/{if($2>0) printf("%.1f"), $3/$2 * 100; else print "0"}')
        
        local current_time=$(date +%s)
        
        # Check for emergency conditions
        if (( $(echo "$memory_usage > $MEMORY_EMERGENCY_THRESHOLD" | bc -l) )) || \
           (( $(echo "$swap_usage > $SWAP_EMERGENCY_THRESHOLD" | bc -l) )); then
            echo -e "${RED}🚨 EMERGENCY STOP: Memory at ${memory_usage}%, Swap at ${swap_usage}%${NC}"
            echo -e "${RED}🚨 Killing command to prevent system crash${NC}"
            
            # Kill the command and all its children
            pkill -P $command_pid 2>/dev/null || true
            kill $command_pid 2>/dev/null || true
            
            # Also kill any cargo/rust processes that might be hanging
            pkill -f "cargo.*test" 2>/dev/null || true
            pkill -f "rustc" 2>/dev/null || true
            pkill -f "ld" 2>/dev/null || true
            
            exit 130  # Exit with signal termination code
        fi
        
        # Check for warning conditions (but don't spam)
        if (( $(echo "$memory_usage > $MEMORY_WARNING_THRESHOLD" | bc -l) )) && \
           (( current_time - last_warning_time > 10 )); then
            echo -e "${YELLOW}⚠️  High memory usage: ${memory_usage}% RAM, ${swap_usage}% Swap${NC}"
            last_warning_time=$current_time
        fi
        
        sleep $CHECK_INTERVAL
    done
}

# Start memory monitoring in background
monitor_memory &
monitor_pid=$!

# Wait for command to complete
wait $command_pid
command_exit_code=$?

# Clean up monitor
kill $monitor_pid 2>/dev/null || true
wait $monitor_pid 2>/dev/null || true

echo ""
if [ $command_exit_code -eq 0 ]; then
    echo -e "${GREEN}✅ Command completed successfully${NC}"
else
    echo -e "${YELLOW}⚠️  Command exited with code $command_exit_code${NC}"
fi

exit $command_exit_code