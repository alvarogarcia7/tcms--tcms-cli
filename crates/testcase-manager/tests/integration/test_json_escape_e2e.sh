#!/bin/bash
#
# End-to-end integration test for json-escape feature
#
# This test validates:
# 1. Building json-escape binary
# 2. Generating test scripts with RustBinary/ShellFallback/Auto modes
# 3. Executing scripts with commands containing special characters
# 4. Validating JSON output with jq
# 5. Testing with json-escape binary removed from PATH
# 6. Verifying Auto mode falls back correctly
#
# Usage: ./tests/integration/test_json_escape_e2e.sh [--no-remove]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source logger library
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/shellcheck-helper.sh" || true

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

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

section "json-escape End-to-End Integration Test"

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
setup_cleanup "$TEMP_DIR"
if [[ $REMOVE_TEMP -eq 0 ]]; then
    disable_cleanup
    info "Temporary files will not be removed: $TEMP_DIR"
fi

info "Using temporary directory: $TEMP_DIR"

# ============================================================================
# Test 1: Build json-escape binary
# ============================================================================
section "Test 1: Build json-escape binary"

log_info "Building json-escape binary..."
if cargo build -p json-escape > "$TEMP_DIR/build.log" 2>&1; then
    pass "json-escape binary built successfully"
    ((++TESTS_PASSED))
else
    fail "Failed to build json-escape binary"
    cat "$TEMP_DIR/build.log"
    ((++TESTS_FAILED))
    exit 1
fi

# Source shared libraries for finding binaries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1

# Find json-escape binary using workspace-aware search
cd "$PROJECT_ROOT"
JSON_ESCAPE_BIN=$(find_binary "json-escape")
if [[ -z "$JSON_ESCAPE_BIN" ]]; then
    fail "json-escape binary not found after build"
    ((++TESTS_FAILED))
    exit 1
fi
pass "json-escape binary exists at $JSON_ESCAPE_BIN"
((++TESTS_PASSED))

# ============================================================================
# Test 2: Test json-escape with special characters
# ============================================================================
section "Test 2: Test json-escape with special characters"

# Test basic escaping
log_info "Testing basic escaping..."
INPUT='Hello "world"'
EXPECTED='Hello \"world\"'
OUTPUT=$(echo -n "$INPUT" | "$JSON_ESCAPE_BIN")
if [[ "$OUTPUT" == "$EXPECTED" ]]; then
    pass "Basic quote escaping works"
    ((++TESTS_PASSED))
else
    fail "Basic quote escaping failed. Expected: $EXPECTED, Got: $OUTPUT"
    ((++TESTS_FAILED))
fi

# Test newline escaping
log_info "Testing newline escaping..."
INPUT=$'Line1\nLine2\nLine3'
OUTPUT=$(printf '%s' "$INPUT" | "$JSON_ESCAPE_BIN")
if [[ "$OUTPUT" == *"\\n"* ]]; then
    pass "Newline escaping works"
    ((++TESTS_PASSED))
else
    fail "Newline escaping failed. Output: $OUTPUT"
    ((++TESTS_FAILED))
fi

# Test backslash escaping
log_info "Testing backslash escaping..."
INPUT='C:\test\path'
OUTPUT=$(echo -n "$INPUT" | "$JSON_ESCAPE_BIN")
if [[ "$OUTPUT" == *"\\\\"* ]]; then
    pass "Backslash escaping works"
    ((++TESTS_PASSED))
else
    fail "Backslash escaping failed. Output: $OUTPUT"
    ((++TESTS_FAILED))
fi

# Test tab escaping
log_info "Testing tab escaping..."
INPUT=$'Col1\tCol2\tCol3'
OUTPUT=$(printf '%s' "$INPUT" | "$JSON_ESCAPE_BIN")
if [[ "$OUTPUT" == *"\\t"* ]]; then
    pass "Tab escaping works"
    ((++TESTS_PASSED))
