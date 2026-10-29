# Prerequisites Test Cases

This directory contains comprehensive test cases for validating the prerequisite functionality of the test execution framework.

## Overview

Prerequisites are conditions that must be satisfied before a test case can execute. The framework supports two types of prerequisites:

1. **Manual Prerequisites**: Require human verification and user confirmation
2. **Automatic Prerequisites**: Verified programmatically with a command

## Test Cases

### PREREQ_AUTO_PASS_001.yaml
**Purpose**: Verify automatic prerequisites that pass verification

**Prerequisites**:
- Check that echo command is available
- Verify /tmp directory exists and is writable
- Confirm bash version 3.2 or higher is available

**Expected Behavior**: All prerequisites pass, test sequences execute successfully

**Validation Points**:
- Prerequisites are checked before any test steps execute
- All prerequisite verification commands return exit code 0
- Test sequences proceed after successful prerequisite validation

---

### PREREQ_AUTO_FAIL_001.yaml
**Purpose**: Verify automatic prerequisites that fail verification

**Prerequisites**:
- Check for a non-existent command (should fail)
- Verify a non-existent directory (should fail)

**Expected Behavior**: Prerequisites fail, test execution stops before executing test sequences

**Validation Points**:
- Prerequisite failures are detected and reported
- Test sequences do not execute when prerequisites fail
- Exit code indicates prerequisite failure

---

### PREREQ_MANUAL_001.yaml
**Purpose**: Verify manual prerequisites requiring user confirmation

**Prerequisites**:
- Ensure you have network connectivity to the internet
- Verify that you have appropriate permissions to create files in /tmp
- Confirm that no other test processes are currently running

**Expected Behavior**: 
- In interactive mode: User is prompted to confirm each manual prerequisite
- In non-interactive mode: Manual prerequisites are assumed satisfied
- Test sequences execute after user confirmation

**Validation Points**:
- Manual prerequisites prompt for user confirmation in interactive mode
- Test execution continues after manual prerequisite confirmation
- Manual test steps work correctly with prerequisite workflow

---

### PREREQ_MIXED_001.yaml
**Purpose**: Verify mixed automatic and manual prerequisites

**Prerequisites**:
- Manual: Ensure you are logged in with appropriate user privileges
- Automatic: Check that /tmp directory is available
- Manual: Verify that you have reviewed the test plan documentation
- Automatic: Confirm date command is available
- Automatic: Verify grep command is available
- Manual: Confirm that test execution logs are being captured

**Expected Behavior**: 
- Manual prerequisites prompt for confirmation (interactive mode)
- Automatic prerequisites are verified with commands
- All prerequisites must pass before test execution
- Prerequisites are only checked once at the beginning

**Validation Points**:
- Both manual and automatic prerequisites work together
- Prerequisites are checked in the order defined
- Test sequences execute only after all prerequisites pass
- Multiple test sequences share the same prerequisite validation

---

### PREREQ_COMPLEX_001.yaml
**Purpose**: Complex test case with multiple automatic prerequisites testing various system conditions

**Prerequisites**:
- Verify bash shell is available
- Check that temporary directory exists and is writable
- Confirm echo command works correctly
- Verify test command is available
- Check that grep supports extended regex
- Verify sed is available for text processing
- Confirm awk is available for text processing

**Expected Behavior**: All 7 automatic prerequisites pass, enabling comprehensive text processing and file operations

**Validation Points**:
- Multiple automatic prerequisites can be validated sequentially
- Prerequisites verify required utilities before they are used
- Complex prerequisite chains work correctly
- Test sequences can rely on utilities validated in prerequisites

---

### PREREQ_PARTIAL_FAIL_001.yaml
**Purpose**: Test case where some automatic prerequisites pass but at least one fails

**Prerequisites**:
- Check that echo command is available (should pass)
- Verify /tmp directory exists (should pass)
- Check for non-existent utility (should fail)
- This check should not be reached (after failure)

**Expected Behavior**: 
- First two prerequisites pass
- Third prerequisite fails
- Fourth prerequisite is not checked (execution stops at first failure)
- Test sequences do not execute

**Validation Points**:
- Prerequisite checking stops at first failure
- Partial success does not allow test execution
- Failure is reported clearly
- Subsequent prerequisites are not checked after a failure

---

### PREREQ_NONE_001.yaml
**Purpose**: Baseline test case with no prerequisites defined

**Prerequisites**: None

**Expected Behavior**: Test executes immediately without any prerequisite checks

**Validation Points**:
- Tests can execute without prerequisites
- No prerequisite section in YAML is valid
- Test execution proceeds normally
- Provides baseline comparison for prerequisite functionality

---

## Schema Validation

All test cases comply with the test-case.schema.json schema requirements:

- Prerequisites are optional (array)
- Each prerequisite has required fields: `type` and `description`
- Automatic prerequisites must include `verification_command`
- Manual prerequisites do not require `verification_command`
- Type must be either "manual" or "automatic"

## Execution Workflow

1. **Validation Stage**: All YAML files are validated against schema
2. **Generation Stage**: Bash scripts are generated with prerequisite checks
3. **Execution Stage**: 
   - Prerequisites are checked first
   - Manual prerequisites prompt for confirmation (interactive mode)
   - Automatic prerequisites execute verification commands
   - Test sequences execute only if all prerequisites pass
4. **Verification Stage**: Execution logs are verified
5. **Documentation Stage**: Reports are generated

## Testing Prerequisites

To test these test cases:

```bash
# Validate all prerequisite test cases
./target/debug/validate-yaml --schema schemas/test-case.schema.json \
  test-acceptance/00_test_cases/prerequisites/*.yaml

# Generate scripts for all prerequisite test cases
./target/debug/test-executor generate --json-log \
  --output /tmp/prereq_test.sh \
  test-acceptance/00_test_cases/prerequisites/PREREQ_AUTO_PASS_001.yaml

# Run acceptance test suite (includes prerequisite tests)
make acceptance-test
```

## Expected Outcomes

### Success Cases
- `PREREQ_AUTO_PASS_001.yaml`: ✓ Pass (all prerequisites satisfied)
- `PREREQ_MANUAL_001.yaml`: ✓ Pass (in non-interactive mode or with user confirmation)
- `PREREQ_MIXED_001.yaml`: ✓ Pass (all prerequisites satisfied)
- `PREREQ_COMPLEX_001.yaml`: ✓ Pass (all utilities available)
- `PREREQ_NONE_001.yaml`: ✓ Pass (no prerequisites to check)

### Failure Cases
- `PREREQ_AUTO_FAIL_001.yaml`: ✗ Fail (prerequisite verification fails)
- `PREREQ_PARTIAL_FAIL_001.yaml`: ✗ Fail (one prerequisite fails)

## Documentation Generation

All prerequisite test cases support documentation generation via test-plan-documentation-generator:

```bash
# Generate documentation for prerequisite tests
make generate-docs
```

This creates AsciiDoc and Markdown reports documenting:
- Test case metadata
- Prerequisites with their types and descriptions
- Test sequences and steps
- Verification results
