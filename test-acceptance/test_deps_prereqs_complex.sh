#!/usr/bin/env bash
# Test script for dependencies, prerequisites, and complex test cases
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Binaries
VALIDATE_YAML="${PROJECT_ROOT}/target/debug/validate-yaml"
TEST_EXECUTOR="${PROJECT_ROOT}/target/debug/test-executor"
VERIFIER="${PROJECT_ROOT}/target/debug/verifier"

# Schema
SCHEMA_DIR="$PROJECT_ROOT/schemas"
TEST_CASE_SCHEMA="$SCHEMA_DIR/test-case.schema.json"

# Directories to test
DEPENDENCIES_DIR="$SCRIPT_DIR/00_test_cases/dependencies"
PREREQUISITES_DIR="$SCRIPT_DIR/00_test_cases/prerequisites"
COMPLEX_DIR="$SCRIPT_DIR/00_test_cases/complex"

echo "========================================"
echo "Testing Dependencies, Prerequisites, and Complex Test Cases"
echo "========================================"
echo ""

# Stage 1: Validate all YAML files
echo "=== Stage 1: Validating YAML Files ==="
echo ""

validate_dir() {
    local dir="$1"
    local dir_name=$(basename "$dir")
    echo "Validating $dir_name..."
    
    local passed=0
    local failed=0
    
    for yaml_file in "$dir"/*.yaml; do
        if [ -f "$yaml_file" ]; then
            local basename=$(basename "$yaml_file")
            if "$VALIDATE_YAML" --schema "$TEST_CASE_SCHEMA" "$yaml_file" > /dev/null 2>&1; then
                echo "  ✓ $basename"
                ((passed++))
            else
                echo "  ✗ $basename"
                ((failed++))
                "$VALIDATE_YAML" --schema "$TEST_CASE_SCHEMA" "$yaml_file" 2>&1 | sed 's/^/    /'
            fi
        fi
    done
    
    echo "  Result: $passed passed, $failed failed"
    echo ""
    
    return $failed
}

total_failures=0

validate_dir "$DEPENDENCIES_DIR" || ((total_failures++))
validate_dir "$PREREQUISITES_DIR" || ((total_failures++))
validate_dir "$COMPLEX_DIR" || ((total_failures++))

# Stage 2: Generate scripts
echo "=== Stage 2: Generating Test Scripts ==="
echo ""

SCRIPTS_DIR="$SCRIPT_DIR/scripts_test"
mkdir -p "$SCRIPTS_DIR"

generate_dir() {
    local dir="$1"
    local dir_name=$(basename "$dir")
    echo "Generating scripts for $dir_name..."
    
    local passed=0
    local failed=0
    
    for yaml_file in "$dir"/*.yaml; do
        if [ -f "$yaml_file" ]; then
            local basename=$(basename "$yaml_file" .yaml)
            local script_file="$SCRIPTS_DIR/${basename}.sh"
            
            if "$TEST_EXECUTOR" generate --json-log --test-case-dir "$SCRIPT_DIR/00_test_cases" --output "$script_file" "$yaml_file" > /dev/null 2>&1; then
                echo "  ✓ $basename.sh"
                chmod +x "$script_file"
                ((passed++))
            else
                echo "  ✗ $basename.sh"
                ((failed++))
                "$TEST_EXECUTOR" generate --json-log --test-case-dir "$SCRIPT_DIR/00_test_cases" --output "$script_file" "$yaml_file" 2>&1 | sed 's/^/    /'
            fi
        fi
    done
    
    echo "  Result: $passed passed, $failed failed"
    echo ""
    
    return $failed
}

generate_dir "$DEPENDENCIES_DIR" || ((total_failures++))
generate_dir "$PREREQUISITES_DIR" || ((total_failures++))
generate_dir "$COMPLEX_DIR" || ((total_failures++))

# Stage 3: Execute a sample of scripts (not all to avoid hanging)
echo "=== Stage 3: Executing Sample Scripts ==="
echo ""

# Execute one test from each category
execute_script() {
    local script_file="$1"
    local basename=$(basename "$script_file" .sh)
    
    echo "Executing: $basename.sh"
    
    # Run with stdin closed to avoid blocking on interactive prompts
    if bash "$script_file" < /dev/null > /dev/null 2>&1; then
        echo "  ✓ Completed"
        
        # Check if JSON log was created
        local generated_log="$(dirname "$script_file")/${basename}_execution_log.json"
        if [ -f "$generated_log" ]; then
            echo "  ✓ JSON log created"
            
            # Validate JSON syntax
            if python3 -m json.tool "$generated_log" > /dev/null 2>&1; then
                echo "  ✓ JSON is valid"
            else
                echo "  ✗ JSON is invalid"
                ((total_failures++))
            fi
        else
            echo "  ✗ JSON log not found"
            ((total_failures++))
        fi
    else
        echo "  ✗ Failed or timed out"
        ((total_failures++))
    fi
    echo ""
}

# Test one from each category
if [ -f "$SCRIPTS_DIR/TC_DEPENDENCY_SIMPLE_001.sh" ]; then
    execute_script "$SCRIPTS_DIR/TC_DEPENDENCY_SIMPLE_001.sh"
fi

if [ -f "$SCRIPTS_DIR/PREREQ_AUTO_PASS_001.sh" ]; then
    execute_script "$SCRIPTS_DIR/PREREQ_AUTO_PASS_001.sh"
fi

if [ -f "$SCRIPTS_DIR/TC_COMPLEX_BDD_HOOKS_VARS_001.sh" ]; then
    execute_script "$SCRIPTS_DIR/TC_COMPLEX_BDD_HOOKS_VARS_001.sh"
fi

# Summary
echo "========================================"
if [ $total_failures -eq 0 ]; then
    echo "SUCCESS: All tests passed"
    exit 0
else
    echo "FAILURE: $total_failures stage(s) had failures"
    exit 1
fi
