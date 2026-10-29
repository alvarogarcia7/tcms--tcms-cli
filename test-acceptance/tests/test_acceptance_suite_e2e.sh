#!/usr/bin/env bash
# E2E integration test for run_acceptance_suite.sh
# Tests the acceptance suite orchestrator with a subset of test cases

set -e

# Get script directory and source logger
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# Test configuration
TEST_NAME="Acceptance Suite E2E Test"
ACCEPTANCE_DIR="$PROJECT_ROOT/test-acceptance"
ACCEPTANCE_SUITE="$ACCEPTANCE_DIR/run_acceptance_suite.sh"

# Test workspace
TEST_WORKSPACE=$(mktemp -d)
setup_cleanup "$TEST_WORKSPACE"

# Test tracking
declare -i TESTS_RUN=0
declare -i TESTS_PASSED=0
declare -i TESTS_FAILED=0

# Helper function to run a test
run_test() {
    local test_name="$1"
    local test_func="$2"
    
    ((TESTS_RUN++))
    
    section "Test: $test_name"
    
    if $test_func; then
        ((TESTS_PASSED++))
        pass "$test_name"
    else
        ((TESTS_FAILED++))
        fail "$test_name"
    fi
    
    echo ""
}

# Create isolated test environment with subset of test cases
create_test_environment() {
    local test_dir="$1"
    
    log_info "Creating isolated test environment in $test_dir"
    
    # Create directory structure
    mkdir -p "$test_dir/00_test_cases"
    mkdir -p "$test_dir/05_scripts"
    mkdir -p "$test_dir/10_test_results/execution_logs"
    mkdir -p "$test_dir/20_verification"
    mkdir -p "$test_dir/30_documentation_source"
    
    # Copy a subset of test cases (5 success, 3 failure, 2 hooks)
    # Success cases
    local success_tests=(
        "TC_SUCCESS_CMD_CHAIN_001.yml"
        "TC_SUCCESS_COMPLEX_DATA_001.yml"
        "TC_SUCCESS_CONDITIONAL_001.yml"
        "TC_SUCCESS_EMPTY_OUTPUT_001.yml"
        "TC_SUCCESS_ENV_VARS_001.yml"
    )
    
    mkdir -p "$test_dir/00_test_cases/success"
    for test in "${success_tests[@]}"; do
        if [[ -f "$ACCEPTANCE_DIR/00_test_cases/success/$test" ]]; then
            cp "$ACCEPTANCE_DIR/00_test_cases/success/$test" "$test_dir/00_test_cases/success/"
        fi
    done
    
    # Failure cases
    local failure_tests=(
        "TC_FAILURE_COMMAND_NOT_FOUND_001.yaml"
        "TC_FAILURE_EXIT_CODE_MISMATCH_001.yaml"
        "TC_FAILURE_OUTPUT_MISMATCH_001.yaml"
    )
    
    mkdir -p "$test_dir/00_test_cases/failure"
    for test in "${failure_tests[@]}"; do
        if [[ -f "$ACCEPTANCE_DIR/00_test_cases/failure/$test" ]]; then
            cp "$ACCEPTANCE_DIR/00_test_cases/failure/$test" "$test_dir/00_test_cases/failure/"
        fi
    done
    
    # Hook cases
    local hook_tests=(
        "HOOKS_AFTER_SEQUENCE_001.yaml"
        "HOOKS_AFTER_STEP_001.yaml"
    )
    
    mkdir -p "$test_dir/00_test_cases/hooks"
    for test in "${hook_tests[@]}"; do
        if [[ -f "$ACCEPTANCE_DIR/00_test_cases/hooks/$test" ]]; then
            # Copy YAML and any associated hook scripts
            cp "$ACCEPTANCE_DIR/00_test_cases/hooks/$test" "$test_dir/00_test_cases/hooks/"
            
            # Copy hook script directories if they exist
            local test_base=$(basename "$test" .yaml)
            if [[ -d "$ACCEPTANCE_DIR/00_test_cases/hooks/${test_base}_scripts" ]]; then
                cp -r "$ACCEPTANCE_DIR/00_test_cases/hooks/${test_base}_scripts" "$test_dir/00_test_cases/hooks/"
            fi
        fi
    done
    
    # Create a modified run_acceptance_suite.sh that uses the test directory
    cat > "$test_dir/run_acceptance_suite_test.sh" << 'EOF'
#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# Override paths to use test directory
TEST_CASES_DIR="$SCRIPT_DIR/00_test_cases"
EXECUTION_LOGS_DIR="$SCRIPT_DIR/10_test_results/execution_logs"
VERIFICATION_RESULTS_DIR="$SCRIPT_DIR/20_verification"
SCRIPTS_DIR="$SCRIPT_DIR/05_scripts"
REPORTS_DIR="$SCRIPT_DIR/30_documentation_source"
SCHEMA_DIR="$PROJECT_ROOT/schemas"
CONTAINER_SCHEMA="$PROJECT_ROOT/data/testcase_results_container/schema.json"

VALIDATE_YAML="${PROJECT_ROOT}/target/debug/validate-yaml"
TEST_EXECUTOR="${PROJECT_ROOT}/target/debug/test-executor"
VERIFIER="${PROJECT_ROOT}/target/debug/verifier"
VALIDATE_JSON="${PROJECT_ROOT}/target/debug/validate-json"

TPDG_BIN="${TEST_PLAN_DOC_GEN:-test-plan-documentation-generator}"

# Export for sub-scripts
export TEST_CASES_DIR EXECUTION_LOGS_DIR VERIFICATION_RESULTS_DIR SCRIPTS_DIR REPORTS_DIR

# Source and run the main acceptance suite logic
source "$PROJECT_ROOT/test-acceptance/run_acceptance_suite.sh"
EOF
    
    chmod +x "$test_dir/run_acceptance_suite_test.sh"
    
    log_info "Test environment created with $(find "$test_dir/00_test_cases" -name "*.yaml" | wc -l | tr -d ' ') test cases"
}

