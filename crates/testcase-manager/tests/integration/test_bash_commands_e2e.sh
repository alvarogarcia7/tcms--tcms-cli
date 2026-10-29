#!/bin/bash
#
# End-to-end integration test for bash command test cases
#
# Full pipeline testing:
# 1. YAML validation against schema
# 2. Script generation
# 3. Script execution
#

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

cd "$PROJECT_ROOT"
TEST_EXECUTOR_BIN=$(find_binary "test-executor") || exit 1
VALIDATE_YAML_BIN=$(find_binary "validate-yaml") || exit 1

TESTS_PASSED=0
TESTS_FAILED=0

echo "======================================"
echo "Bash Commands End-to-End Integration Test"
echo "======================================"
echo ""

# ============================================================================
# Test 1: TC_BASH_CONDITIONALS_001
# ============================================================================
section "TC_BASH_CONDITIONALS_001: Conditional Expressions"

COND_YAML="test-acceptance/00_test_cases/bash_commands/TC_BASH_CONDITIONALS_001.yml"
COND_SCRIPT=$(mktemp)
trap "rm -f $COND_SCRIPT" EXIT

# Validation
log_info "Stage 1: YAML Validation"
if "$VALIDATE_YAML_BIN" "$COND_YAML" > /dev/null 2>&1; then
    pass "YAML validation passed"
else
    fail "YAML validation failed"
    ((TESTS_FAILED++))
    exit 1
fi

# Generation
log_info "Stage 2: Script Generation"
if "$TEST_EXECUTOR_BIN" generate "$COND_YAML" --output "$COND_SCRIPT" > /dev/null 2>&1; then
    pass "Script generation succeeded"
    chmod +x "$COND_SCRIPT"
else
    fail "Script generation failed"
    ((TESTS_FAILED++))
    exit 1
fi

# Execution
log_info "Stage 3: Script Execution"
if bash "$COND_SCRIPT" > /dev/null 2>&1; then
    pass "Script execution succeeded"
    ((TESTS_PASSED++))
else
    fail "Script execution failed"
    ((TESTS_FAILED++))
    exit 1
fi

echo ""

# ============================================================================
# Test 2: TC_BASH_LOOPS_001
# ============================================================================
section "TC_BASH_LOOPS_001: Loop Constructs"

LOOPS_YAML="test-acceptance/00_test_cases/bash_commands/TC_BASH_LOOPS_001.yml"
LOOPS_SCRIPT=$(mktemp)

# Validation
log_info "Stage 1: YAML Validation"
if "$VALIDATE_YAML_BIN" "$LOOPS_YAML" > /dev/null 2>&1; then
    pass "YAML validation passed"
else
    fail "YAML validation failed"
    ((TESTS_FAILED++))
    exit 1
fi

# Generation
log_info "Stage 2: Script Generation"
if "$TEST_EXECUTOR_BIN" generate "$LOOPS_YAML" --output "$LOOPS_SCRIPT" > /dev/null 2>&1; then
    pass "Script generation succeeded"
    chmod +x "$LOOPS_SCRIPT"
else
    fail "Script generation failed"
    ((TESTS_FAILED++))
    exit 1
fi

# Execution
log_info "Stage 3: Script Execution"
if bash "$LOOPS_SCRIPT" > /dev/null 2>&1; then
    pass "Script execution succeeded"
    ((TESTS_PASSED++))
else
    fail "Script execution failed"
    ((TESTS_FAILED++))
    exit 1
fi

rm -f "$LOOPS_SCRIPT"

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "======================================"
echo "Test Summary"
echo "======================================"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo ""
    pass "All bash command tests passed!"
    exit 0
else
    echo ""
    fail "Some tests failed"
    exit 1
fi