else
    fail "Tab escaping failed. Output: $OUTPUT"
    ((++TESTS_FAILED))
fi

# ============================================================================
# Test 3: Test json-escape validation mode
# ============================================================================
section "Test 3: Test json-escape validation mode"

log_info "Testing validation mode with valid input..."
INPUT='Simple text'
if echo -n "$INPUT" | "$JSON_ESCAPE_BIN" --test > /dev/null 2>&1; then
    pass "Validation mode accepts valid input"
    ((++TESTS_PASSED))
else
    fail "Validation mode rejected valid input"
    ((++TESTS_FAILED))
fi

log_info "Testing validation mode with special characters..."
INPUT=$'Line1\nLine2\tTabbed\r\nCRLF'
if printf '%s' "$INPUT" | "$JSON_ESCAPE_BIN" --test > /dev/null 2>&1; then
    pass "Validation mode accepts special characters"
    ((++TESTS_PASSED))
else
    fail "Validation mode rejected special characters"
    ((++TESTS_FAILED))
fi

# ============================================================================
# Test 4: Generate and execute test script with RustBinary mode
# ============================================================================
section "Test 4: Generate and execute test script with RustBinary mode"

log_info "Creating test case YAML..."
TEST_YAML="$TEMP_DIR/test_rust_binary.yaml"
cat > "$TEST_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: REQ001
item: 1
tc: 1
id: TEST_RUST_BINARY
description: Test json-escape with RustBinary mode
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  System:
    - Shell is ready
test_sequences:
  - id: 1
    name: Test commands with special characters
    description: Test commands with special characters
    initial_conditions:
      System:
        - Commands can be executed
    steps:
      - step: 1
        description: Echo with quotes
        command: echo 'Hello "world" from test'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 2
        description: Printf with newlines
        command: printf 'Line1\nLine2\nLine3'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 3
        description: Echo with backslashes
        command: |-
          echo 'Path: C:\test\file'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 4
        description: Printf with tabs
        command: printf 'Col1\tCol2\tCol3'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

log_info "Creating config file for RustBinary mode..."
CONFIG_FILE="$TEMP_DIR/config_rust_binary.toml"
cat > "$CONFIG_FILE" << EOF
[script_generation.json_escaping]
method = "rust_binary"
enabled = true
EOF

log_info "Generating test script with RustBinary mode..."
if cargo run -p test-executor -- \
    --config "$CONFIG_FILE" \
    generate "$TEST_YAML" -o "$TEMP_DIR/TEST_RUST_BINARY_test.sh" > "$TEMP_DIR/generate_rust_binary.log" 2>&1; then
    pass "Test script generated with RustBinary mode"
    ((++TESTS_PASSED))
else
    fail "Failed to generate test script with RustBinary mode"
    cat "$TEMP_DIR/generate_rust_binary.log"
    ((++TESTS_FAILED))
fi

SCRIPT_PATH="$TEMP_DIR/TEST_RUST_BINARY_test.sh"
if [[ ! -f "$SCRIPT_PATH" ]]; then
    fail "Generated script not found at $SCRIPT_PATH"
    ((++TESTS_FAILED))
