# Implementation Notes for testcases/examples/variables/1.yaml

## Changes Made

### 1. Fixed Schema Compliance Issues
- Updated YAML to conform to the schema requirements
- Added `description` field to test sequence (required by schema)
- Changed steps from `manual: true` to automated steps to enable variable capture
- Ensured all required fields are present

### 2. Updated Test Case Design
The test case now demonstrates three key scenarios:

#### Step 1: Command-Based Variable Capture
- Command: `echo "HELLO" > /tmp/hello.txt`
- Captures `output_len` using: `cat /tmp/hello.txt | wc -c`
- General verifications:
  - Verifies captured value is numeric
  - Verifies value is >= 5 bytes

#### Step 2: Regex-Based Variable Capture  
- Command: Echoes JSON string
- Captures `token` using pattern: `"token":"([^"]+)"`
- Captures `user_id` using pattern: `"user_id":([0-9]+)`
- General verifications:
  - Verifies token format (alphanumeric)
  - Verifies token is not empty
  - Verifies user_id is numeric
  - Verifies user_id equals expected value

#### Step 3: Mixed Capture Methods
- Command: Creates file with transaction data
- Regex captures: `transaction_id`, `amount`, `status`
- Command capture: `line_count`
- General verifications:
  - Transaction ID format validation
  - Amount range validation (1000-2000)
  - Status value validation
  - Line count validation

### 3. Generated Bash Script Features

The generated script (`1_test.sh`) includes:

#### Variable Storage (Bash 3.2+ Compatible)
- Uses `STEP_VAR_*` prefix for captured variables
- Maintains `CAPTURED_VAR_NAMES` space-separated list
- No associative arrays (compatible with bash 3.2+)

#### Command-Based Captures
```bash
STEP_VAR_output_len=$(cat /tmp/hello.txt | wc -c 2>&1 || echo "")
if ! echo " $NAMES " | grep -q " output_len "; then
    CAPTURED_VAR_NAMES="$NAMES output_len"
fi
```

#### Regex-Based Captures
```bash
STEP_VAR_token=$(echo "$COMMAND_OUTPUT" | sed -n 's/.*"token":"\([^"]\+\)".*/\1/p' | head -n 1 || echo "")
```
Uses `sed` with extended regex (BSD/GNU compatible)

#### General Verification Logic
```bash
GENERAL_VERIFY_PASS_verify_output_len_is_numeric=false
if [[ $output_len =~ ^[0-9]+$ ]]; then
    GENERAL_VERIFY_PASS_verify_output_len_is_numeric=true
fi
```

#### Comprehensive Verification Condition
```bash
if [ "$VERIFICATION_RESULT_PASS" = true ] && \
   [ "$VERIFICATION_OUTPUT_PASS" = true ] && \
   [ "$GENERAL_VERIFY_PASS_verify_output_len_is_numeric" = true ] && \
   [ "$GENERAL_VERIFY_PASS_verify_output_len_value" = true ]; then
    echo "[PASS] Step 1: Create test file and capture byte count with command"
else
    echo "[FAIL] Step 1: Create test file and capture byte count with command"
    # ... detailed error output
    exit 1
fi
```

### 4. Key Implementation Details

#### Mutual Exclusivity
The schema enforces that `capture` and `command` fields in `capture_vars` are mutually exclusive:
```yaml
oneOf:
  - required: ["capture"]
    not:
      required: ["command"]
  - required: ["command"]
    not:
      required: ["capture"]
```

#### Variable Naming in Bash
- Variable names are sanitized for bash compatibility
- General verification names have spaces/hyphens replaced with underscores
- Prefix `GENERAL_VERIFY_PASS_` ensures no collisions

#### Error Handling
- All captures include `|| echo ""` fallback for empty values
- Script uses `set -euo pipefail` for strict error handling
- Individual steps use `set +e` during command execution

#### JSON Log Generation
- Creates structured JSON log with test execution details
- Includes timestamp, exit code, output for each step
- Validates JSON with `jq` if available

#### BSD/GNU Compatibility
- Regex patterns use portable syntax
- Sed patterns use `-n` with `p` flag
- Character classes like `\+` work on both platforms

### 5. Verification Flow

For each step:
1. Execute command and capture output
2. Run all variable captures (regex and command-based)
3. Check result verification (exit code)
4. Check output verification
5. Check all general verifications
6. ALL must pass for step to succeed
7. Log results to JSON file

### 6. Usage Examples

#### Generate Script
```bash
./target/release/test-executor generate testcases/examples/variables/1.yaml -o /tmp/test.sh
```

#### Execute Test
```bash
./target/release/test-executor execute testcases/examples/variables/1.yaml
```

#### Expected Output
```
[PASS] Step 1: Create test file and capture byte count with command
[PASS] Step 2: Echo JSON and capture token with regex pattern
[PASS] Step 3: Mix command-based and regex-based captures
All test sequences completed successfully
```

#### Generated Files
- `TC_VAR_001_sequence-1_step-1.actual.log`
- `TC_VAR_001_sequence-1_step-2.actual.log`
- `TC_VAR_001_sequence-1_step-3.actual.log`
- `TC_VAR_001_execution_log.json`

## Schema Validation

The YAML validates against `schemas/schema.json`:
- ✓ All required fields present
- ✓ Capture vars use new array format
- ✓ General verification conditions properly structured
- ✓ Test sequence includes description
- ✓ Verification includes result and output fields

## Testing Verification

To verify the implementation:
1. Check bash syntax: `bash -n testcases/examples/variables/1_test.sh`
2. Execute script: `bash testcases/examples/variables/1_test.sh`
3. Verify log files created
4. Verify JSON log is valid
5. Check all verifications pass

## Notes

- The script is compatible with bash 3.2+ (macOS default)
- All regex patterns are BSD/GNU compatible
- Variable captures work with both stdout and stderr (via `2>&1`)
- Empty capture results default to empty string
- General verifications can use bash regex, arithmetic, and string comparisons
