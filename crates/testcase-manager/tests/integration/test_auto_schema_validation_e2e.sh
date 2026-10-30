#!/usr/bin/env bash
set -euo pipefail

# Integration test for auto-schema validation functionality
# Tests validate-yaml and validate-json binaries with auto-resolution of schemas

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Temporary directory for test files
TEST_DIR=""

# Cleanup function
cleanup() {
    if [[ -n "${TEST_DIR}" && -d "${TEST_DIR}" ]]; then
        rm -rf "${TEST_DIR}"
        echo "Cleaned up test directory: ${TEST_DIR}"
    fi
}

trap cleanup EXIT

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

run_test() {
    local test_name="$1"
    TESTS_RUN=$((TESTS_RUN + 1))
    echo ""
    echo "=========================================="
    echo "Test #${TESTS_RUN}: ${test_name}"
    echo "=========================================="
}

assert_success() {
    local test_name="$1"
    if [[ $? -eq 0 ]]; then
        log_info "✓ ${test_name} - PASSED"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ ${test_name} - FAILED"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

assert_failure() {
    local test_name="$1"
    if [[ $? -ne 0 ]]; then
        log_info "✓ ${test_name} - PASSED (expected failure)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ ${test_name} - FAILED (expected failure but succeeded)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

# Setup
log_info "Setting up test environment..."
TEST_DIR=$(mktemp -d -t auto_schema_test.XXXXXX)
log_info "Test directory: ${TEST_DIR}"

# Create schemas directory structure
SCHEMAS_DIR="${TEST_DIR}/schemas"
mkdir -p "${SCHEMAS_DIR}/tcms"

# Create test schemas
cat > "${SCHEMAS_DIR}/tcms/test-case.schema.v1.json" <<'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Test Case Schema",
  "type": "object",
  "required": ["type", "schema", "id", "description"],
  "properties": {
    "type": {
      "type": "string",
      "const": "test_case"
    },
    "schema": {
      "type": "string",
      "pattern": "^tcms/.*\\.schema\\.v\\d+\\.json$"
    },
    "id": {
      "type": "string"
    },
    "description": {
      "type": "string"
    },
    "test_sequences": {
      "type": "array"
    }
  }
}
EOF

cat > "${SCHEMAS_DIR}/tcms/test-execution.schema.v1.json" <<'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Test Execution Schema",
  "type": "object",
  "required": ["type", "schema", "test_sequence", "step"],
  "properties": {
    "type": {
      "type": "string",
      "const": "test_execution"
    },
    "schema": {
      "type": "string",
      "pattern": "^tcms/.*\\.schema\\.v\\d+\\.json$"
    },
    "test_sequence": {
      "type": "integer"
    },
    "step": {
      "type": "integer"
    },
    "command": {
      "type": "string"
    },
    "exit_code": {
      "type": "integer"
    },
    "output": {
      "type": "string"
    }
  }
}
EOF

cat > "${SCHEMAS_DIR}/tcms/test-result.schema.v1.json" <<'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Test Result Schema",
  "type": "object",
  "required": ["type", "schema", "test_case_id", "result"],
  "properties": {
    "type": {
      "type": "string",
      "const": "test_result"
    },
    "schema": {
      "type": "string",
      "pattern": "^tcms/.*\\.schema\\.v\\d+\\.json$"
    },
    "test_case_id": {
      "type": "string"
    },
    "result": {
      "type": "string",
      "enum": ["pass", "fail", "skip"]
    }
  }
}
EOF

# Build binaries
log_info "Building validate-yaml and validate-json binaries..."
cd "${PROJECT_ROOT}"
cargo build -p validate-yaml -p validate-json --quiet

# Find binaries using workspace-aware search
VALIDATE_YAML=$(find_binary "validate-yaml")
if [[ -z "$VALIDATE_YAML" ]]; then
    echo "[ERROR] validate-yaml binary not found after build" >&2
    exit 1
fi

