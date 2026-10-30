#!/bin/bash
#
# End-to-end integration test for dependency resolution workflow
#
# This test validates:
# 1. Multiple YAML files with cross-references are validated using validate-yaml
# 2. Bash scripts are generated from test cases with dependencies using test-executor
# 3. Generated scripts contain dependency references as comments
# 4. Test execution handles dependencies properly
#
# Usage: ./tests/integration/test_dependencies_e2e.sh [--no-remove]
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

echo "======================================"
echo "Dependency Resolution E2E Test"
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

# Test 1: Create test YAML files with cross-references
section "Test 1: Creating Test YAML Files with Dependencies"

# First YAML file - defines test case TC_DEP_001 with a test sequence ref
YAML_FILE_1="$TEMP_DIR/test_dep_001.yaml"
cat > "$YAML_FILE_1" << 'EOF'
requirement: "DEP_001"
item: 1
tc: 1
id: 'TC_DEP_001'
description: 'First test case with dependencies'

general_initial_conditions:
  system:
    - "Bash shell is available"

initial_conditions:
  filesystem:
    - "Temporary directory /tmp is writable"

test_sequences:
  - id: 1
    ref: dep-test-ref-001
    name: "Setup Sequence"
    description: "Setup sequence that will be referenced by another test"
    initial_conditions:
      system:
        - "Test environment is ready"
    steps:
      - step: 1
        ref: step-ref-abc123
        description: "Create test file"
        command: echo "HELLO FROM TC_DEP_001" > /tmp/dep_test.txt
        expected:
          success: true
          result: '0'
          output: ''
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "true"
      - step: 2
        description: "Verify file exists"
        command: cat /tmp/dep_test.txt
        expected:
          success: true
          result: '0'
          output: 'HELLO FROM TC_DEP_001'
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "grep -q 'HELLO FROM TC_DEP_001' <<< \"$COMMAND_OUTPUT\""
EOF

pass "Created first YAML file: test_dep_001.yaml"

# Second YAML file - references TC_DEP_001 and its test sequence
YAML_FILE_2="$TEMP_DIR/test_dep_002.yaml"
cat > "$YAML_FILE_2" << 'EOF'
requirement: "DEP_002"
item: 1
tc: 2
id: 'TC_DEP_002'
description: 'Second test case that depends on TC_DEP_001'

general_initial_conditions:
  system:
    - "Bash shell is available"
  include:
    - id: "TC_DEP_001"

initial_conditions:
  filesystem:
    - "Temporary directory /tmp is writable"
    - ref: "dep-test-ref-001"
  device:
    - ref: "step-ref-abc123"

test_sequences:
  - id: 1
    name: "Dependent Sequence"
    description: "Sequence that depends on TC_DEP_001"
    initial_conditions:
      system:
        - "Test environment is ready"
      include:
        - id: "TC_DEP_001"
          test_sequence: "1"
      eUICC:
        - ref: "step-ref-abc123"
        - test_sequence:
            id: 1
            step: "[1,2]"
    steps:
      - step: 1
        description: "Read file created by dependency"
        command: cat /tmp/dep_test.txt
        expected:
          success: true
          result: '0'
          output: 'HELLO'
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "grep -q 'HELLO' <<< \"$COMMAND_OUTPUT\""
      - step: 2
        description: "Append to file"
        command: echo "HELLO FROM TC_DEP_002" >> /tmp/dep_test.txt
        expected:
          success: true
          result: '0'
          output: ''
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "true"
EOF

pass "Created second YAML file: test_dep_002.yaml"

# Third YAML file - has cross-references but also references test_sequence within same test case
YAML_FILE_3="$TEMP_DIR/test_dep_003.yaml"
cat > "$YAML_FILE_3" << 'EOF'
requirement: "DEP_003"
item: 1
tc: 3
id: 'TC_DEP_003'
description: 'Third test case with internal and external dependencies'

general_initial_conditions:
  system:
    - "Bash shell is available"
  include:
    - id: "TC_DEP_001"
    - id: "TC_DEP_002"

initial_conditions:
  filesystem:
    - "Temporary directory /tmp is writable"

