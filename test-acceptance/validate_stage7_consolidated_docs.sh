#!/usr/bin/env bash
#
# validate_stage7_consolidated_docs.sh - Validates Stage 7 (consolidated documentation generation)
#
# DESCRIPTION:
#   This script validates Stage 7 by running verifier in --folder mode to generate a
#   consolidated all_tests_container.yaml with all test case results aggregated, then
#   using TPDG to generate unified documentation. It performs:
#   - Running verifier --folder mode on 10_test_results/execution_logs/ directory
#   - Validating all_tests_container.yaml is generated
#   - Validating container YAML against schema
#   - Verifying metadata section has correct total_test_cases count
#   - Confirming total_test_cases matches number of execution logs
#   - Invoking TPDG to generate all_tests.adoc from container YAML
#   - Invoking TPDG to generate all_tests.md from container YAML
#   - Checking both files exist and are non-zero size
#   - Validating content markers (test case IDs, sequence information, pass/fail status)
#     appear for multiple test cases in generated documentation
#   - Checking HTML generation from AsciiDoc (if asciidoctor available)
#   - Generating comprehensive validation report with statistics
#
# USAGE:
#   ./test-acceptance/validate_stage7_consolidated_docs.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose         Enable verbose output
#   -h, --help            Show this help message
#   --skip-html           Skip HTML generation validation
#   --skip-schema         Skip JSON schema validation
#   --min-file-size SIZE  Minimum expected file size in bytes (default: 100)
#   --title TITLE         Title for container YAML (default: "Acceptance Test Suite - All Test Cases")
#   --project PROJECT     Project name (default: "Test Case Manager - Acceptance Suite")
#   --environment ENV     Environment for metadata (default: "Automated Test Environment")
#
# EXIT CODES:
#   0 - All validations passed
#   1 - One or more validations failed
#
# OUTPUT:
#   Generates a detailed validation report with:
#   - Verifier --folder mode execution status
#   - Container YAML validation results
#   - Schema compliance checks
#   - Metadata accuracy verification
#   - TPDG documentation generation status
#   - File existence and size checks
#   - Content validation results (test case IDs, sequences, pass/fail)
#   - HTML generation status (if enabled)
#   - Summary statistics
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source logger and report generator libraries
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1
source "$REPO_ROOT/scripts/lib/report_generator.sh" || exit 1
source "$REPO_ROOT/scripts/lib/find-binary.sh" || exit 1

# Configuration
EXECUTION_LOGS_DIR="$SCRIPT_DIR/10_test_results/execution_logs"
TEST_CASES_DIR="$SCRIPT_DIR/00_test_cases"
REPORTS_DIR="$SCRIPT_DIR/30_documentation_source"
CONSOLIDATED_DIR="$REPORTS_DIR/consolidated"
CONTAINER_SCHEMA="$REPO_ROOT/data/testcase_results_container/schema.json"
VERBOSE=0
SKIP_HTML=0
SKIP_SCHEMA=0
MIN_FILE_SIZE=100
TITLE="Acceptance Test Suite - All Test Cases"
PROJECT="Test Case Manager - Acceptance Suite"
ENVIRONMENT="Automated Test Environment"

# File paths
CONSOLIDATED_CONTAINER="$CONSOLIDATED_DIR/all_tests_container.yaml"
CONSOLIDATED_ASCIIDOC="$CONSOLIDATED_DIR/all_tests.adoc"
CONSOLIDATED_MARKDOWN="$CONSOLIDATED_DIR/all_tests.md"
CONSOLIDATED_HTML="$CONSOLIDATED_DIR/all_tests.html"

# Validation counters
VERIFIER_SUCCESS=0
VERIFIER_FAILED=0
CONTAINER_EXISTS=0
CONTAINER_MISSING=0
SCHEMA_VALIDATION_PASSED=0
SCHEMA_VALIDATION_FAILED=0
METADATA_VALIDATION_PASSED=0
METADATA_VALIDATION_FAILED=0
ASCIIDOC_GENERATED=0
ASCIIDOC_FAILED=0
MARKDOWN_GENERATED=0
MARKDOWN_FAILED=0
HTML_GENERATED=0
HTML_FAILED=0
CONTENT_VALIDATION_PASSED=0
CONTENT_VALIDATION_FAILED=0
FILE_SIZE_PASSED=0
FILE_SIZE_FAILED=0