else
    pass "Generated script exists"
    ((++TESTS_PASSED))
    
    # Verify script contains json-escape usage
    if grep -q "json-escape" "$SCRIPT_PATH"; then
        pass "Script uses json-escape binary"
        ((++TESTS_PASSED))
    else
        fail "Script does not use json-escape binary"
        ((++TESTS_FAILED))
    fi
    
    # Make script executable and run it
    chmod +x "$SCRIPT_PATH"
    log_info "Executing test script with RustBinary mode..."
    # rust_binary mode invokes json-escape by name, so its directory must be on PATH
    JSON_ESCAPE_DIR="$(cd "$(dirname "$JSON_ESCAPE_BIN")" && pwd)"
    if (cd "$TEMP_DIR" && PATH="$JSON_ESCAPE_DIR:$PATH" bash "$SCRIPT_PATH") > "$TEMP_DIR/execute_rust_binary.log" 2>&1; then
        pass "Test script executed successfully"
        ((++TESTS_PASSED))
    else
        fail "Test script execution failed"
        cat "$TEMP_DIR/execute_rust_binary.log"
        ((++TESTS_FAILED))
    fi
    
    # Verify JSON log was created
    JSON_LOG="$TEMP_DIR/TEST_RUST_BINARY_execution_log.json"
    if [[ ! -f "$JSON_LOG" ]]; then
        fail "JSON log not created at $JSON_LOG"
        ((++TESTS_FAILED))
    else
        pass "JSON log created"
        ((++TESTS_PASSED))
        
        # Validate JSON with jq if available
        if command -v jq >/dev/null 2>&1; then
            if jq empty "$JSON_LOG" >/dev/null 2>&1; then
                pass "JSON log is valid (jq validation)"
                ((++TESTS_PASSED))
            else
                fail "JSON log is invalid"
                cat "$JSON_LOG"
                ((++TESTS_FAILED))
            fi
            
            # Verify JSON structure
            NUM_ENTRIES=$(jq 'length' "$JSON_LOG")
            if [[ "$NUM_ENTRIES" -eq 4 ]]; then
                pass "JSON log contains 4 entries (4 non-manual steps)"
                ((++TESTS_PASSED))
            else
                fail "JSON log should contain 4 entries, got $NUM_ENTRIES"
                ((++TESTS_FAILED))
            fi
            
            # Verify special characters are properly escaped in JSON
            FIRST_OUTPUT=$(jq -r '.[0].output' "$JSON_LOG")
            if [[ "$FIRST_OUTPUT" == *"world"* ]]; then
                pass "First step output contains expected text"
                ((++TESTS_PASSED))
            else
                fail "First step output missing expected text: $FIRST_OUTPUT"
                ((++TESTS_FAILED))
            fi
        else
            info "jq not available, skipping JSON validation"
        fi
    fi
fi

# ============================================================================
# Test 5: Generate and execute test script with ShellFallback mode
# ============================================================================
section "Test 5: Generate and execute test script with ShellFallback mode"

log_info "Creating config file for ShellFallback mode..."
CONFIG_FILE_FALLBACK="$TEMP_DIR/config_shell_fallback.toml"
cat > "$CONFIG_FILE_FALLBACK" << EOF
[script_generation.json_escaping]
method = "shell_fallback"
enabled = true
EOF

log_info "Generating test script with ShellFallback mode..."
if cargo run -p test-executor -- \
    --config "$CONFIG_FILE_FALLBACK" \
    generate "$TEST_YAML" -o "$TEMP_DIR/TEST_RUST_BINARY_test.sh" > "$TEMP_DIR/generate_shell_fallback.log" 2>&1; then
    pass "Test script generated with ShellFallback mode"
    ((++TESTS_PASSED))
else
    fail "Failed to generate test script with ShellFallback mode"
    cat "$TEMP_DIR/generate_shell_fallback.log"
    ((++TESTS_FAILED))
fi

