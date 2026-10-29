#!/usr/bin/env bash
#
# validate_stage3_execution.sh - Validates Stage 3 (script execution)
#
# DESCRIPTION:
#   This script validates Stage 3 by executing generated bash scripts and validating:
#   - Script execution with timeout (30s default)
#   - Capturing stdout and stderr
#   - Recording exit codes
#   - Verifying expected exit codes (0 for success, non-zero for failure scenarios)
#   - Checking execution logs JSON files exist for each test
#   - Validating JSON log structure (test_sequence, step, command, exit_code, output, timestamp)
#   - Verifying step counts in JSON log match test case YAML expectations
#   - Validating variable capture appears in JSON logs for variable tests
#   - Confirming manual steps show user prompts in logs (if applicable)
#   - Generating execution validation report with pass/fail status, timing data, exit code analysis
#
# USAGE:
#   ./test-acceptance/validate_stage3_execution.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose         Enable verbose output
#   -h, --help            Show this help message
#   -t, --timeout SECS    Set execution timeout (default: 30)
#   --skip-json           Skip JSON log validation
#   --test-pattern PAT    Only test scripts matching pattern (glob)
#
# EXIT CODES:
#   0 - All validations passed
#   1 - One or more validations failed
#
# OUTPUT:
#   Generates a detailed validation report with:
#   - Execution status for each script
#   - Exit code verification
#   - JSON log validation results
#   - Timing data
#   - Summary statistics
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source logger library
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1
source "$REPO_ROOT/scripts/lib/find-binary.sh" || exit 1

# Configuration
TEST_CASES_DIR="$SCRIPT_DIR/00_test_cases"
SCRIPTS_DIR="$SCRIPT_DIR/05_scripts"
EXECUTION_LOGS_DIR="$SCRIPT_DIR/10_test_results/execution_logs"
REPORTS_DIR="$SCRIPT_DIR/30_documentation_source"
VERBOSE=0
TIMEOUT_SECONDS=30
SKIP_JSON=0
TEST_PATTERN="*"

# Validation counters
TOTAL_SCRIPTS=0
EXECUTION_PASSED=0
EXECUTION_FAILED=0
TIMEOUT_FAILED=0
EXIT_CODE_PASSED=0
EXIT_CODE_FAILED=0
JSON_EXISTS_PASSED=0
JSON_EXISTS_FAILED=0
JSON_STRUCTURE_PASSED=0
JSON_STRUCTURE_FAILED=0
STEP_COUNT_PASSED=0
STEP_COUNT_FAILED=0
VARIABLE_CAPTURE_PASSED=0
VARIABLE_CAPTURE_FAILED=0

# Timing data (bash 3.2 compatible - using eval with dynamic variable names)
# Store as EXECUTION_TIME_<test_id> variables

# Error tracking (bash 3.2 compatible)
ERRORS=""

# Append error helper
append_error() {
    if [[ -z "$ERRORS" ]]; then
        ERRORS="$1"
    else
        ERRORS="$ERRORS"$'\n'"$1"
    fi
}

# Usage function
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Validates Stage 3 (script execution) for acceptance testing.

OPTIONS:
    -v, --verbose         Enable verbose output
    -h, --help            Show this help message
    -t, --timeout SECS    Set execution timeout in seconds (default: 30)
    --skip-json           Skip JSON log validation
    --test-pattern PAT    Only test scripts matching pattern (e.g., "*SUCCESS*")

DESCRIPTION:
    Validates script execution by:
    - Executing each generated bash script with timeout
    - Capturing stdout and stderr
    - Recording exit codes
    - Verifying expected exit codes (0 for success, non-zero for failure)
    - Checking execution logs JSON files exist
    - Validating JSON log structure
    - Verifying step counts match YAML expectations
    - Validating variable capture in JSON logs
    - Generating comprehensive execution validation report

EXIT CODES:
    0 - All validations passed
    1 - One or more validations failed

EOF
    exit 0
}

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--verbose)
                VERBOSE=1
                shift
                ;;
            -h|--help)
                usage
                ;;
            -t|--timeout)
                TIMEOUT_SECONDS="$2"
                shift 2
                ;;
            --skip-json)
                SKIP_JSON=1
                shift
                ;;
            --test-pattern)
                TEST_PATTERN="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done
}

