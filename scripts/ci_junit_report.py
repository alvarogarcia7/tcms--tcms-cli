#!/usr/bin/env python3
"""
Convert captured cargo test output into a JUnit XML report.

GitLab renders JUnit XML in the pipeline's test report tab when the job
declares it under artifacts:reports:junit.

Usage:
    ci_junit_report.py --output junit-rust.xml test_output.txt doc_test_output.txt

Each "Running ..." or "Doc-tests ..." header starts a testsuite. Test lines
become testcases, and the "---- <name> stdout ----" blocks become failure
messages.
"""

import argparse
import os
import re
import sys
import xml.etree.ElementTree as ET
from typing import Any

RUNNING_RE = re.compile(r'^\s*Running (?:.+?) \((.+?)\)\s*$')
DOC_TESTS_RE = re.compile(r'^\s*Doc-tests (\S+)\s*$')
TEST_RE = re.compile(r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, (.*))?$')
RESULT_RE = re.compile(r'^test result: \w+\. \d+ passed; \d+ failed; \d+ ignored;.*?finished in ([\d.]+)s')
FAILURE_HEADER_RE = re.compile(r'^---- (.+) stdout ----$')
FAILURES_END_RE = re.compile(r'^(failures:|---- .+ stdout ----|test result:)')
HASH_SUFFIX_RE = re.compile(r'-[0-9a-f]{16}$')
INVALID_XML_CHARS_RE = re.compile(r'[\x00-\x08\x0b\x0c\x0e-\x1f]')


def _new_suite(name: str) -> dict[str, Any]:
    return {'name': name, 'tests': [], 'time': '0'}


def _suite_name_from_binary(path: str) -> str:
    """Turn 'target/debug/deps/testcase_models-3f2a9b8c1d4e5f60' into 'testcase_models'."""
    return HASH_SUFFIX_RE.sub('', os.path.basename(path))


def _clean(text: str) -> str:
    return INVALID_XML_CHARS_RE.sub('', text)


def parse_cargo_output(content: str) -> list[dict[str, Any]]:
    """Return one suite per test binary or doc-test run, with its testcases."""
    suites: list[dict[str, Any]] = []
    current = _new_suite('cargo')
    failure_for: str | None = None
    failure_lines: list[str] = []
    failures: dict[str, str] = {}

    def close_failure() -> None:
        nonlocal failure_for, failure_lines
        if failure_for is not None:
            failures[failure_for] = '\n'.join(failure_lines).strip()
        failure_for = None
        failure_lines = []

    for line in content.splitlines():
        running = RUNNING_RE.match(line)
        doc_tests = DOC_TESTS_RE.match(line)
        if running or doc_tests:
            close_failure()
            suites.append(current)
            name = _suite_name_from_binary(running.group(1)) if running else f'doc-tests {doc_tests.group(1)}'
            current = _new_suite(name)
            continue

        if failure_for is not None:
            if FAILURES_END_RE.match(line):
                close_failure()
            else:
                failure_lines.append(line)
                continue

        header = FAILURE_HEADER_RE.match(line)
        if header:
            failure_for = header.group(1)
            continue

        test = TEST_RE.match(line)
        if test:
            test_name, status, reason = test.groups()
            current['tests'].append({'name': test_name, 'status': status, 'reason': reason or ''})
            continue

        result = RESULT_RE.match(line)
        if result:
            current['time'] = result.group(1)

    close_failure()
    suites.append(current)

    for suite in suites:
        for test in suite['tests']:
            test['failure'] = failures.get(test['name'], '')
    return [suite for suite in suites if suite['tests']]


def build_junit(suites: list[dict[str, Any]]) -> ET.Element:
    root = ET.Element('testsuites')
    total_tests = total_failures = total_skipped = 0
    total_time = 0.0

    for suite in suites:
        suite_el = ET.SubElement(root, 'testsuite', name=suite['name'])
        failures = skipped = 0
        for test in suite['tests']:
            case_el = ET.SubElement(
                suite_el, 'testcase', name=_clean(test['name']), classname=suite['name']
            )
            if test['status'] == 'FAILED':
                failures += 1
                message = next((ln for ln in test['failure'].splitlines() if ln.strip()), 'Test failed')
                failure_el = ET.SubElement(case_el, 'failure', message=_clean(message[:200]))
                failure_el.text = _clean(test['failure'] or 'Test failed')
            elif test['status'] == 'ignored':
                skipped += 1
                ET.SubElement(case_el, 'skipped', message=_clean(test['reason']))

        suite_time = float(suite['time'])
        suite_el.set('tests', str(len(suite['tests'])))
        suite_el.set('failures', str(failures))
        suite_el.set('skipped', str(skipped))
        suite_el.set('errors', '0')
        suite_el.set('time', f'{suite_time:.3f}')

        total_tests += len(suite['tests'])
        total_failures += failures
        total_skipped += skipped
        total_time += suite_time

    root.set('tests', str(total_tests))
    root.set('failures', str(total_failures))
    root.set('skipped', str(total_skipped))
    root.set('errors', '0')
    root.set('time', f'{total_time:.3f}')
    ET.indent(root)
    return root


def convert(input_files: list[str], output_file: str) -> int:
    suites: list[dict[str, Any]] = []
    for path in input_files:
        with open(path, 'r', encoding='utf-8', errors='replace') as f:
            suites.extend(parse_cargo_output(f.read()))

    root = build_junit(suites)
    ET.ElementTree(root).write(output_file, encoding='utf-8', xml_declaration=True)
    print(
        f'Wrote {output_file}: {root.get("tests")} tests, '
        f'{root.get("failures")} failures, {root.get("skipped")} skipped',
        file=sys.stderr,
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description='Convert cargo test output to JUnit XML')
    parser.add_argument('inputs', nargs='+', help='cargo test output files')
    parser.add_argument('--output', default='junit.xml', help='JUnit XML file to write')
    args = parser.parse_args(argv)
    return convert(args.inputs, args.output)


if __name__ == '__main__':
    sys.exit(main())
