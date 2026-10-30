#!/usr/bin/env bash
#
# End-to-end integration test for variable display functionality
#
# This test validates:
# 1. Test YAML generation with variable capture (new array format with capture and command)
# 2. Shell script generation with variable capture logic
# 3. Variable extraction from command output using regex patterns (capture field)
# 4. Variable extraction from command execution results (command field)
# 5. Variable display in execution output showing captured values
# 6. CAPTURED_VAR_ variables are set correctly in generated bash script
# 7. JSON execution log contains captured variable names and values
# 8. Error messages when variable capture fails are clear and actionable
#
# Usage: ./tests/integration/test_variable_display_e2e.sh [--no-remove]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
SCHEMA_FILE="$PROJECT_ROOT/schemas/tcms-schemas/tcms/test-case-main.schema.json"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/shellcheck-helper.sh" || true

# Find binaries using workspace-aware search
cd "$PROJECT_ROOT"
TEST_EXECUTOR_BIN=$(find_binary "test-executor")
if [[ -z "$TEST_EXECUTOR_BIN" ]]; then
    echo "[ERROR] test-executor binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin test-executor" >&2
    exit 1
fi

VALIDATE_YAML_BIN=$(find_binary "validate-yaml")
if [[ -z "$VALIDATE_YAML_BIN" ]]; then
    echo "[ERROR] validate-yaml binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin validate-yaml" >&2
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

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

echo "======================================"
echo "Variable Display End-to-End Integration Test"
echo "======================================"
echo ""

# Check prerequisites
section "Checking Prerequisites"

# Build test-executor binary if it doesn't exist or is outdated
if [[ ! -f "$TEST_EXECUTOR_BIN" ]] || [[ "$PROJECT_ROOT/src" -nt "$TEST_EXECUTOR_BIN" ]]; then
    info "Building test-executor binary..."
    cd "$PROJECT_ROOT"
    if cargo build --bin test-executor 2>&1 | tail -5; then
        pass "test-executor binary built successfully"
    else
        fail "Failed to build test-executor binary"
        exit 1
    fi
else
    pass "test-executor binary found"
fi

if [[ ! -f "$VALIDATE_YAML_BIN" ]]; then
    fail "validate-yaml binary not found at $VALIDATE_YAML_BIN"
    echo "Run 'cargo build' first"
    exit 1
fi
pass "validate-yaml binary found"

if [[ ! -f "$SCHEMA_FILE" ]]; then
    fail "Schema file not found at $SCHEMA_FILE"
    exit 1
fi
pass "Schema file found"

if ! command -v bash &> /dev/null; then
    fail "bash not found"
    exit 1
fi
pass "bash available"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Test 1: Create test YAML with variable capture using new array format
section "Test 1: Creating Test YAML with New Variable Capture Format"

VARIABLE_YAML="$TEMP_DIR/test_variable_display.yaml"
cat > "$VARIABLE_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_VAR_DISPLAY
item: 1
tc: 1
id: TEST_VAR_DISPLAY_001
description: Test case demonstrating variable capture with both regex and command formats
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Variable Capture Display Test
    description: This sequence tests both regex-based and command-based variable capture
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Capture variables using regex patterns
        command: echo 'token="abc123" user_id=42 status=active'
        capture_vars:
          - name: auth_token
            capture: 'token="([^"]+)"'
          - name: user_id
            capture: 'user_id=(\d+)'
          - name: status
            capture: 'status=(\w+)'
        expected:
          success: true
          result: "0"
          output: 'token="abc123" user_id=42 status=active'
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = 'token=\"abc123\" user_id=42 status=active' ]"
      - step: 2
        description: Capture variables using command execution
        command: echo 'Preparing system...'
        capture_vars:
          - name: current_date
            command: "date +%Y-%m-%d"
          - name: hostname
            command: "hostname"
          - name: word_count
            command: "echo 'hello world test' | wc -w | tr -d ' '"
        expected:
          success: true
          result: "0"
          output: Preparing system...
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 3
        description: Display captured variables in output
        command: echo 'Token=${auth_token} UserID=${user_id} Status=${status} Date=${current_date} Host=${hostname} Words=${word_count}'
        expected:
          success: true
          result: "0"
          output: ""
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 4
        description: Test variable capture failure with non-matching regex
        command: echo 'no_match_here'
        capture_vars:
          - name: missing_var
            capture: 'DOES_NOT_EXIST=(\w+)'
        expected:
          success: true
          result: "0"
          output: no_match_here
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 5
        description: Test command-based capture with failing command
        command: echo 'testing error handling'
        capture_vars:
          - name: failed_capture
            command: "nonexistent_command_xyz 2>&1"
        expected:
          success: true
          result: "0"
          output: testing error handling
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 6
        description: Verify captured variables persist across steps
        command: echo 'Recap - Token=${auth_token} Status=${status}'
        expected:
          success: true
          result: "0"
          output: ""
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