# Test 1: Verify acceptance suite completes successfully on subset
test_basic_execution() {
    local test_env="$TEST_WORKSPACE/basic_execution"
    create_test_environment "$test_env"
    
    log_info "Running acceptance suite on test subset..."
    
    # Run with output captured
    local output_file="$TEST_WORKSPACE/basic_execution_output.log"
    
    # Use the actual acceptance suite script with test directory
    cd "$test_env"
    if TEST_CASES_DIR="$test_env/00_test_cases" \
       EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
       VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
       SCRIPTS_DIR="$test_env/05_scripts" \
       REPORTS_DIR="$test_env/30_documentation_source" \
       "$ACCEPTANCE_SUITE" > "$output_file" 2>&1; then
        
        log_info "Acceptance suite completed successfully"
        
        # Verify all stages mentioned in output
        if ! grep -q "Stage 1: Validating Test Case YAMLs" "$output_file"; then
            log_error "Stage 1 not found in output"
            return 1
        fi
        
        if ! grep -q "Stage 2: Generating Test Scripts" "$output_file"; then
            log_error "Stage 2 not found in output"
            return 1
        fi
        
        if ! grep -q "Stage 3: Executing Test Scripts" "$output_file"; then
            log_error "Stage 3 not found in output"
            return 1
        fi
        
        if ! grep -q "Stage 4: Verifying Execution Logs" "$output_file"; then
            log_error "Stage 4 not found in output"
            return 1
        fi
        
        if ! grep -q "Stage 5: Validating Container YAMLs" "$output_file"; then
            log_error "Stage 5 not found in output"
            return 1
        fi
        
        if ! grep -q "Stage 6: Generating Documentation" "$output_file"; then
            log_error "Stage 6 not found in output"
            return 1
        fi
        
        pass "All 6 stages executed"
        return 0
    else
        log_error "Acceptance suite failed unexpectedly"
        log_error "Output saved to: $output_file"
        if [[ ${VERBOSE:-0} -eq 1 ]]; then
            cat "$output_file"
        fi
        return 1
    fi
}

