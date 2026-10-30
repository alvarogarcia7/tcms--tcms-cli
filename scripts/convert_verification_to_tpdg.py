#!/usr/bin/env python3.14
"""
Convert verification results to TPDG (Test Plan Data Generator) container format.

This script supports three modes:

1. Direct conversion from verifier JSON (--input):
   - Reads verifier JSON output (ContainerReport, BatchVerificationReport, or TestCaseVerificationResult)
   - Converts to TPDG container YAML format
   - Supports both internally-tagged (status field) and externally-tagged formats

2. Test case + execution log mode (--test-case-dir + --logs-dir):
   - Scans for test case YAML files (type: test_case)
   - Matches with execution log JSON files
   - Builds verification results from scratch
   - Converts to TPDG container YAML format

3. Stdin mode:
   - Reads JSON from stdin when --input is omitted
   - Useful for piping verification results

The TPDG container format includes:
- type: test_results_container
- test_results: array of test case results with Pass/Fail/NotExecuted step variants
- metadata: test execution metadata

Features:
- Automatic format detection (internally-tagged vs externally-tagged)
- Optional schema validation with validate-yaml binary
- Custom metadata (title, project, test_date)
- Recursive directory scanning
"""

import sys
import json
import argparse
import tempfile
import subprocess
from pathlib import Path
from typing import Any, Optional
from datetime import datetime, timezone

try:
    import yaml
    YAML_AVAILABLE = True
except ImportError:
    YAML_AVAILABLE = False
    print("Error: PyYAML is required. Install with: pip3 install pyyaml")
    sys.exit(1)


