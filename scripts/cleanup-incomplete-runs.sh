#!/bin/bash
# Cleanup incomplete agent runs
#
# This script removes:
# - Agent log files from incomplete runs (no "Agent execution completed" message)
# - Orphaned page workspaces without corresponding completed logs

set -euo pipefail

# Configuration
LOG_DIR="$HOME/.local/share/awsdash/logs/agents"
PAGE_WORKSPACE_DIR="$HOME/.local/share/awsdash/pages"
MIN_AGE_HOURS=24  # Only clean files older than this

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Agent Run Cleanup ==="
echo "Log directory: $LOG_DIR"
echo "Page workspace directory: $PAGE_WORKSPACE_DIR"
echo "Minimum age: ${MIN_AGE_HOURS} hours"
echo

# Check if log directory exists
if [ ! -d "$LOG_DIR" ]; then
    echo -e "${YELLOW}Warning: Log directory does not exist: $LOG_DIR${NC}"
    echo "No logs to clean."
else
    # Find incomplete log files
    incomplete_logs=0
    total_size=0

    echo "Scanning for incomplete agent logs..."
    while IFS= read -r -d '' log_file; do
        # Check if log contains completion marker
        if ! grep -q "Agent execution completed" "$log_file" 2>/dev/null; then
            size=$(stat -c%s "$log_file" 2>/dev/null || echo 0)
            total_size=$((total_size + size))
            incomplete_logs=$((incomplete_logs + 1))

            echo -e "${RED}Removing:${NC} $(basename "$log_file") ($(numfmt --to=iec-i --suffix=B "$size" 2>/dev/null || echo "${size}B"))"
            rm "$log_file"
        fi
    done < <(find "$LOG_DIR" -name "*.log" -type f -mmin +$((MIN_AGE_HOURS * 60)) -print0 2>/dev/null)

    if [ $incomplete_logs -eq 0 ]; then
        echo -e "${GREEN}No incomplete logs found${NC}"
    else
        echo -e "${GREEN}Removed $incomplete_logs incomplete log file(s)${NC}"
        echo -e "Total space freed: $(numfmt --to=iec-i --suffix=B "$total_size" 2>/dev/null || echo "${total_size}B")"
    fi
fi

echo

# Check if page workspace directory exists
if [ ! -d "$PAGE_WORKSPACE_DIR" ]; then
    echo -e "${YELLOW}Warning: Page workspace directory does not exist: $PAGE_WORKSPACE_DIR${NC}"
    echo "No workspaces to clean."
else
    # Find orphaned page workspaces
    # A workspace is orphaned if there's no corresponding completed PageBuilder log
    orphaned_workspaces=0
    total_ws_size=0

    echo "Scanning for orphaned page workspaces..."
    while IFS= read -r -d '' workspace_dir; do
        workspace_name=$(basename "$workspace_dir")

        # Check if there's a completed PageBuilder log for this workspace
        # Look for logs containing the workspace name and completion marker
        found_completed=false
        while IFS= read -r -d '' log_file; do
            if grep -q "$workspace_name" "$log_file" 2>/dev/null && \
               grep -q "Agent execution completed" "$log_file" 2>/dev/null && \
               grep -q "PageBuilder" "$log_file" 2>/dev/null; then
                found_completed=true
                break
            fi
        done < <(find "$LOG_DIR" -name "*PageBuilder*.log" -type f -print0 2>/dev/null)

        # If no completed log found and workspace is old enough, remove it
        if [ "$found_completed" = false ]; then
            # Check age
            age_minutes=$(find "$workspace_dir" -maxdepth 0 -type d -mmin +$((MIN_AGE_HOURS * 60)) -print 2>/dev/null | wc -l)
            if [ "$age_minutes" -eq 1 ]; then
                size=$(du -sb "$workspace_dir" 2>/dev/null | cut -f1 || echo 0)
                total_ws_size=$((total_ws_size + size))
                orphaned_workspaces=$((orphaned_workspaces + 1))

                echo -e "${RED}Removing:${NC} $workspace_name ($(numfmt --to=iec-i --suffix=B "$size" 2>/dev/null || echo "${size}B"))"
                rm -rf "$workspace_dir"
            fi
        fi
    done < <(find "$PAGE_WORKSPACE_DIR" -mindepth 1 -maxdepth 1 -type d -print0 2>/dev/null)

    if [ $orphaned_workspaces -eq 0 ]; then
        echo -e "${GREEN}No orphaned workspaces found${NC}"
    else
        echo -e "${GREEN}Removed $orphaned_workspaces orphaned workspace(s)${NC}"
        echo -e "Total space freed: $(numfmt --to=iec-i --suffix=B "$total_ws_size" 2>/dev/null || echo "${total_ws_size}B")"
    fi
fi

echo
echo -e "${GREEN}Cleanup complete!${NC}"
