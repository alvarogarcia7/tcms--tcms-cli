#!/usr/bin/env bash
# NOTE: This file must have executable permissions (chmod +x tests/integration/test_verifier_e2e.sh)
#
# End-to-end integration test for verifier binary
#
# This test validates:
# 1. Single-file mode: Verify a single log file against a test case
# 2. Folder discovery mode: Verify multiple log files in a directory
# 3. YAML output format validation against expected report
# 4. JSON output format validation against expected report
# 5. Exit code verification (0 for passing tests, 1 for failing tests)
# 6. Error handling and validation
#
# Usage: ./tests/integration/test_verifier_e2e.sh [--no-remove]
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

# Find verifier binary using workspace-aware search
cd "$PROJECT_ROOT"
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
echo "verifier End-to-End Integration Test"
echo "======================================"
echo ""

# Schema validation is performed internally by the verifier binary
# No external validation needed - if verifier succeeds, schema is valid

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$VERIFIER_BIN" ]]; then
    fail "verifier binary not found at $VERIFIER_BIN"
    echo "Run 'cargo build' or 'make build-verifier' first"
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

# Create test case directory structure
TEST_CASE_DIR="$TEMP_DIR/testcases"
mkdir -p "$TEST_CASE_DIR"

# Test 1: Create a passing test case and execution log
section "Test 1: Setup Passing Test Case"

PASSING_TEST_YAML="$TEST_CASE_DIR/TEST_PASSING_001.yml"
cat > "$PASSING_TEST_YAML" << 'EOF'
requirement: TEST_REQ_001
item: 1
tc: 1
id: TEST_PASSING_001
description: Test case with all passing verifications
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Passing Sequence
    description: All steps should pass
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo hello
        command: echo 'hello'
        expected:
          success: true
          result: "0"
          output: hello
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "[[ \"$COMMAND_OUTPUT\" == \"hello\" ]]"
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
    name: Second Passing Sequence
    description: Another sequence that passes
    initial_conditions:
      System:
        - Ready
    steps:
      - step: 1
        description: Echo world
        command: echo 'world'
        expected:
          success: true
          result: "0"
          output: world
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "[[ \"$COMMAND_OUTPUT\" == \"world\" ]]"
EOF

pass "Created passing test case YAML"

# Create corresponding execution log
PASSING_LOG="$TEMP_DIR/TEST_PASSING_001_execution_log.json"
cat > "$PASSING_LOG" << 'EOF'
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo 'hello'",
    "exit_code": 0,
    "output": "hello",
    "timestamp": "2026-02-02T10:00:00.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true
  },
  {
    "test_sequence": 1,
    "step": 2,
    "command": "true",
    "exit_code": 0,
    "output": "",
    "timestamp": "2026-02-02T10:00:01.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true
  },
  {
    "test_sequence": 2,
    "step": 1,
    "command": "echo 'world'",
    "exit_code": 0,
    "output": "world",
    "timestamp": "2026-02-02T10:00:02.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true
  }
]
EOF

pass "Created passing execution log"

# Test 2: Create a failing test case and execution log
section "Test 2: Setup Failing Test Case"

FAILING_TEST_YAML="$TEST_CASE_DIR/TEST_FAILING_002.yml"
cat > "$FAILING_TEST_YAML" << 'EOF'
requirement: TEST_REQ_002
item: 1
tc: 2
id: TEST_FAILING_002
description: Test case with some failing verifications
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Mixed Results Sequence
    description: Some steps pass, some fail
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo that passes
        command: echo 'pass'
        expected:
          success: true
          result: "0"
          output: pass
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "[[ \"$COMMAND_OUTPUT\" == \"pass\" ]]"
      - step: 2
        description: Command that should fail
        command: echo 'wrong'
        expected:
          success: true
          result: "0"
          output: expected
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "[[ \"$COMMAND_OUTPUT\" == \"expected\" ]]"
      - step: 3
        description: Exit code mismatch
        command: "false"
        expected:
          success: true
          result: "0"
          output: ""
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "true"
EOF

pass "Created failing test case YAML"