if [[ -f "$SCRIPT_PATH" ]]; then
    # Verify script uses shell fallback (sed/awk)
    if grep -q "sed 's/" "$SCRIPT_PATH" && grep -q "awk" "$SCRIPT_PATH"; then
        pass "Script uses shell fallback (sed/awk)"
        ((++TESTS_PASSED))
    else
        fail "Script does not use shell fallback"
        ((++TESTS_FAILED))
    fi
    
    # Verify script does NOT check for json-escape binary
    if ! grep -q "if command -v json-escape" "$SCRIPT_PATH"; then
        pass "Script does not check for json-escape in ShellFallback mode"
        ((++TESTS_PASSED))
    else
        fail "Script should not check for json-escape in ShellFallback mode"
        ((++TESTS_FAILED))
    fi
    
    # Execute script
    log_info "Executing test script with ShellFallback mode..."
    if (cd "$TEMP_DIR" && bash "$SCRIPT_PATH") > "$TEMP_DIR/execute_shell_fallback.log" 2>&1; then
        pass "Test script with ShellFallback executed successfully"
        ((++TESTS_PASSED))
    else
        fail "Test script with ShellFallback execution failed"
        cat "$TEMP_DIR/execute_shell_fallback.log"
        ((++TESTS_FAILED))
    fi
    
    # Verify JSON log
    if [[ -f "$JSON_LOG" ]]; then
        if command -v jq >/dev/null 2>&1; then
            if jq empty "$JSON_LOG" >/dev/null 2>&1; then
                pass "JSON log from ShellFallback is valid"
                ((++TESTS_PASSED))
            else
                fail "JSON log from ShellFallback is invalid"
                cat "$JSON_LOG"
                ((++TESTS_FAILED))
            fi
        fi
    fi
fi

# ============================================================================
# Test 6: Generate and execute test script with Auto mode (binary available)
# ============================================================================
section "Test 6: Generate and execute test script with Auto mode (binary available)"

log_info "Creating config file for Auto mode..."
CONFIG_FILE_AUTO="$TEMP_DIR/config_auto.toml"
cat > "$CONFIG_FILE_AUTO" << EOF
[script_generation.json_escaping]
method = "auto"
enabled = true
EOF

log_info "Generating test script with Auto mode..."
if cargo run -p test-executor -- \
    --config "$CONFIG_FILE_AUTO" \
    generate "$TEST_YAML" -o "$TEMP_DIR/TEST_RUST_BINARY_test.sh" > "$TEMP_DIR/generate_auto.log" 2>&1; then
    pass "Test script generated with Auto mode"
    ((++TESTS_PASSED))
else
    fail "Failed to generate test script with Auto mode"
    cat "$TEMP_DIR/generate_auto.log"
    ((++TESTS_FAILED))
fi

if [[ -f "$SCRIPT_PATH" ]]; then
    # Verify script has both json-escape check and fallback
    if grep -q "if command -v json-escape" "$SCRIPT_PATH"; then
        pass "Script checks for json-escape availability"
        ((++TESTS_PASSED))
    else
        fail "Script should check for json-escape in Auto mode"
        ((++TESTS_FAILED))
    fi
    
    if grep -q "sed 's/" "$SCRIPT_PATH" && grep -q "awk" "$SCRIPT_PATH"; then
        pass "Script contains shell fallback for Auto mode"
        ((++TESTS_PASSED))
    else
        fail "Script should contain shell fallback in Auto mode"
        ((++TESTS_FAILED))
    fi
    
    # Execute script with json-escape in PATH
    log_info "Executing test script with Auto mode (binary in PATH)..."
    if (cd "$TEMP_DIR" && bash "$SCRIPT_PATH") > "$TEMP_DIR/execute_auto_with_binary.log" 2>&1; then
        pass "Test script with Auto mode executed successfully (binary available)"
        ((++TESTS_PASSED))
    else
        fail "Test script with Auto mode execution failed"
        cat "$TEMP_DIR/execute_auto_with_binary.log"
        ((++TESTS_FAILED))
    fi
    
    # Verify JSON log
    if [[ -f "$JSON_LOG" ]]; then
        if command -v jq >/dev/null 2>&1; then
            if jq empty "$JSON_LOG" >/dev/null 2>&1; then
                pass "JSON log from Auto mode (binary available) is valid"
                ((++TESTS_PASSED))
            else
                fail "JSON log from Auto mode is invalid"
                cat "$JSON_LOG"
                ((++TESTS_FAILED))
            fi
        fi
    fi
fi

# ============================================================================
# Test 7: Test Auto mode fallback when binary is not in PATH
# ============================================================================
section "Test 7: Test Auto mode fallback when binary is not in PATH"

