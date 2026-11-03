#!/usr/bin/env bash
# End-to-end integration test for audit-verifier binary
#
# This test validates:
# 1. Generate sample payload (test case YAML and execution log)
# 2. Verify payload with audit-verifier (with and without existing key)
# 3. Sign the payload
# 4. Verify the signature
#
# Usage: ./tests/integration/test_audit_verifier_e2e.sh [--no-remove]

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

# Find audit-verifier binary using workspace-aware search
cd "$PROJECT_ROOT"
AUDIT_VERIFIER_BIN=$(find_binary "audit-verifier")
if [[ -z "$AUDIT_VERIFIER_BIN" ]]; then
    echo "[ERROR] audit-verifier binary not found" >&2
    echo "[ERROR] Please build it with: cargo build --bin audit-verifier" >&2
    exit 1
fi

# Find verify-audit-signature binary
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

echo "==========================================="
echo "audit-verifier End-to-End Integration Test"
echo "==========================================="
echo ""

# Check prerequisites
section "Checking Prerequisites"

if [[ ! -f "$AUDIT_VERIFIER_BIN" ]]; then
    fail "audit-verifier binary not found at $AUDIT_VERIFIER_BIN"
    echo "Run 'cargo build -p audit-verifier' first"
    exit 1
fi
pass "audit-verifier binary found"

if [[ ! -f "$VERIFY_SIGNATURE_BIN" ]]; then
    fail "verify-audit-signature binary not found at $VERIFY_SIGNATURE_BIN"
    echo "Run 'cargo build -p audit-verifier' first"
    exit 1
fi
pass "verify-audit-signature binary found"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# Test 1: Generate sample test case YAML
section "Test 1: Generate Sample Test Case YAML"

TEST_YAML="$TEMP_DIR/test_case.yaml"
cat > "$TEST_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: AUDIT_TEST_001
item: 1
tc: 1
id: AUDIT_TEST_001
description: Simple test case for audit verification
general_initial_conditions:
  system:
  - Shell environment is available
initial_conditions:
  system:
  - All commands can be executed
test_sequences:
- id: 1
  name: Basic Commands
  description: Test sequence with basic shell commands
  initial_conditions:
    system:
    - Shell is ready
  steps:
  - step: 1
    description: Echo hello
    command: echo "hello"
    expected:
      success: true
      result: 0
      output: hello
    verification:
      result: '[[ $EXIT_CODE -eq 0 ]]'
      output: grep -q 'hello' <<< "$COMMAND_OUTPUT"
  - step: 2
    description: Echo world
    command: echo "world"
    expected:
      success: true
      result: 0
      output: world
    verification:
      result: '[[ $EXIT_CODE -eq 0 ]]'
      output: grep -q 'world' <<< "$COMMAND_OUTPUT"
EOF

pass "Created test case YAML: $TEST_YAML"

# Compute expected hash
EXPECTED_HASH=$(shasum -a 256 "$TEST_YAML" | awk '{print $1}')
info "Expected SHA-256 hash: $EXPECTED_HASH"

# Test 2: Generate sample execution log with hash
section "Test 2: Generate Sample Execution Log"

EXECUTION_LOG="$TEMP_DIR/execution_log.json"
cat > "$EXECUTION_LOG" << EOF
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo \"hello\"",
    "exit_code": 0,
    "output": "hello",
    "timestamp": "2024-01-01T10:00:00.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true,
    "source_yaml_sha256": "$EXPECTED_HASH"
  },
  {
    "test_sequence": 1,
    "step": 2,
    "command": "echo \"world\"",
    "exit_code": 0,
    "output": "world",
    "timestamp": "2024-01-01T10:00:01.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true,
    "source_yaml_sha256": "$EXPECTED_HASH"
  }
]
EOF

pass "Created execution log: $EXECUTION_LOG"

# Test 3: Verify payload with generated key (no existing key)
section "Test 3: Verify and Sign with Generated Key"

GENERATED_KEY="$TEMP_DIR/generated_key.pem"
SIGNED_OUTPUT_1="$TEMP_DIR/signed_output_generated_key.json"

if "$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "$EXECUTION_LOG" \
    --save-key "$GENERATED_KEY" \
    --key-id "test-key-generated" \
    --output "$SIGNED_OUTPUT_1" \
    --log-level info > /dev/null 2>&1; then
    pass "audit-verifier completed with generated key"
