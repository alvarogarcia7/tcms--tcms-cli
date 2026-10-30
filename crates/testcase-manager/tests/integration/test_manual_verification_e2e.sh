#!/bin/bash
#
# End-to-end integration test for manual step verification
#
# This test validates:
# 1. Test YAML generation with manual steps that include verification
# 2. Shell script generation with read_true_false helper function
# 3. USER_VERIFICATION variable usage for manual steps
# 4. Y/n response simulation for verification prompts
# 5. PASS/FAIL messages based on USER_VERIFICATION result
# 6. Helper functions work in both interactive and non-interactive modes
#
# Usage: ./tests/integration/test_manual_verification_e2e.sh
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
echo "Manual Verification End-to-End Integration Test"
echo "======================================"
echo ""

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$TEST_EXECUTOR_BIN" ]]; then
    fail "test-executor binary not found at $TEST_EXECUTOR_BIN"
    echo "Run 'cargo build' first"
    exit 1
fi
pass "test-executor binary found"

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

# ============================================================================
# Test 1: Create test YAML with manual steps requiring verification
# ============================================================================
section "Test 1: Creating Test YAML with Manual Verification"

MANUAL_VERIFY_YAML="$TEMP_DIR/test_manual_verify.yaml"
cat > "$MANUAL_VERIFY_YAML" << EOF
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_MANUAL_VERIFY
item: 1
tc: 1
id: TEST_MANUAL_VERIFY_001
description: Test case with manual steps requiring verification
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Manual Verification Sequence
    description: This sequence tests manual verification with Y/n prompts
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        manual: true
        description: Manually verify LED is green
        command: Check LED color
        expected:
          success: true
          result: "0"
          output: green
        verification:
          result: "[ -f $TEMP_DIR/led_green ]"
          output: "grep -q 'green' $TEMP_DIR/led_status.log"
      - step: 2
        manual: true
        description: Manually verify display shows correct message
        command: Check display message
        expected:
          success: true
          result: "0"
          output: Welcome
        verification:
          result: "[ -f $TEMP_DIR/display_ok ]"
          output: "true"
      - step: 3
        description: Automated check after manual verification
        command: echo 'automated_check'
        expected:
          success: true
          result: "0"
          output: automated_check
        verification:
          result: "[ \$EXIT_CODE -eq 0 ]"
          output: "[ \"\$COMMAND_OUTPUT\" = \"automated_check\" ]"
EOF

pass "Created test YAML with manual verification steps"

# ============================================================================
# Test 2: Validate YAML against schema
# ============================================================================
section "Test 2: YAML Schema Validation"

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$MANUAL_VERIFY_YAML" > /dev/null 2>&1; then
    pass "YAML validates against schema"
else
    fail "YAML failed schema validation"
    "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$MANUAL_VERIFY_YAML"
fi

# ============================================================================
# Test 3: Generate shell script from YAML
# ============================================================================
section "Test 3: Shell Script Generation"

MANUAL_VERIFY_SCRIPT="$TEMP_DIR/test_manual_verify.sh"
if "$TEST_EXECUTOR_BIN" generate "$MANUAL_VERIFY_YAML" -o "$MANUAL_VERIFY_SCRIPT" > /dev/null 2>&1; then
    pass "Generated script from YAML with manual verification"
else
    fail "Failed to generate script from YAML"
fi

if [[ -f "$MANUAL_VERIFY_SCRIPT" ]]; then
    pass "Script file created"
else
    fail "Script file not found"
    exit 1
fi

# ============================================================================
# Test 4: Validate bash syntax
# ============================================================================
section "Test 4: Bash Syntax Validation"

if bash -n "$MANUAL_VERIFY_SCRIPT" 2>/dev/null; then
    pass "Script has valid bash syntax"
else
    fail "Script has invalid bash syntax"
    bash -n "$MANUAL_VERIFY_SCRIPT" 2>&1
    exit 1
fi
validate_with_shellcheck "$MANUAL_VERIFY_SCRIPT" "Manual verification script"

# ============================================================================
# Test 5: Verify script contains helper functions
# ============================================================================
section "Test 5: Verify Helper Functions Present"

# Check for read_true_false function
if grep -q "read_true_false()" "$MANUAL_VERIFY_SCRIPT"; then
    pass "Script contains read_true_false function"