# Error tracking (bash 3.2 compatible)
ERRORS=""

# Append error helper
append_error() {
    if [[ -z "$ERRORS" ]]; then
        ERRORS="$1"
    else
        ERRORS="$ERRORS"$'\n'"$1"
    fi
}

# Usage function
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Validates Stage 7 (consolidated documentation generation) for acceptance testing.

OPTIONS:
    -v, --verbose         Enable verbose output
    -h, --help            Show this help message
    --skip-html           Skip HTML generation validation
    --skip-schema         Skip JSON schema validation
    --min-file-size SIZE  Minimum expected file size in bytes (default: 100)
    --title TITLE         Title for container YAML (default: auto-generated)
    --project PROJECT     Project name (default: "Test Case Manager - Acceptance Suite")
    --environment ENV     Environment for metadata (default: "Automated Test Environment")

DESCRIPTION:
    Validates consolidated documentation generation by:
    - Running verifier --folder mode to generate all_tests_container.yaml
    - Validating container YAML against schema
    - Verifying metadata section has correct statistics
    - Confirming total_test_cases matches number of execution logs
    - Generating AsciiDoc documentation using TPDG
    - Generating Markdown documentation using TPDG
    - Validating .adoc and .md file existence and content
    - Checking content markers (test case IDs, sequences, pass/fail)
    - Generating HTML from AsciiDoc (if asciidoctor available)
    - Verifying file sizes are non-zero and reasonable
    - Generating comprehensive validation report

EXIT CODES:
    0 - All validations passed
    1 - One or more validations failed

EOF
    exit 0
}

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--verbose)
                VERBOSE=1
                shift
                ;;
            -h|--help)
                usage
                ;;
            --skip-html)
                SKIP_HTML=1
                shift
                ;;
            --skip-schema)
                SKIP_SCHEMA=1
                shift
                ;;
            --min-file-size)
                MIN_FILE_SIZE="$2"
                shift 2
                ;;
            --title)
                TITLE="$2"
                shift 2
                ;;
            --project)
                PROJECT="$2"
                shift 2
                ;;
            --environment)
                ENVIRONMENT="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done
}

# Ensure directories exist
ensure_directories() {
    if [[ ! -d "$EXECUTION_LOGS_DIR" ]]; then
        log_error "Execution logs directory not found: $EXECUTION_LOGS_DIR"
        log_error "Please run validate_stage3_execution.sh first"
        exit 1
    fi
    
    mkdir -p "$CONSOLIDATED_DIR"
    
    log_verbose "Using execution logs directory: $EXECUTION_LOGS_DIR"
    log_verbose "Using consolidated output directory: $CONSOLIDATED_DIR"
}

# Check schema file exists
check_schema_file() {
    if [[ $SKIP_SCHEMA -eq 0 ]]; then
        if [[ ! -f "$CONTAINER_SCHEMA" ]]; then
            log_error "Container schema file not found: $CONTAINER_SCHEMA"
            exit 1
        fi
        log_verbose "Using container schema: $CONTAINER_SCHEMA"
    fi
}

# Find verifier binary
find_verifier_binary() {
    local verifier_path
    
    # Find verifier binary using workspace-aware search
    cd "$REPO_ROOT"
    verifier_path=$(find_binary "verifier")
    if [[ -z "$verifier_path" ]]; then
        log_error "Verifier binary not found. Please build it first with: cargo build --bin verifier"
        exit 1
    fi
    
    echo "$verifier_path"
}

# Check TPDG binary availability
check_tpdg_availability() {
    log_info "Checking test-plan-documentation-generator availability..."
    
    if check_test_plan_doc_gen_available; then
        local binary_path
        binary_path=$(find_test_plan_doc_gen)
        pass "test-plan-documentation-generator found: $binary_path"
        
        # Verify binary works
        if verify_test_plan_doc_gen_binary "$binary_path"; then
            pass "Binary verification successful"
        else
            log_error "Binary found but verification failed"
            append_error "TPDG binary verification failed"
            return 1
        fi
        
        return 0
    else
        fail "test-plan-documentation-generator not found"
        log_error "TPDG is not available in \$TEST_PLAN_DOC_GEN or PATH"
        log_error "Please install it with: cargo install test-plan-documentation-generator"
        append_error "TPDG binary not found"
        return 1
    fi
}

