# Bash Commands Test Cases

This directory contains comprehensive test cases for validating bash command execution, covering simple to complex scenarios with strict validation.

## Overview

These test cases demonstrate and validate bash command functionality across multiple dimensions:
- Command execution in steps
- Command-based verification expressions
- Variable capture from command output
- Complex bash features and syntax

## Test Cases

### TC_BASH_SIMPLE_001 - Simple Bash Commands
**Purpose**: Basic bash commands (echo, pwd, whoami, date, true)  
**Features**:
- Simple echo output validation
- Working directory verification
- Username format validation
- Date format validation (ISO 8601)
- Exit code verification
- Empty output handling

**Validation**: Strict output matching, regex patterns, length checks

---

### TC_BASH_INTERMEDIATE_001 - Text Processing Commands
**Purpose**: Intermediate text processing with grep, sed, awk, wc  
**Features**:
- Grep pattern matching with line counting
- Sed substitution and validation
- Awk field extraction with numeric validation
- Word and line counting
- Email regex validation
- Line deletion with sed

**Validation**: Variable capture, numeric range checks, pattern presence/absence

---

### TC_BASH_COMPLEX_001 - Complex Command Pipelines
**Purpose**: Advanced bash features and multi-stage pipelines  
**Features**:
- Multi-stage pipelines with sort, head, awk
- Command substitution with arithmetic
- Complex grep with uniq counting
- Nested command substitution
- Complex awk with conditionals
- Process substitution with diff
- Find with file counting

**Validation**: Arithmetic verification, sum calculations, file system checks

---

### TC_BASH_VERIFICATION_001 - Commands in Verification
**Purpose**: Using bash commands within verification expressions  
**Features**:
- Grep-based output verification
- Arithmetic in verification conditions
- File content verification with cat
- Timestamp validation with date commands
- Sed-based extraction in verification
- Version string parsing

**Validation**: Dynamic verification using command execution, complex conditional logic

---

### TC_BASH_ARRAYS_001 - Array Operations
**Purpose**: Bash indexed arrays (bash 3.2+ compatible)  
**Features**:
- Array creation and element access
- Array length calculation
- Array iteration with summation
- Array slicing
- Array appending
- Element existence checking

**Validation**: Element counting, value verification, iteration correctness

---

### TC_BASH_STRING_OPS_001 - String Manipulation
**Purpose**: Bash string operations and parameter expansion  
**Features**:
- String length calculation
- Substring extraction
- Pattern replacement (single/all)
- Prefix/suffix removal (non-greedy/greedy)
- Case conversion (uppercase/lowercase)
- Default value substitution

**Validation**: Length checks, pattern matching, transformation verification

---

### TC_BASH_CONDITIONALS_001 - Conditional Expressions
**Purpose**: If/then/else, test operators, logical operators  
**Features**:
- Numeric comparisons (eq, gt, lt)
- String comparisons and regex matching
- File/directory existence tests
- Logical AND/OR operators
- String empty/non-empty tests
- Nested conditionals

**Validation**: Conditional logic verification, file system state checks

---

### TC_BASH_LOOPS_001 - Loop Constructs
**Purpose**: For, while, until loops with control flow  
**Features**:
- For loops with ranges and arrays
- While loops with counters
- Until loops
- Break statement
- Continue statement
- Nested loops
- File iteration

**Validation**: Iteration counting, value accumulation, break/continue behavior

---

### TC_BASH_FILE_OPS_001 - File Operations
**Purpose**: File creation, reading, writing, modification  
**Features**:
- File creation with content
- Append operations
- File size checking
- Copy and diff operations
- File renaming
- Line extraction with sed
- Permission setting
- File deletion

**Validation**: File existence, content verification, permission checks, cleanup validation

---

### TC_BASH_MATH_OPS_001 - Arithmetic Operations
**Purpose**: Bash arithmetic expansion and calculations  
**Features**:
- Basic operations (add, subtract, multiply, divide, modulo)
- Order of operations
- Parentheses precedence
- Increment/decrement operators
- Complex expressions
- Power operations with bc

**Validation**: Exact numeric results, arithmetic correctness

---

### TC_BASH_ENV_VARS_001 - Environment Variables
**Purpose**: Variable setting, exporting, and usage  
**Features**:
- Variable assignment and reading
- Numeric variables in arithmetic
- Export for subshells
- PATH and HOME checking
- Default value substitution
- Variable interpolation
- Multiple variable operations
- USER environment variable
- Readonly variables

