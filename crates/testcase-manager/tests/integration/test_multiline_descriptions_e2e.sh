#!/bin/bash
#
# End-to-end integration test for multi-line step descriptions
#
# This test validates:
# 1. YAML test case generation with multi-line step descriptions (| and > syntax)
# 2. Shell script generation with properly commented multi-line descriptions
# 3. Bash syntax validation of generated scripts
# 4. Successful execution of generated scripts
# 5. JSON execution log generation and validation
#
# Usage: ./tests/integration/test_multiline_descriptions_e2e.sh [--no-remove]
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
echo "Multi-line Descriptions End-to-End Integration Test"
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

# Test 1: Create test YAML with multi-line descriptions using pipe (|) syntax
section "Test 1: Creating Test YAML with Multi-line Descriptions (Pipe Syntax)"

PIPE_YAML="$TEMP_DIR/test_pipe_syntax.yaml"
cat > "$PIPE_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_MULTILINE_001
item: 1
tc: 1
id: TEST_MULTILINE_PIPE
description: |
  Test case demonstrating multi-line descriptions
  using YAML pipe (|) syntax which preserves line breaks
  and formatting exactly as written.
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Multi-line Pipe Syntax Sequence
    description: |
      This sequence tests that multi-line descriptions using pipe syntax
      are properly handled in generated scripts.
      Each line should appear as a separate comment.
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: |
          This step demonstrates a multi-line description.
          Line 2: Execute a simple echo command.
          Line 3: Verify the output matches expected value.
          Line 4: End of multi-line description.
        command: echo 'hello world'
        expected:
          success: true
          result: "0"
          output: hello world
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"hello world\" ]"
      - step: 2
        description: |
          Another multi-line description with special formatting:
          - Bullet point 1
          - Bullet point 2
          - Bullet point 3
          
          This includes a blank line above.
        command: echo 'test complete'
        expected:
          success: true
          result: "0"
          output: test complete
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test complete\" ]"
EOF

pass "Created test YAML with pipe syntax multi-line descriptions"

# Test 2: YAML with multi-line descriptions using fold (>) syntax
# NOTE: This test has been converted to a Rust E2E test
# See: crates/testcase-manager/tests/multiline_descriptions_e2e_test.rs
# Test file: crates/testcase-manager/tests/sample/test_fold_syntax.yaml
section "Test 2: Multi-line Descriptions (Fold Syntax) - Rust E2E Test"

# Use the test file from the sample directory instead of inline heredoc
FOLD_YAML="$PROJECT_ROOT/crates/testcase-manager/tests/sample/test_fold_syntax.yaml"

if [[ ! -f "$FOLD_YAML" ]]; then
    fail "Fold syntax test file not found at $FOLD_YAML"
    echo "This test has been converted to a Rust E2E test."
    echo "Please ensure the file exists: crates/testcase-manager/tests/sample/test_fold_syntax.yaml"
    ((TESTS_FAILED++))
else
    pass "Fold syntax YAML test file found (Rust E2E test)"
    ((TESTS_PASSED++))
fi

# Test 3: Create test YAML with mixed syntax
section "Test 3: Creating Test YAML with Mixed Multi-line Syntax"

MIXED_YAML="$TEMP_DIR/test_mixed_syntax.yaml"
cat > "$MIXED_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: TEST_MULTILINE_003
item: 1
tc: 3
id: TEST_MULTILINE_MIXED
description: |
  Test case with mixed multi-line description styles
  combining both pipe and fold syntax in different fields.
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Mixed Syntax Sequence
    description: >
      This sequence uses fold syntax for the sequence description.
      It should appear as a continuous paragraph.
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: |
          Step 1: Multi-line with pipe syntax
          - Creates specific file structure
          - Maintains exact formatting
          - Preserves indentation
        command: echo 'step one'
        expected:
          success: true
          result: "0"
          output: step one
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"step one\" ]"
      - step: 2
        description: >
          Step 2: Multi-line with fold syntax that combines
          multiple sentences into a single flowing description
          suitable for detailed explanations.
        command: echo 'step two'
        expected:
          success: true
          result: "0"
          output: step two
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"step two\" ]"
      - step: 3
        description: |
          Step 3: Complex multi-line description
          
          This includes:
          1. Numbered items
          2. Multiple paragraphs
          3. Blank lines for separation
          
          Final paragraph with conclusion.
        command: echo 'step three'
        expected:
          success: true
          result: "0"
          output: step three
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"step three\" ]"
EOF

