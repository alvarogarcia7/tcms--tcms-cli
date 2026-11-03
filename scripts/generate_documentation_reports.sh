#!/usr/bin/env bash
#
# generate_documentation_reports.sh - Orchestrate end-to-end report generation
#
# This script orchestrates the full report generation pipeline:
# 1. Run verifier on execution logs using folder mode
# 2. Convert verification JSON to result YAML files (uses --multiple mode by default)
# 3. Build test-plan-doc-gen if needed
# 4. Generate test results reports (AsciiDoc and Markdown) from result container YAML
# 5. Generate test plan reports (AsciiDoc and Markdown) from test case YAML files
# 6. Print paths to all generated reports
#
# Step 2 Output Modes:
#   --multiple (default): Generates individual YAML files, one per test case
#                         (e.g., TC_001_result.yaml, TC_002_result.yaml)
#                         Each file is self-contained and can be processed independently.
#   --single: Generates a single YAML file containing all test cases as an array
#             (e.g., results.yaml with all results in one file)
#             Useful when downstream tools expect a single aggregated file.
#
# This script uses --multiple mode to enable independent processing of each
# test case result file and support parallel report generation workflows.
#
# Usage: ./scripts/generate_documentation_reports.sh [OPTIONS]
#
# Options:
#   --logs-dir DIR           Directory containing execution logs (default: testcases/verifier_scenarios)
#   --test-case-dir DIR      Directory containing test case YAML files (default: testcases/verifier_scenarios)
#   --output-dir DIR         Output directory for reports (default: reports/documentation)
#   --test-plan-doc-gen DIR  Path to test-plan-doc-gen sibling directory (default: ../test-plan-doc-gen)
#   --container-template     Path to container template YAML (default: testcases/expected_output_reports/container_data.yml)
#   --config FILE            Path to container config file (default: container_config.yml)
#   --title TITLE            Override report title
#   --project PROJECT        Override project name
#   --environment ENV        Override environment information
#   --platform PLATFORM      Override platform information
#   --executor EXECUTOR      Override executor information
#   --help                   Show this help message
#

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source required libraries
source "$SCRIPT_DIR/lib/logger.sh" || exit 1
source "$SCRIPT_DIR/lib/report_generator.sh" || exit 1
source "$SCRIPT_DIR/lib/find-binary.sh" || exit 1

# Default configuration
LOGS_DIR="$PROJECT_ROOT/testcases/verifier_scenarios"
TEST_CASE_DIR="$PROJECT_ROOT/testcases/verifier_scenarios"
OUTPUT_DIR="$PROJECT_ROOT/reports/documentation"
TEST_PLAN_DOC_GEN_DIR="../test-plan-doc-gen"
CONTAINER_TEMPLATE="$PROJECT_ROOT/testcases/expected_output_reports/container_data.yml"
CONFIG_FILE="$PROJECT_ROOT/container_config.yml"

# CLI overrides
TITLE=""
PROJECT=""
ENVIRONMENT=""
PLATFORM=""
EXECUTOR=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --logs-dir)
            LOGS_DIR="$2"
            shift 2
            ;;
        --test-case-dir)
            TEST_CASE_DIR="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --test-plan-doc-gen)
            TEST_PLAN_DOC_GEN_DIR="$2"
            shift 2
            ;;
        --container-template)
            CONTAINER_TEMPLATE="$2"
            shift 2
            ;;
        --config)
            CONFIG_FILE="$2"
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
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --executor)
            EXECUTOR="$2"
            shift 2
            ;;
        --help)
            head -n 30 "$0" | tail -n +2 | sed 's/^# //'
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Display configuration
section "Documentation Report Generation"
log_info "Configuration:"
log_info "  Logs directory: $LOGS_DIR"
log_info "  Test case directory: $TEST_CASE_DIR"
log_info "  Output directory: $OUTPUT_DIR"
log_info "  test-plan-doc-gen: $TEST_PLAN_DOC_GEN_DIR"
log_info "  Container template: $CONTAINER_TEMPLATE"
if [ -f "$CONFIG_FILE" ]; then
    log_info "  Config file: $CONFIG_FILE"
else
    log_info "  Config file: $CONFIG_FILE (not found, using defaults)"
fi
echo ""