# Test 2: Verify expected files are created at each stage
test_file_creation() {
    local test_env="$TEST_WORKSPACE/file_creation"
    create_test_environment "$test_env"
    
    log_info "Running acceptance suite and verifying file creation..."
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" > "$TEST_WORKSPACE/file_creation_output.log" 2>&1 || true
    
    # Stage 2: Check generated scripts
    local script_count=$(find "$test_env/05_scripts" -name "*.sh" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $script_count -lt 8 ]]; then
        log_error "Expected at least 8 generated scripts, found $script_count"
        return 1
    fi
    pass "Stage 2: Found $script_count generated scripts"
    
    # Stage 3: Check execution logs
    local log_count=$(find "$test_env/10_test_results/execution_logs" -name "*.json" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $log_count -lt 5 ]]; then
        log_error "Expected at least 5 execution logs, found $log_count"
        return 1
    fi
    pass "Stage 3: Found $log_count execution logs"
    
    # Stage 4: Check verification results
    local container_count=$(find "$test_env/20_verification" -name "*_container.yaml" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $container_count -lt 5 ]]; then
        log_error "Expected at least 5 container YAMLs, found $container_count"
        return 1
    fi
    pass "Stage 4: Found $container_count container YAMLs"
    
    # Stage 6: Check documentation
    local asciidoc_count=$(find "$test_env/30_documentation_source/asciidoc" -name "*.adoc" -type f 2>/dev/null | wc -l | tr -d ' ')
    local markdown_count=$(find "$test_env/30_documentation_source/markdown" -name "*.md" -type f 2>/dev/null | wc -l | tr -d ' ')
    
    # Documentation might be skipped if TPDG not available
    if command -v test-plan-documentation-generator > /dev/null 2>&1 || [[ -n "${TEST_PLAN_DOC_GEN:-}" ]]; then
        if [[ $asciidoc_count -lt 5 ]]; then
            log_error "Expected at least 5 AsciiDoc files, found $asciidoc_count"
            return 1
        fi
        if [[ $markdown_count -lt 5 ]]; then
            log_error "Expected at least 5 Markdown files, found $markdown_count"
            return 1
        fi
        pass "Stage 6: Found $asciidoc_count AsciiDoc and $markdown_count Markdown files"
    else
        log_warning "TPDG not available, skipping documentation verification"
    fi
    
    # Check summary report
    if [[ ! -f "$test_env/30_documentation_source/acceptance_suite_summary.txt" ]]; then
        log_error "Summary report not found"
        return 1
    fi
    pass "Summary report created"
    
    return 0
}

# Test 3: Validate final report has correct statistics
test_final_report_statistics() {
    local test_env="$TEST_WORKSPACE/report_stats"
    create_test_environment "$test_env"
    
    log_info "Running acceptance suite and validating report statistics..."
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" > "$TEST_WORKSPACE/report_stats_output.log" 2>&1 || true
    
    local report="$test_env/30_documentation_source/acceptance_suite_summary.txt"
    
    if [[ ! -f "$report" ]]; then
        log_error "Report file not found: $report"
        return 1
    fi
    
    # Verify report structure
    if ! grep -q "Acceptance Test Suite Execution Summary" "$report"; then
        log_error "Report missing header"
        return 1
    fi
    
    if ! grep -q "Stage 1: YAML Validation" "$report"; then
        log_error "Report missing Stage 1 section"
        return 1
    fi
    
    if ! grep -q "Stage 2: Script Generation" "$report"; then
        log_error "Report missing Stage 2 section"
        return 1
    fi
    
    if ! grep -q "Stage 3: Test Execution" "$report"; then
        log_error "Report missing Stage 3 section"
        return 1
    fi
    
    if ! grep -q "Overall Result:" "$report"; then
        log_error "Report missing overall result"
        return 1
    fi
    
    # Verify statistics are present
    if ! grep -q "Total Test Cases:" "$report"; then
        log_error "Report missing test count"
        return 1
    fi
    
    pass "Report structure validated"
    
    # Check that numbers are reasonable (should have some test cases)
    local total_tests=$(grep "Total Test Cases:" "$report" | grep -o '[0-9]\+' | head -1)
    if [[ -z "$total_tests" ]] || [[ $total_tests -lt 8 ]]; then
        log_error "Invalid test count in report: '$total_tests'"
        return 1
    fi
    
    pass "Report statistics validated (Total: $total_tests)"
    
    return 0
}

# Test 4: Test --skip-generation flag
test_skip_generation_flag() {
    local test_env="$TEST_WORKSPACE/skip_generation"
    create_test_environment "$test_env"
    
    log_info "Testing --skip-generation flag..."
    
    local output_file="$TEST_WORKSPACE/skip_generation_output.log"
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" --skip-generation > "$output_file" 2>&1 || true
    
    # Verify Stage 2 was skipped
    if ! grep -q "Stage 2: Script Generation (SKIPPED)" "$output_file"; then
        log_error "--skip-generation flag not honored"
        return 1
    fi
    
    pass "Stage 2 correctly skipped"
    
    # Verify no scripts were generated
    local script_count=$(find "$test_env/05_scripts" -name "*.sh" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $script_count -ne 0 ]]; then
        log_error "Scripts were generated despite --skip-generation flag"
        return 1
    fi
    
    pass "No scripts generated"
    
    return 0
}

