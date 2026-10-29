#!/usr/bin/env bash
#
# validate_stage4_verification.sh - Validates Stage 4 (verification results)
#
# DESCRIPTION:
#   This script validates Stage 4 by running the verifier binary on execution logs
#   and validating the generated container YAML files. It performs:
#   - Running verifier binary on each execution log with proper metadata
#   - Validating verifier exit codes (0 for all-pass, 1 for some-fail)
#   - Checking container YAML output files exist in 20_verification/
#   - Validating container YAML structure against data/testcase_results_container/schema.json
#   - Verifying test_results array contains TestCaseVerificationResult entries
#   - Checking metadata section has correct statistics
#   - Validating step_results use externally tagged enum format (Pass/Fail/NotExecuted)
#   - Confirming overall_pass boolean matches actual test results
#   - Generating verification validation report
#
# USAGE:
#   ./test-acceptance/validate_stage4_verification.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose         Enable verbose output
#   -h, --help            Show this help message
#   --skip-schema         Skip JSON schema validation
#   --test-pattern PAT    Only test logs matching pattern (glob)
#   --title TITLE         Title for container YAML (default: auto-generated)
#   --project PROJECT     Project name for container YAML (default: "Test Case Manager - Acceptance Tests")
#   --environment ENV     Environment for metadata (default: "Acceptance Test Environment")
#
# EXIT CODES:
#   0 - All validations passed
#   1 - One or more validations failed
#
# OUTPUT:
#   Generates a detailed validation report with:
#   - Verifier execution status for each log
#   - Exit code verification
#   - Container YAML validation results
#   - Schema compliance checks
#   - Data accuracy verification
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
VERIFICATION_RESULTS_DIR="$SCRIPT_DIR/20_verification"
REPORTS_DIR="$SCRIPT_DIR/30_documentation_source"
CONTAINER_SCHEMA="$REPO_ROOT/data/testcase_results_container/schema.json"
VERBOSE=0
SKIP_SCHEMA=0
TEST_PATTERN="*"
TITLE=""
PROJECT="Test Case Manager - Acceptance Tests"
ENVIRONMENT="Acceptance Test Environment"

# Validation counters
TOTAL_LOGS=0
VERIFIER_EXECUTED=0
VERIFIER_FAILED=0
EXIT_CODE_PASSED=0
EXIT_CODE_FAILED=0
CONTAINER_EXISTS_PASSED=0
CONTAINER_EXISTS_FAILED=0
SCHEMA_VALIDATION_PASSED=0
SCHEMA_VALIDATION_FAILED=0
STRUCTURE_VALIDATION_PASSED=0
STRUCTURE_VALIDATION_FAILED=0
METADATA_VALIDATION_PASSED=0
METADATA_VALIDATION_FAILED=0
ENUM_FORMAT_PASSED=0
ENUM_FORMAT_FAILED=0
OVERALL_PASS_PASSED=0
OVERALL_PASS_FAILED=0

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

Validates Stage 4 (verification results) for acceptance testing.

OPTIONS:
    -v, --verbose         Enable verbose output
    -h, --help            Show this help message
    --skip-schema         Skip JSON schema validation
    --test-pattern PAT    Only test logs matching pattern (e.g., "*SUCCESS*")
    --title TITLE         Title for container YAML (default: auto-generated)
    --project PROJECT     Project name (default: "Test Case Manager - Acceptance Tests")
    --environment ENV     Environment for metadata (default: "Acceptance Test Environment")