# Create output directories
mkdir -p "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/verification"
mkdir -p "$OUTPUT_DIR/results"
mkdir -p "$OUTPUT_DIR/reports"

log_verbose "Created output directory structure"

# Array to track generated report paths
declare -a GENERATED_REPORTS

# ============================================================================
# Step 1: Run verifier on execution logs using folder mode
# ============================================================================

section "Step 1: Run Verifier (Folder Mode)"

# Build verifier binary if needed
if [[ ! -f "$PROJECT_ROOT/target/release/verifier" ]]; then
    log_info "Building verifier binary..."
    cd "$PROJECT_ROOT"
    if cargo build --release --bin verifier 2>&1 | while IFS= read -r line; do
        log_verbose "$line"
    done; then
        pass "Verifier binary built successfully"
    else
        fail "Failed to build verifier binary"
        exit 1
    fi
else
    pass "Verifier binary already exists"
fi

VERIFIER_BIN="$PROJECT_ROOT/target/release/verifier"
VERIFICATION_OUTPUT="$OUTPUT_DIR/verification/batch_verification.json"

# Check if logs directory exists
if [[ ! -d "$LOGS_DIR" ]]; then
    fail "Logs directory not found: $LOGS_DIR"
    exit 1
fi

log_info "Running verifier in folder mode..."

# Build verifier command with config file and CLI overrides
VERIFIER_CMD="\"$VERIFIER_BIN\" --folder \"$LOGS_DIR\" --format json --output \"$VERIFICATION_OUTPUT\" --test-case-dir \"$TEST_CASE_DIR\""

# Add config file if it exists
if [[ -f "$CONFIG_FILE" ]]; then
    VERIFIER_CMD="$VERIFIER_CMD --config \"$CONFIG_FILE\""
fi

# Add CLI overrides
if [[ -n "$TITLE" ]]; then
    VERIFIER_CMD="$VERIFIER_CMD --title \"$TITLE\""
fi

if [[ -n "$PROJECT" ]]; then
    VERIFIER_CMD="$VERIFIER_CMD --project \"$PROJECT\""
fi

if [[ -n "$ENVIRONMENT" ]]; then
    VERIFIER_CMD="$VERIFIER_CMD --environment \"$ENVIRONMENT\""
fi

if [[ -n "$PLATFORM" ]]; then
    VERIFIER_CMD="$VERIFIER_CMD --platform \"$PLATFORM\""
fi

if [[ -n "$EXECUTOR" ]]; then
    VERIFIER_CMD="$VERIFIER_CMD --executor \"$EXECUTOR\""
fi

log_verbose "Command: $VERIFIER_CMD"

VERIFIER_LOG="$OUTPUT_DIR/verification/batch_verifier.log"
mkdir -p "$(dirname "$VERIFIER_LOG")"
# Capture the verifier's own exit status (a pipeline would report the status of the loop instead).
if eval "$VERIFIER_CMD" > "$VERIFIER_LOG" 2>&1; then
    VERIFIER_EXIT=0
else
    VERIFIER_EXIT=$?
fi
while IFS= read -r line; do
    log_verbose "$line"
done < "$VERIFIER_LOG"

if [[ $VERIFIER_EXIT -eq 0 ]]; then
    pass "Verifier completed successfully"
elif [[ $VERIFIER_EXIT -eq 1 ]]; then
    # Exit code 1 means some test cases failed, which is expected; the output check below decides
    log_info "Verifier exit 1: some test cases failed (expected), checking output"
else
    fail "Verifier failed with exit code: $VERIFIER_EXIT"
    tail -n 20 "$VERIFIER_LOG" >&2
    exit 1
fi

if [[ ! -f "$VERIFICATION_OUTPUT" ]]; then
    fail "Verification output not generated: $VERIFICATION_OUTPUT"
    tail -n 20 "$VERIFIER_LOG" >&2
    exit 1
fi

pass "Verification report: $VERIFICATION_OUTPUT"
log_verbose "File size: $(stat -f%z "$VERIFICATION_OUTPUT" 2>/dev/null || stat -c%s "$VERIFICATION_OUTPUT" 2>/dev/null || echo "unknown") bytes"

# ============================================================================
# Step 2: Convert verification JSON to result YAML files
# ============================================================================

section "Step 2: Convert Verification JSON to Result YAML"

