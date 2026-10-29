# Complex Test Cases

This directory contains comprehensive test cases that combine multiple advanced features of the test harness. Each test case demonstrates complex real-world testing scenarios.

## Test Cases Overview

### TC_COMPLEX_MULTI_SEQ_HOOKS_001
**Multi-sequence test with hooks, variables, and manual steps**

- **Features**: All 8 hook types, multi-sequence execution, variable capture, manual steps, conditional verification
- **Sequences**: 3 sequences covering setup, manual verification, and cleanup
- **Key Capabilities**:
  - Demonstrates full hook lifecycle (script_start, setup_test, before_sequence, after_sequence, before_step, after_step, teardown_test, script_end)
  - Variable capture and reuse across sequences
  - Manual verification steps interspersed with automated steps
  - Hydration variables for environment configuration
  - Conditional verification based on captured values
  - Test duration tracking and reporting

### TC_COMPLEX_PREREQ_DEPS_HOOKS_001
**Test with prerequisites, dependencies, and hooks**

- **Features**: Prerequisites (manual + automatic), sequence dependencies, hooks, variable capture
- **Sequences**: 4 sequences with explicit dependency chains
- **Key Capabilities**:
  - Manual and automatic prerequisite validation
  - Inter-sequence dependencies (Sequence N depends on Sequence N-1)
  - Session management with unique IDs
  - Resource allocation and cleanup
  - Archive creation and integrity verification
  - Comprehensive dependency tracking

### TC_COMPLEX_BDD_HOOKS_VARS_001
**BDD-style test with initial conditions, hooks, and variables**

- **Features**: BDD pattern (Given/When/Then), comprehensive initial conditions, hooks, variable capture
- **Sequences**: 3 sequences following BDD structure
- **Key Capabilities**:
  - Given: User authentication context establishment
  - When: User actions and system behavior
  - Then: System behavior verification
  - Role-based access control testing
  - Feature flag conditional logic
  - Detailed BDD test reporting

### TC_COMPLEX_ALL_HOOKS_CAPTURE_001
**Comprehensive test with all 8 hook types and extensive variable capture**

- **Features**: All 8 hook types, extensive variable capture, lifecycle demonstration
- **Sequences**: 4 sequences demonstrating complete hook lifecycle
- **Key Capabilities**:
  - Explicit demonstration of each hook type
  - Hook execution tracking and counting
  - Timing measurements for hook execution
  - Step-level hook demonstration
  - Metrics collection throughout lifecycle
  - Complete hook execution summary report

### TC_COMPLEX_FAILED_TEARDOWN_001
**Failed step scenario with teardown hooks ensuring cleanup**

- **Features**: Intentional failure, cleanup guarantee, teardown hooks
- **Sequences**: 3 sequences (setup, intentional failure, verification)
- **Key Capabilities**:
  - Resource allocation before failure
  - Intentional step failure (exit 1)
  - Cleanup hooks execute even on failure
  - Lock file and temporary resource management
  - Cleanup manifest tracking
  - Demonstrates on_error: continue behavior

### TC_COMPLEX_HYDRATION_CONDITIONAL_001
**Test with extensive hydration variables and conditional verification**

- **Features**: 8 hydration variables, environment-driven configuration, conditional logic
- **Sequences**: 3 sequences covering configuration, behavior, and validation
- **Key Capabilities**:
  - Multiple required and optional hydration variables
  - Environment-specific behavior (production/staging/development)
  - Conditional verification based on environment type
  - Role-based configuration
  - Feature flag handling
  - Complete configuration validation report

### TC_COMPLEX_PERFORMANCE_TIMING_001
**Performance test with timing capture and validation**

- **Features**: High-resolution timing, performance metrics, threshold validation
- **Sequences**: 4 sequences covering baseline, file I/O, processing, and reporting
- **Key Capabilities**:
  - Nanosecond-precision timing capture
  - Baseline performance measurement
  - File I/O performance testing
  - Data processing performance testing
  - Threshold-based pass/fail criteria
  - Performance breakdown analysis
  - Comprehensive performance reporting

### TC_COMPLEX_SECURITY_AUTH_API_001
**Security test with authentication token capture and API calls**

- **Features**: Authentication, token management, API simulation, secure cleanup
- **Sequences**: 4 sequences covering auth, API access, validation, and cleanup
- **Key Capabilities**:
  - Secure token generation and capture
  - Authorization header construction
  - Role-based API access control
  - Token expiry validation
  - File permission security checks
  - API audit logging
  - Secure session data cleanup and wiping
  - Security audit reporting

### TC_COMPLEX_DATA_DRIVEN_ITERATIONS_001
**Data-driven test with multiple iterations and aggregated results**

- **Features**: Iterative testing, result aggregation, statistical analysis, CSV data format
- **Sequences**: 5 sequences covering preparation, 3 iterations, and analysis
- **Key Capabilities**:
  - Test run identifier generation
  - CSV-based results tracking
  - Multiple test iterations with timing
  - Data processing and validation per iteration
  - Statistical analysis (average, min, max)
  - Success rate calculation
  - Quality threshold validation
  - Comprehensive analysis reporting
  - Results archival

## Common Patterns

### Hook Usage
All tests demonstrate proper hook usage with appropriate error handling:
- `on_error: "fail"` for critical setup hooks (script_start, setup_test)
- `on_error: "continue"` for cleanup hooks (teardown_test, script_end)

### Variable Capture
Tests capture variables using regex patterns and reuse them throughout sequences:
```yaml
capture_vars:
  - name: variable_name
    capture: '([0-9]+)'
```

### Conditional Verification
Tests use conditional logic for environment-aware verification:
```yaml
verification:
  output:
    condition: "[[ $variable -gt 10 ]]"
    if_true:
      - "echo 'Condition met'"
    if_false:
      - "echo 'Condition not met'"
    always:
      - "echo 'Always executed'"
```

### Hydration Variables
Tests use environment variables for configuration:
```yaml
hydration_vars:
  VAR_NAME:
    name: "VAR_NAME"
    description: "Variable description"
    default_value: "default"
    required: true
```

## Running Complex Tests

These tests can be run using the standard test harness workflow:

1. **Generate test script**: Run the verifier to generate executable bash script
2. **Execute test**: Run the generated script
3. **Review results**: Check execution logs and verification results
4. **Generate reports**: Use test-plan-documentation-generator for documentation

## Test Complexity Metrics

- **Total Test Cases**: 9
- **Total Sequences**: 31 sequences across all tests
- **Total Steps**: 130+ steps across all tests
- **Hook Types Used**: All 8 hook types demonstrated
- **Variable Captures**: 70+ variable captures
- **Conditional Verifications**: 25+ conditional verification blocks
- **Hydration Variables**: 25+ environment variables

## Notes

- These tests demonstrate production-ready patterns for complex testing scenarios
- All tests follow bash 3.2+ compatibility requirements
- Tests include proper error handling and cleanup
- Security test (TC_COMPLEX_SECURITY_AUTH_API_001) includes sensitive data handling patterns
- Performance test (TC_COMPLEX_PERFORMANCE_TIMING_001) uses nanosecond precision when available