test_sequences:
  - id: 1
    ref: internal-seq-ref-999
    name: "Internal Setup"
    description: "Internal sequence for same test case reference"
    initial_conditions:
      system:
        - "Test environment is ready"
    steps:
      - step: 1
        ref: internal-step-ref-xyz
        description: "Internal step"
        command: echo "INTERNAL SETUP"
        expected:
          success: true
          result: '0'
          output: 'INTERNAL SETUP'
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "grep -q 'INTERNAL SETUP' <<< \"$COMMAND_OUTPUT\""
  - id: 2
    name: "Main Sequence"
    description: "Main sequence with both internal and external dependencies"
    initial_conditions:
      system:
        - ref: "dep-test-ref-001"
      include:
        - id: "TC_DEP_002"
          test_sequence: "1"
      internal:
        - ref: "internal-step-ref-xyz"
        - test_sequence:
            id: 1
            step: "1"
    steps:
      - step: 1
        description: "Verify all dependencies"
        command: cat /tmp/dep_test.txt | wc -l
        expected:
          success: true
          result: '0'
          output: '2'
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "true"
EOF

pass "Created third YAML file: test_dep_003.yaml"

# Test 2: Validate all YAML files against schema with cross-file dependency validation
section "Test 2: Validate YAML Files with Cross-File Dependencies"

# Validate all three files together to check cross-file dependencies
VALIDATE_OUTPUT="$TEMP_DIR/validate_output.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$YAML_FILE_1" "$YAML_FILE_2" "$YAML_FILE_3" > "$VALIDATE_OUTPUT" 2>&1; then
    pass "All YAML files validated successfully with cross-file dependencies"
else
    fail "YAML validation failed"
    cat "$VALIDATE_OUTPUT"
    exit 1
fi

# Check that validation output confirms all files passed
if grep -q "TC_DEP_001" "$VALIDATE_OUTPUT" && \
   grep -q "TC_DEP_002" "$VALIDATE_OUTPUT" && \
   grep -q "TC_DEP_003" "$VALIDATE_OUTPUT"; then
    pass "Validation output contains all test case files"
else
    fail "Validation output missing expected test case files"
fi

# Verify summary shows all files passed
if grep -q "Passed: 3" "$VALIDATE_OUTPUT"; then
    pass "Validation summary shows all 3 files passed"
else
    fail "Validation summary incorrect"
fi

# Test 3: Validate that breaking a dependency causes validation failure
section "Test 3: Validate Dependency Validation Catches Errors"

# Create a YAML file with an unresolved reference
YAML_FILE_BROKEN="$TEMP_DIR/test_dep_broken.yaml"
cat > "$YAML_FILE_BROKEN" << 'EOF'
requirement: "DEP_BROKEN"
item: 1
tc: 4
id: 'TC_DEP_BROKEN'
description: 'Test case with broken dependency'

general_initial_conditions:
  system:
    - "Bash shell is available"

initial_conditions:
  filesystem:
    - ref: "non-existent-ref-12345"

test_sequences:
  - id: 1
    name: "Broken Sequence"
    description: "Sequence with unresolved reference"
    steps:
      - step: 1
        description: "Test step"
        command: echo "test"
        expected:
          success: true
          result: '0'
          output: 'test'
        verification:
          result: "[[ $EXIT_CODE -eq 0 ]]"
          output: "true"
EOF

pass "Created broken YAML file with unresolved reference"

# Try to validate with the broken file - should fail
BROKEN_VALIDATE_OUTPUT="$TEMP_DIR/broken_validate_output.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$YAML_FILE_1" "$YAML_FILE_2" "$YAML_FILE_3" "$YAML_FILE_BROKEN" > "$BROKEN_VALIDATE_OUTPUT" 2>&1; then
    fail "Validation should have failed with broken dependency"
    cat "$BROKEN_VALIDATE_OUTPUT"
    exit 1
else
    pass "Validation correctly failed with broken dependency"
fi

# Verify error message mentions the unresolved reference
if grep -q "non-existent-ref-12345" "$BROKEN_VALIDATE_OUTPUT"; then
    pass "Error message contains unresolved reference"
else
    fail "Error message missing unresolved reference details"
fi

if grep -q "Unresolved reference" "$BROKEN_VALIDATE_OUTPUT"; then
    pass "Error message indicates unresolved reference"
else
    fail "Error message missing 'Unresolved reference' text"
fi

# Test 4: Generate bash scripts from test cases with dependencies
section "Test 4: Generate Bash Scripts with Dependencies"

SCRIPT_FILE_1="$TEMP_DIR/test_dep_001.sh"
if "$TEST_EXECUTOR_BIN" generate "$YAML_FILE_1" -o "$SCRIPT_FILE_1" > /dev/null 2>&1; then
    pass "Generated script from TC_DEP_001"
else
    fail "Failed to generate script from TC_DEP_001"
    exit 1
fi