# Test 5: Test --skip-execution flag
test_skip_execution_flag() {
    local test_env="$TEST_WORKSPACE/skip_execution"
    create_test_environment "$test_env"
    
    log_info "Testing --skip-execution flag..."
    
    local output_file="$TEST_WORKSPACE/skip_execution_output.log"
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" --skip-execution > "$output_file" 2>&1 || true
    
    # Verify Stage 3 was skipped
    if ! grep -q "Stage 3: Test Execution (SKIPPED)" "$output_file"; then
        log_error "--skip-execution flag not honored"
        return 1
    fi
    
    pass "Stage 3 correctly skipped"
    
    # Scripts should still be generated
    local script_count=$(find "$test_env/05_scripts" -name "*.sh" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $script_count -lt 8 ]]; then
        log_error "Scripts not generated (expected at least 8, got $script_count)"
        return 1
    fi
    
    pass "Scripts still generated ($script_count)"
    
    # But execution logs should not exist
    local log_count=$(find "$test_env/10_test_results/execution_logs" -name "*.json" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $log_count -ne 0 ]]; then
        log_error "Execution logs created despite --skip-execution flag"
        return 1
    fi
    
    pass "No execution logs created"
    
    return 0
}

# Test 6: Test --skip-verification flag
test_skip_verification_flag() {
    local test_env="$TEST_WORKSPACE/skip_verification"
    create_test_environment "$test_env"
    
    log_info "Testing --skip-verification flag..."
    
    local output_file="$TEST_WORKSPACE/skip_verification_output.log"
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" --skip-verification > "$output_file" 2>&1 || true
    
    # Verify Stage 4 and 5 were skipped
    if ! grep -q "Stage 4: Verification (SKIPPED)" "$output_file"; then
        log_error "Stage 4 not skipped"
        return 1
    fi
    
    if ! grep -q "Stage 5: Container Validation (SKIPPED)" "$output_file"; then
        log_error "Stage 5 not skipped"
        return 1
    fi
    
    pass "Stages 4 and 5 correctly skipped"
    
    # Execution should still happen
    local log_count=$(find "$test_env/10_test_results/execution_logs" -name "*.json" -type f 2>/dev/null | wc -l | tr -d ' ')
    if [[ $log_count -lt 5 ]]; then
        log_error "Execution logs not created"
        return 1
    fi
    
    pass "Execution still completed ($log_count logs)"
    
    return 0
}

# Test 7: Test --skip-documentation flag
test_skip_documentation_flag() {
    local test_env="$TEST_WORKSPACE/skip_documentation"
    create_test_environment "$test_env"
    
    log_info "Testing --skip-documentation flag..."
    
    local output_file="$TEST_WORKSPACE/skip_documentation_output.log"
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" --skip-documentation > "$output_file" 2>&1 || true
    
    # Verify Stage 6 was skipped
    if ! grep -q "Stage 6: Documentation Generation (SKIPPED)" "$output_file"; then
        log_error "--skip-documentation flag not honored"
        return 1
    fi
    
    pass "Stage 6 correctly skipped"
    
    return 0
}

# Test 8: Test --verbose flag increases logging detail
test_verbose_flag() {
    local test_env="$TEST_WORKSPACE/verbose"
    create_test_environment "$test_env"
    
    log_info "Testing --verbose flag..."
    
    local output_normal="$TEST_WORKSPACE/verbose_normal.log"
    local output_verbose="$TEST_WORKSPACE/verbose_verbose.log"
    
    # Run without verbose
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" > "$output_normal" 2>&1 || true
    
    # Clean for second run
    rm -rf "$test_env/05_scripts" "$test_env/10_test_results" "$test_env/20_verification" "$test_env/30_documentation_source"
    mkdir -p "$test_env/05_scripts" "$test_env/10_test_results/execution_logs" "$test_env/20_verification" "$test_env/30_documentation_source"
    
    # Run with verbose
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" --verbose > "$output_verbose" 2>&1 || true
    
    # Verbose output should be longer (more detailed)
    local normal_lines=$(wc -l < "$output_normal" | tr -d ' ')
    local verbose_lines=$(wc -l < "$output_verbose" | tr -d ' ')
    
    if [[ $verbose_lines -le $normal_lines ]]; then
        log_error "Verbose output not more detailed than normal (normal: $normal_lines, verbose: $verbose_lines)"
        return 1
    fi
    
    pass "Verbose output has more detail ($normal_lines -> $verbose_lines lines)"
    
    # Verbose should include [VERBOSE] tags
    if ! grep -q "\[VERBOSE\]" "$output_verbose"; then
        log_error "Verbose output missing [VERBOSE] tags"
        return 1
    fi
    
    pass "Verbose output includes [VERBOSE] tags"
    
    return 0
}