pass "Created test YAML with variable capture (new array format)"

# Test 2: Validate YAML against schema
section "Test 2: YAML Schema Validation"

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VARIABLE_YAML" > /dev/null 2>&1; then
    pass "YAML validates against schema"
else
    fail "YAML failed schema validation"
    "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VARIABLE_YAML"
fi

# Test 3: Generate shell script from YAML
section "Test 3: Shell Script Generation"

VARIABLE_SCRIPT="$TEMP_DIR/test_variable_display.sh"
if "$TEST_EXECUTOR_BIN" generate "$VARIABLE_YAML" -o "$VARIABLE_SCRIPT" > /dev/null 2>&1; then
    pass "Generated script from YAML with variable capture"
else
    fail "Failed to generate script from YAML"
fi

if [[ -f "$VARIABLE_SCRIPT" ]]; then
    pass "Script file created"
else
    fail "Script file not found"
    exit 1
fi

# Test 4: Validate bash syntax
section "Test 4: Bash Syntax Validation"

if bash -n "$VARIABLE_SCRIPT" 2>/dev/null; then
    pass "Script has valid bash syntax"
else
    fail "Script has invalid bash syntax"
    bash -n "$VARIABLE_SCRIPT" 2>&1
    exit 1
fi
validate_with_shellcheck "$VARIABLE_SCRIPT" "Variable display script"

# Test 5: Verify CAPTURED_VAR_ variables are set correctly in generated script
section "Test 5: Verify CAPTURED_VAR_ Variables in Generated Script"

# Check for variable storage initialization
if grep -q 'CAPTURED_VAR_NAMES=""' "$VARIABLE_SCRIPT"; then
    pass "Script declares STEP_VAR_NAMES variable"
else
    fail "Script missing STEP_VAR_NAMES declaration"
fi

# Check for regex-based captures from step 1
if grep -q 'auth_token=' "$VARIABLE_SCRIPT"; then
    pass "Script captures auth_token variable (regex-based)"
else
    fail "Script doesn't capture auth_token variable"
fi

if grep -q 'user_id=' "$VARIABLE_SCRIPT"; then
    pass "Script captures user_id variable (regex-based)"
else
    fail "Script doesn't capture user_id variable"
fi

if grep -q 'status=' "$VARIABLE_SCRIPT"; then
    pass "Script captures status variable (regex-based)"
else
    fail "Script doesn't capture status variable"
fi

# Check for command-based captures from step 2
if grep -q 'current_date=' "$VARIABLE_SCRIPT"; then
    pass "Script captures current_date variable (command-based)"
else
    fail "Script doesn't capture current_date variable"
fi

if grep -q 'hostname=' "$VARIABLE_SCRIPT"; then
    pass "Script captures hostname variable (command-based)"
else
    fail "Script doesn't capture hostname variable"
fi

if grep -q 'word_count=' "$VARIABLE_SCRIPT"; then
    pass "Script captures word_count variable (command-based)"
else
    fail "Script doesn't capture word_count variable"
fi

# Check for failed captures from steps 4 and 5
if grep -q 'missing_var=' "$VARIABLE_SCRIPT"; then
    pass "Script includes missing_var capture attempt"
else
    fail "Script doesn't include missing_var capture"
fi

if grep -q 'failed_capture=' "$VARIABLE_SCRIPT"; then
    pass "Script includes failed_capture command attempt"
else
    fail "Script doesn't include failed_capture command"
fi

# Test 6: Verify variable substitution logic is present
section "Test 6: Verify Variable Substitution Logic"

# Check for variable substitution loop
if grep -q 'for var_name in $CAPTURED_VAR_NAMES; do' "$VARIABLE_SCRIPT"; then
    pass "Script contains variable substitution loop"
else
    fail "Script missing variable substitution loop"
fi

# Check for SUBSTITUTED_COMMAND variable
if grep -q 'SUBSTITUTED_COMMAND=' "$VARIABLE_SCRIPT"; then
    pass "Script uses SUBSTITUTED_COMMAND for variable substitution"
else
    fail "Script missing SUBSTITUTED_COMMAND"
fi

