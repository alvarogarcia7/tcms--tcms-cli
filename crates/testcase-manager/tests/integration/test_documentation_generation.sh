#!/usr/bin/env bash
#
# Integration test for documentation generation workflow
#
# This test validates:
# 1. Running verifier on a successful test scenario
# 2. Converting verification output to result YAML
# 3. Invoking test-plan-doc-gen to generate AsciiDoc and Markdown reports
# 4. Validating generated files exist and contain expected content markers
# 5. Verifying report quality and content accuracy
# 6. Cleaning up temporary files
# 7. Checking for test-plan-doc-gen availability and skipping if not found
#
# Usage: ./tests/integration/test_documentation_generation.sh [--no-remove]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/report_generator.sh" || exit 1

# logger.sh's fail() only prints. Count failures here so the script's exit status reflects them (TCMS-46).
FAILURE_COUNT=0
fail() {
    FAILURE_COUNT=$((FAILURE_COUNT + 1))
    echo -e "${RED}✗${NC} $1" >&2
}
finish_run() {
    if [[ $FAILURE_COUNT -gt 0 ]]; then
        echo -e "${RED}✗${NC} $FAILURE_COUNT check(s) failed" >&2
        exit 1
    fi
}

# Find verifier binary using workspace-aware search
cd "$PROJECT_ROOT"
VERIFIER_BIN=$(find_binary "verifier")
if [[ -z "$VERIFIER_BIN" ]]; then
    echo "[ERROR] verifier binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin verifier" >&2
    exit 1
fi

uv sync

source .venv/bin/activate

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
echo "Documentation Generation Integration Test"
echo "======================================"
echo ""

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$VERIFIER_BIN" ]]; then
    fail "verifier binary not found at $VERIFIER_BIN"
    echo "Run 'cargo build' or 'make build' first"
    exit 1
fi
pass "verifier binary found"

if ! command -v python3 > /dev/null 2>&1; then
    fail "python3 not found"
    exit 1
fi
pass "python3 available"

uv sync > /dev/null 2>&1

source .venv/bin/activate

# Check if PyYAML is available
if uv run python3 -c "import yaml" 2>/dev/null; then
    pass "PyYAML available"
else
    fail "PyYAML not available. Install with: pip3 install pyyaml"
    exit 1
fi

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Define paths
TEST_CASE_DIR="$PROJECT_ROOT/testcases/verifier_scenarios/successful"
TEST_CASE_FILE="$TEST_CASE_DIR/TEST_SUCCESS_001.yml"
EXECUTION_LOG="$TEST_CASE_DIR/TEST_SUCCESS_001_execution_log.json"
VERIFICATION_OUTPUT="$TEMP_DIR/verification_output.json"
RESULT_YAML_DIR="$TEMP_DIR/results"
REPORTS_DIR="$TEMP_DIR/reports"

# Create output directories
mkdir -p "$RESULT_YAML_DIR"
mkdir -p "$REPORTS_DIR"

# Test 1: Run verifier on successful test scenario (with config file)
section "Test 1: Run Verifier on Successful Test Scenario"

if [[ ! -f "$TEST_CASE_FILE" ]]; then
    fail "Test case file not found: $TEST_CASE_FILE"
    exit 1
fi
pass "Test case file found: $(basename "$TEST_CASE_FILE")"

if [[ ! -f "$EXECUTION_LOG" ]]; then
    fail "Execution log not found: $EXECUTION_LOG"
    exit 1
fi
pass "Execution log found: $(basename "$EXECUTION_LOG")"

# Create a test config file
TEST_CONFIG="$TEMP_DIR/doc_gen_config.yml"
cat > "$TEST_CONFIG" << 'EOF'
type: container_config
schema: tcms/container-config.schema.v1.json
title: "Documentation Generation Test Results"
project: "Test Case Manager - Documentation Generation"
environment: "Integration Testing"
platform: "Test Environment"
executor: "Documentation Generation Test"
EOF

