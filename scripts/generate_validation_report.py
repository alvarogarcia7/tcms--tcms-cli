#!/usr/bin/env python3
"""
Script: generate_validation_report.py
Purpose: Generate a comprehensive validation report for all test case YAML files.
         Categorizes failures by error type and documents common error patterns.
"""

import subprocess
import sys
import re
from pathlib import Path
from datetime import datetime
from collections import defaultdict
from typing import List, Dict, Optional, Any

# Configuration - Using canonical schema path
# Legacy schema (backward compatible): schemas/tcms-schemas/tcms/test-case-main.schema.json
# Versioned schema (recommended): schemas/tcms-schemas/tcms/test-case-main.schema.json
SCHEMA_FILE = "schemas/tcms-schemas/tcms/test-case-main.schema.json"
VALIDATE_YAML_BIN = "./target/debug/validate-yaml"
REPORT_DIR = "reports"
REPORT_FILE = f"{REPORT_DIR}/validation_report.txt"


class ValidationResult:
    """Container for validation results of a single file."""
    
    def __init__(self, filepath: str, passed: bool, errors: Optional[List[str]] = None):
        self.filepath = filepath
        self.passed = passed
        self.errors = errors or []
        self.error_categories = self._categorize_errors()
    
    def _categorize_errors(self) -> Dict[str, List[str]]:
        """Categorize errors by type."""
        categories = defaultdict(list)
        
        for error in self.errors:
            if "is a required property" in error:
                categories["missing_required_fields"].append(error)
            elif "does not match" in error or "is not valid" in error:
                categories["invalid_field_values"].append(error)
            elif "Additional properties are not allowed" in error:
                categories["additional_properties"].append(error)
            elif "must be" in error or "should be" in error:
                categories["type_mismatch"].append(error)
            elif "Failed to parse YAML" in error or "YAML syntax" in error:
                categories["malformed_yaml"].append(error)
            elif "oneOf" in error or "anyOf" in error or "allOf" in error:
                categories["schema_constraint_violations"].append(error)
            else:
                categories["other"].append(error)
        
        return dict(categories)


def discover_test_files() -> List[str]:
    """Discover all test case YAML files."""
    try:
        # Find all YAML files in testcases, tests/sample, data, and test-acceptance/00_test_cases directories
        result = subprocess.run(
            [
                "find", "testcases", "tests/sample", "data", "test-acceptance/00_test_cases",
                "-type", "f",
                "(", "-name", "*.yml", "-o", "-name", "*.yaml", ")"
            ],
            capture_output=True,
            text=True,
            check=False
        )
        
        # Patterns to exclude (these are not test case files)
        exclude_patterns = [
            "te.y",  # Malformed test files
            "wrong",  # Intentionally wrong test files
            "tests/sample/",  # Test fixture files (intentionally invalid for testing validator)
            "sample_test_runs.yaml",  # Test run metadata
            "/expected_test_results/test_case_result/",  # Expected test result samples
            "/expected_test_results/container/",  # Expected container samples
            "/expected_output_reports/",  # Expected output reports
            "/testcase_results_container/",  # Test results container samples
            "container_config.yml",  # Container configuration files
            "container_data.yml",  # Container data files
            "_report.yaml",  # Report files
        ]
        
        files = []
        for line in result.stdout.strip().split('\n'):
            if not line:
                continue
            
            # Skip files matching any exclude pattern
            if any(pattern in line for pattern in exclude_patterns):
                continue
            
            files.append(line.strip())
        
        return sorted(files)
    except Exception as e:
        print(f"Error discovering files: {e}", file=sys.stderr)
        return []


def validate_file(filepath: str) -> ValidationResult:
    """Validate a single YAML file against the schema."""
    try:
        result = subprocess.run(
            [VALIDATE_YAML_BIN, "--schema", SCHEMA_FILE, filepath],
            capture_output=True,
            text=True,
            timeout=10
        )
        
        if result.returncode == 0:
            return ValidationResult(filepath, True)
        else:
            # Parse error messages from output
            errors = parse_error_output(result.stdout + result.stderr)
            return ValidationResult(filepath, False, errors)
    
    except subprocess.TimeoutExpired:
        return ValidationResult(filepath, False, ["Validation timeout (>10s)"])
    except Exception as e:
        return ValidationResult(filepath, False, [f"Validation error: {str(e)}"])


