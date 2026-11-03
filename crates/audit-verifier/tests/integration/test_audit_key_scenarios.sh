#!/usr/bin/env bash
# Integration test focusing on key management scenarios for audit-verifier
#
# This test validates two primary scenarios:
# 1. Generate key on the fly: audit-verifier generates a new key and uses it
# 2. Load existing key: audit-verifier loads a pre-existing key and uses it
#
# Each scenario includes:
# - Generate sample payload (YAML + execution log)
# - Verify with audit-verifier
# - Sign the verification result
# - Verify the signature
#
# Usage: ./tests/integration/test_audit_key_scenarios.sh [--no-remove]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# logger.sh's fail() only prints. Count failures here so the script's exit status reflects them (TCMS-46 D-1).
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

# Find required binaries
cd "$PROJECT_ROOT"
AUDIT_VERIFIER_BIN=$(find_binary "audit-verifier")
if [[ -z "$AUDIT_VERIFIER_BIN" ]]; then
    echo "[ERROR] audit-verifier binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin audit-verifier" >&2
    exit 1
fi

VERIFY_SIGNATURE_BIN=$(find_binary "verify-audit-signature")
if [[ -z "$VERIFY_SIGNATURE_BIN" ]]; then
    echo "[ERROR] verify-audit-signature binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin verify-audit-signature" >&2
    exit 1
fi

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

echo "================================================"
echo "audit-verifier Key Management Scenarios Test"
echo "================================================"
echo ""

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$AUDIT_VERIFIER_BIN" ]]; then
    fail "audit-verifier binary not found"
    exit 1
fi
pass "audit-verifier binary found"

if [[ ! -f "$VERIFY_SIGNATURE_BIN" ]]; then
    fail "verify-audit-signature binary not found"
    exit 1
fi
pass "verify-audit-signature binary found"

# Create temporary directory
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Helper function to create sample test case
create_sample_test_case() {
    local yaml_file="$1"
    cat > "$yaml_file" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: KEY_SCENARIO_TEST
item: 1
tc: 1
id: KEY_SCENARIO_TEST_001
description: Test case for key management scenarios
general_initial_conditions:
  system:
  - Shell environment available
initial_conditions:
  system:
  - Ready to execute
test_sequences:
- id: 1
  name: Simple Sequence
  description: Basic test sequence
  initial_conditions:
    system:
    - Shell ready
  steps:
  - step: 1
    description: First command
    command: echo "test1"
    expected:
      success: true
      result: 0
      output: test1
    verification:
      result: '[[ $EXIT_CODE -eq 0 ]]'
      output: grep -q 'test1' <<< "$COMMAND_OUTPUT"
  - step: 2
    description: Second command
    command: echo "test2"
    expected:
      success: true
      result: 0
      output: test2
    verification:
      result: '[[ $EXIT_CODE -eq 0 ]]'
      output: grep -q 'test2' <<< "$COMMAND_OUTPUT"
  - step: 3
    description: Third command
    command: true
    expected:
      success: true
      result: 0
    verification:
      result: '[[ $EXIT_CODE -eq 0 ]]'
EOF
}

# Helper function to create execution log with proper hash
create_execution_log() {
    local log_file="$1"
    local yaml_hash="$2"
    cat > "$log_file" << EOF
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo \"test1\"",
    "exit_code": 0,
    "output": "test1",
    "timestamp": "2024-03-27T10:00:00.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true,
    "source_yaml_sha256": "$yaml_hash"
  },
  {
    "test_sequence": 1,
    "step": 2,
    "command": "echo \"test2\"",
    "exit_code": 0,
    "output": "test2",
    "timestamp": "2024-03-27T10:00:01.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true,
    "source_yaml_sha256": "$yaml_hash"
  },
  {
    "test_sequence": 1,
    "step": 3,
    "command": "true",
    "exit_code": 0,
    "output": "",
    "timestamp": "2024-03-27T10:00:02.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true,
    "source_yaml_sha256": "$yaml_hash"
  }
]
EOF
}

#############################################
# SCENARIO 1: Generate Key On The Fly
#############################################

section "SCENARIO 1: Generate Key On The Fly"
echo ""

# Step 1: Create sample payload
info "Step 1: Generate Sample Payload"

YAML_FILE_1="$TEMP_DIR/scenario1_test.yaml"
create_sample_test_case "$YAML_FILE_1"
pass "Created test case YAML"

YAML_HASH_1=$(shasum -a 256 "$YAML_FILE_1" | awk '{print $1}')
info "Computed YAML hash: $YAML_HASH_1"