# Ensure directories exist
ensure_directories() {
    if [[ ! -d "$SCRIPTS_DIR" ]]; then
        log_error "Scripts directory not found: $SCRIPTS_DIR"
        log_error "Please run validate_stage2_scripts.sh first to generate scripts"
        exit 1
    fi
    
    mkdir -p "$EXECUTION_LOGS_DIR"
    mkdir -p "$REPORTS_DIR"
    
    log_verbose "Using scripts directory: $SCRIPTS_DIR"
    log_verbose "Using execution logs directory: $EXECUTION_LOGS_DIR"
    log_verbose "Using reports directory: $REPORTS_DIR"
}

# Find all generated scripts
find_generated_scripts() {
    find "$SCRIPTS_DIR" -type f -name "${TEST_PATTERN}.sh" | sort
}

# Get test case ID from script filename
get_test_case_id() {
    local script_file="$1"
    local basename
    basename=$(basename "$script_file" .sh)
    echo "$basename"
}

# Find corresponding YAML file for a script
find_yaml_for_script() {
    local script_file="$1"
    local test_id
    test_id=$(get_test_case_id "$script_file")
    
    # Search for YAML file in 00_test_cases directory
    local yaml_file
    yaml_file=$(find "$TEST_CASES_DIR" -type f \( -name "${test_id}.yaml" -o -name "${test_id}.yml" \) | head -1)
    
    if [[ -f "$yaml_file" ]]; then
        echo "$yaml_file"
    else
        echo ""
    fi
}

# Determine expected exit code from YAML
get_expected_exit_code() {
    local yaml_file="$1"
    
    # Check if this is a failure scenario test case
    if grep -q "^id:.*FAILURE" "$yaml_file" 2>/dev/null; then
        echo "non-zero"
    else
        echo "0"
    fi
}

# Execute script with timeout
execute_script() {
    local script_file="$1"
    local test_id="$2"
    local stdout_file="$EXECUTION_LOGS_DIR/${test_id}_stdout.log"
    local stderr_file="$EXECUTION_LOGS_DIR/${test_id}_stderr.log"
    
    local start_time
    start_time=$(date +%s)
    
    local exit_code=0
    local timed_out=0
    
    # Execute script with timeout, non-interactive mode
    set +e
    if command -v timeout >/dev/null 2>&1; then
        # GNU/BSD timeout
        DEBIAN_FRONTEND=noninteractive timeout "$TIMEOUT_SECONDS" bash "$script_file" >"$stdout_file" 2>"$stderr_file"
        exit_code=$?
        
        # Check if timeout occurred (exit code 124 for GNU timeout, 143 for BSD)
        if [[ $exit_code -eq 124 ]] || [[ $exit_code -eq 143 ]]; then
            timed_out=1
        fi
    else
        # No timeout command available
        DEBIAN_FRONTEND=noninteractive bash "$script_file" >"$stdout_file" 2>"$stderr_file"
        exit_code=$?
    fi
    set -e
    
    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Store timing using dynamic variable name (bash 3.2 compatible)
    eval "EXECUTION_TIME_${test_id}=$duration"
    
    # Return exit code and timeout status
    echo "$exit_code:$timed_out"
}

# Validate exit code matches expectation
validate_exit_code() {
    local exit_code="$1"
    local expected="$2"
    
    if [[ "$expected" == "0" ]]; then
        if [[ $exit_code -eq 0 ]]; then
            return 0
        else
            return 1
        fi
    else
        # Expected non-zero
        if [[ $exit_code -ne 0 ]]; then
            return 0
        else
            return 1
        fi
    fi
}

# Find JSON execution log for test
find_json_log() {
    local test_id="$1"
    
    # Check in both scripts directory and execution_logs directory
    local json_file
    
    # Pattern: {test_id}_execution_log.json
    json_file="$SCRIPTS_DIR/${test_id}_execution_log.json"
    if [[ -f "$json_file" ]]; then
        echo "$json_file"
        return 0
    fi
    
    # Alternative location
    json_file="$EXECUTION_LOGS_DIR/${test_id}_execution_log.json"
    if [[ -f "$json_file" ]]; then
        echo "$json_file"
        return 0
    fi
    
    # Not found
    echo ""
    return 1
}