def parse_error_output(output: str) -> List[str]:
    """Parse error messages from validate-yaml output."""
    errors = []
    
    # Look for schema constraint violations section
    if "Schema constraint violations:" in output:
        # Extract all error entries
        error_pattern = r"Error #\d+:.*?(?=Error #\d+:|Summary:|$)"
        matches = re.findall(error_pattern, output, re.DOTALL)
        
        for match in matches:
            # Clean up the error text
            error_text = match.strip()
            if error_text:
                errors.append(error_text)
    
    # If no structured errors found, capture the whole output
    if not errors and output.strip():
        errors.append(output.strip())
    
    return errors


def analyze_error_patterns(results: List[ValidationResult]) -> Dict[str, Any]:
    """Analyze error patterns across all failed validations."""
    pattern_analysis: Dict[str, Any] = {
        "total_errors": 0,
        "by_category": defaultdict(int),
        "by_type": defaultdict(int),
        "common_missing_fields": defaultdict(int),
        "common_constraints": defaultdict(int)
    }
    
    for result in results:
        if not result.passed:
            pattern_analysis["total_errors"] += len(result.errors)
            
            for category, errors in result.error_categories.items():
                pattern_analysis["by_category"][category] += len(errors)
            
            for error in result.errors:
                # Extract missing field names
                if "is a required property" in error:
                    match = re.search(r'"([^"]+)" is a required property', error)
                    if match:
                        field = match.group(1)
                        pattern_analysis["common_missing_fields"][field] += 1
                
                # Extract constraint types
                if "Constraint:" in error:
                    match = re.search(r'Constraint: (.+?)(?:\n|$)', error)
                    if match:
                        constraint = match.group(1).strip()
                        pattern_analysis["common_constraints"][constraint] += 1
    
    return pattern_analysis


def generate_report(results: List[ValidationResult], pattern_analysis: Dict) -> str:
    """Generate comprehensive validation report."""
    passed = [r for r in results if r.passed]
    failed = [r for r in results if not r.passed]
    
    report_lines = []
    report_lines.append("=" * 80)
    report_lines.append("                    Test Case Validation Report")
    report_lines.append("=" * 80)
    report_lines.append("")
    report_lines.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report_lines.append(f"Schema: {SCHEMA_FILE}")
    report_lines.append("")
    
    # Summary Section
    report_lines.append("=" * 80)
    report_lines.append("                              Summary")
    report_lines.append("=" * 80)
    report_lines.append("")
    report_lines.append(f"Total files validated: {len(results)}")
    report_lines.append(f"Passed: {len(passed)}")
    report_lines.append(f"Failed: {len(failed)}")
    report_lines.append("")
    
    if len(failed) == 0:
        report_lines.append("✓ All test case files passed validation!")
        report_lines.append("")
    else:
        # Error Categorization
        report_lines.append("=" * 80)
        report_lines.append("                        Failures by Error Type")
        report_lines.append("=" * 80)
        report_lines.append("")
        
        category_names = {
            "missing_required_fields": "Missing Required Fields",
            "invalid_field_values": "Invalid Field Values",
            "schema_constraint_violations": "Schema Constraint Violations",
            "type_mismatch": "Type Mismatch Errors",
            "malformed_yaml": "Malformed YAML Syntax",
            "additional_properties": "Additional/Unknown Properties",
            "other": "Other Errors"
        }
        
        for category, count in sorted(pattern_analysis["by_category"].items(), 
                                       key=lambda x: x[1], reverse=True):
            if count > 0:
                category_name = category_names.get(category, category)
                report_lines.append(f"  {category_name}: {count} errors")
        
        report_lines.append("")
        
        # Common Error Patterns
        report_lines.append("=" * 80)
        report_lines.append("                        Common Error Patterns")
        report_lines.append("=" * 80)
        report_lines.append("")
        
        if pattern_analysis["common_missing_fields"]:
            report_lines.append("Most Common Missing Required Fields:")
            for field, count in sorted(pattern_analysis["common_missing_fields"].items(),
                                       key=lambda x: x[1], reverse=True)[:10]:
                report_lines.append(f"  - '{field}': missing in {count} file(s)")
            report_lines.append("")
        
        if pattern_analysis["common_constraints"]:
            report_lines.append("Most Common Schema Constraints Violated:")
            for constraint, count in sorted(pattern_analysis["common_constraints"].items(),
                                           key=lambda x: x[1], reverse=True)[:10]:
                # Truncate long constraints
                constraint_display = constraint[:70] + "..." if len(constraint) > 70 else constraint
                report_lines.append(f"  - {constraint_display}")
                report_lines.append(f"    (violated in {count} error(s))")
            report_lines.append("")
        
        # Failed Files Detail
        report_lines.append("=" * 80)
        report_lines.append("                           Failed Files")
        report_lines.append("=" * 80)
        report_lines.append("")
        
        for result in failed:
            report_lines.append(f"✗ {result.filepath}")
            report_lines.append("")
            
            if result.error_categories:
                report_lines.append("  Error Categories:")
                for category, errors in result.error_categories.items():
                    category_name = category_names.get(category, category)
                    report_lines.append(f"    - {category_name}: {len(errors)} error(s)")
                report_lines.append("")
            
            report_lines.append("  Error Details:")
            report_lines.append("  " + "-" * 76)
            for i, error in enumerate(result.errors, 1):
                # Format multi-line errors with proper indentation
                error_lines = error.split('\n')
                for j, line in enumerate(error_lines):
                    if j == 0:
                        report_lines.append(f"  {line}")
                    else:
                        report_lines.append(f"    {line}")
            report_lines.append("")
            
            report_lines.append("  To validate this file manually, run:")
            report_lines.append(f"    {VALIDATE_YAML_BIN} --schema {SCHEMA_FILE} {result.filepath}")
            report_lines.append("")
            report_lines.append("  " + "-" * 76)
            report_lines.append("")
    
    # Validation Results Detail
    report_lines.append("=" * 80)
    report_lines.append("                        Validation Results Detail")
    report_lines.append("=" * 80)
    report_lines.append("")
    
    for result in results:
        status = "✓" if result.passed else "✗"
        report_lines.append(f"{status} {result.filepath}")
        if not result.passed and result.errors:
            for error in result.errors[:2]:  # Show first 2 errors
                error_preview = error.split('\n')[0][:70]
                report_lines.append(f"    {error_preview}...")
    
    report_lines.append("")
    report_lines.append("=" * 80)
    report_lines.append("                           End of Report")
    report_lines.append("=" * 80)
    
    return '\n'.join(report_lines)


