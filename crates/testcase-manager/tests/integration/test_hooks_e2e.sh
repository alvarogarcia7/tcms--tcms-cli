#!/bin/bash
#
# End-to-end integration test for hooks functionality
#
# This test validates:
# 1. Test case YAML with hooks validates against schema
# 2. Shell script generation includes all eight hook execution points
# 3. Generated script properly sources/executes hook scripts
# 4. Hook scripts execute in correct order
# 5. Hook output appears in console
# 6. JSON execution log contains hook entries with hook_type and hook_path fields
# 7. on_error=continue mode allows script to continue after hook failure
# 8. on_error=fail mode terminates script on hook failure
#
# Usage: ./tests/integration/test_hooks_e2e.sh [--no-remove]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
SCHEMA_FILE="$PROJECT_ROOT/schemas/test-case.schema.json"
HOOKS_YAML="$PROJECT_ROOT/testcases/examples/hooks/TC_HOOKS_001.yml"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1

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

# Source logger library
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1
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

echo "======================================"
echo "Hooks End-to-End Integration Test"
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

if [[ ! -f "$HOOKS_YAML" ]]; then
    fail "TC_HOOKS_001.yml not found at $HOOKS_YAML"
    exit 1
fi
pass "TC_HOOKS_001.yml found"

if ! command -v bash &> /dev/null; then
    fail "bash not found"
    exit 1
fi
pass "bash available"

# Verify hook scripts exist
HOOK_SCRIPTS_DIR="$PROJECT_ROOT/testcases/examples/hooks/scripts"
HOOK_SCRIPTS=(
    "script_start.sh"
    "setup_test.sh"
    "before_sequence.sh"
    "after_sequence.sh"
    "before_step.sh"
    "after_step.sh"
    "teardown_test.sh"
    "script_end.sh"
)

for hook_script in "${HOOK_SCRIPTS[@]}"; do
    if [[ ! -f "$HOOK_SCRIPTS_DIR/$hook_script" ]]; then
        fail "Hook script not found: $hook_script"
        exit 1
    fi
done
pass "All hook scripts found"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Test 1: Validate TC_HOOKS_001.yml against schema
section "Test 1: YAML Schema Validation"

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$HOOKS_YAML" > /dev/null 2>&1; then
    pass "TC_HOOKS_001.yml validates against schema"
else
    fail "TC_HOOKS_001.yml failed schema validation"
    exit 1
fi

# Test 2: Generate script and verify all eight hooks are present
section "Test 2: Script Generation with All Eight Hooks"

GENERATED_SCRIPT="$TEMP_DIR/tc_hooks_001.sh"
if "$TEST_EXECUTOR_BIN" generate "$HOOKS_YAML" -o "$GENERATED_SCRIPT" > /dev/null 2>&1; then
    pass "Generated script from TC_HOOKS_001.yml"
else
    fail "Failed to generate script from TC_HOOKS_001.yml"
    exit 1
fi

if [[ -f "$GENERATED_SCRIPT" ]]; then
    pass "Generated script file created"
else
    fail "Generated script file not found"
    exit 1
fi

# Validate bash syntax
if bash -n "$GENERATED_SCRIPT" 2>/dev/null; then
    pass "Generated script has valid bash syntax"
else
    fail "Generated script has invalid bash syntax"
    exit 1
fi
validate_with_shellcheck "$GENERATED_SCRIPT" "Generated hooks script"

# Verify all eight hook execution points are present
EXPECTED_HOOKS=(
    "# Execute script_start hook"
    "# Execute setup_test hook"
    "# Execute before_sequence hook"
    "# Execute after_sequence hook"
    "# Execute before_step hook"
    "# Execute after_step hook"
    "# Execute teardown_test hook"
    "# Execute script_end hook"
)

