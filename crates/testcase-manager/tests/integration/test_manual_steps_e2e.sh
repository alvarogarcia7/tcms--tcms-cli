#!/bin/bash
#
# End-to-end integration test for manual steps in test-executor
#
# This test validates:
# 1. Test YAML generation with manual steps
# 2. Shell script generation with manual step handling
# 3. Bash syntax validation with bash -n
# 4. Script contains echo statements for description/command
# 5. Script contains read prompts for user interaction
# 6. Script execution with simulated ENTER input
#
# Usage: ./tests/integration/test_manual_steps_e2e.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
SCHEMA_FILE="$PROJECT_ROOT/schemas/tcms-schemas/tcms/test-case-main.schema.json"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

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
source "$PROJECT_ROOT/scripts/lib/shellcheck-helper.sh" || true

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
echo "Manual Steps End-to-End Integration Test"
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

# Test 1: Create test YAML with manual steps
section "Test 1: Creating Test YAML with Manual Steps"

MANUAL_YAML="$TEMP_DIR/test_manual.yaml"
cat > "$MANUAL_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_MANUAL
item: 1
tc: 1
id: TEST_MANUAL_001
description: Test case with manual steps
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Manual Steps Sequence
    description: This sequence contains manual steps for user interaction
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        manual: true
        description: Manually connect to device via SSH
        command: ssh user@device
        expected:
          success: true
          result: "0"
          output: connected
        verification:
          result: "true"
          output: "true"
      - step: 2
        description: Automated check after manual step
        command: echo 'automated'
        expected:
          success: true
          result: "0"
          output: automated
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"automated\" ]"
      - step: 3
        manual: true
        description: Manually verify LED status
        command: check LED
        expected:
          success: true
          result: "0"
          output: LED on
        verification:
          result: "true"
          output: "true"
  - id: 2
    name: Mixed Sequence
    description: Sequence with both automated and manual steps
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Automated step before manual
        command: echo 'before manual'
        expected:
          success: true
          result: "0"
          output: before manual
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"before manual\" ]"
      - step: 2
        manual: true
        description: Check physical connection
        command: inspect cable
        expected:
          success: true
          result: "0"
          output: cable ok
        verification:
          result: "true"
          output: "true"
      - step: 3
        description: Automated step after manual
        command: echo 'after manual'
        expected:
          success: true
          result: "0"
          output: after manual
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"after manual\" ]"
EOF

pass "Created test YAML with manual steps"

# Test 2: Validate YAML against schema
section "Test 2: YAML Schema Validation"

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$MANUAL_YAML" > /dev/null 2>&1; then
    pass "YAML validates against schema"
else
    fail "YAML failed schema validation"
    "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$MANUAL_YAML"
fi

# Test 3: Generate shell script from YAML
section "Test 3: Shell Script Generation"

MANUAL_SCRIPT="$TEMP_DIR/test_manual.sh"
MANUAL_GEN_OUTPUT="$TEMP_DIR/manual_gen_output.txt"
if "$TEST_EXECUTOR_BIN" generate "$MANUAL_YAML" -o "$MANUAL_SCRIPT" > "$MANUAL_GEN_OUTPUT" 2>&1; then
    pass "Generated script from YAML with manual steps"
else
    fail "Failed to generate script from YAML"
    info "Error output: $(cat "$MANUAL_GEN_OUTPUT" 2>/dev/null | head -20)"
fi

if [[ -f "$MANUAL_SCRIPT" ]]; then
    pass "Script file created"
else
    fail "Script file not found"
    exit 1
fi

# Test 4: Validate bash syntax
section "Test 4: Bash Syntax Validation"

if bash -n "$MANUAL_SCRIPT" 2>/dev/null; then
    pass "Script has valid bash syntax"
else
    fail "Script has invalid bash syntax"
    bash -n "$MANUAL_SCRIPT" 2>&1
    exit 1
fi
validate_with_shellcheck "$MANUAL_SCRIPT" "Manual steps script"

# Test 5: Verify script contains expected elements for manual steps
section "Test 5: Verify Script Contains Manual Step Elements"

# Check for bash shebang
if grep -q "#!/bin/bash" "$MANUAL_SCRIPT"; then
    pass "Script has bash shebang"
