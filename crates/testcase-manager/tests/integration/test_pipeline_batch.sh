#!/bin/bash
#
# Batch Pipeline Processor - Apply 5-stage pipeline to all test case YAMLs
#
# This script discovers all test case YAML files in the project directory and
# applies the full 5-stage pipeline to each one:
# Stage 1: YAML test case validation
# Stage 2: Shell script generation with syntax and shellcheck validation
# Stage 3: Script execution with JSON output validation
# Stage 4: Verification against test case (YAML output)
# Stage 5: Result summary generation (JSON output)
#
# Reports grand totals of success/failure per stage and per test case.
#
# Usage: ./test_pipeline_batch.sh [--no-remove] [--testcases-dir DIR]
#
# Requires: bash 3.2+
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/shellcheck-helper.sh" || exit 1

# Find binaries using workspace-aware search
cd "$PROJECT_ROOT"
TEST_EXECUTOR_BIN=$(find_binary_or_exit "test-executor")
VALIDATE_YAML_BIN=$(find_binary_or_exit "validate-yaml")
VERIFIER_BIN=$(find_binary_or_exit "verifier")

# Default values
REMOVE_TEMP=1
TESTCASES_DIR="$PROJECT_ROOT/testcases"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-remove)
            REMOVE_TEMP=0
            shift
            ;;
        --testcases-dir)
            TESTCASES_DIR="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--no-remove] [--testcases-dir DIR]"
            exit 1
            ;;
    esac
done

echo "=============================================="
echo "Batch Pipeline Processor"
echo "=============================================="
echo ""

# Check prerequisites
section "Checking Prerequisites"

pass "test-executor binary found: $TEST_EXECUTOR_BIN"
pass "validate-yaml binary found: $VALIDATE_YAML_BIN"
pass "verifier binary found: $VERIFIER_BIN"

if ! command -v jq &> /dev/null; then
    fail "jq not found, required for JSON validation"
    exit 1
fi
pass "jq available for JSON validation"

# Discover all test case YAML files
section "Discovering Test Cases"

if [[ ! -d "$TESTCASES_DIR" ]]; then
    fail "Test cases directory not found: $TESTCASES_DIR"
    exit 1
fi

info "Searching for test case YAMLs in: $TESTCASES_DIR"

# Find all YAML files (bash 3.2 compatible - no mapfile)
ALL_YAMLS=()
while IFS= read -r yaml_file; do
    ALL_YAMLS+=("$yaml_file")
done < <(find "$TESTCASES_DIR" -type f \( -name "*.yml" -o -name "*.yaml" \) | sort)

