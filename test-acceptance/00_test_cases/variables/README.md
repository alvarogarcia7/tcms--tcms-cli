# Variable Capture Examples

This directory contains example test cases demonstrating variable capture functionality.

## Files

### 1.yaml
A test case demonstrating both command-based and regex-based variable captures with general verification conditions.

**Features:**
- Command-based variable capture (using `command` field in `capture_vars`)
- Regex-based variable capture (using `capture` field in `capture_vars`)
- General verification conditions that use captured variables
- Multiple capture variables in a single step
- Mix of both capture methods in one step

**Steps:**
1. **Step 1**: Creates a file and captures byte count using command-based capture
   - Captures `output_len` via `wc -c` command
   - Verifies the captured value is numeric and >= 5

2. **Step 2**: Echoes JSON and captures fields using regex patterns
   - Captures `token` from JSON using regex pattern `"token":"([^"]+)"`
   - Captures `user_id` from JSON using regex pattern `"user_id":([0-9]+)`
   - Verifies token format and user_id value

3. **Step 3**: Demonstrates mixing command-based and regex-based captures
   - Captures `transaction_id`, `amount`, `status` using regex patterns
   - Captures `line_count` using command `wc -l`
   - Verifies all captured values with multiple conditions

### 1_test.sh
The generated bash script from `1.yaml`. This script:
- Captures variables using both methods
- Stores captured variables in `STEP_VAR_*` bash variables
- Executes general verification conditions
- Creates `.actual.log` files for each step
- Generates JSON execution log

### TC_VAR_CAPTURE_002.yaml
A comprehensive test case with extensive examples of:
- Command-based captures with various Unix tools (wc, grep, awk, jq)
- Regex-based captures with complex patterns
- Mixed capture methods in single steps
- Advanced bash pattern matching in verification conditions
- Arithmetic comparisons in general verifications

## Variable Capture Methods

### Command-Based Capture
```yaml
capture_vars:
  - name: output_len
    command: "cat /tmp/hello.txt | wc -c"
```
Executes the command and stores the result in the named variable.

### Regex-Based Capture
```yaml
capture_vars:
  - name: token
    capture: '"token":"([^"]+)"'
```
Extracts values from command output using regex patterns (first capture group).

### Mixed Capture
```yaml
capture_vars:
  - name: status
    capture: 'Status: ([A-Z]+)'
  - name: line_count
    command: "wc -l /tmp/file.txt | awk '{print $1}'"
```
Both methods can be used in the same step.

## General Verification Conditions

General verifications allow testing captured variables:

```yaml
verification:
  result: "[[ $EXIT_CODE -eq 0 ]]"
  output: "true"
  general:
    - name: verify token format
      condition: "[[ $token =~ ^[a-zA-Z0-9]+$ ]]"
    - name: verify amount range
      condition: "[[ $amount -ge 1000 && $amount -le 2000 ]]"
```

All general verification conditions must pass for the step to succeed.

## Usage

Generate bash script from YAML:
```bash
./target/release/test-executor generate testcases/examples/variables/1.yaml -o /tmp/test.sh
```

Execute test case:
```bash
./target/release/test-executor execute testcases/examples/variables/1.yaml
```

Validate YAML against schema:
```bash
./target/release/validate-yaml --schema schemas/schema.json testcases/examples/variables/1.yaml
```