else
    fail "Script missing bash shebang"
fi

# Check for test case ID
if grep -q "TEST_MANUAL_001" "$MANUAL_SCRIPT"; then
    pass "Script contains test case ID"
else
    fail "Script missing test case ID"
fi

# Check for echo statement showing step description (for step 1, manual)
if grep -q 'echo "Step 1: Manually connect to device via SSH"' "$MANUAL_SCRIPT"; then
    pass "Script contains echo for manual step description"
else
    fail "Script missing echo for manual step description"
fi

# Check for echo statement showing command (for step 1, manual)
if grep -q 'echo "Command: ssh user@device"' "$MANUAL_SCRIPT"; then
    pass "Script contains echo for manual step command"
else
    fail "Script missing echo for manual step command"
fi

# Check for manual step info message
if grep -q 'echo "INFO: This is a manual step. You must perform this action manually."' "$MANUAL_SCRIPT"; then
    pass "Script contains manual step info message"
else
    fail "Script missing manual step info message"
fi

# Check for read_true_false prompt for manual steps
if grep -q 'read_true_false "Have you completed the manual step?"' "$MANUAL_SCRIPT"; then
    pass "Script contains read_true_false prompt for manual steps"
else
    fail "Script missing read_true_false prompt"
fi

# Verify multiple read_true_false prompts for multiple manual steps
READ_PROMPT_COUNT=$(grep -c 'read_true_false "Have you completed the manual step?"' "$MANUAL_SCRIPT")
if [[ $READ_PROMPT_COUNT -eq 3 ]]; then
    pass "Script contains correct number of read_true_false prompts (3 manual steps)"
else
    fail "Script has incorrect number of read_true_false prompts: expected 3, got $READ_PROMPT_COUNT"
fi

# Test 6: Verify manual steps don't create log files
section "Test 6: Verify Manual Steps Skip Log File Creation"

# Manual step 1 should not have LOG_FILE declaration
MANUAL_STEP_1_LOG="TEST_MANUAL_001_sequence-1_step-1.actual.log"
if ! grep -q "LOG_FILE=\"$MANUAL_STEP_1_LOG\"" "$MANUAL_SCRIPT"; then
    pass "Manual step 1 correctly has no LOG_FILE declaration"
else
    fail "Manual step 1 incorrectly has LOG_FILE declaration"
fi

# Manual step 3 (sequence 1) should not have LOG_FILE declaration
MANUAL_STEP_3_LOG="TEST_MANUAL_001_sequence-1_step-3.actual.log"
if ! grep -q "LOG_FILE=\"$MANUAL_STEP_3_LOG\"" "$MANUAL_SCRIPT"; then
    pass "Manual step 3 (seq 1) correctly has no LOG_FILE declaration"
else
    fail "Manual step 3 (seq 1) incorrectly has LOG_FILE declaration"
fi

# Manual step 2 (sequence 2) should not have LOG_FILE declaration
MANUAL_STEP_SEQ2_LOG="TEST_MANUAL_001_sequence-2_step-2.actual.log"
if ! grep -q "LOG_FILE=\"$MANUAL_STEP_SEQ2_LOG\"" "$MANUAL_SCRIPT"; then
    pass "Manual step 2 (seq 2) correctly has no LOG_FILE declaration"
else
    fail "Manual step 2 (seq 2) incorrectly has LOG_FILE declaration"
fi

# Automated steps should have LOG_FILE declarations
if grep -q "LOG_FILE=\"TEST_MANUAL_001_sequence-1_step-2.actual.log\"" "$MANUAL_SCRIPT"; then
    pass "Automated step 2 (seq 1) has LOG_FILE declaration"
else
    fail "Automated step 2 (seq 1) missing LOG_FILE declaration"
fi

if grep -q "LOG_FILE=\"TEST_MANUAL_001_sequence-2_step-1.actual.log\"" "$MANUAL_SCRIPT"; then
    pass "Automated step 1 (seq 2) has LOG_FILE declaration"
else
    fail "Automated step 1 (seq 2) missing LOG_FILE declaration"
fi

if grep -q "LOG_FILE=\"TEST_MANUAL_001_sequence-2_step-3.actual.log\"" "$MANUAL_SCRIPT"; then
    pass "Automated step 3 (seq 2) has LOG_FILE declaration"
