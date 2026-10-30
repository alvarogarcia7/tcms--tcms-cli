#!/bin/bash
#
# End-to-end integration test for validate-yaml watch mode
#
# This test validates:
# 1. Generate test YAML files (valid and invalid)
# 2. Start validate-yaml in watch mode in background
# 3. Modify a valid file and verify it re-validates successfully
# 4. Modify a file to be invalid and verify it reports failure
# 5. Fix the invalid file and verify full validation runs when all files pass
# 6. Clean up the background process
# 7. Skip test on Windows (watch mode not supported)
#
# Usage: ./tests/integration/test_validate_yaml_watch_e2e.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
SCHEMA_FILE="$PROJECT_ROOT/schemas/tcms-schemas/tcms/test-case-main.schema.json"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# Find validate-yaml binary using workspace-aware search
cd "$PROJECT_ROOT"
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

# PID of background process
WATCH_PID=""

echo "======================================"
echo "validate-yaml Watch Mode E2E Integration Test"
echo "======================================"
echo ""

# Check if running on Windows
section "Checking Platform"

if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    info "Windows platform detected - watch mode not supported"
    echo ""
    echo -e "${YELLOW}SKIPPED: Watch mode is not supported on Windows${NC}"
    exit 0
fi

pass "Non-Windows platform detected"

# Check prerequisites
section "Checking Prerequisites"

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

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi
info "Using temporary directory: $TEMP_DIR"

# Create test YAML files
section "Creating Test YAML Files"

# Valid YAML file 1
VALID_YAML_1="$TEMP_DIR/watch_valid_1.yaml"
cat > "$VALID_YAML_1" << 'EOF'
requirement: WATCH001
item: 1
tc: 1
id: WATCH_TEST_1
description: First watch test case
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Watch Sequence 1
    description: Watch test sequence
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo test
        command: echo 'watch1'
        expected:
          success: true
          result: "0"
          output: watch1
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"watch1\" ]"
EOF

pass "Created watch_valid_1.yaml"

# Valid YAML file 2
VALID_YAML_2="$TEMP_DIR/watch_valid_2.yaml"
cat > "$VALID_YAML_2" << 'EOF'
requirement: WATCH002
item: 2
tc: 2
id: WATCH_TEST_2
description: Second watch test case
general_initial_conditions:
  System:
    - Initialized
initial_conditions:
  Device:
    - Ready
test_sequences:
  - id: 1
    name: Watch Sequence 2
    description: Another watch test sequence
    initial_conditions:
      LPA:
        - Ready
    steps:
      - step: 1
        description: Simple command
        command: "true"
        expected:
          success: true
          result: "0"
          output: none
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

pass "Created watch_valid_2.yaml"

# Valid YAML file 3 (will be modified during test)
VALID_YAML_3="$TEMP_DIR/watch_valid_3.yaml"
cat > "$VALID_YAML_3" << 'EOF'
requirement: WATCH003
item: 3
tc: 3
id: WATCH_TEST_3
description: Third watch test case
general_initial_conditions:
  System:
    - Active
initial_conditions:
  Device:
    - Online
test_sequences:
  - id: 1
    name: Watch Sequence 3
    description: Third watch test sequence
    initial_conditions:
      LPA:
        - Online
    steps:
      - step: 1
        description: Echo test
        command: echo 'watch3'
        expected:
          success: true
          result: "0"
          output: watch3
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"watch3\" ]"
EOF

pass "Created watch_valid_3.yaml"

# Start watch mode in background
section "Starting Watch Mode"

WATCH_LOG="$TEMP_DIR/watch_output.log"
"$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" --watch "$VALID_YAML_1" "$VALID_YAML_2" "$VALID_YAML_3" > "$WATCH_LOG" 2>&1 &
WATCH_PID=$!
register_background_pid "$WATCH_PID"

info "Started watch mode with PID: $WATCH_PID"

# Wait for watch mode to initialize
sleep 2

# Check if watch process is still running
if ! kill -0 "$WATCH_PID" 2>/dev/null; then
    fail "Watch mode process died unexpectedly"
    cat "$WATCH_LOG"
    exit 1
fi
pass "Watch mode process is running"

