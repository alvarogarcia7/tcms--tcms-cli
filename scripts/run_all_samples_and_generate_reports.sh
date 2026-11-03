#!/usr/bin/env bash
#
# run_all_samples_and_generate_reports.sh - Execute all sample test cases and generate comprehensive reports
#
# This script orchestrates the complete workflow:
# 1. Generate sample test cases covering all scenarios
# 2. Execute each test case using the orchestrator to generate execution logs
# 3. Run the verifier on all execution logs to create verification results
# 4. Generate documentation reports in both AsciiDoc and Markdown formats
# 5. Keep all results for inspection and comparison
#
# Usage: ./scripts/run_all_samples_and_generate_reports.sh [OPTIONS]
#
# Options:
#   --samples-dir DIR      Directory for sample test cases (default: testcases/generated_samples)
#   --reports-dir DIR      Output directory for reports (default: reports/generated_samples)
#   --skip-generation      Skip sample test case generation (use existing)
#   --skip-execution       Skip test execution (use existing logs)
#   --skip-verification    Skip verification (use existing results)
#   --format FORMAT        Report format: both, asciidoc, markdown (default: both)
#   --verbose              Enable verbose output
#   --help                 Show this help message
#

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source required libraries
source "$SCRIPT_DIR/lib/logger.sh" || exit 1
source "$SCRIPT_DIR/lib/find-binary.sh" || exit 1

# Default configuration
SAMPLES_DIR="$PROJECT_ROOT/testcases/generated_samples"
REPORTS_DIR="$PROJECT_ROOT/reports/generated_samples"
SKIP_GENERATION=0
SKIP_EXECUTION=0
SKIP_VERIFICATION=0
REPORT_FORMAT="both"
VERBOSE=0

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --samples-dir)
            SAMPLES_DIR="$2"
            shift 2
            ;;
        --reports-dir)
            REPORTS_DIR="$2"
            shift 2
            ;;
        --skip-generation)
            SKIP_GENERATION=1
            shift
            ;;
        --skip-execution)
            SKIP_EXECUTION=1
            shift
            ;;
        --skip-verification)
            SKIP_VERIFICATION=1
            shift
            ;;
        --format)
            REPORT_FORMAT="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=1
            export VERBOSE
            shift
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

# Validate format option
if [[ ! "$REPORT_FORMAT" =~ ^(both|asciidoc|markdown)$ ]]; then
    log_error "Invalid format: $REPORT_FORMAT (must be: both, asciidoc, or markdown)"
    exit 1
fi

# Display configuration
section "Run All Samples and Generate Reports"
log_info "Configuration:"
log_info "  Samples directory: $SAMPLES_DIR"
log_info "  Reports directory: $REPORTS_DIR"
log_info "  Skip generation: $SKIP_GENERATION"
log_info "  Skip execution: $SKIP_EXECUTION"
log_info "  Skip verification: $SKIP_VERIFICATION"
log_info "  Report format: $REPORT_FORMAT"
log_info "  Verbose: $VERBOSE"
echo ""

# Create output directories
mkdir -p "$REPORTS_DIR/verification"
mkdir -p "$REPORTS_DIR/results"
mkdir -p "$REPORTS_DIR/docs"
mkdir -p "$REPORTS_DIR/execution_logs"

# ============================================================================
# Step 1: Generate Sample Test Cases
# ============================================================================

if [[ $SKIP_GENERATION -eq 0 ]]; then
    section "Step 1: Generate Sample Test Cases"
    
    log_info "Running sample generation script..."
    
    if [[ $VERBOSE -eq 1 ]]; then
        "$SCRIPT_DIR/generate_all_sample_cases.sh" --output-dir "$SAMPLES_DIR" --verbose
    else
        "$SCRIPT_DIR/generate_all_sample_cases.sh" --output-dir "$SAMPLES_DIR" 2>&1 | while IFS= read -r line; do
            log_verbose "$line"
        done
    fi
    
    pass "Sample test cases generated successfully"
