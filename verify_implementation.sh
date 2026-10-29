#!/usr/bin/env bash
# Verification script for dependencies, prerequisites, and complex test cases implementation

set -e

echo "========================================="
echo "Implementation Verification"
echo "========================================="
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() {
    echo -e "${GREEN}✓${NC} $1"
}

fail() {
    echo -e "${RED}✗${NC} $1"
}

warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Track failures
FAILURES=0

echo "=== 1. Checking Test Case Directories ==="
echo ""

if [ -d "test-acceptance/00_test_cases/dependencies" ]; then
    count=$(find test-acceptance/00_test_cases/dependencies -name "*.yaml" | wc -l)
    pass "Dependencies directory exists ($count test cases)"
else
    fail "Dependencies directory missing"
    ((FAILURES++))
fi

if [ -d "test-acceptance/00_test_cases/prerequisites" ]; then
    count=$(find test-acceptance/00_test_cases/prerequisites -name "*.yaml" | wc -l)
    pass "Prerequisites directory exists ($count test cases)"
else
    fail "Prerequisites directory missing"
    ((FAILURES++))
fi

if [ -d "test-acceptance/00_test_cases/complex" ]; then
    count=$(find test-acceptance/00_test_cases/complex -name "*.yaml" | wc -l)
    pass "Complex directory exists ($count test cases)"
else
    fail "Complex directory missing"
    ((FAILURES++))
fi

echo ""
echo "=== 2. Checking Hook Scripts ==="
echo ""

HOOK_SCRIPTS=(
    "script_start_init.sh"
    "setup_test_workspace.sh"
    "before_sequence_log.sh"
    "after_sequence_cleanup.sh"
    "before_step_validate.sh"
    "after_step_metrics.sh"
    "teardown_test_final.sh"
    "script_end_summary.sh"
)

for script in "${HOOK_SCRIPTS[@]}"; do
    if [ -f "test-acceptance/scripts/hooks/$script" ]; then
        if [ -x "test-acceptance/scripts/hooks/$script" ]; then
            pass "$script (executable)"
        else
            warn "$script (not executable)"
        fi
    else
        fail "$script (missing)"
        ((FAILURES++))
    fi
done

echo ""
echo "=== 3. Checking Code Changes ==="
echo ""

# Check test-executor changes
if grep -q "test_case_dir" crates/testcase-manager/src/bin/test-executor.rs; then
    pass "test-executor.rs: --test-case-dir parameter added"
else
    fail "test-executor.rs: --test-case-dir parameter missing"
    ((FAILURES++))
fi

if grep -q "build_dependency_resolver_from_dir" crates/testcase-manager/src/bin/test-executor.rs; then
    pass "test-executor.rs: build_dependency_resolver_from_dir() function added"
else
    fail "test-executor.rs: build_dependency_resolver_from_dir() function missing"
    ((FAILURES++))
fi

if grep -q "load_all_yaml_files_from_dir_recursive" crates/testcase-manager/src/bin/test-executor.rs; then
    pass "test-executor.rs: load_all_yaml_files_from_dir_recursive() function added"
else
    fail "test-executor.rs: load_all_yaml_files_from_dir_recursive() function missing"
    ((FAILURES++))
fi

# Check acceptance suite changes
if grep -q -- "--test-case-dir" test-acceptance/run_acceptance_suite.sh; then
    pass "run_acceptance_suite.sh: --test-case-dir parameter used"
else
    fail "run_acceptance_suite.sh: --test-case-dir parameter not used"
    ((FAILURES++))
fi

if grep -q "< /dev/null" test-acceptance/run_acceptance_suite.sh; then
    pass "run_acceptance_suite.sh: Non-interactive execution mode enabled"
else
    fail "run_acceptance_suite.sh: Non-interactive execution mode not enabled"
    ((FAILURES++))
fi

echo ""
echo "=== 4. Checking Documentation ==="
echo ""

DOCS=(
    "test-acceptance/00_test_cases/dependencies/DEPENDENCY_RESOLUTION_STATUS.md"
    "test-acceptance/DEPENDENCIES_PREREQUISITES_COMPLEX_STATUS.md"
    "CHANGELOG_DEPENDENCIES_PREREQUISITES_COMPLEX.md"
)

