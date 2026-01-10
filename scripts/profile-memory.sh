#!/bin/bash
# Memory profiling script using bytehound
# Automates the process of profiling, analysis, and visualization

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BYTEHOUND_LIB="$HOME/.local/lib/libbytehound.so"
BYTEHOUND_BIN="$HOME/.local/bin/bytehound"
PROFILE_DIR="$PROJECT_ROOT/memory-profiles"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  AWS Dash Memory Profiling Automation${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# Check if bytehound is installed
if [ ! -f "$BYTEHOUND_LIB" ] || [ ! -f "$BYTEHOUND_BIN" ]; then
    echo -e "${RED}Error: Bytehound not found!${NC}"
    echo "Expected locations:"
    echo "  Library: $BYTEHOUND_LIB"
    echo "  Binary: $BYTEHOUND_BIN"
    exit 1
fi

# Create profile directory
mkdir -p "$PROFILE_DIR"
cd "$PROFILE_DIR"

# Check if app is built
if [ ! -f "$PROJECT_ROOT/target/debug/awsdash" ]; then
    echo -e "${YELLOW}Debug build not found. Building now...${NC}"
    cd "$PROJECT_ROOT"
    cargo build
    cd "$PROFILE_DIR"
fi

# Generate timestamp for this profiling session
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
SESSION_NAME="memory-profile-${TIMESTAMP}"

echo -e "${GREEN}✓ Setup complete${NC}"
echo
echo -e "${YELLOW}Profiling session: ${SESSION_NAME}${NC}"
echo -e "${YELLOW}Profile data will be saved to: ${PROFILE_DIR}/${NC}"
echo

# Set bytehound options
export MEMORY_PROFILER_OUTPUT="${SESSION_NAME}.dat"
export MEMORY_PROFILER_LOG=info

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Testing Instructions${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo
echo -e "${GREEN}The application will now start with memory profiling enabled.${NC}"
echo
echo -e "${YELLOW}Please perform these actions:${NC}"
echo "  1. Wait for app to fully load (baseline memory)"
echo "  2. Execute a query to load ~314 resources"
echo "  3. Wait for Phase 1 completion"
echo "  4. Wait for Phase 2 enrichment to complete"
echo "  5. Navigate the tree:"
echo "     - Expand/collapse nodes"
echo "     - Scroll through resources"
echo "     - Change grouping modes"
echo "  6. Quit the application normally (File > Quit)"
echo
echo -e "${YELLOW}What to observe:${NC}"
echo "  - Baseline memory after startup: ~100-200MB expected"
echo "  - After loading resources: ~200-400MB expected (NOT 1015MB!)"
echo "  - No continuous growth during tree navigation"
echo
echo -e "${RED}Press ENTER to start the profiled application...${NC}"
read

echo -e "${GREEN}Starting application with bytehound profiler...${NC}"
echo

# Run the application with bytehound
cd "$PROJECT_ROOT"
LD_PRELOAD="$BYTEHOUND_LIB" \
  target/debug/awsdash

echo
echo -e "${GREEN}✓ Application closed${NC}"
echo

# Check if profile was generated
PROFILE_FILE="$PROFILE_DIR/${SESSION_NAME}.dat"
if [ ! -f "$PROFILE_FILE" ]; then
    echo -e "${RED}Error: Profile file not generated!${NC}"
    echo "Expected: $PROFILE_FILE"
    echo
    echo "Checking for any .dat files in profile directory..."
    LATEST_PROFILE=$(ls -t "$PROFILE_DIR"/*.dat 2>/dev/null | head -1)
    if [ -n "$LATEST_PROFILE" ]; then
        echo -e "${YELLOW}Found profile: $LATEST_PROFILE${NC}"
        PROFILE_FILE="$LATEST_PROFILE"
    else
        echo -e "${RED}No profile files found!${NC}"
        exit 1
    fi
fi

PROFILE_SIZE=$(du -h "$PROFILE_FILE" | cut -f1)
echo -e "${GREEN}✓ Profile generated: $PROFILE_FILE (${PROFILE_SIZE})${NC}"
echo

# Generate a quick summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Quick Analysis${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# Extract basic stats using bytehound script
cat > /tmp/bytehound-summary.js << 'EOF'
const data = get_data();
const allocations = data.allocations();

let total_allocated = 0;
let total_freed = 0;
let live_allocations = 0;
let peak_memory = 0;

allocations.forEach(alloc => {
    total_allocated += alloc.size;
    if (alloc.deallocation) {
        total_freed += alloc.size;
    } else {
        live_allocations += alloc.size;
    }

    const current_memory = total_allocated - total_freed;
    if (current_memory > peak_memory) {
        peak_memory = current_memory;
    }
});

console.log("Total allocated: " + (total_allocated / 1024 / 1024).toFixed(2) + " MB");
console.log("Total freed: " + (total_freed / 1024 / 1024).toFixed(2) + " MB");
console.log("Live allocations: " + (live_allocations / 1024 / 1024).toFixed(2) + " MB");
console.log("Peak memory: " + (peak_memory / 1024 / 1024).toFixed(2) + " MB");
EOF

echo -e "${YELLOW}Running analysis script...${NC}"
"$BYTEHOUND_BIN" script "$PROFILE_FILE" /tmp/bytehound-summary.js 2>/dev/null || echo -e "${YELLOW}(Script analysis failed - will use web UI)${NC}"
echo

# Start the web server
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Starting Web UI${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo
echo -e "${GREEN}Launching bytehound web server...${NC}"
echo -e "${YELLOW}Web UI will open at: http://localhost:8080${NC}"
echo
echo -e "${YELLOW}Key sections to check:${NC}"
echo "  📊 Timeline - Memory usage over time"
echo "  🔥 Flamegraph - Allocation sources"
echo "  📈 Live Allocations - Currently allocated memory"
echo "  🎯 Top Allocators - Which code allocates most"
echo
echo -e "${YELLOW}What to verify:${NC}"
echo "  ✓ Peak memory ~200-400MB (not 1015MB)"
echo "  ✓ No continuous growth during tree navigation"
echo "  ✓ TreeNode allocations are small (indices, not full resources)"
echo "  ✓ Stable memory after query completion"
echo
echo -e "${RED}Press Ctrl+C to stop the web server when done.${NC}"
echo

# Try to open browser automatically
if command -v xdg-open &> /dev/null; then
    sleep 2 && xdg-open http://localhost:8080 &
elif command -v firefox &> /dev/null; then
    sleep 2 && firefox http://localhost:8080 &
fi

# Start the server (blocks until Ctrl+C)
"$BYTEHOUND_BIN" server "$PROFILE_FILE"
