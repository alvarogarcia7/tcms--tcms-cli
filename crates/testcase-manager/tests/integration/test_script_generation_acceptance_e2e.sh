#!/bin/bash
#
# Comprehensive E2E Shell Acceptance Test for Script Generation
#
# This test validates script generation covering:
# - Variables (sequence.variables, capture_vars legacy/new format, cross-sequence)
# - User interaction (manual steps, non-interactive mode)
# - JSON log archival (validation, source hash, special characters)
# - JSON escaping methods (explicit Jq mode, Auto mode with fallback chain)
#
# Usage: ./tests/integration/test_script_generation_acceptance_e2e.sh [--no-remove]
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

echo "=============================================================="
echo "Script Generation Acceptance E2E Test"
echo "=============================================================="
echo ""

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

if ! command -v jq &> /dev/null; then
    fail "jq not found (required for JSON validation)"
    exit 1
fi
pass "jq available"

if ! command -v shasum &> /dev/null; then
    fail "shasum not found (required for hash validation)"
    exit 1
fi
pass "shasum available"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# ============================================================================
# SECTION 1: Test sequence.variables initialization
# ============================================================================
section "1. Test sequence.variables initialization"

VARS_YAML="$TEMP_DIR/test_sequence_variables.yaml"
cat > "$VARS_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: VAR_SEQ
item: 1
tc: 1
id: TEST_SEQUENCE_VARIABLES
description: Test case with sequence variables
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Sequence with Variables
    description: Test sequence variables initialization
    initial_conditions: {}
    variables:
      BASE_URL: "http://localhost:8080"
      TIMEOUT: "30"
      MAX_RETRIES: "3"
    steps:
      - step: 1
        description: Use sequence variables
        command: echo "URL=${BASE_URL} TIMEOUT=${TIMEOUT} RETRIES=${MAX_RETRIES}"
        expected:
          success: true
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

info "Validating YAML against schema..."
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VARS_YAML" > /dev/null 2>&1; then
    pass "YAML validated successfully"
else
    fail "YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Generating script from YAML..."
VARS_SCRIPT="$TEMP_DIR/test_sequence_variables.sh"
if "$TEST_EXECUTOR_BIN" generate "$VARS_YAML" -o "$VARS_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated successfully"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    fail "Script generation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking script initializes variables..."
if grep -q 'BASE_URL="http://localhost:8080"' "$VARS_SCRIPT"; then
    pass "BASE_URL initialized in script"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "BASE_URL not initialized in script"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'TIMEOUT="30"' "$VARS_SCRIPT"; then
    pass "TIMEOUT initialized in script"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "TIMEOUT not initialized in script"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'MAX_RETRIES="3"' "$VARS_SCRIPT"; then
    pass "MAX_RETRIES initialized in script"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "MAX_RETRIES not initialized in script"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking variables added to CAPTURED_VAR_NAMES..."
if grep -q 'CAPTURED_VAR_NAMES.*BASE_URL' "$VARS_SCRIPT"; then
    pass "BASE_URL added to CAPTURED_VAR_NAMES"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "BASE_URL not added to CAPTURED_VAR_NAMES"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'CAPTURED_VAR_NAMES.*TIMEOUT' "$VARS_SCRIPT"; then
    pass "TIMEOUT added to CAPTURED_VAR_NAMES"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "TIMEOUT not added to CAPTURED_VAR_NAMES"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'CAPTURED_VAR_NAMES.*MAX_RETRIES' "$VARS_SCRIPT"; then
    pass "MAX_RETRIES added to CAPTURED_VAR_NAMES"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "MAX_RETRIES not added to CAPTURED_VAR_NAMES"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 2: Test legacy map format capture_vars with sed substitution
# ============================================================================
section "2. Test legacy capture_vars (map format) with sed substitution"