else
    EXIT_CODE=$?
    fail "audit-verifier failed with exit code: $EXIT_CODE"
    exit 1
fi

# Verify generated key exists
if [[ -f "$GENERATED_KEY" ]]; then
    pass "Private key was generated and saved: $GENERATED_KEY"
else
    fail "Private key file not created"
fi

# Verify signed output exists
if [[ -f "$SIGNED_OUTPUT_1" ]]; then
    pass "Signed output created: $SIGNED_OUTPUT_1"
else
    fail "Signed output file not created"
fi

# Validate signed output structure
if command -v jq > /dev/null 2>&1; then
    if jq . "$SIGNED_OUTPUT_1" > /dev/null 2>&1; then
        pass "Signed output is valid JSON"
    else
        fail "Signed output is not valid JSON"
    fi
    
    # Check for required fields
    VERIFICATION_PASSED=$(jq -r '.verification_result.verification_passed' "$SIGNED_OUTPUT_1" 2>/dev/null)
    if [[ "$VERIFICATION_PASSED" == "true" ]]; then
        pass "Verification passed in signed output"
    else
        fail "Verification should have passed"
    fi
    
    COMPUTED_HASH=$(jq -r '.verification_result.computed_hash' "$SIGNED_OUTPUT_1" 2>/dev/null)
    if [[ "$COMPUTED_HASH" == "$EXPECTED_HASH" ]]; then
        pass "Computed hash matches expected: $EXPECTED_HASH"
    else
        fail "Computed hash mismatch: expected $EXPECTED_HASH, got $COMPUTED_HASH"
    fi
    
    SIGNATURE=$(jq -r '.signature' "$SIGNED_OUTPUT_1" 2>/dev/null)
    if [[ -n "$SIGNATURE" ]] && [[ "$SIGNATURE" != "null" ]]; then
        pass "Signature present in output"
    else
        fail "Signature missing from output"
    fi
    
    PUBLIC_KEY=$(jq -r '.public_key' "$SIGNED_OUTPUT_1" 2>/dev/null)
    if [[ -n "$PUBLIC_KEY" ]] && [[ "$PUBLIC_KEY" != "null" ]]; then
        pass "Public key present in output"
    else
        fail "Public key missing from output"
    fi
    
    KEY_ID=$(jq -r '.key_id' "$SIGNED_OUTPUT_1" 2>/dev/null)
    if [[ "$KEY_ID" == "test-key-generated" ]]; then
        pass "Key ID matches expected: test-key-generated"
    else
        fail "Key ID mismatch: expected test-key-generated, got $KEY_ID"
    fi
else
    info "jq not available, skipping JSON structure validation"
fi

# Test 4: Verify signature with verify-audit-signature
section "Test 4: Verify Signature (Generated Key)"

if "$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_1" > /dev/null 2>&1; then
    pass "Signature verification passed for generated key"
else
    fail "Signature verification failed for generated key"
fi

# Test 5: Verify and sign with existing key
section "Test 5: Verify and Sign with Existing Key"

EXISTING_KEY="$TEMP_DIR/existing_key.pem"
SIGNED_OUTPUT_2="$TEMP_DIR/signed_output_existing_key.json"

# First, generate a key to use as "existing"
TEMP_SIGNED="$TEMP_DIR/temp_signed.json"
"$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "$EXECUTION_LOG" \
    --save-key "$EXISTING_KEY" \
    --key-id "setup-key" \
    --output "$TEMP_SIGNED" > /dev/null 2>&1

if [[ -f "$EXISTING_KEY" ]]; then
    pass "Created existing key for testing: $EXISTING_KEY"
else
    fail "Failed to create existing key"
    exit 1
fi

# Now use the existing key
if "$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "$EXECUTION_LOG" \
    --private-key "$EXISTING_KEY" \
    --key-id "test-key-existing" \
    --output "$SIGNED_OUTPUT_2" \
    --log-level info > /dev/null 2>&1; then
    pass "audit-verifier completed with existing key"
else
    EXIT_CODE=$?
    fail "audit-verifier failed with existing key, exit code: $EXIT_CODE"
    exit 1
fi

# Verify signed output exists
if [[ -f "$SIGNED_OUTPUT_2" ]]; then
    pass "Signed output created with existing key: $SIGNED_OUTPUT_2"
else
    fail "Signed output file not created with existing key"
