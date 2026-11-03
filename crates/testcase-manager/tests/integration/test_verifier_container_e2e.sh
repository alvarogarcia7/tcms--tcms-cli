#!/usr/bin/env bash
# End-to-end integration test for verifier container format generation
#
# This test validates:
# 1. Generate a test case using test-executor
# 2. Execute the generated script to produce an execution log
# 3. Invoke verifier with --config flag to generate container YAML report
# 4. Invoke verifier with individual CLI metadata flags to generate container YAML report
# 5. Validate container format structure includes all required fields:
#    - title
#    - project
#    - test_date
#    - test_results (array of TestCaseVerificationResult entries)
#    - metadata (with statistics and optional environment/platform/executor)
# 6. Verify metadata statistics are accurate (total/passed/failed counts)
# 7. Validate both YAML and JSON output formats
# 8. Test config file + CLI override precedence
#
# Usage: ./tests/integration/test_verifier_container_e2e.sh [--no-remove]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# logger.sh's fail() only prints. Count failures here so the script's exit status reflects them (TCMS-46 D-1).
FAILURE_COUNT=0
fail() {
    FAILURE_COUNT=$((FAILURE_COUNT + 1))
    echo -e "${RED}✗${NC} $1" >&2
}
finish_run() {
    if [[ $FAILURE_COUNT -gt 0 ]]; then
        echo -e "${RED}✗${NC} $FAILURE_COUNT check(s) failed" >&2
        exit 1
    fi
}

# Find binaries using workspace-aware search
cd "$PROJECT_ROOT"
TEST_EXECUTOR_BIN=$(find_binary "test-executor")
if [[ -z "$TEST_EXECUTOR_BIN" ]]; then
    echo "[ERROR] test-executor binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin test-executor" >&2
    exit 1
fi

VERIFIER_BIN=$(find_binary "verifier")
if [[ -z "$VERIFIER_BIN" ]]; then
    echo "[ERROR] verifier binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin verifier" >&2
    exit 1
fi

# Handle --no-remove flag
REMOVE_TEMP=1
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-remove)
            REMOVE_TEMP=0
            shift
            ;;
        *)
            shift
            ;;
    esac
done

echo "======================================"
echo "Verifier Container Format E2E Test"
echo "======================================"
echo ""

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$TEST_EXECUTOR_BIN" ]]; then
    fail "test-executor binary not found at $TEST_EXECUTOR_BIN"
    echo "Run 'cargo build' or 'make build' first"
    exit 1
fi
pass "test-executor binary found"

if [[ ! -f "$VERIFIER_BIN" ]]; then
    fail "verifier binary not found at $VERIFIER_BIN"
    echo "Run 'cargo build' or 'make build' first"
    exit 1
fi
pass "verifier binary found"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Test 1: Create a test case YAML
section "Test 1: Create Test Case YAML"

TEST_CASE_DIR="$TEMP_DIR/testcases"
mkdir -p "$TEST_CASE_DIR"

TEST_YAML="$TEST_CASE_DIR/TC_CONTAINER_001.yaml"
cat > "$TEST_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: REQ_CONTAINER_001
item: 1
tc: 1
id: TC_CONTAINER_001
description: Test case for container format validation

general_initial_conditions:
  System:
    - Ready

initial_conditions:
  Device:
    - Connected

test_sequences:
  - id: 1
    name: First Sequence
    description: First test sequence
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo hello
        command: echo 'hello world'
        expected:
          success: true
          result: "0"
          output: hello world
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "[[ \"$COMMAND_OUTPUT\" == \"hello world\" ]]"
      - step: 2
        description: True command
        command: "true"
        expected:
          success: true
          result: "0"
          output: ""
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "true"
  - id: 2
    name: Second Sequence
    description: Second test sequence
    initial_conditions:
      System:
        - Ready
    steps:
      - step: 1
        description: Echo test
        command: echo 'test'
        expected:
          success: true
          result: "0"
          output: test
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "[[ \"$COMMAND_OUTPUT\" == \"test\" ]]"
EOF