VALIDATE_JSON=$(find_binary "validate-json")
if [[ -z "$VALIDATE_JSON" ]]; then
    echo "[ERROR] validate-json binary not found after build" >&2
    exit 1
fi

# Test 1: validate-yaml with auto-resolution (test_case schema)
run_test "validate-yaml: Auto-resolve test_case schema"
cat > "${TEST_DIR}/test_case.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
id: TC_001
description: Test case with schema field
test_sequences: []
EOF

"${VALIDATE_YAML}" "${TEST_DIR}/test_case.yaml" --schemas-root "${SCHEMAS_DIR}" --log-level=debug
assert_success "validate-yaml auto-resolution for test_case"

# Test 2: validate-json with auto-resolution (test_execution schema)
run_test "validate-json: Auto-resolve test_execution schema"
cat > "${TEST_DIR}/test_execution.json" <<'EOF'
{
  "type": "test_execution",
  "schema": "tcms/test-execution.schema.v1.json",
  "test_sequence": 1,
  "step": 1,
  "command": "echo test",
  "exit_code": 0,
  "output": "test"
}
EOF

"${VALIDATE_JSON}" "${TEST_DIR}/test_execution.json" --schemas-root "${SCHEMAS_DIR}" --log-level=debug
assert_success "validate-json auto-resolution for test_execution"

# Test 3: validate-yaml with explicit --schema override
run_test "validate-yaml: Explicit --schema override (should skip auto-resolution)"
cat > "${TEST_DIR}/test_case_override.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
id: TC_002
description: Test case with schema field that matches what we're validating with
test_sequences: []
EOF

"${VALIDATE_YAML}" "${TEST_DIR}/test_case_override.yaml" --schema "${SCHEMAS_DIR}/tcms/test-case.schema.v1.json" --log-level=debug
assert_success "validate-yaml explicit schema override"

# Test 4: validate-json with explicit schema override
run_test "validate-json: Explicit schema override (should skip auto-resolution)"
cat > "${TEST_DIR}/test_result_override.json" <<'EOF'
{
  "type": "test_result",
  "schema": "tcms/test-result.schema.v1.json",
  "test_case_id": "TC_001",
  "result": "pass"
}
EOF

"${VALIDATE_JSON}" "${TEST_DIR}/test_result_override.json" "${SCHEMAS_DIR}/tcms/test-result.schema.v1.json" --log-level=debug
assert_success "validate-json explicit schema override"

# Test 5: validate-yaml with missing schema field (should fail)
run_test "validate-yaml: Missing schema field (expected failure)"
cat > "${TEST_DIR}/no_schema.yaml" <<'EOF'
type: test_case
id: TC_003
description: Test case without schema field
test_sequences: []
EOF

if ! "${VALIDATE_YAML}" "${TEST_DIR}/no_schema.yaml" --schemas-root "${SCHEMAS_DIR}" > "${TEST_DIR}/output.txt" 2>&1; then
    if grep -q "Missing 'schema' field" "${TEST_DIR}/output.txt"; then
        log_info "✓ validate-yaml missing schema field - PASSED (expected failure)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ validate-yaml missing schema field - FAILED (wrong error message)"
        cat "${TEST_DIR}/output.txt"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    log_error "✗ validate-yaml missing schema field - FAILED (did not fail)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Test 6: validate-json with missing schema field (should fail)
run_test "validate-json: Missing schema field (expected failure)"
cat > "${TEST_DIR}/no_schema.json" <<'EOF'
{
  "type": "test_result",
  "test_case_id": "TC_001",
  "result": "pass"
}
EOF

if ! "${VALIDATE_JSON}" "${TEST_DIR}/no_schema.json" --schemas-root "${SCHEMAS_DIR}" > "${TEST_DIR}/output2.txt" 2>&1; then
    if grep -q "Missing 'schema' field" "${TEST_DIR}/output2.txt"; then
        log_info "✓ validate-json missing schema field - PASSED (expected failure)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ validate-json missing schema field - FAILED (wrong error message)"
        cat "${TEST_DIR}/output2.txt"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    log_error "✗ validate-json missing schema field - FAILED (did not fail)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Test 7: validate-yaml with unknown schema URI (should fail)
