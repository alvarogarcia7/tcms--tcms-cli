#!/bin/bash
#
# End-to-end integration test for validate-yaml with multi-file support
#
# This test validates:
# 1. Multiple valid YAML files (3-5) - verify exit code 0 and all success messages
# 2. Mix of valid and invalid files (2 valid + 2 invalid) - verify exit code 1,
#    all files processed, success markers for valid files, error markers for invalid files
# 3. Only invalid files - verify exit code 1 with all errors shown
# 4. Single file validation - ensure backward compatibility
# 5. Summary statistics accuracy in each scenario
#
# Usage: ./tests/integration/test_validate_yaml_multi_e2e.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
SCHEMA_FILE="$PROJECT_ROOT/schemas/tcms-schemas/tcms/test-case-main.schema.json"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1

# Find validate-yaml binary using workspace-aware search
cd "$PROJECT_ROOT"
VALIDATE_YAML_BIN=$(find_binary "validate-yaml")
if [[ -z "$VALIDATE_YAML_BIN" ]]; then
    echo "[ERROR] validate-yaml binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin validate-yaml" >&2
    exit 1
fi

# Color codes for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

echo "======================================"
echo "validate-yaml Multi-File E2E Integration Test"
echo "======================================"
echo ""

# Function to print test status
pass() {
    echo -e "${GREEN}✓${NC} $1"
    TESTS_PASSED=$((TESTS_PASSED+1))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED+1))
}

info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

section() {
    echo ""
    echo -e "${YELLOW}=== $1 ===${NC}"
}

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
trap 'rm -rf "$TEMP_DIR"' EXIT

info "Using temporary directory: $TEMP_DIR"

# Create test YAML files
section "Creating Test YAML Files"

# Valid YAML file 1
VALID_YAML_1="$TEMP_DIR/valid_1.yaml"
cat > "$VALID_YAML_1" << 'EOF'
requirement: VALID001
item: 1
tc: 1
id: VALID_TEST_1
description: First valid test case
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Valid Sequence 1
    description: Valid test sequence
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Echo test
        command: echo 'test1'
        expected:
          success: true
          result: "0"
          output: test1
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test1\" ]"
EOF

pass "Created valid_1.yaml"

# Valid YAML file 2
VALID_YAML_2="$TEMP_DIR/valid_2.yaml"
cat > "$VALID_YAML_2" << 'EOF'
requirement: VALID002
item: 2
tc: 2
id: VALID_TEST_2
description: Second valid test case
general_initial_conditions:
  System:
    - Initialized
initial_conditions:
  Device:
    - Ready
test_sequences:
  - id: 1
    name: Valid Sequence 2
    description: Another valid test sequence
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

pass "Created valid_2.yaml"

# Valid YAML file 3
VALID_YAML_3="$TEMP_DIR/valid_3.yaml"
cat > "$VALID_YAML_3" << 'EOF'
requirement: VALID003
item: 3
tc: 3
id: VALID_TEST_3
description: Third valid test case
general_initial_conditions:
  System:
    - Active
initial_conditions:
  Device:
    - Online
test_sequences:
  - id: 1
    name: Valid Sequence 3
    description: Third valid test sequence
    initial_conditions:
      LPA:
        - Online
    steps:
      - step: 1
        description: Echo another test
        command: echo 'test3'
        expected:
          success: true
          result: "0"
          output: test3
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test3\" ]"
EOF

pass "Created valid_3.yaml"

# Valid YAML file 4
VALID_YAML_4="$TEMP_DIR/valid_4.yaml"
cat > "$VALID_YAML_4" << 'EOF'
requirement: VALID004
item: 4
tc: 4
id: VALID_TEST_4
description: Fourth valid test case
general_initial_conditions:
  System:
    - Running
initial_conditions:
  Device:
    - Available
test_sequences:
  - id: 1
    name: Valid Sequence 4
    description: Fourth valid test sequence
    initial_conditions:
      LPA:
        - Available
    steps:
      - step: 1
        description: Test command
        command: echo 'test4'
        expected:
          success: true
          result: "0"
          output: test4
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test4\" ]"
EOF

pass "Created valid_4.yaml"

# Valid YAML file 5
VALID_YAML_5="$TEMP_DIR/valid_5.yaml"
cat > "$VALID_YAML_5" << 'EOF'
requirement: VALID005
item: 5
tc: 5
id: VALID_TEST_5
description: Fifth valid test case
general_initial_conditions:
  System:
    - Operational
initial_conditions:
  Device:
    - Standby
