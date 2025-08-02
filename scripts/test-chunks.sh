#!/usr/bin/env bash
# Chunked testing strategy for aws-dash
# Runs tests in smaller chunks to avoid context window issues

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Enhanced verbosity system
# Level 0: quiet (only summaries)
# Level 1: smart (failures without flood) - DEFAULT for assistants
# Level 2: detailed (failure details)
# Level 3: full (all output)

# Parse verbosity level with backwards compatibility
VERBOSITY=${TEST_VERBOSITY:-1}  # Default to smart mode

# Handle legacy VERBOSE variable
if [ "$VERBOSE" = "true" ]; then
    VERBOSITY=3
elif [ "$VERBOSE" = "false" ]; then
    VERBOSITY=0
fi

# Handle named modes for clarity
case "${TEST_MODE:-}" in
    "quiet") VERBOSITY=0 ;;
    "smart") VERBOSITY=1 ;;
    "detailed") VERBOSITY=2 ;;
    "full") VERBOSITY=3 ;;
esac

# Function to print status
print_status() {
    echo -e "${GREEN}[TEST CHUNK]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_test_info() {
    echo -e "${BLUE}[RUNNING]${NC} $1"
    echo -e "${YELLOW}🔍 MEMORY MONITOR:${NC} Starting test '$1' - check memory usage now"
}

# Function to run a single test with multi-level verbosity
run_single_test() {
    local test_name="$1"
    local test_type="$2"  # --test, --lib, or --doc
    
    case $VERBOSITY in
        0) run_test_quiet "$test_name" "$test_type" ;;
        1) run_test_smart "$test_name" "$test_type" ;;
        2) run_test_detailed "$test_name" "$test_type" ;;
        3) run_test_full "$test_name" "$test_type" ;;
        *) run_test_smart "$test_name" "$test_type" ;;  # Default fallback
    esac
}

# Level 0: Quiet mode - only show summaries
run_test_quiet() {
    local test_name="$1"
    local test_type="$2"
    
    local output
    if output=$(run_cargo_test "$test_name" "$test_type" 2>&1); then
        echo "✓ $test_name: $(echo "$output" | grep -E "test result:" | tail -1)"
    else
        print_error "FAILED: $test_name"
        echo "$output"
        return 1
    fi
}

# Level 1: Smart mode - perfect for assistants
run_test_smart() {
    local test_name="$1"
    local test_type="$2"
    
    print_test_info "Running $test_name"
    local output
    local start_time=$(date +%s)
    
    if output=$(run_cargo_test "$test_name" "$test_type" 2>&1); then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        # Extract key metrics
        local result_line=$(echo "$output" | grep -E "test result:" | tail -1)
        local passed=$(echo "$result_line" | grep -o '[0-9]\+ passed' | grep -o '[0-9]\+' || echo "0")
        local failed=$(echo "$result_line" | grep -o '[0-9]\+ failed' | grep -o '[0-9]\+' || echo "0")
        
        if [ "${failed:-0}" -eq 0 ]; then
            echo "✓ $test_name: $passed passed (${duration}s)"
        else
            echo "❌ $test_name: $passed passed, $failed failed (${duration}s)"
            # Show only failed test names for context (assistant-friendly)
            echo "$output" | grep -E "test .* \.\.\. FAILED$" | sed 's/^/   └─ FAILED /' | head -5
        fi
    else
        print_error "COMPILATION FAILED: $test_name"
        # Show only compilation errors, not full output (context-window friendly)
        echo "$output" | grep -E "error\[|Error:|Failed|cannot find" | head -3 | sed 's/^/   └─ /'
        return 1
    fi
}

# Level 2: Detailed mode - show failure details for debugging
run_test_detailed() {
    local test_name="$1"
    local test_type="$2"
    
    print_test_info "Running $test_name"
    local output
    local start_time=$(date +%s)
    
    if output=$(run_cargo_test "$test_name" "$test_type" 2>&1); then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        # Extract key metrics
        local result_line=$(echo "$output" | grep -E "test result:" | tail -1)
        local passed=$(echo "$result_line" | grep -o '[0-9]\+ passed' | grep -o '[0-9]\+' || echo "0")
        local failed=$(echo "$result_line" | grep -o '[0-9]\+ failed' | grep -o '[0-9]\+' || echo "0")
        
        if [ "${failed:-0}" -eq 0 ]; then
            echo "✓ $test_name: $passed passed (${duration}s)"
        else
            echo "❌ $test_name: $passed passed, $failed failed (${duration}s)"
            echo ""
            echo "   FAILED TESTS:"
            echo "$output" | grep -E "test .* \.\.\. FAILED$" | sed 's/^/   /'
            echo ""
            echo "   FAILURE DETAILS:"
            echo "$output" | grep -A 10 -E "failures:|FAILURES:" | head -20 | sed 's/^/   /'
        fi
    else
        print_error "COMPILATION FAILED: $test_name"
        echo ""
        echo "   COMPILATION ERRORS:"
        echo "$output" | grep -A 3 -E "error\[|Error:" | head -15 | sed 's/^/   /'
        return 1
    fi
}

