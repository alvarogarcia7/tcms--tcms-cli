# Dependency Test Cases

This directory contains test cases for validating dependency resolution and error handling in the test harness.

## Test Cases

### 1. TC_DEPENDENCY_SIMPLE_001
**Simple Dependency Test**
- Tests basic include dependency where one test case references another via `include` in `initial_conditions`
- References: `TC_SUCCESS_SIMPLE_001`
- Demonstrates: Basic test case-level dependency resolution

### 2. TC_DEPENDENCY_SEQUENCE_001
**Sequence-Level Dependency Test**
- Tests sequence-specific dependencies using the `test_sequence` field in include references
- Contains 3 sequences:
  - Sequence 1: Independent (no dependencies)
  - Sequence 2: Depends on `TC_SUCCESS_MULTI_SEQ_001` sequence 1
  - Sequence 3: Depends on multiple sequences from different test cases
- Demonstrates: Fine-grained dependency resolution at the sequence level

### 3. TC_DEPENDENCY_NESTED_001
**Nested Dependency Test**
- Tests transitive dependencies (A → B → C)
- Dependency chain: `TC_DEPENDENCY_NESTED_001` → `TC_DEPENDENCY_SIMPLE_001` → `TC_SUCCESS_SIMPLE_001`
- Contains 3 sequences with varying levels of nested dependencies
- Demonstrates: 
  - Transitive dependency resolution
  - Multi-level dependency graphs
  - Dependency closure computation

### 4. TC_DEPENDENCY_CIRCULAR_001 + TC_DEPENDENCY_CIRCULAR_002
**Circular Dependency Detection Test**
- Two test cases that create a circular dependency:
  - `TC_DEPENDENCY_CIRCULAR_001` includes `TC_DEPENDENCY_CIRCULAR_002`
  - `TC_DEPENDENCY_CIRCULAR_002` includes `TC_DEPENDENCY_CIRCULAR_001`
- Expected outcome: Should trigger circular dependency detection error
- Demonstrates: Error handling for circular references

### 5. TC_DEPENDENCY_MISSING_001
**Missing Dependency Error Handling Test**
- References non-existent test cases:
  - `TC_NONEXISTENT_999` (does not exist)
  - `TC_DOES_NOT_EXIST_001` (does not exist)
- Contains 2 sequences:
  - Sequence 1: Missing test case dependency
  - Sequence 2: Missing test sequence dependency (existing test case, non-existent sequence)
- Expected outcome: Should trigger missing dependency errors
- Demonstrates: Error handling for unresolved dependencies

### 6. TC_DEPENDENCY_COMPLEX_001
**Complex Dependency Scenario Test**
- Tests multiple dependencies at both test case and sequence levels
- Contains 4 sequences demonstrating:
  - Sequence 1: Root-level dependencies only
  - Sequence 2: Combined root and sequence dependencies (4 total)
  - Sequence 3: Dependency deduplication when includes appear at multiple levels
  - Sequence 4: Transitive dependency resolution in complex graphs
- Demonstrates:
  - Dependency merging and inheritance
  - Deduplication of duplicate includes
  - Complex dependency graph resolution

### 7. TC_DEPENDENCY_SELF_REF_001
**Self-Reference Detection Test**
- Test case includes itself: `TC_DEPENDENCY_SELF_REF_001` → `TC_DEPENDENCY_SELF_REF_001`
- Contains 2 sequences:
  - Sequence 1: Test case-level self-reference
  - Sequence 2: Sequence-level self-reference
- Expected outcome: Should trigger self-reference/circular dependency detection error
- Demonstrates: Detection of direct self-referential dependencies

## Dependency Features Tested

### Include Reference Structure
```yaml
initial_conditions:
  include:
    - id: 'TC_OTHER_TEST_001'              # Test case dependency
    - id: 'TC_ANOTHER_TEST_001'
      test_sequence: '2'                    # Specific sequence dependency
```

### Test Case Level Dependencies
Dependencies defined at the root `initial_conditions` apply to the entire test case.

### Sequence Level Dependencies
Dependencies defined in a sequence's `initial_conditions` are specific to that sequence and are combined with root-level dependencies.

### Dependency Resolution Features
1. **Transitive Resolution**: Dependencies of dependencies are automatically resolved
2. **Deduplication**: Duplicate includes are detected and resolved once
3. **Circular Detection**: Circular dependencies are detected and reported as errors
4. **Missing Detection**: Missing test cases or sequences are detected and reported
5. **Self-Reference Detection**: Self-referential includes are detected as circular dependencies
6. **Inheritance**: Sequence dependencies inherit and extend root-level dependencies

## Expected Behavior

### Success Cases
- `TC_DEPENDENCY_SIMPLE_001`: Should resolve dependencies successfully
- `TC_DEPENDENCY_SEQUENCE_001`: Should resolve all sequence-specific dependencies
- `TC_DEPENDENCY_NESTED_001`: Should resolve transitive dependencies
- `TC_DEPENDENCY_COMPLEX_001`: Should handle complex dependency graphs

### Error Cases
- `TC_DEPENDENCY_CIRCULAR_001` + `TC_DEPENDENCY_CIRCULAR_002`: Should detect circular dependency
- `TC_DEPENDENCY_MISSING_001`: Should detect missing test cases and sequences
- `TC_DEPENDENCY_SELF_REF_001`: Should detect self-reference as circular dependency

## Usage

These test cases are designed to validate the dependency resolution implementation in the test harness. They can be used for:

1. **Validation Testing**: Verify YAML parsing correctly identifies include references
2. **Resolution Testing**: Verify dependency graphs are correctly computed
3. **Error Handling Testing**: Verify proper error messages for invalid dependencies
4. **Integration Testing**: Verify end-to-end dependency resolution in script generation

## Notes

- All dependency test cases follow the same YAML schema as other test cases
- The `include` field is optional and only used when dependencies exist
- Dependencies can reference both test case IDs and specific sequence IDs
- Circular dependencies should be detected during validation before execution
- Missing dependencies should be reported with clear error messages indicating which dependencies are missing