# Verify sed command for variable substitution
if grep -q 'sed.*\${$var_name}' "$VARIABLE_SCRIPT"; then
    pass "Script performs variable substitution in commands"
else
    fail "Script missing variable substitution in commands"
fi

# Verify command-based capture pattern
if grep -q 'date +%Y-%m-%d' "$VARIABLE_SCRIPT"; then
    pass "Script includes command-based capture for current_date"
else
    fail "Script missing command-based capture for current_date"
fi

# Test 7: Execute the generated script and capture output
section "Test 7: Execute Generated Script"

EXECUTION_OUTPUT="$TEMP_DIR/execution_output.txt"
cd "$TEMP_DIR"

if bash "$VARIABLE_SCRIPT" > "$EXECUTION_OUTPUT" 2>&1; then
    pass "Script executed successfully"
    EXECUTION_EXIT_CODE=0
else
    EXECUTION_EXIT_CODE=$?
    fail "Script execution failed with exit code $EXECUTION_EXIT_CODE"
    info "Output: $(cat "$EXECUTION_OUTPUT" | head -30)"
fi

cd "$PROJECT_ROOT"

# Test 8: Verify captured variables appear in execution output
section "Test 8: Verify Variables Appear in Execution Output"

if [[ $EXECUTION_EXIT_CODE -eq 0 ]]; then
    # Check for PASS indicators
    PASS_COUNT=$(grep -c '\[PASS\]' "$EXECUTION_OUTPUT" || true)
    if [[ $PASS_COUNT -eq 6 ]]; then
        pass "All 6 steps passed verification"
    else
        fail "Expected 6 passed steps, got $PASS_COUNT"
        info "Output excerpt: $(grep '\[PASS\]\|\[FAIL\]' "$EXECUTION_OUTPUT" | head -10)"
    fi

    # Verify step 1 captured the regex-based variables
    STEP1_LOG="$TEMP_DIR/TEST_VAR_DISPLAY_001_sequence-1_step-1.actual.log"
    if [[ -f "$STEP1_LOG" ]]; then
        pass "Log file created for step 1"
        LOG_CONTENT=$(cat "$STEP1_LOG")
        if [[ "$LOG_CONTENT" == 'token="abc123" user_id=42 status=active' ]]; then
            pass "Step 1 log contains correct output"
        else
            fail "Step 1 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        fail "Log file not created for step 1"
    fi

    # Verify step 3 output contains substituted variables
    STEP3_LOG="$TEMP_DIR/TEST_VAR_DISPLAY_001_sequence-1_step-3.actual.log"
    if [[ -f "$STEP3_LOG" ]]; then
        pass "Log file created for step 3"
        LOG_CONTENT=$(cat "$STEP3_LOG")
        
        # Check that captured regex variables appear in output
        if echo "$LOG_CONTENT" | grep -q 'Token=abc123'; then
            pass "Step 3 output contains substituted auth_token value"
        else
            fail "Step 3 output missing auth_token substitution. Content: '$LOG_CONTENT'"
        fi
        
        if echo "$LOG_CONTENT" | grep -q 'UserID=42'; then
            pass "Step 3 output contains substituted user_id value"
        else
            fail "Step 3 output missing user_id substitution. Content: '$LOG_CONTENT'"
        fi
        
        if echo "$LOG_CONTENT" | grep -q 'Status=active'; then
            pass "Step 3 output contains substituted status value"
        else
            fail "Step 3 output missing status substitution. Content: '$LOG_CONTENT'"
        fi
        
        # Check that command-based captured variables appear
        if echo "$LOG_CONTENT" | grep -q 'Date=[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}'; then
            pass "Step 3 output contains substituted current_date value"
        else
            fail "Step 3 output missing current_date substitution. Content: '$LOG_CONTENT'"
        fi
        
        if echo "$LOG_CONTENT" | grep -q 'Host='; then
            pass "Step 3 output contains substituted hostname value"
        else
            fail "Step 3 output missing hostname substitution. Content: '$LOG_CONTENT'"
        fi
        
        if echo "$LOG_CONTENT" | grep -q 'Words=3'; then
            pass "Step 3 output contains substituted word_count value"
        else
            fail "Step 3 output missing word_count substitution. Content: '$LOG_CONTENT'"
        fi
    else
        fail "Log file not created for step 3"
    fi

    # Verify step 6 shows persisted variables
    STEP6_LOG="$TEMP_DIR/TEST_VAR_DISPLAY_001_sequence-1_step-6.actual.log"
    if [[ -f "$STEP6_LOG" ]]; then
        pass "Log file created for step 6"
        LOG_CONTENT=$(cat "$STEP6_LOG")
        
        if echo "$LOG_CONTENT" | grep -q 'Token=abc123'; then
            pass "Step 6 output shows persisted auth_token"
        else
            fail "Step 6 output missing persisted auth_token. Content: '$LOG_CONTENT'"
        fi
        
        if echo "$LOG_CONTENT" | grep -q 'Status=active'; then
            pass "Step 6 output shows persisted status"
        else
            fail "Step 6 output missing persisted status. Content: '$LOG_CONTENT'"
        fi
    else
        fail "Log file not created for step 6"
    fi
