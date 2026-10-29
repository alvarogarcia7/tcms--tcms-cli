# Hook Test Cases Index

Quick reference guide for all hook test scenarios.

## By Hook Type

### script_start
- **HOOKS_SCRIPT_START_001**: Basic environment setup
- **HOOKS_SCRIPT_END_001**: Initialize tracking for summary
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test
- **HOOKS_VARIABLES_001**: Environment variable logging

### setup_test
- **HOOKS_SETUP_TEST_001**: Resource creation and database init
- **HOOKS_TEARDOWN_TEST_001**: Create resources for teardown
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test
- **HOOKS_ON_ERROR_FAIL_001**: Failing hook stops execution

### before_sequence
- **HOOKS_BEFORE_SEQUENCE_001**: Logging with sequence context
- **HOOKS_AFTER_SEQUENCE_001**: Create temp files per sequence
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test
- **HOOKS_SEQUENCE_CONTEXT_001**: Sequence context variables

### after_sequence
- **HOOKS_AFTER_SEQUENCE_001**: Cleanup temp files
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test
- **HOOKS_SEQUENCE_CONTEXT_001**: Sequence context variables

### before_step
- **HOOKS_BEFORE_STEP_001**: Pre-step validation
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test
- **HOOKS_ON_ERROR_CONTINUE_001**: Failing hook continues
- **HOOKS_STEP_CONTEXT_001**: Step context variables

### after_step
- **HOOKS_AFTER_STEP_001**: Metric collection and exit codes
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test
- **HOOKS_VARIABLES_001**: Variable access in hook
- **HOOKS_STEP_CONTEXT_001**: Step context with exit codes

### teardown_test
- **HOOKS_TEARDOWN_TEST_001**: Final resource cleanup
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test

### script_end
- **HOOKS_SCRIPT_END_001**: Execution summary generation
- **HOOKS_MULTI_COMBINED_001**: Part of all-hooks test

## By Feature

### Error Handling
- **HOOKS_ON_ERROR_FAIL_001**: on_error: fail stops execution
- **HOOKS_ON_ERROR_CONTINUE_001**: on_error: continue allows test to proceed

### Context Variables
- **HOOKS_SEQUENCE_CONTEXT_001**: TEST_SEQUENCE_ID, TEST_SEQUENCE_NAME
- **HOOKS_STEP_CONTEXT_001**: TEST_STEP_NUMBER, TEST_STEP_DESCRIPTION, STEP_EXIT_CODE
- **HOOKS_VARIABLES_001**: Environment vars, sequence vars, captured vars

### Use Cases
- **HOOKS_SCRIPT_START_001**: Environment setup
- **HOOKS_SETUP_TEST_001**: Resource creation
- **HOOKS_BEFORE_SEQUENCE_001**: Logging
- **HOOKS_AFTER_SEQUENCE_001**: Cleanup
- **HOOKS_BEFORE_STEP_001**: Validation
- **HOOKS_AFTER_STEP_001**: Metrics
- **HOOKS_TEARDOWN_TEST_001**: Final cleanup
- **HOOKS_SCRIPT_END_001**: Summary generation

### Combinations
- **HOOKS_MULTI_COMBINED_001**: All 8 hooks working together

## Quick Start

To run a test case:
```bash
# Generate script from test case
cargo run --bin verifier -- test-acceptance/00_test_cases/hooks/HOOKS_SCRIPT_START_001.yaml

# Execute the generated script
./generated_test_script.sh
```

## Files Structure

```
test-acceptance/
├── test_cases/hooks/           # Test case YAML files
│   ├── HOOKS_*.yaml           # 14 test scenarios
│   ├── README.md              # Detailed documentation
│   ├── SUMMARY.md             # Coverage statistics
│   └── INDEX.md               # This file
└── scripts/hooks/             # Hook implementation scripts
    └── *.sh                   # 27 hook scripts
```
