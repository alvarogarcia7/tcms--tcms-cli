#!/usr/bin/env python3
"""
Script to add envelope fields (type and schema) to all test data files.

This script updates:
- Test case YAML files: add 'type: test_case' and 'schema: tcms/test-case.schema.v1.json'
- Execution log JSON files: add envelope fields to each array element
- Verification result YAML files: add 'type: test_case_verification_result' and appropriate schema
- Container YAML files: add 'type: test_results_container' and 'schema: tcms/testcase_results_container.schema.v1.json'
"""

import sys
import json
import yaml
from pathlib import Path
from typing import Any, Dict, List


def is_verification_result(data: Dict[str, Any]) -> bool:
    """Check if the data is a verification result instead of a test case."""
    # Verification results have 'sequences' with 'step_results' containing Pass/Fail/NotExecuted
    # Test cases have 'test_sequences' with 'steps' containing 'expected'
    if 'sequences' in data and 'test_sequences' not in data:
        # Check if it has test_case_id instead of id (verification result field)
        if 'test_case_id' in data and 'id' not in data:
            return True
        # Check if sequences contain step_results
        sequences = data.get('sequences', [])
        if sequences and isinstance(sequences, list) and len(sequences) > 0:
            first_seq = sequences[0]
            if 'step_results' in first_seq:
                return True
    return False


def add_envelope_to_test_case(data: Dict[str, Any]) -> Dict[str, Any]:
    """Add envelope fields to a test case YAML structure."""
    # Create new dict with envelope fields first
    result = {}
    result['type'] = 'test_case'
    result['schema'] = 'tcms/test-case.schema.v1.json'
    
    # Copy remaining fields in original order
    for key, value in data.items():
        if key not in ['type', 'schema']:
            result[key] = value
    
    return result


def add_envelope_to_verification_result(data: Dict[str, Any]) -> Dict[str, Any]:
    """Add envelope fields to a verification result YAML structure."""
    # Create new dict with envelope fields first
    result = {}
    result['type'] = 'test_case_verification_result'
    result['schema'] = 'tcms/test-case-verification-result.schema.v1.json'
    
    # Copy remaining fields in original order
    for key, value in data.items():
        if key not in ['type', 'schema']:
            result[key] = value
    
    return result


def add_envelope_to_execution_log_entry(entry: Dict[str, Any]) -> Dict[str, Any]:
    """Add envelope fields to a single execution log entry."""
    # Create new dict with envelope fields first
    result = {}
    result['type'] = 'test_execution'
    result['schema'] = 'tcms/execution-log.schema.v1.json'
    
    # Copy remaining fields in original order
    for key, value in entry.items():
        if key not in ['type', 'schema']:
            result[key] = value
    
    return result


def add_envelope_to_container(data: Dict[str, Any]) -> Dict[str, Any]:
    """Add envelope fields to a container YAML structure."""
    # Create new dict with envelope fields first
    result = {}
    result['type'] = 'test_results_container'
    result['schema'] = 'tcms/testcase_results_container.schema.v1.json'
    
    # Copy remaining fields in original order
    for key, value in data.items():
        if key not in ['type', 'schema']:
            result[key] = value
    
    return result


def update_test_case_yaml(file_path: Path) -> bool:
    """Update a test case YAML file with envelope fields."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            data = yaml.safe_load(f)
        
        if not isinstance(data, dict):
            print(f"  Skipping {file_path}: not a dict")
            return False
        
        # Check if already has envelope fields
        if 'type' in data and 'schema' in data:
            print(f"  Skipping {file_path}: already has envelope fields")
            return False
        
        # Determine if this is a verification result or a test case
        if is_verification_result(data):
            updated_data = add_envelope_to_verification_result(data)
            print(f"  ✓ Updated {file_path} (verification result)")
        else:
            updated_data = add_envelope_to_test_case(data)
            print(f"  ✓ Updated {file_path} (test case)")
        
        # Write back to file
        with open(file_path, 'w', encoding='utf-8') as f:
            yaml.dump(updated_data, f, default_flow_style=False, sort_keys=False, allow_unicode=True)
        
        return True
    except Exception as e:
        print(f"  ✗ Error updating {file_path}: {e}")
        return False


def update_execution_log_json(file_path: Path) -> bool:
    """Update an execution log JSON file with envelope fields."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        if not isinstance(data, list):
            print(f"  Skipping {file_path}: not a list")
            return False
        
        # Check if already has envelope fields
        if data and isinstance(data[0], dict) and 'type' in data[0] and 'schema' in data[0]:
            print(f"  Skipping {file_path}: already has envelope fields")
            return False
        
        # Add envelope fields to each entry
        updated_data = [add_envelope_to_execution_log_entry(entry) for entry in data]
        
        # Write back to file with pretty formatting
        with open(file_path, 'w', encoding='utf-8') as f:
            json.dump(updated_data, f, indent=2, ensure_ascii=False)
            f.write('\n')  # Add trailing newline
        
        print(f"  ✓ Updated {file_path}")
        return True
    except Exception as e:
        print(f"  ✗ Error updating {file_path}: {e}")
        return False


