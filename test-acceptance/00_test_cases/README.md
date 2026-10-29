# Test Acceptance Test Cases

This directory contains comprehensive test cases for acceptance testing of the YAML-based test harness. The test cases are organized into subdirectories by category.

## Directory Structure

```
test-acceptance/test_cases/
├── success/           # Success scenario test cases
├── failure/           # Failure scenario test cases
├── hooks/             # Hook lifecycle test cases
└── README.md         # This file
```

## Success Scenario Test Cases

The `success/` directory contains 13 comprehensive test cases demonstrating all major features of the test harness:

### 1. TC_SUCCESS_SIMPLE_001 - Simple Single-Sequence Test
- **Purpose**: Basic command execution with 3 steps
- **Features**: Single sequence, simple commands, basic output validation
- **Steps**: 
  - Print hello world message
  - Display current date with format validation
  - List temporary directory

### 2. TC_SUCCESS_MULTI_SEQ_001 - Multi-Sequence Test
- **Purpose**: Test organization with multiple sequences
- **Features**: 3 sequences with 2-4 steps each
- **Sequences**:
  1. File Creation (3 steps) - Create test files
  2. File Content Processing (4 steps) - Read and process files
  3. File Cleanup (4 steps) - Remove files and verify

### 3. TC_SUCCESS_VAR_CAPTURE_001 - Variable Capture and Usage
- **Purpose**: Capture variables from output and reuse them
- **Features**: Regex capture, variable reuse in subsequent steps
- **Steps**:
  - Generate and capture timestamp
  - Create file with captured timestamp
  - Read back and verify using captured value
  - Cleanup using captured variable

### 4. TC_SUCCESS_REGEX_VALIDATION_001 - Regex Pattern Validation
- **Purpose**: Complex pattern matching in verification
- **Features**: Multiple regex patterns for validation
- **Patterns Validated**:
  - UUID format
  - Email address format
  - IP address format
  - URL format
  - Semantic version format

### 5. TC_SUCCESS_ENV_VARS_001 - Environment Variable Usage
- **Purpose**: Use environment variables in commands
- **Features**: Hydration variables, required/optional vars
- **Variables**:
  - TEST_USER (required)
  - TEST_HOME (required)
  - TEST_PORT (optional)
- **Steps**: Create directories, config files using env vars

### 6. TC_SUCCESS_CMD_CHAIN_001 - Command Chaining with &&
- **Purpose**: Sequential command execution with dependencies
- **Features**: Multiple commands chained with && operator
- **Examples**:
  - Directory creation and file operations
  - File operations with multiple steps
  - Grep and awk pipelines
  - Directory navigation and operations

### 7. TC_SUCCESS_STEP_DEPS_001 - Step Dependencies
- **Purpose**: Steps that depend on variables from previous steps
- **Features**: Complex variable dependencies across steps
- **Flow**:
  - Generate unique ID → capture
  - Create directory using ID
  - Create files in directory
  - Read files and capture content
  - Calculate sizes
  - Cleanup using all captured variables

### 8. TC_SUCCESS_LONG_RUNNING_001 - Long-Running Commands
- **Purpose**: Handle commands that take time to execute
- **Features**: Sleep commands, timing measurements, progress updates
- **Examples**:
  - 2-second delay operations
  - Measure operation duration
  - Progress updates over time
  - Batch processing with timestamps

### 9. TC_SUCCESS_EMPTY_OUTPUT_001 - Empty Output Validation
- **Purpose**: Handle commands with no output
- **Features**: Validation of empty output, file operations without output
- **Steps**:
  - Create files silently (touch)
  - Remove files silently
  - Verify empty directories
  - Read empty files

### 10. TC_SUCCESS_CONDITIONAL_001 - Conditional Verification Logic
- **Purpose**: Complex if/then/else verification
- **Features**: Conditional verification with captured variables
- **Examples**:
  - Score-based pass/fail logic
  - Status checking with conditional actions
  - Multiple variable conditions
  - Resource thresholds with alerts
  - Environment-specific validation

### 11. TC_SUCCESS_COMPLEX_DATA_001 - Complex Data Processing
- **Purpose**: Real-world data extraction and validation
- **Features**: Multi-field captures, complex validations
- **Data Types**:
  - JSON-like API responses (8+ captured fields)
  - Server log entries (7+ captured fields)
  - Performance metrics with calculations