pass "Created test YAML with mixed multi-line syntax"

# Test 4: Validate YAML files against schema
section "Test 4: YAML Schema Validation"

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$PIPE_YAML" > /dev/null 2>&1; then
    pass "Pipe syntax YAML validates against schema"
    ((TESTS_PASSED++))
else
    fail "Pipe syntax YAML failed schema validation"
    ((TESTS_FAILED++))
fi

# Fold syntax YAML validation - using static file from Rust E2E test
if [[ -f "$FOLD_YAML" ]]; then
    if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$FOLD_YAML" > /dev/null 2>&1; then
        pass "Fold syntax YAML validates against schema (Rust E2E test file)"
        ((TESTS_PASSED++))
    else
        fail "Fold syntax YAML failed schema validation"
        ((TESTS_FAILED++))
    fi
else
    info "Fold syntax YAML validation skipped - file managed by Rust E2E test"
fi

if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$MIXED_YAML" > /dev/null 2>&1; then
    pass "Mixed syntax YAML validates against schema"
    ((TESTS_PASSED++))
else
    fail "Mixed syntax YAML failed schema validation"
    ((TESTS_FAILED++))
fi

# Test 5: Generate shell scripts from YAML files
section "Test 5: Shell Script Generation"

PIPE_SCRIPT="$TEMP_DIR/test_pipe_syntax.sh"
if "$TEST_EXECUTOR_BIN" generate "$PIPE_YAML" -o "$PIPE_SCRIPT" > "$TEMP_DIR/pipe_gen_output.txt" 2>&1; then
    pass "Generated script from pipe syntax YAML"
    ((TESTS_PASSED++))
else
    fail "Failed to generate script from pipe syntax YAML"
    info "Error output: $(cat "$TEMP_DIR/pipe_gen_output.txt" 2>/dev/null | head -20)"
    ((TESTS_FAILED++))
fi

if [[ -f "$PIPE_SCRIPT" ]]; then
    pass "Pipe syntax script file created"
    ((TESTS_PASSED++))
else
    fail "Pipe syntax script file not found"
    ((TESTS_FAILED++))
fi

# NOTE: Fold syntax test is now handled by Rust E2E test
# See: crates/testcase-manager/tests/multiline_descriptions_e2e_test.rs
# Skipping fold syntax script generation in bash test
info "Fold syntax test is handled by Rust E2E test - skipping bash generation"
FOLD_SCRIPT=""

MIXED_SCRIPT="$TEMP_DIR/test_mixed_syntax.sh"
if "$TEST_EXECUTOR_BIN" generate "$MIXED_YAML" -o "$MIXED_SCRIPT" > "$TEMP_DIR/mixed_gen_output.txt" 2>&1; then
    pass "Generated script from mixed syntax YAML"
    ((TESTS_PASSED++))
else
    fail "Failed to generate script from mixed syntax YAML"
    info "Error output: $(cat "$TEMP_DIR/mixed_gen_output.txt" 2>/dev/null | head -20)"
    ((TESTS_FAILED++))
fi

if [[ -f "$MIXED_SCRIPT" ]]; then
    pass "Mixed syntax script file created"
    ((TESTS_PASSED++))
else
    fail "Mixed syntax script file not found"
    ((TESTS_FAILED++))
fi

# Test 6: Validate bash syntax of generated scripts
section "Test 6: Bash Syntax Validation"

if [[ -f "$PIPE_SCRIPT" ]]; then
    if bash -n "$PIPE_SCRIPT" 2>/dev/null; then
        pass "Pipe syntax script has valid bash syntax"
        ((TESTS_PASSED++))
    else
        fail "Pipe syntax script has invalid bash syntax"
        bash -n "$PIPE_SCRIPT" 2>&1 | head -20
        ((TESTS_FAILED++))
    fi
    validate_with_shellcheck "$PIPE_SCRIPT" "Pipe syntax script"