fi

# Validate signed output structure
if command -v jq > /dev/null 2>&1; then
    VERIFICATION_PASSED=$(jq -r '.verification_result.verification_passed' "$SIGNED_OUTPUT_2" 2>/dev/null)
    if [[ "$VERIFICATION_PASSED" == "true" ]]; then
        pass "Verification passed with existing key"
    else
        fail "Verification should have passed with existing key"
    fi
    
    KEY_ID=$(jq -r '.key_id' "$SIGNED_OUTPUT_2" 2>/dev/null)
    if [[ "$KEY_ID" == "test-key-existing" ]]; then
        pass "Key ID matches expected: test-key-existing"
    else
        fail "Key ID mismatch: expected test-key-existing, got $KEY_ID"
    fi
fi

# Test 6: Verify signature with existing key
section "Test 6: Verify Signature (Existing Key)"

if "$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_2" > /dev/null 2>&1; then
    pass "Signature verification passed for existing key"
else
    fail "Signature verification failed for existing key"
fi

# Test 7: Verify signature with verbose output
section "Test 7: Verify Signature with Verbose Output"

VERBOSE_OUTPUT="$TEMP_DIR/verify_verbose.txt"
if "$VERIFY_SIGNATURE_BIN" --input "$SIGNED_OUTPUT_2" --verbose > "$VERBOSE_OUTPUT" 2>&1; then
    pass "Verbose signature verification completed"
else
    fail "Verbose signature verification failed"
fi

if [[ -f "$VERBOSE_OUTPUT" ]]; then
    if grep -q "SIGNATURE VALID" "$VERBOSE_OUTPUT"; then
        pass "Verbose output confirms signature valid"
    else
        fail "Verbose output should show signature valid"
    fi
    
    if grep -q "Key ID:" "$VERBOSE_OUTPUT"; then
        pass "Verbose output includes key ID"
    else
        fail "Verbose output should include key ID"
    fi
fi

# Test 8: Test with mismatched hash (negative test)
section "Test 8: Negative Test - Hash Mismatch"

WRONG_LOG="$TEMP_DIR/wrong_execution_log.json"
cat > "$WRONG_LOG" << 'EOF'
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo \"hello\"",
    "exit_code": 0,
    "output": "hello",
    "timestamp": "2024-01-01T10:00:00.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true,
    "source_yaml_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
  }
]
EOF

SIGNED_OUTPUT_FAIL="$TEMP_DIR/signed_output_fail.json"

if "$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "$WRONG_LOG" \
    --save-key "$TEMP_DIR/fail_key.pem" \
    --key-id "test-key-fail" \
    --output "$SIGNED_OUTPUT_FAIL" > /dev/null 2>&1; then
    fail "audit-verifier should fail with hash mismatch (got exit 0)"
else
    EXIT_CODE=$?
    pass "audit-verifier correctly failed with hash mismatch (exit code: $EXIT_CODE)"
fi

# Verify output still created (even on failure)
if [[ -f "$SIGNED_OUTPUT_FAIL" ]]; then
    pass "Signed output created even on verification failure"
    
    if command -v jq > /dev/null 2>&1; then
        VERIFICATION_PASSED=$(jq -r '.verification_result.verification_passed' "$SIGNED_OUTPUT_FAIL" 2>/dev/null)
        if [[ "$VERIFICATION_PASSED" == "false" ]]; then
            pass "Verification result correctly shows failure"
        else
            fail "Verification result should show failure"
        fi
        
        HASH_MISMATCHES=$(jq -r '.verification_result.hash_mismatches' "$SIGNED_OUTPUT_FAIL" 2>/dev/null)
        if [[ "$HASH_MISMATCHES" -gt 0 ]]; then
            pass "Hash mismatches correctly reported: $HASH_MISMATCHES"
        else
            fail "Hash mismatches should be reported"
        fi
    fi
fi

# Test 9: Test with missing hash field (negative test)
section "Test 9: Negative Test - Missing Hash Field"

MISSING_HASH_LOG="$TEMP_DIR/missing_hash_log.json"
cat > "$MISSING_HASH_LOG" << 'EOF'
[
  {
    "test_sequence": 1,
    "step": 1,
    "command": "echo \"hello\"",
    "exit_code": 0,
    "output": "hello",
    "timestamp": "2024-01-01T10:00:00.000000+00:00",
    "result_verification_pass": true,
    "output_verification_pass": true
  }
]
EOF

