# Acceptance Test Results

This directory contains the generated TPDG (Test Plan Data Generator) container YAML files that consolidate acceptance test results.

## Files

### `acceptance_test_results_container.yaml`

Main TPDG container file that includes all acceptance test cases with their execution results.

**Generation**: This file is generated using the `convert_verification_to_tpdg.py` script in dual-source mode, which:
- Scans all test case YAML files in `test-acceptance/test_cases/`
- Matches them with execution logs in `test-acceptance/execution_logs/`
- Builds verification results from scratch
- Converts to TPDG container YAML format

**Contents**:
- **type**: `test_results_container`
- **test_results**: Array of test case results with Pass/Fail/NotExecuted step variants
- **metadata**: Test execution metadata including total test cases, passed/failed counts

## Regenerating Results

To regenerate the TPDG container YAML file after running acceptance tests:

### Using the Helper Script

```bash
# From project root
./scripts/generate_acceptance_tpdg_container.sh
```

The script will:
1. Verify prerequisites (Python 3, PyYAML)
2. Create the results directory if needed
3. Run the conversion script in dual-source mode
4. Generate the TPDG container YAML
5. Stage the file for git commit

### Manual Generation

```bash
# From project root
python3 scripts/convert_verification_to_tpdg.py \
    --test-case-dir test-acceptance/00_test_cases \
    --logs-dir test-acceptance/execution_logs \
    --recursive \
    --output test-acceptance/results/acceptance_test_results_container.yaml \
    --title "Acceptance Test Suite Results" \
    --project "Test Case Manager - Acceptance Test Suite" \
    --verbose
```

## Execution Logs

The conversion script looks for execution logs in `test-acceptance/execution_logs/` with the naming pattern:
```
{test_case_id}_execution_log.json
```

If an execution log is not found for a test case, all steps will be marked as `NotExecuted`.

## Test Case Scanning

The conversion script recursively scans for test case YAML files in:
- `test-acceptance/test_cases/bash_commands/`
- `test-acceptance/test_cases/complex/`
- `test-acceptance/test_cases/dependencies/`
- `test-acceptance/test_cases/failure/`
- `test-acceptance/test_cases/hooks/`
- `test-acceptance/test_cases/manual/`
- `test-acceptance/test_cases/prerequisites/`
- `test-acceptance/test_cases/success/`
- `test-acceptance/test_cases/variables/`

Only files with `type: test_case` are processed.

## TPDG Container Format

The generated container follows the TPDG format specification:

```yaml
type: test_results_container
schema: tcms/testcase_results_container.schema.v1.json
title: "Acceptance Test Suite Results"
project: "Test Case Manager - Acceptance Test Suite"
test_date: "2024-03-26T18:14:51.035617"
test_results:
  - test_case_id: TC_EXAMPLE_001
    description: "Test description"
    sequences:
      - sequence_id: 1
        name: "Sequence name"
        step_results:
          - Pass:
              step: 1
              description: "Step description"
          - Fail:
              step: 2
              description: "Step description"
              expected:
                success: true
                result: "0"
                output: "expected output"
              actual_result: "1"
              actual_output: "actual output"
              reason: "result verification failed"
          - NotExecuted:
              step: 3
              description: "Step description"
        all_steps_passed: false
    total_steps: 3
    passed_steps: 1
    failed_steps: 1
    not_executed_steps: 1
    overall_pass: false
metadata:
  total_test_cases: 76
  passed_test_cases: 0
  failed_test_cases: 76
```

## Schema Validation

To validate the generated container YAML:

```bash
# Using validate-yaml binary
validate-yaml --schema data/testcase_results_container/schema.json \
    test-acceptance/results/acceptance_test_results_container.yaml

# Or using validate-json (if container is in JSON format)
validate-json --schema data/testcase_results_container/schema.json \
    test-acceptance/results/acceptance_test_results_container.json
```

## Integration with Acceptance Suite

This TPDG container generation is integrated into the acceptance test suite orchestrator:

```bash
# Run full acceptance suite (includes TPDG generation)
./test-acceptance/run_acceptance_suite.sh
```

The orchestrator automatically:
1. Validates test case YAMLs
2. Generates bash scripts
3. Executes tests
4. Generates execution logs
5. Runs verifier to create container YAMLs
6. Generates documentation using TPDG

## See Also

- [Acceptance Test Suite README](../README.md)
- [TPDG Conversion Script](../../scripts/convert_verification_to_tpdg.py)
- [Test Cases Directory](../00_test_cases/README.md)
- [Container Schema](../../data/testcase_results_container/schema.json)