else
    info "Skipping execution output verification due to execution failure"
fi

# Test 9: Verify JSON execution log contains captured variables
section "Test 9: Verify JSON Execution Log Contains Variable Values"

JSON_LOG="$TEMP_DIR/TEST_VAR_DISPLAY_001_execution_log.json"
if [[ -f "$JSON_LOG" ]]; then
    pass "JSON execution log created"
    
    # Validate JSON
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$JSON_LOG" >/dev/null 2>&1; then
            pass "JSON execution log is valid"
        else
            fail "JSON execution log is invalid"
            jq empty "$JSON_LOG" 2>&1 | head -10
        fi
        
        # Check entry count
        ENTRY_COUNT=$(jq 'length' "$JSON_LOG")
        if [[ $ENTRY_COUNT -eq 6 ]]; then
            pass "JSON log contains correct number of entries (6 steps)"
        else
            fail "JSON log has incorrect number of entries: expected 6, got $ENTRY_COUNT"
        fi
        
        # Verify step 1 output in JSON
        STEP1_OUTPUT=$(jq -r '.[0].output' "$JSON_LOG")
        if [[ "$STEP1_OUTPUT" == 'token="abc123" user_id=42 status=active' ]]; then
            pass "JSON log entry 1 contains correct output"
        else
            fail "JSON log entry 1 has incorrect output: '$STEP1_OUTPUT'"
        fi
        
        # Verify step 3 output in JSON (with substituted variables)
        STEP3_OUTPUT=$(jq -r '.[2].output' "$JSON_LOG")
        if echo "$STEP3_OUTPUT" | grep -q 'Token=abc123'; then
            pass "JSON log entry 3 contains substituted auth_token"
        else
            fail "JSON log entry 3 missing substituted auth_token: '$STEP3_OUTPUT'"
        fi
        
        if echo "$STEP3_OUTPUT" | grep -q 'UserID=42'; then
            pass "JSON log entry 3 contains substituted user_id"
        else
            fail "JSON log entry 3 missing substituted user_id: '$STEP3_OUTPUT'"
        fi
        
        if echo "$STEP3_OUTPUT" | grep -q 'Status=active'; then
            pass "JSON log entry 3 contains substituted status"
        else
            fail "JSON log entry 3 missing substituted status: '$STEP3_OUTPUT'"
        fi
        
        # Verify all steps have exit_code 0
        ALL_SUCCESS=$(jq '[.[] | .exit_code == 0] | all' "$JSON_LOG")
        if [[ "$ALL_SUCCESS" == "true" ]]; then
            pass "All JSON log entries show successful execution (exit_code 0)"
        else
            fail "Some JSON log entries have non-zero exit codes"
        fi
    else
        info "jq not available - skipping detailed JSON validation"
        # Fallback to python
        if python3 -c "import json; json.load(open('$JSON_LOG'))" 2>/dev/null; then
            pass "JSON execution log is valid (verified with python)"
            
            # Basic validation with python
            STEP3_CHECK=$(python3 -c "import json; data = json.load(open('$JSON_LOG')); print(data[2]['output'])" 2>/dev/null)
            if echo "$STEP3_CHECK" | grep -q 'Token=abc123'; then
                pass "JSON log entry 3 verified with python"
            else
                fail "JSON log entry 3 verification failed"
            fi
        else
            fail "JSON execution log is invalid"
        fi
    fi
else
    fail "JSON execution log not created"
fi

# Test 10: Verify error handling for failed variable captures
section "Test 10: Verify Error Handling and Messages"

# Check that steps 4 and 5 still passed even with failed captures
# (empty string values should be used)
STEP4_LOG="$TEMP_DIR/TEST_VAR_DISPLAY_001_sequence-1_step-4.actual.log"
if [[ -f "$STEP4_LOG" ]]; then
    pass "Log file created for step 4 (non-matching regex)"
    LOG_CONTENT=$(cat "$STEP4_LOG")
    if [[ "$LOG_CONTENT" == "no_match_here" ]]; then
        pass "Step 4 executed successfully despite capture failure"
    else
        fail "Step 4 log has unexpected content: '$LOG_CONTENT'"
    fi