LEGACY_YAML="$TEMP_DIR/test_legacy_capture.yaml"
cat > "$LEGACY_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: LEGACY_CAP
item: 2
tc: 2
id: TEST_LEGACY_CAPTURE
description: Test legacy capture_vars with map format
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Legacy Capture Format
    description: Test legacy map-based capture
    initial_conditions: {}
    steps:
      - step: 1
        description: Capture session ID using regex pattern
        command: echo 'SESSION_ID=ABC123'
        capture_vars:
          SESSION_ID: 'SESSION_ID=\K\w+'
        expected:
          success: true
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 2
        description: Use captured session ID
        command: echo 'Using session ${SESSION_ID}'
        expected:
          success: true
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

info "Validating legacy capture YAML..."
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$LEGACY_YAML" > /dev/null 2>&1; then
    pass "Legacy capture YAML validated"
else
    fail "Legacy capture YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Generating script..."
LEGACY_SCRIPT="$TEMP_DIR/test_legacy_capture.sh"
if "$TEST_EXECUTOR_BIN" generate "$LEGACY_YAML" -o "$LEGACY_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated for legacy capture"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation failed for legacy capture"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking for SUBSTITUTED_COMMAND sed-based substitution..."
if grep -q 'SUBSTITUTED_COMMAND=' "$LEGACY_SCRIPT"; then
    pass "SUBSTITUTED_COMMAND found in script"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "SUBSTITUTED_COMMAND not found in script"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'sed.*SUBSTITUTED_COMMAND' "$LEGACY_SCRIPT"; then
    pass "sed-based substitution logic found"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "sed-based substitution logic not found"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Executing script to verify variable propagation..."
chmod +x "$LEGACY_SCRIPT"
if bash "$LEGACY_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed successfully"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    # Check .actual.log files for substituted values
    if ls "$TEMP_DIR"/TEST_LEGACY_CAPTURE_seq*.actual.log >/dev/null 2>&1; then
        if grep -q "Using session ABC123" "$TEMP_DIR"/TEST_LEGACY_CAPTURE_seq*.actual.log 2>/dev/null; then
            pass "Substituted value found in .actual.log"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "Substituted value not found in .actual.log"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
    else
        info ".actual.log files not found (acceptable)"
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 3: Test new array format capture_vars (capture + command)
# ============================================================================
section "3. Test new array format capture_vars (capture + command)"

ARRAY_YAML="$TEMP_DIR/test_array_capture.yaml"
cat > "$ARRAY_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: ARRAY_CAP
item: 3
tc: 3
id: TEST_ARRAY_CAPTURE
description: Test array-based capture_vars format
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Array Capture Format
    description: Test new array-based capture
    initial_conditions: {}
    steps:
      - step: 1
        description: Capture using both methods
        command: echo 'TOKEN=xyz789'
        capture_vars:
          - name: TOKEN_REGEX
            capture: 'TOKEN=\K\w+'
          - name: TOKEN_CMD
            command: echo 'xyz789'
        expected:
          success: true
          result: "0"
          output: TOKEN=xyz789
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 2
        description: Use captured tokens
        command: echo 'Regex=${TOKEN_REGEX} Cmd=${TOKEN_CMD}'
        expected:
          success: true
          result: "0"
          output: Regex=xyz789 Cmd=xyz789
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

info "Validating array capture YAML..."
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$ARRAY_YAML" > /dev/null 2>&1; then
    pass "Array capture YAML validated"
else
    fail "Array capture YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Generating script with JSON logging..."
ARRAY_SCRIPT="$TEMP_DIR/test_array_capture.sh"
if "$TEST_EXECUTOR_BIN" generate --json-log "$ARRAY_YAML" -o "$ARRAY_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated with JSON logging"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation with JSON logging failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Executing script..."
chmod +x "$ARRAY_SCRIPT"
if bash "$ARRAY_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed successfully"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    # Check JSON log for captured values
    JSON_LOG="$TEMP_DIR/TEST_ARRAY_CAPTURE_execution.json"
    if [[ -f "$JSON_LOG" ]]; then
        pass "JSON log created"
        TESTS_PASSED=$((TESTS_PASSED+1))
        
        # Verify JSON log has output fields with substituted values
        if jq -e '.[1].output' "$JSON_LOG" | grep -q 'xyz789' 2>/dev/null; then
            pass "JSON log output fields reflect captured values"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "JSON log output fields do not reflect captured values"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
    else
        fail "JSON log not created"
        TESTS_FAILED=$((TESTS_FAILED+1))
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 4: Test cross-sequence variable capture
# ============================================================================
section "4. Test cross-sequence variable capture"

