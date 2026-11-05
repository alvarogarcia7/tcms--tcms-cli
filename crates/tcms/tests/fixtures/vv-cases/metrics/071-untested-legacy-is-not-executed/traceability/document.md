# VV fixture metrics/071-untested-legacy-is-not-executed

## Document Information

**Project:** tcms-cli VV fixtures

**Test Date:** 2026-11-05T00:00:00Z

**Environment:** VV fixture

**Platform:** linux

**Executor:** tcms-cli fixtures



---

## Executive Summary

### Test Execution Overview

This document presents the results of test execution for tcms-cli VV fixtures.

| Metric | Value |
|--------|-------|
| Total Test Cases | 2 |
| Passed Test Cases | 1 |
| Failed Test Cases | 1 |
| Not Executed Test Cases | 0 |

### Success Rate

**Pass Rate:** 50%


---

# Detailed Test Results

## Test Case: TC_001

**Description**: VV fixture metrics/071-untested-legacy-is-not-executed: verdict pass

**Overall Status**: ✓ PASS

## Execution Summary

| Metric | Count |
|--------|-------|
| Total Steps | 3 |
| Passed Steps | 3 |
| Failed Steps | 0 |
| Not Executed Steps | 0 |


## Test Sequence 1: VV fixture steps

**Sequence Status**: ✓ PASS

### Step Results

| Step | Description | Status |
|------|-------------|--------|
| 1 | step 1 | ✓ Pass |
| 2 | step 2 | ✓ Pass |
| 3 | step 3 | ✓ Pass |






## Test Case: TC_002

**Description**: VV fixture metrics/071-untested-legacy-is-not-executed: verdict untested

**Overall Status**: ✗ FAIL

## Execution Summary

| Metric | Count |
|--------|-------|
| Total Steps | 3 |
| Passed Steps | 0 |
| Failed Steps | 0 |
| Not Executed Steps | 3 |


## Test Sequence 1: VV fixture steps

**Sequence Status**: ✗ FAIL

### Step Results

| Step | Description | Status |
|------|-------------|--------|
| 1 | step 1 | ⊘ Not Executed |
| 2 | step 2 | ⊘ Not Executed |
| 3 | step 3 | ⊘ Not Executed |





---

## Final Verification Verdict

### Overall Test Execution Status

| Verdict | Result |
|---------|--------|
| Overall Status | ✗ **FAIL** |
| Total Test Cases | 2 |
| Passed | 1 |
| Failed | 1 |
| Not Executed | 0 |

### Conclusion

1 test case(s) failed during execution. Review the detailed test results section for failure analysis and remediation actions.


---

## Traceability Matrix

| Requirement | SUT1-1 |
|---|---|
| Tests | ✓ TC_001<br><span style="color:red">✗</span> TC_002<br> |
| Verdict | FAIL |


---

## Copyright Notice

(C) GSMA 2024. All rights reserved.