def update_container_yaml(file_path: Path) -> bool:
    """Update a container YAML file with envelope fields."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            data = yaml.safe_load(f)
        
        if not isinstance(data, dict):
            print(f"  Skipping {file_path}: not a dict")
            return False
        
        # Check if already has envelope fields
        if 'type' in data and 'schema' in data:
            print(f"  Skipping {file_path}: already has envelope fields")
            return False
        
        # Add envelope fields
        updated_data = add_envelope_to_container(data)
        
        # Write back to file
        with open(file_path, 'w', encoding='utf-8') as f:
            yaml.dump(updated_data, f, default_flow_style=False, sort_keys=False, allow_unicode=True)
        
        print(f"  ✓ Updated {file_path}")
        return True
    except Exception as e:
        print(f"  ✗ Error updating {file_path}: {e}")
        return False


def main():
    """Main function to update all data files."""
    repo_root = Path(__file__).parent.parent
    
    # Statistics
    test_cases_updated = 0
    verification_results_updated = 0
    execution_logs_updated = 0
    containers_updated = 0
    
    # Update test case YAMLs
    print("\n=== Updating Test Case YAML Files ===")
    
    test_case_dirs = [
        repo_root / "testcases",
        repo_root / "test-acceptance" / "00_test_cases",
    ]
    
    for base_dir in test_case_dirs:
        if not base_dir.exists():
            continue
        
        for yaml_file in base_dir.rglob("*.yml"):
            # Skip container files
            if 'container' in yaml_file.name.lower():
                continue
            result = update_test_case_yaml(yaml_file)
            if result:
                # Check if it was updated as verification result
                with open(yaml_file, 'r', encoding='utf-8') as f:
                    data = yaml.safe_load(f)
                    if data.get('type') == 'test_case_verification_result':
                        verification_results_updated += 1
                    else:
                        test_cases_updated += 1
        
        for yaml_file in base_dir.rglob("*.yaml"):
            # Skip container files
            if 'container' in yaml_file.name.lower():
                continue
            result = update_test_case_yaml(yaml_file)
            if result:
                # Check if it was updated as verification result
                with open(yaml_file, 'r', encoding='utf-8') as f:
                    data = yaml.safe_load(f)
                    if data.get('type') == 'test_case_verification_result':
                        verification_results_updated += 1
                    else:
                        test_cases_updated += 1
    
    # Update execution log JSONs
    print("\n=== Updating Execution Log JSON Files ===")
    
    execution_log_dirs = [
        repo_root / "testcases" / "verifier_scenarios",
    ]
    
    for base_dir in execution_log_dirs:
        if not base_dir.exists():
            continue
        
        for json_file in base_dir.rglob("*execution_log.json"):
            if update_execution_log_json(json_file):
                execution_logs_updated += 1
    
    # Update container YAMLs
    print("\n=== Updating Container YAML Files ===")
    
    container_files = [
        repo_root / "testcases" / "verifier_scenarios" / "container_config.yml",
        repo_root / "testcases" / "verifier_scenarios" / "minimal_container_config.yml",
        repo_root / "testcases" / "verifier_scenarios" / "full_container_config.yml",
        repo_root / "testcases" / "expected_output_reports" / "container_data.yml",
        repo_root / "testcases" / "examples" / "expected_test_results" / "container" / "container_data.yml",
        repo_root / "examples" / "config" / "container_config.yml",
        repo_root / "examples" / "config" / "container_config_ci.yml",
        repo_root / "examples" / "config" / "container_config_full.yml",
        repo_root / "container_config.yml",
    ]
    
    for container_file in container_files:
        if container_file.exists():
            if update_container_yaml(container_file):
                containers_updated += 1
    
    # Print summary
    print("\n=== Summary ===")
    print(f"Test case YAMLs updated: {test_cases_updated}")
    print(f"Verification result YAMLs updated: {verification_results_updated}")
    print(f"Execution log JSONs updated: {execution_logs_updated}")
    print(f"Container YAMLs updated: {containers_updated}")
    print(f"Total files updated: {test_cases_updated + verification_results_updated + execution_logs_updated + containers_updated}")


if __name__ == "__main__":
    main()