# Verify initial validation in log
if grep -q "Initial validation:" "$WATCH_LOG" && \
   grep -q "watch_valid_1.yaml" "$WATCH_LOG" && \
   grep -q "watch_valid_2.yaml" "$WATCH_LOG" && \
   grep -q "watch_valid_3.yaml" "$WATCH_LOG"; then
    pass "Initial validation completed"
else
    fail "Initial validation not found in log"
fi

if grep -q "Passed: 3" "$WATCH_LOG" && grep -q "Failed: 0" "$WATCH_LOG"; then
    pass "Initial validation shows all files passed"
else
    fail "Initial validation did not pass all files"
fi

# Test 1: Modify a valid file with valid content
section "Test 1: Modify Valid File (Valid Content)"

# Update watch_valid_1.yaml with a minor change
cat > "$VALID_YAML_1" << 'EOF'
requirement: WATCH001
item: 1
tc: 1
id: WATCH_TEST_1_MODIFIED
description: First watch test case (modified)
general_initial_conditions:
  System:
    - Ready
    - Modified
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Watch Sequence 1 Modified
    description: Watch test sequence modified
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo test modified
        command: echo 'watch1-modified'
        expected:
          success: true
          result: "0"
          output: watch1-modified
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"watch1-modified\" ]"
EOF

info "Modified watch_valid_1.yaml"

# Wait for watch to detect change and re-validate
sleep 3

# Check if re-validation occurred
if grep -q "File changes detected:" "$WATCH_LOG" && \
   grep -q "watch_valid_1.yaml" "$WATCH_LOG"; then
    pass "File change detected by watch mode"
else
    fail "File change not detected by watch mode"
fi

# Verify the file passed validation
if tail -n 50 "$WATCH_LOG" | grep -A 20 "Validating changed files:" | grep -q "✓.*watch_valid_1.yaml"; then
    pass "Modified valid file re-validated successfully"
else
    fail "Modified valid file did not re-validate successfully"
fi

# Verify full validation ran since all files passed
if tail -n 50 "$WATCH_LOG" | grep -q "All changed files passed! Running full validation"; then
    pass "Full validation triggered after all changed files passed"
else
    fail "Full validation not triggered"
fi

# Test 2: Modify a file to be invalid
section "Test 2: Modify File to Invalid Content"

# Make watch_valid_2.yaml invalid by removing required 'id' field
cat > "$VALID_YAML_2" << 'EOF'
requirement: WATCH002
item: 2
tc: 2
description: Second watch test case (now invalid - missing id)
general_initial_conditions:
  System:
    - Initialized
initial_conditions:
  Device:
    - Ready
test_sequences:
  - id: 1
    name: Watch Sequence 2
    description: Another watch test sequence
    initial_conditions:
      LPA:
        - Ready
    steps:
      - step: 1
        description: Simple command
        command: "true"
        expected:
          success: true
          result: "0"
          output: none
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

info "Modified watch_valid_2.yaml to be invalid (missing id field)"

# Wait for watch to detect change and re-validate
sleep 3

# Check if re-validation occurred for invalid file
if tail -n 50 "$WATCH_LOG" | grep -q "File changes detected:"; then
    pass "File change detected for invalid modification"
else
    fail "File change not detected for invalid modification"
fi

# Verify the file failed validation
if tail -n 50 "$WATCH_LOG" | grep -A 20 "Validating changed files:" | grep -q "✗.*watch_valid_2.yaml"; then
    pass "Invalid file reported failure correctly"
else
    fail "Invalid file did not report failure"
fi

# Verify failure count
if tail -n 50 "$WATCH_LOG" | grep -A 5 "Changed files summary:" | grep -q "Failed: 1"; then
    pass "Failure count reported correctly"
else
    fail "Failure count not reported correctly"
fi

# Test 3: Fix the invalid file
section "Test 3: Fix Invalid File"

# Fix watch_valid_2.yaml by adding back the 'id' field
cat > "$VALID_YAML_2" << 'EOF'
requirement: WATCH002
item: 2
tc: 2
id: WATCH_TEST_2_FIXED
description: Second watch test case (now fixed)
general_initial_conditions:
  System:
    - Initialized
    - Fixed