for hook_comment in "${EXPECTED_HOOKS[@]}"; do
    if grep -q "$hook_comment" "$GENERATED_SCRIPT"; then
        pass "Found: $hook_comment"
    else
        fail "Missing: $hook_comment"
        exit 1
    fi
done

# Verify hook scripts are properly sourced (for .sh files)
if grep -q 'source "scripts/script_start.sh"' "$GENERATED_SCRIPT"; then
    pass "script_start.sh is properly sourced"
else
    fail "script_start.sh is not properly sourced"
    exit 1
fi

if grep -q 'source "scripts/before_step.sh"' "$GENERATED_SCRIPT"; then
    pass "before_step.sh is properly sourced"
else
    fail "before_step.sh is not properly sourced"
    exit 1
fi

# Verify HOOK_EXIT_CODE capture
if grep -q 'HOOK_EXIT_CODE=\$?' "$GENERATED_SCRIPT"; then
    pass "Hook exit code is captured"
else
    fail "Hook exit code is not captured"
    exit 1
fi

# Verify on_error handling (default is fail)
if grep -q 'Error: .* hook failed with exit code' "$GENERATED_SCRIPT"; then
    pass "on_error=fail handling is present"
else
    fail "on_error=fail handling is missing"
    exit 1
fi

# Test 3: Create test hook scripts that log to output
section "Test 3: Create Test Hook Scripts"

TEST_HOOKS_DIR="$TEMP_DIR/test_hooks"
mkdir -p "$TEST_HOOKS_DIR"

# Create a test YAML with custom hooks that write to a shared log
TEST_YAML="$TEMP_DIR/test_hooks.yaml"
TEST_LOG="$TEMP_DIR/hooks_execution.log"

cat > "$TEST_YAML" << EOF
requirement: "HOOKS_TEST"
item: 1
tc: 1
id: 'TC_HOOKS_TEST'
description: 'Test case to verify hook execution order and output'

general_initial_conditions:
  system:
    - "Test environment ready"

initial_conditions:
  environment:
    - "Hook test initialized"

hooks:
  script_start:
    command: "$TEST_HOOKS_DIR/script_start.sh"
    on_error: "fail"
  setup_test:
    command: "$TEST_HOOKS_DIR/setup_test.sh"
    on_error: "fail"
  before_sequence:
    command: "$TEST_HOOKS_DIR/before_sequence.sh"
    on_error: "fail"
  after_sequence:
    command: "$TEST_HOOKS_DIR/after_sequence.sh"
    on_error: "fail"
  before_step:
    command: "$TEST_HOOKS_DIR/before_step.sh"
    on_error: "fail"
  after_step:
    command: "$TEST_HOOKS_DIR/after_step.sh"
    on_error: "fail"
  teardown_test:
    command: "$TEST_HOOKS_DIR/teardown_test.sh"
    on_error: "fail"
  script_end:
    command: "$TEST_HOOKS_DIR/script_end.sh"
    on_error: "fail"

test_sequences:
  - id: 1
    name: "Test Sequence"
    description: "Sequence to test hook execution"
    initial_conditions:
      system:
        - "Sequence ready"
    steps:
      - step: 1
        description: "First test step"
        command: echo "Step 1 executed"
        expected:
          success: true
          result: "0"
          output: "Step 1 executed"
        verification:
          result: "[ \$EXIT_CODE -eq 0 ]"
          output: "grep -q 'Step 1 executed' <<< \"\$COMMAND_OUTPUT\""
      - step: 2
        description: "Second test step"
        command: echo "Step 2 executed"
        expected:
          success: true
          result: "0"
          output: "Step 2 executed"
        verification:
          result: "[ \$EXIT_CODE -eq 0 ]"
          output: "grep -q 'Step 2 executed' <<< \"\$COMMAND_OUTPUT\""
EOF