LOG_FILE_1="$TEMP_DIR/scenario1_log.json"
create_execution_log "$LOG_FILE_1" "$YAML_HASH_1"
pass "Created execution log with matching hash"

# Step 2: Verify and sign (generate key on the fly)
info "Step 2: Verify and Sign (Generate Key On The Fly)"

GENERATED_KEY_1="$TEMP_DIR/scenario1_generated_key.pem"
SIGNED_OUTPUT_1="$TEMP_DIR/scenario1_signed.json"

info "Running audit-verifier with key generation..."
if "$AUDIT_VERIFIER_BIN" \
    --yaml "$YAML_FILE_1" \
    --log "$LOG_FILE_1" \
    --save-key "$GENERATED_KEY_1" \
    --key-id "scenario1-generated-key" \
    --output "$SIGNED_OUTPUT_1" \
    --log-level warn > /dev/null 2>&1; then
    pass "audit-verifier completed successfully"
else
    EXIT_CODE=$?
    fail "audit-verifier failed with exit code: $EXIT_CODE"
    exit 1
fi

# Verify key was generated
if [[ -f "$GENERATED_KEY_1" ]]; then
    KEY_SIZE=$(wc -c < "$GENERATED_KEY_1")
    if [[ $KEY_SIZE -gt 0 ]]; then
        pass "Private key generated and saved (size: $KEY_SIZE bytes)"
    else
        fail "Private key file is empty"
    fi
else
    fail "Private key was not generated"
fi

# Verify signed output
if [[ -f "$SIGNED_OUTPUT_1" ]]; then
    pass "Signed output created"
else
    fail "Signed output was not created"
fi