CONVERT_SCRIPT="$SCRIPT_DIR/convert_verification_to_result_yaml.py"

if [[ ! -f "$CONVERT_SCRIPT" ]]; then
    fail "Conversion script not found: $CONVERT_SCRIPT"
    exit 1
fi

log_info "Converting verification JSON to result YAML files..."

# Find Python interpreter
PYTHON_CMD=$(find_python)
if [[ -z "$PYTHON_CMD" ]]; then
    fail "Python interpreter not found"
    exit 1
fi

log_verbose "Command: $PYTHON_CMD $CONVERT_SCRIPT $VERIFICATION_OUTPUT -o $OUTPUT_DIR/results --multiple"

# Use --multiple mode to generate individual YAML files (one per test case).
# This enables independent processing and supports parallel report generation.
# Alternative: Use --single to generate a single aggregated YAML array file.
if $PYTHON_CMD "$CONVERT_SCRIPT" \
    "$VERIFICATION_OUTPUT" \
    -o "$OUTPUT_DIR/results" \
    --multiple 2>&1 | while IFS= read -r line; do
        log_verbose "$line"
    done; then
    pass "Conversion completed successfully"
else
    fail "Conversion failed"
    exit 1
fi

# Count generated result files
RESULT_COUNT=$(find "$OUTPUT_DIR/results" -name "*_result.yaml" -type f 2>/dev/null | wc -l | tr -d ' ')
log_info "Generated $RESULT_COUNT result YAML file(s)"

# ============================================================================
# Step 3: Build test-plan-doc-gen if needed
# ============================================================================

section "Step 3: Build test-plan-doc-gen"

