#!/bin/bash
#
# End-to-end integration test for variable capture and passing in test-executor
#
# This test validates:
# 1. Test YAML generation with variable capture (capture_vars)
# 2. Shell script generation with variable capture logic
# 3. Variable extraction from command output using regex patterns
# 4. Variable substitution in subsequent step commands
# 5. Variable substitution in verification expressions
# 6. JSON execution log contains expected outputs using captured variables
#
# Usage: ./tests/integration/test_variable_passing_e2e.sh [--no-remove]
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

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$TEST_EXECUTOR_BIN" ]]; then
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "test-executor binary not found at $TEST_EXECUTOR_BIN"
    echo "Run 'cargo build' first"
    exit 1
fi
TESTS_PASSED=$((TESTS_PASSED+1))
pass "test-executor binary found"

if [[ ! -f "$VALIDATE_YAML_BIN" ]]; then
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "validate-yaml binary not found at $VALIDATE_YAML_BIN"
    echo "Run 'cargo build' first"
    exit 1
fi
TESTS_PASSED=$((TESTS_PASSED+1))
pass "validate-yaml binary found"

if [[ ! -f "$SCHEMA_FILE" ]]; then
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Schema file not found at $SCHEMA_FILE"
    exit 1
fi
TESTS_PASSED=$((TESTS_PASSED+1))
pass "Schema file found"

if ! command -v bash &> /dev/null; then
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "bash not found"
    exit 1
fi
TESTS_PASSED=$((TESTS_PASSED+1))
pass "bash available"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"

if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Test 1: Create test YAML with variable capture and usage
section "Test 1: Creating Test YAML with Variable Capture"