DESCRIPTION:
    Validates verification results by:
    - Running verifier binary on each execution log
    - Validating verifier exit codes
    - Checking container YAML output files exist
    - Validating container YAML structure against schema
    - Verifying test_results array format
    - Checking metadata statistics
    - Validating step_results enum format
    - Confirming overall_pass accuracy
    - Generating comprehensive validation report

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
            --skip-schema)
                SKIP_SCHEMA=1
                shift
                ;;
            --test-pattern)
                TEST_PATTERN="$2"
                shift 2
                ;;
            --title)
                TITLE="$2"
                shift 2
                ;;
            --project)
                PROJECT="$2"
                shift 2
                ;;
            --environment)
                ENVIRONMENT="$2"
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
        log_error "Please run validate_stage2_scripts.sh first"
        exit 1
    fi
    
    if [[ ! -d "$EXECUTION_LOGS_DIR" ]]; then
        log_error "Execution logs directory not found: $EXECUTION_LOGS_DIR"
        log_error "Please run validate_stage3_execution.sh first"
        exit 1
    fi
    
    mkdir -p "$VERIFICATION_RESULTS_DIR"
    mkdir -p "$REPORTS_DIR"
    
    log_verbose "Using scripts directory: $SCRIPTS_DIR"
    log_verbose "Using execution logs directory: $EXECUTION_LOGS_DIR"
    log_verbose "Using verification results directory: $VERIFICATION_RESULTS_DIR"
    log_verbose "Using reports directory: $REPORTS_DIR"
}

# Validate schema file exists
validate_schema_file() {
    if [[ $SKIP_SCHEMA -eq 0 ]]; then
        if [[ ! -f "$CONTAINER_SCHEMA" ]]; then
            log_error "Container schema file not found: $CONTAINER_SCHEMA"
            exit 1
        fi
        log_verbose "Using container schema: $CONTAINER_SCHEMA"
    fi
}

# Find verifier binary
find_verifier_binary() {
    local verifier_path
    
    # Find verifier binary using workspace-aware search
    cd "$REPO_ROOT"
    verifier_path=$(find_binary "verifier")
    if [[ -z "$verifier_path" ]]; then
        log_error "Verifier binary not found. Please build it first with: cargo build --bin verifier --release"
        exit 1
    fi
    
    echo "$verifier_path"
}

# Find all execution log JSON files
find_execution_logs() {
    find "$SCRIPTS_DIR" -type f -name "${TEST_PATTERN}_execution_log.json" | sort
}

# Get test case ID from log filename
get_test_case_id() {
    local log_file="$1"
    local basename
    basename=$(basename "$log_file" _execution_log.json)
    echo "$basename"
}

# Find corresponding YAML file for a test case
find_yaml_for_test() {
    local test_id="$1"
    
    # Search for YAML file in 00_test_cases directory
    local yaml_file
    yaml_file=$(find "$TEST_CASES_DIR" -type f \( -name "${test_id}.yaml" -o -name "${test_id}.yml" \) | head -1)
    
    if [[ -f "$yaml_file" ]]; then
        echo "$yaml_file"
    else
        echo ""
    fi
}

# Determine expected verifier exit code from YAML
get_expected_verifier_exit_code() {
    local yaml_file="$1"
    
    # Check if this is a failure scenario test case
    if grep -q "^id:.*FAILURE\|^id:.*FAILED" "$yaml_file" 2>/dev/null; then
        echo "1"  # Verifier exits with 1 when tests fail
    else
        echo "0"  # Verifier exits with 0 when all tests pass
    fi
}

# Run verifier on execution log
run_verifier() {
    local verifier_path="$1"
    local log_file="$2"
    local test_id="$3"
    local output_file="$4"
    
    local title_arg="${TITLE:-Test Execution Results - $test_id}"
    
    log_verbose "  Running verifier on $test_id..."
    
    # Run verifier with folder mode to generate container YAML
    # We'll create a temporary directory with just this log file
    local temp_dir
    temp_dir=$(mktemp -d)
    
    # Copy log file to temp directory
    cp "$log_file" "$temp_dir/"
    
    # Run verifier in folder mode to generate container YAML format
    local exit_code=0
    set +e
    "$verifier_path" \
        --folder "$temp_dir" \
        --test-case-dir "$TEST_CASES_DIR" \
        --format yaml \
        --output "$output_file" \
        >/dev/null 2>&1
    exit_code=$?
    set -e
    
    # Clean up temp directory
    rm -rf "$temp_dir"
    
    echo "$exit_code"
}