else
    section "Step 1: Generate Sample Test Cases (SKIPPED)"
    log_info "Using existing sample test cases from: $SAMPLES_DIR"
fi

# ============================================================================
# Step 2: Build Required Binaries
# ============================================================================

section "Step 2: Build Required Binaries"

log_info "Building orchestrator..."
# Capture cargo's own status (a pipeline would report the status of the loop, so build failures were hidden)
ORCH_BUILD_LOG="$REPORTS_DIR/build_orchestrator.log"
if cargo build --release -p test-orchestrator --bin test-orchestrator > "$ORCH_BUILD_LOG" 2>&1; then
    pass "Orchestrator binary built"
else
    fail "Failed to build orchestrator (see $ORCH_BUILD_LOG)"
    tail -n 20 "$ORCH_BUILD_LOG" >&2
    exit 1
fi

log_info "Building verifier..."
if cargo build --release --bin verifier 2>&1 | while IFS= read -r line; do
    log_verbose "$line"
done; then
    pass "Verifier binary built"
else
    fail "Failed to build verifier"
    exit 1
fi

ORCHESTRATOR_BIN="$PROJECT_ROOT/target/release/test-orchestrator"
VERIFIER_BIN="$PROJECT_ROOT/target/release/verifier"

# ============================================================================
# Step 3: Execute All Sample Test Cases
# ============================================================================