else
    fail "Script missing read_true_false function"
fi

# Check for read_verification function
if grep -q "read_verification()" "$MANUAL_VERIFY_SCRIPT"; then
    pass "Script contains read_verification function"
else
    fail "Script missing read_verification function"
fi

# Check for TTY detection
if grep -q "if \[\[ \"\${DEBIAN_FRONTEND:-}\" == 'noninteractive' \]\] || ! \[ -t 0 \]; then" "$MANUAL_VERIFY_SCRIPT"; then
    pass "Script contains TTY detection logic"
else
    fail "Script missing TTY detection logic"
fi

# Check for non-interactive mode handling
if grep -q "# Non-interactive mode: return default" "$MANUAL_VERIFY_SCRIPT"; then
    pass "Script contains non-interactive mode handling"
else
    fail "Script missing non-interactive mode handling"
fi

# ============================================================================
# Test 6: Verify USER_VERIFICATION variable usage
# ============================================================================
section "Test 6: Verify USER_VERIFICATION Variable Usage"

# Check that USER_VERIFICATION_RESULT is initialized
if grep -q "USER_VERIFICATION_RESULT=false" "$MANUAL_VERIFY_SCRIPT"; then
    pass "USER_VERIFICATION_RESULT initialized"
else
    fail "USER_VERIFICATION_RESULT not initialized"
fi

# Check that USER_VERIFICATION_OUTPUT is initialized
if grep -q "USER_VERIFICATION_OUTPUT=false" "$MANUAL_VERIFY_SCRIPT"; then
    pass "USER_VERIFICATION_OUTPUT initialized"
else
    fail "USER_VERIFICATION_OUTPUT not initialized"
fi

# Check that USER_VERIFICATION is set based on result and output
if grep -q "if \[ \"\$USER_VERIFICATION_RESULT\" = true \] && \[ \"\$USER_VERIFICATION_OUTPUT\" = true \]; then" "$MANUAL_VERIFY_SCRIPT"; then
    pass "USER_VERIFICATION combined check present"
else
    fail "USER_VERIFICATION combined check missing"
fi

# Check that USER_VERIFICATION is set to true on success
if grep -q "USER_VERIFICATION=true" "$MANUAL_VERIFY_SCRIPT"; then
    pass "USER_VERIFICATION set to true"
else
    fail "USER_VERIFICATION not set to true"
fi

# Check that USER_VERIFICATION is set to false on failure
if grep -q "USER_VERIFICATION=false" "$MANUAL_VERIFY_SCRIPT"; then
    pass "USER_VERIFICATION set to false"
else
    fail "USER_VERIFICATION not set to false"
fi

# ============================================================================
# Test 7: Verify PASS/FAIL messages based on USER_VERIFICATION
# ============================================================================
section "Test 7: Verify PASS/FAIL Message Logic"

# Check for PASS message based on USER_VERIFICATION
if grep -q "if \[ \"\$USER_VERIFICATION\" = true \]; then" "$MANUAL_VERIFY_SCRIPT"; then
    pass "PASS condition check present"
else
    fail "PASS condition check missing"
fi

# Check for [PASS] message format
if grep -q "\[PASS\] Step 1: Manually verify LED is green" "$MANUAL_VERIFY_SCRIPT"; then
    pass "PASS message for step 1 present"
else
    fail "PASS message for step 1 missing"
fi

# Check for [FAIL] message format
if grep -q "\[FAIL\] Step 1: Manually verify LED is green" "$MANUAL_VERIFY_SCRIPT"; then
    pass "FAIL message for step 1 present"
else
    fail "FAIL message for step 1 missing"
fi

# Check for verification details in FAIL case
if grep -q "echo \"  Result verification: \$USER_VERIFICATION_RESULT\"" "$MANUAL_VERIFY_SCRIPT"; then
    pass "Result verification detail in FAIL message"
else
    fail "Result verification detail missing"
fi

if grep -q "echo \"  Output verification: \$USER_VERIFICATION_OUTPUT\"" "$MANUAL_VERIFY_SCRIPT"; then
    pass "Output verification detail in FAIL message"
else
    fail "Output verification detail missing"
fi

# ============================================================================
# Test 8: Execute script with successful verification (Y responses)
# ============================================================================
section "Test 8: Script Execution with Successful Verification"