# Create corresponding execution log with mismatches
FAILING_LOG="$TEMP_DIR/TEST_FAILING_002_execution_log.json"
cat > "$FAILING_LOG" << 'EOF'
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo 'pass'",
    "exit_code": 0,
    "output": "pass",
    "timestamp": "2026-02-02T10:00:00.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true
  },
  {
    "test_sequence": 1,
    "step": 2,
    "command": "echo 'wrong'",
    "exit_code": 0,
    "output": "wrong",
    "timestamp": "2026-02-02T10:00:01.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": false
  },
  {
    "test_sequence": 1,
    "step": 3,
    "command": "false",
    "exit_code": 1,
    "output": "",
    "timestamp": "2026-02-02T10:00:02.000000+00:00",
    "result_verification_pass": false,
    "output_verification_pass": true
  }
]
EOF

pass "Created failing execution log"

# Test 3: Single-file mode with passing test (using default config)
section "Test 3: Single-File Mode - Passing Test (Default Config)"

# Create a test config file
TEST_CONFIG="$TEMP_DIR/test_config.yml"
cat > "$TEST_CONFIG" << 'EOF'
title: "E2E Integration Test Results"
project: "Verifier E2E Tests"
environment: "Testing"
platform: "CI Environment"
executor: "Integration Test Suite"
EOF

SINGLE_YAML_OUTPUT="$TEMP_DIR/single_pass_report.yaml"
SINGLE_YAML_ERROR="$TEMP_DIR/single_pass_error.log"
if "$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$SINGLE_YAML_OUTPUT" \
    --config "$TEST_CONFIG" 2> "$SINGLE_YAML_ERROR"; then
    SINGLE_PASS_EXIT=$?
    pass "Single-file mode completed successfully with passing test"
else
    SINGLE_PASS_EXIT=$?
    fail "Single-file mode failed with passing test (exit code: $SINGLE_PASS_EXIT)"
    if [[ -f "$SINGLE_YAML_ERROR" ]]; then
        echo "Error output:" >&2
        cat "$SINGLE_YAML_ERROR" >&2
    fi
fi

if [[ $SINGLE_PASS_EXIT -eq 0 ]]; then
    pass "Exit code is 0 for passing test"
else
    fail "Exit code should be 0 for passing test, got $SINGLE_PASS_EXIT"
fi

if [[ -f "$SINGLE_YAML_OUTPUT" ]]; then
    pass "YAML report file created"
else
    fail "YAML report file not created"
fi

# Validate YAML output structure (container format)
if [[ -f "$SINGLE_YAML_OUTPUT" ]]; then
    if grep -q "test_results:" "$SINGLE_YAML_OUTPUT"; then
        pass "YAML report contains test_results field"
    else
        fail "YAML report missing test_results field"
    fi
    
    if grep -q "test_case_id: TEST_PASSING_001" "$SINGLE_YAML_OUTPUT"; then
        pass "YAML report contains correct test case ID"
    else
        fail "YAML report missing correct test case ID"
    fi
    
    if grep -q "overall_pass: true" "$SINGLE_YAML_OUTPUT"; then
        pass "YAML report shows overall pass"
    else
        fail "YAML report should show overall pass"
    fi
fi

# Test 4: Single-file mode with failing test (using CLI flags)
section "Test 4: Single-File Mode - Failing Test (CLI Flags)"

SINGLE_YAML_FAIL_OUTPUT="$TEMP_DIR/single_fail_report.yaml"
if "$VERIFIER_BIN" \
    --log "$FAILING_LOG" \
    --test-case "TEST_FAILING_002" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$SINGLE_YAML_FAIL_OUTPUT" \
    --title "CLI Override Test" \
    --project "E2E Test with CLI Flags" \
    --environment "Development" > /dev/null 2>&1; then
    SINGLE_FAIL_EXIT=$?
    fail "Single-file mode should fail with failing test (got exit code 0)"
else
    SINGLE_FAIL_EXIT=$?
    pass "Single-file mode returned non-zero exit code: $SINGLE_FAIL_EXIT"
fi

