#!/bin/bash
#
# End-to-end integration test for bash command test cases
#
# This test validates the full pipeline:
# 1. YAML validation against schema
# 2. Script generation with bash syntax validation
# 3. Script execution and verification
#
# Usage: ./tests/integration/test_bash_commands_e2e.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# Find binaries
cd "$PROJECT_ROOT"
TEST_EXECUTOR_BIN=$(find_binary "test-executor")
VALIDATE_YAML_BIN=$(find_binary "validate-yaml")

if [[ -z "$TEST_EXECUTOR_BIN" ]] || [[ -z "$VALIDATE_YAML_BIN" ]]; then
    fail "Required binaries not found. Please run: cargo build"
    exit 1
fi

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

echo "======================================"
echo "Bash Commands End-to-End Integration Test"
echo "======================================"
echo ""

# Test files
CONDITIONALS_YAML="test-acceptance/test_cases/bash_commands/TC_BASH_CONDITIONALS_001.yaml"
LOOPS_YAML="test-acceptance/test_cases/bash_commands/TC_BASH_LOOPS_001.yaml"

# ============================================================================
# Test 1: TC_BASH_CONDITIONALS_001
# ============================================================================
run_test_conditionals() {
    section "TC_BASH_CONDITIONALS_001: Conditional Expressions"

    local temp_dir=$(mktemp -d)

    # Stage 1: YAML Validation
    log_info "Stage 1: YAML Validation"
    if "$VALIDATE_YAML_BIN" "$CONDITIONALS_YAML" > /dev/null 2>&1; then
        pass "YAML validation passed"
    else
        fail "YAML validation failed"
        return 1
    fi

    # Stage 2: Script Generation
    log_info "Stage 2: Script Generation"
    local script="$temp_dir/TC_BASH_CONDITIONALS_001.sh"
    if "$TEST_EXECUTOR_BIN" generate "$CONDITIONALS_YAML" --output "$script" > /dev/null 2>&1; then
        pass "Script generation succeeded"
    else
        fail "Script generation failed"
        return 1
    fi

    # Verify script exists and make it executable
    if [[ -f "$script" ]]; then
        chmod +x "$script"
        pass "Generated script created and made executable"
    else
        fail "Generated script was not created"
        return 1
    fi

    # Stage 3: Script Execution
    log_info "Stage 3: Script Execution"
    if bash "$script" > "$temp_dir/output.log" 2>&1; then
        pass "Script execution succeeded"
        return 0
    else
        fail "Script execution failed"
        cat "$temp_dir/output.log"
        return 1
    fi
}

# ============================================================================
# Test 2: TC_BASH_LOOPS_001
# ============================================================================
run_test_loops() {
    section "TC_BASH_LOOPS_001: Loop Constructs"

    local temp_dir=$(mktemp -d)

    # Stage 1: YAML Validation
    log_info "Stage 1: YAML Validation"
    if "$VALIDATE_YAML_BIN" "$LOOPS_YAML" > /dev/null 2>&1; then
        pass "YAML validation passed"
    else
        fail "YAML validation failed"
        return 1
    fi

    # Stage 2: Script Generation
    log_info "Stage 2: Script Generation"
    local script="$temp_dir/TC_BASH_LOOPS_001.sh"
    if "$TEST_EXECUTOR_BIN" generate "$LOOPS_YAML" --output "$script" > /dev/null 2>&1; then
        pass "Script generation succeeded"
    else
        fail "Script generation failed"
        return 1
    fi

    # Verify script exists and make it executable
    if [[ -f "$script" ]]; then
        chmod +x "$script"
        pass "Generated script created and made executable"
    else
        fail "Generated script was not created"
        return 1
    fi

    # Stage 3: Script Execution
    log_info "Stage 3: Script Execution"
    if bash "$script" > "$temp_dir/output.log" 2>&1; then
        pass "Script execution succeeded"
        return 0
    else
        fail "Script execution failed"
        cat "$temp_dir/output.log"
        return 1
    fi
}

# ============================================================================
# Run tests
# ============================================================================
run_test_conditionals && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
echo ""
run_test_loops && ((TESTS_PASSED++)) || ((TESTS_FAILED++))

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