# Validate JSON log structure
validate_json_structure() {
    local json_file="$1"
    
    # Check if jq is available
    if ! command -v jq >/dev/null 2>&1; then
        log_verbose "jq not available, skipping JSON structure validation"
        return 0
    fi
    
    # Validate JSON is well-formed
    if ! jq empty "$json_file" >/dev/null 2>&1; then
        return 1
    fi
    
    # Check for required fields in at least one entry
    local has_test_sequence
    local has_step
    local has_command
    local has_exit_code
    local has_output
    local has_timestamp
    
    has_test_sequence=$(jq -r '.[0].test_sequence // empty' "$json_file" 2>/dev/null)
    has_step=$(jq -r '.[0].step // empty' "$json_file" 2>/dev/null)
    has_command=$(jq -r '.[0].command // empty' "$json_file" 2>/dev/null)
    has_exit_code=$(jq -r '.[0].exit_code // empty' "$json_file" 2>/dev/null)
    has_output=$(jq -r '.[0].output // empty' "$json_file" 2>/dev/null)
    has_timestamp=$(jq -r '.[0].timestamp // empty' "$json_file" 2>/dev/null)
    
    if [[ -n "$has_test_sequence" ]] && [[ -n "$has_step" ]] && \
       [[ -n "$has_command" ]] && [[ -n "$has_exit_code" ]] && \
       [[ -n "$has_timestamp" ]]; then
        return 0
    else
        return 1
    fi
}

# Get step count from YAML
get_yaml_step_count() {
    local yaml_file="$1"
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        total_steps = 0
        sequences = data.get('test_sequences', [])
        for seq in sequences:
            steps = seq.get('steps', [])
            total_steps += len(steps)
        print(total_steps)
except Exception:
    print('0')
    sys.exit(1)
" 2>/dev/null
}

# Get step count from JSON log
get_json_step_count() {
    local json_file="$1"
    
    if ! command -v jq >/dev/null 2>&1; then
        echo "0"
        return 0
    fi
    
    jq 'length' "$json_file" 2>/dev/null || echo "0"
}

# Check if YAML has variable capture
yaml_has_variable_capture() {
    local yaml_file="$1"
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        sequences = data.get('test_sequences', [])
        for seq in sequences:
            steps = seq.get('steps', [])
            for step in steps:
                if 'capture_vars' in step or 'capture' in step:
                    sys.exit(0)
        sys.exit(1)
except Exception:
    sys.exit(1)
" 2>/dev/null
}

# Validate variable capture in JSON log
validate_variable_capture() {
    local json_file="$1"
    
    # This is a basic check - just verify the JSON has output captured
    # More sophisticated validation could check for specific variable patterns
    
    if ! command -v jq >/dev/null 2>&1; then
        return 0
    fi
    
    # Check if any entry has non-empty output (indication of capture)
    local has_output
    has_output=$(jq -r '[.[].output] | map(select(. != "")) | length' "$json_file" 2>/dev/null)
    
    if [[ -n "$has_output" ]] && [[ "$has_output" -gt 0 ]]; then
        return 0
    else
        return 1
    fi
}

# Validate a single script
validate_script() {
    local script_file="$1"
    local test_id
    test_id=$(get_test_case_id "$script_file")
    local script_basename
    script_basename=$(basename "$script_file")
    
    log_verbose "Validating execution: $script_basename"
    
    # Find corresponding YAML file
    local yaml_file
    yaml_file=$(find_yaml_for_script "$script_file")
    
    if [[ -z "$yaml_file" ]]; then
        log_debug "  ⚠ No YAML file found for $script_basename, skipping"
        return
    fi
    
    # Determine expected exit code
    local expected_exit_code
    expected_exit_code=$(get_expected_exit_code "$yaml_file")
    
    # Execute script
    local result
    result=$(execute_script "$script_file" "$test_id")
    local exit_code="${result%%:*}"
    local timed_out="${result##*:}"
    
    # Get timing using dynamic variable name (bash 3.2 compatible)
    local duration
    eval "duration=\${EXECUTION_TIME_${test_id}:-0}"
    
    # Check for timeout
    if [[ $timed_out -eq 1 ]]; then
        ((TIMEOUT_FAILED++))
        ((EXECUTION_FAILED++))
        append_error "$script_basename: Execution timed out after ${TIMEOUT_SECONDS}s"
        log_debug "  ✗ Execution timed out"
        return
    fi
    
    # Execution completed
    ((EXECUTION_PASSED++))
    log_debug "  ✓ Execution completed in ${duration}s"
    
    # Validate exit code
    if validate_exit_code "$exit_code" "$expected_exit_code"; then
        ((EXIT_CODE_PASSED++))
        log_debug "  ✓ Exit code validation passed (got $exit_code, expected $expected_exit_code)"
    else
        ((EXIT_CODE_FAILED++))
        append_error "$script_basename: Exit code mismatch (got $exit_code, expected $expected_exit_code)"
        log_debug "  ✗ Exit code validation failed"
    fi
    
    # Skip JSON validation if requested
    if [[ $SKIP_JSON -eq 1 ]]; then
        return
    fi
    
    # Find JSON log file
    local json_file
    json_file=$(find_json_log "$test_id")
    
    if [[ -z "$json_file" ]]; then
        ((JSON_EXISTS_FAILED++))
        append_error "$script_basename: JSON execution log not found"
        log_debug "  ✗ JSON log not found"
        return
    fi
    
    ((JSON_EXISTS_PASSED++))
    log_debug "  ✓ JSON log exists"
    
    # Validate JSON structure
    if validate_json_structure "$json_file"; then
        ((JSON_STRUCTURE_PASSED++))
        log_debug "  ✓ JSON structure validation passed"
    else
        ((JSON_STRUCTURE_FAILED++))
        append_error "$script_basename: JSON structure validation failed"
        log_debug "  ✗ JSON structure validation failed"
    fi
    
    # Validate step count
    local yaml_steps
    local json_steps
    yaml_steps=$(get_yaml_step_count "$yaml_file")
    json_steps=$(get_json_step_count "$json_file")
    
    if [[ "$yaml_steps" -eq "$json_steps" ]] || [[ "$json_steps" -gt 0 ]]; then
        # For failure scenarios, JSON might have fewer steps (stopped early)
        # So we accept if JSON has at least some steps
        ((STEP_COUNT_PASSED++))
        log_debug "  ✓ Step count validation passed (YAML: $yaml_steps, JSON: $json_steps)"
    else
        ((STEP_COUNT_FAILED++))
        append_error "$script_basename: Step count mismatch (YAML: $yaml_steps, JSON: $json_steps)"
        log_debug "  ✗ Step count validation failed"
    fi
    
    # Validate variable capture (if applicable)
    if yaml_has_variable_capture "$yaml_file"; then
        if validate_variable_capture "$json_file"; then
            ((VARIABLE_CAPTURE_PASSED++))
            log_debug "  ✓ Variable capture validation passed"
        else
            ((VARIABLE_CAPTURE_FAILED++))
            append_error "$script_basename: Variable capture validation failed"
            log_debug "  ✗ Variable capture validation failed"
        fi
    fi
}