test_sequences:
  - id: 1
    name: Valid Sequence 5
    description: Fifth valid test sequence
    initial_conditions:
      LPA:
        - Standby
    steps:
      - step: 1
        description: Final test
        command: echo 'test5'
        expected:
          success: true
          result: "0"
          output: test5
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "[ \"$COMMAND_OUTPUT\" = \"test5\" ]"
EOF

pass "Created valid_5.yaml"

# Invalid YAML file 1 - Missing required field
INVALID_YAML_1="$TEMP_DIR/invalid_1.yaml"
cat > "$INVALID_YAML_1" << 'EOF'
requirement: INVALID001
item: 1
tc: 1
description: Missing id field
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Invalid Sequence 1
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Test step
        command: echo 'test'
        expected:
          success: true
          result: "0"
          output: test
EOF

pass "Created invalid_1.yaml (missing id field)"

# Invalid YAML file 2 - Wrong data type
INVALID_YAML_2="$TEMP_DIR/invalid_2.yaml"
cat > "$INVALID_YAML_2" << 'EOF'
requirement: INVALID002
item: "not_an_integer"
tc: 2
id: INVALID_TEST_2
description: Invalid item field (wrong type)
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Invalid Sequence 2
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Test step
        command: echo 'test'
        expected:
          success: true
          result: "0"
          output: test
EOF

pass "Created invalid_2.yaml (wrong data type)"

# Invalid YAML file 3 - Malformed YAML syntax
INVALID_YAML_3="$TEMP_DIR/invalid_3.yaml"
cat > "$INVALID_YAML_3" << 'EOF'
requirement: INVALID003
item: 3
tc: 3
id: INVALID_TEST_3
description: Malformed YAML
general_initial_conditions:
  System:
    - Ready
initial_conditions
  Device:
    - Connected
test_sequences:
  - id: 1
    name: Invalid Sequence 3
    initial_conditions:
      LPA:
        - Active
    steps:
      - step: 1
        description: Test step
        command: echo 'test'
EOF

pass "Created invalid_3.yaml (malformed syntax)"

# Invalid YAML file 4 - Missing required array field
INVALID_YAML_4="$TEMP_DIR/invalid_4.yaml"
cat > "$INVALID_YAML_4" << 'EOF'
requirement: INVALID004
item: 4
tc: 4
id: INVALID_TEST_4
description: Missing test_sequences
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  Device:
    - Connected
EOF

pass "Created invalid_4.yaml (missing test_sequences)"

# Scenario 1: Validate 3-5 valid YAML files
section "Scenario 1: Validate Multiple Valid Files (5 files)"

OUTPUT_1="$TEMP_DIR/output_scenario_1.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VALID_YAML_1" "$VALID_YAML_2" "$VALID_YAML_3" "$VALID_YAML_4" "$VALID_YAML_5" > "$OUTPUT_1" 2>&1; then
    EXIT_CODE_1=0
    pass "Exit code is 0 for all valid files"
else
    EXIT_CODE_1=$?
    fail "Exit code is $EXIT_CODE_1 (expected 0)"
fi

# Check for success markers
if grep -q "✓.*valid_1.yaml" "$OUTPUT_1"; then
    pass "Success marker found for valid_1.yaml"
else
    fail "Success marker missing for valid_1.yaml"
fi

if grep -q "✓.*valid_2.yaml" "$OUTPUT_1"; then
    pass "Success marker found for valid_2.yaml"
else
    fail "Success marker missing for valid_2.yaml"
fi

if grep -q "✓.*valid_3.yaml" "$OUTPUT_1"; then
    pass "Success marker found for valid_3.yaml"
else
    fail "Success marker missing for valid_3.yaml"
fi

if grep -q "✓.*valid_4.yaml" "$OUTPUT_1"; then
    pass "Success marker found for valid_4.yaml"
else
    fail "Success marker missing for valid_4.yaml"
fi

if grep -q "✓.*valid_5.yaml" "$OUTPUT_1"; then
    pass "Success marker found for valid_5.yaml"
else
    fail "Success marker missing for valid_5.yaml"
fi

# Check summary statistics
if grep -q "Total files validated: 5" "$OUTPUT_1"; then
    pass "Total files count is correct (5)"
else
    fail "Total files count is incorrect"
fi

if grep -q "Passed: 5" "$OUTPUT_1"; then
    pass "Passed count is correct (5)"
else
    fail "Passed count is incorrect"
fi

if grep -q "Failed: 0" "$OUTPUT_1"; then
    pass "Failed count is correct (0)"
else
    fail "Failed count is incorrect"
fi

# Scenario 2: Mix of valid and invalid files (2 valid + 2 invalid)
section "Scenario 2: Mix of Valid and Invalid Files (2 valid + 2 invalid)"

