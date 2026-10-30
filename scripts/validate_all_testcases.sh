#!/usr/bin/env bash
set -e

# Get script directory and source logger
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

# Configuration
SCHEMA_FILE="${SCHEMA_FILE:-schemas/tcms-schemas/tcms/test-case-main.schema.json}"
OUTPUT_FILE="${OUTPUT_FILE:-testcase_validation_report.txt}"
BACKLOG_FILE="${BACKLOG_FILE:-backlog.md}"
USE_MCP="${USE_MCP:-true}"

# Counters
total_files=0
passed_files=0
failed_files=0
declare -a failed_file_list

log_info "Starting validation of all test case files..."
log_info "Schema file: $SCHEMA_FILE"
echo ""

# Check if schema exists
if [ ! -f "$SCHEMA_FILE" ]; then
    log_error "Schema file not found: $SCHEMA_FILE"
    exit 1
fi

# Create temporary file for results
temp_results=$(mktemp)
setup_cleanup "$temp_results"

# Find all YAML files in testcases, test-acceptance, tests/sample directories
# Exclude files with specific patterns that are known to be invalid or test data
log_info "Discovering test case files..."

test_case_files=$(find testcases test-acceptance tests/sample -type f \( -name "*.yml" -o -name "*.yaml" \) 2>/dev/null | \
    grep -v "te\.y" | \
    grep -v "sample_test_runs\.yaml" | \
    grep -v "_wrong\." | \
    grep -v "/incorrect/" | \
    sort)

if [ -z "$test_case_files" ]; then
    log_warning "No test case files found to validate"
    exit 0
fi

log_info "Found $(echo "$test_case_files" | wc -l | tr -d ' ') test case files"
echo ""

# Validate each file
cargo build --bin validate-yaml

while IFS= read -r file; do
    if [ -z "$file" ]; then
        continue
    fi
    
    total_files=$((total_files + 1))

    VERIFIER=${VERIFIER:-./target/debug/validate-yaml}
    
    # Run validation using validate-yaml binary
    if  ${VERIFIER} --schema "$SCHEMA_FILE" "$file" > /dev/null 2>&1; then
        echo "$(pass "$file")" >> "$temp_results"
        passed_files=$((passed_files + 1))
    else
        # Capture detailed error
        error_output=$(${VERIFIER} --schema "$SCHEMA_FILE" "$file" 2>&1 || true)
        echo "$(fail "$file")" >> "$temp_results"
        echo "  Error details:" >> "$temp_results"
        echo "$error_output" | sed 's/^/    /' >> "$temp_results"
        echo "" >> "$temp_results"
        failed_files=$((failed_files + 1))
        failed_file_list+=("$file")
    fi
done <<< "$test_case_files"

# Display results
echo ""
section "Validation Results"
cat "$temp_results"

# Summary
echo ""
section "Summary"
log_info "Total files validated: $total_files"
pass "Passed: $passed_files"
fail "Failed: $failed_files"

# Save detailed report to file
{
    echo "# Test Case Validation Report"
    echo "Generated: $(date)"
    echo ""
    echo "## Summary"
    echo "- Total files validated: $total_files"
    echo "- Passed: $passed_files"
    echo "- Failed: $failed_files"
    echo ""
    echo "## Validation Results"
    echo ""
    cat "$temp_results" | sed 's/\x1b\[[0-9;]*m//g'  # Strip color codes
} > "$OUTPUT_FILE"

log_info ""
log_info "Detailed report saved to: $OUTPUT_FILE"