def main():
    """Main entry point."""
    print("[INFO] Starting test case validation report generation...")
    print(f"[INFO] Schema file: {SCHEMA_FILE}")
    print(f"[INFO] Validate-yaml binary: {VALIDATE_YAML_BIN}")
    print()
    
    # Check prerequisites
    if not Path(SCHEMA_FILE).exists():
        print(f"[ERROR] Schema file not found: {SCHEMA_FILE}", file=sys.stderr)
        sys.exit(1)
    
    if not Path(VALIDATE_YAML_BIN).exists():
        print(f"[ERROR] validate-yaml binary not found: {VALIDATE_YAML_BIN}", file=sys.stderr)
        print("[INFO] Please build it first: cargo build --bin validate-yaml", file=sys.stderr)
        sys.exit(1)
    
    # Discover test files
    print("[INFO] Discovering test case files...")
    test_files = discover_test_files()
    
    if not test_files:
        print("[ERROR] No test case files found to validate", file=sys.stderr)
        sys.exit(1)
    
    print(f"[INFO] Found {len(test_files)} test case files")
    print()
    
    # Validate each file
    print("[INFO] Validating files...")
    results = []
    for i, filepath in enumerate(test_files, 1):
        print(f"  [{i}/{len(test_files)}] {filepath}", end="")
        result = validate_file(filepath)
        results.append(result)
        status = " ✓" if result.passed else " ✗"
        print(status)
    
    print()
    
    # Analyze error patterns
    print("[INFO] Analyzing error patterns...")
    pattern_analysis = analyze_error_patterns(results)
    
    # Generate report
    print("[INFO] Generating report...")
    report_content = generate_report(results, pattern_analysis)
    
    # Create reports directory
    Path(REPORT_DIR).mkdir(exist_ok=True)
    
    # Write report to file
    with open(REPORT_FILE, 'w') as f:
        f.write(report_content)
    
    print(f"[INFO] Detailed report saved to: {REPORT_FILE}")
    print()
    
    # Display summary
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed)
    
    print("=" * 80)
    print("                              Summary")
    print("=" * 80)
    print(f"Total files validated: {len(results)}")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")
    print()
    
    if failed > 0:
        print("[ERROR] Validation failed for {} file(s)".format(failed), file=sys.stderr)
        return 1
    else:
        print("[INFO] All validations passed successfully!")
        return 0


if __name__ == "__main__":
    sys.exit(main())