if [[ $SINGLE_FAIL_EXIT -ne 0 ]]; then
    pass "Exit code is non-zero for failing test"
else
    fail "Exit code should be non-zero for failing test"
fi

if [[ -f "$SINGLE_YAML_FAIL_OUTPUT" ]]; then
    pass "YAML report file created for failing test"
else
    fail "YAML report file not created for failing test"
fi

# Validate failure reporting in YAML (container format)
if [[ -f "$SINGLE_YAML_FAIL_OUTPUT" ]]; then
    if grep -q "overall_pass: false" "$SINGLE_YAML_FAIL_OUTPUT"; then
        pass "YAML report shows overall failure"
    else
        fail "YAML report should show overall failure"
    fi
    
    if grep -q "failed_test_cases:" "$SINGLE_YAML_FAIL_OUTPUT"; then
        pass "YAML report contains failed_test_cases count in metadata"
    else
        fail "YAML report missing failed_test_cases count in metadata"
    fi
fi

# Test 5: JSON output format (combining config file and CLI overrides)
section "Test 5: JSON Output Format (Config + CLI Overrides)"

SINGLE_JSON_OUTPUT="$TEMP_DIR/single_pass_report.json"
if "$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format json \
    --output "$SINGLE_JSON_OUTPUT" \
    --config "$TEST_CONFIG" \
    --executor "E2E Test Runner v1.0" > /dev/null 2>&1; then
    pass "JSON format output generated successfully"
else
    fail "Failed to generate JSON format output"
fi

if [[ -f "$SINGLE_JSON_OUTPUT" ]]; then
    pass "JSON report file created"
    
    # Validate JSON structure
    if command -v jq > /dev/null 2>&1; then
        if jq . "$SINGLE_JSON_OUTPUT" > /dev/null 2>&1; then
            pass "JSON report is valid JSON"
        else
            fail "JSON report is not valid JSON"
        fi
        
        # Check JSON fields (container format)
        TEST_CASE_ID=$(jq -r '.test_results[0].test_case_id' "$SINGLE_JSON_OUTPUT" 2>/dev/null)
        if [[ "$TEST_CASE_ID" == "TEST_PASSING_001" ]]; then
            pass "JSON report contains correct test case ID"
        else
            fail "JSON report has incorrect test case ID: $TEST_CASE_ID"
        fi
        
        OVERALL_PASS=$(jq -r '.test_results[0].overall_pass' "$SINGLE_JSON_OUTPUT" 2>/dev/null)
        if [[ "$OVERALL_PASS" == "true" ]]; then
            pass "JSON report shows overall pass"
        else
            fail "JSON report should show overall pass, got: $OVERALL_PASS"
        fi
    else
        info "jq not available, skipping JSON validation"
    fi
else
    fail "JSON report file not created"
fi

# Test 6: Folder discovery mode
section "Test 6: Folder Discovery Mode"

# Create a logs directory with multiple execution logs
LOGS_DIR="$TEMP_DIR/logs"
mkdir -p "$LOGS_DIR"

# Copy logs to the folder
cp "$PASSING_LOG" "$LOGS_DIR/"
cp "$FAILING_LOG" "$LOGS_DIR/"

FOLDER_YAML_OUTPUT="$TEMP_DIR/folder_report.yaml"
if "$VERIFIER_BIN" \
    --folder "$LOGS_DIR" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$FOLDER_YAML_OUTPUT" \
    --config "$TEST_CONFIG" > /dev/null 2>&1; then
    FOLDER_EXIT=$?
    fail "Folder mode should fail when any test fails (got exit code 0)"
else
    FOLDER_EXIT=$?
    pass "Folder mode returned non-zero exit code: $FOLDER_EXIT"
fi