fi

# Fold syntax bash validation skipped - handled by Rust E2E test
if [[ -n "$FOLD_SCRIPT" && -f "$FOLD_SCRIPT" ]]; then
    if bash -n "$FOLD_SCRIPT" 2>/dev/null; then
        pass "Fold syntax script has valid bash syntax"
        ((TESTS_PASSED++))
    else
        fail "Fold syntax script has invalid bash syntax"
        bash -n "$FOLD_SCRIPT" 2>&1 | head -20
        ((TESTS_FAILED++))
    fi
    validate_with_shellcheck "$FOLD_SCRIPT" "Fold syntax script"
else
    info "Fold syntax test skipped - handled by Rust E2E test"
fi

if [[ -f "$MIXED_SCRIPT" ]]; then
    if bash -n "$MIXED_SCRIPT" 2>/dev/null; then
        pass "Mixed syntax script has valid bash syntax"
        ((TESTS_PASSED++))
    else
        fail "Mixed syntax script has invalid bash syntax"
        bash -n "$MIXED_SCRIPT" 2>&1 | head -20
        ((TESTS_FAILED++))
    fi
    validate_with_shellcheck "$MIXED_SCRIPT" "Mixed syntax script"
fi

# Test 7: Verify generated scripts contain properly commented multi-line descriptions
section "Test 7: Verify Multi-line Description Comments in Generated Scripts"

if [[ -f "$PIPE_SCRIPT" ]]; then
    # Check that multi-line descriptions are present as comments
    if grep -q "# This step demonstrates a multi-line description" "$PIPE_SCRIPT"; then
        pass "Pipe script contains first line of multi-line description"
        ((TESTS_PASSED++))
    else
        fail "Pipe script missing first line of multi-line description"
        ((TESTS_FAILED++))
    fi
    
    if grep -q "# Line 2: Execute a simple echo command" "$PIPE_SCRIPT"; then
        pass "Pipe script contains second line of multi-line description"
        ((TESTS_PASSED++))
    else
        fail "Pipe script missing second line of multi-line description"
        ((TESTS_FAILED++))
    fi
    
    if grep -q "# Line 3: Verify the output matches expected value" "$PIPE_SCRIPT"; then
        pass "Pipe script contains third line of multi-line description"
        ((TESTS_PASSED++))
    else
        fail "Pipe script missing third line of multi-line description"
        ((TESTS_FAILED++))
    fi
    
    # Check for bullet points in second step
    if grep -q "# - Bullet point 1" "$PIPE_SCRIPT"; then
        pass "Pipe script preserves bullet point formatting"
        ((TESTS_PASSED++))
    else
        fail "Pipe script does not preserve bullet point formatting"
        ((TESTS_FAILED++))
    fi
fi

# Fold syntax description verification skipped - handled by Rust E2E test
if [[ -n "$FOLD_SCRIPT" && -f "$FOLD_SCRIPT" ]]; then
    # Fold syntax should combine lines into single comment or multiple comments
    # Check that description text is present (regardless of line breaks)
    if grep -q "This step demonstrates a folded multi-line description" "$FOLD_SCRIPT"; then
        pass "Fold script contains multi-line description text"
        ((TESTS_PASSED++))
    else
        fail "Fold script missing multi-line description text"
        ((TESTS_FAILED++))
    fi
else
    info "Fold syntax description test skipped - handled by Rust E2E test"
fi