else
    fail "Automated step 3 (seq 2) missing LOG_FILE declaration"
fi

# Verify manual steps don't use tee command
MANUAL_STEPS_WITH_TEE=$(grep -B 5 'read_true_false "Have you completed the manual step?"' "$MANUAL_SCRIPT" | grep -c '| tee' || true)
if [[ $MANUAL_STEPS_WITH_TEE -eq 0 ]]; then
    pass "Manual steps correctly don't use tee command"
else
    fail "Manual steps incorrectly use tee command: found $MANUAL_STEPS_WITH_TEE instances"
fi

# Test 7: Verify script structure for manual steps
section "Test 7: Verify Script Structure for Manual Steps"

# Extract the section for manual step 1 and verify it doesn't have execution logic
MANUAL_STEP_1_SECTION=$(sed -n '/# Step 1: Manually connect to device via SSH/,/# Step 2:/p' "$MANUAL_SCRIPT" | tail -n -1)

# Manual step should not have COMMAND_OUTPUT= assignment
if ! echo "$MANUAL_STEP_1_SECTION" | grep -q 'COMMAND_OUTPUT=$('; then
    pass "Manual step 1 correctly doesn't execute command"
else
    fail "Manual step 1 incorrectly executes command"
fi

# Manual step should not have EXIT_CODE assignment
if ! echo "$MANUAL_STEP_1_SECTION" | grep -q 'EXIT_CODE=\$?'; then
    pass "Manual step 1 correctly doesn't capture exit code"
else
    fail "Manual step 1 incorrectly captures exit code"
fi

# Manual step should not have verification logic
if ! echo "$MANUAL_STEP_1_SECTION" | grep -q 'VERIFICATION_RESULT_PASS'; then
    pass "Manual step 1 correctly has no verification logic"
else
    fail "Manual step 1 incorrectly has verification logic"
fi

# Test 8: Execute script with simulated ENTER input
section "Test 8: Script Execution with Simulated Input"

# Execute script with three empty lines (one ENTER for each manual step)
EXECUTION_OUTPUT="$TEMP_DIR/execution_output.txt"
cd "$TEMP_DIR"

# Provide three newlines for the three manual steps
if echo -e "\n\n\n" | bash "$MANUAL_SCRIPT" > "$EXECUTION_OUTPUT" 2>&1; then
    pass "Script executed successfully with simulated ENTER input"
else
    EXECUTION_EXIT_CODE=$?
    fail "Script execution failed with exit code $EXECUTION_EXIT_CODE"
    info "Output: $(cat "$EXECUTION_OUTPUT" | head -20)"
fi

cd "$PROJECT_ROOT"

# Test 9: Verify execution output contains expected messages
section "Test 9: Verify Execution Output"

if grep -q "Step 1: Manually connect to device via SSH" "$EXECUTION_OUTPUT"; then
    pass "Execution output contains manual step 1 description"
else
    fail "Execution output missing manual step 1 description"
fi

if grep -q "Command: ssh user@device" "$EXECUTION_OUTPUT"; then
    pass "Execution output contains manual step 1 command"
else
    fail "Execution output missing manual step 1 command"
fi

if grep -q "INFO: This is a manual step" "$EXECUTION_OUTPUT"; then
    pass "Execution output contains manual step info message"
else
    fail "Execution output missing manual step info message"
fi

if grep -q "Step 3: Manually verify LED status" "$EXECUTION_OUTPUT"; then
    pass "Execution output contains manual step 3 description"
else
    fail "Execution output missing manual step 3 description"
fi

if grep -q "Step 2: Check physical connection" "$EXECUTION_OUTPUT"; then
    pass "Execution output contains manual step from sequence 2"
else
    fail "Execution output missing manual step from sequence 2"
fi

# Check automated steps executed correctly
if grep -q "\[PASS\] Step 2: Automated check after manual step" "$EXECUTION_OUTPUT"; then
    pass "Automated step 2 (seq 1) executed and passed"
else
    fail "Automated step 2 (seq 1) didn't execute or pass"
fi

if grep -q "\[PASS\] Step 1: Automated step before manual" "$EXECUTION_OUTPUT"; then
    pass "Automated step 1 (seq 2) executed and passed"
