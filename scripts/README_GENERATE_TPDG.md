# Generate Acceptance TPDG Container Script

## Overview

The `generate_acceptance_tpdg_container.sh` script executes the `convert_verification_to_tpdg.py` Python script in dual-source mode to generate a TPDG (Test Plan Data Generator) container YAML file from acceptance test cases and their execution logs.

## Features

### Automatic Logging
- **Creates**: `test-acceptance/results/generation.log`
- **Captures**: All output from the conversion process
- **Format**: Timestamped entries for audit trail
- **Size**: ~12KB (typical run with 76 test cases)

### Output Files
1. `acceptance_test_results_container.yaml` - TPDG container with all test results
2. `generation.log` - Complete log of the generation process

## Usage

### Basic Execution

```bash
# From project root
./scripts/generate_acceptance_tpdg_container.sh
```

### What It Does

1. **Verifies Prerequisites**
   - Checks for Python 3.14 or Python 3
   - Verifies PyYAML is installed
   - Validates required directories exist

2. **Creates Output Directory**
   - Ensures `test-acceptance/results/` exists

3. **Initializes Log File**
   - Creates `generation.log` with header
   - Adds timestamp for start of execution

4. **Runs Conversion**
   - Executes `convert_verification_to_tpdg.py` with flags:
     ```
     --test-case-dir test-acceptance/00_test_cases
     --logs-dir test-acceptance/execution_logs
     --recursive
     --output test-acceptance/results/acceptance_test_results_container.yaml
     --title "Acceptance Test Suite Results"
     --project "Test Case Manager - Acceptance Test Suite"
     --verbose
     ```
   - Captures all output using `tee` (shows on screen + saves to log)

5. **Reports Statistics**
   - File size and line count
   - Conversion duration
   - Success/failure status

6. **Stages for Git**
   - Automatically adds generated YAML to git staging area
   - Provides commit command for user

## Log File Structure

The `generation.log` file contains:

```
========================================
TPDG Container Generation Log
Started: Wed Mar 26 21:06:45 UTC 2026
========================================

[2026-03-26 21:06:45] [INFO] Verifying prerequisites...
[2026-03-26 21:06:45] [INFO] Using Python: /usr/bin/python3
[2026-03-26 21:06:45] [SUCCESS] Prerequisites verified
[2026-03-26 21:06:45] [INFO] Running conversion script in dual-source mode...
[2026-03-26 21:06:45] [INFO]   Test cases: /path/to/test-acceptance/00_test_cases
[2026-03-26 21:06:45] [INFO]   Logs: /path/to/test-acceptance/execution_logs
[2026-03-26 21:06:45] [INFO]   Output: /path/to/test-acceptance/results/acceptance_test_results_container.yaml
[2026-03-26 21:06:45] [INFO]   Log file: /path/to/test-acceptance/results/generation.log

Found 89 YAML files to process
Processing test case: TC_COMPLEX_ALL_HOOKS_CAPTURE_001
  Warning: No execution log found at test-acceptance/execution_logs/TC_COMPLEX_ALL_HOOKS_CAPTURE_001_execution_log.json
...
✓ Wrote TPDG container to: test-acceptance/results/acceptance_test_results_container.yaml

✓ Successfully generated TPDG container with 76 test case(s)
  Output: test-acceptance/results/acceptance_test_results_container.yaml

[2026-03-26 21:06:46] [SUCCESS] TPDG container YAML generated successfully!
[2026-03-26 21:06:46] [INFO] Output file: /path/to/test-acceptance/results/acceptance_test_results_container.yaml
[2026-03-26 21:06:46] [INFO] Conversion time: 1s
[2026-03-26 21:06:46] [INFO] Generated file statistics:
[2026-03-26 21:06:46] [INFO]   Size: 86K
[2026-03-26 21:06:46] [INFO]   Lines: 2980

========================================
Completed: Wed Mar 26 21:06:46 UTC 2026
========================================
```

## Output Details

### YAML Container
- **Location**: `test-acceptance/results/acceptance_test_results_container.yaml`
- **Type**: `test_results_container`
- **Schema**: `tcms/testcase_results_container.schema.v1.json`
- **Size**: ~86KB for 76 test cases
- **Lines**: ~2,980 lines
- **Content**: All test cases with Pass/Fail/NotExecuted step results

### Generation Log
- **Location**: `test-acceptance/results/generation.log`
- **Size**: ~12KB
- **Lines**: ~157 lines
- **Content**: 
  - Script initialization
  - Prerequisite checks
  - Conversion progress
  - Warnings about missing execution logs
  - Success/failure messages
  - Statistics and timing

## Integration with Git

The script automatically stages the generated YAML file:

```bash
git add test-acceptance/results/acceptance_test_results_container.yaml
```

To commit:
```bash
git commit -m 'Add acceptance test results TPDG container'
```

**Note**: The `generation.log` file is in `.gitignore` by default (matches `*.log` pattern), but can be force-added if needed:
```bash
git add -f test-acceptance/results/generation.log
```

## Troubleshooting

### No Execution Logs Found

**Symptom**: Warnings like:
```
Warning: No execution log found at test-acceptance/execution_logs/TC_XXX_execution_log.json
```

**Cause**: Tests haven't been executed yet or logs are in different location

**Result**: All test steps marked as `NotExecuted` in container YAML

**Solution**: Run the acceptance test suite first:
```bash
./test-acceptance/run_acceptance_suite.sh
```

### PyYAML Not Installed

**Symptom**: 
```
[ERROR] PyYAML not installed
```

**Solution**:
```bash
pip3 install pyyaml
# or
pip3.14 install pyyaml
```

### Python Not Found

**Symptom**:
```
[ERROR] Python 3 not found
```

**Solution**: Install Python 3.14 or Python 3:
```bash
# macOS
brew install python@3.14

# Ubuntu/Debian
sudo apt-get install python3

# Verify
python3 --version
```

## Related Files

- **Conversion Script**: `scripts/convert_verification_to_tpdg.py`
- **Test Cases**: `test-acceptance/00_test_cases/` (recursive)
- **Execution Logs**: `test-acceptance/execution_logs/`
- **Results Directory**: `test-acceptance/results/`
- **Documentation**: `test-acceptance/results/README.md`
- **Implementation Doc**: `IMPLEMENTATION_TPDG_DUAL_SOURCE.md`

## See Also

- [Test Acceptance README](../test-acceptance/README.md)
- [Results Directory README](../test-acceptance/results/README.md)
- [Convert Verification to TPDG Script](./convert_verification_to_tpdg.py)