# Create verification files that will make verifications pass
mkdir -p "$TEMP_DIR"
touch "$TEMP_DIR/led_green"
touch "$TEMP_DIR/display_ok"
echo "green" > "$TEMP_DIR/led_status.log"

# Execute script in current directory
cd "$TEMP_DIR"

SUCCESS_OUTPUT="$TEMP_DIR/success_output.txt"
# Execute in non-interactive mode to avoid hanging on prompts
# Even though verifications pass automatically (files exist), the script will
# still prompt "Press ENTER after completing the manual action..."
if DEBIAN_FRONTEND=noninteractive bash "$MANUAL_VERIFY_SCRIPT" > "$SUCCESS_OUTPUT" 2>&1; then
    pass "Script executed successfully with passing verifications"
else
    EXECUTION_EXIT_CODE=$?
    fail "Script execution failed with exit code $EXECUTION_EXIT_CODE"
    info "Output: $(cat "$SUCCESS_OUTPUT" | head -30)"
fi

cd "$PROJECT_ROOT"

# ============================================================================
# Test 9: Verify successful execution output
# ============================================================================
section "Test 9: Verify Successful Execution Output"

# Check for PASS messages for manual steps
if grep -q "\[PASS\] Step 1: Manually verify LED is green" "$SUCCESS_OUTPUT"; then
    pass "Step 1 passed with verification"
else
    fail "Step 1 did not pass"
    info "Output: $(grep "Step 1" "$SUCCESS_OUTPUT")"
fi

if grep -q "\[PASS\] Step 2: Manually verify display shows correct message" "$SUCCESS_OUTPUT"; then
    pass "Step 2 passed with verification"
else
    fail "Step 2 did not pass"
    info "Output: $(grep "Step 2" "$SUCCESS_OUTPUT")"
fi

# Check for automated step
if grep -q "\[PASS\] Step 3: Automated check after manual verification" "$SUCCESS_OUTPUT"; then
    pass "Automated step 3 executed and passed"
else
    fail "Automated step 3 didn't execute or pass"
fi

# ============================================================================
# Test 10: Execute script with failed verification
# ============================================================================
section "Test 10: Script Execution with Failed Verification"

# Create a new YAML with verification that will fail
FAIL_VERIFY_YAML="$TEMP_DIR/test_fail_verify.yaml"
cat > "$FAIL_VERIFY_YAML" << EOF
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_FAIL_VERIFY
item: 1
tc: 1
id: TEST_FAIL_VERIFY_001
description: Test case with failing manual verification
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Failing Verification Sequence
    description: Manual step that should fail verification
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        manual: true
        description: Check for non-existent file
        command: Check file
        expected:
          success: true
          result: "0"
          output: present
        verification:
          result: "[ -f $TEMP_DIR/nonexistent_file_xyz ]"
          output: "true"
EOF

# Generate script for failing verification
FAIL_VERIFY_SCRIPT="$TEMP_DIR/test_fail_verify.sh"
if "$TEST_EXECUTOR_BIN" generate "$FAIL_VERIFY_YAML" -o "$FAIL_VERIFY_SCRIPT" > /dev/null 2>&1; then
    pass "Generated script for failing verification test"
else
    fail "Failed to generate script for failing verification"
fi

# Execute script (should fail)
cd "$TEMP_DIR"
FAIL_OUTPUT="$TEMP_DIR/fail_output.txt"
# Execute in non-interactive mode to avoid hanging on manual step prompts
if DEBIAN_FRONTEND=noninteractive bash "$FAIL_VERIFY_SCRIPT" > "$FAIL_OUTPUT" 2>&1; then
    fail "Script should have failed but passed"
else
    pass "Script correctly failed due to verification failure"
fi
cd "$PROJECT_ROOT"

# ============================================================================
# Test 11: Verify failed execution output
# ============================================================================
section "Test 11: Verify Failed Execution Output"

# Check for FAIL message
if grep -q "\[FAIL\] Step 1: Check for non-existent file" "$FAIL_OUTPUT"; then
    pass "FAIL message present for failed verification"
else
    fail "FAIL message missing for failed verification"
fi

# Check for verification details
if grep -q "Result verification:" "$FAIL_OUTPUT"; then
    pass "Result verification detail present"
else
    fail "Result verification detail missing"
fi

