#!/usr/bin/env python3
"""
Parse Rust test output and generate JSON results for CI/CD reporting.
JUnit XML for the GitLab test report is produced by ci_junit_report.py.
"""

import sys
import re
import json
from typing import Any


def parse_test_output(output_file: str) -> dict[str, Any]:
    """Parse cargo test output and extract test results."""
    
    with open(output_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    results = {
        'passed': 0,
        'failed': 0,
        'ignored': 0,
        'total': 0,
        'test_cases': [],
        'failures': [],
        'duration': '0s'
    }
    
    test_pattern = r'test ([\w:]+(?:::\w+)*) \.\.\. (\w+)'
    for match in re.finditer(test_pattern, content):
        test_name, status = match.groups()
        results['test_cases'].append({
            'name': test_name,
            'status': status
        })
        results['total'] += 1
        
        if status == 'ok':
            results['passed'] += 1
        elif status == 'FAILED':
            results['failed'] += 1
            results['failures'].append(test_name)
        elif status == 'ignored':
            results['ignored'] += 1
    
    summary_pattern = r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; \d+ filtered out; finished in ([\d.]+s)'
    summary_match = re.search(summary_pattern, content)
    if summary_match:
        results['duration'] = summary_match.group(5)
    
    return results


def main():
    if len(sys.argv) < 2:
        print("Usage: parse_test_results.py <test_output_file>", file=sys.stderr)
        sys.exit(1)
    
    output_file = sys.argv[1]
    results = parse_test_output(output_file)
    
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