### 12. TC_SUCCESS_FILE_OPS_001 - Advanced File Operations
- **Purpose**: Comprehensive file system operations
- **Features**: File metadata, permissions, content manipulation
- **Sequences**:
  1. File Creation and Metadata - Create files, verify size/permissions
  2. File Content Operations - Append, search, copy files
  3. Cleanup and Verification - Remove files and verify

### 13. TC_SUCCESS_TEXT_PROCESSING_001 - Advanced Text Processing
- **Purpose**: Complex text manipulation with sed, awk, grep
- **Features**: Pattern matching, text transformation, aggregation
- **Sequences**:
  1. Pattern Matching and Extraction - awk field extraction, sed transformation, grep counting
  2. Complex Text Transformations - CSV parsing, log formatting, data aggregation

## Feature Coverage Summary

| Feature | Test Cases |
|---------|-----------|
| Single sequence (3 steps) | TC_SUCCESS_SIMPLE_001 |
| Multi-sequence (3 sequences, 2-4 steps) | TC_SUCCESS_MULTI_SEQ_001 |
| Variable capture (regex) | TC_SUCCESS_VAR_CAPTURE_001, TC_SUCCESS_REGEX_VALIDATION_001 |
| Variable capture (command) | TC_SUCCESS_STEP_DEPS_001, TC_SUCCESS_COMPLEX_DATA_001 |
| Output validation with regex | TC_SUCCESS_REGEX_VALIDATION_001 |
| Environment variable usage | TC_SUCCESS_ENV_VARS_001 |
| Command chaining with && | TC_SUCCESS_CMD_CHAIN_001 |
| Step dependencies | TC_SUCCESS_STEP_DEPS_001 |
| Long-running commands | TC_SUCCESS_LONG_RUNNING_001 |
| Empty output validation | TC_SUCCESS_EMPTY_OUTPUT_001 |
| Complex conditional verification | TC_SUCCESS_CONDITIONAL_001 |
| Complex data processing | TC_SUCCESS_COMPLEX_DATA_001 |
| File operations | TC_SUCCESS_FILE_OPS_001 |
| Text processing | TC_SUCCESS_TEXT_PROCESSING_001 |

## Running Test Cases

To validate the test cases:

```bash
# Validate a single test case
cargo run --bin verifier -- test-acceptance/00_test_cases/success/TC_SUCCESS_SIMPLE_001.yaml

# Validate all test cases
for file in test-acceptance/00_test_cases/success/*.yaml; do
  echo "Validating $file"
  cargo run --bin verifier -- "$file"
done
```

## Test Case Schema

All test cases follow the schema defined in `schemas/test-case.schema.json` and include:

- **requirement**: Feature requirement identifier
- **item**: Item number
- **tc**: Test case number
- **id**: Unique test case identifier
- **description**: Test case description
- **general_initial_conditions**: Top-level general conditions
- **initial_conditions**: Device/system specific conditions
- **hydration_vars**: Optional environment variables (in some test cases)
- **test_sequences**: Array of test sequences with steps

Each step includes:
- **step**: Step number
- **description**: Step description
- **command**: Command to execute
- **capture_vars**: Optional variables to capture (array format)
- **expected**: Expected result and output
- **verification**: Verification expressions (result, output, general)

## Hooks Test Cases

The `hooks/` directory contains 13 comprehensive test cases demonstrating the test execution lifecycle hooks feature:

### 1. TC_HOOKS_001 - Comprehensive Hook Lifecycle
- **Purpose**: Demonstrate all eight hook types with external scripts
- **Features**: script_start, setup_test, before_sequence, after_sequence, before_step, after_step, teardown_test, script_end
- **Hook Scripts**: Uses external shell scripts in `hooks/scripts/` directory
- **Mode**: on_error: fail
- **Sequences**: 3 sequences testing hook integration, variable capture, and error handling

### 2. TC_HOOKS_SIMPLE_001 - Basic Hook Functionality
- **Purpose**: Simple test with only script_start and script_end hooks
- **Features**: Inline hook commands, marker file creation
- **Mode**: on_error: fail
- **Demonstrates**: Basic hook execution at test boundaries

### 3. TC_HOOKS_INLINE_001 - Inline Hook Commands
- **Purpose**: Demonstrate hooks defined as inline bash commands (no external scripts)
- **Features**: All eight hook types as inline YAML commands
- **Mode**: on_error: fail
- **Demonstrates**: Hook context variables (TEST_SEQUENCE_ID, TEST_STEP), marker file creation

### 4. TC_HOOKS_CONTINUE_001 - Continue on Hook Error
- **Purpose**: Test on_error: continue mode where hook failures don't stop execution
- **Features**: Inline hooks with intentional failures
- **Mode**: on_error: continue
- **Demonstrates**: Test continues executing even when before_step hook fails on step 2