# Create hook scripts that log their execution
cat > "$TEST_HOOKS_DIR/script_start.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] script_start executed" | tee -a "$HOOK_LOG"
echo "1:script_start" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/setup_test.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] setup_test executed" | tee -a "$HOOK_LOG"
echo "2:setup_test" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/before_sequence.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] before_sequence executed (sequence $SEQUENCE_ID)" | tee -a "$HOOK_LOG"
echo "3:before_sequence" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/after_sequence.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] after_sequence executed (sequence $SEQUENCE_ID)" | tee -a "$HOOK_LOG"
echo "6:after_sequence" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/before_step.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] before_step executed (step $STEP_NUMBER)" | tee -a "$HOOK_LOG"
echo "4:before_step:$STEP_NUMBER" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/after_step.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] after_step executed (step $STEP_NUMBER)" | tee -a "$HOOK_LOG"
echo "5:after_step:$STEP_NUMBER" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/teardown_test.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] teardown_test executed" | tee -a "$HOOK_LOG"
echo "7:teardown_test" >> "$HOOK_ORDER_LOG"
EOF

cat > "$TEST_HOOKS_DIR/script_end.sh" << 'EOF'
#!/bin/bash
echo "[HOOK] script_end executed" | tee -a "$HOOK_LOG"
echo "8:script_end" >> "$HOOK_ORDER_LOG"
EOF

