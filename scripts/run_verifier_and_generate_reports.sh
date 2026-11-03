#!/usr/bin/env bash
#
# Run verifier on all test scenarios and generate PDF reports
#
# Usage: ./scripts/run_verifier_and_generate_reports.sh [OPTIONS]
#
# Options:
#   --config FILE       Path to container config file (default: container_config.yml)
#   --title TITLE       Override report title
#   --project PROJECT   Override project name
#   --environment ENV   Override environment information
#   --platform PLATFORM Override platform information
#   --executor EXECUTOR Override executor information
#

set -e

BUILD_VARIANT="${BUILD_VARIANT:---release}"

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default config file
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
            head -n 15 "$0" | tail -n +2 | sed 's/^# //'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "======================================================================="
echo "Test Verifier Report Generator"
echo "======================================================================="
echo ""
echo "Project root: $PROJECT_ROOT"

# Create output directory
OUTPUT_DIR="$PROJECT_ROOT/reports/verifier_scenarios"
mkdir -p "$OUTPUT_DIR"
# Count of steps that failed for real (verifier error, missing output, doc generation failure).
# Any non-zero count makes this script exit 1 (TCMS-46).
FAILURES=0
echo "Output directory: $OUTPUT_DIR"

# Build verifier binary
echo ""
echo "=== Building verifier binary ==="
cd "$PROJECT_ROOT"
cargo build -p verifier ${BUILD_VARIANT} --bin verifier

if [ $? -ne 0 ]; then
    echo "✗ Failed to build verifier binary"
    exit 1
fi

echo "✓ Verifier binary built successfully"

# Define test scenarios
declare -a SCENARIOS=(
    "successful:TEST_SUCCESS_001"
    "failed_first:TEST_FAILED_FIRST_001"
    "failed_intermediate:TEST_FAILED_INTERMEDIATE_001"
    "failed_last:TEST_FAILED_LAST_001"
    "interrupted:TEST_INTERRUPTED_001"
    "multiple_sequences:TEST_MULTI_SEQ_001"
    "hooks:TEST_HOOK_SCRIPT_START_001"
)

declare -a VERIFICATION_FILES=()

# Build verifier command with config file and CLI overrides
build_verifier_cmd() {
    local log_file="$1"
    local test_case_id="$2"
    local output_file="$3"

    local cmd="cargo run -p verifier ${BUILD_VARIANT} --bin verifier --"
    cmd="$cmd --log \"$log_file\""
    cmd="$cmd --test-case \"$test_case_id\""
    cmd="$cmd --format json"
    cmd="$cmd --output \"$output_file\""
    
    # Add config file if it exists
    if [ -f "$CONFIG_FILE" ]; then
        cmd="$cmd --config \"$CONFIG_FILE\""
    fi
    
    # Add CLI overrides
    if [ -n "$TITLE" ]; then
        cmd="$cmd --title \"$TITLE\""
    fi
    
    if [ -n "$PROJECT" ]; then
        cmd="$cmd --project \"$PROJECT\""
    fi
    
    if [ -n "$ENVIRONMENT" ]; then
        cmd="$cmd --environment \"$ENVIRONMENT\""
    fi
    
    if [ -n "$PLATFORM" ]; then
        cmd="$cmd --platform \"$PLATFORM\""
    fi
    
    if [ -n "$EXECUTOR" ]; then
        cmd="$cmd --executor \"$EXECUTOR\""
    fi
    
    echo "$cmd"
}

# Process each scenario
for SCENARIO_ENTRY in "${SCENARIOS[@]}"; do
    IFS=':' read -r SCENARIO_DIR TEST_CASE_ID <<< "$SCENARIO_ENTRY"
    
    echo ""
    echo "======================================================================="
    echo "Processing: $TEST_CASE_ID"
    echo "======================================================================="
    
    EXECUTION_LOG="$PROJECT_ROOT/testcases/verifier_scenarios/$SCENARIO_DIR/${TEST_CASE_ID}_execution_log.json"
    VERIFICATION_OUTPUT="$OUTPUT_DIR/${TEST_CASE_ID}_verification.json"
    
    # Check if execution log exists
    if [ ! -f "$EXECUTION_LOG" ]; then
        echo "⚠ Execution log not found: $EXECUTION_LOG"
        echo "  Skipping $TEST_CASE_ID"
        continue
    fi
    
    echo "✓ Execution log found: $EXECUTION_LOG"
    
    # Run verifier
    echo ""
    echo "Running verifier..."
    
    # Build and execute command
    VERIFIER_CMD=$(build_verifier_cmd "$EXECUTION_LOG" "$TEST_CASE_ID" "$VERIFICATION_OUTPUT")
    VERIFIER_LOG="$OUTPUT_DIR/${TEST_CASE_ID}_verifier.log"
    # 'set -e' is active: capture the verifier status without aborting the loop
    VERIFIER_EXIT=0
    eval "$VERIFIER_CMD" > "$VERIFIER_LOG" 2>&1 || VERIFIER_EXIT=$?
    tail -20 "$VERIFIER_LOG"
    
    # Exit codes: 0 = all tests passed, 1 = some tests failed (expected for failure scenarios)
    if [ $VERIFIER_EXIT -ne 0 ] && [ $VERIFIER_EXIT -ne 1 ]; then
        echo "✗ Verifier failed with unexpected exit code: $VERIFIER_EXIT"
        FAILURES=$((FAILURES + 1))
        continue
    fi
    
    if [ -f "$VERIFICATION_OUTPUT" ]; then
        echo "✓ Verification report: $VERIFICATION_OUTPUT"
        VERIFICATION_FILES+=("$VERIFICATION_OUTPUT")
    else
        echo "✗ Verification report not generated"
        FAILURES=$((FAILURES + 1))
    fi