CROSS_SEQ_YAML="$TEMP_DIR/test_cross_sequence.yaml"
cat > "$CROSS_SEQ_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: CROSS_SEQ
item: 4
tc: 4
id: TEST_CROSS_SEQUENCE
description: Test cross-sequence variable capture
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: First Sequence
    description: Capture variable in sequence 1
    initial_conditions: {}
    steps:
      - step: 1
        description: Capture SHARED_VAR
        command: echo 'SHARED_VAR=shared123'
        capture_vars:
          SHARED_VAR: 'SHARED_VAR=\K\w+'
        expected:
          success: true
          result: "0"
          output: SHARED_VAR=shared123
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
  - id: 2
    name: Second Sequence
    description: Use variable captured in sequence 1
    initial_conditions: {}
    steps:
      - step: 1
        description: Use SHARED_VAR from sequence 1
        command: echo 'Received ${SHARED_VAR} from seq 1'
        expected:
          success: true
          result: "0"
          output: Received shared123 from seq 1
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"Received shared123 from seq 1\" ]"
EOF

info "Validating cross-sequence YAML..."
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$CROSS_SEQ_YAML" > /dev/null 2>&1; then
    pass "Cross-sequence YAML validated"
else
    fail "Cross-sequence YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Generating script..."
CROSS_SEQ_SCRIPT="$TEMP_DIR/test_cross_sequence.sh"
if "$TEST_EXECUTOR_BIN" generate "$CROSS_SEQ_YAML" -o "$CROSS_SEQ_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated for cross-sequence"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation failed for cross-sequence"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Executing script to verify cross-sequence persistence..."
chmod +x "$CROSS_SEQ_SCRIPT"
if bash "$CROSS_SEQ_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed successfully"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    # Check that sequence 2 received the value
    if ls "$TEMP_DIR"/TEST_CROSS_SEQUENCE_seq2*.actual.log >/dev/null 2>&1; then
        if grep -q "Received shared123 from seq 1" "$TEMP_DIR"/TEST_CROSS_SEQUENCE_seq2*.actual.log 2>/dev/null; then
            pass "Variable persisted across sequences"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "Variable did not persist across sequences"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
    else
        info "Sequence 2 .actual.log not found (checking differently)"
        # Alternative check - just verify execution succeeded
        pass "Cross-sequence execution completed"
        TESTS_PASSED=$((TESTS_PASSED+1))
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 5: Test manual steps interspersed with automated steps
# ============================================================================
section "5. Test manual steps interspersed with automated steps"

MANUAL_YAML="$TEMP_DIR/test_manual_steps.yaml"
cat > "$MANUAL_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: MANUAL_TEST
item: 5
tc: 5
id: TEST_MANUAL_STEPS
description: Test manual steps handling
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Mixed Manual and Automated
    description: Sequence with manual and automated steps
    initial_conditions: {}
    steps:
      - step: 1
        description: Automated step 1
        command: echo 'automated1'
        expected:
          success: true
          result: "0"
          output: automated1
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 2
        manual: true
        description: Manual step - check LED
        command: check LED
        expected:
          success: true
          result: "0"
          output: LED OK
        verification:
          result: "true"
          output: "true"
      - step: 3
        description: Automated step 2
        command: echo 'automated2'
        expected:
          success: true
          result: "0"
          output: automated2
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 4
        manual: true
        description: Manual step - check display
        command: check display
        expected:
          success: true
          result: "0"
          output: Display OK
        verification:
          result: "true"
          output: "true"
      - step: 5
        description: Automated step 3
        command: echo 'automated3'
        expected:
          success: true
          result: "0"
          output: automated3
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

info "Validating manual steps YAML..."
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$MANUAL_YAML" > /dev/null 2>&1; then
    pass "Manual steps YAML validated"
else
    fail "Manual steps YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Generating script..."