### 5. TC_HOOKS_MISSING_001 - Missing Hook Script Error
- **Purpose**: Verify error handling when hook script file doesn't exist
- **Features**: References non-existent script file
- **Mode**: on_error: fail
- **Expected**: Test fails immediately due to missing script_start hook

### 6. TEST_HOOK_SCRIPT_START_001 - Script Start Hook Error
- **Purpose**: Hook error scenario where script_start hook exits with error code
- **Features**: External error script
- **Mode**: on_error: fail
- **Expected**: Test terminates immediately, no steps execute

### 7. TEST_HOOK_SETUP_TEST_001 - Setup Test Hook Error
- **Purpose**: Hook error scenario where setup_test hook references non-existent script
- **Features**: Missing script file
- **Mode**: on_error: fail
- **Expected**: Test fails during setup phase

### 8. TEST_HOOK_BEFORE_SEQ_001 - Before Sequence Hook Error
- **Purpose**: Hook error scenario where before_sequence hook fails on sequence 1
- **Features**: External error script that fails on sequence 1
- **Mode**: on_error: fail
- **Expected**: Sequence 1 steps execute, but subsequent sequences don't

### 9. TEST_HOOK_AFTER_SEQ_001 - After Sequence Hook Error
- **Purpose**: Hook error scenario where after_sequence hook fails after sequence 1
- **Features**: External error script that fails after sequence 1
- **Mode**: on_error: fail
- **Expected**: Sequence 1 completes, but sequence 2 and 3 don't execute

### 10. TEST_HOOK_BEFORE_STEP_001 - Before Step Hook Error
- **Purpose**: Hook error scenario where before_step hook fails on step 3
- **Features**: External error script that fails before step 3
- **Mode**: on_error: fail
- **Expected**: Steps 1 and 2 execute, step 3 and beyond don't execute

### 11. TEST_HOOK_AFTER_STEP_001 - After Step Hook Error
- **Purpose**: Hook error scenario where after_step hook fails after step 2
- **Features**: External error script that fails after step 2
- **Mode**: on_error: fail
- **Expected**: Steps 1 and 2 execute, step 3 and beyond don't execute

### 12. TEST_HOOK_TEARDOWN_001 - Teardown Test Hook Error
- **Purpose**: Hook error scenario where teardown_test hook fails
- **Features**: External error script that fails during teardown
- **Mode**: on_error: fail
- **Expected**: All sequences complete, but teardown hook failure is reported

### 13. TEST_HOOK_SCRIPT_END_001 - Script End Hook Error
- **Purpose**: Hook error scenario where script_end hook fails
- **Features**: External error script that fails at script end
- **Mode**: on_error: fail
- **Expected**: All test execution completes, but script_end hook failure is reported

### Hook Scripts

All hook scripts are located in `hooks/scripts/` directory:

**Success Hook Scripts** (used by TC_HOOKS_001):
- `script_start.sh` - Logs test start time, creates marker file
- `setup_test.sh` - Creates test workspace and sequence directories
- `before_sequence.sh` - Logs sequence start, creates sequence log
- `after_sequence.sh` - Cleans up sequence resources
- `before_step.sh` - Logs step details and variables
- `after_step.sh` - Validates step output, saves results
- `teardown_test.sh` - Removes temporary directories and files
- `script_end.sh` - Logs test completion time and duration

**Error Hook Scripts** (used by TEST_HOOK_* test cases):
- `hook_script_start_error.sh` - Always fails (exit 1)
- `hook_before_sequence_error.sh` - Fails on sequence 1
- `hook_after_sequence_error.sh` - Fails after sequence 1
- `hook_before_step_error.sh` - Fails on step 3
- `hook_after_step_error.sh` - Fails after step 2
- `hook_teardown_error.sh` - Always fails (exit 1)
- `hook_script_end_error.sh` - Always fails (exit 1)

## Manual Test Cases

The `manual/` directory contains 9 test cases that require manual intervention:

All manual test cases use the `manual: true` flag on steps that require human interaction. When running the acceptance suite:
- Manual tests are **automatically skipped** by default
- Use `--include-manual` flag to include manual tests in execution
- Manual tests are tracked separately in the execution report