# Level 3: Full mode - show all output (current verbose behavior)
run_test_full() {
    local test_name="$1"
    local test_type="$2"
    
    print_test_info "Running $test_name"
    run_cargo_test "$test_name" "$test_type"
}

# Helper function to execute the appropriate cargo test command
run_cargo_test() {
    local test_name="$1"
    local test_type="$2"
    
    # No job limits - use default CPU count (28) for maximum performance
    unset CARGO_BUILD_JOBS
    unset RUSTFLAGS
    
    case "$test_type" in
        "--lib")
            cargo test --lib
            ;;
        "--doc")
            cargo test --workspace --doc
            ;;
        "--test")
            cargo test --test "$test_name"
            ;;
        *)
            echo "Unknown test type: $test_type" >&2
            return 1
            ;;
    esac
}

# Chunk 1: Fast Core Tests (~60 tests, <30s)
run_chunk_core() {
    print_status "Running Chunk 1: Core Tests (frozen, API contracts, unit tests)"
    echo -e "${YELLOW}🔍 MEMORY MONITOR:${NC} Starting CORE CHUNK - monitor memory for frozen tests"
    run_single_test "aws_identity_frozen_tests" "--test"
    run_single_test "projects_frozen_tests" "--test"
    run_single_test "cfn_dag_frozen_tests" "--test"
    run_single_test "api_contract_simple_tests" "--test"
    run_single_test "unit tests" "--lib"
}

# Chunk 2: CloudFormation Logic (~50 tests, 1-2min)
run_chunk_cfn() {
    print_status "Running Chunk 2: CloudFormation Tests"
    echo -e "${YELLOW}🔍 MEMORY MONITOR:${NC} Starting CFN CHUNK - monitor memory for template tests"
    run_single_test "cfn_template_tests" "--test"
    run_single_test "cfn_template_verification_tests" "--test"
    run_single_test "cfn_dependency_validation_tests" "--test"
    run_single_test "cloudformation_graph_verification_tests" "--test"
}

# Chunk 3: UI Components - REMOVED (August 2025)
run_chunk_ui() {
    print_status "UI Component Tests - REMOVED"
    echo -e "${YELLOW}⚠️  UI tests were removed due to compilation issues${NC}"
    echo -e "${YELLOW}📋 See tests/UI_TESTING_SETUP.md for complete archive${NC}"
    echo "✓ UI chunk skipped (no tests to run)"
}

# Chunk 4: Project Management (~25 tests, 30s)
run_chunk_projects() {
    print_status "Running Chunk 4: Project Management Tests"
    echo -e "${YELLOW}🔍 MEMORY MONITOR:${NC} Starting PROJECTS CHUNK - monitor memory for project tests"
    run_single_test "projects_tests" "--test"
    run_single_test "schema_constraint_tests" "--test"
    run_single_test "value_editor_window_tests" "--test"
    run_single_test "fixture_import_discrepancies_tests" "--test"
}

# Chunk 5: Integration Tests (separate, long-running)
run_chunk_integration() {
    print_status "Running Chunk 5: Integration Tests (may take 10-30 minutes)"
    echo -e "${YELLOW}🔍 MEMORY MONITOR:${NC} Starting INTEGRATION CHUNK - HIGH MEMORY USAGE EXPECTED"
    print_warning "These tests are normally ignored. Running explicitly..."
    
    print_test_info "Running aws_real_world_templates (with --ignored flag)"
    if [ "$VERBOSE" = "true" ]; then
        cargo test --test aws_real_world_templates -- --ignored
    else
        local output
        if output=$(cargo test --test aws_real_world_templates -- --ignored 2>&1); then
            echo "✓ aws_real_world_templates: $(echo "$output" | grep -E "test result:" | tail -1)"
        else
            print_error "FAILED: aws_real_world_templates"
            echo "$output"
            return 1
        fi
    fi
}