else
    fail "Automated step 1 (seq 2) didn't execute or pass"
fi

if grep -q "\[PASS\] Step 3: Automated step after manual" "$EXECUTION_OUTPUT"; then
    pass "Automated step 3 (seq 2) executed and passed"
else
    fail "Automated step 3 (seq 2) didn't execute or pass"
fi

# Test 10: Verify log files created only for automated steps
section "Test 10: Verify Log Files Created Only for Automated Steps"

# Automated steps should have log files
if [[ -f "$TEMP_DIR/TEST_MANUAL_001_sequence-1_step-2.actual.log" ]]; then
    pass "Log file created for automated step 2 (seq 1)"
    LOG_CONTENT=$(cat "$TEMP_DIR/TEST_MANUAL_001_sequence-1_step-2.actual.log")
    if [[ "$LOG_CONTENT" == "automated" ]]; then
        pass "Log file for automated step 2 (seq 1) has correct content"
    else
        fail "Log file for automated step 2 (seq 1) has incorrect content: '$LOG_CONTENT'"
    fi
else
    fail "Log file not created for automated step 2 (seq 1)"
fi

if [[ -f "$TEMP_DIR/TEST_MANUAL_001_sequence-2_step-1.actual.log" ]]; then
    pass "Log file created for automated step 1 (seq 2)"
    LOG_CONTENT=$(cat "$TEMP_DIR/TEST_MANUAL_001_sequence-2_step-1.actual.log")
    if [[ "$LOG_CONTENT" == "before manual" ]]; then
        pass "Log file for automated step 1 (seq 2) has correct content"
    else
        fail "Log file for automated step 1 (seq 2) has incorrect content: '$LOG_CONTENT'"
    fi
else
    fail "Log file not created for automated step 1 (seq 2)"
fi

if [[ -f "$TEMP_DIR/TEST_MANUAL_001_sequence-2_step-3.actual.log" ]]; then
    pass "Log file created for automated step 3 (seq 2)"
    LOG_CONTENT=$(cat "$TEMP_DIR/TEST_MANUAL_001_sequence-2_step-3.actual.log")
    if [[ "$LOG_CONTENT" == "after manual" ]]; then
        pass "Log file for automated step 3 (seq 2) has correct content"
    else
        fail "Log file for automated step 3 (seq 2) has incorrect content: '$LOG_CONTENT'"
    fi
else
    fail "Log file not created for automated step 3 (seq 2)"
fi

# Manual steps should NOT have log files
if [[ ! -f "$TEMP_DIR/TEST_MANUAL_001_sequence-1_step-1.actual.log" ]]; then
    pass "No log file created for manual step 1 (seq 1)"
else
    fail "Log file incorrectly created for manual step 1 (seq 1)"
fi

if [[ ! -f "$TEMP_DIR/TEST_MANUAL_001_sequence-1_step-3.actual.log" ]]; then
    pass "No log file created for manual step 3 (seq 1)"
else
    fail "Log file incorrectly created for manual step 3 (seq 1)"
fi

if [[ ! -f "$TEMP_DIR/TEST_MANUAL_001_sequence-2_step-2.actual.log" ]]; then
    pass "No log file created for manual step 2 (seq 2)"
else
    fail "Log file incorrectly created for manual step 2 (seq 2)"
fi

# Test 11: Verify JSON execution log
section "Test 11: Verify JSON Execution Log"

JSON_LOG="$TEMP_DIR/TEST_MANUAL_001_execution_log.json"
if [[ -f "$JSON_LOG" ]]; then
    pass "JSON execution log created"
    
    # Validate JSON
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$JSON_LOG" >/dev/null 2>&1; then
            pass "JSON execution log is valid"
        else
            fail "JSON execution log is invalid"
            jq empty "$JSON_LOG" 2>&1
        fi
        
        # Check that JSON contains only automated steps (not manual)
        ENTRY_COUNT=$(jq 'length' "$JSON_LOG")
        if [[ $ENTRY_COUNT -eq 6 ]]; then
            pass "JSON log contains correct number of entries (3 automated steps, 3 manual)"
        else
            fail "JSON log has incorrect number of entries: expected 6, got $ENTRY_COUNT"
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