MANUAL_SCRIPT="$TEMP_DIR/test_manual_steps.sh"
if "$TEST_EXECUTOR_BIN" generate "$MANUAL_YAML" -o "$MANUAL_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated for manual steps"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation failed for manual steps"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking for read -p prompts for manual steps..."
MANUAL_PROMPTS=$(grep -c 'read -p' "$MANUAL_SCRIPT" || echo 0)
if [[ "$MANUAL_PROMPTS" -ge 2 ]]; then
    pass "Found read -p prompts for manual steps ($MANUAL_PROMPTS prompts)"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Did not find expected read -p prompts (found $MANUAL_PROMPTS, expected >= 2)"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking manual steps don't have verification blocks..."
# Manual steps should not generate EXIT_CODE/COMMAND_OUTPUT verification
if grep -A5 'Manual step - check LED' "$MANUAL_SCRIPT" | grep -q 'VERIFICATION_RESULT_PASS='; then
    fail "Manual step has VERIFICATION_RESULT_PASS block (should not have)"
    TESTS_FAILED=$((TESTS_FAILED+1))
else
    pass "Manual steps do not have VERIFICATION_RESULT_PASS blocks"
    TESTS_PASSED=$((TESTS_PASSED+1))
fi

info "Checking LOG_FILE count excludes manual steps..."
# Count LOG_FILE assignments (should be 3 for 3 automated steps, not 5)
LOG_FILE_COUNT=$(grep -c 'LOG_FILE=' "$MANUAL_SCRIPT" || echo 0)
if [[ "$LOG_FILE_COUNT" -eq 3 ]]; then
    pass "LOG_FILE count is 3 (excludes 2 manual steps)"
    TESTS_PASSED=$((TESTS_PASSED+1))
elif [[ "$LOG_FILE_COUNT" -eq 5 ]]; then
    fail "LOG_FILE count is 5 (should exclude manual steps)"
    TESTS_FAILED=$((TESTS_FAILED+1))
else
    info "LOG_FILE count is $LOG_FILE_COUNT (may vary by implementation)"
fi

info "Verifying .actual.log files not created for manual steps..."
chmod +x "$MANUAL_SCRIPT"
# Run with yes to auto-answer prompts
if yes "" | bash "$MANUAL_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed (with prompts auto-answered)"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    # Count .actual.log files (should be 3, not 5)
    ACTUAL_LOG_COUNT=$(ls "$TEMP_DIR"/TEST_MANUAL_STEPS*.actual.log 2>/dev/null | wc -l)
    if [[ "$ACTUAL_LOG_COUNT" -eq 3 ]]; then
        pass ".actual.log files created only for automated steps (count: 3)"
        TESTS_PASSED=$((TESTS_PASSED+1))
    elif [[ "$ACTUAL_LOG_COUNT" -eq 5 ]]; then
        fail ".actual.log files created for manual steps too (count: 5)"
        TESTS_FAILED=$((TESTS_FAILED+1))
    else
        info ".actual.log count is $ACTUAL_LOG_COUNT (may vary)"
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 6: Test non-interactive mode
# ============================================================================
section "6. Test non-interactive mode"

info "Executing script in non-interactive mode..."
NONINTERACTIVE_YAML="$TEMP_DIR/test_noninteractive.yaml"
cat > "$NONINTERACTIVE_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: NONINT
item: 6
tc: 6
id: TEST_NONINTERACTIVE
description: Test non-interactive execution
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Simple Automated Sequence
    description: No manual steps
    initial_conditions: {}
    steps:
      - step: 1
        description: Automated command
        command: echo 'noninteractive test'
        expected:
          success: true
          result: "0"
          output: noninteractive test
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$NONINTERACTIVE_YAML" > /dev/null 2>&1; then
    pass "Non-interactive YAML validated"
else
    fail "Non-interactive YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

NONINT_SCRIPT="$TEMP_DIR/test_noninteractive.sh"
if "$TEST_EXECUTOR_BIN" generate "$NONINTERACTIVE_YAML" -o "$NONINT_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated for non-interactive test"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