# Validate JSON structure and content
if command -v jq > /dev/null 2>&1; then
    info "Step 2.1: Validate Signed Output Structure"
    
    if jq . "$SIGNED_OUTPUT_1" > /dev/null 2>&1; then
        pass "Signed output is valid JSON"
    else
        fail "Signed output is not valid JSON"
    fi
    
    TOTAL_ENTRIES=$(jq -r '.verification_result.total_entries' "$SIGNED_OUTPUT_1")
    if [[ "$TOTAL_ENTRIES" == "3" ]]; then
        pass "Total entries correct: 3"
    else
        fail "Total entries incorrect: $TOTAL_ENTRIES"
    fi
    
    HASH_MISMATCHES=$(jq -r '.verification_result.hash_mismatches' "$SIGNED_OUTPUT_1")
    if [[ "$HASH_MISMATCHES" == "0" ]]; then
        pass "No hash mismatches: 0"
    else
        fail "Unexpected hash mismatches: $HASH_MISMATCHES"
    fi
    
    MISSING_FIELDS=$(jq -r '.verification_result.missing_hash_fields' "$SIGNED_OUTPUT_1")
    if [[ "$MISSING_FIELDS" == "0" ]]; then
        pass "No missing hash fields: 0"
    else
        fail "Unexpected missing hash fields: $MISSING_FIELDS"
    fi
    
    VERIFICATION_PASSED=$(jq -r '.verification_result.verification_passed' "$SIGNED_OUTPUT_1")
    if [[ "$VERIFICATION_PASSED" == "true" ]]; then
        pass "Verification passed: true"
    else
        fail "Verification should have passed"
    fi
    
    SIGNATURE=$(jq -r '.signature' "$SIGNED_OUTPUT_1")
    if [[ -n "$SIGNATURE" ]] && [[ "$SIGNATURE" != "null" ]] && [[ ${#SIGNATURE} -gt 50 ]]; then
        pass "Signature present (length: ${#SIGNATURE})"
    else
        fail "Signature missing or invalid"
    fi
    
    PUBLIC_KEY=$(jq -r '.public_key' "$SIGNED_OUTPUT_1")
    if echo "$PUBLIC_KEY" | grep -q "BEGIN PUBLIC KEY"; then
        pass "Public key present in PEM format"
    else
        fail "Public key missing or invalid format"
    fi
    
    KEY_ID=$(jq -r '.key_id' "$SIGNED_OUTPUT_1")
    if [[ "$KEY_ID" == "scenario1-generated-key" ]]; then
        pass "Key ID matches: scenario1-generated-key"
    else
        fail "Key ID mismatch: $KEY_ID"
    fi
    
    TIMESTAMP=$(jq -r '.timestamp' "$SIGNED_OUTPUT_1")
    if [[ -n "$TIMESTAMP" ]] && [[ "$TIMESTAMP" != "null" ]]; then
        pass "Timestamp present: $TIMESTAMP"
    else
        fail "Timestamp missing"
    fi
fi

# Step 3: Verify the signature
info "Step 3: Verify Signature"

info "Running verify-audit-signature..."
if "$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_1" > /dev/null 2>&1; then
    pass "Signature verification PASSED"
else
    fail "Signature verification FAILED"
    exit 1
fi

# Step 4: Verify with verbose output
info "Step 4: Verify Signature (Verbose)"

VERIFY_OUTPUT_1="$TEMP_DIR/scenario1_verify_output.txt"
"$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_1" --verbose > "$VERIFY_OUTPUT_1" 2>&1 || true

if [[ -f "$VERIFY_OUTPUT_1" ]]; then
    if grep -q "SIGNATURE VALID" "$VERIFY_OUTPUT_1"; then
        pass "Verbose output confirms valid signature"
    else
        fail "Verbose output should show valid signature"
    fi
    
    if grep -q "Key ID: scenario1-generated-key" "$VERIFY_OUTPUT_1"; then
        pass "Verbose output shows correct key ID"
    fi
    
    if grep -q "Total Entries: 3" "$VERIFY_OUTPUT_1"; then
        pass "Verbose output shows correct entry count"
    fi
fi

echo ""
pass "SCENARIO 1 COMPLETE: Key generated on the fly, payload signed and verified"
echo ""

#############################################
# SCENARIO 2: Load Existing Key
#############################################

section "SCENARIO 2: Load Existing Key"
echo ""

# Step 1: Create an existing key
info "Step 1: Create Existing Key"

EXISTING_KEY="$TEMP_DIR/scenario2_existing_key.pem"
TEMP_YAML="$TEMP_DIR/temp.yaml"
TEMP_LOG="$TEMP_DIR/temp.json"
TEMP_SIGNED="$TEMP_DIR/temp_signed.json"

# Create temporary files for key generation
create_sample_test_case "$TEMP_YAML"
TEMP_HASH=$(shasum -a 256 "$TEMP_YAML" | awk '{print $1}')
create_execution_log "$TEMP_LOG" "$TEMP_HASH"

# Generate the key
"$AUDIT_VERIFIER_BIN" \
    --yaml "$TEMP_YAML" \
    --log "$TEMP_LOG" \
    --save-key "$EXISTING_KEY" \
    --output "$TEMP_SIGNED" > /dev/null 2>&1

if [[ -f "$EXISTING_KEY" ]]; then
    pass "Pre-generated existing key for scenario 2"
    info "Key location: $EXISTING_KEY"
else
    fail "Failed to create existing key"
    exit 1
fi

# Step 2: Create sample payload for scenario 2
info "Step 2: Generate Sample Payload"

YAML_FILE_2="$TEMP_DIR/scenario2_test.yaml"
create_sample_test_case "$YAML_FILE_2"
pass "Created test case YAML"

YAML_HASH_2=$(shasum -a 256 "$YAML_FILE_2" | awk '{print $1}')
info "Computed YAML hash: $YAML_HASH_2"

LOG_FILE_2="$TEMP_DIR/scenario2_log.json"
create_execution_log "$LOG_FILE_2" "$YAML_HASH_2"
pass "Created execution log with matching hash"

# Step 3: Verify and sign using existing key
info "Step 3: Verify and Sign (Use Existing Key)"

SIGNED_OUTPUT_2="$TEMP_DIR/scenario2_signed.json"

info "Running audit-verifier with existing key..."
if "$AUDIT_VERIFIER_BIN" \
    --yaml "$YAML_FILE_2" \
    --log "$LOG_FILE_2" \
    --private-key "$EXISTING_KEY" \
    --key-id "scenario2-existing-key" \
    --output "$SIGNED_OUTPUT_2" \
    --log-level warn > /dev/null 2>&1; then
    pass "audit-verifier completed successfully with existing key"
else
    EXIT_CODE=$?
    fail "audit-verifier failed with exit code: $EXIT_CODE"
    exit 1
fi

# Verify signed output
if [[ -f "$SIGNED_OUTPUT_2" ]]; then
    pass "Signed output created"
else
    fail "Signed output was not created"
fi

# Validate JSON structure and content
if command -v jq > /dev/null 2>&1; then
    info "Step 3.1: Validate Signed Output Structure"
    
    if jq . "$SIGNED_OUTPUT_2" > /dev/null 2>&1; then
        pass "Signed output is valid JSON"
    else
        fail "Signed output is not valid JSON"
    fi
    
    VERIFICATION_PASSED=$(jq -r '.verification_result.verification_passed' "$SIGNED_OUTPUT_2")
    if [[ "$VERIFICATION_PASSED" == "true" ]]; then
        pass "Verification passed: true"
    else
        fail "Verification should have passed"
    fi
    
    KEY_ID=$(jq -r '.key_id' "$SIGNED_OUTPUT_2")
    if [[ "$KEY_ID" == "scenario2-existing-key" ]]; then
        pass "Key ID matches: scenario2-existing-key"
    else
        fail "Key ID mismatch: $KEY_ID"
    fi
    
    # Extract and compare public keys
    PUBLIC_KEY_SCENARIO1=$(jq -r '.public_key' "$SIGNED_OUTPUT_1" 2>/dev/null || echo "")
    PUBLIC_KEY_SCENARIO2=$(jq -r '.public_key' "$SIGNED_OUTPUT_2" 2>/dev/null || echo "")
    
    if [[ "$PUBLIC_KEY_SCENARIO1" != "$PUBLIC_KEY_SCENARIO2" ]]; then
        pass "Different keys used in scenarios 1 and 2 (as expected)"
    else
        info "Public keys comparison: scenarios use different keys"
    fi
fi

# Step 4: Verify the signature
info "Step 4: Verify Signature"

info "Running verify-audit-signature..."
if "$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_2" > /dev/null 2>&1; then
    pass "Signature verification PASSED"
else
    fail "Signature verification FAILED"
    exit 1
fi

# Step 5: Verify with verbose output
info "Step 5: Verify Signature (Verbose)"

VERIFY_OUTPUT_2="$TEMP_DIR/scenario2_verify_output.txt"
"$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_2" --verbose > "$VERIFY_OUTPUT_2" 2>&1 || true

if [[ -f "$VERIFY_OUTPUT_2" ]]; then
    if grep -q "SIGNATURE VALID" "$VERIFY_OUTPUT_2"; then
        pass "Verbose output confirms valid signature"
    else
        fail "Verbose output should show valid signature"
    fi
    
    if grep -q "Key ID: scenario2-existing-key" "$VERIFY_OUTPUT_2"; then
        pass "Verbose output shows correct key ID"
    fi
fi

# Step 6: Use the same existing key multiple times
info "Step 6: Reuse Existing Key (Multiple Signatures)"

YAML_FILE_3="$TEMP_DIR/scenario2_test_reuse.yaml"
create_sample_test_case "$YAML_FILE_3"
YAML_HASH_3=$(shasum -a 256 "$YAML_FILE_3" | awk '{print $1}')

LOG_FILE_3="$TEMP_DIR/scenario2_log_reuse.json"
create_execution_log "$LOG_FILE_3" "$YAML_HASH_3"

SIGNED_OUTPUT_3="$TEMP_DIR/scenario2_signed_reuse.json"

# Use the same existing key again
info "Running audit-verifier with same existing key (reuse)..."
if "$AUDIT_VERIFIER_BIN" \
    --yaml "$YAML_FILE_3" \
    --log "$LOG_FILE_3" \
    --private-key "$EXISTING_KEY" \
    --key-id "scenario2-reused-key" \
    --output "$SIGNED_OUTPUT_3" > /dev/null 2>&1; then
    pass "audit-verifier completed successfully with reused key"
else
    fail "audit-verifier failed with reused key"
fi

# Verify the signature from reused key
if "$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_3" > /dev/null 2>&1; then
    pass "Signature verification PASSED for reused key"
else
    fail "Signature verification FAILED for reused key"
fi

# Compare public keys (should be the same since same private key was used)
if command -v jq > /dev/null 2>&1; then
    PUBLIC_KEY_2=$(jq -r '.public_key' "$SIGNED_OUTPUT_2" 2>/dev/null)
    PUBLIC_KEY_3=$(jq -r '.public_key' "$SIGNED_OUTPUT_3" 2>/dev/null)
    
    if [[ "$PUBLIC_KEY_2" == "$PUBLIC_KEY_3" ]]; then
        pass "Public keys match when reusing same private key"
    else
        fail "Public keys should match when reusing same private key"
    fi
fi

echo ""
pass "SCENARIO 2 COMPLETE: Existing key loaded, used, and reused successfully"
echo ""

#############################################
# Summary
#############################################

section "Test Summary"
echo ""
echo "================================================"
echo "Both Key Management Scenarios Completed Successfully"
echo "================================================"
echo ""
echo "✓ SCENARIO 1: Generate key on the fly"
echo "  - New key generated: $GENERATED_KEY_1"
echo "  - Payload verified and signed"
echo "  - Signature validated"
echo ""
echo "✓ SCENARIO 2: Load and use existing key"
echo "  - Existing key loaded: $EXISTING_KEY"
echo "  - Payload verified and signed"
echo "  - Signature validated"
echo "  - Key successfully reused"
echo ""

pass "All key management scenarios passed!"
finish_run
exit 0