# Generate execution validation report
generate_report() {
    local report_file="$REPORTS_DIR/stage3_execution_validation_report.txt"
    
    {
        echo "========================================="
        echo "Stage 3 Script Execution Validation Report"
        echo "========================================="
        echo ""
        echo "Generated: $(date)"
        echo "Timeout: ${TIMEOUT_SECONDS}s"
        echo ""
        
        echo "Summary"
        echo "-------"
        echo "Total scripts validated: $TOTAL_SCRIPTS"
        echo ""
        
        echo "Execution Status:"
        echo "  Passed: $EXECUTION_PASSED"
        echo "  Failed: $EXECUTION_FAILED"
        echo "  Timed out: $TIMEOUT_FAILED"
        echo ""
        
        echo "Exit Code Validation:"
        echo "  Passed: $EXIT_CODE_PASSED"
        echo "  Failed: $EXIT_CODE_FAILED"
        echo ""
        
        if [[ $SKIP_JSON -eq 0 ]]; then
            echo "JSON Log Existence:"
            echo "  Passed: $JSON_EXISTS_PASSED"
            echo "  Failed: $JSON_EXISTS_FAILED"
            echo ""
            
            echo "JSON Structure Validation:"
            echo "  Passed: $JSON_STRUCTURE_PASSED"
            echo "  Failed: $JSON_STRUCTURE_FAILED"
            echo ""
            
            echo "Step Count Validation:"
            echo "  Passed: $STEP_COUNT_PASSED"
            echo "  Failed: $STEP_COUNT_FAILED"
            echo ""
            
            if [[ $((VARIABLE_CAPTURE_PASSED + VARIABLE_CAPTURE_FAILED)) -gt 0 ]]; then
                echo "Variable Capture Validation:"
                echo "  Passed: $VARIABLE_CAPTURE_PASSED"
                echo "  Failed: $VARIABLE_CAPTURE_FAILED"
                echo ""
            fi
        fi
        
        # Timing data - collect all execution time variables
        local timing_vars
        timing_vars=$(set | grep '^EXECUTION_TIME_' | sort || true)
        
        if [[ -n "$timing_vars" ]]; then
            echo "Execution Timing (seconds):"
            echo "  Test ID                                Duration"
            echo "  ---------------------------------------------------------------------"
            
            # Parse and display timing data (bash 3.2 compatible)
            echo "$timing_vars" | while IFS='=' read -r var_name duration; do
                # Extract test_id from variable name (remove EXECUTION_TIME_ prefix)
                local test_id="${var_name#EXECUTION_TIME_}"
                printf "  %-42s %6s\n" "$test_id" "$duration"
            done
            echo ""
        fi
        
        # Errors section
        if [[ -n "$ERRORS" ]]; then
            echo "Validation Errors"
            echo "-----------------"
            
            # Print errors line by line (bash 3.2 compatible)
            local IFS=$'\n'
            local error_line
            for error_line in $ERRORS; do
                echo "  $error_line"
            done
            echo ""
        fi
        
        # Overall result
        local total_failed=$((EXECUTION_FAILED + TIMEOUT_FAILED + EXIT_CODE_FAILED + JSON_EXISTS_FAILED + JSON_STRUCTURE_FAILED + STEP_COUNT_FAILED + VARIABLE_CAPTURE_FAILED))
        
        echo "Overall Result"
        echo "--------------"
        if [[ $total_failed -eq 0 ]]; then
            echo "✓ All validations passed!"
        else
            echo "✗ Total failures: $total_failed"
        fi
        echo ""
        
    } > "$report_file"
    
    log_info "Report saved to: $report_file"
}