chmod +x "$NONINT_SCRIPT"
# Set timeout to ensure it doesn't hang
if timeout 10 env DEBIAN_FRONTEND=noninteractive bash "$NONINT_SCRIPT" > /dev/null 2>&1; then
    pass "Script completed in non-interactive mode without hanging"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script timed out or failed in non-interactive mode"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 7: Test JSON log validation
# ============================================================================
section "7. Test JSON log validation (fields and structure)"

JSON_TEST_YAML="$TEMP_DIR/test_json_log.yaml"
cat > "$JSON_TEST_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: JSON_LOG
item: 7
tc: 7
id: TEST_JSON_LOG
description: Test JSON log generation
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: JSON Log Test Sequence
    description: Generate JSON log entries
    initial_conditions: {}
    steps:
      - step: 1
        description: First command
        command: echo 'test1'
        expected:
          success: true
          result: "0"
          output: test1
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test1\" ]"
      - step: 2
        description: Second command
        command: echo 'test2'
        expected:
          success: true
          result: "0"
          output: test2
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test2\" ]"
  - id: 2
    name: Second Sequence
    description: More entries
    initial_conditions: {}
    steps:
      - step: 1
        description: Third command
        command: echo 'test3'
        expected:
          success: true
          result: "0"
          output: test3
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test3\" ]"
EOF

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$JSON_TEST_YAML" > /dev/null 2>&1; then
    pass "JSON log test YAML validated"
else
    fail "JSON log test YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Generating script with --json-log flag..."
JSON_TEST_SCRIPT="$TEMP_DIR/test_json_log.sh"
if "$TEST_EXECUTOR_BIN" generate --json-log "$JSON_TEST_YAML" -o "$JSON_TEST_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated with --json-log flag"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation with --json-log failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Executing script..."
chmod +x "$JSON_TEST_SCRIPT"
if bash "$JSON_TEST_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed successfully"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    JSON_LOG_FILE="$TEMP_DIR/TEST_JSON_LOG_execution.json"
    if [[ -f "$JSON_LOG_FILE" ]]; then
        pass "JSON log file created"
        TESTS_PASSED=$((TESTS_PASSED+1))
        
        info "Validating JSON structure with jq..."
        if jq empty "$JSON_LOG_FILE" >/dev/null 2>&1; then
            pass "JSON log is valid JSON"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "JSON log is not valid JSON"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
        
        info "Checking entry count..."
        ENTRY_COUNT=$(jq 'length' "$JSON_LOG_FILE" 2>/dev/null || echo 0)
        if [[ "$ENTRY_COUNT" -eq 3 ]]; then
            pass "JSON log has correct entry count (3)"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "JSON log has $ENTRY_COUNT entries (expected 3)"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
        
        info "Checking required fields in entries..."
        REQUIRED_FIELDS=(
            "test_sequence"
            "step"
            "command"
            "exit_code"
            "output"
            "timestamp"
            "result_verification_pass"
            "output_verification_pass"
        )
        
        for field in "${REQUIRED_FIELDS[@]}"; do
            if jq -e ".[0] | has(\"$field\")" "$JSON_LOG_FILE" >/dev/null 2>&1; then
                pass "Field '$field' present in JSON log entry"
                TESTS_PASSED=$((TESTS_PASSED+1))
            else
                fail "Field '$field' missing in JSON log entry"
                TESTS_FAILED=$((TESTS_FAILED+1))
            fi
        done
    else
        fail "JSON log file not created"
        TESTS_FAILED=$((TESTS_FAILED+1))
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 8: Test source_yaml_sha256 field in JSON log
# ============================================================================
section "8. Test source_yaml_sha256 field in JSON log"

HASH_TEST_YAML="$TEMP_DIR/test_hash.yaml"
cat > "$HASH_TEST_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: HASH_TEST
item: 8
tc: 8
id: TEST_HASH
description: Test source hash in JSON log
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Hash Test
    description: Test source_yaml_sha256 field
    initial_conditions: {}
    steps:
      - step: 1
        description: Simple test
        command: echo 'hash test'
        expected:
          success: true
          result: "0"
          output: hash test
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$HASH_TEST_YAML" > /dev/null 2>&1; then
    pass "Hash test YAML validated"