if [[ -f "$FOLDER_YAML_OUTPUT" ]]; then
    pass "Folder discovery report created"
    
    # Verify report contains both test cases
    PASSING_COUNT=$(grep -c "test_case_id: TEST_PASSING_001" "$FOLDER_YAML_OUTPUT" || true)
    FAILING_COUNT=$(grep -c "test_case_id: TEST_FAILING_002" "$FOLDER_YAML_OUTPUT" || true)
    
    if [[ $PASSING_COUNT -eq 1 ]] && [[ $FAILING_COUNT -eq 1 ]]; then
        pass "Folder report contains both test cases"
    else
        fail "Folder report should contain both test cases (passing: $PASSING_COUNT, failing: $FAILING_COUNT)"
    fi
    
    # Check summary counts
    if grep -q "total_test_cases: 2" "$FOLDER_YAML_OUTPUT"; then
        pass "Folder report shows correct total test cases"
    else
        fail "Folder report has incorrect total test cases count"
    fi
    
    if grep -q "passed_test_cases: 1" "$FOLDER_YAML_OUTPUT"; then
        pass "Folder report shows correct passed test cases count"
    else
        fail "Folder report has incorrect passed test cases count"
    fi
    
    if grep -q "failed_test_cases: 1" "$FOLDER_YAML_OUTPUT"; then
        pass "Folder report shows correct failed test cases count"
    else
        fail "Folder report has incorrect failed test cases count"
    fi
else
    fail "Folder discovery report not created"
fi

# Test 7: Folder discovery with JSON format
section "Test 7: Folder Discovery Mode - JSON Format"

FOLDER_JSON_OUTPUT="$TEMP_DIR/folder_report.json"
if "$VERIFIER_BIN" \
    --folder "$LOGS_DIR" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format json \
    --output "$FOLDER_JSON_OUTPUT" \
    --config "$TEST_CONFIG" > /dev/null 2>&1; then
    fail "Folder mode JSON should fail when any test fails"
else
    pass "Folder mode JSON returned non-zero exit code"
fi

if [[ -f "$FOLDER_JSON_OUTPUT" ]]; then
    pass "Folder discovery JSON report created"
    
    if command -v jq > /dev/null 2>&1; then
        if jq . "$FOLDER_JSON_OUTPUT" > /dev/null 2>&1; then
            pass "Folder JSON report is valid JSON"
        else
            fail "Folder JSON report is not valid JSON"
        fi
        
        TOTAL_CASES=$(jq -r '.metadata.total_test_cases' "$FOLDER_JSON_OUTPUT" 2>/dev/null)
        if [[ "$TOTAL_CASES" == "2" ]]; then
            pass "Folder JSON shows correct total test cases"
        else
            fail "Folder JSON has incorrect total: $TOTAL_CASES"
        fi
        
        PASSED_CASES=$(jq -r '.metadata.passed_test_cases' "$FOLDER_JSON_OUTPUT" 2>/dev/null)
        if [[ "$PASSED_CASES" == "1" ]]; then
            pass "Folder JSON shows correct passed test cases"
        else
            fail "Folder JSON has incorrect passed count: $PASSED_CASES"
        fi
        
        FAILED_CASES=$(jq -r '.metadata.failed_test_cases' "$FOLDER_JSON_OUTPUT" 2>/dev/null)
        if [[ "$FAILED_CASES" == "1" ]]; then
            pass "Folder JSON shows correct failed test cases"
        else
            fail "Folder JSON has incorrect failed count: $FAILED_CASES"
        fi
    fi
else
    fail "Folder discovery JSON report not created"
fi

# Test 8: Error handling - missing log file
section "Test 8: Error Handling - Missing Log File"

if "$VERIFIER_BIN" \
    --log "/nonexistent/log.json" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml > /dev/null 2>&1; then
    fail "Verifier should fail with nonexistent log file"
else
    pass "Verifier correctly failed with nonexistent log file"
fi

# Test 9: Error handling - missing test case directory
section "Test 9: Error Handling - Missing Test Case Directory"

if "$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "/nonexistent/testcases" \
    --format yaml > /dev/null 2>&1; then
    fail "Verifier should fail with nonexistent test case directory"
else
    pass "Verifier correctly failed with nonexistent test case directory"
fi

# Test 10: Error handling - invalid format
section "Test 10: Error Handling - Invalid Format"

if "$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format xml > /dev/null 2>&1; then
    fail "Verifier should fail with invalid format"
else
    pass "Verifier correctly failed with invalid format"