for doc in "${DOCS[@]}"; do
    if [ -f "$doc" ]; then
        pass "$(basename "$doc")"
    else
        warn "$(basename "$doc") (missing)"
    fi
done

# Check README updates
if grep -q "dependencies, prerequisites, complex" test-acceptance/README.md; then
    pass "test-acceptance/README.md: Updated with new categories"
else
    warn "test-acceptance/README.md: May need updates"
fi

echo ""
echo "=== 5. Checking Binary ==="
echo ""

if [ -f "target/debug/test-executor" ]; then
    pass "test-executor binary exists"
    
    # Test --help to verify new parameter
    if target/debug/test-executor generate --help 2>&1 | grep -q "test-case-dir"; then
        pass "test-executor: --test-case-dir parameter available"
    else
        fail "test-executor: --test-case-dir parameter not available (rebuild needed?)"
        ((FAILURES++))
    fi
else
    fail "test-executor binary missing (run: cargo build --bin test-executor)"
    ((FAILURES++))
fi

echo ""
echo "=== 6. Test Generation Sample ==="
echo ""

# Try generating a dependency test case
if [ -f "test-acceptance/00_test_cases/dependencies/TC_DEPENDENCY_SIMPLE_001.yml" ]; then
    if target/debug/test-executor generate \
        --test-case-dir test-acceptance/00_test_cases \
        --output /tmp/test_dep_verify.sh \
        test-acceptance/00_test_cases/dependencies/TC_DEPENDENCY_SIMPLE_001.yml \
        > /dev/null 2>&1; then
        pass "TC_DEPENDENCY_SIMPLE_001 generates successfully"
        rm -f /tmp/test_dep_verify.sh
    else
        fail "TC_DEPENDENCY_SIMPLE_001 generation failed"
        ((FAILURES++))
    fi
fi

# Try generating a prerequisite test case
if [ -f "test-acceptance/00_test_cases/prerequisites/PREREQ_AUTO_PASS_001.yml" ]; then
    if target/debug/test-executor generate \
        --test-case-dir test-acceptance/00_test_cases \
        --output /tmp/test_prereq_verify.sh \
        test-acceptance/00_test_cases/prerequisites/PREREQ_AUTO_PASS_001.yml \
        > /dev/null 2>&1; then
        pass "PREREQ_AUTO_PASS_001 generates successfully"
        rm -f /tmp/test_prereq_verify.sh
    else
        fail "PREREQ_AUTO_PASS_001 generation failed"
        ((FAILURES++))
    fi
fi

# Try generating a complex test case
if [ -f "test-acceptance/00_test_cases/complex/TC_COMPLEX_BDD_HOOKS_VARS_001.yml" ]; then
    if target/debug/test-executor generate \
        --test-case-dir test-acceptance/00_test_cases \
        --output /tmp/test_complex_verify.sh \
        test-acceptance/00_test_cases/complex/TC_COMPLEX_BDD_HOOKS_VARS_001.yml \
        > /dev/null 2>&1; then
        pass "TC_COMPLEX_BDD_HOOKS_VARS_001 generates successfully"
        rm -f /tmp/test_complex_verify.sh
    else
        fail "TC_COMPLEX_BDD_HOOKS_VARS_001 generation failed"
        ((FAILURES++))
    fi
fi

echo ""
echo "========================================="
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}✓ All verifications passed!${NC}"
    echo ""
    echo "Implementation is complete and ready for testing."
    echo ""
    echo "Next steps:"
    echo "1. Run: bash test-acceptance/test_deps_prereqs_complex.sh"
    echo "2. Run: bash test-acceptance/run_acceptance_suite.sh --verbose"
    echo "3. Review: test-acceptance/DEPENDENCIES_PREREQUISITES_COMPLEX_STATUS.md"
    exit 0
else
    echo -e "${RED}✗ $FAILURES verification(s) failed${NC}"
    echo ""
    echo "Please fix the issues above before proceeding."
    exit 1
fi