# Documentation tests
run_doc_tests() {
    print_status "Running Documentation Tests"
    echo -e "${YELLOW}🔍 MEMORY MONITOR:${NC} Starting DOC CHUNK - monitor memory for doc tests"
    run_single_test "documentation tests" "--doc"
}

# Show usage information
show_usage() {
    echo "Usage: $0 [OPTION]"
    echo "       VERBOSE=true $0 [OPTION]  # Enable verbose output"
    echo ""
    echo "Test chunk options:"
    echo "  core         Run core tests (frozen, API contracts, unit tests)"
    echo "  cfn          Run CloudFormation logic tests"
    echo "  ui           Run UI component tests"
    echo "  projects     Run project management tests"
    echo "  integration  Run integration tests (long-running)"
    echo "  docs         Run documentation tests"
    echo "  fast         Run chunks 1-4 (excludes integration tests)"
    echo "  all          Run all test chunks including integration"
    echo ""
    echo "Default behavior (no args): Run fast test chunks (1-4)"
    echo ""
    echo "Output modes:"
    echo "  Normal:  Shows test names and results summary. Full output only on failure."
    echo "  Verbose: Shows all cargo test output (set VERBOSE=true)"
    echo ""
    echo "Examples:"
    echo "  $0 core                    # Run core tests with summary output"
    echo "  VERBOSE=true $0 core       # Run core tests with full output"
    echo "  $0 fast                    # Run all fast tests with summary output"
}

# Run specific chunk based on argument
case "${1:-fast}" in
    "core")
        run_chunk_core
        ;;
    "cfn")
        run_chunk_cfn
        ;;
    "ui")
        run_chunk_ui
        ;;
    "projects")
        run_chunk_projects
        ;;
    "integration")
        run_chunk_integration
        ;;
    "docs")
        run_doc_tests
        ;;
    "fast")
        print_status "Running fast test suite (chunks 1-4)"
        
        # Track failed chunks but continue running all chunks
        failed_chunks=()
        
        if ! run_chunk_core; then
            failed_chunks+=("core")
        fi
        
        if ! run_chunk_cfn; then
            failed_chunks+=("cfn")
        fi
        
        if ! run_chunk_ui; then
            failed_chunks+=("ui")
        fi
        
        if ! run_chunk_projects; then
            failed_chunks+=("projects")
        fi
        
        if ! run_doc_tests; then
            failed_chunks+=("docs")
        fi
        
        # Report results
        if [ ${#failed_chunks[@]} -eq 0 ]; then
            print_status "✅ Fast test suite completed successfully!"
        else
            echo ""
            print_error "⚠️  Fast test suite finished with some failures:"
            for chunk in "${failed_chunks[@]}"; do
                echo "   - $chunk tests failed"
            done
            echo ""
            print_status "All fast chunks completed despite failures. Check individual chunk output above."
        fi
        ;;
    "all")
        print_status "Running complete test suite (all chunks)"
        
        # Track failed chunks but continue running all chunks
        failed_chunks=()
        
        echo "Starting chunk: Core Tests"
        if ! run_chunk_core; then
            failed_chunks+=("core")
        fi
        
        echo "Starting chunk: CloudFormation Tests"
        if ! run_chunk_cfn; then
            failed_chunks+=("cfn")
        fi
        
        echo "Starting chunk: UI Component Tests"
        if ! run_chunk_ui; then
            failed_chunks+=("ui")
        fi
        
        echo "Starting chunk: Project Management Tests"
        if ! run_chunk_projects; then
            failed_chunks+=("projects")
        fi
        
        echo "Starting chunk: Documentation Tests"
        if ! run_doc_tests; then
            failed_chunks+=("docs")
        fi
        
        echo "Starting chunk: Integration Tests"
        if ! run_chunk_integration; then
            failed_chunks+=("integration")
        fi
        
        # Report results
        if [ ${#failed_chunks[@]} -eq 0 ]; then
            print_status "✅ Complete test suite finished successfully!"
        else
            echo ""
            print_error "⚠️  Complete test suite finished with some failures:"
            for chunk in "${failed_chunks[@]}"; do
                echo "   - $chunk tests failed"
            done
            echo ""
            print_status "All chunks completed despite failures. Check individual chunk output above."
        fi
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    *)
        print_error "Unknown option: $1"
        show_usage
        exit 1
        ;;
esac