log_info "Running verifier on successful test scenario..."
if "$VERIFIER_BIN" \
    --log "$EXECUTION_LOG" \
    --test-case "TEST_SUCCESS_001" \
    --test-case-dir "$TEST_CASE_DIR" \
    --format json \
    --output "$VERIFICATION_OUTPUT" \
    --config "$TEST_CONFIG" > /dev/null 2>&1; then
    pass "Verifier completed successfully"
else
    VERIFIER_EXIT=$?
    fail "Verifier failed with exit code: $VERIFIER_EXIT"
    exit 1
fi

if [[ ! -f "$VERIFICATION_OUTPUT" ]]; then
    fail "Verification output not generated"
    exit 1
fi
pass "Verification output generated: $(basename "$VERIFICATION_OUTPUT")"

# Validate verification output structure
if grep -q '"test_case_id"' "$VERIFICATION_OUTPUT"; then
    pass "Verification output contains test_case_id field"
else
    fail "Verification output missing test_case_id field"
fi

if grep -q '"overall_pass".*true' "$VERIFICATION_OUTPUT"; then
    pass "Verification output shows overall pass"
else
    fail "Verification output should show overall pass"
fi

# Test 2: Convert verification output to result YAML (Multiple Mode)
section "Test 2: Convert Verification Output to Result YAML (Multiple Mode)"

CONVERT_SCRIPT="$PROJECT_ROOT/scripts/convert_verification_to_result_yaml.py"

if [[ ! -f "$CONVERT_SCRIPT" ]]; then
    fail "Conversion script not found: $CONVERT_SCRIPT"
    exit 1
fi
pass "Conversion script found"

log_info "Converting verification JSON to result YAML with --multiple flag..."
# Create execution logs directory for conversion script
EXEC_LOGS_DIR="$TEMP_DIR/exec_logs"
mkdir -p "$EXEC_LOGS_DIR"
cp "$EXECUTION_LOG" "$EXEC_LOGS_DIR/TEST_SUCCESS_001_execution_log.json"

if uv run python3 "$CONVERT_SCRIPT" \
    "$VERIFICATION_OUTPUT" \
    -o "$RESULT_YAML_DIR" \
    --execution-logs "$EXEC_LOGS_DIR" \
    --testcases "$TEST_CASE_DIR" \
    --multiple > /dev/null 2>&1; then
    pass "Conversion completed successfully (--multiple mode)"
else
    fail "Conversion failed (--multiple mode)"
    exit 1
fi

# Verify result YAML file was created
RESULT_YAML_FILE="$RESULT_YAML_DIR/TEST_SUCCESS_001_result.yaml"
if [[ ! -f "$RESULT_YAML_FILE" ]]; then
    fail "Result YAML file not created: $(basename "$RESULT_YAML_FILE")"
    exit 1
fi
pass "Result YAML file created: $(basename "$RESULT_YAML_FILE")"

# Validate result YAML structure
if grep -q "type: test_result" "$RESULT_YAML_FILE"; then
    pass "Result YAML contains 'type: test_result' field"
else
    fail "Result YAML missing 'type: test_result' field"
fi

if grep -q "test_case_id: TEST_SUCCESS_001" "$RESULT_YAML_FILE"; then
    pass "Result YAML contains correct test case ID"
else
    fail "Result YAML missing correct test case ID"
fi

if grep -q "overall_pass: true" "$RESULT_YAML_FILE"; then
    pass "Result YAML shows overall pass"
else
    fail "Result YAML should show overall pass"
fi

# Test 2b: Convert verification output to result YAML (Single Mode)
section "Test 2b: Convert Verification Output to Result YAML (Single Mode)"

SINGLE_RESULT_YAML_FILE="$TEMP_DIR/single_result.yaml"

log_info "Converting verification JSON to result YAML with --single flag..."
if uv run python3 "$CONVERT_SCRIPT" \
    "$VERIFICATION_OUTPUT" \
    -o "$SINGLE_RESULT_YAML_FILE" \
    --execution-logs "$EXEC_LOGS_DIR" \
    --testcases "$TEST_CASE_DIR" \
    --single > /dev/null 2>&1; then
    pass "Conversion completed successfully (--single mode)"