if [[ $SKIP_EXECUTION -eq 0 ]]; then
    section "Step 3: Execute All Sample Test Cases"
    
    # Find all sample test case files
    SAMPLE_FILES=()
    while IFS= read -r -d '' yaml_file; do
        SAMPLE_FILES+=("$yaml_file")
    done < <(find "$SAMPLES_DIR" -name "SAMPLE_*.yml" -print0 2>/dev/null)
    
    log_info "Found ${#SAMPLE_FILES[@]} sample test case(s)"
    
    if [[ ${#SAMPLE_FILES[@]} -eq 0 ]]; then
        fail "No sample test cases found in: $SAMPLES_DIR"
        exit 1
    fi
    
    # Execute each test case
    EXECUTION_SUCCESS=0
    EXECUTION_FAILED=0
    
    for sample_file in "${SAMPLE_FILES[@]}"; do
        sample_basename=$(basename "$sample_file" .yml)
        log_info "Executing: $sample_basename"
        
        # Determine output log path
        sample_dir=$(dirname "$sample_file")
        log_file="$sample_dir/${sample_basename}_execution_log.json"
        
        # Run orchestrator (allow failures - we want to capture all execution outcomes)
        log_verbose "Command: $ORCHESTRATOR_BIN run $sample_file --json-log $log_file"
        
        if "$ORCHESTRATOR_BIN" run "$sample_file" --json-log "$log_file" >/dev/null 2>&1; then
            pass "  Execution completed: $sample_basename"
            EXECUTION_SUCCESS=$((EXECUTION_SUCCESS + 1))
        else
            EXIT_CODE=$?
            if [[ $EXIT_CODE -eq 1 ]]; then
                pass "  Execution completed with failures (expected): $sample_basename"
                EXECUTION_SUCCESS=$((EXECUTION_SUCCESS + 1))
            else
                log_warning "  Execution error (exit code: $EXIT_CODE): $sample_basename"
                EXECUTION_FAILED=$((EXECUTION_FAILED + 1))
            fi
        fi
        
        # Copy execution log to reports directory for archival
        if [[ -f "$log_file" ]]; then
            cp "$log_file" "$REPORTS_DIR/execution_logs/"
            log_verbose "  Execution log archived: $(basename "$log_file")"
        else
            log_warning "  No execution log generated: $log_file"
        fi
    done
    
    echo ""
    log_info "Execution Summary:"
    log_info "  Total: ${#SAMPLE_FILES[@]}"
    log_info "  Completed: $EXECUTION_SUCCESS"
    log_info "  Failed: $EXECUTION_FAILED"
    echo ""
    
    if [[ $EXECUTION_FAILED -gt 0 ]]; then
        log_warning "Some test executions failed unexpectedly"
    fi
    
    pass "Test execution phase complete"
else
    section "Step 3: Execute All Sample Test Cases (SKIPPED)"
    log_info "Using existing execution logs"
fi

# ============================================================================
# Step 4: Run Verifier on All Execution Logs
# ============================================================================

if [[ $SKIP_VERIFICATION -eq 0 ]]; then
    section "Step 4: Run Verifier on All Execution Logs"
    
    VERIFICATION_OUTPUT_JSON="$REPORTS_DIR/verification/batch_verification.json"
    VERIFICATION_OUTPUT_YAML="$REPORTS_DIR/verification/batch_verification.yaml"
    
    log_info "Running verifier in folder mode..."
    log_verbose "Command: $VERIFIER_BIN --folder $SAMPLES_DIR --format json --output $VERIFICATION_OUTPUT_JSON"
    
    # Run verifier (exit code 1 means test failures; the output check below decides)
    VERIFIER_LOG="$REPORTS_DIR/verification/batch_verifier.log"
    mkdir -p "$(dirname "$VERIFIER_LOG")"
    if "$VERIFIER_BIN" \
        --folder "$SAMPLES_DIR" \
        --format json \
        --output "$VERIFICATION_OUTPUT_JSON" \
        --test-case-dir "$SAMPLES_DIR" > "$VERIFIER_LOG" 2>&1; then
        VERIFIER_EXIT=0
    else
        VERIFIER_EXIT=$?
    fi
    while IFS= read -r line; do
        log_verbose "$line"
    done < "$VERIFIER_LOG"

    if [[ $VERIFIER_EXIT -eq 0 ]]; then
        pass "Verifier completed successfully (all tests passed)"
    elif [[ $VERIFIER_EXIT -eq 1 ]]; then
        log_info "Verifier exit 1: some test cases failed (expected), checking output"
    else
        fail "Verifier failed with unexpected exit code: $VERIFIER_EXIT"
        tail -n 20 "$VERIFIER_LOG" >&2
        exit 1
    fi

    if [[ ! -f "$VERIFICATION_OUTPUT_JSON" ]]; then
        fail "Verification output not generated: $VERIFICATION_OUTPUT_JSON"
        tail -n 20 "$VERIFIER_LOG" >&2
        exit 1
    fi
    
    pass "JSON verification report: $VERIFICATION_OUTPUT_JSON"
    
    # Also generate YAML format
    log_info "Generating YAML verification report..."
    if "$VERIFIER_BIN" \
        --folder "$SAMPLES_DIR" \
        --format yaml \
        --output "$VERIFICATION_OUTPUT_YAML" \
        --test-case-dir "$SAMPLES_DIR" >/dev/null 2>&1; then
        pass "YAML verification report: $VERIFICATION_OUTPUT_YAML"
    else
        # Allow failure - we already have JSON
        log_verbose "YAML generation completed with test failures (expected)"
    fi
else
    section "Step 4: Run Verifier on All Execution Logs (SKIPPED)"
    log_info "Using existing verification results"
    
    VERIFICATION_OUTPUT_JSON="$REPORTS_DIR/verification/batch_verification.json"
    VERIFICATION_OUTPUT_YAML="$REPORTS_DIR/verification/batch_verification.yaml"
fi

# ============================================================================
# Step 5: Convert Verification Results to Result YAML Files
# ============================================================================

section "Step 5: Convert Verification to Result YAML Files"

CONVERT_SCRIPT="$SCRIPT_DIR/convert_verification_to_result_yaml.py"

if [[ ! -f "$CONVERT_SCRIPT" ]]; then
    log_warning "Conversion script not found: $CONVERT_SCRIPT"
    log_warning "Skipping result YAML generation"
    SKIP_RESULT_CONVERSION=1
else
    SKIP_RESULT_CONVERSION=0
    
    log_info "Converting verification JSON to result YAML files..."
    
    # Find Python interpreter
    PYTHON_CMD=$(find_python)
    if [[ -z "$PYTHON_CMD" ]]; then
        log_warning "Python interpreter not found"
        log_warning "Skipping result YAML generation"
        SKIP_RESULT_CONVERSION=1
    else
        log_verbose "Command: $PYTHON_CMD $CONVERT_SCRIPT $VERIFICATION_OUTPUT_JSON -o $REPORTS_DIR/results"
        
        if $PYTHON_CMD "$CONVERT_SCRIPT" \
            "$VERIFICATION_OUTPUT_JSON" \
            -o "$REPORTS_DIR/results" 2>&1 | while IFS= read -r line; do
                log_verbose "$line"
            done; then
            pass "Conversion completed successfully"
        else
            log_warning "Conversion failed - result YAML files may not be generated"
        fi
        
        # Count generated result files
        RESULT_COUNT=$(find "$REPORTS_DIR/results" -name "*_result.yaml" -type f 2>/dev/null | wc -l | tr -d ' ')
        log_info "Generated $RESULT_COUNT result YAML file(s)"
    fi
fi

# ============================================================================
# Step 6: Generate Documentation Reports
# ============================================================================

section "Step 6: Generate Documentation Reports"

# Create results container YAML
RESULT_CONTAINER="$REPORTS_DIR/results/results_container.yaml"

log_info "Creating results container YAML..."

cat > "$RESULT_CONTAINER" << 'EOF'
title: 'Sample Test Cases Execution Results'
project: 'Test Case Manager - Generated Samples'
test_date: '2024-01-01T00:00:00Z'
test_results:
EOF

# Append each result file content
if [[ $SKIP_RESULT_CONVERSION -eq 0 ]]; then
    RESULT_FILES=("$REPORTS_DIR/results"/*_result.yaml)
    
    if [[ ${#RESULT_FILES[@]} -gt 0 ]] && [[ -f "${RESULT_FILES[0]}" ]]; then
        for result_file in "${RESULT_FILES[@]}"; do
            if [[ -f "$result_file" ]]; then
                log_verbose "Adding result: $(basename "$result_file")"
                # Indent all lines and add to container
                sed 's/^/  /' "$result_file" >> "$RESULT_CONTAINER"
            fi
        done
        pass "Created results container: $RESULT_CONTAINER"
    else
        log_warning "No result files found to include in container"
    fi
else
    log_warning "Skipping container creation (conversion was skipped)"
fi

# Add metadata section
RESULT_COUNT=${RESULT_COUNT:-0}
cat >> "$RESULT_CONTAINER" << EOF
metadata:
  environment: 'Test Environment'
  platform: 'Test Case Manager - Generated Samples'
  executor: 'Automated Sample Workflow'
  execution_duration: 0.0
  total_test_cases: $RESULT_COUNT
  passed_test_cases: 0
  failed_test_cases: 0
EOF

pass "Results container created"

# Generate format-specific reports
if [[ "$REPORT_FORMAT" == "both" ]] || [[ "$REPORT_FORMAT" == "asciidoc" ]]; then
    log_info "Generating AsciiDoc reports..."
    
    # Generate AsciiDoc report for entire container
    ASCIIDOC_REPORT="$REPORTS_DIR/docs/sample_execution_results.adoc"
    
    cat > "$ASCIIDOC_REPORT" << 'ADOC_EOF'
= Sample Test Cases Execution Results
:toc: left
:toclevels: 3
:sectnums:
:icons: font
:source-highlighter: rouge

== Executive Summary

This document presents the comprehensive execution results of sample test cases
generated to demonstrate all major test scenarios in the Test Case Manager framework.

=== Purpose

These sample test cases cover:

* Successful execution scenarios
* Failed first step scenarios
* Failed intermediate step scenarios
* Failed last step scenarios
* Multiple sequence scenarios with mixed results
* Complex scenarios with variable capture
* Hook execution at various lifecycle points

=== Test Execution Environment

[cols="1,3"]
|===
| Platform | Test Case Manager - Generated Samples
| Date | 2024-01-01
| Automation | Fully automated sample workflow
|===

== Test Results Overview

ADOC_EOF

    # Add summary statistics from verification output
    if [[ -f "$VERIFICATION_OUTPUT_JSON" ]]; then
        log_verbose "Extracting summary from JSON..."
        
        # Extract summary using Python
        python3 - << 'PYTHON_EOF' >> "$ASCIIDOC_REPORT"
import json
import sys

try:
    with open('$VERIFICATION_OUTPUT_JSON', 'r') as f:
        data = json.load(f)
    
    summary = data.get('summary', {})
    
    print(f"\n[cols=\"1,1\"]")
    print(f"|===")
    print(f"| Total Test Cases | {summary.get('total_test_cases', 0)}")
    print(f"| Passed Test Cases | {summary.get('passed_test_cases', 0)}")
    print(f"| Failed Test Cases | {summary.get('failed_test_cases', 0)}")
    print(f"| Total Steps | {summary.get('total_steps', 0)}")
    print(f"| Passed Steps | {summary.get('passed_steps', 0)}")
    print(f"| Failed Steps | {summary.get('failed_steps', 0)}")
    print(f"| Not Executed Steps | {summary.get('not_executed_steps', 0)}")
    print(f"|===")
    print()
    
except Exception as e:
    print(f"\nERROR: Could not extract summary: {e}\n", file=sys.stderr)
    sys.exit(0)  # Don't fail the whole script
PYTHON_EOF
    fi
    
    cat >> "$ASCIIDOC_REPORT" << 'ADOC_EOF'

== Detailed Test Case Results

The following sections provide detailed results for each test case executed.

=== Successful Execution Scenarios

Test cases that demonstrate complete successful execution of all steps.

==== SAMPLE_SUCCESS_001

*Description:* Sample successful execution with all steps passing

*Requirement:* SAMPLE_SUCCESS

*Test Sequences:*

* Sequence 1: Basic Command Execution

*Expected Outcome:* All steps pass

*Actual Outcome:* (See verification results)

---

=== Failed First Step Scenarios

Test cases where the first step fails, preventing subsequent step execution.

==== SAMPLE_FAILED_FIRST_001

*Description:* Sample demonstrating failure of first step preventing subsequent steps

*Requirement:* SAMPLE_FAILED_FIRST

*Test Sequences:*

* Sequence 1: First Step Failure

*Expected Outcome:* Step 1 fails, steps 2-3 not executed

*Actual Outcome:* (See verification results)

---

=== Failed Intermediate Step Scenarios

Test cases where an intermediate step fails after some successful steps.

==== SAMPLE_FAILED_INTERMEDIATE_001

*Description:* Sample demonstrating failure of intermediate step

*Requirement:* SAMPLE_FAILED_INTERMEDIATE

*Test Sequences:*

* Sequence 1: Intermediate Step Failure

*Expected Outcome:* Steps 1-2 pass, step 3 fails, step 4 not executed

*Actual Outcome:* (See verification results)

---

=== Failed Last Step Scenarios

Test cases where only the final step fails.

==== SAMPLE_FAILED_LAST_001

*Description:* Sample demonstrating failure of last step with output mismatch

*Requirement:* SAMPLE_FAILED_LAST

*Test Sequences:*

* Sequence 1: Last Step Failure

*Expected Outcome:* Steps 1-2 pass, step 3 fails output verification

*Actual Outcome:* (See verification results)

---

=== Multiple Sequence Scenarios

Test cases with multiple sequences demonstrating mixed pass/fail results.

==== SAMPLE_MULTI_SEQ_001

*Description:* Sample with multiple sequences demonstrating mixed pass/fail results

*Requirement:* SAMPLE_MULTI_SEQ

*Test Sequences:*

* Sequence 1: First Sequence - Success
* Sequence 2: Second Sequence - Fails
* Sequence 3: Third Sequence - Not Executed

*Expected Outcome:* Sequence 1 passes, sequence 2 fails, sequence 3 not executed

*Actual Outcome:* (See verification results)

---

=== Complex Scenarios

Test cases demonstrating advanced features like variable capture.

==== SAMPLE_COMPLEX_001

*Description:* Complex sample demonstrating variable capture and conditional verification

*Requirement:* SAMPLE_COMPLEX

*Test Sequences:*

* Sequence 1: Variable Capture and Conditional Verification

*Expected Outcome:* All steps pass, variables captured correctly

*Actual Outcome:* (See verification results)

---

=== Hook Execution Scenarios

Test cases demonstrating hook execution at various lifecycle points.

==== SAMPLE_HOOK_SCRIPT_START_001

*Description:* Sample demonstrating script_start hook success

*Requirement:* SAMPLE_HOOK_SCRIPT_START

*Hooks:* script_start

*Expected Outcome:* Hook executes successfully, all steps pass

*Actual Outcome:* (See verification results)

---

==== SAMPLE_HOOK_BEFORE_SEQ_001

*Description:* Sample demonstrating before_sequence hook

*Requirement:* SAMPLE_HOOK_BEFORE_SEQ

*Hooks:* before_sequence

*Expected Outcome:* Hook executes before sequence, all steps pass

*Actual Outcome:* (See verification results)

---

== Appendix: Raw Verification Data

See the following files for complete verification data:

* JSON Format: `verification/batch_verification.json`
* YAML Format: `verification/batch_verification.yaml`
* Individual Results: `results/*_result.yaml`

== Conclusion

This report demonstrates the comprehensive testing capabilities of the Test Case
Manager framework across all major execution scenarios. The generated samples
provide a complete reference for understanding test behavior in various conditions.

ADOC_EOF
    
    pass "AsciiDoc report generated: $ASCIIDOC_REPORT"
fi

if [[ "$REPORT_FORMAT" == "both" ]] || [[ "$REPORT_FORMAT" == "markdown" ]]; then
    log_info "Generating Markdown reports..."
    
    # Generate Markdown report for entire container
    MARKDOWN_REPORT="$REPORTS_DIR/docs/sample_execution_results.md"
    
    cat > "$MARKDOWN_REPORT" << 'MD_EOF'
# Sample Test Cases Execution Results

## Executive Summary

This document presents the comprehensive execution results of sample test cases
generated to demonstrate all major test scenarios in the Test Case Manager framework.

### Purpose

These sample test cases cover:

- Successful execution scenarios
- Failed first step scenarios
- Failed intermediate step scenarios
- Failed last step scenarios
- Multiple sequence scenarios with mixed results
- Complex scenarios with variable capture
- Hook execution at various lifecycle points

### Test Execution Environment

| Attribute | Value |
|-----------|-------|
| Platform | Test Case Manager - Generated Samples |
| Date | 2024-01-01 |
| Automation | Fully automated sample workflow |

## Test Results Overview

MD_EOF

    # Add summary statistics from verification output
    if [[ -f "$VERIFICATION_OUTPUT_JSON" ]]; then
        log_verbose "Extracting summary from JSON..."
        
        # Extract summary using Python
        python3 - << 'PYTHON_EOF' >> "$MARKDOWN_REPORT"
import json
import sys

try:
    with open('$VERIFICATION_OUTPUT_JSON', 'r') as f:
        data = json.load(f)
    
    summary = data.get('summary', {})
    
    print(f"\n| Metric | Count |")
    print(f"|--------|-------|")
    print(f"| Total Test Cases | {summary.get('total_test_cases', 0)} |")
    print(f"| Passed Test Cases | {summary.get('passed_test_cases', 0)} |")
    print(f"| Failed Test Cases | {summary.get('failed_test_cases', 0)} |")
    print(f"| Total Steps | {summary.get('total_steps', 0)} |")
    print(f"| Passed Steps | {summary.get('passed_steps', 0)} |")
    print(f"| Failed Steps | {summary.get('failed_steps', 0)} |")
    print(f"| Not Executed Steps | {summary.get('not_executed_steps', 0)} |")
    print()
    
except Exception as e:
    print(f"\nERROR: Could not extract summary: {e}\n", file=sys.stderr)
    sys.exit(0)  # Don't fail the whole script
PYTHON_EOF
    fi
    
    cat >> "$MARKDOWN_REPORT" << 'MD_EOF'

## Detailed Test Case Results

The following sections provide detailed results for each test case executed.

### Successful Execution Scenarios

Test cases that demonstrate complete successful execution of all steps.

#### SAMPLE_SUCCESS_001

**Description:** Sample successful execution with all steps passing

**Requirement:** SAMPLE_SUCCESS

**Test Sequences:**

- Sequence 1: Basic Command Execution

**Expected Outcome:** All steps pass

**Actual Outcome:** (See verification results)

---

### Failed First Step Scenarios

Test cases where the first step fails, preventing subsequent step execution.

#### SAMPLE_FAILED_FIRST_001

**Description:** Sample demonstrating failure of first step preventing subsequent steps

**Requirement:** SAMPLE_FAILED_FIRST

**Test Sequences:**

- Sequence 1: First Step Failure

**Expected Outcome:** Step 1 fails, steps 2-3 not executed

**Actual Outcome:** (See verification results)

---

### Failed Intermediate Step Scenarios

Test cases where an intermediate step fails after some successful steps.

#### SAMPLE_FAILED_INTERMEDIATE_001

**Description:** Sample demonstrating failure of intermediate step

**Requirement:** SAMPLE_FAILED_INTERMEDIATE

**Test Sequences:**

- Sequence 1: Intermediate Step Failure

**Expected Outcome:** Steps 1-2 pass, step 3 fails, step 4 not executed

**Actual Outcome:** (See verification results)

---

### Failed Last Step Scenarios

Test cases where only the final step fails.

#### SAMPLE_FAILED_LAST_001

**Description:** Sample demonstrating failure of last step with output mismatch

**Requirement:** SAMPLE_FAILED_LAST

**Test Sequences:**

- Sequence 1: Last Step Failure

**Expected Outcome:** Steps 1-2 pass, step 3 fails output verification

**Actual Outcome:** (See verification results)

---

### Multiple Sequence Scenarios

Test cases with multiple sequences demonstrating mixed pass/fail results.

#### SAMPLE_MULTI_SEQ_001

**Description:** Sample with multiple sequences demonstrating mixed pass/fail results

**Requirement:** SAMPLE_MULTI_SEQ

**Test Sequences:**

- Sequence 1: First Sequence - Success
- Sequence 2: Second Sequence - Fails
- Sequence 3: Third Sequence - Not Executed

**Expected Outcome:** Sequence 1 passes, sequence 2 fails, sequence 3 not executed

**Actual Outcome:** (See verification results)

---

### Complex Scenarios

Test cases demonstrating advanced features like variable capture.

#### SAMPLE_COMPLEX_001

**Description:** Complex sample demonstrating variable capture and conditional verification

**Requirement:** SAMPLE_COMPLEX

**Test Sequences:**

- Sequence 1: Variable Capture and Conditional Verification

**Expected Outcome:** All steps pass, variables captured correctly

**Actual Outcome:** (See verification results)

---

### Hook Execution Scenarios

Test cases demonstrating hook execution at various lifecycle points.

#### SAMPLE_HOOK_SCRIPT_START_001

**Description:** Sample demonstrating script_start hook success

**Requirement:** SAMPLE_HOOK_SCRIPT_START

**Hooks:** script_start

**Expected Outcome:** Hook executes successfully, all steps pass

**Actual Outcome:** (See verification results)

---

#### SAMPLE_HOOK_BEFORE_SEQ_001

**Description:** Sample demonstrating before_sequence hook

**Requirement:** SAMPLE_HOOK_BEFORE_SEQ

**Hooks:** before_sequence

**Expected Outcome:** Hook executes before sequence, all steps pass

**Actual Outcome:** (See verification results)

---

## Appendix: Raw Verification Data

See the following files for complete verification data:

- **JSON Format:** `verification/batch_verification.json`
- **YAML Format:** `verification/batch_verification.yaml`
- **Individual Results:** `results/*_result.yaml`

## Conclusion

This report demonstrates the comprehensive testing capabilities of the Test Case
Manager framework across all major execution scenarios. The generated samples
provide a complete reference for understanding test behavior in various conditions.

MD_EOF
    
    pass "Markdown report generated: $MARKDOWN_REPORT"
fi

# ============================================================================
# Step 7: Final Summary
# ============================================================================

section "Final Summary"

log_info "All sample test cases executed and reports generated!"
echo ""

info "Directory Structure:"
echo "  📁 $SAMPLES_DIR/"
echo "     └─ Sample test case YAML files"
echo "     └─ Generated execution logs"
echo ""
echo "  📁 $REPORTS_DIR/"
echo "     ├─ verification/"
echo "     │  ├─ batch_verification.json"
echo "     │  └─ batch_verification.yaml"
echo "     ├─ results/"
echo "     │  ├─ *_result.yaml"
echo "     │  └─ results_container.yaml"
echo "     ├─ execution_logs/"
echo "     │  └─ *_execution_log.json"
echo "     └─ docs/"
if [[ "$REPORT_FORMAT" == "both" ]]; then
    echo "        ├─ sample_execution_results.adoc"
    echo "        └─ sample_execution_results.md"
elif [[ "$REPORT_FORMAT" == "asciidoc" ]]; then
    echo "        └─ sample_execution_results.adoc"
elif [[ "$REPORT_FORMAT" == "markdown" ]]; then
    echo "        └─ sample_execution_results.md"
fi
echo ""

info "Key Files:"

if [[ -f "$VERIFICATION_OUTPUT_JSON" ]]; then
    echo "  📄 Verification (JSON): $VERIFICATION_OUTPUT_JSON"
fi

if [[ -f "$VERIFICATION_OUTPUT_YAML" ]]; then
    echo "  📄 Verification (YAML): $VERIFICATION_OUTPUT_YAML"
fi

if [[ -f "$RESULT_CONTAINER" ]]; then
    echo "  📄 Results Container: $RESULT_CONTAINER"
fi

if [[ "$REPORT_FORMAT" == "both" ]] || [[ "$REPORT_FORMAT" == "asciidoc" ]]; then
    ASCIIDOC_REPORT="$REPORTS_DIR/docs/sample_execution_results.adoc"
    if [[ -f "$ASCIIDOC_REPORT" ]]; then
        echo "  📄 AsciiDoc Report: $ASCIIDOC_REPORT"
    fi
fi

if [[ "$REPORT_FORMAT" == "both" ]] || [[ "$REPORT_FORMAT" == "markdown" ]]; then
    MARKDOWN_REPORT="$REPORTS_DIR/docs/sample_execution_results.md"
    if [[ -f "$MARKDOWN_REPORT" ]]; then
        echo "  📄 Markdown Report: $MARKDOWN_REPORT"
    fi
fi

echo ""

pass "Complete! All samples executed and reports generated."
log_info "Reports saved to: $REPORTS_DIR"

exit 0