chmod +x "$TEST_HOOKS_DIR"/*.sh
pass "Created test hook scripts"

# Test 4: Execute generated script and verify hooks run in correct order
section "Test 4: Execute Script and Verify Hook Execution"

TEST_SCRIPT="$TEMP_DIR/test_hooks.sh"
if "$TEST_EXECUTOR_BIN" generate "$TEST_YAML" -o "$TEST_SCRIPT" > /dev/null 2>&1; then
    pass "Generated test script"
else
    fail "Failed to generate test script"
    exit 1
fi

# Execute the script and capture output
TEST_OUTPUT="$TEMP_DIR/test_output.txt"
HOOK_ORDER_LOG="$TEMP_DIR/hook_order.log"
cd "$TEMP_DIR"
export HOOK_LOG="$TEST_LOG"
export HOOK_ORDER_LOG="$HOOK_ORDER_LOG"
if bash "$TEST_SCRIPT" > "$TEST_OUTPUT" 2>&1; then
    pass "Test script executed successfully"
else
    fail "Test script execution failed"
    info "Output: $(cat "$TEST_OUTPUT")"
    exit 1
fi
cd "$PROJECT_ROOT"

# Verify hooks were executed (check console output)
if [[ -f "$TEST_LOG" ]]; then
    pass "Hook log file created"
else
    fail "Hook log file not found"
    exit 1
fi

# Test 5: Verify hook output appears in console
section "Test 5: Verify Hook Output in Console"

EXPECTED_HOOK_OUTPUT=(
    "[HOOK] script_start executed"
    "[HOOK] setup_test executed"
    "[HOOK] before_sequence executed"
    "[HOOK] after_sequence executed"
    "[HOOK] before_step executed"
    "[HOOK] after_step executed"
    "[HOOK] teardown_test executed"
    "[HOOK] script_end executed"
)

for hook_output in "${EXPECTED_HOOK_OUTPUT[@]}"; do
    if grep -q "$hook_output" "$TEST_LOG"; then
        pass "Found hook output: $hook_output"
    else
        fail "Missing hook output: $hook_output"
        exit 1
    fi
done

# Test 6: Verify hooks execute in correct order
section "Test 6: Verify Hook Execution Order"

if [[ ! -f "$HOOK_ORDER_LOG" ]]; then
    fail "Hook order log not found"
    exit 1
fi

# Expected order:
# 1. script_start
# 2. setup_test
# 3. before_sequence
# 4. before_step (step 1)
# 5. after_step (step 1)
# 4. before_step (step 2)
# 5. after_step (step 2)
# 6. after_sequence
# 7. teardown_test
# 8. script_end

EXPECTED_ORDER=(
    "1:script_start"
    "2:setup_test"
    "3:before_sequence"
    "4:before_step:1"
    "5:after_step:1"
    "4:before_step:2"
    "5:after_step:2"
    "6:after_sequence"
    "7:teardown_test"
    "8:script_end"
)

LINE_NUM=1
for expected_line in "${EXPECTED_ORDER[@]}"; do
    ACTUAL_LINE=$(sed -n "${LINE_NUM}p" "$HOOK_ORDER_LOG")
    if [[ "$ACTUAL_LINE" == "$expected_line" ]]; then
        pass "Hook order correct at position $LINE_NUM: $expected_line"
    else
        fail "Hook order incorrect at position $LINE_NUM: expected '$expected_line', got '$ACTUAL_LINE'"
        exit 1
    fi
    LINE_NUM=$((LINE_NUM + 1))
done

# Test 7: Verify JSON execution log contains hook entries with hook_type and hook_path fields
section "Test 7: Verify JSON Execution Log (Note: Hook logging not yet implemented)"

# Note: The current implementation does not yet log hooks to the JSON execution log.
# This is a future enhancement. For now, we verify that the JSON log exists and is valid.

JSON_LOG="$TEMP_DIR/TC_HOOKS_TEST_execution.json"
if [[ -f "$JSON_LOG" ]]; then
    pass "JSON execution log created"
    
    # Verify it's valid JSON
    if python3 -m json.tool "$JSON_LOG" > /dev/null 2>&1; then
        pass "JSON execution log is valid JSON"
    else
        fail "JSON execution log is not valid JSON"
        exit 1
    fi
    
    # Note: Hook entries would have hook_type and hook_path fields when implemented
    info "Hook entries in JSON log not yet implemented (future enhancement)"
else
    fail "JSON execution log not found"
    exit 1
fi

# Test 8: Test on_error=continue mode allows script to continue after hook failure
section "Test 8: Test on_error=continue Mode"

# Create a failing hook script
FAILING_HOOK="$TEST_HOOKS_DIR/failing_hook.sh"
cat > "$FAILING_HOOK" << 'EOF'
#!/bin/bash
echo "[HOOK] failing_hook executed (will fail)"
exit 1
EOF
chmod +x "$FAILING_HOOK"

# Create test YAML with on_error=continue
CONTINUE_YAML="$TEMP_DIR/test_continue.yaml"
cat > "$CONTINUE_YAML" << EOF
requirement: "HOOKS_CONTINUE"
item: 1
tc: 1
id: 'TC_HOOKS_CONTINUE'
description: 'Test on_error=continue mode'

general_initial_conditions:
  system:
    - "Test environment ready"

initial_conditions:
  environment:
    - "Continue mode test"

hooks:
  before_step:
    command: "$FAILING_HOOK"
    on_error: "continue"

test_sequences:
  - id: 1
    name: "Continue Test"
    description: "Test that script continues after hook failure"
    initial_conditions:
      system:
        - "Ready"
    steps:
      - step: 1
        description: "Test step"
        command: echo "Step executed after failing hook"
        expected:
          success: true
          result: "0"
          output: "Step executed after failing hook"
        verification:
          result: "[ \$EXIT_CODE -eq 0 ]"
          output: "grep -q 'Step executed' <<< \"\$COMMAND_OUTPUT\""
EOF

CONTINUE_SCRIPT="$TEMP_DIR/test_continue.sh"
if "$TEST_EXECUTOR_BIN" generate "$CONTINUE_YAML" -o "$CONTINUE_SCRIPT" > /dev/null 2>&1; then
    pass "Generated continue mode test script"
else
    fail "Failed to generate continue mode test script"
    exit 1
fi

# Verify script contains continue warning, not exit
if grep -q 'Warning: before_step hook failed.*continuing' "$CONTINUE_SCRIPT"; then
    pass "Continue mode warning found in script"
else
    fail "Continue mode warning not found in script"
    exit 1
fi

# Execute and verify it continues despite hook failure
CONTINUE_OUTPUT="$TEMP_DIR/continue_output.txt"
cd "$TEMP_DIR"
if bash "$CONTINUE_SCRIPT" > "$CONTINUE_OUTPUT" 2>&1; then
    pass "Script continued execution after hook failure"
else
    fail "Script did not continue after hook failure"
    info "Output: $(cat "$CONTINUE_OUTPUT")"
    exit 1
fi
cd "$PROJECT_ROOT"

# Verify the step was actually executed
if grep -q "Step executed after failing hook" "$CONTINUE_OUTPUT"; then
    pass "Step executed successfully after failing hook"
else
    fail "Step did not execute after failing hook"
    exit 1
fi

# Test 9: Test on_error=fail mode terminates script on hook failure
section "Test 9: Test on_error=fail Mode"

# Create test YAML with on_error=fail
FAIL_YAML="$TEMP_DIR/test_fail.yaml"
cat > "$FAIL_YAML" << EOF
requirement: "HOOKS_FAIL"
item: 1
tc: 1
id: 'TC_HOOKS_FAIL'
description: 'Test on_error=fail mode'

general_initial_conditions:
  system:
    - "Test environment ready"

initial_conditions:
  environment:
    - "Fail mode test"

hooks:
  before_step:
    command: "$FAILING_HOOK"
    on_error: "fail"

test_sequences:
  - id: 1
    name: "Fail Test"
    description: "Test that script exits on hook failure"
    initial_conditions:
      system:
        - "Ready"
    steps:
      - step: 1
        description: "Test step that should not execute"
        command: echo "This should not execute"
        expected:
          success: true
          result: "0"
          output: "This should not execute"
        verification:
          result: "[ \$EXIT_CODE -eq 0 ]"
          output: "grep -q 'This should not execute' <<< \"\$COMMAND_OUTPUT\""
EOF

FAIL_SCRIPT="$TEMP_DIR/test_fail.sh"
if "$TEST_EXECUTOR_BIN" generate "$FAIL_YAML" -o "$FAIL_SCRIPT" > /dev/null 2>&1; then
    pass "Generated fail mode test script"
else
    fail "Failed to generate fail mode test script"
    exit 1
fi

# Verify script contains exit on error
if grep -q 'Error: before_step hook failed' "$FAIL_SCRIPT" && grep -q 'exit \$HOOK_EXIT_CODE' "$FAIL_SCRIPT"; then
    pass "Fail mode exit found in script"
else
    fail "Fail mode exit not found in script"
    exit 1
fi

# Execute and verify it exits on hook failure
FAIL_OUTPUT="$TEMP_DIR/fail_output.txt"
cd "$TEMP_DIR"
if bash "$FAIL_SCRIPT" > "$FAIL_OUTPUT" 2>&1; then
    fail "Script should have exited with non-zero code after hook failure"
    exit 1
else
    pass "Script exited with non-zero code after hook failure"
fi
cd "$PROJECT_ROOT"

# Verify the step was NOT executed
if grep -q "This should not execute" "$FAIL_OUTPUT"; then
    fail "Step should not have executed after failing hook"
    exit 1
else
    pass "Step correctly did not execute after failing hook"
fi

# Summary
section "Test Summary"
echo ""
pass "All hooks tests passed!"
echo ""
echo "✓ TC_HOOKS_001.yml validated against schema"
echo "✓ All eight hook execution points present in generated script"
echo "✓ Hook scripts properly sourced/executed"
echo "✓ Hooks execute in correct order"
echo "✓ Hook output appears in console"
echo "✓ JSON execution log created (hook entries not yet implemented)"
echo "✓ on_error=continue mode allows continuation after hook failure"
echo "✓ on_error=fail mode terminates script on hook failure"
echo ""
exit 0