OUTPUT_2="$TEMP_DIR/output_scenario_2.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VALID_YAML_1" "$INVALID_YAML_1" "$VALID_YAML_2" "$INVALID_YAML_2" > "$OUTPUT_2" 2>&1; then
    EXIT_CODE_2=0
    fail "Exit code is 0 (expected 1 for mixed results)"
else
    EXIT_CODE_2=$?
    pass "Exit code is non-zero ($EXIT_CODE_2) for mixed results"
fi

# Verify exit code is 1
if [[ $EXIT_CODE_2 -eq 1 ]]; then
    pass "Exit code is exactly 1"
else
    fail "Exit code is $EXIT_CODE_2 (expected 1)"
fi

# Check all files are processed
if grep -q "valid_1.yaml" "$OUTPUT_2"; then
    pass "valid_1.yaml was processed"
else
    fail "valid_1.yaml was not processed"
fi

if grep -q "invalid_1.yaml" "$OUTPUT_2"; then
    pass "invalid_1.yaml was processed"
else
    fail "invalid_1.yaml was not processed"
fi

if grep -q "valid_2.yaml" "$OUTPUT_2"; then
    pass "valid_2.yaml was processed"
else
    fail "valid_2.yaml was not processed"
fi

if grep -q "invalid_2.yaml" "$OUTPUT_2"; then
    pass "invalid_2.yaml was processed"
else
    fail "invalid_2.yaml was not processed"
fi

# Check for success markers on valid files
if grep -q "✓.*valid_1.yaml" "$OUTPUT_2"; then
    pass "Success marker found for valid_1.yaml"
else
    fail "Success marker missing for valid_1.yaml"
fi

if grep -q "✓.*valid_2.yaml" "$OUTPUT_2"; then
    pass "Success marker found for valid_2.yaml"
else
    fail "Success marker missing for valid_2.yaml"
fi

# Check for error markers on invalid files
if grep -q "✗.*invalid_1.yaml" "$OUTPUT_2"; then
    pass "Error marker found for invalid_1.yaml"
else
    fail "Error marker missing for invalid_1.yaml"
fi

if grep -q "✗.*invalid_2.yaml" "$OUTPUT_2"; then
    pass "Error marker found for invalid_2.yaml"
else
    fail "Error marker missing for invalid_2.yaml"
fi

# Check summary statistics
if grep -q "Total files validated: 4" "$OUTPUT_2"; then
    pass "Total files count is correct (4)"
else
    fail "Total files count is incorrect"
fi

if grep -q "Passed: 2" "$OUTPUT_2"; then
    pass "Passed count is correct (2)"
else
    fail "Passed count is incorrect"
fi

if grep -q "Failed: 2" "$OUTPUT_2"; then
    pass "Failed count is correct (2)"
else
    fail "Failed count is incorrect"
fi

# Scenario 3: Only invalid files
section "Scenario 3: Only Invalid Files"

OUTPUT_3="$TEMP_DIR/output_scenario_3.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$INVALID_YAML_1" "$INVALID_YAML_2" "$INVALID_YAML_3" > "$OUTPUT_3" 2>&1; then
    EXIT_CODE_3=0
    fail "Exit code is 0 (expected 1 for all invalid files)"
else
    EXIT_CODE_3=$?
    pass "Exit code is non-zero ($EXIT_CODE_3) for all invalid files"
fi

# Verify exit code is 1
if [[ $EXIT_CODE_3 -eq 1 ]]; then
    pass "Exit code is exactly 1"
else
    fail "Exit code is $EXIT_CODE_3 (expected 1)"
fi

# Check for error markers on all files
if grep -q "✗.*invalid_1.yaml" "$OUTPUT_3"; then
    pass "Error marker found for invalid_1.yaml"
else
    fail "Error marker missing for invalid_1.yaml"
fi

if grep -q "✗.*invalid_2.yaml" "$OUTPUT_3"; then
    pass "Error marker found for invalid_2.yaml"
else
    fail "Error marker missing for invalid_2.yaml"
fi

if grep -q "✗.*invalid_3.yaml" "$OUTPUT_3"; then
    pass "Error marker found for invalid_3.yaml"
else
    fail "Error marker missing for invalid_3.yaml"
fi

# Verify error messages are shown
if grep -q -i "error\|fail\|constraint\|parse" "$OUTPUT_3"; then
    pass "Error messages are displayed"
else
    fail "Error messages are missing"
fi

# Check summary statistics
if grep -q "Total files validated: 3" "$OUTPUT_3"; then
    pass "Total files count is correct (3)"
else
    fail "Total files count is incorrect"
fi

if grep -q "Passed: 0" "$OUTPUT_3"; then
    pass "Passed count is correct (0)"