run_test "validate-yaml: Unknown schema URI (expected failure)"
cat > "${TEST_DIR}/unknown_schema.yaml" <<'EOF'
type: test_case
schema: tcms/nonexistent.schema.v1.json
id: TC_004
description: Test case with unknown schema
test_sequences: []
EOF

if ! "${VALIDATE_YAML}" "${TEST_DIR}/unknown_schema.yaml" --schemas-root "${SCHEMAS_DIR}" > "${TEST_DIR}/output3.txt" 2>&1; then
    if grep -q "Schema file not found" "${TEST_DIR}/output3.txt"; then
        log_info "✓ validate-yaml unknown schema URI - PASSED (expected failure)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ validate-yaml unknown schema URI - FAILED (wrong error message)"
        cat "${TEST_DIR}/output3.txt"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    log_error "✗ validate-yaml unknown schema URI - FAILED (did not fail)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Test 8: validate-json with unknown schema URI (should fail)
run_test "validate-json: Unknown schema URI (expected failure)"
cat > "${TEST_DIR}/unknown_schema.json" <<'EOF'
{
  "type": "test_result",
  "schema": "tcms/nonexistent.schema.v1.json",
  "test_case_id": "TC_001",
  "result": "pass"
}
EOF

if ! "${VALIDATE_JSON}" "${TEST_DIR}/unknown_schema.json" --schemas-root "${SCHEMAS_DIR}" > "${TEST_DIR}/output4.txt" 2>&1; then
    if grep -q "Schema file not found" "${TEST_DIR}/output4.txt"; then
        log_info "✓ validate-json unknown schema URI - PASSED (expected failure)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ validate-json unknown schema URI - FAILED (wrong error message)"
        cat "${TEST_DIR}/output4.txt"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    log_error "✗ validate-json unknown schema URI - FAILED (did not fail)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Test 9: validate-yaml with multiple files (mixed auto-resolution)
run_test "validate-yaml: Multiple files with auto-resolution"
cat > "${TEST_DIR}/multi_test_1.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
id: TC_MULTI_001
description: First test case
test_sequences: []
EOF

cat > "${TEST_DIR}/multi_test_2.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
id: TC_MULTI_002
description: Second test case
test_sequences: []
EOF

"${VALIDATE_YAML}" "${TEST_DIR}/multi_test_1.yaml" "${TEST_DIR}/multi_test_2.yaml" --schemas-root "${SCHEMAS_DIR}" --log-level=debug
assert_success "validate-yaml multiple files with auto-resolution"

# Test 10: validate-yaml with custom schemas-root
run_test "validate-yaml: Custom schemas-root directory"
CUSTOM_SCHEMAS="${TEST_DIR}/custom_schemas"
mkdir -p "${CUSTOM_SCHEMAS}/tcms"
cp "${SCHEMAS_DIR}/tcms/test-case.schema.v1.json" "${CUSTOM_SCHEMAS}/tcms/"

cat > "${TEST_DIR}/custom_root_test.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
id: TC_CUSTOM
description: Test with custom schemas root
test_sequences: []
EOF

"${VALIDATE_YAML}" "${TEST_DIR}/custom_root_test.yaml" --schemas-root "${CUSTOM_SCHEMAS}" --log-level=debug
assert_success "validate-yaml with custom schemas-root"

# Test 11: Test verifier with execution log auto-resolution (warn on missing schema)
run_test "verifier: Execution log without schema field (should warn)"

# Find verifier binary using workspace-aware search
VERIFIER=$(find_binary "verifier")
if [[ -z "$VERIFIER" ]]; then
    echo "[ERROR] verifier binary not found" >&2
    exit 1
fi

