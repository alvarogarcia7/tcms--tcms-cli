# Hooks Test Cases

This directory contains comprehensive test cases for validating the test execution lifecycle hooks feature of the YAML-based test harness.

## Overview

Hooks provide extensibility points throughout the test execution lifecycle. Eight hook types are available:

1. **script_start** - Executes once at script beginning (global initialization)
2. **setup_test** - Executes once after script_start (test-wide setup)
3. **before_sequence** - Executes before each test sequence (sequence initialization)
4. **after_sequence** - Executes after each test sequence (sequence cleanup)
5. **before_step** - Executes before each test step (step preparation)
6. **after_step** - Executes after each test step (step validation)
7. **teardown_test** - Executes once before script_end (test-wide cleanup)
8. **script_end** - Executes once at script end (final logging/cleanup)

## Test Cases

### Success Scenarios

#### TC_HOOKS_001.yaml - Comprehensive Hook Lifecycle
- **Description**: Demonstrates all eight hook types with external scripts
- **Hook Scripts**: Uses external shell scripts in `scripts/` directory
- **Error Mode**: on_error: fail
- **Sequences**: 3 sequences (12 total steps)
  1. Hook Lifecycle with on_error: fail (4 steps)
  2. Hook Integration with Variables and Capture (3 steps)
  3. Hook Error Handling with on_error: continue (3 steps)
- **Validates**: 
  - All hooks execute at correct lifecycle points
  - Hooks can create/manage temporary resources
  - Hooks have access to context variables
  - Hook marker files are created successfully

#### TC_HOOKS_SIMPLE_001.yaml - Basic Hook Functionality
- **Description**: Simple test with only script_start and script_end hooks
- **Hook Scripts**: Inline hook commands
- **Error Mode**: on_error: fail
- **Sequences**: 1 sequence (3 steps)
- **Validates**:
  - Basic hook execution at test boundaries
  - Hook marker file creation

#### TC_HOOKS_INLINE_001.yaml - Inline Hook Commands
- **Description**: All hooks defined as inline bash commands (no external scripts)
- **Hook Scripts**: All eight hook types as inline YAML commands
- **Error Mode**: on_error: fail
- **Sequences**: 2 sequences (4 total steps)
- **Validates**:
  - Inline hook command execution
  - Hook context variables (TEST_SEQUENCE_ID, TEST_STEP)
  - Marker file creation for each hook type

#### TC_HOOKS_CONTINUE_001.yaml - Continue on Hook Error
- **Description**: Tests on_error: continue mode where hook failures don't stop execution
- **Hook Scripts**: Inline hooks with intentional failures
- **Error Mode**: on_error: continue
- **Sequences**: 1 sequence (4 steps)
- **Validates**:
  - Test continues executing even when before_step hook fails on step 2
  - All steps execute despite hook failure
  - on_error: continue mode working correctly

### Error Scenarios

#### TC_HOOKS_MISSING_001.yaml - Missing Hook Script Error
- **Description**: Verifies error handling when hook script file doesn't exist
- **Hook Scripts**: References non-existent script file
- **Error Mode**: on_error: fail
- **Expected**: Test fails immediately due to missing script_start hook
- **Validates**: Proper error handling for missing hook scripts

#### TEST_HOOK_SCRIPT_START_001.yml - Script Start Hook Error
- **Description**: script_start hook exits with error code
- **Hook Scripts**: `scripts/hook_script_start_error.sh` (always fails)
- **Error Mode**: on_error: fail
- **Expected**: Test terminates immediately, no steps execute
- **Validates**: script_start hook error handling

#### TEST_HOOK_SETUP_TEST_001.yml - Setup Test Hook Error
- **Description**: setup_test hook references non-existent script
- **Hook Scripts**: `scripts/nonexistent_setup_hook.sh` (missing file)
- **Error Mode**: on_error: fail
- **Expected**: Test fails during setup phase
- **Validates**: setup_test hook error handling