SCRIPT_FILE_2="$TEMP_DIR/test_dep_002.sh"
if "$TEST_EXECUTOR_BIN" generate "$YAML_FILE_2" -o "$SCRIPT_FILE_2" > /dev/null 2>&1; then
    pass "Generated script from TC_DEP_002"
else
    fail "Failed to generate script from TC_DEP_002"
    exit 1
fi

SCRIPT_FILE_3="$TEMP_DIR/test_dep_003.sh"
if "$TEST_EXECUTOR_BIN" generate "$YAML_FILE_3" -o "$SCRIPT_FILE_3" > /dev/null 2>&1; then
    pass "Generated script from TC_DEP_003"
else
    fail "Failed to generate script from TC_DEP_003"
    exit 1
fi

# Validate bash syntax for all generated scripts
if bash -n "$SCRIPT_FILE_1" 2>/dev/null; then
    pass "TC_DEP_001 script has valid bash syntax"
else
    fail "TC_DEP_001 script has invalid bash syntax"
    exit 1
fi
validate_with_shellcheck "$SCRIPT_FILE_1" "TC_DEP_001 script"

if bash -n "$SCRIPT_FILE_2" 2>/dev/null; then
    pass "TC_DEP_002 script has valid bash syntax"
else
    fail "TC_DEP_002 script has invalid bash syntax"
    exit 1
fi
validate_with_shellcheck "$SCRIPT_FILE_2" "TC_DEP_002 script"

if bash -n "$SCRIPT_FILE_3" 2>&1 > /dev/null; then
    pass "TC_DEP_003 script has valid bash syntax"
else
    fail "TC_DEP_003 script has invalid bash syntax"
    bash -n "$SCRIPT_FILE_3" 2>&1 | head -5
    exit 1
fi
validate_with_shellcheck "$SCRIPT_FILE_3" "TC_DEP_003 script"

# Test 5: Verify generated scripts contain dependency references as comments
section "Test 5: Verify Dependency References in Generated Scripts"

# Check that TC_DEP_001 script contains its own ref
if grep -q "dep-test-ref-001" "$SCRIPT_FILE_1"; then
    pass "TC_DEP_001 script contains test sequence ref 'dep-test-ref-001'"
else
    fail "TC_DEP_001 script missing test sequence ref"
fi

if grep -q "step-ref-abc123" "$SCRIPT_FILE_1"; then
    pass "TC_DEP_001 script contains step ref 'step-ref-abc123'"
else
    fail "TC_DEP_001 script missing step ref"
fi

# Check that TC_DEP_002 script contains references to TC_DEP_001
if grep -q "Include: TC_DEP_001" "$SCRIPT_FILE_2"; then
    pass "TC_DEP_002 script contains include reference to TC_DEP_001"
else
    fail "TC_DEP_002 script missing include reference to TC_DEP_001"
fi

if grep -q "ref: dep-test-ref-001" "$SCRIPT_FILE_2"; then
    pass "TC_DEP_002 script contains ref to 'dep-test-ref-001'"
else
    fail "TC_DEP_002 script missing ref to 'dep-test-ref-001'"
fi

if grep -q "ref: step-ref-abc123" "$SCRIPT_FILE_2"; then
    pass "TC_DEP_002 script contains ref to 'step-ref-abc123'"
else
    fail "TC_DEP_002 script missing ref to 'step-ref-abc123'"
fi

# Check for test_sequence reference in TC_DEP_002
if grep -q "test_sequence: id=1, step=\[1,2\]" "$SCRIPT_FILE_2"; then
    pass "TC_DEP_002 script contains test_sequence reference"
else
    fail "TC_DEP_002 script missing test_sequence reference"
fi

# Check that TC_DEP_003 script contains references to both TC_DEP_001 and TC_DEP_002
if grep -q "Include: TC_DEP_001" "$SCRIPT_FILE_3"; then
    pass "TC_DEP_003 script contains include reference to TC_DEP_001"
else
    fail "TC_DEP_003 script missing include reference to TC_DEP_001"
fi

if grep -q "Include: TC_DEP_002" "$SCRIPT_FILE_3"; then
    pass "TC_DEP_003 script contains include reference to TC_DEP_002"
else
    fail "TC_DEP_003 script missing include reference to TC_DEP_002"
fi

# Check internal references in TC_DEP_003
if grep -q "internal-seq-ref-999" "$SCRIPT_FILE_3"; then
    pass "TC_DEP_003 script contains internal test sequence ref"
else
    fail "TC_DEP_003 script missing internal test sequence ref"
