# VV fixture metrics/040-requirements-mixed

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
| Total Test Cases | 4 |
| Passed Test Cases | 1 |
| Failed Test Cases | 3 |
| Not Executed Test Cases | 0 |

### Success Rate

**Pass Rate:** 25%


---

# Detailed Test Results

## Test Case: TC_A

**Description**: VV fixture metrics/040-requirements-mixed: verdict pass

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






## Test Case: TC_B

**Description**: VV fixture metrics/040-requirements-mixed: verdict fail

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




## Test Case: TC_C

**Description**: VV fixture metrics/040-requirements-mixed: verdict not_executed

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






## Test Case: TC_D

**Description**: VV fixture metrics/040-requirements-mixed: verdict skipped

**Overall Status**: ✗ FAIL

## Execution Summary

| Metric | Count |
|--------|-------|
| Total Steps | 3 |
| Passed Steps | 3 |
| Failed Steps | 0 |
| Not Executed Steps | 3 |


## Test Sequence 1: VV fixture steps

**Sequence Status**: ✗ FAIL

### Step Results

| Step | Description | Status |
|------|-------------|--------|
| 1 | step 1 | ✓ Pass |
| 2 | step 2 | ✓ Pass |
| 3 | step 3 | ✓ Pass |





---

## Final Verification Verdict

### Overall Test Execution Status

| Verdict | Result |
|---------|--------|
| Overall Status | ✗ **FAIL** |
| Total Test Cases | 4 |
| Passed | 1 |
| Failed | 3 |
| Not Executed | 0 |

### Conclusion

3 test case(s) failed during execution. Review the detailed test results section for failure analysis and remediation actions.


---

## Traceability Matrix

| Requirement | SUT1-1 | SUT1-2 | SUT1-3 | SUT1-4 |
|---|---|---|---|---|
| Tests | ✓ TC_A<br> | <span style="color:red">✗</span> TC_B<br> | <span style="color:red">✗</span> TC_C<br> | <span style="color:red">✗</span> TC_D<br> |
| Verdict | PASS | FAIL | FAIL | FAIL |


---

## Copyright Notice

(C) GSMA 2024. All rights reserved.