# Create a test case for verifier
TESTCASES_DIR="${TEST_DIR}/testcases"
mkdir -p "${TESTCASES_DIR}"
cat > "${TESTCASES_DIR}/TC_VERIFIER_001.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: REQ_VERIFIER
item: 1
tc: 1
id: TC_VERIFIER_001
description: Verifier test case
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Test sequence
    description: Test sequence
    initial_conditions: {}
    steps:
      - step: 1
        description: Test step
        command: echo test
        expected:
          success: true
          result: '0'
          output: test
        verification:
          result: '[[ $? -eq 0 ]]'
          output: 'true'
EOF

# Create execution log without schema field
LOGS_DIR="${TEST_DIR}/logs"
mkdir -p "${LOGS_DIR}"
cat > "${LOGS_DIR}/TC_VERIFIER_001_execution_log.json" <<'EOF'
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo test",
    "exit_code": 0,
    "output": "test",
    "timestamp": "2024-01-15T10:30:00Z",
    "result_verification_pass": true,
    "output_verification_pass": true
  }
]
EOF

# Run verifier and check for warning
if cargo build -p verifier --quiet && \
   "${VERIFIER}" --folder "${LOGS_DIR}" --test-case-dir "${TESTCASES_DIR}" --log-level=warn --success-on-completion > "${TEST_DIR}/verifier_output1.txt" 2>&1; then
    if grep -q "Missing 'schema' field in execution log" "${TEST_DIR}/verifier_output1.txt"; then
        log_info "✓ verifier warns on missing schema field - PASSED"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ verifier warns on missing schema field - FAILED (no warning found)"
        cat "${TEST_DIR}/verifier_output1.txt"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    log_error "✗ verifier warns on missing schema field - FAILED (verifier failed)"
    cat "${TEST_DIR}/verifier_output1.txt"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Test 12: Test verifier with execution log containing schema field
run_test "verifier: Execution log with schema field (should not warn)"
cat > "${LOGS_DIR}/TC_VERIFIER_002_execution_log.json" <<'EOF'
[
  {
    "type": "test_execution",
    "schema": "tcms/test-execution.schema.v1.json",
    "test_sequence": 1,
    "step": 1,
    "command": "echo test",
    "exit_code": 0,
    "output": "test",
    "timestamp": "2024-01-15T10:30:00Z",
    "result_verification_pass": true,
    "output_verification_pass": true
  }
]
EOF

cat > "${TESTCASES_DIR}/TC_VERIFIER_002.yaml" <<'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: REQ_VERIFIER
item: 1
tc: 2
id: TC_VERIFIER_002
description: Verifier test case with schema
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
  - id: 1
    name: Test sequence
    description: Test sequence
    initial_conditions: {}
    steps:
      - step: 1
        description: Test step
        command: echo test
        expected:
          success: true
          result: '0'
          output: test
EOF

if "${VERIFIER}" --folder "${LOGS_DIR}" --test-case-dir "${TESTCASES_DIR}" --log-level=debug --success-on-completion > "${TEST_DIR}/verifier_output2.txt" 2>&1; then
    if grep -q "Execution log schema: tcms/test-execution.schema.v1.json" "${TEST_DIR}/verifier_output2.txt"; then
        log_info "✓ verifier logs schema field - PASSED"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "✗ verifier logs schema field - FAILED (no schema log found)"
        cat "${TEST_DIR}/verifier_output2.txt" | grep -i schema
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    log_error "✗ verifier logs schema field - FAILED (verifier failed)"
    cat "${TEST_DIR}/verifier_output2.txt"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Print summary
echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo "Tests run: ${TESTS_RUN}"
echo -e "${GREEN}Tests passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}Tests failed: ${TESTS_FAILED}${NC}"
echo ""

if [[ ${TESTS_FAILED} -eq 0 ]]; then
    log_info "All tests passed!"
    exit 0
else
    log_error "${TESTS_FAILED} test(s) failed"
    exit 1
fi