# Function to create MCP task for a failed file
create_mcp_task() {
    local file_path="$1"
    local task_id="$2"
    
    # Extract directory and filename
    local dir=$(dirname "$file_path")
    local filename=$(basename "$file_path")
    
    # Get current date in yyyy-mm-dd format
    local created_date=$(date +%Y-%m-%d)
    
    # Create task file in backlog/tasks/
    local task_file="backlog/tasks/TCMS-${task_id}: Fix validation for ${filename}.md"
    
    cat > "$task_file" << EOF
---
id: TCMS-${task_id}
title: Fix validation for ${filename}
status: To Do
assignee: []
created_date: '${created_date}'
labels:
  - validation
  - test-case
  - schema
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
This test case file failed schema validation and needs to be fixed.

**File:** \`${file_path}\`

**Directory:** \`${dir}\`

### Validation Error

Run the following command to see the validation error:

\`\`\`bash
cargo run --bin validate-yaml -- --schema schemas/tcms-schemas/tcms/test-case-main.schema.json ${file_path}
\`\`\`

### How to Fix

1. Run the validation command above to see specific error messages
2. Review the schema at \`schemas/tcms-schemas/tcms/test-case-main.schema.json\`
3. Fix the YAML file to conform to the schema
4. Re-run validation to verify the fix:
   \`\`\`bash
   cargo run --bin validate-yaml -- --schema schemas/tcms-schemas/tcms/test-case-main.schema.json ${file_path}
   \`\`\`
5. Once fixed, run full validation:
   \`\`\`bash
   make validate-testcases-report
   \`\`\`

### Common Issues

- Missing required properties (e.g., \`test_sequences\`)
- Invalid types (e.g., string instead of integer)
- oneOf constraint violations in initial_conditions
- Missing required fields in test steps

### Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 File passes schema validation
- [ ] #2 No validation errors when running validate-yaml
- [ ] #3 Full validation suite passes (make validate-testcases-report)
<!-- DOD:END -->

<!-- SECTION:DESCRIPTION:END -->
EOF

    log_info "Created MCP task: TCMS-${task_id} for ${file_path}"
}

# Create backlog.md with failed files and optionally create MCP tasks
if [ $failed_files -gt 0 ]; then
    {
        echo "# Test Case Validation Backlog"
        echo ""
        echo "This file tracks test case YAML files that need to be fixed to pass schema validation."
        echo ""
        echo "**Generated:** $(date)"
        echo ""
        echo "**Total Failed Files:** $failed_files"
        echo ""
        echo "## How to Validate"
        echo ""
        echo "To validate all test cases, run:"
        echo "\`\`\`bash"
        echo "make build"
        echo "./scripts/validate_all_testcases.sh"
        echo "\`\`\`"
        echo ""
        echo "To validate a specific file:"
        echo "\`\`\`bash"
        echo "cargo run --bin validate-yaml -- --schema schemas/tcms-schemas/tcms/test-case-main.schema.json <file_path>"
        echo "\`\`\`"
        echo ""
        echo "## Failed Files"
        echo ""
        
        # Group by directory for better organization
        current_dir=""
        for file in "${failed_file_list[@]}"; do
            dir=$(dirname "$file")
            if [ "$dir" != "$current_dir" ]; then
                current_dir="$dir"
                echo ""
                echo "### $dir"
                echo ""
            fi
            echo "- [ ] \`$file\`"
        done
        
        echo ""
        echo "## Validation Details"
        echo ""
        echo "For detailed error messages, see \`$OUTPUT_FILE\`"
        echo ""
        echo "## Next Steps"
        echo ""
        echo "1. Review each failed file in the list above"
        echo "2. Run validation on individual files to see specific error messages"
        echo "3. Fix the schema violations in each file"
        echo "4. Re-run validation to verify fixes"
        echo "5. Check off completed items in the list above"
        echo ""
    } > "$BACKLOG_FILE"
    
    log_info "Backlog file created: $BACKLOG_FILE"
    
    # Create MCP tasks if enabled
    if [ "$USE_MCP" = "true" ]; then
        log_info ""
        section "Creating MCP Tasks"
        
        # Get the highest existing TCMS task number
        highest_task=0
        if [ -d "backlog/tasks" ]; then
            for task_file in backlog/tasks/TCMS-*.md; do
                if [ -f "$task_file" ]; then
                    # Extract number from filename like "TCMS-12: Title.md"
                    task_num=$(basename "$task_file" | sed 's/TCMS-\([0-9]*\).*/\1/')
                    if [ "$task_num" -gt "$highest_task" ]; then
                        highest_task=$task_num
                    fi
                fi
            done
        fi
        
        log_info "Highest existing task number: TCMS-$highest_task"
        next_task=$((highest_task + 1))
        
        # Create a task for each failed file
        task_count=0
        for file in "${failed_file_list[@]}"; do
            create_mcp_task "$file" "$next_task"
            next_task=$((next_task + 1))
            task_count=$((task_count + 1))
        done
        
        log_info ""
        pass "Created $task_count MCP tasks in backlog/tasks/"
        log_info "Tasks numbered from TCMS-$((highest_task + 1)) to TCMS-$((highest_task + task_count))"
    fi
    
    log_info ""
    pass "Validation complete (with failures)"
    
    exit 1
else
    # No failures, create empty backlog or remove it
    {
        echo "# Test Case Validation Backlog"
        echo ""
        echo "**Generated:** $(date)"
        echo ""
        echo "✅ All test cases are valid! No items in backlog."
        echo ""
    } > "$BACKLOG_FILE"
    
    log_info "Backlog file updated: $BACKLOG_FILE"
    log_info ""
    pass "All test cases validated successfully!"
    
    exit 0
fi
