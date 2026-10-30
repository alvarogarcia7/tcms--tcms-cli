#!/usr/bin/env bash
#
# validate_stage1_yaml.sh - Validates Stage 1 (test case YAML generation)
#
# DESCRIPTION:
#   This script validates all test case YAML files in the Stage 1 acceptance test directory.
#   It performs comprehensive validation including:
#   - JSON schema validation against schemas/tcms-schemas/tcms/test-case-main.schema.json
#   - YAML structure validation (required fields)
#   - Test sequences validation (at least 1 sequence with valid steps)
#   - Hooks syntax validation (if present)
#   - Hydration variables validation (if present)
#   - Prerequisites structure validation (if present)
#
# USAGE:
#   ./test-acceptance/validate_stage1_yaml.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose    Enable verbose output
#   -h, --help       Show this help message
#
# EXIT CODES:
#   0 - All validations passed
#   1 - One or more validations failed
#
# OUTPUT:
#   Generates a detailed validation report with:
#   - Pass/fail counts for each validation category
#   - Specific error messages for each failed file
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
SCHEMA_FILE="$REPO_ROOT/schemas/tcms-schemas/tcms/test-case-main.schema.json"
VERBOSE=0

# Validation counters
TOTAL_FILES=0
SCHEMA_PASSED=0
SCHEMA_FAILED=0
STRUCTURE_PASSED=0
STRUCTURE_FAILED=0
SEQUENCES_PASSED=0
SEQUENCES_FAILED=0
HOOKS_PASSED=0
HOOKS_FAILED=0
HYDRATION_PASSED=0
HYDRATION_FAILED=0
PREREQS_PASSED=0
PREREQS_FAILED=0

# Error tracking
declare -a ERRORS

# Usage function
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Validates Stage 1 (test case YAML generation) for acceptance testing.

OPTIONS:
    -v, --verbose    Enable verbose output
    -h, --help       Show this help message

DESCRIPTION:
    Validates all test case YAML files in test-acceptance/00_test_cases/ by:
    - Running schema validation against schemas/tcms-schemas/tcms/test-case-main.schema.json
    - Verifying required YAML structure fields
    - Checking test_sequences array has at least 1 valid sequence
    - Validating hooks syntax if present
    - Checking hydration_vars declarations if present
    - Verifying prerequisites structure if present

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
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done
}

# Find validate-yaml binary
find_validate_yaml() {
    local validate_yaml
    validate_yaml=$(find_binary "validate-yaml" "VALIDATE_YAML_BIN")
    
    if [[ -z "$validate_yaml" ]]; then
        log_error "validate-yaml binary not found"
        log_error "Please build it with: cargo build --bin validate-yaml"
        exit 1
    fi
    
    echo "$validate_yaml"
}

# Validate schema file exists
validate_schema_file() {
    if [[ ! -f "$SCHEMA_FILE" ]]; then
        log_error "Schema file not found: $SCHEMA_FILE"
        exit 1
    fi
    log_verbose "Using schema: $SCHEMA_FILE"
}

# Validate test cases directory exists
validate_test_cases_dir() {
    if [[ ! -d "$TEST_CASES_DIR" ]]; then
        log_error "Test cases directory not found: $TEST_CASES_DIR"
        exit 1
    fi
    log_verbose "Using test cases directory: $TEST_CASES_DIR"
}

# Find all YAML files in test cases directory
find_yaml_files() {
    find "$TEST_CASES_DIR" -type f \( -name "*.yaml" -o -name "*.yml" \) | sort
}

# Validate YAML file against schema
validate_yaml_schema() {
    local yaml_file="$1"
    local validate_yaml="$2"
    local error_msg
    
    if error_msg=$("$validate_yaml" --schema "$SCHEMA_FILE" "$yaml_file" 2>&1); then
        return 0
    else
        ERRORS+=("$(basename "$yaml_file"): Schema validation failed: $error_msg")
        return 1
    fi
}