# Count execution log files
count_execution_logs() {
    local count
    count=$(find "$EXECUTION_LOGS_DIR" -type f -name "*.json" 2>/dev/null | wc -l | tr -d ' ')
    echo "$count"
}

# Run verifier in --folder mode to generate consolidated container YAML
run_verifier_folder_mode() {
    local verifier_path="$1"
    
    log_info "Running verifier --folder mode on execution logs..."
    log_verbose "  Execution logs directory: $EXECUTION_LOGS_DIR"
    log_verbose "  Output container: $CONSOLIDATED_CONTAINER"
    
    # Get hostname for environment metadata
    local hostname
    hostname=$(hostname 2>/dev/null || echo "unknown")
    local environment_full="${ENVIRONMENT} - ${hostname}"
    
    # Run verifier in folder mode
    if "$verifier_path" \
        --folder "$EXECUTION_LOGS_DIR" \
        --test-case-dir "$TEST_CASES_DIR" \
        --title "$TITLE" \
        --project "$PROJECT" \
        --environment "$environment_full" \
        --output "$CONSOLIDATED_CONTAINER" \
        >/dev/null 2>&1; then
        
        ((VERIFIER_SUCCESS++))
        pass "Verifier --folder mode completed successfully"
        return 0
    else
        ((VERIFIER_FAILED++))
        fail "Verifier --folder mode failed"
        append_error "Verifier --folder mode execution failed"
        return 1
    fi
}

# Validate container YAML exists
validate_container_exists() {
    log_verbose "Checking if container YAML was generated..."
    
    if [[ -f "$CONSOLIDATED_CONTAINER" ]]; then
        ((CONTAINER_EXISTS++))
        pass "Container YAML exists: all_tests_container.yaml"
        return 0
    else
        ((CONTAINER_MISSING++))
        fail "Container YAML not found: all_tests_container.yaml"
        append_error "Container YAML file not created"
        return 1
    fi
}

# Validate container YAML against schema
validate_container_schema() {
    log_verbose "Validating container YAML against schema..."
    
    # Find validate-yaml binary using workspace-aware search
    local validate_yaml
    cd "$REPO_ROOT"
    validate_yaml=$(find_binary "validate-yaml")
    if [[ -z "$validate_yaml" ]]; then
        log_warning "validate-yaml binary not found, skipping schema validation"
        return 0
    fi
    
    if "$validate_yaml" --schema "$CONTAINER_SCHEMA" "$CONSOLIDATED_CONTAINER" >/dev/null 2>&1; then
        ((SCHEMA_VALIDATION_PASSED++))
        pass "Schema validation passed"
        return 0
    else
        ((SCHEMA_VALIDATION_FAILED++))
        fail "Schema validation failed"
        append_error "Container YAML does not conform to schema"
        return 1
    fi
}