if [[ -f "$MIXED_SCRIPT" ]]; then
    # Check pipe syntax preservation in step 1
    if grep -q "# Step 1: Multi-line with pipe syntax" "$MIXED_SCRIPT"; then
        pass "Mixed script contains pipe syntax description"
        ((TESTS_PASSED++))
    else
        fail "Mixed script missing pipe syntax description"
        ((TESTS_FAILED++))
    fi
    
    if grep -q "# - Creates specific file structure" "$MIXED_SCRIPT"; then
        pass "Mixed script preserves bullet points from pipe syntax"
        ((TESTS_PASSED++))
    else
        fail "Mixed script does not preserve bullet points from pipe syntax"
        ((TESTS_FAILED++))
    fi
    
    # Check fold syntax in step 2
    if grep -q "Step 2: Multi-line with fold syntax" "$MIXED_SCRIPT"; then
        pass "Mixed script contains fold syntax description"
        ((TESTS_PASSED++))
    else
        fail "Mixed script missing fold syntax description"
        ((TESTS_FAILED++))
    fi
    
    # Check complex multi-line in step 3
    if grep -q "# Step 3: Complex multi-line description" "$MIXED_SCRIPT"; then
        pass "Mixed script contains complex multi-line description"
        ((TESTS_PASSED++))
    else
        fail "Mixed script missing complex multi-line description"
        ((TESTS_FAILED++))
    fi
    
    if grep -q "# 1. Numbered items" "$MIXED_SCRIPT"; then
        pass "Mixed script preserves numbered list formatting"
        ((TESTS_PASSED++))
    else
        fail "Mixed script does not preserve numbered list formatting"
        ((TESTS_FAILED++))
    fi
fi

# Test 8: Execute generated scripts
section "Test 8: Execute Generated Scripts"

if [[ -f "$PIPE_SCRIPT" ]]; then
    PIPE_OUTPUT="$TEMP_DIR/pipe_output.txt"
    cd "$TEMP_DIR"
    if bash "$PIPE_SCRIPT" > "$PIPE_OUTPUT" 2>&1; then
        pass "Pipe syntax script executed successfully"
        ((TESTS_PASSED++))
    else
        fail "Pipe syntax script execution failed"
        info "Output: $(cat "$PIPE_OUTPUT" | head -30)"
        ((TESTS_FAILED++))
    fi
    cd "$PROJECT_ROOT"
fi

# Fold syntax execution skipped - handled by Rust E2E test
if [[ -n "$FOLD_SCRIPT" && -f "$FOLD_SCRIPT" ]]; then
    FOLD_OUTPUT="$TEMP_DIR/fold_output.txt"
    cd "$TEMP_DIR"
    if bash "$FOLD_SCRIPT" > "$FOLD_OUTPUT" 2>&1; then
        pass "Fold syntax script executed successfully"
        ((TESTS_PASSED++))
    else
        fail "Fold syntax script execution failed"
        info "Output: $(cat "$FOLD_OUTPUT" | head -30)"
        ((TESTS_FAILED++))
    fi
    cd "$PROJECT_ROOT"
else
    info "Fold syntax execution test skipped - handled by Rust E2E test"
fi

if [[ -f "$MIXED_SCRIPT" ]]; then
    MIXED_OUTPUT="$TEMP_DIR/mixed_output.txt"
    cd "$TEMP_DIR"
    if bash "$MIXED_SCRIPT" > "$MIXED_OUTPUT" 2>&1; then
        pass "Mixed syntax script executed successfully"
        ((TESTS_PASSED++))
    else
        fail "Mixed syntax script execution failed"
        info "Output: $(cat "$MIXED_OUTPUT" | head -30)"
        ((TESTS_FAILED++))
    fi
    cd "$PROJECT_ROOT"
fi

# Test 9: Verify JSON execution logs are generated
section "Test 9: Verify JSON Execution Logs"

PIPE_JSON="$TEMP_DIR/TEST_MULTILINE_PIPE_execution_log.json"
if [[ -f "$PIPE_JSON" ]]; then
    pass "Pipe syntax JSON log created"
    ((TESTS_PASSED++))
    
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$PIPE_JSON" >/dev/null 2>&1; then
            pass "Pipe syntax JSON log is valid"
            ((TESTS_PASSED++))
        else
            fail "Pipe syntax JSON log is invalid"
            ((TESTS_FAILED++))
        fi
        
        ENTRY_COUNT=$(jq 'length' "$PIPE_JSON")
        if [[ $ENTRY_COUNT -eq 2 ]]; then
            pass "Pipe syntax JSON log contains correct number of entries (2 steps)"
            ((TESTS_PASSED++))
        else
            fail "Pipe syntax JSON log has incorrect entry count: expected 2, got $ENTRY_COUNT"
            ((TESTS_FAILED++))
        fi
        
        FIRST_OUTPUT=$(jq -r '.[0].output' "$PIPE_JSON")
        if [[ "$FIRST_OUTPUT" == "hello world" ]]; then
            pass "Pipe syntax JSON log contains expected output for step 1"
            ((TESTS_PASSED++))
        else
            fail "Pipe syntax JSON log has incorrect output for step 1: '$FIRST_OUTPUT'"
            ((TESTS_FAILED++))
        fi
    else
        info "jq not available - skipping detailed JSON validation"
    fi