#### TEST_HOOK_BEFORE_SEQ_001.yml - Before Sequence Hook Error
- **Description**: before_sequence hook fails on sequence 2
- **Hook Scripts**: `scripts/hook_before_sequence_error.sh` (fails on sequence 2)
- **Error Mode**: on_error: fail
- **Expected**: Sequence 1 executes, sequences 2 and 3 don't execute
- **Validates**: before_sequence hook error handling

#### TEST_HOOK_AFTER_SEQ_001.yml - After Sequence Hook Error
- **Description**: after_sequence hook fails after sequence 1
- **Hook Scripts**: `scripts/hook_after_sequence_error.sh` (fails after sequence 1)
- **Error Mode**: on_error: fail
- **Expected**: Sequence 1 completes, sequences 2 and 3 don't execute
- **Validates**: after_sequence hook error handling

#### TEST_HOOK_BEFORE_STEP_001.yml - Before Step Hook Error
- **Description**: before_step hook fails on step 3
- **Hook Scripts**: `scripts/hook_before_step_error.sh` (fails on step 3)
- **Error Mode**: on_error: fail
- **Expected**: Steps 1 and 2 execute, step 3 and beyond don't execute
- **Validates**: before_step hook error handling

#### TEST_HOOK_AFTER_STEP_001.yml - After Step Hook Error
- **Description**: after_step hook fails after step 2
- **Hook Scripts**: `scripts/hook_after_step_error.sh` (fails after step 2)
- **Error Mode**: on_error: fail
- **Expected**: Steps 1 and 2 execute, step 3 and beyond don't execute
- **Validates**: after_step hook error handling

#### TEST_HOOK_TEARDOWN_001.yml - Teardown Test Hook Error
- **Description**: teardown_test hook fails during cleanup
- **Hook Scripts**: `scripts/hook_teardown_error.sh` (always fails)
- **Error Mode**: on_error: fail
- **Expected**: All sequences complete, teardown hook failure is reported
- **Validates**: teardown_test hook error handling

#### TEST_HOOK_SCRIPT_END_001.yml - Script End Hook Error
- **Description**: script_end hook fails at script termination
- **Hook Scripts**: `scripts/hook_script_end_error.sh` (always fails)
- **Error Mode**: on_error: fail
- **Expected**: All test execution completes, script_end hook failure is reported
- **Validates**: script_end hook error handling

## Hook Scripts

All hook scripts are located in the `scripts/` directory:

### Success Hook Scripts

Used by TC_HOOKS_001.yaml to demonstrate successful hook execution:

- **script_start.sh** - Logs test start time, creates marker file at `/tmp/tc_hooks_001_start_time.txt`
- **setup_test.sh** - Creates test workspace at `/tmp/tc_hooks_001_test_workspace` and sequence directory at `/tmp/tc_hooks_001_sequences`
- **before_sequence.sh** - Logs sequence start, creates sequence log file
- **after_sequence.sh** - Cleans up sequence resources, removes sequence-specific files
- **before_step.sh** - Logs step details and variables before execution
- **after_step.sh** - Validates step output, saves results to log files
- **teardown_test.sh** - Removes temporary directories and files created during test
- **script_end.sh** - Logs test completion time, calculates total duration

### Error Hook Scripts

Used by TEST_HOOK_* test cases to verify error handling:

- **hook_script_start_error.sh** - Always exits with code 1 (fails immediately)
- **hook_before_sequence_error.sh** - Fails on sequence 2 (allows sequence 1 to pass)
- **hook_after_sequence_error.sh** - Fails after sequence 1 (sequence 1 completes first)
- **hook_before_step_error.sh** - Fails on step 3 (steps 1-2 execute)
- **hook_after_step_error.sh** - Fails after step 2 (steps 1-2 execute)
- **hook_teardown_error.sh** - Always exits with code 1 (fails during cleanup)
- **hook_script_end_error.sh** - Always exits with code 1 (fails at script end)

All hook scripts use bash and are compatible with bash 3.2+. Success hooks use the logger library at `scripts/lib/logger.sh` for consistent formatted output.

## Running the Tests

### Run acceptance suite on hooks test cases:

```bash
# Run acceptance suite with hooks test cases
cd test-acceptance
./run_acceptance_suite.sh --verbose

# Or run specific stages
./run_acceptance_suite.sh --verbose --skip-execution --skip-verification
```

### Validate individual test cases:

```bash
# Validate YAML syntax
cargo run --bin validate-yaml -- \
  --schema schemas/test-case.schema.json \
  test-acceptance/00_test_cases/hooks/TC_HOOKS_001.yaml

# Generate bash script
cargo run --bin test-executor generate \
  --json-log \
  --output /tmp/TC_HOOKS_001.sh \
  test-acceptance/00_test_cases/hooks/TC_HOOKS_001.yaml

# Execute generated script
bash /tmp/TC_HOOKS_001.sh

# Verify execution results
cargo run --bin verifier -- \
  --test-case TC_HOOKS_001 \
  --log /tmp/TC_HOOKS_001_execution_log.json \
  --test-case-dir test-acceptance/00_test_cases/hooks \
  --output /tmp/TC_HOOKS_001_container.yaml
```

## Expected Results

### Success Test Cases
- **TC_HOOKS_001**: All hooks execute successfully, marker files created, 3 sequences complete
- **TC_HOOKS_SIMPLE_001**: script_start and script_end hooks execute, marker files created
- **TC_HOOKS_INLINE_001**: All inline hooks execute, context variables accessible
- **TC_HOOKS_CONTINUE_001**: Test continues despite hook failure on step 2

### Error Test Cases
- **TC_HOOKS_MISSING_001**: Fails immediately with "hook script not found" error
- **TEST_HOOK_SCRIPT_START_001**: Fails before any steps execute
- **TEST_HOOK_SETUP_TEST_001**: Fails during setup phase
- **TEST_HOOK_BEFORE_SEQ_001**: Sequence 1 passes, fails before sequence 2
- **TEST_HOOK_AFTER_SEQ_001**: Sequence 1 passes, fails after sequence 1
- **TEST_HOOK_BEFORE_STEP_001**: Steps 1-2 pass, fails before step 3
- **TEST_HOOK_AFTER_STEP_001**: Steps 1-2 pass, fails after step 2
- **TEST_HOOK_TEARDOWN_001**: All sequences pass, fails during teardown
- **TEST_HOOK_SCRIPT_END_001**: All sequences pass, fails at script end

## Hook Execution Order

For a test with 2 sequences, each with 2 steps:

```
1. script_start        (once at beginning)
2. setup_test          (once after script_start)
3. before_sequence     (sequence 1)
4.   before_step       (sequence 1, step 1)
5.   [execute step]
6.   after_step        (sequence 1, step 1)
7.   before_step       (sequence 1, step 2)
8.   [execute step]
9.   after_step        (sequence 1, step 2)
10. after_sequence     (sequence 1)
11. before_sequence    (sequence 2)
12.   before_step      (sequence 2, step 1)
13.   [execute step]
14.   after_step       (sequence 2, step 1)
15.   before_step      (sequence 2, step 2)
16.   [execute step]
17.   after_step       (sequence 2, step 2)
18. after_sequence     (sequence 2)
19. teardown_test      (once before script_end)
20. script_end         (once at end)
```

## Hook Context Variables

Hooks have access to the following environment variables:

- **TEST_SEQUENCE_ID**: Current sequence ID (before_sequence, after_sequence, before_step, after_step)
- **TEST_STEP**: Current step number (before_step, after_step)
- **STEP_EXIT_CODE**: Exit code of completed step (after_step only)

Additional context variables may be available depending on the test executor implementation.

## Notes

- Hook scripts must be executable (chmod +x)
- Hook scripts should use bash 3.2+ compatible syntax
- Hooks can create temporary files in /tmp for tracking state
- Hook failures with on_error: fail terminate test execution immediately
- Hook failures with on_error: continue allow test execution to continue
- All hook marker files should be cleaned up by teardown_test or script_end hooks
- Hook scripts should use the logger library for consistent output formatting