done

# Summary of verification reports
echo ""
echo "======================================================================="
echo "Verification Reports Generated"
echo "======================================================================="
echo ""
echo "Generated ${#VERIFICATION_FILES[@]} verification reports:"
for VF in "${VERIFICATION_FILES[@]}"; do
    echo "  • $VF"
done

# Check for reportlab and generate PDF reports
echo ""
echo "======================================================================="
echo "PDF Report Generation"
echo "======================================================================="
echo ""

uv sync

source .venv/bin/activate

if command -v python3 >/dev/null 2>&1; then
    echo "Python 3 found. Checking for reportlab..."
    
    if uv run python3 -c "import reportlab" 2>/dev/null; then
        echo "✓ reportlab is installed"
        echo ""
        echo "Generating PDF reports..."
        
        cd "$PROJECT_ROOT"
        uv run python3 scripts/generate_verifier_reports.py
        
        if [ $? -eq 0 ]; then
            echo ""
            echo "✓ PDF reports generated successfully"
            echo ""
            echo "Generated PDF reports:"
            for VF in "${VERIFICATION_FILES[@]}"; do
                PDF_REPORT="${VF%.json}_report.pdf"
                PDF_REPORT="${PDF_REPORT/_verification_report.pdf/_report.pdf}"
                if [ -f "$PDF_REPORT" ]; then
                    echo "  • $PDF_REPORT"
                fi
            done
        else
            echo "✗ PDF report generation failed"
        fi
    else
        echo "⚠ reportlab not installed"
        echo ""
        echo "To generate PDF reports, install reportlab:"
        echo "  pip3 install reportlab"
        echo ""
        echo "Then run:"
        echo "  uv run python3 scripts/generate_verifier_reports.py"
    fi
else
    echo "⚠ Python 3 not found"
    echo ""
    echo "PDF report generation requires Python 3 and reportlab."
fi

# Generate documentation reports
echo ""
echo "======================================================================="
echo "Documentation Report Generation"
echo "======================================================================="
echo ""

# Determine test-plan-doc-gen directory
if [ -n "$TESTPLAN_DOC_GEN_DIR" ]; then
    DOC_GEN_DIR="$TESTPLAN_DOC_GEN_DIR"
else
    # Default to sibling directory
    DOC_GEN_DIR="$(cd "$PROJECT_ROOT/.." && pwd)/test-plan-doc-gen"
fi

echo "test-plan-doc-gen directory: $DOC_GEN_DIR"

# Run documentation report generation
DOC_SCRIPT="$SCRIPT_DIR/generate_documentation_reports.sh"

if [ -f "$DOC_SCRIPT" ]; then
    echo "Running documentation report generator..."
    echo ""
    
    # Build doc script command with config file and CLI overrides
    DOC_CMD="\"$DOC_SCRIPT\" --output-dir \"$OUTPUT_DIR\" --test-case-dir \"$PROJECT_ROOT/testcases\" --test-plan-doc-gen \"$DOC_GEN_DIR\""
    
    if [ -f "$CONFIG_FILE" ]; then
        DOC_CMD="$DOC_CMD --config \"$CONFIG_FILE\""
    fi
    
    if [ -n "$TITLE" ]; then
        DOC_CMD="$DOC_CMD --title \"$TITLE\""
    fi
    
    if [ -n "$PROJECT" ]; then
        DOC_CMD="$DOC_CMD --project \"$PROJECT\""
    fi
    
    if [ -n "$ENVIRONMENT" ]; then
        DOC_CMD="$DOC_CMD --environment \"$ENVIRONMENT\""
    fi
    
    if [ -n "$PLATFORM" ]; then
        DOC_CMD="$DOC_CMD --platform \"$PLATFORM\""
    fi
    
    if [ -n "$EXECUTOR" ]; then
        DOC_CMD="$DOC_CMD --executor \"$EXECUTOR\""
    fi
    
    if eval "$DOC_CMD"; then
        echo ""
        echo "✓ Documentation reports generated successfully"
    else
        echo ""
        echo "✗ Documentation report generation failed"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "✗ Documentation report generator not found: $DOC_SCRIPT"
    echo ""
    echo "To generate documentation reports, ensure the script exists at:"
    echo "  $DOC_SCRIPT"
    FAILURES=$((FAILURES + 1))
fi

echo ""
echo "======================================================================="
echo "Complete"
echo "======================================================================="
echo ""
echo "All verification reports are in: $OUTPUT_DIR"

if [ "$FAILURES" -gt 0 ]; then
    echo "✗ $FAILURES verifier/documentation step(s) failed"
    exit 1
fi
exit 0