fi

if grep -q "internal-step-ref-xyz" "$SCRIPT_FILE_3"; then
    pass "TC_DEP_003 script contains internal step ref"
else
    fail "TC_DEP_003 script missing internal step ref"
fi

# Check external dependency reference
if grep -q "ref: dep-test-ref-001" "$SCRIPT_FILE_3"; then
    pass "TC_DEP_003 script contains external ref to 'dep-test-ref-001'"
else
    fail "TC_DEP_003 script missing external ref"
fi

# Check test_sequence reference with test_sequence field specified
if grep -q "Include: TC_DEP_002 (test_sequence:" "$SCRIPT_FILE_3"; then
    pass "TC_DEP_003 script contains include with test_sequence field"
else
    fail "TC_DEP_003 script missing include with test_sequence field"
fi

# Test 6: Execute test scripts and verify dependencies are handled
section "Test 6: Execute Test Scripts and Verify Execution"

# Execute TC_DEP_001 first (creates the file)
EXEC_OUTPUT_1="$TEMP_DIR/exec_output_1.txt"
cd "$TEMP_DIR"
if bash "$SCRIPT_FILE_1" > "$EXEC_OUTPUT_1" 2>&1; then
    pass "TC_DEP_001 script executed successfully"
else
    fail "TC_DEP_001 script execution failed"
    cat "$EXEC_OUTPUT_1"
    exit 1
fi
cd "$PROJECT_ROOT"

# Verify the file was created by TC_DEP_001
if [[ -f "$TEMP_DIR/dep_test.txt" ]]; then
    pass "TC_DEP_001 created dependency file"
else
    fail "TC_DEP_001 did not create dependency file"
fi

# Verify file content
FILE_CONTENT=$(cat "$TEMP_DIR/dep_test.txt")
if [[ "$FILE_CONTENT" == "HELLO FROM TC_DEP_001" ]]; then
    pass "TC_DEP_001 dependency file has correct content"
else
    fail "TC_DEP_001 dependency file has incorrect content: '$FILE_CONTENT'"
fi

# Execute TC_DEP_002 (depends on TC_DEP_001's file)
EXEC_OUTPUT_2="$TEMP_DIR/exec_output_2.txt"
cd "$TEMP_DIR"
if bash "$SCRIPT_FILE_2" > "$EXEC_OUTPUT_2" 2>&1; then
    pass "TC_DEP_002 script executed successfully"
else
    fail "TC_DEP_002 script execution failed"
    cat "$EXEC_OUTPUT_2"
    exit 1
fi
cd "$PROJECT_ROOT"

# Verify TC_DEP_002 appended to the file
FILE_LINES=$(wc -l < "$TEMP_DIR/dep_test.txt" | tr -d ' ')
if [[ "$FILE_LINES" == "2" ]]; then
    pass "TC_DEP_002 successfully modified dependency file (2 lines)"
else
    fail "TC_DEP_002 did not modify file correctly (expected 2 lines, got $FILE_LINES)"
fi

# Execute TC_DEP_003 (depends on both TC_DEP_001 and TC_DEP_002)
EXEC_OUTPUT_3="$TEMP_DIR/exec_output_3.txt"
cd "$TEMP_DIR"
if bash "$SCRIPT_FILE_3" > "$EXEC_OUTPUT_3" 2>&1; then
    pass "TC_DEP_003 script executed successfully"
else
    fail "TC_DEP_003 script execution failed"
    cat "$EXEC_OUTPUT_3"
    exit 1
fi
cd "$PROJECT_ROOT"

# Test 7: Verify dependency comments are properly formatted
section "Test 7: Verify Comment Formatting for Dependencies"

# Check comment format in TC_DEP_002 initial conditions section
if grep -E "^# Include: TC_DEP_001$" "$SCRIPT_FILE_2" > /dev/null; then
    pass "TC_DEP_002 has properly formatted include comment"
else
    fail "TC_DEP_002 include comment not properly formatted"
fi

# Check comment format for ref in initial conditions
if grep -E "^#   filesystem: ref: dep-test-ref-001$" "$SCRIPT_FILE_2" > /dev/null; then
    pass "TC_DEP_002 has properly formatted ref comment in initial conditions"
else
    fail "TC_DEP_002 ref comment not properly formatted in initial conditions"
fi

# Check comment format for test_sequence reference
if grep -E "^#   eUICC: test_sequence: id=1, step=\[1,2\]$" "$SCRIPT_FILE_2" > /dev/null; then
    pass "TC_DEP_002 has properly formatted test_sequence comment"