if [ ${#ALL_YAMLS[@]} -eq 0 ]; then
    fail "No YAML files found in $TESTCASES_DIR"
    exit 1
fi

pass "Found ${#ALL_YAMLS[@]} YAML files"

# Create temporary directory for test outputs
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Initialize stage counters (bash 3.2 compatible - no associative arrays)
STAGE1_SUCCESS=0
STAGE1_FAILURE=0
STAGE2_SUCCESS=0
STAGE2_FAILURE=0
STAGE3_SUCCESS=0
STAGE3_FAILURE=0
STAGE4_SUCCESS=0
STAGE4_FAILURE=0
STAGE5_SUCCESS=0
STAGE5_FAILURE=0

# Track overall test case results
TESTCASE_RESULTS=()

# Create results subdirectories
mkdir -p "$TEMP_DIR/logs"
mkdir -p "$TEMP_DIR/scripts"
mkdir -p "$TEMP_DIR/executions"
mkdir -p "$TEMP_DIR/verifications"
mkdir -p "$TEMP_DIR/results"

# Process each YAML file
section "Processing Test Cases"
echo ""

TOTAL_TESTCASES=${#ALL_YAMLS[@]}
CURRENT=0

for YAML_FILE in "${ALL_YAMLS[@]}"; do
    CURRENT=$((CURRENT + 1))
    YAML_BASENAME=$(basename "$YAML_FILE")
    YAML_NAME="${YAML_BASENAME%.*}"
    
    echo "[$CURRENT/$TOTAL_TESTCASES] Processing: $YAML_BASENAME"
    
    # Track stages for this test case
    TC_STAGE1=0
    TC_STAGE2=0
    TC_STAGE3=0
    TC_STAGE4=0
    TC_STAGE5=0
    TC_FAILED_STAGE=""
    
    #-------------------------------------------------------------------------
    # Stage 1: YAML Test Case Validation
    #-------------------------------------------------------------------------

    STAGE1_OUTPUT="$TEMP_DIR/logs/${YAML_NAME}_stage1.log"

    # Validate YAML files that are test cases (type: test_case)
    # Use legacy schema which works with current YAML files
    # Skip files that don't have 'type: test_case' (containers, verification results, etc.)
    # Use word boundary to avoid matching test_case_verification_result, etc.
    if ! grep -qE "^type: test_case\b" "$YAML_FILE" 2>/dev/null; then
        # Not a test case YAML file, skip to next file
        echo "  ⊘ Stage 1: Skipped (not a test case file)"
        continue
    fi

    # Validate the test case YAML against the schema
    if "$VALIDATE_YAML_BIN" --schemas-root "$PROJECT_ROOT/schemas" --schema "$PROJECT_ROOT/schemas/tcms-schemas/tcms/test-case-main.schema.json" "$YAML_FILE" > "$STAGE1_OUTPUT" 2>&1; then
        TC_STAGE1=1
        STAGE1_SUCCESS=$((STAGE1_SUCCESS + 1))
        echo "  ✓ Stage 1: YAML validation passed"
    else
        TC_STAGE1=0
        STAGE1_FAILURE=$((STAGE1_FAILURE + 1))
        TC_FAILED_STAGE="Stage 1"
        echo "  ✗ Stage 1: YAML validation failed"
        TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
        continue
    fi
    
    # Extract test case ID from YAML for proper naming
    TEST_CASE_ID=$(grep -m1 "^id:" "$YAML_FILE" | awk '{print $2}' | tr -d '"' || echo "$YAML_NAME")
    
    #-------------------------------------------------------------------------
    # Stage 2: Generate Script with Syntax Validation
    #-------------------------------------------------------------------------
    
    GENERATED_SCRIPT="$TEMP_DIR/scripts/${TEST_CASE_ID}_test.sh"
    STAGE2_OUTPUT="$TEMP_DIR/logs/${YAML_NAME}_stage2.log"
    
    if "$TEST_EXECUTOR_BIN" generate --json-log "$YAML_FILE" -o "$GENERATED_SCRIPT" > "$STAGE2_OUTPUT" 2>&1; then
        # Check bash syntax
        if bash -n "$GENERATED_SCRIPT" 2>&1; then
            # Run shellcheck (optional - warnings don't fail)
            if command -v shellcheck &> /dev/null; then
                if validate_with_shellcheck "$GENERATED_SCRIPT" "Generated script" > /dev/null 2>&1; then
                    TC_STAGE2=1
                    STAGE2_SUCCESS=$((STAGE2_SUCCESS + 1))
                    echo "  ✓ Stage 2: Script generation and validation passed"
                else
                    # Shellcheck warnings - treat as success but note it
                    TC_STAGE2=1
                    STAGE2_SUCCESS=$((STAGE2_SUCCESS + 1))
                    echo "  ✓ Stage 2: Script generation passed (shellcheck warnings)"
                fi
            else
                TC_STAGE2=1
                STAGE2_SUCCESS=$((STAGE2_SUCCESS + 1))
                echo "  ✓ Stage 2: Script generation and syntax validation passed"
            fi
        else
            TC_STAGE2=0
            STAGE2_FAILURE=$((STAGE2_FAILURE + 1))
            TC_FAILED_STAGE="Stage 2"
            echo "  ✗ Stage 2: Bash syntax validation failed"
            TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
            continue
        fi
    else
        TC_STAGE2=0
        STAGE2_FAILURE=$((STAGE2_FAILURE + 1))
        TC_FAILED_STAGE="Stage 2"
        echo "  ✗ Stage 2: Script generation failed"
        TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
        continue
    fi
    
    #-------------------------------------------------------------------------
    # Stage 3: Execute Script with JSON Output Validation
    #-------------------------------------------------------------------------
    
    STAGE3_OUTPUT="$TEMP_DIR/logs/${YAML_NAME}_stage3.log"
    
    # Execute the script in a subshell with timeout
    cd "$TEMP_DIR/executions"
    export DEBIAN_FRONTEND=noninteractive
    
    # Execute with timeout (60 seconds per test case)
    # Use timeout if available, otherwise run without timeout
    EXEC_EXIT=0
    if command -v timeout >/dev/null 2>&1; then
        if timeout 60 bash "$GENERATED_SCRIPT" > "$STAGE3_OUTPUT" 2>&1; then
            EXEC_EXIT=0
        else
            EXEC_EXIT=$?
        fi
    else
        # No timeout command available (e.g., macOS) - run without timeout
        if bash "$GENERATED_SCRIPT" > "$STAGE3_OUTPUT" 2>&1; then
            EXEC_EXIT=0
        else
            EXEC_EXIT=$?
        fi
    fi
    
    unset DEBIAN_FRONTEND
    cd "$PROJECT_ROOT"
    
    # Find execution JSON (should exist even for failed executions)
    EXECUTION_JSON=$(find "$TEMP_DIR/executions" -name "${TEST_CASE_ID}*_execution*.json" | head -1)
    
    if [[ -n "$EXECUTION_JSON" ]] && [[ -f "$EXECUTION_JSON" ]]; then
        # Validate JSON is well-formed
        if jq empty "$EXECUTION_JSON" 2>/dev/null; then
            # Check if we have at least one step recorded
            STEP_COUNT=$(jq 'length' "$EXECUTION_JSON" 2>/dev/null || echo "0")
            if [ "$STEP_COUNT" -ge 1 ]; then
                TC_STAGE3=1
                STAGE3_SUCCESS=$((STAGE3_SUCCESS + 1))
                echo "  ✓ Stage 3: Script execution completed ($STEP_COUNT steps recorded)"
            else
                TC_STAGE3=0
                STAGE3_FAILURE=$((STAGE3_FAILURE + 1))
                TC_FAILED_STAGE="Stage 3"
                echo "  ✗ Stage 3: No steps recorded in execution JSON"
                TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
                continue
            fi
        else
            TC_STAGE3=0
            STAGE3_FAILURE=$((STAGE3_FAILURE + 1))
            TC_FAILED_STAGE="Stage 3"
            echo "  ✗ Stage 3: Execution JSON is malformed"
            TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
            continue
        fi
    else
        TC_STAGE3=0
        STAGE3_FAILURE=$((STAGE3_FAILURE + 1))
        TC_FAILED_STAGE="Stage 3"
        echo "  ✗ Stage 3: No execution JSON found"
        TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
        continue
    fi
    
    #-------------------------------------------------------------------------
    # Stage 4: Verify Execution with YAML Output
    #-------------------------------------------------------------------------
    
    VERIFICATION_YAML="$TEMP_DIR/verifications/${TEST_CASE_ID}_verification.yaml"
    STAGE4_OUTPUT="$TEMP_DIR/logs/${YAML_NAME}_stage4.log"
    
    # Get the directory containing the original YAML for -d parameter
    YAML_DIR=$(dirname "$YAML_FILE")
    
    # Run verifier with the YAML filename (may exit non-zero if test failed, but that's ok)
    "$VERIFIER_BIN" --log "$EXECUTION_JSON" --test-case "$YAML_BASENAME" -d "$YAML_DIR" --format yaml -o "$VERIFICATION_YAML" > "$STAGE4_OUTPUT" 2>&1 || true
    
    if [[ -f "$VERIFICATION_YAML" ]]; then
        # Validate verification YAML against schema (auto-resolved from YAML's schema field)
        if "$VALIDATE_YAML_BIN" --schemas-root "$PROJECT_ROOT/schemas" "$VERIFICATION_YAML" > /dev/null 2>&1; then
            TC_STAGE4=1
            STAGE4_SUCCESS=$((STAGE4_SUCCESS + 1))
            
            # Check if test passed or failed
            if grep -q "overall_pass: true" "$VERIFICATION_YAML"; then
                echo "  ✓ Stage 4: Verification completed (test passed)"
            else
                echo "  ✓ Stage 4: Verification completed (test failed)"
            fi
        else
            TC_STAGE4=0
            STAGE4_FAILURE=$((STAGE4_FAILURE + 1))
            TC_FAILED_STAGE="Stage 4"
            echo "  ✗ Stage 4: Verification YAML schema validation failed"
            TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
            continue
        fi
    else
        TC_STAGE4=0
        STAGE4_FAILURE=$((STAGE4_FAILURE + 1))
        TC_FAILED_STAGE="Stage 4"
        echo "  ✗ Stage 4: Verification YAML not generated"
        TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
        continue
    fi
    
    #-------------------------------------------------------------------------
    # Stage 5: Generate Test Result Summary (JSON)
    #-------------------------------------------------------------------------
    
    RESULT_JSON="$TEMP_DIR/results/${TEST_CASE_ID}_result.json"
    STAGE5_OUTPUT="$TEMP_DIR/logs/${YAML_NAME}_stage5.log"
    
    # Run verifier to generate JSON result
    "$VERIFIER_BIN" --log "$EXECUTION_JSON" --test-case "$TEST_CASE_ID" -d "$YAML_DIR" --format json -o "$RESULT_JSON" > "$STAGE5_OUTPUT" 2>&1 || true
    
    if [[ -f "$RESULT_JSON" ]]; then
        # Validate JSON is well-formed
        if jq empty "$RESULT_JSON" 2>/dev/null; then
            # Check for required fields
            OVERALL_PASS=$(jq -r '.test_results[0].overall_pass' "$RESULT_JSON" 2>/dev/null || echo "null")
            if [[ "$OVERALL_PASS" != "null" ]]; then
                TC_STAGE5=1
                STAGE5_SUCCESS=$((STAGE5_SUCCESS + 1))
                
                if [[ "$OVERALL_PASS" == "true" ]]; then
                    echo "  ✓ Stage 5: Result summary generated (overall: PASS)"
                    TESTCASE_RESULTS+=("$YAML_BASENAME|PASS|All stages")
                else
                    echo "  ✓ Stage 5: Result summary generated (overall: FAIL)"
                    TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|Test verification")
                fi
            else
                TC_STAGE5=0
                STAGE5_FAILURE=$((STAGE5_FAILURE + 1))
                TC_FAILED_STAGE="Stage 5"
                echo "  ✗ Stage 5: Result JSON missing required fields"
                TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
                continue
            fi
        else
            TC_STAGE5=0
            STAGE5_FAILURE=$((STAGE5_FAILURE + 1))
            TC_FAILED_STAGE="Stage 5"
            echo "  ✗ Stage 5: Result JSON is malformed"
            TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
            continue
        fi
    else
        TC_STAGE5=0
        STAGE5_FAILURE=$((STAGE5_FAILURE + 1))
        TC_FAILED_STAGE="Stage 5"
        echo "  ✗ Stage 5: Result JSON not generated"
        TESTCASE_RESULTS+=("$YAML_BASENAME|FAIL|$TC_FAILED_STAGE")
        continue
    fi
    
    echo ""
done

#=============================================================================
# Generate Summary Report
#=============================================================================

section "Pipeline Execution Summary"
echo ""

echo "=============================================="
echo "Stage Results"
echo "=============================================="
echo ""

# Calculate totals
STAGE1_TOTAL=$((STAGE1_SUCCESS + STAGE1_FAILURE))
STAGE2_TOTAL=$((STAGE2_SUCCESS + STAGE2_FAILURE))
STAGE3_TOTAL=$((STAGE3_SUCCESS + STAGE3_FAILURE))
STAGE4_TOTAL=$((STAGE4_SUCCESS + STAGE4_FAILURE))
STAGE5_TOTAL=$((STAGE5_SUCCESS + STAGE5_FAILURE))

# Stage 1
echo "Stage 1: YAML Test Case Validation"
echo "  Total:   $STAGE1_TOTAL"
echo "  Success: $STAGE1_SUCCESS"
echo "  Failure: $STAGE1_FAILURE"
if [ $STAGE1_TOTAL -gt 0 ]; then
    STAGE1_PCT=$((STAGE1_SUCCESS * 100 / STAGE1_TOTAL))
    echo "  Success Rate: ${STAGE1_PCT}%"
fi
echo ""

# Stage 2
echo "Stage 2: Script Generation and Validation"
echo "  Total:   $STAGE2_TOTAL"
echo "  Success: $STAGE2_SUCCESS"
echo "  Failure: $STAGE2_FAILURE"
if [ $STAGE2_TOTAL -gt 0 ]; then
    STAGE2_PCT=$((STAGE2_SUCCESS * 100 / STAGE2_TOTAL))
    echo "  Success Rate: ${STAGE2_PCT}%"
fi
echo ""

# Stage 3
echo "Stage 3: Script Execution and JSON Validation"
echo "  Total:   $STAGE3_TOTAL"
echo "  Success: $STAGE3_SUCCESS"
echo "  Failure: $STAGE3_FAILURE"
if [ $STAGE3_TOTAL -gt 0 ]; then
    STAGE3_PCT=$((STAGE3_SUCCESS * 100 / STAGE3_TOTAL))
    echo "  Success Rate: ${STAGE3_PCT}%"
fi
echo ""

# Stage 4
echo "Stage 4: Verification with YAML Output"
echo "  Total:   $STAGE4_TOTAL"
echo "  Success: $STAGE4_SUCCESS"
echo "  Failure: $STAGE4_FAILURE"
if [ $STAGE4_TOTAL -gt 0 ]; then
    STAGE4_PCT=$((STAGE4_SUCCESS * 100 / STAGE4_TOTAL))
    echo "  Success Rate: ${STAGE4_PCT}%"
fi
echo ""

# Stage 5
echo "Stage 5: Result Summary Generation"
echo "  Total:   $STAGE5_TOTAL"
echo "  Success: $STAGE5_SUCCESS"
echo "  Failure: $STAGE5_FAILURE"
if [ $STAGE5_TOTAL -gt 0 ]; then
    STAGE5_PCT=$((STAGE5_SUCCESS * 100 / STAGE5_TOTAL))
    echo "  Success Rate: ${STAGE5_PCT}%"
fi
echo ""

echo "=============================================="
echo "Test Case Results"
echo "=============================================="
echo ""

# Count test case outcomes
TESTCASE_PASS=0
TESTCASE_FAIL=0

for RESULT in "${TESTCASE_RESULTS[@]}"; do
    IFS='|' read -r NAME STATUS STAGE <<< "$RESULT"
    if [[ "$STATUS" == "PASS" ]]; then
        TESTCASE_PASS=$((TESTCASE_PASS + 1))
    else
        TESTCASE_FAIL=$((TESTCASE_FAIL + 1))
    fi
done

echo "Total Test Cases: $TOTAL_TESTCASES"
echo "Completed Full Pipeline: $STAGE5_TOTAL"
echo "Test Verification Pass: $TESTCASE_PASS"
echo "Test Verification Fail: $TESTCASE_FAIL"
echo ""

if [[ $STAGE5_TOTAL -gt 0 ]]; then
    TESTCASE_PASS_PCT=$((TESTCASE_PASS * 100 / STAGE5_TOTAL))
    echo "Test Pass Rate: ${TESTCASE_PASS_PCT}%"
    echo ""
fi

# Detailed results
if [[ $TESTCASE_FAIL -gt 0 ]]; then
    echo "Failed Test Cases:"
    for RESULT in "${TESTCASE_RESULTS[@]}"; do
        IFS='|' read -r NAME STATUS STAGE <<< "$RESULT"
        if [[ "$STATUS" == "FAIL" ]]; then
            echo "  - $NAME (failed at: $STAGE)"
        fi
    done
    echo ""
fi

if [[ $TESTCASE_PASS -gt 0 ]]; then
    echo "Passed Test Cases:"
    for RESULT in "${TESTCASE_RESULTS[@]}"; do
        IFS='|' read -r NAME STATUS STAGE <<< "$RESULT"
        if [[ "$STATUS" == "PASS" ]]; then
            echo "  - $NAME"
        fi
    done
    echo ""
fi

echo "=============================================="
echo "Output Locations"
echo "=============================================="
echo ""
echo "Temporary directory: $TEMP_DIR"
echo "  - logs/          : Stage execution logs"
echo "  - scripts/       : Generated test scripts"
echo "  - executions/    : Execution JSON outputs"
echo "  - verifications/ : Verification YAML outputs"
echo "  - results/       : Result JSON outputs"
echo ""

if [[ $REMOVE_TEMP -eq 0 ]]; then
    info "Temporary files preserved for inspection"
else
    info "Temporary files will be removed on exit"
fi

echo ""
echo "=============================================="
echo "Batch Pipeline Processing Complete"
echo "=============================================="

exit 0