if grep -q "Output verification:" "$FAIL_OUTPUT"; then
    pass "Output verification detail present"
else
    fail "Output verification detail missing"
fi

# ============================================================================
# Test 12: Test non-interactive mode behavior
# ============================================================================
section "Test 12: Non-Interactive Mode Behavior"

# Test with DEBIAN_FRONTEND=noninteractive
NONINTERACTIVE_OUTPUT="$TEMP_DIR/noninteractive_output.txt"
cd "$TEMP_DIR"
touch "$TEMP_DIR/led_green"
touch "$TEMP_DIR/display_ok"
echo "green" > "$TEMP_DIR/led_status.log"

if DEBIAN_FRONTEND=noninteractive bash "$MANUAL_VERIFY_SCRIPT" > "$NONINTERACTIVE_OUTPUT" 2>&1; then
    pass "Script executed in non-interactive mode"
else
    EXECUTION_EXIT_CODE=$?
    fail "Script failed in non-interactive mode with exit code $EXECUTION_EXIT_CODE"
fi

cd "$PROJECT_ROOT"

# Verify it still works correctly in non-interactive mode
if grep -q "\[PASS\] Step 1: Manually verify LED is green" "$NONINTERACTIVE_OUTPUT"; then
    pass "Non-interactive mode: Step 1 passed"
else
    fail "Non-interactive mode: Step 1 did not pass"
fi

if grep -q "\[PASS\] Step 2: Manually verify display shows correct message" "$NONINTERACTIVE_OUTPUT"; then
    pass "Non-interactive mode: Step 2 passed"
else
    fail "Non-interactive mode: Step 2 did not pass"
fi

# ============================================================================
# Test 13: Test with piped input (non-TTY)
# ============================================================================
section "Test 13: Piped Input (Non-TTY) Behavior"

PIPED_OUTPUT="$TEMP_DIR/piped_output.txt"
cd "$TEMP_DIR"

# Execute with empty input piped in (simulating non-TTY)
if echo "" | bash "$MANUAL_VERIFY_SCRIPT" > "$PIPED_OUTPUT" 2>&1; then
    pass "Script executed with piped input"
else
    EXECUTION_EXIT_CODE=$?
    fail "Script failed with piped input, exit code $EXECUTION_EXIT_CODE"
fi

cd "$PROJECT_ROOT"

# Verify it still works correctly with piped input
if grep -q "\[PASS\] Step 1: Manually verify LED is green" "$PIPED_OUTPUT"; then
    pass "Piped input: Step 1 passed"
else
    fail "Piped input: Step 1 did not pass"
fi

# ============================================================================
# Test 14: Verify no log files created for manual steps
# ============================================================================
section "Test 14: Verify No Log Files for Manual Steps"

# Manual steps should NOT create log files
if [[ ! -f "$TEMP_DIR/TEST_MANUAL_VERIFY_001_sequence-1_step-1.actual.log" ]]; then
    pass "No log file created for manual step 1"
else
    fail "Log file incorrectly created for manual step 1"
fi

if [[ ! -f "$TEMP_DIR/TEST_MANUAL_VERIFY_001_sequence-1_step-2.actual.log" ]]; then
    pass "No log file created for manual step 2"
else
    fail "Log file incorrectly created for manual step 2"
fi

# Automated step should have log file
if [[ -f "$TEMP_DIR/TEST_MANUAL_VERIFY_001_sequence-1_step-3.actual.log" ]]; then
    pass "Log file created for automated step 3"
    LOG_CONTENT=$(cat "$TEMP_DIR/TEST_MANUAL_VERIFY_001_sequence-1_step-3.actual.log")
    if [[ "$LOG_CONTENT" == "automated_check" ]]; then
        pass "Log file for automated step 3 has correct content"
    else
        fail "Log file for automated step 3 has incorrect content: '$LOG_CONTENT'"
    fi
else
    fail "Log file not created for automated step 3"
fi

# ============================================================================
# Test 15: Test with conditional verification in manual steps
# ============================================================================
section "Test 15: Conditional Verification in Manual Steps"