# Validate metadata section
validate_metadata() {
    log_verbose "Validating metadata section..."
    
    local python_cmd
    python_cmd=$(find_python)
    
    if [[ -z "$python_cmd" ]]; then
        log_verbose "  Python not available, skipping metadata validation"
        return 0
    fi
    
    local expected_count
    expected_count=$(count_execution_logs)
    
    # Validate metadata using Python
    local validation_output
    validation_output=$($python_cmd - "$CONSOLIDATED_CONTAINER" "$expected_count" 2>&1 << 'PYTHON_SCRIPT'
import sys
import yaml

container_file = sys.argv[1]
expected_count = int(sys.argv[2])

try:
    with open(container_file, 'r') as f:
        data = yaml.safe_load(f)
    
    metadata = data.get('metadata', {})
    
    # Check if metadata has statistics
    if 'total_test_cases' not in metadata:
        print("Metadata missing total_test_cases")
        sys.exit(1)
    
    if 'passed_test_cases' not in metadata:
        print("Metadata missing passed_test_cases")
        sys.exit(1)
    
    if 'failed_test_cases' not in metadata:
        print("Metadata missing failed_test_cases")
        sys.exit(1)
    
    # Verify total_test_cases matches expected count
    total_test_cases = metadata['total_test_cases']
    if total_test_cases != expected_count:
        print(f"Metadata total_test_cases ({total_test_cases}) doesn't match execution logs count ({expected_count})")
        sys.exit(1)
    
    # Verify statistics consistency
    test_results = data.get('test_results', [])
    actual_total = len(test_results)
    actual_passed = sum(1 for r in test_results if r.get('overall_pass', False))
    actual_failed = actual_total - actual_passed
    
    if metadata['total_test_cases'] != actual_total:
        print(f"Metadata total_test_cases ({metadata['total_test_cases']}) doesn't match test_results count ({actual_total})")
        sys.exit(1)
    
    if metadata['passed_test_cases'] != actual_passed:
        print(f"Metadata passed_test_cases ({metadata['passed_test_cases']}) doesn't match actual ({actual_passed})")
        sys.exit(1)
    
    if metadata['failed_test_cases'] != actual_failed:
        print(f"Metadata failed_test_cases ({metadata['failed_test_cases']}) doesn't match actual ({actual_failed})")
        sys.exit(1)
    
    sys.exit(0)
    
except Exception as e:
    print(f"Metadata validation error: {e}")
    sys.exit(1)
PYTHON_SCRIPT
)
    
    local exit_code=$?
    
    if [[ $exit_code -eq 0 ]]; then
        ((METADATA_VALIDATION_PASSED++))
        pass "Metadata validation passed"
        log_debug "  total_test_cases matches execution logs count"
        return 0
    else
        ((METADATA_VALIDATION_FAILED++))
        fail "Metadata validation failed"
        append_error "Metadata validation failed: $validation_output"
        log_debug "  $validation_output"
        return 1
    fi
}

# Generate AsciiDoc documentation
generate_asciidoc() {
    log_info "Generating AsciiDoc documentation..."
    log_verbose "  Output: $CONSOLIDATED_ASCIIDOC"
    
    # Invoke TPDG to generate AsciiDoc
    if invoke_test_plan_doc_gen \
        --input "$CONSOLIDATED_CONTAINER" \
        --output "$CONSOLIDATED_ASCIIDOC" \
        --format asciidoc >/dev/null 2>&1; then
        
        ((ASCIIDOC_GENERATED++))
        pass "AsciiDoc generated: all_tests.adoc"
        return 0
    else
        ((ASCIIDOC_FAILED++))
        fail "Failed to generate AsciiDoc"
        append_error "AsciiDoc generation failed"
        return 1
    fi
}

# Generate Markdown documentation
generate_markdown() {
    log_info "Generating Markdown documentation..."
    log_verbose "  Output: $CONSOLIDATED_MARKDOWN"
    
    # Invoke TPDG to generate Markdown
    if invoke_test_plan_doc_gen \
        --input "$CONSOLIDATED_CONTAINER" \
        --output "$CONSOLIDATED_MARKDOWN" \
        --format markdown >/dev/null 2>&1; then
        
        ((MARKDOWN_GENERATED++))
        pass "Markdown generated: all_tests.md"
        return 0
    else
        ((MARKDOWN_FAILED++))
        fail "Failed to generate Markdown"
        append_error "Markdown generation failed"
        return 1
    fi
}

# Validate file size
validate_file_size() {
    local file_path="$1"
    local file_type="$2"
    
    if [[ ! -f "$file_path" ]]; then
        log_debug "  ✗ $file_type file not found for size check"
        return 1
    fi
    
    # Get file size (portable across BSD and GNU)
    local file_size
    file_size=$(stat -f%z "$file_path" 2>/dev/null || stat -c%s "$file_path" 2>/dev/null || echo "0")
    
    if [[ "$file_size" -eq 0 ]]; then
        log_debug "  ✗ $file_type file is empty"
        append_error "$file_type file is empty (0 bytes)"
        return 1
    elif [[ "$file_size" -lt "$MIN_FILE_SIZE" ]]; then
        log_debug "  ✗ $file_type file is too small ($file_size bytes, minimum: $MIN_FILE_SIZE bytes)"
        append_error "$file_type file is too small ($file_size bytes)"
        return 1
    else
        log_debug "  ✓ $file_type file size OK ($file_size bytes)"
        return 0
    fi
}