**Validation**: Variable value checks, environment variable validation

---

### TC_BASH_PROCESS_OPS_001 - Process Operations
**Purpose**: Exit codes, process information, command chaining  
**Features**:
- Success/failure exit codes
- Current process ID ($$)
- Background process ID ($!)
- Command chaining with && and ||
- Process counting
- Shell level (SHLVL)
- Custom exit codes
- Argument counting

**Validation**: Exit code verification, PID validation, process counting

---

### TC_BASH_REDIRECTION_001 - I/O Redirection
**Purpose**: Stdin, stdout, stderr redirection and pipes  
**Features**:
- Stdout redirection (>)
- Append redirection (>>)
- Stderr redirection (2>)
- Combined redirection (2>&1)
- Stdin redirection (<)
- Multi-command pipes
- /dev/null redirection
- Here string (<<<)
- Tee command

**Validation**: File content checks, redirection verification, pipe correctness

---

## Statistics

- **Total Test Cases**: 12
- **Total Test Sequences**: 12
- **Total Steps**: 109
- **Average Steps per Test Case**: 9.1

## Coverage

### Command Types
- ✅ Simple commands (echo, pwd, whoami, date, true)
- ✅ Text processing (grep, sed, awk, wc, sort, uniq)
- ✅ File operations (cat, cp, mv, rm, touch, chmod, stat)
- ✅ Array operations (indexed arrays, bash 3.2+ compatible)
- ✅ String manipulation (length, substring, replacement, pattern matching)
- ✅ Conditionals (if/then/else, test operators, logical operators)
- ✅ Loops (for, while, until, break, continue)
- ✅ Arithmetic (all basic operations, increment/decrement, complex expressions)
- ✅ Environment variables (set, export, default values, readonly)
- ✅ Process operations (exit codes, PIDs, command chaining)
- ✅ I/O redirection (stdout, stderr, stdin, pipes, here strings)

### Validation Types
- ✅ Exit code verification
- ✅ Exact output matching
- ✅ Regex pattern matching
- ✅ Numeric range validation
- ✅ Variable capture (regex and command-based)
- ✅ File system state verification
- ✅ Arithmetic validation
- ✅ String length and format checks
- ✅ Dynamic verification with commands
- ✅ General verification conditions

## Cross-Platform Compatibility

All test cases are designed for:
- **macOS**: BSD utilities, bash 3.2+
- **Linux**: GNU utilities, bash 3.2+

### Compatibility Considerations
- Uses bash 3.2+ compatible syntax (no associative arrays)
- Avoids GNU-specific flags (e.g., uses `-E` instead of `-r` for sed)
- Portable regex patterns (POSIX-compliant)
- BSD/GNU stat command compatibility with fallback
- Tested with both BSD and GNU coreutils

## Usage

### Validate All Test Cases
```bash
for file in test-acceptance/00_test_cases/bash_commands/*.yaml; do
  echo "Validating $file"
  cargo run --bin verifier -- "$file" || echo "FAILED: $file"
done
```

### Generate Test Scripts
```bash
for file in test-acceptance/00_test_cases/bash_commands/TC_*.yaml; do
  echo "Generating script for $file"
  cargo run -- "$file"
done
```

### Execute Test Scripts
```bash
for file in test-acceptance/00_test_cases/bash_commands/TC_*.sh; do
  echo "Executing $file"
  bash "$file"
done
```

## Validation Features

### Step Command Execution
Commands are executed as the primary step action with strict output validation.

### Verification in Commands
Bash commands are used within verification expressions for dynamic validation:
```yaml
verification:
  result: "[[ $EXIT_CODE -eq 0 && $(command) -eq expected ]]"
  output: "[[ $(grep -c 'pattern' <<< \"$COMMAND_OUTPUT\") -eq 1 ]]"
```

### Variable Capture
Both regex-based and command-based variable capture:
```yaml
capture_vars:
  - name: value
    capture: '^([0-9]+)$'
  - name: computed
    command: "echo 'test' | wc -w"
```

### General Verification
Additional validation beyond result and output:
```yaml
verification:
  general:
    - name: check_condition
      condition: "[[ $variable -gt 0 ]]"
```

## Test Execution Integration

These test cases are integrated into the test suite through a Rust integration test that:
1. Validates YAML schema compliance
2. Generates bash scripts from YAML
3. Executes scripts and captures results
4. Verifies all assertions pass

See `tests/bash_commands_test.rs` for the integration test implementation.