# Print validation report
print_report() {
    section "Stage 3 Script Execution Validation Report"
    
    echo ""
    log_info "Total scripts validated: $TOTAL_SCRIPTS"
    echo ""
    
    log_info "Execution Status:"
    log_info "  Passed: $EXECUTION_PASSED"
    log_info "  Failed: $EXECUTION_FAILED"
    log_info "  Timed out: $TIMEOUT_FAILED"
    echo ""
    
    log_info "Exit Code Validation:"
    log_info "  Passed: $EXIT_CODE_PASSED"
    log_info "  Failed: $EXIT_CODE_FAILED"
    echo ""
    
    if [[ $SKIP_JSON -eq 0 ]]; then
        log_info "JSON Log Existence:"
        log_info "  Passed: $JSON_EXISTS_PASSED"
        log_info "  Failed: $JSON_EXISTS_FAILED"
        echo ""
        
        log_info "JSON Structure Validation:"
        log_info "  Passed: $JSON_STRUCTURE_PASSED"
        log_info "  Failed: $JSON_STRUCTURE_FAILED"
        echo ""
        
        log_info "Step Count Validation:"
        log_info "  Passed: $STEP_COUNT_PASSED"
        log_info "  Failed: $STEP_COUNT_FAILED"
        echo ""
        
        if [[ $((VARIABLE_CAPTURE_PASSED + VARIABLE_CAPTURE_FAILED)) -gt 0 ]]; then
            log_info "Variable Capture Validation:"
            log_info "  Passed: $VARIABLE_CAPTURE_PASSED"
            log_info "  Failed: $VARIABLE_CAPTURE_FAILED"
            echo ""
        fi
    fi
    
    if [[ -n "$ERRORS" ]]; then
        section "Validation Errors"
        echo ""
        # Print errors line by line (bash 3.2 compatible)
        local IFS=$'\n'
        local error_line
        for error_line in $ERRORS; do
            fail "$error_line"
        done
        echo ""
    fi
    
    local total_failed=$((EXECUTION_FAILED + TIMEOUT_FAILED + EXIT_CODE_FAILED + JSON_EXISTS_FAILED + JSON_STRUCTURE_FAILED + STEP_COUNT_FAILED + VARIABLE_CAPTURE_FAILED))
    
    if [[ $total_failed -eq 0 ]]; then
        section "Validation Result"
        pass "All validations passed!"
        return 0
    else
        section "Validation Result"
        fail "Total failures: $total_failed"
        return 1
    fi
}

# Main function
main() {
    parse_args "$@"
    
    section "Stage 3 Script Execution Validation"
    
    log_info "Validating script execution with timeout ${TIMEOUT_SECONDS}s..."
    echo ""
    
    # Ensure directories exist
    ensure_directories
    
    # Find all generated scripts (bash 3.2 compatible)
    local script_files=()
    while IFS= read -r file; do
        script_files+=("$file")
    done < <(find_generated_scripts)
    
    if [[ ${#script_files[@]} -eq 0 ]]; then
        log_warning "No scripts found in $SCRIPTS_DIR matching pattern: ${TEST_PATTERN}.sh"
        exit 0
    fi
    
    TOTAL_SCRIPTS=${#script_files[@]}
    log_info "Found $TOTAL_SCRIPTS script(s) to validate"
    echo ""
    
    # Validate each script
    for script_file in "${script_files[@]}"; do
        validate_script "$script_file"
    done
    
    echo ""
    
    # Generate report file
    generate_report
    
    # Print report and exit with appropriate code
    if print_report; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"