log_info "Executing test script with json-escape removed from PATH..."

# Remove json-escape from PATH by creating a restricted environment
CLEAN_PATH=$(echo "$PATH" | tr ':' '\n' | grep -v "target/debug" | tr '\n' ':' | sed 's/:$//')

# Execute script without json-escape in PATH
if (cd "$TEMP_DIR" && env PATH="$CLEAN_PATH" bash "$SCRIPT_PATH") > "$TEMP_DIR/execute_auto_without_binary.log" 2>&1; then
    pass "Test script with Auto mode executed successfully (binary NOT available)"
    ((++TESTS_PASSED))
else
    fail "Test script with Auto mode failed when binary not available"
    cat "$TEMP_DIR/execute_auto_without_binary.log"
    ((++TESTS_FAILED))
fi

# Verify JSON log is still created and valid
if [[ -f "$JSON_LOG" ]]; then
    if command -v jq >/dev/null 2>&1; then
        if jq empty "$JSON_LOG" >/dev/null 2>&1; then
            pass "JSON log from Auto mode (fallback) is valid"
            ((++TESTS_PASSED))
        else
            fail "JSON log from Auto mode (fallback) is invalid"
            cat "$JSON_LOG"
            ((++TESTS_FAILED))
        fi
    fi
else
    fail "JSON log not created when using Auto mode fallback"
    ((++TESTS_FAILED))
fi

# ============================================================================
# Test 8: Test with complex special characters
# ============================================================================
section "Test 8: Test with complex special characters"

log_info "Creating test case with complex special characters..."
COMPLEX_YAML="$TEMP_DIR/test_complex_chars.yaml"
cat > "$COMPLEX_YAML" << 'EOF'
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: REQ002
item: 1
tc: 1
id: TEST_COMPLEX_CHARS
description: Test with complex special characters
general_initial_conditions:
  System:
    - Ready
initial_conditions:
  System:
    - Shell is ready
test_sequences:
  - id: 1
    name: Complex special character tests
    description: Complex special character tests
    initial_conditions:
      System:
        - Commands can be executed
    steps:
      - step: 1
        description: Mixed special characters
        command: printf 'Line1\nLine2\tTabbed\rReturn "quoted" C:\\path'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 2
        description: Multiple quotes and backslashes
        command: echo 'He said "hello" and she said "world" at C:\test\path'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
      - step: 3
        description: JSON-like output
        command: |-
          echo '{"key": "value", "nested": {"data": "test"}}'
        expected:
          success: true
          result: "0"
          output: "true"
        verification:
          result: "[ $EXIT_CODE -eq 0 ]"
          output: "true"
EOF

log_info "Generating test script for complex characters..."
COMPLEX_SCRIPT="$TEMP_DIR/TEST_COMPLEX_CHARS_test.sh"
if cargo run -p test-executor -- \
    --config "$CONFIG_FILE_AUTO" \
    generate "$COMPLEX_YAML" -o "$TEMP_DIR/TEST_COMPLEX_CHARS_test.sh" > "$TEMP_DIR/generate_complex.log" 2>&1; then
    pass "Test script generated for complex characters"
    ((++TESTS_PASSED))
else
    fail "Failed to generate test script for complex characters"
    cat "$TEMP_DIR/generate_complex.log"
    ((++TESTS_FAILED))
fi

