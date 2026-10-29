# Complex Test Cases - Quick Index

## Files in This Directory

### Test Case YAML Files (9 files)

| File | Lines | Sequences | Features |
|------|-------|-----------|----------|
| [TC_COMPLEX_MULTI_SEQ_HOOKS_001.yaml](TC_COMPLEX_MULTI_SEQ_HOOKS_001.yaml) | 312 | 3 | Multi-sequence + All 8 hooks + Variables + Manual steps |
| [TC_COMPLEX_PREREQ_DEPS_HOOKS_001.yaml](TC_COMPLEX_PREREQ_DEPS_HOOKS_001.yaml) | 356 | 4 | Prerequisites + Dependencies + Hooks |
| [TC_COMPLEX_BDD_HOOKS_VARS_001.yaml](TC_COMPLEX_BDD_HOOKS_VARS_001.yaml) | 409 | 3 | BDD (Given/When/Then) + Hooks + Variables |
| [TC_COMPLEX_ALL_HOOKS_CAPTURE_001.yaml](TC_COMPLEX_ALL_HOOKS_CAPTURE_001.yaml) | 478 | 4 | All 8 hooks + Extensive variable capture |
| [TC_COMPLEX_FAILED_TEARDOWN_001.yaml](TC_COMPLEX_FAILED_TEARDOWN_001.yaml) | 240 | 3 | Intentional failure + Cleanup guarantees |
| [TC_COMPLEX_HYDRATION_CONDITIONAL_001.yaml](TC_COMPLEX_HYDRATION_CONDITIONAL_001.yaml) | 470 | 3 | 8 hydration vars + Conditional logic |
| [TC_COMPLEX_PERFORMANCE_TIMING_001.yaml](TC_COMPLEX_PERFORMANCE_TIMING_001.yaml) | 474 | 4 | Performance + Timing + Thresholds |
| [TC_COMPLEX_SECURITY_AUTH_API_001.yaml](TC_COMPLEX_SECURITY_AUTH_API_001.yaml) | 524 | 4 | Authentication + Tokens + API + Security |
| [TC_COMPLEX_DATA_DRIVEN_ITERATIONS_001.yaml](TC_COMPLEX_DATA_DRIVEN_ITERATIONS_001.yaml) | 568 | 5 | Iterations + Result aggregation + Statistics |

### Documentation Files (3 files)

| File | Purpose |
|------|---------|
| [README.md](README.md) | Comprehensive documentation for all test cases with detailed descriptions |
| [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) | Implementation details, feature matrix, and statistics |
| [INDEX.md](INDEX.md) | This file - quick reference index |

### Supporting Hook Scripts (3 files)

| File | Purpose |
|------|---------|
| `../../../scripts/hooks/script_start_init.sh` | Global test initialization hook |
| `../../../scripts/hooks/setup_test_workspace.sh` | Test workspace preparation hook |
| `../../../scripts/hooks/teardown_test_final.sh` | Final cleanup hook |

## Quick Reference

### By Feature

**All 8 Hook Types:**
- TC_COMPLEX_MULTI_SEQ_HOOKS_001.yaml
- TC_COMPLEX_ALL_HOOKS_CAPTURE_001.yaml

**Prerequisites & Dependencies:**
- TC_COMPLEX_PREREQ_DEPS_HOOKS_001.yaml

**BDD Pattern:**
- TC_COMPLEX_BDD_HOOKS_VARS_001.yaml

**Manual Steps:**
- TC_COMPLEX_MULTI_SEQ_HOOKS_001.yaml

**Intentional Failure:**
- TC_COMPLEX_FAILED_TEARDOWN_001.yaml

**Environment Variables:**
- TC_COMPLEX_HYDRATION_CONDITIONAL_001.yaml (8 vars)

**Performance Testing:**
- TC_COMPLEX_PERFORMANCE_TIMING_001.yaml

**Security Testing:**
- TC_COMPLEX_SECURITY_AUTH_API_001.yaml

**Data-Driven Testing:**
- TC_COMPLEX_DATA_DRIVEN_ITERATIONS_001.yaml

### By Complexity Level

**High Complexity (500+ lines):**
1. TC_COMPLEX_DATA_DRIVEN_ITERATIONS_001.yaml (568 lines)
2. TC_COMPLEX_SECURITY_AUTH_API_001.yaml (524 lines)
3. TC_COMPLEX_ALL_HOOKS_CAPTURE_001.yaml (478 lines)
4. TC_COMPLEX_PERFORMANCE_TIMING_001.yaml (474 lines)
5. TC_COMPLEX_HYDRATION_CONDITIONAL_001.yaml (470 lines)

**Medium Complexity (300-499 lines):**
1. TC_COMPLEX_BDD_HOOKS_VARS_001.yaml (409 lines)
2. TC_COMPLEX_PREREQ_DEPS_HOOKS_001.yaml (356 lines)
3. TC_COMPLEX_MULTI_SEQ_HOOKS_001.yaml (312 lines)

**Lower Complexity (200-299 lines):**
1. TC_COMPLEX_FAILED_TEARDOWN_001.yaml (240 lines)

### By Test Pattern

**Sequential Multi-Step:**
- All test cases follow sequential multi-step pattern

**Iterative:**
- TC_COMPLEX_DATA_DRIVEN_ITERATIONS_001.yaml

**State-Based:**
- TC_COMPLEX_BDD_HOOKS_VARS_001.yaml (Given/When/Then)
- TC_COMPLEX_SECURITY_AUTH_API_001.yaml (Auth flow)

**Failure Handling:**
- TC_COMPLEX_FAILED_TEARDOWN_001.yaml

## Statistics Summary

- **Total YAML Files**: 9
- **Total Lines**: 3,831
- **Total Sequences**: 31
- **Total Steps**: 130+
- **Hook Scripts**: 3
- **Documentation Files**: 3

## Getting Started

1. **Read**: Start with [README.md](README.md) for detailed descriptions
2. **Review**: Check [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) for technical details
3. **Choose**: Pick a test case based on the features you need
4. **Run**: Use the verifier to generate and execute test scripts

## Example Usage

```bash
# Generate script from a complex test case
cargo run --bin verifier -- \
  test-acceptance/00_test_cases/complex/TC_COMPLEX_MULTI_SEQ_HOOKS_001.yaml \
  test_script.sh

# Execute the generated script
bash test_script.sh

# Generate documentation
test-plan-documentation-generator \
  --input test-acceptance/00_test_cases/complex/ \
  --output reports/complex/
```