# Parse YAML file using Python (portable across systems)
parse_yaml_field() {
    local yaml_file="$1"
    local field="$2"
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        value = data.get('$field')
        if value is not None:
            print(value)
        else:
            sys.exit(1)
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null
}

# Check if YAML field exists
yaml_field_exists() {
    local yaml_file="$1"
    local field="$2"
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        if '$field' in data:
            sys.exit(0)
        else:
            sys.exit(1)
except Exception:
    sys.exit(1)
" 2>/dev/null
}

# Get number of test sequences
get_test_sequences_count() {
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
        print(len(sequences))
except Exception:
    print('0')
    sys.exit(1)
" 2>/dev/null
}

# Validate sequence has steps
validate_sequence_steps() {
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
            if not steps or len(steps) == 0:
                print(f'Sequence {seq.get(\"id\", \"unknown\")} has no steps', file=sys.stderr)
                sys.exit(1)
        sys.exit(0)
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null
}

# Validate YAML structure (required fields)
validate_yaml_structure() {
    local yaml_file="$1"
    local required_fields=("requirement" "item" "tc" "id" "description" "test_sequences")
    local missing_fields=()
    
    for field in "${required_fields[@]}"; do
        if ! yaml_field_exists "$yaml_file" "$field"; then
            missing_fields+=("$field")
        fi
    done
    
    if [[ ${#missing_fields[@]} -gt 0 ]]; then
        ERRORS+=("$(basename "$yaml_file"): Missing required fields: ${missing_fields[*]}")
        return 1
    fi
    
    return 0
}

# Validate test sequences
validate_test_sequences() {
    local yaml_file="$1"
    local seq_count
    
    seq_count=$(get_test_sequences_count "$yaml_file")
    
    if [[ "$seq_count" -lt 1 ]]; then
        ERRORS+=("$(basename "$yaml_file"): test_sequences must have at least 1 sequence")
        return 1
    fi
    
    if ! validate_sequence_steps "$yaml_file"; then
        ERRORS+=("$(basename "$yaml_file"): One or more sequences have no steps")
        return 1
    fi
    
    return 0
}

# Validate hooks syntax
validate_hooks() {
    local yaml_file="$1"
    
    # Hooks are optional
    if ! yaml_field_exists "$yaml_file" "hooks"; then
        return 0
    fi
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        hooks = data.get('hooks', {})
        
        if not isinstance(hooks, dict):
            print('hooks must be a dictionary', file=sys.stderr)
            sys.exit(1)
        
        valid_hook_types = [
            'script_start', 'setup_test', 'before_sequence', 'after_sequence',
            'before_step', 'after_step', 'teardown_test', 'script_end'
        ]
        
        for hook_type, hook_value in hooks.items():
            if hook_type == 'on_error':
                # on_error is a special field
                if hook_value not in ['fail', 'continue']:
                    print(f'Invalid on_error value: {hook_value}', file=sys.stderr)
                    sys.exit(1)
            elif hook_type not in valid_hook_types:
                print(f'Invalid hook type: {hook_type}', file=sys.stderr)
                sys.exit(1)
            else:
                # Hook value should be a string (path) or object with command
                if isinstance(hook_value, str):
                    # Simple string format (path)
                    pass
                elif isinstance(hook_value, dict):
                    # Object format with command and on_error
                    if 'command' not in hook_value:
                        print(f'Hook {hook_type} missing required field: command', file=sys.stderr)
                        sys.exit(1)
                    if 'on_error' in hook_value and hook_value['on_error'] not in ['fail', 'continue']:
                        print(f'Invalid on_error value for {hook_type}: {hook_value[\"on_error\"]}', file=sys.stderr)
                        sys.exit(1)
                else:
                    print(f'Invalid hook value type for {hook_type}', file=sys.stderr)
                    sys.exit(1)
        
        sys.exit(0)
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null
    
    local result=$?
    if [[ $result -ne 0 ]]; then
        ERRORS+=("$(basename "$yaml_file"): Invalid hooks syntax")
        return 1
    fi
    
    return 0
}

# Validate hydration_vars
validate_hydration_vars() {
    local yaml_file="$1"
    
    # Hydration vars are optional
    if ! yaml_field_exists "$yaml_file" "hydration_vars"; then
        return 0
    fi
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        hydration_vars = data.get('hydration_vars', {})
        
        if not isinstance(hydration_vars, dict):
            print('hydration_vars must be a dictionary', file=sys.stderr)
            sys.exit(1)
        
        for var_name, var_config in hydration_vars.items():
            if not isinstance(var_config, dict):
                print(f'Variable {var_name} config must be a dictionary', file=sys.stderr)
                sys.exit(1)
            
            if 'name' not in var_config:
                print(f'Variable {var_name} missing required field: name', file=sys.stderr)
                sys.exit(1)
        
        sys.exit(0)
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null
    
    local result=$?
    if [[ $result -ne 0 ]]; then
        ERRORS+=("$(basename "$yaml_file"): Invalid hydration_vars syntax")
        return 1
    fi
    
    return 0
}

# Validate prerequisites
validate_prerequisites() {
    local yaml_file="$1"
    
    # Prerequisites are optional
    if ! yaml_field_exists "$yaml_file" "prerequisites"; then
        return 0
    fi
    
    local python_cmd
    python_cmd=$(find_python)
    
    $python_cmd -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
        prerequisites = data.get('prerequisites', [])
        
        if not isinstance(prerequisites, list):
            print('prerequisites must be an array', file=sys.stderr)
            sys.exit(1)
        
        for i, prereq in enumerate(prerequisites):
            if not isinstance(prereq, dict):
                print(f'Prerequisite {i} must be an object', file=sys.stderr)
                sys.exit(1)
            
            if 'type' not in prereq:
                print(f'Prerequisite {i} missing required field: type', file=sys.stderr)
                sys.exit(1)
            
            if prereq['type'] not in ['manual', 'automatic']:
                print(f'Prerequisite {i} has invalid type: {prereq[\"type\"]}', file=sys.stderr)
                sys.exit(1)
            
            if 'description' not in prereq:
                print(f'Prerequisite {i} missing required field: description', file=sys.stderr)
                sys.exit(1)
            
            # Automatic prerequisites must have verification_command
            if prereq['type'] == 'automatic' and 'verification_command' not in prereq:
                print(f'Automatic prerequisite {i} missing required field: verification_command', file=sys.stderr)
                sys.exit(1)
        
        sys.exit(0)
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null
    
    local result=$?
    if [[ $result -ne 0 ]]; then
        ERRORS+=("$(basename "$yaml_file"): Invalid prerequisites syntax")
        return 1
    fi
    
    return 0
}

# Validate a single YAML file
validate_yaml_file() {
    local yaml_file="$1"
    local validate_yaml="$2"
    local file_basename
    file_basename=$(basename "$yaml_file")
    
    log_verbose "Validating: $file_basename"
    
    # 1. Schema validation
    if validate_yaml_schema "$yaml_file" "$validate_yaml"; then
        ((SCHEMA_PASSED++))
        log_debug "  ✓ Schema validation passed"
    else
        ((SCHEMA_FAILED++))
        log_debug "  ✗ Schema validation failed"
    fi
    
    # 2. Structure validation
    if validate_yaml_structure "$yaml_file"; then
        ((STRUCTURE_PASSED++))
        log_debug "  ✓ Structure validation passed"
    else
        ((STRUCTURE_FAILED++))
        log_debug "  ✗ Structure validation failed"
    fi
    
    # 3. Test sequences validation
    if validate_test_sequences "$yaml_file"; then
        ((SEQUENCES_PASSED++))
        log_debug "  ✓ Test sequences validation passed"
    else
        ((SEQUENCES_FAILED++))
        log_debug "  ✗ Test sequences validation failed"
    fi
    
    # 4. Hooks validation (if present)
    if validate_hooks "$yaml_file"; then
        ((HOOKS_PASSED++))
        log_debug "  ✓ Hooks validation passed"
    else
        ((HOOKS_FAILED++))
        log_debug "  ✗ Hooks validation failed"
    fi
    
    # 5. Hydration vars validation (if present)
    if validate_hydration_vars "$yaml_file"; then
        ((HYDRATION_PASSED++))
        log_debug "  ✓ Hydration vars validation passed"
    else
        ((HYDRATION_FAILED++))
        log_debug "  ✗ Hydration vars validation failed"
    fi
    
    # 6. Prerequisites validation (if present)
    if validate_prerequisites "$yaml_file"; then
        ((PREREQS_PASSED++))
        log_debug "  ✓ Prerequisites validation passed"
    else
        ((PREREQS_FAILED++))
        log_debug "  ✗ Prerequisites validation failed"
    fi
}

# Print validation report
print_report() {
    section "Stage 1 YAML Validation Report"
    
    echo ""
    log_info "Total files validated: $TOTAL_FILES"
    echo ""
    
    log_info "Schema Validation:"
    log_info "  Passed: $SCHEMA_PASSED"
    log_info "  Failed: $SCHEMA_FAILED"
    echo ""
    
    log_info "Structure Validation:"
    log_info "  Passed: $STRUCTURE_PASSED"
    log_info "  Failed: $STRUCTURE_FAILED"
    echo ""
    
    log_info "Test Sequences Validation:"
    log_info "  Passed: $SEQUENCES_PASSED"
    log_info "  Failed: $SEQUENCES_FAILED"
    echo ""
    
    log_info "Hooks Validation:"
    log_info "  Passed: $HOOKS_PASSED"
    log_info "  Failed: $HOOKS_FAILED"
    echo ""
    
    log_info "Hydration Variables Validation:"
    log_info "  Passed: $HYDRATION_PASSED"
    log_info "  Failed: $HYDRATION_FAILED"
    echo ""
    
    log_info "Prerequisites Validation:"
    log_info "  Passed: $PREREQS_PASSED"
    log_info "  Failed: $PREREQS_FAILED"
    echo ""
    
    if [[ ${#ERRORS[@]} -gt 0 ]]; then
        section "Validation Errors"
        echo ""
        for error in "${ERRORS[@]}"; do
            fail "$error"
        done
        echo ""
    fi
    
    local total_failed=$((SCHEMA_FAILED + STRUCTURE_FAILED + SEQUENCES_FAILED + HOOKS_FAILED + HYDRATION_FAILED + PREREQS_FAILED))
    
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
    
    section "Stage 1 YAML Validation"
    
    log_info "Validating test case YAML files..."
    echo ""
    
    # Validate prerequisites
    validate_schema_file
    validate_test_cases_dir
    
    # Find validate-yaml binary
    local validate_yaml
    validate_yaml=$(find_validate_yaml)
    log_verbose "Using validate-yaml: $validate_yaml"
    
    # Find all YAML files (bash 3.2 compatible)
    local yaml_files=()
    while IFS= read -r file; do
        yaml_files+=("$file")
    done < <(find_yaml_files)
    
    if [[ ${#yaml_files[@]} -eq 0 ]]; then
        log_warning "No YAML files found in $TEST_CASES_DIR"
        exit 0
    fi
    
    TOTAL_FILES=${#yaml_files[@]}
    log_info "Found $TOTAL_FILES YAML file(s) to validate"
    echo ""
    
    # Validate each file
    for yaml_file in "${yaml_files[@]}"; do
        validate_yaml_file "$yaml_file" "$validate_yaml"
    done
    
    echo ""
    
    # Print report and exit with appropriate code
    if print_report; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"