pass "Created test case YAML: TC_CONTAINER_001.yaml"

# Test 2: Generate executable script using test-executor
section "Test 2: Generate Executable Script"

GENERATED_SCRIPT="$TEMP_DIR/tc_container_001.sh"
if "$TEST_EXECUTOR_BIN" generate "$TEST_YAML" -o "$GENERATED_SCRIPT" > /dev/null 2>&1; then
    pass "Generated executable script"
else
    fail "Failed to generate executable script"
    exit 1
fi

if [[ -f "$GENERATED_SCRIPT" ]]; then
    pass "Script file created: $GENERATED_SCRIPT"
else
    fail "Script file not found"
    exit 1
fi

# Verify bash syntax
if bash -n "$GENERATED_SCRIPT" 2>/dev/null; then
    pass "Generated script has valid bash syntax"
else
    fail "Generated script has invalid bash syntax"
    exit 1
fi

# Test 3: Execute the generated script to produce execution log
section "Test 3: Execute Script to Generate Execution Log"

cd "$TEMP_DIR"
SCRIPT_OUTPUT="$TEMP_DIR/script_output.txt"
if bash "$GENERATED_SCRIPT" > "$SCRIPT_OUTPUT" 2>&1; then
    pass "Script executed successfully"
else
    fail "Script execution failed"
    info "Output: $(cat "$SCRIPT_OUTPUT")"
    exit 1
fi
cd "$PROJECT_ROOT"

# Verify execution log was created
EXECUTION_LOG="$TEMP_DIR/TC_CONTAINER_001_execution_log.json"
if [[ -f "$EXECUTION_LOG" ]]; then
    pass "Execution log created: $EXECUTION_LOG"
else
    fail "Execution log not found at $EXECUTION_LOG"
    exit 1
fi

# Validate JSON log format
if command -v jq > /dev/null 2>&1; then
    if jq . "$EXECUTION_LOG" > /dev/null 2>&1; then
        pass "Execution log is valid JSON"
    else
        fail "Execution log is not valid JSON"
        exit 1
    fi
else
    info "jq not available, skipping JSON validation"
fi

# Test 4: Create verifier config file
section "Test 4: Create Verifier Config File"

CONFIG_FILE="$TEMP_DIR/verifier-config.yaml"
cat > "$CONFIG_FILE" << 'EOF'
type: container_config
schema: tcms/container-config.schema.v1.json
title: "Container Format Test Results"
project: "Verifier Container E2E Tests"
environment: "Testing"
platform: "CI Environment"
executor: "Integration Test Suite"
EOF

pass "Created verifier config file"

# Test 5: Generate container YAML report using --config flag
section "Test 5: Generate Container YAML Report (Config File)"

CONTAINER_YAML_CONFIG="$TEMP_DIR/container_report_config.yaml"
if "$VERIFIER_BIN" \
    --log "$EXECUTION_LOG" \
    --test-case "TC_CONTAINER_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$CONTAINER_YAML_CONFIG" \
    --config "$CONFIG_FILE" > /dev/null 2>&1; then
    pass "Generated container YAML report using config file"
else
    fail "Failed to generate container YAML report"
    exit 1
fi

if [[ -f "$CONTAINER_YAML_CONFIG" ]]; then
    pass "Container YAML report file created"
else
    fail "Container YAML report file not found"
    exit 1
fi

# Test 6: Validate container YAML structure
section "Test 6: Validate Container YAML Structure (Config File)"

# Check required fields
if grep -q "^title:" "$CONTAINER_YAML_CONFIG"; then
    TITLE=$(grep "^title:" "$CONTAINER_YAML_CONFIG" | sed 's/^title: *//')
    pass "Container YAML has 'title' field: $TITLE"
else
    fail "Container YAML missing 'title' field"
    exit 1
fi

if grep -q "^project:" "$CONTAINER_YAML_CONFIG"; then
    PROJECT=$(grep "^project:" "$CONTAINER_YAML_CONFIG" | sed 's/^project: *//')
    pass "Container YAML has 'project' field: $PROJECT"