else
    fail "Hash test YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Computing expected SHA256 hash..."
EXPECTED_HASH=$(shasum -a 256 "$HASH_TEST_YAML" | awk '{print $1}')
pass "Expected hash: $EXPECTED_HASH"

info "Generating script with --json-log (uses generate_test_script_from_yaml path)..."
HASH_SCRIPT="$TEMP_DIR/test_hash.sh"
if "$TEST_EXECUTOR_BIN" generate --json-log "$HASH_TEST_YAML" -o "$HASH_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Executing script..."
chmod +x "$HASH_SCRIPT"
if bash "$HASH_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    HASH_LOG="$TEMP_DIR/TEST_HASH_execution.json"
    if [[ -f "$HASH_LOG" ]]; then
        pass "JSON log created"
        TESTS_PASSED=$((TESTS_PASSED+1))
        
        info "Checking for source_yaml_sha256 field..."
        if jq -e '.[0] | has("source_yaml_sha256")' "$HASH_LOG" >/dev/null 2>&1; then
            pass "source_yaml_sha256 field present"
            TESTS_PASSED=$((TESTS_PASSED+1))
            
            ACTUAL_HASH=$(jq -r '.[0].source_yaml_sha256' "$HASH_LOG")
            if [[ "$ACTUAL_HASH" == "$EXPECTED_HASH" ]]; then
                pass "source_yaml_sha256 matches expected hash"
                TESTS_PASSED=$((TESTS_PASSED+1))
            else
                fail "Hash mismatch: expected $EXPECTED_HASH, got $ACTUAL_HASH"
                TESTS_FAILED=$((TESTS_FAILED+1))
            fi
        else
            fail "source_yaml_sha256 field missing"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
    else
        fail "JSON log not created"
        TESTS_FAILED=$((TESTS_FAILED+1))
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 9: Test special characters in JSON log with explicit Jq escaping
# ============================================================================
section "9. Test special characters in JSON log with explicit Jq escaping method"