# Resolve test-plan-doc-gen directory path
if [[ ! "$TEST_PLAN_DOC_GEN_DIR" = /* ]]; then
    TEST_PLAN_DOC_GEN_DIR="$PROJECT_ROOT/$TEST_PLAN_DOC_GEN_DIR"
fi

# Check if test-plan-doc-gen directory exists
if [[ ! -d "$TEST_PLAN_DOC_GEN_DIR" ]]; then
    log_warning "test-plan-doc-gen directory not found: $TEST_PLAN_DOC_GEN_DIR"
    log_warning "Skipping test-plan-doc-gen report generation"
    log_info "To enable test-plan-doc-gen reports, clone the repository:"
    log_info "  cd $(dirname "$PROJECT_ROOT")"
    log_info "  git clone <test-plan-doc-gen-repo-url> test-plan-doc-gen"
    SKIP_DOC_GEN=1
else
    SKIP_DOC_GEN=0
    
    # Check if binary exists
    if check_test_plan_doc_gen_available "$TEST_PLAN_DOC_GEN_DIR"; then
        pass "test-plan-doc-gen binary found"
    else
        log_info "Building test-plan-doc-gen..."
        if build_test_plan_doc_gen "$TEST_PLAN_DOC_GEN_DIR"; then
            pass "test-plan-doc-gen built successfully"
        else
            fail "Failed to build test-plan-doc-gen"
            SKIP_DOC_GEN=1
        fi
    fi
fi

# ============================================================================
# Step 4: Generate test results reports from container YAML
# ============================================================================

section "Step 4: Generate Test Results Reports from Container"

if [[ $SKIP_DOC_GEN -eq 1 ]]; then
    log_warning "Skipping test results report generation (test-plan-doc-gen not available)"
else
    # Create a container YAML with all result files
    RESULT_CONTAINER="$OUTPUT_DIR/results/results_container.yaml"
    
    log_info "Creating results container YAML..."
    
    # Start with container template header or create a basic one
    if [[ -f "$CONTAINER_TEMPLATE" ]]; then
        # Extract header metadata from template (title, project, test_date, etc.)
        log_verbose "Using container template: $CONTAINER_TEMPLATE"
        
        # Create container with metadata
        cat > "$RESULT_CONTAINER" << 'EOF'
title: 'Test Execution Results Report'
project: 'Test Case Manager - Verification Results'
test_date: '2024-01-01T00:00:00Z'
test_results:
EOF
    else
        log_verbose "No container template found, using basic structure"
        cat > "$RESULT_CONTAINER" << 'EOF'
title: 'Test Execution Results Report'
project: 'Test Case Manager - Verification Results'
test_date: '2024-01-01T00:00:00Z'
test_results:
EOF
    fi
    
    # Append each result file content (without the 'type: result' line)
    RESULT_FILES=("$OUTPUT_DIR/results"/*_result.yaml)
    
    if [[ ${#RESULT_FILES[@]} -eq 0 ]]; then
        log_warning "No result files found to include in container"
    else
        for result_file in "${RESULT_FILES[@]}"; do
            if [[ -f "$result_file" ]]; then
                log_verbose "Adding result: $(basename "$result_file")"
                # Indent all lines except 'type: result' and add to container
                sed '/^type: result/d' "$result_file" | sed 's/^/  /' >> "$RESULT_CONTAINER"
            fi
        done
        pass "Created results container: $RESULT_CONTAINER"
    fi
    
    # Add metadata section
    cat >> "$RESULT_CONTAINER" << EOF
metadata:
  environment: 'Test Environment'
  platform: 'Test Case Manager'
  executor: 'Automated Test Framework'
  execution_duration: 0.0
  total_test_cases: $RESULT_COUNT
  passed_test_cases: 0
  failed_test_cases: 0
EOF
    
    # Set TEST_PLAN_DOC_GEN environment variable for report_generator.sh
    export TEST_PLAN_DOC_GEN=$(find_test_plan_doc_gen "$TEST_PLAN_DOC_GEN_DIR")
    
    # Generate AsciiDoc report from container
    ASCIIDOC_OUTPUT="$OUTPUT_DIR/reports/test_results_report.adoc"
    
    log_info "Generating AsciiDoc test results report from container..."
    log_verbose "Command: invoke_test_plan_doc_gen --container $RESULT_CONTAINER --output $ASCIIDOC_OUTPUT --format asciidoc"
    
    if invoke_test_plan_doc_gen \
        --container "$RESULT_CONTAINER" \
        --output "$ASCIIDOC_OUTPUT" \
        --format asciidoc 2>&1 | while IFS= read -r line; do
            log_verbose "$line"
        done; then
        pass "AsciiDoc report generated: $ASCIIDOC_OUTPUT"
        GENERATED_REPORTS+=("$ASCIIDOC_OUTPUT")
    else
        fail "Failed to generate AsciiDoc report from container"
    fi
    
    # Generate Markdown report from container
    MARKDOWN_OUTPUT="$OUTPUT_DIR/reports/test_results_report.md"
    
    log_info "Generating Markdown test results report from container..."
    log_verbose "Command: invoke_test_plan_doc_gen --container $RESULT_CONTAINER --output $MARKDOWN_OUTPUT --format markdown"
    
    if invoke_test_plan_doc_gen \
        --container "$RESULT_CONTAINER" \
        --output "$MARKDOWN_OUTPUT" \
        --format markdown 2>&1 | while IFS= read -r line; do
            log_verbose "$line"
        done; then
        pass "Markdown report generated: $MARKDOWN_OUTPUT"
        GENERATED_REPORTS+=("$MARKDOWN_OUTPUT")
    else
        fail "Failed to generate Markdown report from container"
    fi
fi

# ============================================================================
# Step 5: Generate test plan reports from test case YAML files
# ============================================================================

section "Step 5: Generate Test Plan Reports from Test Cases"

if [[ $SKIP_DOC_GEN -eq 1 ]]; then
    log_warning "Skipping test plan report generation (test-plan-doc-gen not available)"
else
    # Find all test case YAML files (excluding container files)
    TEST_CASE_FILES=()
    
    log_info "Discovering test case YAML files in: $TEST_CASE_DIR"
    
    while IFS= read -r -d '' yaml_file; do
        # Skip files in expected_output_reports and other report directories
        # Also skip result files, container files, and execution logs
        if [[ ! "$yaml_file" =~ expected_output_reports ]] && \
           [[ ! "$yaml_file" =~ /reports/ ]] && \
           [[ ! "$yaml_file" =~ _result\.ya?ml$ ]] && \
           [[ ! "$yaml_file" =~ _container\.ya?ml$ ]] && \
           [[ ! "$yaml_file" =~ _execution_log\. ]]; then
            TEST_CASE_FILES+=("$yaml_file")
            log_verbose "Found test case: $(basename "$yaml_file")"
        fi
    done < <(find "$TEST_CASE_DIR" -type f \( -name "*.yml" -o -name "*.yaml" \) -print0 2>/dev/null)
    
    log_info "Found ${#TEST_CASE_FILES[@]} test case file(s)"
    
    if [[ ${#TEST_CASE_FILES[@]} -eq 0 ]]; then
        log_warning "No test case files found"
    else
        # Generate both AsciiDoc and Markdown reports for each test case
        for test_case_file in "${TEST_CASE_FILES[@]}"; do
            test_case_basename=$(basename "$test_case_file" .yml)
            test_case_basename=$(basename "$test_case_basename" .yaml)
            
            log_info "Generating reports for test case: $test_case_basename"
            
            # Generate AsciiDoc report
            ASCIIDOC_OUTPUT="$OUTPUT_DIR/reports/${test_case_basename}_test_plan.adoc"
            
            log_verbose "Command: invoke_test_plan_doc_gen --test-case $test_case_file --output $ASCIIDOC_OUTPUT --format asciidoc"
            
            if invoke_test_plan_doc_gen \
                --test-case "$test_case_file" \
                --output "$ASCIIDOC_OUTPUT" \
                --format asciidoc 2>&1 | while IFS= read -r line; do
                    log_verbose "$line"
                done; then
                pass "AsciiDoc test plan: $ASCIIDOC_OUTPUT"
                GENERATED_REPORTS+=("$ASCIIDOC_OUTPUT")
            else
                log_warning "Failed to generate AsciiDoc test plan for: $test_case_basename"
            fi

            ls -lah "$ASCIIDOC_OUTPUT"
            
            # Generate Markdown report
            MARKDOWN_OUTPUT="$OUTPUT_DIR/reports/${test_case_basename}_test_plan.md"
            
            log_verbose "Command: invoke_test_plan_doc_gen --test-case $test_case_file --output $MARKDOWN_OUTPUT --format markdown"
            
            if invoke_test_plan_doc_gen \
                --test-case "$test_case_file" \
                --output "$MARKDOWN_OUTPUT" \
                --format markdown 2>&1 | while IFS= read -r line; do
                    log_verbose "$line"
                done; then
                pass "Markdown test plan: $MARKDOWN_OUTPUT"
                GENERATED_REPORTS+=("$MARKDOWN_OUTPUT")
            else
                log_warning "Failed to generate Markdown test plan for: $test_case_basename"
            fi
        done
    fi
fi

# ============================================================================
# Step 6: Print paths to all generated reports
# ============================================================================

section "Report Generation Summary"

log_info "Generated Reports:"
echo ""

# Verification reports
if [[ -f "$VERIFICATION_OUTPUT" ]]; then
    info "Verification JSON:"
    echo "  📄 $VERIFICATION_OUTPUT"
    echo ""
fi

# Result YAML files
if [[ -d "$OUTPUT_DIR/results" ]]; then
    RESULT_FILES=("$OUTPUT_DIR/results"/*_result.yaml)
    if [[ ${#RESULT_FILES[@]} -gt 0 ]] && [[ -f "${RESULT_FILES[0]}" ]]; then
        info "Result YAML files:"
        for result_file in "${RESULT_FILES[@]}"; do
            if [[ -f "$result_file" ]]; then
                echo "  📄 $result_file"
            fi
        done
        echo ""
    fi
fi

# Results container
if [[ -f "$RESULT_CONTAINER" ]]; then
    info "Results Container:"
    echo "  📄 $RESULT_CONTAINER"
    echo ""
fi

# Generated reports (AsciiDoc/Markdown)
if [[ ${#GENERATED_REPORTS[@]} -gt 0 ]]; then
    info "Documentation Reports:"
    for report in "${GENERATED_REPORTS[@]}"; do
        if [[ -f "$report" ]]; then
            echo "  📄 $report"
        fi
    done
    echo ""
fi

# Summary statistics
section "Statistics"
log_info "Total verification reports: 1"
log_info "Total result YAML files: $RESULT_COUNT"
log_info "Total documentation reports: ${#GENERATED_REPORTS[@]}"
echo ""

pass "Report generation complete!"
log_info "All reports saved to: $OUTPUT_DIR"

exit 0