# Transform verifier output to container YAML format
transform_to_container_yaml() {
    local verifier_output="$1"
    local container_output="$2"
    local test_id="$3"
    local title="${4:-Test Execution Results}"
    
    local python_cmd
    python_cmd=$(find_python)
    
    # Use Python to transform the YAML structure
    $python_cmd - "$verifier_output" "$container_output" "$test_id" "$title" "$PROJECT" "$ENVIRONMENT" << 'PYTHON_SCRIPT'
import sys
import yaml
from datetime import datetime

verifier_output = sys.argv[1]
container_output = sys.argv[2]
test_id = sys.argv[3]
title = sys.argv[4]
project = sys.argv[5]
environment = sys.argv[6]

# Read verifier output
with open(verifier_output, 'r') as f:
    data = yaml.safe_load(f)

# Check if this is a BatchVerificationReport (has test_cases field)
if 'test_cases' in data:
    # Transform test_cases to test_results
    test_results = data['test_cases']
    
    # Create container YAML structure
    container = {
        'title': title,
        'project': project,
        'test_date': datetime.now().strftime('%Y-%m-%dT%H:%M:%S'),
        'test_results': test_results,
        'metadata': {
            'environment': environment,
            'platform': 'Test Case Manager',
            'executor': 'validate_stage4_verification.sh',
            'total_test_cases': data.get('total_test_cases', len(test_results)),
            'passed_test_cases': data.get('passed_test_cases', 0),
            'failed_test_cases': data.get('failed_test_cases', 0)
        }
    }
    
    # Write container YAML
    with open(container_output, 'w') as f:
        yaml.dump(container, f, default_flow_style=False, sort_keys=False, allow_unicode=True)
else:
    # If it's already a single test case result, wrap it in container format
    test_results = [data] if 'test_case_id' in data else []
    
    container = {
        'title': title,
        'project': project,
        'test_date': datetime.now().strftime('%Y-%m-%dT%H:%M:%S'),
        'test_results': test_results,
        'metadata': {
            'environment': environment,
            'platform': 'Test Case Manager',
            'executor': 'validate_stage4_verification.sh',
            'total_test_cases': len(test_results),
            'passed_test_cases': 1 if (test_results and test_results[0].get('overall_pass', False)) else 0,
            'failed_test_cases': 0 if (test_results and test_results[0].get('overall_pass', False)) else 1
        }
    }
    
    # Write container YAML
    with open(container_output, 'w') as f:
        yaml.dump(container, f, default_flow_style=False, sort_keys=False, allow_unicode=True)

PYTHON_SCRIPT
}