SIGNED_OUTPUT_MISSING="$TEMP_DIR/signed_output_missing.json"

if "$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "$MISSING_HASH_LOG" \
    --save-key "$TEMP_DIR/missing_key.pem" \
    --key-id "test-key-missing" \
    --output "$SIGNED_OUTPUT_MISSING" > /dev/null 2>&1; then
    fail "audit-verifier should fail with missing hash field (got exit 0)"
else
    EXIT_CODE=$?
    pass "audit-verifier correctly failed with missing hash field (exit code: $EXIT_CODE)"
fi

if [[ -f "$SIGNED_OUTPUT_MISSING" ]]; then
    if command -v jq > /dev/null 2>&1; then
        MISSING_HASH_COUNT=$(jq -r '.verification_result.missing_hash_fields' "$SIGNED_OUTPUT_MISSING" 2>/dev/null)
        if [[ "$MISSING_HASH_COUNT" -gt 0 ]]; then
            pass "Missing hash fields correctly reported: $MISSING_HASH_COUNT"
        else
            fail "Missing hash fields should be reported"
        fi
    fi
fi

# Test 10: Error handling - missing YAML file
section "Test 10: Error Handling - Missing YAML File"

if "$AUDIT_VERIFIER_BIN" \
    --yaml "/nonexistent/test.yaml" \
    --log "$EXECUTION_LOG" \
    --save-key "$TEMP_DIR/error_key.pem" > /dev/null 2>&1; then
    fail "Should fail with nonexistent YAML file"
else
    pass "Correctly failed with nonexistent YAML file"
fi

# Test 11: Error handling - missing log file
section "Test 11: Error Handling - Missing Log File"

if "$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "/nonexistent/log.json" \
    --save-key "$TEMP_DIR/error_key2.pem" > /dev/null 2>&1; then
    fail "Should fail with nonexistent log file"
else
    pass "Correctly failed with nonexistent log file"
fi

# Test 12: Error handling - invalid JSON log
section "Test 12: Error Handling - Invalid JSON Log"

INVALID_LOG="$TEMP_DIR/invalid.json"
echo "not valid json" > "$INVALID_LOG"

if "$AUDIT_VERIFIER_BIN" \
    --yaml "$TEST_YAML" \
    --log "$INVALID_LOG" \
    --save-key "$TEMP_DIR/error_key3.pem" > /dev/null 2>&1; then
    fail "Should fail with invalid JSON log"
else
    pass "Correctly failed with invalid JSON log"
fi

# Test 13: Verify tampered signature (negative test)
section "Test 13: Negative Test - Tampered Signature"

if [[ -f "$SIGNED_OUTPUT_2" ]] && command -v jq > /dev/null 2>&1; then
    # Create a tampered version by modifying the signature
    TAMPERED_OUTPUT="$TEMP_DIR/tampered_output.json"
    jq '.signature = "0000000000000000000000000000000000000000000000000000000000000000"' "$SIGNED_OUTPUT_2" > "$TAMPERED_OUTPUT"
    
    if "$VERIFY_SIGNATURE_BIN" --input "$TAMPERED_OUTPUT" > /dev/null 2>&1; then
        fail "Should fail with tampered signature"
    else
        pass "Correctly failed with tampered signature"
    fi
else
    info "Skipping tampered signature test (jq not available or file missing)"
fi

# Test 14: Verify tampered data (negative test)
section "Test 14: Negative Test - Tampered Data"

if [[ -f "$SIGNED_OUTPUT_2" ]] && command -v jq > /dev/null 2>&1; then
    # Create a tampered version by modifying the verification result
    TAMPERED_DATA="$TEMP_DIR/tampered_data.json"
    jq '.verification_result.total_entries = 999' "$SIGNED_OUTPUT_2" > "$TAMPERED_DATA"
    
    if "$VERIFY_SIGNATURE_BIN" --input "$TAMPERED_DATA" > /dev/null 2>&1; then
        fail "Should fail with tampered data"
    else
        pass "Correctly failed with tampered data"
    fi
else
    info "Skipping tampered data test (jq not available or file missing)"
fi

# Summary
section "Test Summary"
echo ""
echo "==========================================="
echo "All audit-verifier integration tests completed"
echo "==========================================="
echo ""

pass "All tests passed!"
finish_run
exit 0