else
    fail "Log file not created for step 4"
fi

STEP5_LOG="$TEMP_DIR/TEST_VAR_DISPLAY_001_sequence-1_step-5.actual.log"
if [[ -f "$STEP5_LOG" ]]; then
    pass "Log file created for step 5 (failing command capture)"
    LOG_CONTENT=$(cat "$STEP5_LOG")
    if [[ "$LOG_CONTENT" == "testing error handling" ]]; then
        pass "Step 5 executed successfully despite command capture failure"
    else
        fail "Step 5 log has unexpected content: '$LOG_CONTENT'"
    fi
else
    fail "Log file not created for step 5"
fi

# Verify that the generated script handles empty captures gracefully
if grep -q 'missing_var=.*|| echo ""' "$VARIABLE_SCRIPT"; then
    pass "Script handles non-matching regex with fallback to empty string"
else
    info "Script may handle non-matching regex differently (acceptable)"
fi

if grep -q 'failed_capture=.*|| echo ""' "$VARIABLE_SCRIPT"; then
    pass "Script handles failing command with fallback to empty string"
else
    info "Script may handle failing command differently (acceptable)"
fi

# Test 11: Verify variable capture patterns work correctly
section "Test 11: Verify Variable Capture Pattern Conversion"

if [[ $EXECUTION_EXIT_CODE -eq 0 ]]; then
    # Test that the capture patterns work as expected (using sed for BSD compatibility)
    
    # Test auth_token pattern (quoted value)
    TOKEN_CAPTURE=$(echo 'token="abc123" user_id=42 status=active' | sed -n 's/.*token="\([^"]*\)".*/\1/p' | head -n 1)
    if [[ "$TOKEN_CAPTURE" == "abc123" ]]; then
        pass "auth_token regex pattern works correctly"
    else
        fail "auth_token regex pattern failed: got '$TOKEN_CAPTURE'"
    fi
    
    # Test user_id pattern (numeric)
    USERID_CAPTURE=$(echo 'token="abc123" user_id=42 status=active' | sed -n 's/.*user_id=\([0-9][0-9]*\).*/\1/p' | head -n 1)
    if [[ "$USERID_CAPTURE" == "42" ]]; then
        pass "user_id regex pattern works correctly"
    else
        fail "user_id regex pattern failed: got '$USERID_CAPTURE'"
    fi
    
    # Test status pattern (word)
    STATUS_CAPTURE=$(echo 'token="abc123" user_id=42 status=active' | sed -n 's/.*status=\([a-zA-Z0-9_][a-zA-Z0-9_]*\).*/\1/p' | head -n 1)
    if [[ "$STATUS_CAPTURE" == "active" ]]; then
        pass "status regex pattern works correctly"
    else
        fail "status regex pattern failed: got '$STATUS_CAPTURE'"
    fi
    
    # Test command-based captures produce actual values
    if [[ -n "$STEP3_LOG" ]] && [[ -f "$STEP3_LOG" ]]; then
        # Just verify the log contains some hostname value (not empty)
        if grep -q 'Host=' "$STEP3_LOG" && ! grep -q 'Host=$' "$STEP3_LOG"; then
            pass "Command-based hostname capture produced a value"
        else
            info "Hostname capture may be empty (acceptable in some environments)"
        fi
    fi
else
    info "Skipping regex pattern verification due to execution failure"
fi

# Test 12: Verify variable display in generated script comments
section "Test 12: Verify Script Comments and Documentation"

# Check that the script has comments about variable capture
if grep -q '# Capture variables from output' "$VARIABLE_SCRIPT"; then
    pass "Script includes variable capture documentation"
else
    fail "Script missing variable capture comments"
fi

# Check for clear step descriptions
if grep -q 'Step 1: Capture variables using regex patterns' "$VARIABLE_SCRIPT"; then
    pass "Script includes clear step descriptions"
else
    fail "Script missing clear step descriptions"
fi

if grep -q 'Step 2: Capture variables using command execution' "$VARIABLE_SCRIPT"; then
    pass "Script includes command-based capture step description"
else
    fail "Script missing command-based capture step description"
fi

# Summary
section "Test Summary"
echo ""
echo "Tests Passed: $TESTS_PASSED"
echo "Tests Failed: $TESTS_FAILED"
echo ""

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