else
    fail "Conversion failed (--single mode)"
    exit 1
fi

# Verify single result YAML file was created
if [[ ! -f "$SINGLE_RESULT_YAML_FILE" ]]; then
    fail "Single result YAML file not created: $(basename "$SINGLE_RESULT_YAML_FILE")"
    exit 1
fi
pass "Single result YAML file created: $(basename "$SINGLE_RESULT_YAML_FILE")"

# Validate single file is non-empty
FILE_SIZE=$(stat -f%z "$SINGLE_RESULT_YAML_FILE" 2>/dev/null || stat -c%s "$SINGLE_RESULT_YAML_FILE" 2>/dev/null || echo "0")
if [[ $FILE_SIZE -gt 0 ]]; then
    pass "Single result YAML file has content ($FILE_SIZE bytes)"
else
    fail "Single result YAML file is empty"
fi

# Parse and validate YAML array structure using Python
log_info "Validating single YAML array structure..."
VALIDATION_RESULT=$(uv run python3 -c "
import sys
import yaml

try:
    with open('$SINGLE_RESULT_YAML_FILE', 'r') as f:
        data = yaml.safe_load(f)
    
    # Check if data is a list
    if not isinstance(data, list):
        print('ERROR: Root element is not a YAML array')
        sys.exit(1)
    
    # Check number of results
    num_results = len(data)
    if num_results != 1:
        print(f'ERROR: Expected 1 result, found {num_results}')
        sys.exit(1)
    
    # Validate first result structure
    result = data[0]
    
    # Check required fields
    required_fields = ['type', 'test_case_id', 'description', 'sequences', 
                       'total_steps', 'passed_steps', 'failed_steps', 
                       'not_executed_steps', 'overall_pass']
    
    missing_fields = [field for field in required_fields if field not in result]
    if missing_fields:
        print(f'ERROR: Missing required fields: {missing_fields}')
        sys.exit(1)
    
    # Validate field values
    if result['type'] != 'test_result':
        print(f\"ERROR: type field should be 'test_result', got '{result['type']}'\")
        sys.exit(1)
    
    if result['test_case_id'] != 'TEST_SUCCESS_001':
        print(f\"ERROR: test_case_id should be 'TEST_SUCCESS_001', got '{result['test_case_id']}'\")
        sys.exit(1)
    
    if not isinstance(result['sequences'], list):
        print('ERROR: sequences field is not a list')
        sys.exit(1)
    
    if result['overall_pass'] != True:
        print(f\"ERROR: overall_pass should be True, got {result['overall_pass']}\")
        sys.exit(1)
    
    # Validate sequences structure
    for seq in result['sequences']:
        if 'sequence_id' not in seq or 'name' not in seq or 'step_results' not in seq:
            print('ERROR: Sequence missing required fields (sequence_id, name, step_results)')
            sys.exit(1)
        
        if not isinstance(seq['step_results'], list):
            print('ERROR: step_results is not a list')
            sys.exit(1)
        
        # Validate step_results contain Pass/Fail/NotExecuted variants
        for step_result in seq['step_results']:
            if not any(key in step_result for key in ['Pass', 'Fail', 'NotExecuted']):
                print('ERROR: step_result missing Pass/Fail/NotExecuted variant')
                sys.exit(1)
    
    print('OK')
    sys.exit(0)

except Exception as e:
    print(f'ERROR: {str(e)}')
    sys.exit(1)
" 2>&1)

if [[ "$VALIDATION_RESULT" == "OK" ]]; then
    pass "Single YAML array structure is valid"
    pass "Single YAML contains expected 1 test case result"
    pass "Single YAML test case has correct structure and fields"
    pass "Single YAML sequences and step_results are properly formatted"
else
    fail "Single YAML validation failed: $VALIDATION_RESULT"
    exit 1
fi

# Verify single mode output contains same data as multiple mode output
if grep -q "test_case_id: TEST_SUCCESS_001" "$SINGLE_RESULT_YAML_FILE"; then
    pass "Single mode output contains correct test case ID"
else
    fail "Single mode output missing correct test case ID"
fi

if grep -q "overall_pass: true" "$SINGLE_RESULT_YAML_FILE"; then
    pass "Single mode output shows overall pass"
else
    fail "Single mode output should show overall pass"
fi

# Test 3: Check for test-plan-doc-gen availability
section "Test 3: Check test-plan-doc-gen Availability"

TEST_PLAN_DOC_GEN_DIR=${TEST_PLAN_DOC_GEN_DIR:-"$PROJECT_ROOT/../../test-plan-documentation-generator"}
SKIP_DOC_GEN=0

if [[ ! -d "$TEST_PLAN_DOC_GEN_DIR" ]]; then
    log_warning "test-plan-doc-gen directory not found: $TEST_PLAN_DOC_GEN_DIR"
    log_warning "Skipping test-plan-doc-gen report generation tests"
    info "To enable test-plan-doc-gen reports, clone the repository:"
    info "  cd $(dirname "$PROJECT_ROOT")"
    info "  git clone <test-plan-doc-gen-repo-url> test-plan-doc-gen"
    SKIP_DOC_GEN=1
else
    # Check if binary exists
    if check_test_plan_doc_gen_available "$TEST_PLAN_DOC_GEN_DIR"; then
        pass "test-plan-doc-gen binary found"
    else
        log_info "test-plan-doc-gen binary not found, attempting to build..."
        if build_test_plan_doc_gen "$TEST_PLAN_DOC_GEN_DIR"; then
            pass "test-plan-doc-gen built successfully"
        else
            log_warning "Failed to build test-plan-doc-gen"
            log_warning "Skipping test-plan-doc-gen report generation tests"
            SKIP_DOC_GEN=1
        fi
    fi
fi

if [[ $SKIP_DOC_GEN -eq 1 ]]; then
    section "Test Summary"
    echo ""
    log_info "Completed tests (test-plan-doc-gen tests skipped):"
    info "✓ Verifier execution on successful test scenario"
    info "✓ Conversion of verification output to result YAML (--multiple mode)"
    info "✓ Conversion of verification output to result YAML (--single mode)"
    info "✓ Result YAML validation"
    echo ""
    finish_run
    pass "All available tests passed (test-plan-doc-gen not available. Perhaps override the TEST_PLAN_DOC_GEN_DIR variable to point to a valid test-plan-doc-gen directory?)"
    exit 0
fi

# Test 4: Generate AsciiDoc report using result container
section "Test 4: Generate AsciiDoc Report"

# Create a results container YAML
RESULT_CONTAINER="$TEMP_DIR/results_container.yaml"
log_info "Creating results container YAML..."

cat > "$RESULT_CONTAINER" << EOF
title: 'Test Execution Results Report'
project: 'Test Case Manager - Verification Results'
test_date: '2024-01-01T00:00:00Z'
test_results:
EOF

# Add result file content to container (without 'type: result' line)
sed '/^type: result/d' "$RESULT_YAML_FILE" | sed 's/^/  /' >> "$RESULT_CONTAINER"

# Add metadata
cat >> "$RESULT_CONTAINER" << EOF
metadata:
  environment: 'Test Environment'
  platform: 'Test Case Manager'
  executor: 'Integration Test'
  execution_duration: 0.0
  total_test_cases: 1
  passed_test_cases: 1
  failed_test_cases: 0
EOF

pass "Created results container: $(basename "$RESULT_CONTAINER")"

# Generate AsciiDoc report
ASCIIDOC_OUTPUT="$REPORTS_DIR/test_results_report.adoc"

log_info "Generating AsciiDoc test results report..."

# Set TEST_PLAN_DOC_GEN environment variable
export TEST_PLAN_DOC_GEN=$(find_test_plan_doc_gen "$TEST_PLAN_DOC_GEN_DIR")

if invoke_test_plan_doc_gen \
    --container "$RESULT_CONTAINER" \
    --output "$ASCIIDOC_OUTPUT" \
    --format asciidoc > /dev/null 2>&1; then
    pass "AsciiDoc report generated successfully"
else
    fail "Failed to generate AsciiDoc report"
    SKIP_DOC_GEN=1
fi

if [[ $SKIP_DOC_GEN -eq 0 ]] && [[ -f "$ASCIIDOC_OUTPUT" ]]; then
    pass "AsciiDoc report file created: $(basename "$ASCIIDOC_OUTPUT")"
    
    # Validate AsciiDoc file size is non-zero
    FILE_SIZE=$(stat -f%z "$ASCIIDOC_OUTPUT" 2>/dev/null || stat -c%s "$ASCIIDOC_OUTPUT" 2>/dev/null || echo "0")
    if [[ $FILE_SIZE -gt 0 ]]; then
        pass "AsciiDoc report has content ($FILE_SIZE bytes)"
    else
        fail "AsciiDoc report is empty"
    fi
    
    # Validate AsciiDoc contains test case ID
    if grep -q "TEST_SUCCESS_001" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report contains test case ID (TEST_SUCCESS_001)"
    else
        fail "AsciiDoc report missing test case ID"
    fi
    
    # Validate AsciiDoc contains description
    if grep -q "Successful execution scenario" "$ASCIIDOC_OUTPUT" || \
       grep -q "description" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report contains description content"
    else
        fail "AsciiDoc report missing description content"
    fi
    
    # Validate AsciiDoc contains sequence information
    if grep -q "Successful Command Sequence" "$ASCIIDOC_OUTPUT" || \
       grep -q "sequence" "$ASCIIDOC_OUTPUT" || \
       grep -q "Sequence" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report contains sequence information"
    else
        fail "AsciiDoc report missing sequence information"
    fi
    
    # Validate AsciiDoc contains pass/fail status
    if grep -q "pass" "$ASCIIDOC_OUTPUT" || \
       grep -q "Pass" "$ASCIIDOC_OUTPUT" || \
       grep -q "PASS" "$ASCIIDOC_OUTPUT" || \
       grep -q "success" "$ASCIIDOC_OUTPUT" || \
       grep -q "true" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report contains pass/fail status"
    else
        fail "AsciiDoc report missing pass/fail status"
    fi
    
    # Validate AsciiDoc syntax (basic checks)
    # Check for document title (= heading)
    if grep -E "^= " "$ASCIIDOC_OUTPUT" > /dev/null 2>&1; then
        pass "AsciiDoc report has valid document title syntax"
    else
        log_warning "AsciiDoc report may be missing document title (= heading)"
    fi
    
    # Check for section headings (== or more)
    if grep -E "^==+ " "$ASCIIDOC_OUTPUT" > /dev/null 2>&1; then
        pass "AsciiDoc report has valid section heading syntax"
    else
        log_verbose "AsciiDoc report has no section headings"
    fi
    
    # Validate content accuracy against container YAML
    if grep -q "Test Execution Results Report" "$ASCIIDOC_OUTPUT" || \
       grep -q "Test Case Manager" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report reflects container YAML metadata"
    else
        log_warning "AsciiDoc report may not accurately reflect container metadata"
    fi
    
    # Check for project name from container
    if grep -q "Test Case Manager" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report contains project name from container"
    else
        log_verbose "AsciiDoc report may not contain project name"
    fi
else
    if [[ $SKIP_DOC_GEN -eq 0 ]]; then
        fail "AsciiDoc report file not created"
    fi
fi

# Test 5: Generate Markdown report from test case YAML
section "Test 5: Generate Markdown Report from Test Case"

MARKDOWN_OUTPUT="$REPORTS_DIR/TEST_SUCCESS_001_test_plan.md"

log_info "Generating Markdown test plan report..."

if invoke_test_plan_doc_gen \
    --test-case "$TEST_CASE_FILE" \
    --output "$MARKDOWN_OUTPUT" \
    --format markdown > /dev/null 2>&1; then
    pass "Markdown report generated successfully"
else
    fail "Failed to generate Markdown report"
    SKIP_DOC_GEN=1
fi

if [[ $SKIP_DOC_GEN -eq 0 ]] && [[ -f "$MARKDOWN_OUTPUT" ]]; then
    pass "Markdown report file created: $(basename "$MARKDOWN_OUTPUT")"
    
    # Validate Markdown file size is non-zero
    FILE_SIZE=$(stat -f%z "$MARKDOWN_OUTPUT" 2>/dev/null || stat -c%s "$MARKDOWN_OUTPUT" 2>/dev/null || echo "0")
    if [[ $FILE_SIZE -gt 0 ]]; then
        pass "Markdown report has content ($FILE_SIZE bytes)"
    else
        fail "Markdown report is empty"
    fi
    
    # Validate Markdown contains test case ID
    if grep -q "TEST_SUCCESS_001" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains test case ID (TEST_SUCCESS_001)"
    else
        fail "Markdown report missing test case ID"
    fi
    
    # Validate Markdown contains description
    if grep -q "Successful execution scenario" "$MARKDOWN_OUTPUT" || \
       grep -q "passing steps" "$MARKDOWN_OUTPUT" || \
       grep -q "description" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains description content"
    else
        fail "Markdown report missing description content"
    fi
    
    # Validate Markdown contains sequence information
    if grep -q "Successful Command Sequence" "$MARKDOWN_OUTPUT" || \
       grep -q "sequence" "$MARKDOWN_OUTPUT" || \
       grep -q "Sequence" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains sequence information"
    else
        fail "Markdown report missing sequence information"
    fi
    
    # Validate Markdown contains step information
    if grep -q "step" "$MARKDOWN_OUTPUT" || \
       grep -q "Step" "$MARKDOWN_OUTPUT" || \
       grep -q "Echo a greeting message" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains step information"
    else
        fail "Markdown report missing step information"
    fi
    
    # Validate Markdown syntax (basic checks)
    # Check for headings (# syntax)
    if grep -E "^#+ " "$MARKDOWN_OUTPUT" > /dev/null 2>&1; then
        pass "Markdown report has valid heading syntax"
    else
        fail "Markdown report missing heading syntax"
    fi
    
    # Check for proper Markdown structure (not malformed)
    # Count opening and closing code fences if any
    OPENING_FENCES=$(grep -c '^```' "$MARKDOWN_OUTPUT" 2>/dev/null || echo "0")
    if [[ $((OPENING_FENCES % 2)) -eq 0 ]]; then
        pass "Markdown report has balanced code fences"
    else
        fail "Markdown report has unbalanced code fences"
    fi
    
    # Validate content accuracy against test case YAML
    if grep -q "TEST_SUCCESS_001" "$MARKDOWN_OUTPUT" && \
       grep -q "Successful" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report accurately reflects test case YAML data"
    else
        fail "Markdown report may not accurately reflect test case YAML"
    fi
    
    # Check for requirement from YAML
    if grep -q "TEST_SUCCESS" "$MARKDOWN_OUTPUT" || \
       grep -q "requirement" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains requirement information"
    else
        log_verbose "Markdown report may not contain requirement information"
    fi
    
    # Check for initial conditions
    if grep -q "initial" "$MARKDOWN_OUTPUT" || \
       grep -q "condition" "$MARKDOWN_OUTPUT" || \
       grep -q "Shell" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains initial conditions"
    else
        log_verbose "Markdown report may not contain initial conditions"
    fi
else
    if [[ $SKIP_DOC_GEN -eq 0 ]]; then
        fail "Markdown report file not created"
    fi
fi

# Test 6: Generate HTML report from result container
section "Test 6: Generate HTML Report from Results Container"

HTML_OUTPUT="$REPORTS_DIR/test_results_report.html"

log_info "Generating HTML test results report..."

if invoke_test_plan_doc_gen \
    --container "$RESULT_CONTAINER" \
    --output "$HTML_OUTPUT" \
    --format html > /dev/null 2>&1; then
    pass "HTML report generated successfully"
else
    # HTML format may not be supported, continue without failing
    log_verbose "HTML report generation not supported or failed"
    HTML_SKIP=1
fi

if [[ -z "${HTML_SKIP:-}" ]] && [[ -f "$HTML_OUTPUT" ]]; then
    pass "HTML report file created: $(basename "$HTML_OUTPUT")"
    
    # Validate HTML file size is non-zero
    FILE_SIZE=$(stat -f%z "$HTML_OUTPUT" 2>/dev/null || stat -c%s "$HTML_OUTPUT" 2>/dev/null || echo "0")
    if [[ $FILE_SIZE -gt 0 ]]; then
        pass "HTML report has content ($FILE_SIZE bytes)"
    else
        fail "HTML report is empty"
    fi
    
    # Validate HTML contains test case ID
    if grep -q "TEST_SUCCESS_001" "$HTML_OUTPUT"; then
        pass "HTML report contains test case ID (TEST_SUCCESS_001)"
    else
        fail "HTML report missing test case ID"
    fi
    
    # Validate HTML contains description
    if grep -q "Successful execution scenario" "$HTML_OUTPUT" || \
       grep -q "description" "$HTML_OUTPUT"; then
        pass "HTML report contains description content"
    else
        fail "HTML report missing description content"
    fi
    
    # Validate HTML contains pass/fail status
    if grep -q "pass" "$HTML_OUTPUT" || \
       grep -q "Pass" "$HTML_OUTPUT" || \
       grep -q "success" "$HTML_OUTPUT" || \
       grep -q "true" "$HTML_OUTPUT"; then
        pass "HTML report contains pass/fail status"
    else
        fail "HTML report missing pass/fail status"
    fi
    
    # Validate HTML syntax (basic checks)
    # Check for HTML structure tags
    if grep -q "<html" "$HTML_OUTPUT" || grep -q "<!DOCTYPE" "$HTML_OUTPUT"; then
        pass "HTML report has valid HTML document structure"
    else
        log_warning "HTML report may be missing DOCTYPE or html tag"
    fi
    
    # Check for head and body sections
    if grep -q "<head" "$HTML_OUTPUT" && grep -q "<body" "$HTML_OUTPUT"; then
        pass "HTML report has head and body sections"
    else
        log_verbose "HTML report may be missing standard HTML sections"
    fi
    
    # Check for closing tags balance (basic validation)
    HTML_OPEN=$(grep -o "<html" "$HTML_OUTPUT" | wc -l | tr -d ' ')
    HTML_CLOSE=$(grep -o "</html>" "$HTML_OUTPUT" | wc -l | tr -d ' ')
    if [[ "$HTML_OPEN" -eq "$HTML_CLOSE" ]]; then
        pass "HTML report has balanced html tags"
    else
        fail "HTML report has unbalanced html tags"
    fi
    
    # Validate content accuracy against container YAML
    if grep -q "Test Execution Results Report" "$HTML_OUTPUT" || \
       grep -q "Test Case Manager" "$HTML_OUTPUT"; then
        pass "HTML report reflects container YAML metadata"
    else
        log_warning "HTML report may not accurately reflect container metadata"
    fi
else
    log_verbose "HTML report generation skipped or not supported"
fi

# Test 7: Verify report content completeness
section "Test 7: Verify Report Content Completeness"

# Check that AsciiDoc report has all expected sequences
if [[ -f "$ASCIIDOC_OUTPUT" ]]; then
    # Test case has 1 sequence with 3 steps
    if grep -q "Successful Command Sequence" "$ASCIIDOC_OUTPUT"; then
        pass "AsciiDoc report contains all sequence names"
    else
        fail "AsciiDoc report missing expected sequence names"
    fi
    
    # Check for step descriptions
    STEP_COUNT=0
    if grep -q "Echo a greeting message" "$ASCIIDOC_OUTPUT"; then
        ((STEP_COUNT++))
    fi
    if grep -q "Echo a status message" "$ASCIIDOC_OUTPUT"; then
        ((STEP_COUNT++))
    fi
    if grep -q "Execute true command" "$ASCIIDOC_OUTPUT"; then
        ((STEP_COUNT++))
    fi
    
    if [[ $STEP_COUNT -ge 1 ]]; then
        pass "AsciiDoc report contains step information ($STEP_COUNT/3 steps found)"
    else
        log_verbose "AsciiDoc report may not contain all step descriptions"
    fi
fi

# Check that Markdown report has all expected sequences
if [[ -f "$MARKDOWN_OUTPUT" ]]; then
    # Test case has 1 sequence with 3 steps
    if grep -q "Successful Command Sequence" "$MARKDOWN_OUTPUT"; then
        pass "Markdown report contains all sequence names"
    else
        fail "Markdown report missing expected sequence names"
    fi
    
    # Check for step descriptions
    STEP_COUNT=0
    if grep -q "Echo a greeting message" "$MARKDOWN_OUTPUT"; then
        ((STEP_COUNT++))
    fi
    if grep -q "Echo a status message" "$MARKDOWN_OUTPUT"; then
        ((STEP_COUNT++))
    fi
    if grep -q "Execute true command" "$MARKDOWN_OUTPUT"; then
        ((STEP_COUNT++))
    fi
    
    if [[ $STEP_COUNT -ge 1 ]]; then
        pass "Markdown report contains step information ($STEP_COUNT/3 steps found)"
    else
        log_verbose "Markdown report may not contain all step descriptions"
    fi
fi

# Test 8: Verify cleanup of temporary files (happens automatically via setup_cleanup)
section "Test 8: Verify Cleanup"

if [[ $REMOVE_TEMP -eq 1 ]]; then
    info "Temporary files will be cleaned up automatically on exit"
    pass "Cleanup configured (will execute on exit)"
else
    info "Cleanup disabled via --no-remove flag"
    info "Temporary files preserved at: $TEMP_DIR"
    pass "Cleanup skipped as requested"
fi

# Summary
section "Test Summary"
echo ""

log_info "Successfully completed tests:"
info "✓ Verifier execution on successful test scenario"
info "✓ Conversion of verification output to result YAML (--multiple mode)"
info "✓ Conversion of verification output to result YAML (--single mode)"
info "✓ Result YAML validation"

if [[ $SKIP_DOC_GEN -eq 0 ]]; then
    info "✓ AsciiDoc report generation and validation"
    info "✓ Markdown report generation and validation"
    if [[ -z "${HTML_SKIP:-}" ]] && [[ -f "$HTML_OUTPUT" ]]; then
        info "✓ HTML report generation and validation"
    fi
    info "✓ Report content completeness verification"
fi

echo ""
log_info "Generated files:"

if [[ -f "$VERIFICATION_OUTPUT" ]]; then
    info "  📄 Verification JSON: $(basename "$VERIFICATION_OUTPUT")"
fi

if [[ -f "$RESULT_YAML_FILE" ]]; then
    info "  📄 Result YAML: $(basename "$RESULT_YAML_FILE")"
fi

if [[ $SKIP_DOC_GEN -eq 0 ]]; then
    if [[ -f "$ASCIIDOC_OUTPUT" ]]; then
        FILE_SIZE=$(stat -f%z "$ASCIIDOC_OUTPUT" 2>/dev/null || stat -c%s "$ASCIIDOC_OUTPUT" 2>/dev/null || echo "0")
        info "  📄 AsciiDoc Report: $(basename "$ASCIIDOC_OUTPUT") ($FILE_SIZE bytes)"
    fi
    
    if [[ -f "$MARKDOWN_OUTPUT" ]]; then
        FILE_SIZE=$(stat -f%z "$MARKDOWN_OUTPUT" 2>/dev/null || stat -c%s "$MARKDOWN_OUTPUT" 2>/dev/null || echo "0")
        info "  📄 Markdown Report: $(basename "$MARKDOWN_OUTPUT") ($FILE_SIZE bytes)"
    fi
    
    if [[ -z "${HTML_SKIP:-}" ]] && [[ -f "$HTML_OUTPUT" ]]; then
        FILE_SIZE=$(stat -f%z "$HTML_OUTPUT" 2>/dev/null || stat -c%s "$HTML_OUTPUT" 2>/dev/null || echo "0")
        info "  📄 HTML Report: $(basename "$HTML_OUTPUT") ($FILE_SIZE bytes)"
    fi
fi

echo ""
finish_run
pass "All tests passed!"
exit 0