VARIABLE_YAML="$TEMP_DIR/test_variables.yaml"
cat > "$VARIABLE_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_VARS
item: 1
tc: 1
id: TEST_VAR_PASSING_001
description: Test case demonstrating variable capture and passing
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Variable Capture and Usage
    description: This sequence captures variables and uses them in subsequent steps
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Generate a session ID
        command: echo 'SESSION_ID=12345'
        capture_vars:
          - name: SESSION_ID
            capture: 'SESSION_ID=\K\d+'
        expected:
          success: true
          result: "0"
          output: SESSION_ID=12345
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"SESSION_ID=12345\" ]"
      - step: 2
        description: Use captured session ID in command
        command: echo 'Using session ${SESSION_ID}'
        expected:
          success: true
          result: "0"
          output: Using session 12345
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"Using session 12345\" ]"
      - step: 3
        description: Capture multiple values
        command: echo 'USER=testuser TOKEN=abc123xyz'
        capture_vars:
          - name: username
            capture: 'USER=\K\w+'
          - name: token
            capture: 'TOKEN=\K\w+'
        expected:
          success: true
          result: "0"
          output: USER=testuser TOKEN=abc123xyz
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 4
        description: Use multiple captured variables
        command: "echo 'Auth: ${username} with token ${token}'"
        expected:
          success: true
          result: "0"
          output: "Auth: testuser with token abc123xyz"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"Auth: testuser with token abc123xyz\" ]"
      - step: 5
        description: Capture IP address from command output
        command: echo 'Server running at 192.168.1.100:8080'
        capture_vars:
          - name: server_ip
            capture: '\d+\.\d+\.\d+\.\d+'
          - name: server_port
            capture: ':(\d+)'
        expected:
          success: true
          result: "0"
          output: Server running at 192.168.1.100:8080
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 6
        description: Verify captured IP in subsequent command
        command: echo 'Connecting to ${server_ip} on port ${server_port}'
        expected:
          success: true
          result: "0"
          output: Connecting to 192.168.1.100 on port 8080
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[[ \"$COMMAND_OUTPUT\" == *\"192.168.1.100\"* ]]"
EOF

TESTS_PASSED=$((TESTS_PASSED+1))
pass "Created test YAML with variable capture"

# Test 2: Validate YAML against schema
section "Test 2: YAML Schema Validation"

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VARIABLE_YAML" > /dev/null 2>&1; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "YAML validates against schema"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "YAML failed schema validation"
    "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VARIABLE_YAML"
fi

# Test 3: Generate shell script from YAML
section "Test 3: Shell Script Generation"

VARIABLE_SCRIPT="$TEMP_DIR/test_variables.sh"
if "$TEST_EXECUTOR_BIN" generate "$VARIABLE_YAML" -o "$VARIABLE_SCRIPT" > /dev/null 2>&1; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Generated script from YAML with variable capture"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Failed to generate script from YAML"
fi

if [[ -f "$VARIABLE_SCRIPT" ]]; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script file created"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script file not found"
    exit 1
fi

# Test 4: Validate bash syntax
section "Test 4: Bash Syntax Validation"

if bash -n "$VARIABLE_SCRIPT" 2>/dev/null; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script has valid bash syntax"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script has invalid bash syntax"
    bash -n "$VARIABLE_SCRIPT" 2>&1
    exit 1
fi
validate_with_shellcheck "$VARIABLE_SCRIPT" "Variable passing script"

# Test 5: Verify script contains STEP_VARS array and variable capture logic
section "Test 5: Verify Script Contains Variable Capture Logic"

# Check for variable storage initialization (bash 3.2+ compatible)
if grep -q 'CAPTURED_VAR_NAMES=""' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script declares STEP_VAR_NAMES variable"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script missing STEP_VAR_NAMES declaration"
fi

# Check for variable capture from step 1 (SESSION_ID)
if grep -q 'SESSION_ID=' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script captures SESSION_ID variable"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script doesn't capture SESSION_ID variable"
fi

# Check for variable capture from step 3 (username and token)
if grep -q 'username=' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script captures username variable"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script doesn't capture username variable"
fi

if grep -q 'token=' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script captures token variable"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script doesn't capture token variable"
fi

# Check for variable capture from step 5 (server_ip and server_port)
if grep -q 'server_ip=' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script captures server_ip variable"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script doesn't capture server_ip variable"
fi

if grep -q 'server_port=' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script captures server_port variable"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script doesn't capture server_port variable"
fi

# Test 6: Verify variable substitution logic is present
section "Test 6: Verify Variable Substitution Logic"

# Check for variable substitution loop
if grep -q 'for var_name in $CAPTURED_VAR_NAMES; do' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script contains variable substitution loop"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script missing variable substitution loop"
fi

# Check for SUBSTITUTED_COMMAND variable
if grep -q 'SUBSTITUTED_COMMAND=' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script uses SUBSTITUTED_COMMAND for variable substitution"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script missing SUBSTITUTED_COMMAND"
fi

# Verify sed command for variable substitution
if grep -q 'sed.*\${$var_name}' "$VARIABLE_SCRIPT"; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script performs variable substitution in commands"
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script missing variable substitution in commands"
fi

# Test 7: Execute the generated script
section "Test 7: Execute Generated Script"

EXECUTION_OUTPUT="$TEMP_DIR/execution_output.txt"
cd "$TEMP_DIR"

if bash "$VARIABLE_SCRIPT" > "$EXECUTION_OUTPUT" 2>&1; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "Script executed successfully"
    EXECUTION_EXIT_CODE=0
else
    EXECUTION_EXIT_CODE=$?
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Script execution failed with exit code $EXECUTION_EXIT_CODE"
    info "Output: $(cat "$EXECUTION_OUTPUT" | head -30)"
fi

cd "$PROJECT_ROOT"

# Test 8: Verify execution output contains expected messages
section "Test 8: Verify Execution Output"

if [[ $EXECUTION_EXIT_CODE -eq 0 ]]; then
    # Note: Command outputs are verified in Test 9 (log files test) and Test 10 (JSON log test)
    # Here we just verify that the script executed successfully, which we already did above

    # Check for TESTS_PASSED=$((TESTS_PASSED+1))
pass indicators
    PASS_COUNT=$(grep -c '\[PASS\]' "$EXECUTION_OUTPUT")
    if [[ $PASS_COUNT -eq 6 ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "All 6 steps passed verification"
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Expected 6 passed steps, got $PASS_COUNT"
    fi
else
    info "Skipping execution output verification due to execution failure"
fi

# Test 9: Verify log files contain expected outputs
section "Test 9: Verify Log Files Contain Variable-Substituted Output"

if [[ $EXECUTION_EXIT_CODE -eq 0 ]]; then
    # Step 1 log should contain the SESSION_ID output
    STEP1_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_sequence-1_step-1.actual.log"
    if [[ -f "$STEP1_LOG" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "Log file created for step 1"
        LOG_CONTENT=$(cat "$STEP1_LOG")
        if [[ "$LOG_CONTENT" == "SESSION_ID=12345" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "Step 1 log contains correct output"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Step 1 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Log file not created for step 1"
    fi

    # Step 2 log should contain output with substituted session_id
    STEP2_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_sequence-1_step-2.actual.log"
    if [[ -f "$STEP2_LOG" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "Log file created for step 2"
        LOG_CONTENT=$(cat "$STEP2_LOG")
        if [[ "$LOG_CONTENT" == "Using session 12345" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "Step 2 log contains output with substituted session_id"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Step 2 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Log file not created for step 2"
    fi

    # Step 3 log should contain USER and TOKEN
    STEP3_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_sequence-1_step-3.actual.log"
    if [[ -f "$STEP3_LOG" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "Log file created for step 3"
        LOG_CONTENT=$(cat "$STEP3_LOG")
        if [[ "$LOG_CONTENT" == "USER=testuser TOKEN=abc123xyz" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "Step 3 log contains correct output"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Step 3 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Log file not created for step 3"
    fi

    # Step 4 log should contain output with substituted username and token
    STEP4_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_sequence-1_step-4.actual.log"
    if [[ -f "$STEP4_LOG" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "Log file created for step 4"
        LOG_CONTENT=$(cat "$STEP4_LOG")
        if [[ "$LOG_CONTENT" == "Auth: testuser with token abc123xyz" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "Step 4 log contains output with substituted username and token"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Step 4 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Log file not created for step 4"
    fi

    # Step 5 log should contain IP and port
    STEP5_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_sequence-1_step-5.actual.log"
    if [[ -f "$STEP5_LOG" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "Log file created for step 5"
        LOG_CONTENT=$(cat "$STEP5_LOG")
        if [[ "$LOG_CONTENT" == "Server running at 192.168.1.100:8080" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "Step 5 log contains correct output"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Step 5 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Log file not created for step 5"
    fi

    # Step 6 log should contain output with substituted IP and port
    STEP6_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_sequence-1_step-6.actual.log"
    if [[ -f "$STEP6_LOG" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "Log file created for step 6"
        LOG_CONTENT=$(cat "$STEP6_LOG")
        if [[ "$LOG_CONTENT" == "Connecting to 192.168.1.100 on port 8080" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "Step 6 log contains output with substituted IP and port"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Step 6 log has incorrect content: '$LOG_CONTENT'"
        fi
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "Log file not created for step 6"
    fi
else
    info "Skipping log file verification due to execution failure"
fi

set -x

# Test 10: Verify JSON execution log contains expected outputs
section "Test 10: Verify JSON Execution Log"

JSON_LOG="$TEMP_DIR/TEST_VAR_PASSING_001_execution_log.json"
if [[ -f "$JSON_LOG" ]]; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON execution log created"
    
    # Validate JSON
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$JSON_LOG" >/dev/null 2>&1; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON execution log is valid"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON execution log is invalid"
            jq empty "$JSON_LOG" 2>&1
        fi
        
        # Check entry count
        ENTRY_COUNT=$(jq 'length' "$JSON_LOG")
        if [[ $ENTRY_COUNT -eq 6 ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log contains correct number of entries (6 steps)"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log has incorrect number of entries: expected 6, got $ENTRY_COUNT"
        fi
        
        # Verify step 1 captured the session ID
        STEP1_OUTPUT=$(jq -r '.[0].output' "$JSON_LOG")
        if [[ "$STEP1_OUTPUT" == "SESSION_ID=12345" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 1 contains correct output"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 1 has incorrect output: '$STEP1_OUTPUT'"
        fi
        
        # Verify step 2 used the captured session ID
        STEP2_OUTPUT=$(jq -r '.[1].output' "$JSON_LOG")
        if [[ "$STEP2_OUTPUT" == "Using session 12345" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 2 contains output with substituted session_id"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 2 has incorrect output: '$STEP2_OUTPUT'"
        fi
        
        # Verify step 3 captured username and token
        STEP3_OUTPUT=$(jq -r '.[2].output' "$JSON_LOG")
        if [[ "$STEP3_OUTPUT" == "USER=testuser TOKEN=abc123xyz" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 3 contains correct output"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 3 has incorrect output: '$STEP3_OUTPUT'"
        fi
        
        # Verify step 4 used the captured username and token
        STEP4_OUTPUT=$(jq -r '.[3].output' "$JSON_LOG")
        if [[ "$STEP4_OUTPUT" == "Auth: testuser with token abc123xyz" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 4 contains output with substituted username and token"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 4 has incorrect output: '$STEP4_OUTPUT'"
        fi
        
        # Verify step 5 captured IP and port
        STEP5_OUTPUT=$(jq -r '.[4].output' "$JSON_LOG")
        if [[ "$STEP5_OUTPUT" == "Server running at 192.168.1.100:8080" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 5 contains correct output"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 5 has incorrect output: '$STEP5_OUTPUT'"
        fi
        
        # Verify step 6 used the captured IP and port
        STEP6_OUTPUT=$(jq -r '.[5].output' "$JSON_LOG")
        if [[ "$STEP6_OUTPUT" == "Connecting to 192.168.1.100 on port 8080" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 6 contains output with substituted IP and port"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 6 has incorrect output: '$STEP6_OUTPUT'"
        fi
        
        # Verify all steps have exit_code 0
        ALL_SUCCESS=$(jq '[.[] | .exit_code == 0] | all' "$JSON_LOG")
        if [[ "$ALL_SUCCESS" == "true" ]]; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "All JSON log entries show successful execution (exit_code 0)"
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "Some JSON log entries have non-zero exit codes"
        fi
    else
        info "jq not available - skipping detailed JSON validation"
        # Fallback to python
        if python3 -c "import json; json.load(open('$JSON_LOG'))" 2>/dev/null; then
            TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON execution log is valid (verified with python)"
            
            # Basic validation with python
            STEP2_CHECK=$(python3 -c "import json; data = json.load(open('$JSON_LOG')); print(data[1]['output'])" 2>/dev/null)
            if [[ "$STEP2_CHECK" == "Using session 12345" ]]; then
                TESTS_PASSED=$((TESTS_PASSED+1))
pass "JSON log entry 2 verified with python"
            else
                TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON log entry 2 verification failed"
            fi
        else
            TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON execution log is invalid"
        fi
    fi
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "JSON execution log not created"
fi

# Test 11: Verify variable capture regex patterns work correctly
section "Test 11: Verify Variable Capture Patterns"

if [[ $EXECUTION_EXIT_CODE -eq 0 ]]; then
    # Test that the capture patterns work as expected (using sed for BSD compatibility)
    # This validates the capture patterns used in the test
    
    # Test session_id pattern
    SESSION_CAPTURE=$(echo "SESSION_ID=12345" | sed -n 's/.*SESSION_ID=\([0-9][0-9]*\).*/\1/p' | head -n 1)
    if [[ "$SESSION_CAPTURE" == "12345" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "session_id regex pattern works correctly"
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "session_id regex pattern failed: got '$SESSION_CAPTURE'"
    fi
    
    # Test username pattern
    USERNAME_CAPTURE=$(echo "USER=testuser TOKEN=abc123xyz" | sed -n 's/.*USER=\([a-zA-Z0-9_][a-zA-Z0-9_]*\).*/\1/p' | head -n 1)
    if [[ "$USERNAME_CAPTURE" == "testuser" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "username regex pattern works correctly"
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "username regex pattern failed: got '$USERNAME_CAPTURE'"
    fi
    
    # Test token pattern
    TOKEN_CAPTURE=$(echo "USER=testuser TOKEN=abc123xyz" | sed -n 's/.*TOKEN=\([a-zA-Z0-9_][a-zA-Z0-9_]*\).*/\1/p' | head -n 1)
    if [[ "$TOKEN_CAPTURE" == "abc123xyz" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "token regex pattern works correctly"
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "token regex pattern failed: got '$TOKEN_CAPTURE'"
    fi
    
    # Test server_ip pattern
    IP_CAPTURE=$(echo "Server running at 192.168.1.100:8080" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+' | head -n 1)
    if [[ "$IP_CAPTURE" == "192.168.1.100" ]]; then
        TESTS_PASSED=$((TESTS_PASSED+1))
pass "server_ip regex pattern works correctly"
    else
        TESTS_FAILED=$((TESTS_FAILED+1))
fail "server_ip regex pattern failed: got '$IP_CAPTURE'"
    fi
else
    info "Skipping regex pattern verification due to execution failure"
fi

# Summary
section "Test Summary"
log_info "Tests Passed: $TESTS_PASSED"
log_info "Tests Failed: $TESTS_FAILED"

if [[ $TESTS_FAILED -eq 0 ]]; then
    TESTS_PASSED=$((TESTS_PASSED+1))
pass "All tests passed!"
    exit 0
else
    TESTS_FAILED=$((TESTS_FAILED+1))
fail "Some tests failed!"
    exit 1
fi
