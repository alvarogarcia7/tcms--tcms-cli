# Acceptance Test Suite - Quick Start Guide

Get started with the acceptance test suite orchestrator in minutes.

## Prerequisites

Build the required binaries:

```bash
cargo build --bin validate-yaml
cargo build --bin test-executor
cargo build --bin verifier
cargo build --bin validate-json
```

## Basic Usage

### Run Everything

Execute the complete test suite:

```bash
cd test-acceptance
./run_acceptance_suite.sh
```

This runs all 6 stages and generates a summary report.

### Run with Verbose Output

Get detailed progress information:

```bash
./run_acceptance_suite.sh --verbose
```

### Include Manual Tests

By default, manual tests are skipped. To include them:

```bash
./run_acceptance_suite.sh --include-manual
```

## Common Workflows

### Quick Validation

Validate YAML files without running tests:

```bash
./run_acceptance_suite.sh --skip-execution --skip-verification --skip-documentation
```

### Regenerate Documentation

Update documentation from existing verification results:

```bash
# Install TPDG first
cargo install test-plan-documentation-generator

# Regenerate docs
./run_acceptance_suite.sh --skip-generation --skip-execution
```

### Debug Test Failures

Run with verbose output and skip documentation:

```bash
./run_acceptance_suite.sh --verbose --skip-documentation
```

### Development Mode

During active development, skip expensive stages:

```bash
# Validate and generate only
./run_acceptance_suite.sh --skip-execution --skip-verification --skip-documentation
```

## Understanding Output

### Success Example

```
=== Stage 1: Validating Test Case YAMLs ===
[INFO] Found 93 test case YAML files

✓ TC_SUCCESS_SIMPLE_001.yaml
✓ TC_SUCCESS_MULTI_SEQ_001.yaml
...

[INFO] Validation: 93 passed, 0 failed

=== Stage 2: Generating Test Scripts ===
✓ TC_SUCCESS_SIMPLE_001.sh
✓ TC_SUCCESS_MULTI_SEQ_001.sh
...

[INFO] Generation: 93 passed, 0 failed
```

### Failure Example

```
=== Stage 3: Executing Test Scripts ===
✓ TC_SUCCESS_SIMPLE_001.sh
✗ TC_FAILURE_FIRST_STEP_001.sh (exit code: 1)
ℹ TC_MANUAL_ALL_001.sh (manual test, skipped)
...

[INFO] Execution: 75 passed, 1 failed, 17 skipped (manual)
```

## Output Artifacts

After running, check these directories:

```
test-acceptance/
├── scripts/               # Generated bash scripts
├── execution_logs/        # JSON execution logs
├── verification_results/  # Container YAMLs
└── reports/              # Documentation and summary
    ├── asciidoc/         # AsciiDoc reports
    ├── markdown/         # Markdown reports
    └── acceptance_suite_summary.txt  # Summary report
```

## Summary Report

View the detailed summary:

```bash
cat reports/acceptance_suite_summary.txt
```

Example output:

```
=========================================
Acceptance Test Suite Execution Summary
=========================================

Execution Date: Mon Mar 17 14:30:00 UTC 2024
Total Test Cases: 93

--- Stage 1: YAML Validation ---
Passed:  93
Failed:  0

--- Stage 2: Script Generation ---
Passed:  93
Failed:  0

--- Stage 3: Test Execution ---
Passed:  75
Failed:  1
Skipped: 17 (manual tests)

--- Stage 4: Verification ---
Passed:  76
Failed:  0

--- Stage 5: Container Validation ---
Passed:  76
Failed:  0

--- Stage 6: Documentation Generation ---
Passed:  76
Failed:  0

=========================================
Overall Result:
SUCCESS - All stages completed without errors
=========================================
```

## Troubleshooting

### "Binary not found" Error

**Problem:**
```
✗ validate-yaml binary not found at: ./target/debug/validate-yaml
[INFO] Run: cargo build --bin validate-yaml
```

**Solution:**
```bash
cargo build --bin validate-yaml
cargo build --bin test-executor
cargo build --bin verifier
cargo build --bin validate-json
```

### "No test cases found"

**Problem:** Script can't find YAML files

**Solution:** Ensure you're in the `test-acceptance` directory:
```bash
cd test-acceptance
./run_acceptance_suite.sh
```

### "TPDG not found"

**Problem:** Documentation generator not installed

**Solution:**
```bash
cargo install test-plan-documentation-generator
```

Or skip documentation:
```bash
./run_acceptance_suite.sh --skip-documentation
```

### Test Execution Failures

**Problem:** Some tests fail during execution

**Solution:**
1. Check which tests failed in the summary
2. Run failed test manually for details:
   ```bash
   ./scripts/TC_FAILED_TEST_001.sh
   ```
3. Review execution log:
   ```bash
   cat execution_logs/TC_FAILED_TEST_001.json
   ```

## Command Reference

```bash
# Full suite
./run_acceptance_suite.sh

# Verbose mode
./run_acceptance_suite.sh --verbose

# Include manual tests
./run_acceptance_suite.sh --include-manual

# Skip stages
./run_acceptance_suite.sh --skip-generation
./run_acceptance_suite.sh --skip-execution
./run_acceptance_suite.sh --skip-verification
./run_acceptance_suite.sh --skip-documentation

# Combine options
./run_acceptance_suite.sh --verbose --skip-documentation

# Help
./run_acceptance_suite.sh --help
```

## Next Steps

- **Full Documentation:** See [ACCEPTANCE_SUITE.md](ACCEPTANCE_SUITE.md)
- **Implementation Details:** See [ORCHESTRATOR_IMPLEMENTATION.md](ORCHESTRATOR_IMPLEMENTATION.md)
- **Test Cases:** See [test_cases/README.md](00_test_cases/README.md)
- **Project Info:** See [README.md](README.md)

## Tips

1. **Start Small:** Run with `--skip-execution` first to validate setup
2. **Use Verbose:** Add `--verbose` when troubleshooting
3. **Skip Documentation:** Save time during development with `--skip-documentation`
4. **Check Summary:** Always review `reports/acceptance_suite_summary.txt`
5. **Manual Tests:** Remember to use `--include-manual` for complete validation

## Getting Help

If you encounter issues:

1. Check the help message: `./run_acceptance_suite.sh --help`
2. Run with `--verbose` for detailed output
3. Review the summary report
4. Check individual test artifacts
5. Consult [ACCEPTANCE_SUITE.md](ACCEPTANCE_SUITE.md) for detailed documentation
