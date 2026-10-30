#!/usr/bin/env bash
set -e

# Script: validate_testcases_report.sh
# Purpose: Discover and validate all YAML test case files against schema,
#          generate a comprehensive validation report with pass/fail status,
#          detailed error messages, and summary statistics.
# Compatible with bash 3.2+ (avoids associative arrays)

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configuration
# Note: Schema path can be overridden by setting SCHEMA_FILE environment variable
# Example: SCHEMA_FILE=data/schema.json ./scripts/validate_testcases_report.sh
SCHEMA_FILE="${SCHEMA_FILE:-schemas/tcms-schemas/tcms/test-case-main.schema.json}"
VALIDATE_YAML_BIN="${VALIDATE_YAML_BIN:-./target/debug/validate-yaml}"
REPORT_DIR="${REPORT_DIR:-reports}"
REPORT_FILE="${REPORT_DIR}/validation_report.txt"

# Counters
total_files=0
passed_files=0
failed_files=0
declare -a failed_file_list

# Color codes for console output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo "[INFO] $*"
}

log_error() {
    echo "[ERROR] $*" >&2
}

pass() {
    echo -e "${GREEN}✓${NC} $1"
}

fail() {
    echo -e "${RED}✗${NC} $1"
}

section() {
    echo ""
    echo -e "${YELLOW}=== $1 ===${NC}"
}

# Start validation
log_info "Starting test case validation report generation..."
log_info "Schema file: $SCHEMA_FILE"
log_info "Validate-yaml binary: $VALIDATE_YAML_BIN"
echo ""

# Check if schema exists
if [ ! -f "$SCHEMA_FILE" ]; then
    log_error "Schema file not found: $SCHEMA_FILE"
    exit 1
fi

# Check if validate-yaml binary exists
if [ ! -x "$VALIDATE_YAML_BIN" ]; then
    log_error "validate-yaml binary not found or not executable: $VALIDATE_YAML_BIN"
    log_info "Please build it first: cargo build --bin validate-yaml"
    exit 1
fi

# Create reports directory if it doesn't exist
mkdir -p "$REPORT_DIR"

# Create temporary files for results and errors
temp_results=$(mktemp)
temp_errors=$(mktemp)
temp_failed_details=$(mktemp)
trap 'rm -f "$temp_results" "$temp_errors" "$temp_failed_details"' EXIT

# Discover all YAML test case files
# Pattern: *.yml and *.yaml
# Exclusions:
# - *te.y* (test execution files)
# - sample_test_runs.yaml
# - *wrong* files (known invalid test files)
log_info "Discovering test case files..."

test_case_files=$(find testcases tests/sample data -type f \( -name "*.yml" -o -name "*.yaml" \) 2>/dev/null | \
    grep -v "te\.y" | \
    grep -v "sample_test_runs\.yaml" | \
    grep -v "wrong" | \
    sort)

if [ -z "$test_case_files" ]; then
    log_error "No test case files found to validate"
    exit 1
fi

file_count=$(echo "$test_case_files" | wc -l | tr -d ' ')
log_info "Found $file_count test case files"
echo ""

# Validate each file
section "Validating Files"
echo ""

while IFS= read -r file; do
    if [ -z "$file" ]; then
        continue
    fi
    
    total_files=$((total_files + 1))
    
    # Run validation using validate-yaml binary
    # Capture both stdout and stderr
    validation_output=$("$VALIDATE_YAML_BIN" --schema "$SCHEMA_FILE" "$file" 2>&1)
    validation_exit_code=$?
    
    if [ $validation_exit_code -eq 0 ]; then
        pass "$file" | tee -a "$temp_results"
        passed_files=$((passed_files + 1))
    else
        fail "$file" | tee -a "$temp_results"
        
        # Extract and store error details
        error_details=$(echo "$validation_output" | grep -A 1000 "Schema constraint violations:" | grep -v "Summary:" | grep -v "Total files validated:" | grep -v "Passed:" | grep -v "Failed:" || echo "$validation_output")
        
        # Store error in temp file with delimiter for later retrieval
        echo "FILE_START:$file" >> "$temp_errors"
        echo "$error_details" >> "$temp_errors"
        echo "FILE_END:$file" >> "$temp_errors"
        
        # Write error details to temp results
        echo "  Error details:" >> "$temp_results"
        echo "$error_details" | sed 's/^/    /' >> "$temp_results"
        echo "" >> "$temp_results"
        
        failed_files=$((failed_files + 1))
        failed_file_list+=("$file")
    fi
done <<< "$test_case_files"

# Display summary
echo ""
section "Summary"
log_info "Total files validated: $total_files"
pass "Passed: $passed_files"
if [ $failed_files -gt 0 ]; then
    fail "Failed: $failed_files"
else
    log_info "Failed: $failed_files"
fi
echo ""

# Generate failed files section for report
if [ $failed_files -gt 0 ]; then
    {
        echo "================================================================================"
        echo "                           Failed Files"
        echo "================================================================================"
        echo ""
        
        for file in "${failed_file_list[@]}"; do
            echo "✗ $file"
            echo ""
            echo "Error Details:"
            echo "-------------"
            
            # Extract error message for this file from temp_errors
            in_file=0
            while IFS= read -r line; do
                if [ "$line" = "FILE_START:$file" ]; then
                    in_file=1
                elif [ "$line" = "FILE_END:$file" ]; then
                    break
                elif [ $in_file -eq 1 ]; then
                    echo "  $line"
                fi
            done < "$temp_errors"
            
            echo ""
            echo "To validate this file manually, run:"
            echo "  $VALIDATE_YAML_BIN --schema $SCHEMA_FILE $file"
            echo ""
            echo "-------------------------------------------------------------------------------"
            echo ""
        done
    } > "$temp_failed_details"
fi

# Generate detailed report
{
    echo "================================================================================"
    echo "                    Test Case Validation Report"
    echo "================================================================================"
    echo ""
    echo "Generated: $(date)"
    echo "Schema: $SCHEMA_FILE"
    echo ""
    echo "================================================================================"
    echo "                              Summary"
    echo "================================================================================"
    echo ""
    echo "Total files validated: $total_files"
    echo "Passed: $passed_files"
    echo "Failed: $failed_files"
    echo ""
    
    if [ $failed_files -eq 0 ]; then
        echo "✓ All test case files passed validation!"
        echo ""
    else
        cat "$temp_failed_details"
    fi
    
    echo "================================================================================"
    echo "                        Validation Results Detail"
    echo "================================================================================"
    echo ""
    
    # Strip color codes from temp results
    sed 's/\x1b\[[0-9;]*m//g' "$temp_results"
    
    echo ""
    echo "================================================================================"
    echo "                           End of Report"
    echo "================================================================================"
} > "$REPORT_FILE"

log_info "Detailed report saved to: $REPORT_FILE"
echo ""

# Exit with error code if any failures found
if [ $failed_files -gt 0 ]; then
    log_error "Validation failed for $failed_files file(s)"
    exit 1
else
    log_info "All validations passed successfully!"
    exit 0
fi