initial_conditions:
  Device:
    - Ready
test_sequences:
  - id: 1
    name: Watch Sequence 2 Fixed
    description: Another watch test sequence (fixed)
    initial_conditions:
      LPA:
        - Ready
    steps:
      - step: 1
        description: Simple command
        command: "true"
        expected:
          success: true
          result: "0"
          output: none
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

info "Fixed watch_valid_2.yaml (added id field back)"

# Wait for watch to detect change and re-validate
sleep 3

# Check if re-validation occurred
if tail -n 50 "$WATCH_LOG" | grep -q "File changes detected:"; then
    pass "File change detected for fix"
else
    fail "File change not detected for fix"
fi

# Verify the file passed validation
if tail -n 100 "$WATCH_LOG" | grep -A 20 "Validating changed files:" | tail -n 20 | grep -q "✓.*watch_valid_2.yaml"; then
    pass "Fixed file re-validated successfully"
else
    fail "Fixed file did not re-validate successfully"
fi

# Verify full validation ran since all files now pass
if tail -n 100 "$WATCH_LOG" | grep -q "All changed files passed! Running full validation"; then
    pass "Full validation triggered after fixing invalid file"
else
    fail "Full validation not triggered after fix"
fi

# Verify full validation passed all files
if tail -n 50 "$WATCH_LOG" | grep -A 5 "Summary:" | grep -q "Passed: 3" && \
   tail -n 50 "$WATCH_LOG" | grep -A 5 "Summary:" | grep -q "Failed: 0"; then
    pass "Full validation shows all files passed"
else
    fail "Full validation did not pass all files"
fi

# Test 4: Multiple simultaneous changes
section "Test 4: Multiple Simultaneous Changes"

# Modify both watch_valid_1.yaml and watch_valid_3.yaml
cat > "$VALID_YAML_1" << 'EOF'
requirement: WATCH001
item: 1
tc: 1
id: WATCH_TEST_1_FINAL
description: First watch test case (final)
general_initial_conditions:
  System:
    - Ready
    - Final
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Watch Sequence 1 Final
    description: Watch test sequence final
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo test final
        command: echo 'watch1-final'
        expected:
          success: true
          result: "0"
          output: watch1-final
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"watch1-final\" ]"
EOF

cat > "$VALID_YAML_3" << 'EOF'
requirement: WATCH003
item: 3
tc: 3
id: WATCH_TEST_3_FINAL
description: Third watch test case (final)
general_initial_conditions:
  System:
    - Active
    - Final
initial_conditions:
  Device:
    - Online
test_sequences:
  - id: 1
    name: Watch Sequence 3 Final
    description: Third watch test sequence final
    initial_conditions:
      LPA:
        - Online
    steps:
      - step: 1
        description: Echo test final
        command: echo 'watch3-final'
        expected:
          success: true
          result: "0"
          output: watch3-final
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"watch3-final\" ]"
EOF

info "Modified both watch_valid_1.yaml and watch_valid_3.yaml"

# Wait for watch to detect changes and re-validate
sleep 3

# Check if re-validation occurred for both files
if tail -n 100 "$WATCH_LOG" | grep -q "File changes detected:"; then
    pass "Multiple file changes detected"
else
    fail "Multiple file changes not detected"
fi

# Verify both files passed validation (may appear in separate validations due to timing)
RECENT_LOG=$(tail -n 100 "$WATCH_LOG")
if echo "$RECENT_LOG" | grep -q "watch_valid_1.yaml" && \
   echo "$RECENT_LOG" | grep -q "watch_valid_3.yaml"; then
    pass "Both modified files were processed"
else
    fail "Not all modified files were processed"
fi

# Cleanup is handled by trap
section "Cleanup"

if kill -0 "$WATCH_PID" 2>/dev/null; then
    kill "$WATCH_PID" 2>/dev/null || true
    wait "$WATCH_PID" 2>/dev/null || true
    pass "Watch mode process terminated"
else
    fail "Watch mode process already terminated"
fi

WATCH_PID=""

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
    echo ""
    echo "Watch log contents:"
    echo "==================="
    cat "$WATCH_LOG"
    exit 1
fi