# Validate AsciiDoc content markers
validate_asciidoc_content() {
    log_verbose "Validating AsciiDoc content markers..."
    
    if [[ ! -f "$CONSOLIDATED_ASCIIDOC" ]]; then
        log_debug "  ✗ AsciiDoc file not found"
        append_error "AsciiDoc file not found for content validation"
        return 1
    fi
    
    local validation_errors=0
    
    # Check for multiple test case IDs (should have at least 2 different test cases)
    local test_case_count
    test_case_count=$(grep -o "test_case_id\|Test Case ID\|ID:" "$CONSOLIDATED_ASCIIDOC" 2>/dev/null | wc -l | tr -d ' ')
    
    if [[ "$test_case_count" -lt 2 ]]; then
        log_debug "  ✗ Not enough test case references found (expected >= 2, found $test_case_count)"
        append_error "AsciiDoc missing multiple test case IDs"
        ((validation_errors++))
    else
        log_debug "  ✓ Multiple test case references found ($test_case_count)"
    fi
    
    # Check for sequence information
    if ! grep -qi "sequence\|seq" "$CONSOLIDATED_ASCIIDOC" 2>/dev/null; then
        log_debug "  ✗ Sequence information not found in AsciiDoc"
        append_error "AsciiDoc missing sequence information"
        ((validation_errors++))
    else
        log_debug "  ✓ Sequence information found"
    fi
    
    # Check for pass/fail status
    if ! grep -qi "pass\|fail\|status" "$CONSOLIDATED_ASCIIDOC" 2>/dev/null; then
        log_debug "  ✗ Pass/fail status not found in AsciiDoc"
        append_error "AsciiDoc missing pass/fail status"
        ((validation_errors++))
    else
        log_debug "  ✓ Pass/fail status found"
    fi
    
    # Check for test date
    if ! grep -q "test_date\|Test Date\|Date:" "$CONSOLIDATED_ASCIIDOC" 2>/dev/null; then
        log_debug "  ✗ test_date not found in AsciiDoc"
        append_error "AsciiDoc missing test_date"
        ((validation_errors++))
    else
        log_debug "  ✓ Test date found"
    fi
    
    # Check for step information
    if ! grep -qi "step\|result" "$CONSOLIDATED_ASCIIDOC" 2>/dev/null; then
        log_debug "  ✗ Step information not found in AsciiDoc"
        append_error "AsciiDoc missing step information"
        ((validation_errors++))
    else
        log_debug "  ✓ Step information found"
    fi
    
    if [[ $validation_errors -eq 0 ]]; then
        pass "AsciiDoc content validation passed"
        return 0
    else
        fail "AsciiDoc content validation failed ($validation_errors errors)"
        return 1
    fi
}

# Validate Markdown content markers
validate_markdown_content() {
    log_verbose "Validating Markdown content markers..."
    
    if [[ ! -f "$CONSOLIDATED_MARKDOWN" ]]; then
        log_debug "  ✗ Markdown file not found"
        append_error "Markdown file not found for content validation"
        return 1
    fi
    
    local validation_errors=0
    
    # Check for headers (# and ##)
    if ! grep -q "^#" "$CONSOLIDATED_MARKDOWN" 2>/dev/null; then
        log_debug "  ✗ No headers found in Markdown"
        append_error "Markdown missing headers"
        ((validation_errors++))
    else
        log_debug "  ✓ Headers found"
    fi
    
    # Check for tables (| symbols)
    if ! grep -q "|" "$CONSOLIDATED_MARKDOWN" 2>/dev/null; then
        log_debug "  ✗ No tables found in Markdown"
        append_error "Markdown missing tables"
        ((validation_errors++))
    else
        log_debug "  ✓ Tables found"
    fi
    
    # Check for multiple test case IDs
    local test_case_count
    test_case_count=$(grep -o "test_case_id\|Test Case ID\|ID:" "$CONSOLIDATED_MARKDOWN" 2>/dev/null | wc -l | tr -d ' ')
    
    if [[ "$test_case_count" -lt 2 ]]; then
        log_debug "  ✗ Not enough test case references found (expected >= 2, found $test_case_count)"
        append_error "Markdown missing multiple test case IDs"
        ((validation_errors++))
    else
        log_debug "  ✓ Multiple test case references found ($test_case_count)"
    fi
    
    # Check for status indicators
    if ! grep -qi "pass\|fail\|success\|✓\|✗" "$CONSOLIDATED_MARKDOWN" 2>/dev/null; then
        log_debug "  ✗ No status indicators found in Markdown"
        append_error "Markdown missing status indicators"
        ((validation_errors++))
    else
        log_debug "  ✓ Status indicators found"
    fi
    
    if [[ $validation_errors -eq 0 ]]; then
        pass "Markdown content validation passed"
        return 0
    else
        fail "Markdown content validation failed ($validation_errors errors)"
        return 1
    fi
}