# Validate container YAML against schema
validate_container_schema() {
    local container_file="$1"
    
    # Check if ajv-cli is available
    if ! command -v ajv >/dev/null 2>&1; then
        log_verbose "  ajv-cli not available, skipping schema validation"
        return 0
    fi
    
    # Validate using ajv
    if ajv validate -s "$CONTAINER_SCHEMA" -d "$container_file" >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Validate container YAML structure
validate_container_structure() {
    local container_file="$1"
    
    local python_cmd
    python_cmd=$(find_python)
    
    if [[ -z "$python_cmd" ]]; then
        log_verbose "  Python not available, skipping structure validation"
        return 0
    fi
    
    # Validate structure using Python
    $python_cmd - "$container_file" << 'PYTHON_SCRIPT'
import sys
import yaml

container_file = sys.argv[1]

try:
    with open(container_file, 'r') as f:
        data = yaml.safe_load(f)
    
    # Check required fields
    if 'test_results' not in data:
        print("Missing required field: test_results")
        sys.exit(1)
    
    if not isinstance(data['test_results'], list):
        print("test_results must be an array")
        sys.exit(1)
    
    if len(data['test_results']) == 0:
        print("test_results array must not be empty")
        sys.exit(1)
    
    # Validate each test result
    for idx, result in enumerate(data['test_results']):
        # Check required fields
        required_fields = ['test_case_id', 'sequences', 'total_steps', 'passed_steps', 
                          'failed_steps', 'not_executed_steps', 'overall_pass']
        for field in required_fields:
            if field not in result:
                print(f"Test result #{idx+1} missing required field: {field}")
                sys.exit(1)
        
        # Validate sequences
        if not isinstance(result['sequences'], list) or len(result['sequences']) == 0:
            print(f"Test result '{result['test_case_id']}' must have at least one sequence")
            sys.exit(1)
        
        for seq_idx, sequence in enumerate(result['sequences']):
            # Check sequence required fields
            seq_required = ['sequence_id', 'name', 'step_results', 'all_steps_passed']
            for field in seq_required:
                if field not in sequence:
                    print(f"Sequence #{seq_idx+1} in '{result['test_case_id']}' missing required field: {field}")
                    sys.exit(1)
    
    sys.exit(0)
    
except Exception as e:
    print(f"Structure validation error: {e}")
    sys.exit(1)
PYTHON_SCRIPT
}

# Validate step results enum format
validate_enum_format() {
    local container_file="$1"
    
    local python_cmd
    python_cmd=$(find_python)
    
    if [[ -z "$python_cmd" ]]; then
        log_verbose "  Python not available, skipping enum format validation"
        return 0
    fi
    
    # Validate enum format using Python
    $python_cmd - "$container_file" << 'PYTHON_SCRIPT'
import sys
import yaml

container_file = sys.argv[1]

try:
    with open(container_file, 'r') as f:
        data = yaml.safe_load(f)
    
    test_results = data.get('test_results', [])
    
    for result in test_results:
        test_case_id = result.get('test_case_id', 'unknown')
        sequences = result.get('sequences', [])
        
        for sequence in sequences:
            step_results = sequence.get('step_results', [])
            
            for step_result in step_results:
                # Check if step_result is a dict with exactly one of: Pass, Fail, NotExecuted
                if not isinstance(step_result, dict):
                    print(f"Step result in '{test_case_id}' is not a dictionary")
                    sys.exit(1)
                
                # Should have exactly one key that is Pass, Fail, or NotExecuted
                keys = list(step_result.keys())
                valid_variants = ['Pass', 'Fail', 'NotExecuted']
                
                if len(keys) != 1:
                    print(f"Step result in '{test_case_id}' should have exactly one key (Pass/Fail/NotExecuted)")
                    sys.exit(1)
                
                if keys[0] not in valid_variants:
                    print(f"Step result in '{test_case_id}' has invalid variant: {keys[0]}")
                    sys.exit(1)
                
                # Validate step data has required fields
                step_data = step_result[keys[0]]
                if 'step' not in step_data or 'description' not in step_data:
                    print(f"Step result in '{test_case_id}' missing required fields (step, description)")
                    sys.exit(1)
    
    sys.exit(0)
    
except Exception as e:
    print(f"Enum format validation error: {e}")
    sys.exit(1)
PYTHON_SCRIPT
}

# Validate metadata section
validate_metadata() {
    local container_file="$1"
    
    local python_cmd
    python_cmd=$(find_python)
    
    if [[ -z "$python_cmd" ]]; then
        log_verbose "  Python not available, skipping metadata validation"
        return 0
    fi
    
    # Validate metadata using Python
    $python_cmd - "$container_file" << 'PYTHON_SCRIPT'
import sys
import yaml

container_file = sys.argv[1]

try:
    with open(container_file, 'r') as f:
        data = yaml.safe_load(f)
    
    metadata = data.get('metadata', {})
    
    # Check if metadata has statistics
    if 'total_test_cases' not in metadata:
        print("Metadata missing total_test_cases")
        sys.exit(1)
    
    if 'passed_test_cases' not in metadata:
        print("Metadata missing passed_test_cases")
        sys.exit(1)
    
    if 'failed_test_cases' not in metadata:
        print("Metadata missing failed_test_cases")
        sys.exit(1)
    
    # Verify statistics match test_results
    test_results = data.get('test_results', [])
    total_test_cases = len(test_results)
    passed_test_cases = sum(1 for r in test_results if r.get('overall_pass', False))
    failed_test_cases = total_test_cases - passed_test_cases
    
    if metadata['total_test_cases'] != total_test_cases:
        print(f"Metadata total_test_cases ({metadata['total_test_cases']}) doesn't match actual ({total_test_cases})")
        sys.exit(1)
    
    if metadata['passed_test_cases'] != passed_test_cases:
        print(f"Metadata passed_test_cases ({metadata['passed_test_cases']}) doesn't match actual ({passed_test_cases})")
        sys.exit(1)
    
    if metadata['failed_test_cases'] != failed_test_cases:
        print(f"Metadata failed_test_cases ({metadata['failed_test_cases']}) doesn't match actual ({failed_test_cases})")
        sys.exit(1)
    
    sys.exit(0)
    
except Exception as e:
    print(f"Metadata validation error: {e}")
    sys.exit(1)
PYTHON_SCRIPT
}

# Validate overall_pass accuracy
validate_overall_pass() {
    local container_file="$1"
    
    local python_cmd
    python_cmd=$(find_python)
    
    if [[ -z "$python_cmd" ]]; then
        log_verbose "  Python not available, skipping overall_pass validation"
        return 0
    fi
    
    # Validate overall_pass using Python
    $python_cmd - "$container_file" << 'PYTHON_SCRIPT'
import sys
import yaml

container_file = sys.argv[1]

try:
    with open(container_file, 'r') as f:
        data = yaml.safe_load(f)
    
    test_results = data.get('test_results', [])
    
    for result in test_results:
        test_case_id = result.get('test_case_id', 'unknown')
        overall_pass = result.get('overall_pass', False)
        sequences = result.get('sequences', [])
        
        # Calculate expected overall_pass based on sequences
        all_sequences_passed = all(seq.get('all_steps_passed', False) for seq in sequences)
        
        # Also check step counts
        failed_steps = result.get('failed_steps', 0)
        expected_pass = all_sequences_passed and failed_steps == 0
        
        if overall_pass != expected_pass:
            print(f"Test '{test_case_id}': overall_pass={overall_pass} but expected={expected_pass}")
            sys.exit(1)
    
    sys.exit(0)
    
except Exception as e:
    print(f"Overall pass validation error: {e}")
    sys.exit(1)
PYTHON_SCRIPT
}

# Validate a single execution log
validate_log() {
    local verifier_path="$1"
    local log_file="$2"
    local test_id
    test_id=$(get_test_case_id "$log_file")
    local log_basename
    log_basename=$(basename "$log_file")
    
    log_verbose "Validating verification: $log_basename"
    
    # Find corresponding YAML file
    local yaml_file
    yaml_file=$(find_yaml_for_test "$test_id")
    
    if [[ -z "$yaml_file" ]]; then
        log_debug "  ⚠ No YAML file found for $test_id, skipping"
        return
    fi
    
    # Determine expected exit code
    local expected_exit_code
    expected_exit_code=$(get_expected_verifier_exit_code "$yaml_file")
    
    # Run verifier
    local verifier_raw_output="$VERIFICATION_RESULTS_DIR/${test_id}_verifier_raw.yaml"
    local container_output="$VERIFICATION_RESULTS_DIR/${test_id}_container.yaml"
    
    local exit_code
    exit_code=$(run_verifier "$verifier_path" "$log_file" "$test_id" "$verifier_raw_output")
    
    ((VERIFIER_EXECUTED++))
    
    # Validate exit code
    if [[ "$exit_code" -eq "$expected_exit_code" ]]; then
        ((EXIT_CODE_PASSED++))
        log_debug "  ✓ Verifier exit code correct ($exit_code)"
    else
        ((EXIT_CODE_FAILED++))
        append_error "$test_id: Verifier exit code mismatch (got $exit_code, expected $expected_exit_code)"
        log_debug "  ✗ Verifier exit code mismatch"
    fi
    
    # Check if raw output was created
    if [[ ! -f "$verifier_raw_output" ]]; then
        ((VERIFIER_FAILED++))
        append_error "$test_id: Verifier did not generate output file"
        log_debug "  ✗ Verifier output file not created"
        return
    fi
    
    # Transform to container YAML format
    local title_arg="${TITLE:-Test Execution Results - $test_id}"
    transform_to_container_yaml "$verifier_raw_output" "$container_output" "$test_id" "$title_arg"
    
    # Check if container YAML exists
    if [[ ! -f "$container_output" ]]; then
        ((CONTAINER_EXISTS_FAILED++))
        append_error "$test_id: Container YAML file not created"
        log_debug "  ✗ Container YAML file not created"
        return
    fi
    
    ((CONTAINER_EXISTS_PASSED++))
    log_debug "  ✓ Container YAML file created"
    
    # Skip remaining validations if skip-schema is set
    if [[ $SKIP_SCHEMA -eq 1 ]]; then
        return
    fi
    
    # Validate against schema
    if validate_container_schema "$container_output" 2>/dev/null; then
        ((SCHEMA_VALIDATION_PASSED++))
        log_debug "  ✓ Schema validation passed"
    else
        ((SCHEMA_VALIDATION_FAILED++))
        append_error "$test_id: Schema validation failed"
        log_debug "  ✗ Schema validation failed"
    fi
    
    # Validate structure
    local structure_error
    structure_error=$(validate_container_structure "$container_output" 2>&1)
    if [[ $? -eq 0 ]]; then
        ((STRUCTURE_VALIDATION_PASSED++))
        log_debug "  ✓ Structure validation passed"
    else
        ((STRUCTURE_VALIDATION_FAILED++))
        append_error "$test_id: Structure validation failed - $structure_error"
        log_debug "  ✗ Structure validation failed"
    fi
    
    # Validate metadata
    local metadata_error
    metadata_error=$(validate_metadata "$container_output" 2>&1)
    if [[ $? -eq 0 ]]; then
        ((METADATA_VALIDATION_PASSED++))
        log_debug "  ✓ Metadata validation passed"
    else
        ((METADATA_VALIDATION_FAILED++))
        append_error "$test_id: Metadata validation failed - $metadata_error"
        log_debug "  ✗ Metadata validation failed"
    fi
    
    # Validate enum format
    local enum_error
    enum_error=$(validate_enum_format "$container_output" 2>&1)
    if [[ $? -eq 0 ]]; then
        ((ENUM_FORMAT_PASSED++))
        log_debug "  ✓ Enum format validation passed"
    else
        ((ENUM_FORMAT_FAILED++))
        append_error "$test_id: Enum format validation failed - $enum_error"
        log_debug "  ✗ Enum format validation failed"
    fi
    
    # Validate overall_pass
    local overall_pass_error
    overall_pass_error=$(validate_overall_pass "$container_output" 2>&1)
    if [[ $? -eq 0 ]]; then
        ((OVERALL_PASS_PASSED++))
        log_debug "  ✓ Overall pass validation passed"
    else
        ((OVERALL_PASS_FAILED++))
        append_error "$test_id: Overall pass validation failed - $overall_pass_error"
        log_debug "  ✗ Overall pass validation failed"
    fi
}

# Generate verification validation report
generate_report() {
    local report_file="$REPORTS_DIR/stage4_verification_validation_report.txt"
    
    {
        echo "========================================="
        echo "Stage 4 Verification Validation Report"
        echo "========================================="
        echo ""
        echo "Generated: $(date)"
        echo ""
        
        echo "Summary"
        echo "-------"
        echo "Total execution logs validated: $TOTAL_LOGS"
        echo ""
        
        echo "Verifier Execution:"
        echo "  Executed: $VERIFIER_EXECUTED"
        echo "  Failed: $VERIFIER_FAILED"
        echo ""
        
        echo "Exit Code Validation:"
        echo "  Passed: $EXIT_CODE_PASSED"
        echo "  Failed: $EXIT_CODE_FAILED"
        echo ""
        
        echo "Container YAML Existence:"
        echo "  Passed: $CONTAINER_EXISTS_PASSED"
        echo "  Failed: $CONTAINER_EXISTS_FAILED"
        echo ""
        
        if [[ $SKIP_SCHEMA -eq 0 ]]; then
            echo "Schema Validation:"
            echo "  Passed: $SCHEMA_VALIDATION_PASSED"
            echo "  Failed: $SCHEMA_VALIDATION_FAILED"
            echo ""
            
            echo "Structure Validation:"
            echo "  Passed: $STRUCTURE_VALIDATION_PASSED"
            echo "  Failed: $STRUCTURE_VALIDATION_FAILED"
            echo ""
            
            echo "Metadata Validation:"
            echo "  Passed: $METADATA_VALIDATION_PASSED"
            echo "  Failed: $METADATA_VALIDATION_FAILED"
            echo ""
            
            echo "Enum Format Validation:"
            echo "  Passed: $ENUM_FORMAT_PASSED"
            echo "  Failed: $ENUM_FORMAT_FAILED"
            echo ""
            
            echo "Overall Pass Validation:"
            echo "  Passed: $OVERALL_PASS_PASSED"
            echo "  Failed: $OVERALL_PASS_FAILED"
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
        local total_failed=$((VERIFIER_FAILED + EXIT_CODE_FAILED + CONTAINER_EXISTS_FAILED + SCHEMA_VALIDATION_FAILED + STRUCTURE_VALIDATION_FAILED + METADATA_VALIDATION_FAILED + ENUM_FORMAT_FAILED + OVERALL_PASS_FAILED))
        
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
    section "Stage 4 Verification Validation Report"
    
    echo ""
    log_info "Total execution logs validated: $TOTAL_LOGS"
    echo ""
    
    log_info "Verifier Execution:"
    log_info "  Executed: $VERIFIER_EXECUTED"
    log_info "  Failed: $VERIFIER_FAILED"
    echo ""
    
    log_info "Exit Code Validation:"
    log_info "  Passed: $EXIT_CODE_PASSED"
    log_info "  Failed: $EXIT_CODE_FAILED"
    echo ""
    
    log_info "Container YAML Existence:"
    log_info "  Passed: $CONTAINER_EXISTS_PASSED"
    log_info "  Failed: $CONTAINER_EXISTS_FAILED"
    echo ""
    
    if [[ $SKIP_SCHEMA -eq 0 ]]; then
        log_info "Schema Validation:"
        log_info "  Passed: $SCHEMA_VALIDATION_PASSED"
        log_info "  Failed: $SCHEMA_VALIDATION_FAILED"
        echo ""
        
        log_info "Structure Validation:"
        log_info "  Passed: $STRUCTURE_VALIDATION_PASSED"
        log_info "  Failed: $STRUCTURE_VALIDATION_FAILED"
        echo ""
        
        log_info "Metadata Validation:"
        log_info "  Passed: $METADATA_VALIDATION_PASSED"
        log_info "  Failed: $METADATA_VALIDATION_FAILED"
        echo ""
        
        log_info "Enum Format Validation:"
        log_info "  Passed: $ENUM_FORMAT_PASSED"
        log_info "  Failed: $ENUM_FORMAT_FAILED"
        echo ""
        
        log_info "Overall Pass Validation:"
        log_info "  Passed: $OVERALL_PASS_PASSED"
        log_info "  Failed: $OVERALL_PASS_FAILED"
        echo ""
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
    
    local total_failed=$((VERIFIER_FAILED + EXIT_CODE_FAILED + CONTAINER_EXISTS_FAILED + SCHEMA_VALIDATION_FAILED + STRUCTURE_VALIDATION_FAILED + METADATA_VALIDATION_FAILED + ENUM_FORMAT_FAILED + OVERALL_PASS_FAILED))
    
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
    
    section "Stage 4 Verification Validation"
    
    log_info "Validating verification results..."
    echo ""
    
    # Ensure directories exist
    ensure_directories
    
    # Validate schema file
    validate_schema_file
    
    # Find verifier binary
    local verifier_path
    verifier_path=$(find_verifier_binary)
    log_verbose "Using verifier: $verifier_path"
    
    # Find all execution log files (bash 3.2 compatible)
    local log_files=()
    while IFS= read -r file; do
        log_files+=("$file")
    done < <(find_execution_logs)
    
    if [[ ${#log_files[@]} -eq 0 ]]; then
        log_warning "No execution log files found in $SCRIPTS_DIR matching pattern: ${TEST_PATTERN}_execution_log.json"
        exit 0
    fi
    
    TOTAL_LOGS=${#log_files[@]}
    log_info "Found $TOTAL_LOGS execution log(s) to validate"
    echo ""
    
    # Validate each log
    for log_file in "${log_files[@]}"; do
        validate_log "$verifier_path" "$log_file"
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