fi

# Test 11: Verify expected report file structure (YAML)
section "Test 11: Expected Report File Validation - YAML"

# Create expected report template (container format)
EXPECTED_YAML="$TEMP_DIR/expected_passing_report.yaml"
cat > "$EXPECTED_YAML" << 'EOF'
title: Test
project: Test
metadata:
  total_test_cases: 1
  passed_test_cases: 1
  failed_test_cases: 0
test_results:
  - test_case_id: TEST_PASSING_001
    overall_pass: true
    passed_steps: 3
    failed_steps: 0
    total_steps: 3
EOF

pass "Created expected YAML report template"

# Generate actual report
ACTUAL_YAML="$TEMP_DIR/actual_passing_report.yaml"
"$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml \
    --output "$ACTUAL_YAML" \
    --title "Test" \
    --project "Test" > /dev/null 2>&1

# Validate key fields match (container format)
if [[ -f "$ACTUAL_YAML" ]]; then
    ACTUAL_TOTAL=$(grep "total_test_cases:" "$ACTUAL_YAML" | awk '{print $2}')
    EXPECTED_TOTAL=$(grep "total_test_cases:" "$EXPECTED_YAML" | awk '{print $2}')
    
    if [[ "$ACTUAL_TOTAL" == "$EXPECTED_TOTAL" ]]; then
        pass "Total test cases matches expected ($EXPECTED_TOTAL)"
    else
        fail "Total test cases mismatch: expected $EXPECTED_TOTAL, got $ACTUAL_TOTAL"
    fi
    
    ACTUAL_PASSED=$(grep "passed_test_cases:" "$ACTUAL_YAML" | awk '{print $2}')
    EXPECTED_PASSED=$(grep "passed_test_cases:" "$EXPECTED_YAML" | awk '{print $2}')
    
    if [[ "$ACTUAL_PASSED" == "$EXPECTED_PASSED" ]]; then
        pass "Passed test cases matches expected ($EXPECTED_PASSED)"
    else
        fail "Passed test cases mismatch: expected $EXPECTED_PASSED, got $ACTUAL_PASSED"
    fi
    
    ACTUAL_FAILED=$(grep "^failed_test_cases:" "$ACTUAL_YAML" | awk '{print $2}')
    EXPECTED_FAILED=$(grep "^failed_test_cases:" "$EXPECTED_YAML" | awk '{print $2}')
    
    if [[ "$ACTUAL_FAILED" == "$EXPECTED_FAILED" ]]; then
        pass "Failed test cases matches expected ($EXPECTED_FAILED)"
    else
        fail "Failed test cases mismatch: expected $EXPECTED_FAILED, got $ACTUAL_FAILED"
    fi
fi

# Test 12: Verify expected report file structure (JSON)
section "Test 12: Expected Report File Validation - JSON"

# Create expected JSON report template (container format)
EXPECTED_JSON="$TEMP_DIR/expected_passing_report.json"
cat > "$EXPECTED_JSON" << 'EOF'
{
  "metadata": {
    "total_test_cases": 1,
    "passed_test_cases": 1,
    "failed_test_cases": 0
  },
  "test_results": [
    {
      "test_case_id": "TEST_PASSING_001",
      "overall_pass": true,
      "passed_steps": 3,
      "failed_steps": 0,
      "total_steps": 3
    }
  ]
}
EOF

pass "Created expected JSON report template"

# Generate actual report
ACTUAL_JSON="$TEMP_DIR/actual_passing_report.json"
"$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format json \
    --output "$ACTUAL_JSON" > /dev/null 2>&1