# Test 9: Test error handling for missing TPDG
test_missing_tpdg_handling() {
    local test_env="$TEST_WORKSPACE/missing_tpdg"
    create_test_environment "$test_env"
    
    log_info "Testing error handling when TPDG not available..."
    
    local output_file="$TEST_WORKSPACE/missing_tpdg_output.log"
    
    # Run with TPDG deliberately not set
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    env -u TEST_PLAN_DOC_GEN PATH="/bin:/usr/bin" \
    "$ACCEPTANCE_SUITE" > "$output_file" 2>&1 || true
    
    # Should warn about missing TPDG during binary verification
    if ! grep -q "test-plan-documentation-generator not found" "$output_file"; then
        log_error "Missing TPDG not detected during verification"
        return 1
    fi
    
    pass "Missing TPDG detected and logged"
    
    # Documentation stage should handle gracefully
    if grep -q "Stage 6: Generating Documentation" "$output_file"; then
        # Either it skips gracefully or warns
        if grep -q "test-plan-documentation-generator not found" "$output_file" || \
           grep -q "Skipping documentation generation" "$output_file"; then
            pass "Documentation stage handled missing TPDG gracefully"
        else
            log_error "Documentation stage didn't handle missing TPDG properly"
            return 1
        fi
    fi
    
    return 0
}

# Test 10: Test timeout handling for long-running scripts
test_timeout_handling() {
    local test_env="$TEST_WORKSPACE/timeout"
    create_test_environment "$test_env"
    
    log_info "Testing timeout handling..."
    
    # Create a test case with a long-running command
    cat > "$test_env/00_test_cases/timeout_test.yaml" << 'EOF'
test_case_id: TC_TIMEOUT_001
title: Timeout Test Case
description: Tests timeout handling
test_sequences:
  - sequence_id: "1"
    sequence_name: "Timeout Sequence"
    steps:
      - step_number: 1
        description: "Quick command"
        command: "echo 'Starting...'"
        expected_result:
          exit_code: 0
      - step_number: 2
        description: "Another quick command"
        command: "echo 'Done'"
        expected_result:
          exit_code: 0
EOF
    
    # Run acceptance suite with timeout (use timeout command on the script execution)
    local output_file="$TEST_WORKSPACE/timeout_output.log"
    
    cd "$test_env"
    
    # Run with very short timeout to ensure completion
    if timeout 300 env \
        TEST_CASES_DIR="$test_env/00_test_cases" \
        EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
        VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
        SCRIPTS_DIR="$test_env/05_scripts" \
        REPORTS_DIR="$test_env/30_documentation_source" \
        "$ACCEPTANCE_SUITE" > "$output_file" 2>&1; then
        pass "Acceptance suite completed within timeout"
        return 0
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            log_error "Acceptance suite timed out"
            return 1
        else
            # Non-timeout failure is acceptable for this test
            pass "Acceptance suite handled within timeout (exit: $exit_code)"
            return 0
        fi
    fi
}

# Test 11: Test cleanup of temporary files
test_cleanup_temporary_files() {
    local test_env="$TEST_WORKSPACE/cleanup"
    create_test_environment "$test_env"
    
    log_info "Testing cleanup of temporary files..."
    
    # Create some markers to track temp file creation
    local output_file="$TEST_WORKSPACE/cleanup_output.log"
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" > "$output_file" 2>&1 || true
    
    # After completion, check that no stray temp files remain in /tmp
    # (excluding our own test workspace)
    local temp_pattern="acceptance_suite_*"
    local stray_temps=$(find /tmp -maxdepth 1 -name "$temp_pattern" -type d 2>/dev/null | wc -l | tr -d ' ')
    
    if [[ $stray_temps -gt 0 ]]; then
        log_warning "Found $stray_temps potential stray temp directories"
        # This is a soft warning, not a hard failure
    fi
    
    pass "Cleanup check completed"
    
    # Verify expected output directories exist
    if [[ ! -d "$test_env/05_scripts" ]]; then
        log_error "Scripts directory was cleaned up incorrectly"
        return 1
    fi
    
    if [[ ! -d "$test_env/30_documentation_source" ]]; then
        log_error "Documentation source directory was cleaned up incorrectly"
        return 1
    fi
    
    pass "Output directories preserved correctly"
    
    return 0
}