else
    fail "Container YAML missing 'project' field"
    exit 1
fi

if grep -q "^test_date:" "$CONTAINER_YAML_CONFIG"; then
    pass "Container YAML has 'test_date' field"
else
    fail "Container YAML missing 'test_date' field"
    exit 1
fi

if grep -q "^test_results:" "$CONTAINER_YAML_CONFIG"; then
    pass "Container YAML has 'test_results' field"
else
    fail "Container YAML missing 'test_results' field"
    exit 1
fi

if grep -q "^metadata:" "$CONTAINER_YAML_CONFIG"; then
    pass "Container YAML has 'metadata' field"
else
    fail "Container YAML missing 'metadata' field"
    exit 1
fi

# Verify test_results contains TestCaseVerificationResult entries
if grep -q "test_case_id: TC_CONTAINER_001" "$CONTAINER_YAML_CONFIG"; then
    pass "test_results contains TC_CONTAINER_001 entry"
else
    fail "test_results missing TC_CONTAINER_001 entry"
    exit 1
fi

if grep -q "overall_pass:" "$CONTAINER_YAML_CONFIG"; then
    pass "TestCaseVerificationResult has 'overall_pass' field"
else
    fail "TestCaseVerificationResult missing 'overall_pass' field"
    exit 1
fi

if grep -q "total_steps:" "$CONTAINER_YAML_CONFIG"; then
    pass "TestCaseVerificationResult has 'total_steps' field"
else
    fail "TestCaseVerificationResult missing 'total_steps' field"
    exit 1
fi

if grep -q "passed_steps:" "$CONTAINER_YAML_CONFIG"; then
    pass "TestCaseVerificationResult has 'passed_steps' field"
else
    fail "TestCaseVerificationResult missing 'passed_steps' field"
    exit 1
fi

if grep -q "failed_steps:" "$CONTAINER_YAML_CONFIG"; then
    pass "TestCaseVerificationResult has 'failed_steps' field"
else
    fail "TestCaseVerificationResult missing 'failed_steps' field"
    exit 1
fi

# Test 7: Validate metadata section
section "Test 7: Validate Metadata Section (Config File)"

# Check metadata fields
if grep -q "environment:" "$CONTAINER_YAML_CONFIG"; then
    ENVIRONMENT=$(grep "environment:" "$CONTAINER_YAML_CONFIG" | sed 's/.*environment: *//')
    pass "metadata has 'environment' field: $ENVIRONMENT"
else
    fail "metadata missing 'environment' field"
    exit 1
fi

if grep -q "platform:" "$CONTAINER_YAML_CONFIG"; then
    PLATFORM=$(grep "platform:" "$CONTAINER_YAML_CONFIG" | sed 's/.*platform: *//')
    pass "metadata has 'platform' field: $PLATFORM"
else
    fail "metadata missing 'platform' field"
    exit 1
fi

if grep -q "executor:" "$CONTAINER_YAML_CONFIG"; then
    EXECUTOR=$(grep "executor:" "$CONTAINER_YAML_CONFIG" | sed 's/.*executor: *//')
    pass "metadata has 'executor' field: $EXECUTOR"
else
    fail "metadata missing 'executor' field"
    exit 1
fi

if grep -q "execution_duration:" "$CONTAINER_YAML_CONFIG"; then
    pass "metadata has 'execution_duration' field"
else
    fail "metadata missing 'execution_duration' field"
    exit 1
fi

if grep -q "total_test_cases:" "$CONTAINER_YAML_CONFIG"; then
    pass "metadata has 'total_test_cases' field"
else
    fail "metadata missing 'total_test_cases' field"
    exit 1
fi

if grep -q "passed_test_cases:" "$CONTAINER_YAML_CONFIG"; then
    pass "metadata has 'passed_test_cases' field"
else
    fail "metadata missing 'passed_test_cases' field"
    exit 1
fi

if grep -q "failed_test_cases:" "$CONTAINER_YAML_CONFIG"; then
    pass "metadata has 'failed_test_cases' field"
else
    fail "metadata missing 'failed_test_cases' field"
    exit 1