# Generate HTML from AsciiDoc
generate_html_from_asciidoc() {
    log_info "Generating HTML from AsciiDoc..."
    
    # Check if asciidoctor is available
    if ! command -v asciidoctor >/dev/null 2>&1; then
        log_info "  asciidoctor not available, skipping HTML generation"
        return 2  # Special return code for "skipped"
    fi
    
    log_verbose "  Output: $CONSOLIDATED_HTML"
    
    # Generate HTML using asciidoctor
    if asciidoctor "$CONSOLIDATED_ASCIIDOC" -o "$CONSOLIDATED_HTML" >/dev/null 2>&1; then
        ((HTML_GENERATED++))
        pass "HTML generated: all_tests.html"
        
        # Validate HTML file size
        if validate_file_size "$CONSOLIDATED_HTML" "HTML"; then
            log_debug "  ✓ HTML file size validation passed"
        else
            log_debug "  ✗ HTML file size validation failed"
        fi
        
        return 0
    else
        ((HTML_FAILED++))
        fail "Failed to generate HTML from AsciiDoc"
        append_error "HTML generation failed"
        return 1
    fi
}

# Generate validation report
generate_report() {
    local report_file="$REPORTS_DIR/stage7_consolidated_docs_validation_report.txt"
    
    {
        echo "========================================="
        echo "Stage 7 Consolidated Documentation Validation Report"
        echo "========================================="
        echo ""
        echo "Generated: $(date)"
        echo ""
        
        echo "Summary"
        echo "-------"
        local total_logs
        total_logs=$(count_execution_logs)
        echo "Total execution logs: $total_logs"
        echo ""
        
        echo "Verifier --folder Mode:"
        if [[ $VERIFIER_SUCCESS -gt 0 ]]; then
            echo "  Status: Success"
        else
            echo "  Status: Failed"
        fi
        echo ""
        
        echo "Container YAML:"
        if [[ $CONTAINER_EXISTS -gt 0 ]]; then
            echo "  Exists: Yes"
        else
            echo "  Exists: No"
        fi
        echo ""
        
        if [[ $SKIP_SCHEMA -eq 0 ]]; then
            echo "Schema Validation:"
            echo "  Passed: $SCHEMA_VALIDATION_PASSED"
            echo "  Failed: $SCHEMA_VALIDATION_FAILED"
            echo ""
        fi
        
        echo "Metadata Validation:"
        echo "  Passed: $METADATA_VALIDATION_PASSED"
        echo "  Failed: $METADATA_VALIDATION_FAILED"
        echo ""
        
        echo "AsciiDoc Generation:"
        echo "  Generated: $ASCIIDOC_GENERATED"
        echo "  Failed: $ASCIIDOC_FAILED"
        echo ""
        
        echo "Markdown Generation:"
        echo "  Generated: $MARKDOWN_GENERATED"
        echo "  Failed: $MARKDOWN_FAILED"
        echo ""
        
        if [[ $SKIP_HTML -eq 0 ]]; then
            echo "HTML Generation:"
            echo "  Generated: $HTML_GENERATED"
            echo "  Failed: $HTML_FAILED"
            if ! command -v asciidoctor >/dev/null 2>&1; then
                echo "  Note: asciidoctor not available"
            fi
            echo ""
        fi
        
        echo "Content Validation:"
        echo "  Passed: $CONTENT_VALIDATION_PASSED"
        echo "  Failed: $CONTENT_VALIDATION_FAILED"
        echo ""
        
        echo "File Size Validation:"
        echo "  Passed: $FILE_SIZE_PASSED"
        echo "  Failed: $FILE_SIZE_FAILED"
        echo "  Minimum size: $MIN_FILE_SIZE bytes"
        echo ""
        
        # Generated files list
        echo "Generated Files:"
        if [[ -f "$CONSOLIDATED_CONTAINER" ]]; then
            echo "  ✓ all_tests_container.yaml"
        else
            echo "  ✗ all_tests_container.yaml (missing)"
        fi
        
        if [[ -f "$CONSOLIDATED_ASCIIDOC" ]]; then
            echo "  ✓ all_tests.adoc"
        else
            echo "  ✗ all_tests.adoc (missing)"
        fi
        
        if [[ -f "$CONSOLIDATED_MARKDOWN" ]]; then
            echo "  ✓ all_tests.md"
        else
            echo "  ✗ all_tests.md (missing)"
        fi
        
        if [[ $SKIP_HTML -eq 0 ]]; then
            if [[ -f "$CONSOLIDATED_HTML" ]]; then
                echo "  ✓ all_tests.html"
            else
                echo "  ✗ all_tests.html (missing)"
            fi
        fi
        echo ""
        
        # Errors section
        if [[ -n "$ERRORS" ]]; then
            echo "Validation Errors"
            echo "-----------------"
            
            # Print errors line by line (bash 3.2 compatible)
            local IFS=$'\n'
            local error_line
            for error_line in $ERRORS; do
                echo "  $error_line"
            done
            echo ""
        fi
        
        # Overall result
        local total_failed=$((VERIFIER_FAILED + CONTAINER_MISSING + SCHEMA_VALIDATION_FAILED + METADATA_VALIDATION_FAILED + ASCIIDOC_FAILED + MARKDOWN_FAILED + HTML_FAILED + CONTENT_VALIDATION_FAILED + FILE_SIZE_FAILED))
        
        echo "Overall Result"
        echo "--------------"
        if [[ $total_failed -eq 0 ]]; then
            echo "✓ All validations passed!"
        else
            echo "✗ Total failures: $total_failed"
        fi
        echo ""
        
        # Quality metrics
        echo "Quality Metrics"
        echo "---------------"
        if [[ $VERIFIER_SUCCESS -gt 0 ]]; then
            echo "  Verifier execution: Success"
            if [[ $ASCIIDOC_GENERATED -gt 0 ]] && [[ $MARKDOWN_GENERATED -gt 0 ]]; then
                echo "  Documentation generation: 100%"
            else
                echo "  Documentation generation: Incomplete"
            fi
        else
            echo "  Verifier execution: Failed"
        fi
        echo ""
        
    } > "$report_file"
    
    log_info "Report saved to: $report_file"
}

