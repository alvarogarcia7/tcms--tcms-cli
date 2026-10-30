#!/usr/bin/env bash
#
# validate-verification.sh - Validate verification JSON output against schema
#
# DESCRIPTION:
#   This script validates test case verification output JSON files against
#   the verification schema. It uses the validate-json binary to perform
#   JSON Schema validation.
#
# USAGE:
#   validate-verification.sh <verification-json-file>
#
# ARGUMENTS:
#   verification-json-file    Path to the verification JSON file to validate
#
# CONFIGURATION:
#   VERIFICATION_SCHEMA  Environment variable to specify the schema file
#                        (default: schemas/tcms-schemas/tcms/verification-result.schema.json)
#   VALIDATE_JSON_BIN    Path to validate-json binary (default: auto-detected)
#
# EXIT CODES:
#   0 - Validation successful
#   1 - Validation failed or error occurred
#
# EXAMPLES:
#   # Validate a verification output file
#   ./scripts/validate-verification.sh test-output/SELF_VALIDATED_EXAMPLE_001_verification.json
#
#   # Use a custom schema
#   VERIFICATION_SCHEMA=custom_schema.json ./scripts/validate-verification.sh output.json
#
#   # Validate all verification files in test-output
#   for file in test-output/*_verification.json; do
#     ./scripts/validate-verification.sh "$file"
#   done
#

set -euo pipefail

# Configuration: Schema file to validate against
VERIFICATION_SCHEMA="${VERIFICATION_SCHEMA:-schemas/tcms-schemas/tcms/verification-result.schema.json}"

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

# Validate arguments
if [[ $# -eq 0 ]]; then
    echo "[ERROR] Missing required argument: verification JSON file path" >&2
    echo "Usage: $(basename "$0") <verification-json-file>" >&2
    exit 1
fi

VERIFICATION_FILE="$1"

# Validate that verification file exists
if [[ ! -f "$VERIFICATION_FILE" ]]; then
    echo "[ERROR] Verification file not found: $VERIFICATION_FILE" >&2
    exit 1
fi

# Validate that schema file exists
if [[ ! -f "$VERIFICATION_SCHEMA" ]]; then
    echo "[ERROR] Verification schema file not found: $VERIFICATION_SCHEMA" >&2
    echo "[ERROR] Set VERIFICATION_SCHEMA environment variable to specify the schema" >&2
    exit 1
fi

# Run validation
echo "[INFO] Validating $VERIFICATION_FILE against $VERIFICATION_SCHEMA"
"$VALIDATE_JSON" "$VERIFICATION_FILE" "$VERIFICATION_SCHEMA"
exit_code=$?

if [[ $exit_code -eq 0 ]]; then
    echo "[SUCCESS] Verification file is valid"
else
    echo "[FAILED] Verification file validation failed"
fi

# Exit with the same code as validate-json
exit $exit_code