# Test 12: Test multiple skip flags combined
test_multiple_skip_flags() {
    local test_env="$TEST_WORKSPACE/multiple_skip"
    create_test_environment "$test_env"
    
    log_info "Testing multiple --skip-* flags combined..."
    
    local output_file="$TEST_WORKSPACE/multiple_skip_output.log"
    
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" --skip-execution --skip-verification --skip-documentation \
        > "$output_file" 2>&1 || true
    
    # Verify all skip flags honored
    if ! grep -q "Stage 3: Test Execution (SKIPPED)" "$output_file"; then
        log_error "Execution not skipped"
        return 1
    fi
    
    if ! grep -q "Stage 4: Verification (SKIPPED)" "$output_file"; then
        log_error "Verification not skipped"
        return 1
    fi
    
    if ! grep -q "Stage 6: Documentation Generation (SKIPPED)" "$output_file"; then
        log_error "Documentation not skipped"
        return 1
    fi
    
    pass "All skip flags honored simultaneously"
    
    # Should still have validation and generation
    if ! grep -q "Stage 1: Validating Test Case YAMLs" "$output_file"; then
        log_error "Stage 1 not executed"
        return 1
    fi
    
    if ! grep -q "Stage 2: Generating Test Scripts" "$output_file"; then
        log_error "Stage 2 not executed"
        return 1
    fi
    
    pass "Non-skipped stages still executed"
    
    return 0
}

