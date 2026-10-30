#!/usr/bin/env bash
#
# validate-execution-log.sh - Validates test execution log files
#
# DESCRIPTION:
#   This script validates test_execution_log.json files against the
#   execution-log.schema.json schema. It can validate either a single
#   execution log file or batch validate all execution logs in a directory.
#
# USAGE:
#   validate-execution-log.sh <execution-log-file>
#   validate-execution-log.sh --all [directory]
#
# ARGUMENTS:
#   execution-log-file    Path to a single execution log JSON file to validate
#   --all                 Validate all *_execution_log.json files in directory
#   directory             Optional directory to search (default: current directory)
#
# CONFIGURATION:
#   SCHEMA_FILE           Path to schema file (default: schemas/tcms-schemas/tcms/execution-log.schema.json)
#   VALIDATE_JSON_BIN     Path to validate-json binary (default: auto-detected)
#
# EXIT CODES:
#   0 - All validations successful
#   1 - One or more validations failed or error occurred
#
# EXAMPLES:
#   # Validate a single execution log
#   ./scripts/validate-execution-log.sh output/test123_execution_log.json
#
#   # Validate all execution logs in current directory
#   ./scripts/validate-execution-log.sh --all
#
#   # Validate all execution logs in a specific directory
#   ./scripts/validate-execution-log.sh --all output/
#
#   # Use a custom schema
#   SCHEMA_FILE=custom_schema.json ./scripts/validate-execution-log.sh test_execution_log.json
#

set -euo pipefail

# Configuration: Schema file to validate against
SCHEMA_FILE="${SCHEMA_FILE:-schemas/tcms-schemas/tcms/execution-log.schema.json}"

# Auto-detect validate-json binary location
if [[ -n "${VALIDATE_JSON_BIN:-}" ]]; then
    VALIDATE_JSON="$VALIDATE_JSON_BIN"
elif [[ -x "target/release/validate-json" ]]; then
    VALIDATE_JSON="target/release/validate-json"
elif [[ -x "target/debug/validate-json" ]]; then
    VALIDATE_JSON="target/debug/validate-json"
elif command -v validate-json >/dev/null 2>&1; then
    VALIDATE_JSON="validate-json"
else
    echo "[ERROR] validate-json binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin validate-json" >&2
    exit 1
fi

# Validate that schema file exists
if [[ ! -f "$SCHEMA_FILE" ]]; then
    echo "[ERROR] Schema file not found: $SCHEMA_FILE" >&2
    echo "[ERROR] Set SCHEMA_FILE environment variable to specify the schema" >&2
    exit 1
fi

# Function to validate a single file
validate_file() {
    local file="$1"
    local exit_code=0
    
    if [[ ! -f "$file" ]]; then
        echo "[ERROR] File not found: $file" >&2
        return 1
    fi
    
    echo "[INFO] Validating: $file"
    if "$VALIDATE_JSON" "$file" "$SCHEMA_FILE"; then
        echo "[PASS] $file"
        return 0
    else
        exit_code=$?
        echo "[FAIL] $file" >&2
        return $exit_code
    fi
}

# Parse arguments
if [[ $# -eq 0 ]]; then
    echo "[ERROR] Missing required argument" >&2
    echo "Usage: $(basename "$0") <execution-log-file>" >&2
    echo "       $(basename "$0") --all [directory]" >&2
    exit 1
fi

# Check if batch validation is requested
if [[ "$1" == "--all" ]]; then
    # Batch validation mode
    SEARCH_DIR="${2:-.}"
    
    if [[ ! -d "$SEARCH_DIR" ]]; then
        echo "[ERROR] Directory not found: $SEARCH_DIR" >&2
        exit 1
    fi
    
    echo "[INFO] Searching for execution log files in: $SEARCH_DIR"
    
    # Find all execution log files
    mapfile -t log_files < <(find "$SEARCH_DIR" -type f -name '*_execution_log.json' | sort)
    
    if [[ ${#log_files[@]} -eq 0 ]]; then
        echo "[WARNING] No execution log files found in: $SEARCH_DIR" >&2
        exit 0
    fi
    
    echo "[INFO] Found ${#log_files[@]} execution log file(s)"
    echo ""
    
    # Validate each file
    failed_count=0
    passed_count=0
    
    for log_file in "${log_files[@]}"; do
        if validate_file "$log_file"; then
            ((passed_count++)) || true
        else
            ((failed_count++)) || true
        fi
        echo ""
    done
    
    # Print summary
    echo "==============================================="
    echo "Validation Summary:"
    echo "  Total:  ${#log_files[@]}"
    echo "  Passed: $passed_count"
    echo "  Failed: $failed_count"
    echo "==============================================="
    
    if [[ $failed_count -gt 0 ]]; then
        exit 1
    fi
    
    exit 0
else
    # Single file validation mode
    LOG_FILE="$1"
    
    if [[ ! -f "$LOG_FILE" ]]; then
        echo "[ERROR] Execution log file not found: $LOG_FILE" >&2
        exit 1
    fi
    
    # Run validation
    "$VALIDATE_JSON" "$LOG_FILE" "$SCHEMA_FILE"
    exit_code=$?
    
    if [[ $exit_code -eq 0 ]]; then
        echo "[PASS] Validation successful: $LOG_FILE"
    else
        echo "[FAIL] Validation failed: $LOG_FILE" >&2
    fi
    
    exit $exit_code
fi