fi

# Test 8: Verify metadata statistics accuracy
section "Test 8: Verify Metadata Statistics"

TOTAL_TEST_CASES=$(grep "total_test_cases:" "$CONTAINER_YAML_CONFIG" | awk '{print $2}')
PASSED_TEST_CASES=$(grep "passed_test_cases:" "$CONTAINER_YAML_CONFIG" | awk '{print $2}')
FAILED_TEST_CASES=$(grep "failed_test_cases:" "$CONTAINER_YAML_CONFIG" | awk '{print $2}')

if [[ "$TOTAL_TEST_CASES" == "1" ]]; then
    pass "total_test_cases is correct: 1"
else
    fail "total_test_cases is incorrect: expected 1, got $TOTAL_TEST_CASES"
    exit 1
fi

if [[ "$PASSED_TEST_CASES" == "1" ]]; then
    pass "passed_test_cases is correct: 1"
else
    fail "passed_test_cases is incorrect: expected 1, got $PASSED_TEST_CASES"
    exit 1
fi

if [[ "$FAILED_TEST_CASES" == "0" ]]; then
    pass "failed_test_cases is correct: 0"
else
    fail "failed_test_cases is incorrect: expected 0, got $FAILED_TEST_CASES"
    exit 1
fi

# Verify TestCaseVerificationResult statistics
TC_TOTAL_STEPS=$(grep "total_steps:" "$CONTAINER_YAML_CONFIG" | awk '{print $2}')
TC_PASSED_STEPS=$(grep "passed_steps:" "$CONTAINER_YAML_CONFIG" | awk '{print $2}')
TC_FAILED_STEPS=$(grep "failed_steps:" "$CONTAINER_YAML_CONFIG" | awk '{print $2}')

if [[ "$TC_TOTAL_STEPS" == "3" ]]; then
    pass "TestCaseVerificationResult total_steps is correct: 3"
else
    fail "TestCaseVerificationResult total_steps is incorrect: expected 3, got $TC_TOTAL_STEPS"
    exit 1
fi

if [[ "$TC_PASSED_STEPS" == "3" ]]; then
    pass "TestCaseVerificationResult passed_steps is correct: 3"
else
    fail "TestCaseVerificationResult passed_steps is incorrect: expected 3, got $TC_PASSED_STEPS"
    exit 1
fi

if [[ "$TC_FAILED_STEPS" == "0" ]]; then
    pass "TestCaseVerificationResult failed_steps is correct: 0"
else
    fail "TestCaseVerificationResult failed_steps is incorrect: expected 0, got $TC_FAILED_STEPS"
    exit 1
fi

# Test 9: Generate container YAML report using CLI flags only
section "Test 9: Generate Container YAML Report (CLI Flags)"

CONTAINER_YAML_CLI="$TEMP_DIR/container_report_cli.yaml"
if "$VERIFIER_BIN" \
    --log "$EXECUTION_LOG" \
    --test-case "TC_CONTAINER_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$CONTAINER_YAML_CLI" \
    --title "CLI Override Test" \
    --project "E2E Test with CLI Flags" \
    --environment "Development" \
    --platform "macOS ARM64" \
    --executor "Manual Test Runner" > /dev/null 2>&1; then
    pass "Generated container YAML report using CLI flags"
else
    fail "Failed to generate container YAML report with CLI flags"
    exit 1
fi

if [[ -f "$CONTAINER_YAML_CLI" ]]; then
    pass "Container YAML report file created (CLI flags)"
else
    fail "Container YAML report file not found (CLI flags)"
    exit 1
fi

# Test 10: Validate CLI flags override values
section "Test 10: Validate CLI Flags Override"

if grep -q "^title: CLI Override Test" "$CONTAINER_YAML_CLI"; then
    pass "CLI title override successful"
else
    fail "CLI title override failed"
    exit 1
fi

if grep -q "^project: E2E Test with CLI Flags" "$CONTAINER_YAML_CLI"; then
    pass "CLI project override successful"
else
    fail "CLI project override failed"
    exit 1
fi