else
    fail "Pipe syntax JSON log not created"
    ((TESTS_FAILED++))
fi

# Fold syntax JSON log verification skipped - handled by Rust E2E test
FOLD_JSON="$TEMP_DIR/TEST_MULTILINE_FOLD_execution_log.json"
if [[ -f "$FOLD_JSON" ]]; then
    pass "Fold syntax JSON log created"
    ((TESTS_PASSED++))
    
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$FOLD_JSON" >/dev/null 2>&1; then
            pass "Fold syntax JSON log is valid"
            ((TESTS_PASSED++))
        else
            fail "Fold syntax JSON log is invalid"
            ((TESTS_FAILED++))
        fi
        
        ENTRY_COUNT=$(jq 'length' "$FOLD_JSON")
        if [[ $ENTRY_COUNT -eq 2 ]]; then
            pass "Fold syntax JSON log contains correct number of entries (2 steps)"
            ((TESTS_PASSED++))
        else
            fail "Fold syntax JSON log has incorrect entry count: expected 2, got $ENTRY_COUNT"
            ((TESTS_FAILED++))
        fi
    else
        info "jq not available - skipping detailed JSON validation"
    fi
else
    info "Fold syntax JSON log test skipped - handled by Rust E2E test"
fi

MIXED_JSON="$TEMP_DIR/TEST_MULTILINE_MIXED_execution_log.json"
if [[ -f "$MIXED_JSON" ]]; then
    pass "Mixed syntax JSON log created"
    ((TESTS_PASSED++))
    
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$MIXED_JSON" >/dev/null 2>&1; then
            pass "Mixed syntax JSON log is valid"
            ((TESTS_PASSED++))
        else
            fail "Mixed syntax JSON log is invalid"
            ((TESTS_FAILED++))
        fi
        
        ENTRY_COUNT=$(jq 'length' "$MIXED_JSON")
        if [[ $ENTRY_COUNT -eq 3 ]]; then
            pass "Mixed syntax JSON log contains correct number of entries (3 steps)"
            ((TESTS_PASSED++))
        else
            fail "Mixed syntax JSON log has incorrect entry count: expected 3, got $ENTRY_COUNT"
            ((TESTS_FAILED++))
        fi
        
        STEP1_OUTPUT=$(jq -r '.[0].output' "$MIXED_JSON")
        if [[ "$STEP1_OUTPUT" == "step one" ]]; then
            pass "Mixed syntax JSON log contains expected output for step 1"
            ((TESTS_PASSED++))
        else
            fail "Mixed syntax JSON log has incorrect output for step 1: '$STEP1_OUTPUT'"
            ((TESTS_FAILED++))
        fi
        
        STEP2_OUTPUT=$(jq -r '.[1].output' "$MIXED_JSON")
        if [[ "$STEP2_OUTPUT" == "step two" ]]; then
            pass "Mixed syntax JSON log contains expected output for step 2"
            ((TESTS_PASSED++))
        else
            fail "Mixed syntax JSON log has incorrect output for step 2: '$STEP2_OUTPUT'"
            ((TESTS_FAILED++))
        fi
        
        STEP3_OUTPUT=$(jq -r '.[2].output' "$MIXED_JSON")
        if [[ "$STEP3_OUTPUT" == "step three" ]]; then
            pass "Mixed syntax JSON log contains expected output for step 3"
            ((TESTS_PASSED++))
        else
            fail "Mixed syntax JSON log has incorrect output for step 3: '$STEP3_OUTPUT'"
            ((TESTS_FAILED++))
        fi
    else
        info "jq not available - skipping detailed JSON validation"
    fi
else
    fail "Mixed syntax JSON log not created"
    ((TESTS_FAILED++))
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