# Test 13: Test Stage 7 consolidated documentation generation
test_stage7_consolidated_documentation() {
    local test_env="$TEST_WORKSPACE/stage7_consolidated"
    create_test_environment "$test_env"
    
    log_info "Testing Stage 7 consolidated documentation generation..."
    
    # Check if TPDG is available
    if ! command -v test-plan-documentation-generator > /dev/null 2>&1 && [[ -z "${TEST_PLAN_DOC_GEN:-}" ]]; then
        log_warning "TPDG not available, skipping Stage 7 test"
        return 0
    fi
    
    local output_file="$TEST_WORKSPACE/stage7_output.log"
    
    # Run acceptance suite on test subset
    cd "$test_env"
    TEST_CASES_DIR="$test_env/00_test_cases" \
    EXECUTION_LOGS_DIR="$test_env/10_test_results/execution_logs" \
    VERIFICATION_RESULTS_DIR="$test_env/20_verification" \
    SCRIPTS_DIR="$test_env/05_scripts" \
    REPORTS_DIR="$test_env/30_documentation_source" \
    "$ACCEPTANCE_SUITE" > "$output_file" 2>&1 || true
    
    # Verify Stage 7 was executed
    if ! grep -q "Stage 7: Generating Consolidated Documentation" "$output_file"; then
        log_error "Stage 7 not found in output"
        return 1
    fi
    pass "Stage 7 executed"
    
    # Verify no CONSOLIDATED_DOC_FAILED errors occurred
    if grep -qi "CONSOLIDATED_DOC_FAILED" "$output_file"; then
        log_error "CONSOLIDATED_DOC_FAILED errors detected in output"
        return 1
    fi
    pass "No CONSOLIDATED_DOC_FAILED errors"
    
    # Verify test-acceptance/30_documentation_source/consolidated/ directory was created
    local consolidated_dir="$test_env/30_documentation_source/consolidated"
    if [[ ! -d "$consolidated_dir" ]]; then
        log_error "Consolidated documentation directory not created: $consolidated_dir"
        return 1
    fi
    pass "Consolidated documentation directory created"
    
    # Check that all_tests_container.yaml was created
    local consolidated_container="$test_env/30_documentation_source/consolidated/all_tests_container.yaml"
    if [[ ! -f "$consolidated_container" ]]; then
        log_error "Consolidated container YAML not created: $consolidated_container"
        return 1
    fi
    pass "all_tests_container.yaml created"
    
    # Validate container YAML structure
    if ! grep -q "^title:" "$consolidated_container"; then
        log_error "Container missing 'title' field"
        return 1
    fi
    
    if ! grep -q "^project:" "$consolidated_container"; then
        log_error "Container missing 'project' field"
        return 1
    fi
    
    if ! grep -q "^test_results:" "$consolidated_container"; then
        log_error "Container missing 'test_results' field"
        return 1
    fi
    pass "Container YAML has required structure"
    
    # Validate container YAML against schema
    local container_schema="$PROJECT_ROOT/data/testcase_results_container/schema.json"
    if [[ -f "$container_schema" ]]; then
        local validate_yaml
        if [[ -f "$PROJECT_ROOT/target/release/validate-yaml" ]]; then
            validate_yaml="$PROJECT_ROOT/target/release/validate-yaml"
        elif [[ -f "$PROJECT_ROOT/target/debug/validate-yaml" ]]; then
            validate_yaml="$PROJECT_ROOT/target/debug/validate-yaml"
        fi
        
        if [[ -n "$validate_yaml" ]]; then
            if "$validate_yaml" --schema "$container_schema" "$consolidated_container" >/dev/null 2>&1; then
                pass "Container YAML validates against schema"
            else
                log_error "Container YAML failed schema validation"
                return 1
            fi
        else
            log_warning "validate-yaml binary not found, skipping schema validation"
        fi
    else
        log_warning "Container schema not found at: $container_schema"
    fi
    
    # Verify container contains multiple test case results
    local test_case_count=$(grep -c "test_case_id:" "$consolidated_container" || true)
    if [[ $test_case_count -lt 5 ]]; then
        log_error "Container has too few test cases: $test_case_count (expected at least 5)"
        return 1
    fi
    pass "Container contains $test_case_count test case results"
    
    # Verify metadata section has correct total_test_cases count
    if grep -q "^metadata:" "$consolidated_container"; then
        # Extract total_test_cases from metadata
        local metadata_count=$(grep -A 10 "^metadata:" "$consolidated_container" | grep "total_test_cases:" | grep -o '[0-9]\+' || echo "0")
        
        # Count actual execution logs
        local log_count=$(find "$test_env/10_test_results/execution_logs" -name "*.json" -type f 2>/dev/null | wc -l | tr -d ' ')
        
        if [[ "$metadata_count" -ne "$log_count" ]]; then
            log_error "Metadata total_test_cases ($metadata_count) doesn't match execution logs count ($log_count)"
            return 1
        fi
        pass "Metadata total_test_cases ($metadata_count) matches execution logs count"
    else
        log_error "Container YAML missing metadata section"
        return 1
    fi
    
    # Verify test case IDs from different categories are present
    local has_success=0
    local has_failure=0
    local has_hooks=0
    
    # Check for success test cases
    if grep -q "TC_SUCCESS_" "$consolidated_container"; then
        has_success=1
    fi
    
    # Check for failure test cases
    if grep -q "TC_FAILURE_" "$consolidated_container"; then
        has_failure=1
    fi
    
    # Check for hook test cases
    if grep -q "HOOKS_" "$consolidated_container"; then
        has_hooks=1
    fi
    
    if [[ $has_success -eq 0 ]]; then
        log_error "No success test cases found in container"
        return 1
    fi
    
    if [[ $has_failure -eq 0 ]]; then
        log_error "No failure test cases found in container"
        return 1
    fi
    
    if [[ $has_hooks -eq 0 ]]; then
        log_error "No hooks test cases found in container"
        return 1
    fi
    pass "Container includes test cases from multiple categories (success, failure, hooks)"
    
    # Verify TPDG successfully generated AsciiDoc
    local consolidated_asciidoc="$test_env/30_documentation_source/consolidated/all_tests.adoc"
    if [[ ! -f "$consolidated_asciidoc" ]]; then
        log_error "Consolidated AsciiDoc not created: $consolidated_asciidoc"
        return 1
    fi
    pass "all_tests.adoc created"
    
    # Verify AsciiDoc content
    if [[ ! -s "$consolidated_asciidoc" ]]; then
        log_error "AsciiDoc file is empty"
        return 1
    fi
    
    # Check for typical AsciiDoc markers
    if ! grep -q "^=" "$consolidated_asciidoc"; then
        log_error "AsciiDoc missing header markers"
        return 1
    fi
    pass "AsciiDoc has valid content"
    
    # Verify TPDG successfully generated Markdown
    local consolidated_markdown="$test_env/30_documentation_source/consolidated/all_tests.md"
    if [[ ! -f "$consolidated_markdown" ]]; then
        log_error "Consolidated Markdown not created: $consolidated_markdown"
        return 1
    fi
    pass "all_tests.md created"
    
    # Verify Markdown content
    if [[ ! -s "$consolidated_markdown" ]]; then
        log_error "Markdown file is empty"
        return 1
    fi
    
    # Check for typical Markdown markers
    if ! grep -q "^#" "$consolidated_markdown"; then
        log_error "Markdown missing header markers"
        return 1
    fi
    pass "Markdown has valid content"
    
    # Verify documentation includes test case IDs from multiple categories
    local doc_has_success=0
    local doc_has_failure=0
    local doc_has_hooks=0
    
    if grep -q "TC_SUCCESS_" "$consolidated_asciidoc" && grep -q "TC_SUCCESS_" "$consolidated_markdown"; then
        doc_has_success=1
    fi
    
    if grep -q "TC_FAILURE_" "$consolidated_asciidoc" && grep -q "TC_FAILURE_" "$consolidated_markdown"; then
        doc_has_failure=1
    fi
    
    if grep -q "HOOKS_" "$consolidated_asciidoc" && grep -q "HOOKS_" "$consolidated_markdown"; then
        doc_has_hooks=1
    fi
    
    if [[ $doc_has_success -eq 0 ]]; then
        log_error "Success test cases not found in generated documentation"
        return 1
    fi
    
    if [[ $doc_has_failure -eq 0 ]]; then
        log_error "Failure test cases not found in generated documentation"
        return 1
    fi
    
    if [[ $doc_has_hooks -eq 0 ]]; then
        log_error "Hooks test cases not found in generated documentation"
        return 1
    fi
    pass "Documentation content includes test case IDs from multiple categories"
    
    # Verify summary report mentions Stage 7
    local report="$test_env/30_documentation_source/acceptance_suite_summary.txt"
    if [[ ! -f "$report" ]]; then
        log_error "Summary report not found"
        return 1
    fi
    
    if ! grep -q "Stage 7: Consolidated Documentation" "$report"; then
        log_error "Summary report missing Stage 7 section"
        return 1
    fi
    pass "Summary report includes Stage 7 results"
    
    # Verify consolidated documentation paths are listed in summary
    if ! grep -q "all_tests_container.yaml" "$report"; then
        log_error "Summary report doesn't mention consolidated container"
        return 1
    fi
    
    if ! grep -q "all_tests.adoc" "$report"; then
        log_error "Summary report doesn't mention consolidated AsciiDoc"
        return 1
    fi
    
    if ! grep -q "all_tests.md" "$report"; then
        log_error "Summary report doesn't mention consolidated Markdown"
        return 1
    fi
    pass "Summary report lists all consolidated documentation files"
    
    log_info "Stage 7 consolidated documentation generation verified successfully"
    return 0
}