if grep -q "environment: Development" "$CONTAINER_YAML_CLI"; then
    pass "CLI environment override successful"
else
    fail "CLI environment override failed"
    exit 1
fi

if grep -q "platform: macOS ARM64" "$CONTAINER_YAML_CLI"; then
    pass "CLI platform override successful"
else
    fail "CLI platform override failed"
    exit 1
fi

if grep -q "executor: Manual Test Runner" "$CONTAINER_YAML_CLI"; then
    pass "CLI executor override successful"
else
    fail "CLI executor override failed"
    exit 1
fi

# Test 11: Test config file + CLI override precedence
section "Test 11: Test Config + CLI Override Precedence"

CONTAINER_YAML_COMBINED="$TEMP_DIR/container_report_combined.yaml"
if "$VERIFIER_BIN" \
    --log "$EXECUTION_LOG" \
    --test-case "TC_CONTAINER_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$CONTAINER_YAML_COMBINED" \
    --config "$CONFIG_FILE" \
    --title "CLI Takes Precedence" \
    --executor "Overridden Executor" > /dev/null 2>&1; then
    pass "Generated container YAML report with config + CLI overrides"
else
    fail "Failed to generate container YAML report with combined sources"
    exit 1
fi

# Verify CLI flags override config file values
if grep -q "^title: CLI Takes Precedence" "$CONTAINER_YAML_COMBINED"; then
    pass "CLI title overrides config file"
else
    fail "CLI title did not override config file"
    exit 1
fi

if grep -q "^project: Verifier Container E2E Tests" "$CONTAINER_YAML_COMBINED"; then
    pass "Config file project retained (no CLI override)"
else
    fail "Config file project not retained"
    exit 1
fi

if grep -q "executor: Overridden Executor" "$CONTAINER_YAML_COMBINED"; then
    pass "CLI executor overrides config file"
else
    fail "CLI executor did not override config file"
    exit 1
fi

if grep -q "environment: Testing" "$CONTAINER_YAML_COMBINED"; then
    pass "Config file environment retained (no CLI override)"
else
    fail "Config file environment not retained"
    exit 1
fi

# Test 12: Generate container JSON report
section "Test 12: Generate Container JSON Report"

CONTAINER_JSON="$TEMP_DIR/container_report.json"
if "$VERIFIER_BIN" \
    --log "$EXECUTION_LOG" \
    --test-case "TC_CONTAINER_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format json \
    --output "$CONTAINER_JSON" \
    --config "$CONFIG_FILE" > /dev/null 2>&1; then
    pass "Generated container JSON report"
else
    fail "Failed to generate container JSON report"
    exit 1
fi

if [[ -f "$CONTAINER_JSON" ]]; then
    pass "Container JSON report file created"
else
    fail "Container JSON report file not found"
    exit 1
fi

# Test 13: Validate JSON structure
section "Test 13: Validate Container JSON Structure"