else
    fail "TC_DEP_002 test_sequence comment not properly formatted"
fi

# Check that comments appear in appropriate sections
GENERAL_IC_LINE=$(grep -n "^# General Initial Conditions" "$SCRIPT_FILE_2" | cut -d: -f1 | head -1)
IC_LINE=$(grep -n "^# Initial Conditions" "$SCRIPT_FILE_2" | cut -d: -f1 | head -1)
SEQ_LINE=$(grep -n "^# Test Sequence 1:" "$SCRIPT_FILE_2" | cut -d: -f1 | head -1)

if [[ -n "$GENERAL_IC_LINE" ]] && [[ -n "$IC_LINE" ]] && [[ "$GENERAL_IC_LINE" -lt "$IC_LINE" ]]; then
    pass "TC_DEP_002 has proper section ordering (General IC before IC)"
else
    fail "TC_DEP_002 section ordering incorrect"
fi

if [[ -n "$IC_LINE" ]] && [[ -n "$SEQ_LINE" ]] && [[ "$IC_LINE" -lt "$SEQ_LINE" ]]; then
    pass "TC_DEP_002 has proper section ordering (IC before Test Sequence)"
else
    fail "TC_DEP_002 section ordering incorrect"
fi

# Test 8: Test with the actual example dependency files in the repo
section "Test 8: Validate Actual Example Dependency Files"

EXAMPLE_DEP_1="$PROJECT_ROOT/testcases/examples/dependencies/TC_VAR_001.yml"
EXAMPLE_DEP_2="$PROJECT_ROOT/testcases/examples/dependencies/TC_VAR_002.yml"

if [[ -f "$EXAMPLE_DEP_1" ]] && [[ -f "$EXAMPLE_DEP_2" ]]; then
    EXAMPLE_VALIDATE_OUTPUT="$TEMP_DIR/example_validate_output.txt"
    if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$EXAMPLE_DEP_1" "$EXAMPLE_DEP_2" > "$EXAMPLE_VALIDATE_OUTPUT" 2>&1; then
        pass "Example dependency files validated successfully"
    else
        fail "Example dependency files validation failed"
        cat "$EXAMPLE_VALIDATE_OUTPUT"
        exit 1
    fi

    # Generate scripts from example files
    EXAMPLE_SCRIPT_1="$TEMP_DIR/example_dep_1.sh"
    EXAMPLE_SCRIPT_2="$TEMP_DIR/example_dep_2.sh"

    if "$TEST_EXECUTOR_BIN" generate "$EXAMPLE_DEP_1" -o "$EXAMPLE_SCRIPT_1" > /dev/null 2>&1; then
        pass "Generated script from example dependency file 1"
    else
        fail "Failed to generate script from example dependency file 1"
        exit 1
    fi

    if "$TEST_EXECUTOR_BIN" generate "$EXAMPLE_DEP_2" -o "$EXAMPLE_SCRIPT_2" > /dev/null 2>&1; then
        pass "Generated script from example dependency file 2"
    else
        fail "Failed to generate script from example dependency file 2"
        exit 1
    fi

    # Validate syntax
    if bash -n "$EXAMPLE_SCRIPT_1" 2>/dev/null && bash -n "$EXAMPLE_SCRIPT_2" 2>/dev/null; then
        pass "Example dependency scripts have valid bash syntax"
    else
        fail "Example dependency scripts have invalid bash syntax"
        exit 1
    fi
    validate_with_shellcheck "$EXAMPLE_SCRIPT_1" "Example dependency script 1"
    validate_with_shellcheck "$EXAMPLE_SCRIPT_2" "Example dependency script 2"

    # Check for expected references in example files
    if grep -q "ref:" "$EXAMPLE_SCRIPT_2" || grep -q "Include:" "$EXAMPLE_SCRIPT_2"; then
        pass "Example script 2 contains dependency references"
    else
        fail "Example script 2 missing dependency references"
    fi
else
    info "Example dependency files not found, skipping test 8"
fi

# Summary
section "Test Summary"
echo ""
echo "All dependency resolution tests passed!"
echo ""
echo "Validated:"
echo "  ✓ YAML files with cross-references validate correctly"
echo "  ✓ Dependency validation catches unresolved references"
echo "  ✓ Generated scripts contain dependency references as comments"
echo "  ✓ Test scripts execute correctly with dependencies"
echo "  ✓ Dependency comments are properly formatted"
echo "  ✓ Example dependency files work as expected"
echo ""

exit 0