# Print validation report
print_report() {
    section "Stage 7 Consolidated Documentation Validation Report"
    
    echo ""
    local total_logs
    total_logs=$(count_execution_logs)
    log_info "Total execution logs: $total_logs"
    echo ""
    
    log_info "Verifier --folder Mode:"
    if [[ $VERIFIER_SUCCESS -gt 0 ]]; then
        pass "  Success"
    else
        fail "  Failed"
    fi
    echo ""
    
    log_info "Container YAML:"
    if [[ $CONTAINER_EXISTS -gt 0 ]]; then
        pass "  Exists"
    else
        fail "  Missing"
    fi
    echo ""
    
    if [[ $SKIP_SCHEMA -eq 0 ]]; then
        log_info "Schema Validation:"
        log_info "  Passed: $SCHEMA_VALIDATION_PASSED"
        log_info "  Failed: $SCHEMA_VALIDATION_FAILED"
        echo ""
    fi
    
    log_info "Metadata Validation:"
    log_info "  Passed: $METADATA_VALIDATION_PASSED"
    log_info "  Failed: $METADATA_VALIDATION_FAILED"
    echo ""
    
    log_info "AsciiDoc Generation:"
    log_info "  Generated: $ASCIIDOC_GENERATED"
    log_info "  Failed: $ASCIIDOC_FAILED"
    echo ""
    
    log_info "Markdown Generation:"
    log_info "  Generated: $MARKDOWN_GENERATED"
    log_info "  Failed: $MARKDOWN_FAILED"
    echo ""
    
    if [[ $SKIP_HTML -eq 0 ]]; then
        log_info "HTML Generation:"
        log_info "  Generated: $HTML_GENERATED"
        log_info "  Failed: $HTML_FAILED"
        if ! command -v asciidoctor >/dev/null 2>&1; then
            log_info "  Note: asciidoctor not available"
        fi
        echo ""
    fi
    
    log_info "Content Validation:"
    log_info "  Passed: $CONTENT_VALIDATION_PASSED"
    log_info "  Failed: $CONTENT_VALIDATION_FAILED"
    echo ""
    
    log_info "File Size Validation:"
    log_info "  Passed: $FILE_SIZE_PASSED"
    log_info "  Failed: $FILE_SIZE_FAILED"
    echo ""
    
    if [[ -n "$ERRORS" ]]; then
        section "Validation Errors"
        echo ""
        # Print errors line by line (bash 3.2 compatible)
        local IFS=$'\n'
        local error_line
        for error_line in $ERRORS; do
            fail "$error_line"
        done
        echo ""
    fi
    
    local total_failed=$((VERIFIER_FAILED + CONTAINER_MISSING + SCHEMA_VALIDATION_FAILED + METADATA_VALIDATION_FAILED + ASCIIDOC_FAILED + MARKDOWN_FAILED + HTML_FAILED + CONTENT_VALIDATION_FAILED + FILE_SIZE_FAILED))
    
    if [[ $total_failed -eq 0 ]]; then
        section "Validation Result"
        pass "All validations passed!"
        return 0
    else
        section "Validation Result"
        fail "Total failures: $total_failed"
        return 1
    fi
}

