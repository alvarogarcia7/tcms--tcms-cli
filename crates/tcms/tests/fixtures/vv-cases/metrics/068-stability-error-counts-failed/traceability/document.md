# VV fixture metrics/068-stability-error-counts-failed

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
| Total Test Cases | 1 |
| Passed Test Cases | 0 |
| Failed Test Cases | 1 |
| Not Executed Test Cases | 0 |

### Success Rate

**Pass Rate:** 0%


---

# Detailed Test Results

## Test Case: TC_X

**Description**: VV fixture metrics/068-stability-error-counts-failed: verdict error

**Overall Status**: ✗ FAIL

## Execution Summary

| Metric | Count |
|--------|-------|
| Total Steps | 3 |
| Passed Steps | 1 |
| Failed Steps | 2 |
| Not Executed Steps | 0 |


## Test Sequence 1: VV fixture steps

**Sequence Status**: ✗ FAIL

### Step Results

| Step | Description | Status |
|------|-------------|--------|
| 1 | step 1 | ✓ Pass |
| 2 | step 2 | ✗ Fail |
| 3 | step 3 | ✗ Fail |



### Step 2 - Failure Details

**Description**: step 2

**Reason**: failed

| Property | Expected | Actual |
|----------|----------|--------|
| Result |  |  |
| Output |  |  |
| Success | true | N/A |

### Step 3 - Failure Details

**Description**: step 3

**Reason**: failed

| Property | Expected | Actual |
|----------|----------|--------|
| Result |  |  |
| Output |  |  |
| Success | true | N/A |



---

## Final Verification Verdict

### Overall Test Execution Status

| Verdict | Result |
|---------|--------|
| Overall Status | ✗ **FAIL** |
| Total Test Cases | 1 |
| Passed | 0 |
| Failed | 1 |
| Not Executed | 0 |

### Conclusion

1 test case(s) failed during execution. Review the detailed test results section for failure analysis and remediation actions.


---

## Traceability Matrix

| Requirement | SUT1-1 |
|---|---|
| Tests | <span style="color:red">✗</span> TC_X<br> |
| Verdict | FAIL |


---

## Copyright Notice

(C) GSMA 2024. All rights reserved.