def find_validate_yaml_binary() -> Optional[str]:
    """
    Find the validate-yaml binary, checking in order:
    1. target/release/validate-yaml
    2. target/debug/validate-yaml
    3. validate-yaml in PATH
    
    Returns:
        Path to validate-yaml binary, or None if not found
    """
    release_path = Path("target/release/validate-yaml")
    if release_path.exists() and release_path.is_file():
        return str(release_path)
    
    debug_path = Path("target/debug/validate-yaml")
    if debug_path.exists() and debug_path.is_file():
        return str(debug_path)
    
    try:
        result = subprocess.run(
            ["which", "validate-yaml"],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    except Exception:
        pass
    
    return None


def validate_test_result(
    test_result: dict[str, Any],
    validate_yaml_bin: str,
    schema_path: str,
    verbose: bool = False
) -> bool:
    """
    Validate a single test result against the schema.
    
    Args:
        test_result: Test result dictionary
        validate_yaml_bin: Path to validate-yaml binary
        schema_path: Path to schema file
        verbose: Whether to print verbose output
        
    Returns:
        True if validation passes, False otherwise
    """
    with tempfile.NamedTemporaryFile(mode='w', suffix='.yaml', delete=False) as tmp:
        yaml.dump(test_result, tmp, default_flow_style=False, sort_keys=False, allow_unicode=True)
        tmp_path = tmp.name
    
    try:
        result = subprocess.run(
            [validate_yaml_bin, "--schema", schema_path, tmp_path],
            capture_output=True,
            text=True,
            check=False
        )
        
        if verbose:
            if result.stdout:
                print(result.stdout, end='')
            if result.stderr:
                print(result.stderr, end='', file=sys.stderr)
        
        return result.returncode == 0
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def transform_step_result(step_result: dict[str, Any]) -> dict[str, Any]:
    """
    Transform a step result from internally-tagged to externally-tagged format.
    
    Input (internally-tagged):
      {"status": "pass", "step": 1, "description": "..."}
      {"status": "fail", "step": 1, "description": "...", "expected": {...}, ...}
      {"status": "not_executed", "step": 1, "description": "..."}
    
    Output (externally-tagged):
      {"Pass": {"step": 1, "description": "..."}}
      {"Fail": {"step": 1, "description": "...", "expected": {...}, ...}}
      {"NotExecuted": {"step": 1, "description": "..."}}
    
    If already externally-tagged, returns as-is.
    
    Args:
        step_result: Step result dictionary with 'status' field or tag
        
    Returns:
        Externally-tagged step result dictionary
    """
    # Check if already externally-tagged
    if any(key in step_result for key in ['Pass', 'Fail', 'NotExecuted']):
        return step_result
    
    # Handle internally-tagged format
    status = step_result.get("status", "").lower()
    
    if not status:
        # No status field, return as-is
        return step_result
    
    # Create a copy without the status field
    step_data = {k: v for k, v in step_result.items() if k != "status"}
    
    # Map status to externally-tagged variant
    if status == "pass":
        return {"Pass": step_data}
    elif status == "fail":
        return {"Fail": step_data}
    elif status == "not_executed":
        return {"NotExecuted": step_data}
    else:
        # Unknown status, return as-is
        return step_result


def parse_test_case_yaml(yaml_path: Path) -> Optional[dict[str, Any]]:
    """
    Parse a test case YAML file and extract relevant metadata.
    
    Args:
        yaml_path: Path to the test case YAML file
        
    Returns:
        Dictionary with test case metadata or None if not a test_case
    """
    try:
        with open(yaml_path, 'r', encoding='utf-8') as f:
            data = yaml.safe_load(f)
        
        if not isinstance(data, dict) or data.get('type') != 'test_case':
            return None
        
        result = {
            'id': data.get('id'),
            'description': data.get('description', ''),
            'requirement': data.get('requirement'),
            'item': data.get('item'),
            'tc': data.get('tc'),
            'test_sequences': []
        }
        
        for seq in data.get('test_sequences', []):
            seq_data = {
                'id': seq.get('id'),
                'name': seq.get('name', ''),
                'steps': []
            }
            
            for step in seq.get('steps', []):
                step_data = {
                    'step': step.get('step'),
                    'description': step.get('description', ''),
                    'expected': step.get('expected', {})
                }
                seq_data['steps'].append(step_data)
            
            result['test_sequences'].append(seq_data)
        
        return result
    except Exception as e:
        print(f"Warning: Failed to parse {yaml_path}: {e}")
        return None


def parse_execution_log(log_path: Path) -> list[dict[str, Any]]:
    """
    Parse an execution log JSON file.
    
    Args:
        log_path: Path to the execution log JSON file
        
    Returns:
        List of execution log entries
    """
    try:
        with open(log_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        if isinstance(data, list):
            return data
        return []
    except Exception as e:
        print(f"Warning: Failed to parse {log_path}: {e}")
        return []


def read_actual_log_file(logs_dir: Path, test_case_id: str, sequence_id: int, step_num: int) -> Optional[str]:
    """
    Read the actual output log file for a specific step.
    
    Args:
        logs_dir: Directory containing log files
        test_case_id: Test case ID
        sequence_id: Sequence ID
        step_num: Step number
        
    Returns:
        Content of the actual log file or None if not found
    """
    actual_log_path = logs_dir / f"{test_case_id}_sequence-{sequence_id}_step-{step_num}.actual.log"
    if actual_log_path.exists():
        try:
            return actual_log_path.read_text(encoding='utf-8')
        except Exception as e:
            print(f"Warning: Failed to read {actual_log_path}: {e}")
    return None


def build_step_result(
    test_case: dict[str, Any],
    sequence_id: int,
    step_num: int,
    execution_entry: Optional[dict[str, Any]],
    logs_dir: Optional[Path]
) -> dict[str, Any]:
    """
    Build a step result (Pass/Fail/NotExecuted) from test case and execution data.
    
    Args:
        test_case: Parsed test case metadata
        sequence_id: Sequence ID
        step_num: Step number
        execution_entry: Execution log entry for this step (or None if not executed)
        logs_dir: Directory containing actual log files (optional)
        
    Returns:
        Step result dictionary with variant (Pass/Fail/NotExecuted)
    """
    step_def = None
    for seq in test_case['test_sequences']:
        if seq['id'] == sequence_id:
            for step in seq['steps']:
                if step['step'] == step_num:
                    step_def = step
                    break
            break
    
    if not step_def:
        return {
            "NotExecuted": {
                "step": step_num,
                "description": f"Step {step_num} (not defined in test case)"
            }
        }
    
    if not execution_entry:
        return {
            "NotExecuted": {
                "step": step_num,
                "description": step_def['description']
            }
        }
    
    result_pass = execution_entry.get('result_verification_pass', False)
    output_pass = execution_entry.get('output_verification_pass', False)
    
    if result_pass and output_pass:
        return {
            "Pass": {
                "step": step_num,
                "description": step_def['description']
            }
        }
    else:
        expected = step_def.get('expected', {})
        
        actual_output = None
        if logs_dir:
            actual_output = read_actual_log_file(logs_dir, test_case['id'], sequence_id, step_num)
        if actual_output is None:
            actual_output = execution_entry.get('output', '')
        
        reasons = []
        if not result_pass:
            reasons.append("result verification failed")
        if not output_pass:
            reasons.append("output verification failed")
        reason = " and ".join(reasons)
        
        return {
            "Fail": {
                "step": step_num,
                "description": step_def['description'],
                "expected": {
                    "success": expected.get('success'),
                    "result": str(expected.get('result', '')),
                    "output": expected.get('output', '')
                },
                "actual_result": str(execution_entry.get('exit_code', '')),
                "actual_output": actual_output,
                "reason": reason
            }
        }


def build_verification_result_from_files(
    test_case: dict[str, Any],
    execution_log: list[dict[str, Any]],
    logs_dir: Optional[Path]
) -> dict[str, Any]:
    """
    Build a complete TestCaseVerificationResult from test case and execution log.
    
    Args:
        test_case: Parsed test case metadata
        execution_log: List of execution log entries
        logs_dir: Directory containing actual log files (optional)
        
    Returns:
        TestCaseVerificationResult dictionary
    """
    exec_lookup = {}
    for entry in execution_log:
        seq_id = entry.get('test_sequence')
        step_num = entry.get('step')
        if seq_id is not None and step_num is not None:
            exec_lookup[(seq_id, step_num)] = entry
    
    sequences = []
    total_steps = 0
    passed_steps = 0
    failed_steps = 0
    not_executed_steps = 0
    
    for seq_def in test_case['test_sequences']:
        seq_id = seq_def['id']
        step_results = []
        seq_all_passed = True
        
        for step_def in seq_def['steps']:
            step_num = step_def['step']
            exec_entry = exec_lookup.get((seq_id, step_num))
            
            step_result = build_step_result(test_case, seq_id, step_num, exec_entry, logs_dir)
            step_results.append(step_result)
            
            total_steps += 1
            if "Pass" in step_result:
                passed_steps += 1
            elif "Fail" in step_result:
                failed_steps += 1
                seq_all_passed = False
            elif "NotExecuted" in step_result:
                not_executed_steps += 1
                seq_all_passed = False
        
        sequence = {
            "sequence_id": seq_id,
            "name": seq_def['name'],
            "step_results": step_results,
            "all_steps_passed": seq_all_passed
        }
        sequences.append(sequence)
    
    result = {
        "test_case_id": test_case['id'],
        "description": test_case['description'],
        "sequences": sequences,
        "total_steps": total_steps,
        "passed_steps": passed_steps,
        "failed_steps": failed_steps,
        "not_executed_steps": not_executed_steps,
        "overall_pass": failed_steps == 0 and not_executed_steps == 0
    }
    
    if test_case.get('requirement') is not None:
        result['requirement'] = test_case['requirement']
    if test_case.get('item') is not None:
        result['item'] = test_case['item']
    if test_case.get('tc') is not None:
        result['tc'] = test_case['tc']
    
    return result


def scan_test_cases(test_case_dir: Path, recursive: bool = False) -> list[Path]:
    """
    Scan directory for test case YAML files.
    
    Args:
        test_case_dir: Directory to scan
        recursive: Whether to scan subdirectories recursively
        
    Returns:
        List of test case YAML file paths
    """
    patterns = ['*.yaml', '*.yml']
    files = []
    
    for pattern in patterns:
        if recursive:
            files.extend(test_case_dir.rglob(pattern))
        else:
            files.extend(test_case_dir.glob(pattern))
    
    return sorted(files)


def process_test_case_directory(
    test_case_dir: Path,
    logs_dir: Path,
    recursive: bool = False,
    verbose: bool = False
) -> list[dict[str, Any]]:
    """
    Process test case directory and build verification results.
    
    Args:
        test_case_dir: Directory containing test case YAML files
        logs_dir: Directory containing execution log files
        recursive: Whether to scan subdirectories recursively
        verbose: Whether to print verbose output
        
    Returns:
        List of TestCaseVerificationResult dictionaries
    """
    test_results = []
    
    yaml_files = scan_test_cases(test_case_dir, recursive)
    
    if verbose:
        print(f"Found {len(yaml_files)} YAML files to process")
    
    for yaml_path in yaml_files:
        test_case = parse_test_case_yaml(yaml_path)
        if not test_case:
            continue
        
        test_case_id = test_case['id']
        if verbose:
            print(f"Processing test case: {test_case_id}")
        
        log_path = logs_dir / f"{test_case_id}_execution_log.json"
        if not log_path.exists():
            if verbose:
                print(f"  Error: No execution log found at {log_path}")
            # Still process it, but all steps will be NotExecuted
            execution_log = []
        else:
            execution_log = parse_execution_log(log_path)
            if verbose:
                print(f"  Found execution log with {len(execution_log)} entries")
        
        result = build_verification_result_from_files(test_case, execution_log, logs_dir)
        test_results.append(result)
    
    return test_results


def convert_test_case_to_tpdg(test_case: dict[str, Any]) -> dict[str, Any]:
    """
    Convert a TestCaseVerificationResult to TPDG test result format.
    
    This function handles both internally-tagged and externally-tagged formats,
    ensuring all step results are in externally-tagged format.
    
    Args:
        test_case: TestCaseVerificationResult dictionary
        
    Returns:
        TPDG test result dictionary
    """
    tpdg_result = {
        "test_case_id": test_case["test_case_id"],
        "description": test_case["description"],
        "sequences": [],
        "total_steps": test_case["total_steps"],
        "passed_steps": test_case["passed_steps"],
        "failed_steps": test_case["failed_steps"],
        "not_executed_steps": test_case["not_executed_steps"],
        "overall_pass": test_case["overall_pass"]
    }
    
    if "requirement" in test_case and test_case["requirement"] is not None:
        tpdg_result["requirement"] = test_case["requirement"]
    if "item" in test_case and test_case["item"] is not None:
        tpdg_result["item"] = test_case["item"]
    if "tc" in test_case and test_case["tc"] is not None:
        tpdg_result["tc"] = test_case["tc"]
    
    for sequence in test_case.get("sequences", []):
        seq_result = {
            "sequence_id": sequence["sequence_id"],
            "name": sequence["name"],
            "step_results": [],
            "all_steps_passed": sequence["all_steps_passed"]
        }
        
        for field in ["requirement", "item", "tc"]:
            if field in sequence and sequence[field] is not None:
                seq_result[field] = sequence[field]
        
        for step_result in sequence.get("step_results", []):
            transformed_step = transform_step_result(step_result)
            seq_result["step_results"].append(transformed_step)
        
        tpdg_result["sequences"].append(seq_result)
    
    return tpdg_result


def create_tpdg_container(
    test_results: list[dict[str, Any]],
    title: Optional[str] = None,
    project: Optional[str] = None,
    test_date: Optional[str] = None
) -> dict[str, Any]:
    """
    Create a TPDG container structure from test results.
    
    Args:
        test_results: List of TestCaseVerificationResult dictionaries
        title: Optional title for the container
        project: Optional project name
        test_date: Optional test date (ISO 8601 format)
        
    Returns:
        TPDG container dictionary
    """
    tpdg_results = [convert_test_case_to_tpdg(tc) for tc in test_results]
    
    total_test_cases = len(test_results)
    passed_test_cases = sum(1 for tc in test_results if tc.get("overall_pass", False))
    failed_test_cases = total_test_cases - passed_test_cases
    
    if title is None:
        title = f"Test Results - {datetime.now().strftime('%Y-%m-%d')}"
    
    if project is None:
        project = "Test Suite Execution"
    
    if test_date is None:
        test_date = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    
    # Using canonical schema path for versioned container schema
    container = {
        "type": "test_results_container",
        "schema": "tcms/test-results-container.schema.v1.json",
        "title": title or f"Test Results - {datetime.now().strftime('%Y-%m-%d')}",
        "project": project or "Test Suite Execution",
        "test_date": datetime.now().strftime('%Y-%m-%dT%H:%M:%SZ'),
        "test_results": tpdg_results,
        "metadata": {
            "execution_duration": 0.0,
            "total_test_cases": total_test_cases,
            "passed_test_cases": passed_test_cases,
            "failed_test_cases": failed_test_cases
        }
    }
    
    return container


def parse_verifier_json(data: dict[str, Any]) -> list[dict[str, Any]]:
    """
    Parse verifier JSON and extract test case results.
    
    Args:
        data: Parsed JSON data
        
    Returns:
        List of TestCaseVerificationResult dictionaries
    """
    if "test_results" in data:
        return data["test_results"]
    elif "test_cases" in data:
        return data["test_cases"]
    elif "test_case_id" in data:
        return [data]
    else:
        raise ValueError("Unknown JSON structure. Expected ContainerReport, BatchVerificationReport, or TestCaseVerificationResult")


def process_verifier_json(
    input_path: Path,
    verbose: bool = False
) -> list[dict[str, Any]]:
    """
    Process verifier JSON input.
    
    Args:
        input_path: Path to input JSON file
        verbose: Whether to print verbose output
        
    Returns:
        List of TestCaseVerificationResult dictionaries
    """
    if verbose:
        print(f"Reading verifier JSON: {input_path}")
    
    try:
        with open(input_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        raise ValueError(f"Failed to parse JSON: {e}")
    except Exception as e:
        raise ValueError(f"Failed to read file: {e}")
    
    test_results = parse_verifier_json(data)
    
    if verbose:
        print(f"Extracted {len(test_results)} test case(s)")
    
    return test_results


def write_tpdg_yaml(container: dict[str, Any], output_path: Path, verbose: bool = False) -> None:
    """
    Write TPDG container to YAML file.
    
    Args:
        container: TPDG container dictionary
        output_path: Path to output YAML file
        verbose: Whether to print verbose output
    """
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', encoding='utf-8') as f:
        yaml.dump(container, f, default_flow_style=False, sort_keys=False, allow_unicode=True)
    
    if verbose:
        print(f"✓ Wrote TPDG container to: {output_path}")


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Convert verification results to TPDG container format",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Convert from verifier JSON
  %(prog)s --input verification.json --output container.yaml

  # Convert from stdin
  cat verification.json | %(prog)s --output container.yaml

  # Build from test cases and execution logs
  %(prog)s --test-case-dir testcases/ --logs-dir logs/ --output container.yaml

  # Recursive scan with custom metadata
  %(prog)s --test-case-dir testcases/ --logs-dir logs/ --recursive \\
           --output container.yaml --title "Q1 2024 Results" --project "My Project"

  # Convert with validation
  %(prog)s --input verification.json --output container.yaml --validate

Modes:
  1. Verifier JSON mode (--input):
     Reads existing verifier JSON output and converts to TPDG format.
     Supports both internally-tagged (status field) and externally-tagged formats.
  
  2. Test case directory mode (--test-case-dir + --logs-dir):
     Scans test case YAML files, matches with execution logs, builds
     verification results, and converts to TPDG format.

  3. Stdin mode:
     Reads JSON from stdin when --input is omitted.
        """
    )
    
    input_group = parser.add_mutually_exclusive_group(required=False)
    input_group.add_argument(
        '--input',
        type=Path,
        help='Input verifier JSON file (ContainerReport, BatchVerificationReport, or TestCaseVerificationResult)'
    )
    input_group.add_argument(
        '--test-case-dir',
        type=Path,
        help='Directory containing test case YAML files'
    )
    
    parser.add_argument(
        '--logs-dir',
        type=Path,
        help='Directory containing execution log JSON files (required with --test-case-dir)'
    )
    parser.add_argument(
        '--recursive',
        action='store_true',
        help='Recursively scan subdirectories in --test-case-dir and --logs-dir'
    )
    
    parser.add_argument(
        '--output',
        type=Path,
        required=True,
        help='Output TPDG container YAML file'
    )
    
    parser.add_argument(
        '--title',
        type=str,
        help='Title for the TPDG container (default: auto-generated)'
    )
    parser.add_argument(
        '--project',
        type=str,
        help='Project name for the TPDG container (default: "Test Suite Execution")'
    )
    parser.add_argument(
        '--test-date',
        type=str,
        help='Test date in ISO 8601 format (default: current UTC time)'
    )
    
    parser.add_argument(
        '--validate',
        action='store_true',
        help='Validate each test result against schema (requires validate-yaml binary)'
    )
    parser.add_argument(
        '--schema',
        type=str,
        default='schemas/tcms-schemas/tcms/verification-result.schema.json',
        help='Schema file for validation (default: schemas/tcms-schemas/tcms/verification-result.schema.json - legacy schema, canonical: schemas/tcms-schemas/tcms/verification-result.schema.json)'
    )
    
    parser.add_argument(
        '-v', '--verbose',
        action='store_true',
        help='Enable verbose output'
    )
    
    args = parser.parse_args()
    
    if args.test_case_dir and not args.logs_dir:
        parser.error("--logs-dir is required when using --test-case-dir")
    
    if not args.input and not args.test_case_dir:
        if sys.stdin.isatty():
            parser.error("either --input or --test-case-dir is required (or provide JSON via stdin)")
    
    try:
        if args.input:
            if not args.input.exists():
                print(f"✗ Error: Input file not found: {args.input}")
                return 1
            
            test_results = process_verifier_json(args.input, args.verbose)
        elif args.test_case_dir:
            if not args.test_case_dir.exists():
                print(f"✗ Error: Test case directory not found: {args.test_case_dir}")
                return 1
            if not args.logs_dir.exists():
                print(f"✗ Error: Logs directory not found: {args.logs_dir}")
                return 1
            
            test_results = process_test_case_directory(
                args.test_case_dir,
                args.logs_dir,
                args.recursive,
                args.verbose
            )
        else:
            if args.verbose:
                print("Reading input from stdin...")
            
            stdin_content = sys.stdin.read()
            data = json.loads(stdin_content)
            test_results = parse_verifier_json(data)
            
            if args.verbose:
                print(f"Extracted {len(test_results)} test case(s)")
        
        if not test_results:
            print("✗ Warning: No test results found")
        
        if args.validate:
            validate_yaml_bin = find_validate_yaml_binary()
            if not validate_yaml_bin:
                print("✗ Error: validate-yaml binary not found", file=sys.stderr)
                print("  Build it with: cargo build --bin validate-yaml", file=sys.stderr)
                return 1
            
            schema_path = Path(args.schema)
            if not schema_path.exists():
                print(f"✗ Error: Schema file not found: {args.schema}", file=sys.stderr)
                return 1
            
            if args.verbose:
                print(f"Using validate-yaml: {validate_yaml_bin}")
                print(f"Using schema: {args.schema}")
            
            print(f"Validating {len(test_results)} test result(s)...")
            validation_failed = False
            
            for i, test_case in enumerate(test_results, 1):
                test_case_id = test_case.get("test_case_id", f"test_{i}")
                
                if args.verbose:
                    print(f"\nValidating test case {i}/{len(test_results)}: {test_case_id}")
                
                is_valid = validate_test_result(
                    test_case,
                    validate_yaml_bin,
                    str(schema_path),
                    args.verbose
                )
                
                if is_valid:
                    print(f"  ✓ {test_case_id}: validation passed")
                else:
                    print(f"  ✗ {test_case_id}: validation failed", file=sys.stderr)
                    validation_failed = True
            
            if validation_failed:
                print("\n✗ Validation failed for one or more test cases", file=sys.stderr)
                return 1
            
            print(f"\n✓ All {len(test_results)} test result(s) validated successfully")
        
        container = create_tpdg_container(
            test_results,
            title=args.title,
            project=args.project,
            test_date=args.test_date
        )
        
        write_tpdg_yaml(container, args.output, args.verbose)
        
        print(f"\n✓ Successfully generated TPDG container with {len(test_results)} test case(s)")
        print(f"  Output: {args.output}")
        
        if args.verbose:
            print(f"  Title: {container['title']}")
            print(f"  Project: {container['project']}")
            print(f"  Test Date: {container['test_date']}")
            print(f"  Passed: {container['metadata']['passed_test_cases']}")
            print(f"  Failed: {container['metadata']['failed_test_cases']}")
        
        return 0
        
    except json.JSONDecodeError as e:
        print(f"\n✗ Error: Failed to parse JSON: {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        return 1
    except Exception as e:
        print(f"\n✗ Error: {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        return 1


if __name__ == '__main__':
    sys.exit(main())