else
    fail "Passed count is incorrect"
fi

if grep -q "Failed: 3" "$OUTPUT_3"; then
    pass "Failed count is correct (3)"
else
    fail "Failed count is incorrect"
fi

# Scenario 4: Single file validation (backward compatibility)
section "Scenario 4: Single File Validation (Backward Compatibility)"

# Test with valid single file
OUTPUT_4A="$TEMP_DIR/output_scenario_4a.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$VALID_YAML_1" > "$OUTPUT_4A" 2>&1; then
    EXIT_CODE_4A=0
    pass "Exit code is 0 for single valid file"
else
    EXIT_CODE_4A=$?
    fail "Exit code is $EXIT_CODE_4A (expected 0) for single valid file"
fi

if grep -q "✓.*valid_1.yaml" "$OUTPUT_4A"; then
    pass "Success marker found for single valid file"
else
    fail "Success marker missing for single valid file"
fi

if grep -q "Total files validated: 1" "$OUTPUT_4A"; then
    pass "Total files count is correct (1)"
else
    fail "Total files count is incorrect"
fi

if grep -q "Passed: 1" "$OUTPUT_4A"; then
    pass "Passed count is correct (1)"
else
    fail "Passed count is incorrect"
fi

if grep -q "Failed: 0" "$OUTPUT_4A"; then
    pass "Failed count is correct (0)"
else
    fail "Failed count is incorrect"
fi

# Test with invalid single file
OUTPUT_4B="$TEMP_DIR/output_scenario_4b.txt"
if "$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$INVALID_YAML_1" > "$OUTPUT_4B" 2>&1; then
    EXIT_CODE_4B=0
    fail "Exit code is 0 (expected 1) for single invalid file"
else
    EXIT_CODE_4B=$?
    pass "Exit code is non-zero ($EXIT_CODE_4B) for single invalid file"
fi

if grep -q "✗.*invalid_1.yaml" "$OUTPUT_4B"; then
    pass "Error marker found for single invalid file"
else
    fail "Error marker missing for single invalid file"
fi

if grep -q "Total files validated: 1" "$OUTPUT_4B"; then
    pass "Total files count is correct (1)"
else
    fail "Total files count is incorrect"
fi

if grep -q "Passed: 0" "$OUTPUT_4B"; then
    pass "Passed count is correct (0)"
else
    fail "Passed count is incorrect"
fi

if grep -q "Failed: 1" "$OUTPUT_4B"; then
    pass "Failed count is correct (1)"
else
    fail "Failed count is incorrect"
fi

# Scenario 5: Verify detailed error messages for invalid files
section "Scenario 5: Verify Detailed Error Messages"

OUTPUT_5="$TEMP_DIR/output_scenario_5.txt"
"$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$INVALID_YAML_1" "$INVALID_YAML_2" "$INVALID_YAML_3" "$INVALID_YAML_4" > "$OUTPUT_5" 2>&1 || true

# Check for detailed error information for missing field
if grep -A 10 "invalid_1.yaml" "$OUTPUT_5" | grep -q -i "constraint\|required\|missing"; then
    pass "Detailed error shown for invalid_1.yaml (missing field)"
else
    fail "Missing detailed error for invalid_1.yaml"
fi

# Check for detailed error information for wrong type
if grep -A 10 "invalid_2.yaml" "$OUTPUT_5" | grep -q -i "constraint\|type\|integer"; then
    pass "Detailed error shown for invalid_2.yaml (wrong type)"
else
    fail "Missing detailed error for invalid_2.yaml"
fi

# Check for detailed error information for malformed YAML
if grep -A 10 "invalid_3.yaml" "$OUTPUT_5" | grep -q -i "parse\|yaml\|syntax"; then
    pass "Detailed error shown for invalid_3.yaml (malformed syntax)"
else
    fail "Missing detailed error for invalid_3.yaml"
fi

# Check for detailed error information for missing required array
if grep -A 10 "invalid_4.yaml" "$OUTPUT_5" | grep -q -i "constraint\|required\|test_sequences"; then
    pass "Detailed error shown for invalid_4.yaml (missing required field)"
else
    fail "Missing detailed error for invalid_4.yaml"
fi

# Check summary for all invalid files
if grep -q "Total files validated: 4" "$OUTPUT_5"; then
    pass "Total files count is correct (4)"
else
    fail "Total files count is incorrect"
fi

if grep -q "Passed: 0" "$OUTPUT_5"; then
    pass "Passed count is correct (0)"
else
    fail "Passed count is incorrect"
fi

if grep -q "Failed: 4" "$OUTPUT_5"; then
    pass "Failed count is correct (4)"
else
    fail "Failed count is incorrect"
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