# Main function
main() {
    parse_args "$@"
    
    section "Stage 7 Consolidated Documentation Validation"
    
    log_info "Validating consolidated documentation generation..."
    echo ""
    
    # Ensure directories exist
    ensure_directories
    
    # Check schema file
    check_schema_file
    
    # Find verifier binary
    local verifier_path
    verifier_path=$(find_verifier_binary)
    log_verbose "Using verifier: $verifier_path"
    
    # Check TPDG availability
    local tpdg_available=0
    if check_tpdg_availability; then
        tpdg_available=1
    else
        log_error "TPDG is not available, cannot proceed with validation"
        
        # Generate report even if TPDG is not available
        generate_report
        print_report
        exit 1
    fi
    
    echo ""
    
    # Check if execution logs exist
    local log_count
    log_count=$(count_execution_logs)
    
    if [[ "$log_count" -eq 0 ]]; then
        log_warning "No execution logs found in $EXECUTION_LOGS_DIR"
        log_info "Please run validate_stage3_execution.sh first"
        
        # Generate report
        generate_report
        print_report
        exit 0
    fi
    
    log_info "Found $log_count execution log(s)"
    echo ""
    
    # Step 1: Run verifier --folder mode
    if ! run_verifier_folder_mode "$verifier_path"; then
        log_error "Failed to generate consolidated container YAML"
    fi
    
    # Step 2: Validate container exists
    if ! validate_container_exists; then
        log_error "Container YAML not generated"
        
        # Generate report and exit
        generate_report
        print_report
        exit 1
    fi
    
    # Step 3: Validate schema (if not skipped)
    if [[ $SKIP_SCHEMA -eq 0 ]]; then
        validate_container_schema
    fi
    
    # Step 4: Validate metadata
    validate_metadata
    
    echo ""
    
    # Step 5: Generate AsciiDoc
    if generate_asciidoc; then
        # Validate AsciiDoc file size
        if validate_file_size "$CONSOLIDATED_ASCIIDOC" "AsciiDoc"; then
            ((FILE_SIZE_PASSED++))
        else
            ((FILE_SIZE_FAILED++))
        fi
        
        # Validate AsciiDoc content
        if validate_asciidoc_content; then
            ((CONTENT_VALIDATION_PASSED++))
        else
            ((CONTENT_VALIDATION_FAILED++))
        fi
        
        # Generate HTML (if not skipped)
        if [[ $SKIP_HTML -eq 0 ]]; then
            generate_html_from_asciidoc
        fi
    fi
    
    echo ""
    
    # Step 6: Generate Markdown
    if generate_markdown; then
        # Validate Markdown file size
        if validate_file_size "$CONSOLIDATED_MARKDOWN" "Markdown"; then
            ((FILE_SIZE_PASSED++))
        else
            ((FILE_SIZE_FAILED++))
        fi
        
        # Validate Markdown content
        if validate_markdown_content; then
            ((CONTENT_VALIDATION_PASSED++))
        else
            ((CONTENT_VALIDATION_FAILED++))
        fi
    fi
    
    echo ""
    
    # Generate report file
    generate_report
    
    # Print report and exit with appropriate code
    if print_report; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"