Manual test cases include:
- `TC_MANUAL_ALL_001.yaml` - All steps are manual
- `TC_MANUAL_CAPTURE_001.yaml` - Manual variable capture workflows
- `TC_MANUAL_CONDITIONAL_001.yaml` - Manual conditional verification
- `TC_MANUAL_FILE_INSPECT_001.yaml` - Manual file inspection
- `TC_MANUAL_MIXED_001.yaml` - Mixed automated and manual steps
- `TC_MANUAL_MULTI_SEQ_001.yaml` - Multiple sequences with manual steps
- `TC_MANUAL_OUTPUT_VERIFY_001.yaml` - Manual output verification
- `TC_MANUAL_PREREQ_001.yaml` - Manual prerequisite checks
- `TC_MANUAL_RESULT_VERIFY_001.yaml` - Manual result verification

## Bash Commands Test Cases

The `bash_commands/` directory contains 13 test cases focused on bash syntax validation:

These test cases verify that the test harness correctly generates bash scripts for various bash language constructs:
- `TC_BASH_SIMPLE_001.yaml` - Simple commands (echo, pwd, whoami, date)
- `TC_BASH_STRING_OPS_001.yaml` - String manipulation operations
- `TC_BASH_CONDITIONALS_001.yaml` - Conditional expressions (if/then/else)
- `TC_BASH_LOOPS_001.yaml` - Loop constructs (for, while)
- `TC_BASH_ARRAYS_001.yaml` - Array operations
- `TC_BASH_MATH_OPS_001.yaml` - Arithmetic operations
- `TC_BASH_FILE_OPS_001.yaml` - File operations and tests
- `TC_BASH_REDIRECTION_001.yaml` - I/O redirection and pipes
- `TC_BASH_PROCESS_OPS_001.yaml` - Process control operations
- `TC_BASH_ENV_VARS_001.yaml` - Environment variable handling
- `TC_BASH_INTERMEDIATE_001.yaml` - Intermediate complexity commands
- `TC_BASH_COMPLEX_001.yaml` - Complex multi-command workflows
- `TC_BASH_VERIFICATION_001.yaml` - Comprehensive verification syntax

**Purpose**: Ensure bash script generation handles all common bash constructs correctly and produces syntactically valid bash 3.2+ compatible scripts.

## Variables Test Cases

The `variables/` directory contains 5 test cases demonstrating variable capture and usage:

These test cases verify the full lifecycle of variable operations:
- **Command-based captures**: Extract values by running commands (`command` field)
- **Regex-based captures**: Extract values from output using patterns (`capture` field)
- **Variable validation**: Use captured variables in verification conditions
- **Variable substitution**: Use variables in subsequent commands

Test cases include:
- `TC_VAR_DEMO_001.yaml` - Basic variable capture, validation, and usage demonstration
- `TC_VAR_CAPTURE_002.yaml` - Comprehensive variable capture scenarios (command + regex)
- `TC_VAR_DISPLAY_001.yaml` - Variable display and formatting workflows
- `1.yaml` - Additional variable workflow example
- `2.yaml` - Additional variable workflow example

**Features Tested**:
- Command-based captures (wc, grep, awk, jq)
- Regex pattern extraction (tokens, IDs, metrics)
- Variable validation in general verification conditions
- Arithmetic operations with captured variables
- Complex pattern matching with multiple capture groups
- Mixed command and regex captures in same step

## Running the Acceptance Suite

To run test cases from specific categories:

```bash
# Run all tests (manual tests skipped by default)
./test-acceptance/run_acceptance_suite.sh --verbose

# Include manual tests in execution
./test-acceptance/run_acceptance_suite.sh --verbose --include-manual

# Skip specific stages
./test-acceptance/run_acceptance_suite.sh --verbose --skip-documentation
```

The acceptance suite will:
1. **Validate** all YAML files against schema
2. **Generate** bash scripts for all test cases
3. **Execute** automated tests (manual tests skipped unless `--include-manual`)
4. **Verify** execution logs against expected outcomes
5. **Validate** container YAMLs against schema
6. **Generate** documentation reports (AsciiDoc, Markdown)
7. **Consolidate** all results into unified documentation

## Notes

- All test cases are designed to be idempotent and can be run multiple times
- Test cases clean up after themselves to avoid side effects
- Cross-platform compatibility (macOS/Linux) is considered in all commands
- All regex patterns use portable POSIX syntax where possible
- Hook test cases verify both successful hook execution and proper error handling
- Hook marker files are created in /tmp to verify hook execution timing
- Manual tests are automatically skipped during acceptance suite execution
- Bash commands tests ensure generated scripts are bash 3.2+ compatible
- Variables tests verify capture, validation, and substitution workflows