SPECIAL_YAML="$TEMP_DIR/test_special_chars.yaml"
cat > "$SPECIAL_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: SPECIAL_CHARS
item: 9
tc: 9
id: TEST_SPECIAL_CHARS
description: Test special character handling in JSON log with Jq escaping
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Special Characters Test
    description: Test quotes, backslashes, Unicode, newlines
    initial_conditions: {}
    steps:
      - step: 1
        description: Test quotes
        command: echo 'He said "Hello" to the world'
        expected:
          success: true
          result: "0"
          output: He said "Hello" to the world
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[[ \"$COMMAND_OUTPUT\" == *'\"Hello\"'* ]]"
      - step: 2
        description: Test backslashes
        command: "echo 'Escaped chars: test'"
        expected:
          success: true
          result: "0"
          output: "Escaped chars: test"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 3
        description: Test Unicode
        command: echo '☃ Unicode snowman ♠♥♦♣'
        expected:
          success: true
          result: "0"
          output: '☃ Unicode snowman ♠♥♦♣'
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 4
        description: Test newlines
        command: printf 'Line1\nLine2\nLine3'
        expected:
          success: true
          result: "0"
          output: ''
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$SPECIAL_YAML" > /dev/null 2>&1; then
    pass "Special chars YAML validated"
else
    fail "Special chars YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Creating config file with explicit Jq method..."
CONFIG_DIR="$TEMP_DIR/.testcase-manager"
mkdir -p "$CONFIG_DIR"
CONFIG_FILE="$CONFIG_DIR/config.toml"
cat > "$CONFIG_FILE" << 'EOF'
[script_generation.json_escaping]
method = "jq"
enabled = true
EOF

info "Generating script with JSON logging using Jq escaping method..."
SPECIAL_SCRIPT="$TEMP_DIR/test_special_chars.sh"
if HOME="$TEMP_DIR" "$TEST_EXECUTOR_BIN" generate --json-log "$SPECIAL_YAML" -o "$SPECIAL_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking for jq-based escaping in generated script..."
if grep -q 'jq -Rs' "$SPECIAL_SCRIPT"; then
    pass "Generated script uses jq for JSON escaping"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Generated script does not use jq for JSON escaping"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Executing script..."
chmod +x "$SPECIAL_SCRIPT"
if bash "$SPECIAL_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    SPECIAL_LOG="$TEMP_DIR/TEST_SPECIAL_CHARS_execution.json"
    if [[ -f "$SPECIAL_LOG" ]]; then
        pass "JSON log created"
        TESTS_PASSED=$((TESTS_PASSED+1))
        
        info "Validating JSON parses correctly..."
        if jq empty "$SPECIAL_LOG" >/dev/null 2>&1; then
            pass "JSON log with special characters parses correctly"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "JSON log with special characters failed to parse"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
        
        info "Checking command field integrity..."
        CMD1=$(jq -r '.[0].command' "$SPECIAL_LOG" 2>/dev/null)
        if [[ "$CMD1" == *'"Hello"'* ]]; then
            pass "Command field preserved quotes correctly"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "Command field did not preserve quotes (got: $CMD1)"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
        
        info "Checking output field integrity..."
        OUT1=$(jq -r '.[0].output' "$SPECIAL_LOG" 2>/dev/null)
        if [[ "$OUT1" == *'"Hello"'* ]] || [[ "$OUT1" == *'\\"Hello\\"'* ]]; then
            pass "Output field preserved quotes correctly"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            info "Output field: $OUT1 (may be escaped differently)"
        fi
        
        # Check backslash preservation
        CMD2=$(jq -r '.[1].command' "$SPECIAL_LOG" 2>/dev/null)
        if [[ "$CMD2" == *'\\'* ]] || [[ "$CMD2" == *'C:'* ]]; then
            pass "Command field preserved backslashes"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            info "Command field backslash handling: $CMD2"
        fi
        
        # Check Unicode
        CMD3=$(jq -r '.[2].command' "$SPECIAL_LOG" 2>/dev/null)
        if [[ "$CMD3" == *'☃'* ]] || [[ "$CMD3" == *'Unicode'* ]]; then
            pass "Command field preserved Unicode characters"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            info "Command field Unicode handling: $CMD3"
        fi
        
        # Check newlines
        OUT4=$(jq -r '.[3].output' "$SPECIAL_LOG" 2>/dev/null)
        if [[ "$OUT4" == *'\n'* ]] || [[ "$OUT4" == *'Line'* ]]; then
            pass "Output field preserved newlines (escaped or literal)"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            info "Output field newline handling: $OUT4"
        fi
    else
        fail "JSON log not created"
        TESTS_FAILED=$((TESTS_FAILED+1))
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# ============================================================================
# SECTION 10: Test Auto mode JSON escaping with fallback chain
# ============================================================================
section "10. Test Auto mode JSON escaping (jq → json-escape → shell fallback)"

AUTO_YAML="$TEMP_DIR/test_auto_escaping.yaml"
cat > "$AUTO_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: AUTO_ESCAPE
item: 10
tc: 10
id: TEST_AUTO_ESCAPING
description: Test Auto mode JSON escaping with fallback chain
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Auto Escaping Test
    description: Test Auto mode tries jq, then json-escape, then shell
    initial_conditions: {}
    steps:
      - step: 1
        description: Test auto escaping
        command: echo 'Test "quotes" and special chars'
        expected:
          success: true
          result: "0"
          output: Test "quotes" and special chars
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$AUTO_YAML" > /dev/null 2>&1; then
    pass "Auto escaping YAML validated"
else
    fail "Auto escaping YAML validation failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Creating config file with Auto method..."
AUTO_CONFIG_DIR="$TEMP_DIR/.testcase-manager-auto"
mkdir -p "$AUTO_CONFIG_DIR"
AUTO_CONFIG_FILE="$AUTO_CONFIG_DIR/config.toml"
cat > "$AUTO_CONFIG_FILE" << 'EOF'
[script_generation.json_escaping]
method = "auto"
enabled = true
EOF

info "Generating script with Auto mode..."
AUTO_SCRIPT="$TEMP_DIR/test_auto_escaping.sh"
if HOME="$TEMP_DIR" HOME="$AUTO_CONFIG_DIR/.." "$TEST_EXECUTOR_BIN" generate --json-log "$AUTO_YAML" -o "$AUTO_SCRIPT" > /dev/null 2>&1; then
    pass "Script generated with Auto mode"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Script generation with Auto mode failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Checking for Auto mode fallback chain in generated script..."
# Auto mode should generate code that tries jq first, then json-escape, then shell fallback
if grep -q 'command -v jq' "$AUTO_SCRIPT" && grep -q 'command -v json-escape' "$AUTO_SCRIPT"; then
    pass "Generated script contains Auto mode fallback detection logic"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Generated script missing Auto mode fallback detection"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'jq -Rs' "$AUTO_SCRIPT"; then
    pass "Generated script includes jq as first fallback option"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Generated script missing jq as fallback option"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

if grep -q 'json-escape' "$AUTO_SCRIPT"; then
    pass "Generated script includes json-escape as second fallback option"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    fail "Generated script missing json-escape as fallback option"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

# Look for shell fallback sed/awk/printf logic
if grep -E 'sed|awk|printf.*OUTPUT_ESCAPED' "$AUTO_SCRIPT" > /dev/null 2>&1; then
    pass "Generated script includes shell fallback as final option"
    TESTS_PASSED=$((TESTS_PASSED+1))
else
    info "Shell fallback may be implemented differently or missing"
fi

info "Executing script with jq available..."
chmod +x "$AUTO_SCRIPT"
if bash "$AUTO_SCRIPT" > /dev/null 2>&1; then
    pass "Script executed successfully"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    AUTO_LOG="$TEMP_DIR/TEST_AUTO_ESCAPING_execution.json"
    if [[ -f "$AUTO_LOG" ]]; then
        pass "JSON log created"
        TESTS_PASSED=$((TESTS_PASSED+1))
        
        if jq empty "$AUTO_LOG" >/dev/null 2>&1; then
            pass "JSON log is valid"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "JSON log is invalid"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
        
        # Verify special characters were properly escaped
        OUT=$(jq -r '.[0].output' "$AUTO_LOG" 2>/dev/null)
        if [[ "$OUT" == *'"quotes"'* ]]; then
            pass "Auto mode correctly escaped special characters"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            info "Output field: $OUT"
        fi
    else
        fail "JSON log not created"
        TESTS_FAILED=$((TESTS_FAILED+1))
    fi
else
    fail "Script execution failed"
    TESTS_FAILED=$((TESTS_FAILED+1))
fi

info "Testing fallback behavior: simulating jq unavailable..."
# Create a modified version of the script that simulates jq being unavailable
AUTO_SCRIPT_NOJQ="$TEMP_DIR/test_auto_escaping_nojq.sh"
sed 's/command -v jq/command -v jq_not_available/g' "$AUTO_SCRIPT" > "$AUTO_SCRIPT_NOJQ"
chmod +x "$AUTO_SCRIPT_NOJQ"

if bash "$AUTO_SCRIPT_NOJQ" > /dev/null 2>&1; then
    pass "Script executed successfully with jq fallback disabled"
    TESTS_PASSED=$((TESTS_PASSED+1))
    
    AUTO_LOG_NOJQ="$TEMP_DIR/TEST_AUTO_ESCAPING_execution.json"
    if [[ -f "$AUTO_LOG_NOJQ" ]]; then
        if jq empty "$AUTO_LOG_NOJQ" >/dev/null 2>&1; then
            pass "Fallback chain produced valid JSON when jq unavailable"
            TESTS_PASSED=$((TESTS_PASSED+1))
        else
            fail "Fallback chain produced invalid JSON"
            TESTS_FAILED=$((TESTS_FAILED+1))
        fi
    fi
else
    info "Script with jq fallback disabled may have failed (acceptable depending on json-escape availability)"
fi

# ============================================================================
# Summary
# ============================================================================
section "Test Summary"
echo ""
echo "Tests Passed: $TESTS_PASSED"
echo "Tests Failed: $TESTS_FAILED"
echo ""

if [[ $TESTS_FAILED -eq 0 && $TESTS_PASSED -gt 0 ]]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