CONDITIONAL_YAML="$TEMP_DIR/test_conditional_manual.yaml"
cat > "$CONDITIONAL_YAML" << EOF
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_COND_MANUAL
item: 1
tc: 1
id: TEST_COND_MANUAL_001
description: Manual step with conditional verification
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Conditional Manual Verification
    description: Test conditional verification in manual step
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        manual: true
        description: Check deployment mode
        command: cat /etc/deployment_mode
        expected:
          success: true
          result: "0"
          output: production
        verification:
          result:
            condition: "[ -f $TEMP_DIR/production_mode ]"
            if_true:
              - "echo 'MARKER_PRODUCTION: Production mode detected'"
              - "USER_VERIFICATION_RESULT=true"
            if_false:
              - "echo 'MARKER_DEVELOPMENT: Development mode detected'"
              - "USER_VERIFICATION_RESULT=false"
            always:
              - "echo 'MARKER_ALWAYS: Mode check complete'"
          output: "true"
EOF

# Generate script
CONDITIONAL_SCRIPT="$TEMP_DIR/test_conditional_manual.sh"
if "$TEST_EXECUTOR_BIN" generate "$CONDITIONAL_YAML" -o "$CONDITIONAL_SCRIPT" > /dev/null 2>&1; then
    pass "Generated script with conditional manual verification"
else
    fail "Failed to generate script with conditional manual verification"
fi

# Validate bash syntax
if bash -n "$CONDITIONAL_SCRIPT" 2>/dev/null; then
    pass "Conditional manual script has valid bash syntax"
else
    fail "Conditional manual script has invalid bash syntax"
fi
validate_with_shellcheck "$CONDITIONAL_SCRIPT" "Conditional manual script"

# Execute with production mode file present
cd "$TEMP_DIR"
touch "$TEMP_DIR/production_mode"
CONDITIONAL_OUTPUT="$TEMP_DIR/conditional_output.txt"

# Execute in non-interactive mode to avoid hanging on manual step prompts
if DEBIAN_FRONTEND=noninteractive bash "$CONDITIONAL_SCRIPT" > "$CONDITIONAL_OUTPUT" 2>&1; then
    pass "Conditional manual script executed successfully"
else
    fail "Conditional manual script execution failed"
fi

cd "$PROJECT_ROOT"

# Verify conditional branches executed correctly
if grep -q "MARKER_PRODUCTION: Production mode detected" "$CONDITIONAL_OUTPUT"; then
    pass "if_true branch executed in conditional verification"
else
    fail "if_true branch did not execute"
fi

if grep -q "MARKER_ALWAYS: Mode check complete" "$CONDITIONAL_OUTPUT"; then
    pass "always branch executed in conditional verification"
else
    fail "always branch did not execute"
fi

if ! grep -q "MARKER_DEVELOPMENT: Development mode detected" "$CONDITIONAL_OUTPUT"; then
    pass "if_false branch correctly did not execute"
else
    fail "if_false branch incorrectly executed"
fi

# ============================================================================
# Test 16: Verify JSON execution log excludes manual steps
# ============================================================================
section "Test 16: Verify JSON Execution Log"

JSON_LOG="$TEMP_DIR/TEST_MANUAL_VERIFY_001_execution_log.json"
if [[ -f "$JSON_LOG" ]]; then
    pass "JSON execution log created"
    
    # Validate JSON
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$JSON_LOG" >/dev/null 2>&1; then
            pass "JSON execution log is valid"
        else
            fail "JSON execution log is invalid"
        fi
        
        # Check that JSON contains only automated step (not manual)
        ENTRY_COUNT=$(jq 'length' "$JSON_LOG")
        if [[ $ENTRY_COUNT -eq 1 ]]; then
            pass "JSON log contains correct number of entries (1 automated step, 0 manual)"
        else
            fail "JSON log has incorrect number of entries: expected 1, got $ENTRY_COUNT"
        fi
        
        # Verify entry is for automated step 3
        if jq -e '.[0] | .test_sequence == 1 and .step == 3' "$JSON_LOG" >/dev/null 2>&1; then
            pass "JSON log entry is for automated step 3"
        else
            fail "JSON log entry is not for automated step 3"
        fi
    else
        info "jq not available - skipping detailed JSON validation"
        # Fallback to python
        if python3 -c "import json; json.load(open('$JSON_LOG'))" 2>/dev/null; then
            pass "JSON execution log is valid (verified with python)"
        else
            fail "JSON execution log is invalid"
        fi
    fi
else
    fail "JSON execution log not created"
fi

# ============================================================================
# Summary
# ============================================================================
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