if command -v jq > /dev/null 2>&1; then
    # Validate JSON is well-formed
    if jq . "$CONTAINER_JSON" > /dev/null 2>&1; then
        pass "Container JSON is valid JSON"
    else
        fail "Container JSON is not valid JSON"
        exit 1
    fi
    
    # Validate required fields
    TITLE_JSON=$(jq -r '.title' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$TITLE_JSON" == "Container Format Test Results" ]]; then
        pass "JSON has correct 'title' field"
    else
        fail "JSON 'title' field incorrect: $TITLE_JSON"
        exit 1
    fi
    
    PROJECT_JSON=$(jq -r '.project' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$PROJECT_JSON" == "Verifier Container E2E Tests" ]]; then
        pass "JSON has correct 'project' field"
    else
        fail "JSON 'project' field incorrect: $PROJECT_JSON"
        exit 1
    fi
    
    if jq -e '.test_date' "$CONTAINER_JSON" > /dev/null 2>&1; then
        pass "JSON has 'test_date' field"
    else
        fail "JSON missing 'test_date' field"
        exit 1
    fi
    
    if jq -e '.test_results' "$CONTAINER_JSON" > /dev/null 2>&1; then
        pass "JSON has 'test_results' field"
    else
        fail "JSON missing 'test_results' field"
        exit 1
    fi
    
    if jq -e '.metadata' "$CONTAINER_JSON" > /dev/null 2>&1; then
        pass "JSON has 'metadata' field"
    else
        fail "JSON missing 'metadata' field"
        exit 1
    fi
    
    # Validate test_results array structure
    TC_ID_JSON=$(jq -r '.test_results[0].test_case_id' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$TC_ID_JSON" == "TC_CONTAINER_001" ]]; then
        pass "JSON test_results contains TC_CONTAINER_001"
    else
        fail "JSON test_results has incorrect test_case_id: $TC_ID_JSON"
        exit 1
    fi
    
    OVERALL_PASS_JSON=$(jq -r '.test_results[0].overall_pass' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$OVERALL_PASS_JSON" == "true" ]]; then
        pass "JSON test_results[0] has correct overall_pass"
    else
        fail "JSON test_results[0] overall_pass incorrect: $OVERALL_PASS_JSON"
        exit 1
    fi
    
    # Validate metadata structure
    ENV_JSON=$(jq -r '.metadata.environment' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$ENV_JSON" == "Testing" ]]; then
        pass "JSON metadata has correct 'environment'"
    else
        fail "JSON metadata 'environment' incorrect: $ENV_JSON"
        exit 1
    fi
    
    PLATFORM_JSON=$(jq -r '.metadata.platform' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$PLATFORM_JSON" == "CI Environment" ]]; then
        pass "JSON metadata has correct 'platform'"
    else
        fail "JSON metadata 'platform' incorrect: $PLATFORM_JSON"
        exit 1
    fi
    
    EXECUTOR_JSON=$(jq -r '.metadata.executor' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$EXECUTOR_JSON" == "Integration Test Suite" ]]; then
        pass "JSON metadata has correct 'executor'"
    else
        fail "JSON metadata 'executor' incorrect: $EXECUTOR_JSON"
        exit 1
    fi
    
    # Validate metadata statistics
    TOTAL_JSON=$(jq -r '.metadata.total_test_cases' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$TOTAL_JSON" == "1" ]]; then
        pass "JSON metadata total_test_cases is correct: 1"
    else
        fail "JSON metadata total_test_cases incorrect: $TOTAL_JSON"
        exit 1
    fi
    
    PASSED_JSON=$(jq -r '.metadata.passed_test_cases' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$PASSED_JSON" == "1" ]]; then
        pass "JSON metadata passed_test_cases is correct: 1"
    else
        fail "JSON metadata passed_test_cases incorrect: $PASSED_JSON"
        exit 1
    fi
    
    FAILED_JSON=$(jq -r '.metadata.failed_test_cases' "$CONTAINER_JSON" 2>/dev/null)
    if [[ "$FAILED_JSON" == "0" ]]; then
        pass "JSON metadata failed_test_cases is correct: 0"
    else
        fail "JSON metadata failed_test_cases incorrect: $FAILED_JSON"
        exit 1
    fi
    
    if jq -e '.metadata.execution_duration' "$CONTAINER_JSON" > /dev/null 2>&1; then
        pass "JSON metadata has 'execution_duration' field"
    else
        fail "JSON metadata missing 'execution_duration' field"
        exit 1
    fi
else
    info "jq not available, skipping detailed JSON validation"
fi

# Summary
section "Test Summary"
echo ""
echo "======================================"
echo "All container format tests completed"
echo "======================================"
echo ""

pass "All tests passed!"
echo ""
echo "✓ Test case generated with test-executor"
echo "✓ Script executed successfully to produce execution log"
echo "✓ Container YAML report generated with --config flag"
echo "✓ Container YAML report generated with CLI flags"
echo "✓ Container format structure validated (YAML)"
echo "✓ Container format structure validated (JSON)"
echo "✓ Metadata statistics verified for accuracy"
echo "✓ Config file + CLI override precedence tested"
echo "✓ All required fields present in container format"
echo ""
finish_run
exit 0