# Validate JSON fields if jq is available
if command -v jq > /dev/null 2>&1 && [[ -f "$ACTUAL_JSON" ]]; then
    ACTUAL_TOTAL_JSON=$(jq -r '.metadata.total_test_cases' "$ACTUAL_JSON" 2>/dev/null)
    EXPECTED_TOTAL_JSON=$(jq -r '.metadata.total_test_cases' "$EXPECTED_JSON" 2>/dev/null)
    
    if [[ "$ACTUAL_TOTAL_JSON" == "$EXPECTED_TOTAL_JSON" ]]; then
        pass "JSON total test cases matches expected ($EXPECTED_TOTAL_JSON)"
    else
        fail "JSON total test cases mismatch: expected $EXPECTED_TOTAL_JSON, got $ACTUAL_TOTAL_JSON"
    fi
    
    ACTUAL_PASSED_JSON=$(jq -r '.metadata.passed_test_cases' "$ACTUAL_JSON" 2>/dev/null)
    EXPECTED_PASSED_JSON=$(jq -r '.metadata.passed_test_cases' "$EXPECTED_JSON" 2>/dev/null)
    
    if [[ "$ACTUAL_PASSED_JSON" == "$EXPECTED_PASSED_JSON" ]]; then
        pass "JSON passed test cases matches expected ($EXPECTED_PASSED_JSON)"
    else
        fail "JSON passed test cases mismatch: expected $EXPECTED_PASSED_JSON, got $ACTUAL_PASSED_JSON"
    fi
    
    ACTUAL_FAILED_JSON=$(jq -r '.metadata.failed_test_cases' "$ACTUAL_JSON" 2>/dev/null)
    EXPECTED_FAILED_JSON=$(jq -r '.metadata.failed_test_cases' "$EXPECTED_JSON" 2>/dev/null)
    
    if [[ "$ACTUAL_FAILED_JSON" == "$EXPECTED_FAILED_JSON" ]]; then
        pass "JSON failed test cases matches expected ($EXPECTED_FAILED_JSON)"
    else
        fail "JSON failed test cases mismatch: expected $EXPECTED_FAILED_JSON, got $ACTUAL_FAILED_JSON"
    fi
    
    # Validate nested test case data
    ACTUAL_TC_ID=$(jq -r '.test_results[0].test_case_id' "$ACTUAL_JSON" 2>/dev/null)
    EXPECTED_TC_ID=$(jq -r '.test_results[0].test_case_id' "$EXPECTED_JSON" 2>/dev/null)
    
    if [[ "$ACTUAL_TC_ID" == "$EXPECTED_TC_ID" ]]; then
        pass "JSON test case ID matches expected ($EXPECTED_TC_ID)"
    else
        fail "JSON test case ID mismatch: expected $EXPECTED_TC_ID, got $ACTUAL_TC_ID"
    fi
    
    ACTUAL_TC_PASS=$(jq -r '.test_results[0].overall_pass' "$ACTUAL_JSON" 2>/dev/null)
    EXPECTED_TC_PASS=$(jq -r '.test_results[0].overall_pass' "$EXPECTED_JSON" 2>/dev/null)
    
    if [[ "$ACTUAL_TC_PASS" == "$EXPECTED_TC_PASS" ]]; then
        pass "JSON test case overall_pass matches expected ($EXPECTED_TC_PASS)"
    else
        fail "JSON test case overall_pass mismatch: expected $EXPECTED_TC_PASS, got $ACTUAL_TC_PASS"
    fi
fi

# Test 13: Stdout output (no output file specified)
section "Test 13: Stdout Output"

STDOUT_OUTPUT="$TEMP_DIR/stdout_capture.txt"
if "$VERIFIER_BIN" \
    --log "$PASSING_LOG" \
    --test-case "TEST_PASSING_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format yaml > "$STDOUT_OUTPUT" 2>&1; then
    pass "Verifier wrote to stdout successfully"
else
    fail "Verifier failed when writing to stdout"
fi

if [[ -f "$STDOUT_OUTPUT" ]] && [[ -s "$STDOUT_OUTPUT" ]]; then
    pass "Stdout contains output"
    
    if grep -q "test_case_id: TEST_PASSING_001" "$STDOUT_OUTPUT"; then
        pass "Stdout output contains test case data"
    else
        fail "Stdout output missing test case data"
    fi
else
    fail "Stdout output is empty"
fi

# Summary
section "Test Summary"
echo ""
echo "======================================"
echo "All verifier integration tests completed"
echo "======================================"
echo ""

pass "All tests passed!"
finish_run
exit 0