if [[ -f "$COMPLEX_SCRIPT" ]]; then
    chmod +x "$COMPLEX_SCRIPT"
    log_info "Executing test script with complex characters..."
    if (cd "$TEMP_DIR" && bash "$COMPLEX_SCRIPT") > "$TEMP_DIR/execute_complex.log" 2>&1; then
        pass "Test script with complex characters executed successfully"
        ((++TESTS_PASSED))
    else
        fail "Test script with complex characters execution failed"
        cat "$TEMP_DIR/execute_complex.log"
        ((++TESTS_FAILED))
    fi
    
    # Verify JSON log
    COMPLEX_JSON="$TEMP_DIR/TEST_COMPLEX_CHARS_execution_log.json"
    if [[ -f "$COMPLEX_JSON" ]]; then
        pass "JSON log created for complex characters test"
        ((++TESTS_PASSED))
        
        if command -v jq >/dev/null 2>&1; then
            if jq empty "$COMPLEX_JSON" >/dev/null 2>&1; then
                pass "JSON log with complex characters is valid"
                ((++TESTS_PASSED))
                
                # Verify all entries are present
                NUM_ENTRIES=$(jq 'length' "$COMPLEX_JSON")
                if [[ "$NUM_ENTRIES" -eq 3 ]]; then
                    pass "JSON log contains 3 entries"
                    ((++TESTS_PASSED))
                else
                    fail "JSON log should contain 3 entries, got $NUM_ENTRIES"
                    ((++TESTS_FAILED))
                fi
            else
                fail "JSON log with complex characters is invalid"
                cat "$COMPLEX_JSON"
                ((++TESTS_FAILED))
            fi
        fi
    else
        fail "JSON log not created for complex characters test"
        ((++TESTS_FAILED))
    fi
fi

# ============================================================================
# Test 9: Verify json-escape handles empty input
# ============================================================================
section "Test 9: Verify json-escape handles empty input"

log_info "Testing json-escape with empty input..."
OUTPUT=$(echo -n "" | "$JSON_ESCAPE_BIN")
if [[ -z "$OUTPUT" ]]; then
    pass "json-escape handles empty input correctly"
    ((++TESTS_PASSED))
else
    fail "json-escape should return empty string for empty input, got: '$OUTPUT'"
    ((++TESTS_FAILED))
fi

# ============================================================================
# Test 10: Verify shell fallback escaping patterns
# ============================================================================
section "Test 10: Verify shell fallback escaping patterns"

log_info "Testing shell fallback escaping directly..."

# Test backslash escaping
INPUT='C:\test\path'
OUTPUT=$(printf '%s' "$INPUT" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r/\\r/g' | awk '{printf "%s%s", (NR>1?"\\n":""), $0}')
if [[ "$OUTPUT" == 'C:\\test\\path' ]]; then
    pass "Shell fallback escapes backslashes correctly"
    ((++TESTS_PASSED))
else
    fail "Shell fallback backslash escaping failed. Expected: 'C:\\\\test\\\\path', Got: '$OUTPUT'"
    ((++TESTS_FAILED))
fi

# Test quote escaping
INPUT='He said "hello"'
OUTPUT=$(printf '%s' "$INPUT" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r/\\r/g' | awk '{printf "%s%s", (NR>1?"\\n":""), $0}')
if [[ "$OUTPUT" == 'He said \"hello\"' ]]; then
    pass "Shell fallback escapes quotes correctly"
    ((++TESTS_PASSED))
else
    fail "Shell fallback quote escaping failed. Expected: 'He said \\\"hello\\\"', Got: '$OUTPUT'"
    ((++TESTS_FAILED))
fi

# Test newline escaping
INPUT=$'Line1\nLine2\nLine3'
OUTPUT=$(printf '%s' "$INPUT" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r/\\r/g' | awk '{printf "%s%s", (NR>1?"\\n":""), $0}')
if [[ "$OUTPUT" == 'Line1\nLine2\nLine3' ]]; then
    pass "Shell fallback escapes newlines correctly"
    ((++TESTS_PASSED))
else
    fail "Shell fallback newline escaping failed. Got: '$OUTPUT'"
    ((++TESTS_FAILED))
fi

# ============================================================================
# Summary
# ============================================================================
section "Test Summary"

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
echo ""
echo "Total tests: $TOTAL_TESTS"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo ""

if [[ $TESTS_FAILED -gt 0 ]]; then
    fail "Some tests failed"
    exit 1
else
    pass "All tests passed!"
    exit 0
fi