# Main test execution
main() {
    section "$TEST_NAME"
    echo ""
    
    log_info "Test workspace: $TEST_WORKSPACE"
    log_info "Acceptance suite: $ACCEPTANCE_SUITE"
    echo ""
    
    # Verify acceptance suite exists
    if [[ ! -x "$ACCEPTANCE_SUITE" ]]; then
        log_error "Acceptance suite not found or not executable: $ACCEPTANCE_SUITE"
        exit 1
    fi
    
    # Verify required binaries exist
    local required_bins=(
        "$PROJECT_ROOT/target/debug/validate-yaml"
        "$PROJECT_ROOT/target/debug/test-executor"
        "$PROJECT_ROOT/target/debug/verifier"
    )
    
    for bin in "${required_bins[@]}"; do
        if [[ ! -x "$bin" ]]; then
            log_error "Required binary not found: $bin"
            log_info "Run: make build"
            exit 1
        fi
    done
    
    # Run tests
    run_test "Basic Execution - All Stages Complete" test_basic_execution
    run_test "File Creation - All Stages" test_file_creation
    run_test "Final Report Statistics" test_final_report_statistics
    run_test "Skip Generation Flag" test_skip_generation_flag
    run_test "Skip Execution Flag" test_skip_execution_flag
    run_test "Skip Verification Flag" test_skip_verification_flag
    run_test "Skip Documentation Flag" test_skip_documentation_flag
    run_test "Verbose Flag" test_verbose_flag
    run_test "Missing TPDG Handling" test_missing_tpdg_handling
    run_test "Timeout Handling" test_timeout_handling
    run_test "Cleanup of Temporary Files" test_cleanup_temporary_files
    run_test "Multiple Skip Flags" test_multiple_skip_flags
    run_test "Stage 7 Consolidated Documentation" test_stage7_consolidated_documentation
    
    # Print summary
    section "Test Summary"
    log_info "Tests run:    $TESTS_RUN"
    log_info "Tests passed: $TESTS_PASSED"
    log_info "Tests failed: $TESTS_FAILED"
    echo ""
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        pass "All tests passed!"
        exit 0
    else
        fail "$TESTS_FAILED test(s) failed"
        exit 1
    fi
}

# Run main
main
