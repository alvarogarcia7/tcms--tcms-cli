#!/usr/bin/env python3
"""Tests for ci_junit_report.py (cargo test output -> JUnit XML)."""

import xml.etree.ElementTree as ET

import ci_junit_report as report

CARGO_OUTPUT = """\
   Compiling testcase-models v0.1.0 (/work/crates/testcase-models)
     Running unittests src/lib.rs (/home/ci/target/debug/deps/testcase_models-0123456789abcdef)

running 3 tests
test models::step::parses ... ok
test models::step::rejects_empty ... FAILED
test models::step::slow_path ... ignored, needs database

failures:

---- models::step::rejects_empty stdout ----
thread 'models::step::rejects_empty' panicked at src/step.rs:42:9:
assertion `left == right` failed
  left: 1
 right: 2

failures:
    models::step::rejects_empty

test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/cli.rs (/home/ci/target/debug/deps/cli-fedcba9876543210)

running 1 tests
test runs_ok ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s

   Doc-tests testcase_models

running 1 tests
test src/lib.rs - parse (line 7) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.50s
"""


def _parse(xml_text: str) -> ET.Element:
    return ET.fromstring(xml_text)


def _convert(tmp_path, content: str) -> ET.Element:
    source = tmp_path / "test_output.txt"
    source.write_text(content, encoding="utf-8")
    output = tmp_path / "junit.xml"
    assert report.main([str(source), "--output", str(output)]) == 0
    return ET.parse(output).getroot()


def test_parse_builds_one_suite_per_binary_and_doc_tests():
    suites = report.parse_cargo_output(CARGO_OUTPUT)

    assert [suite["name"] for suite in suites] == [
        "testcase_models",
        "cli",
        "doc-tests testcase_models",
    ]
    assert [suite["time"] for suite in suites] == ["0.02", "0.31", "1.50"]
    statuses = [test["status"] for test in suites[0]["tests"]]
    assert statuses == ["ok", "FAILED", "ignored"]


def test_parse_captures_failure_output_and_ignore_reason():
    suites = report.parse_cargo_output(CARGO_OUTPUT)
    failing = suites[0]["tests"][1]
    ignored = suites[0]["tests"][2]

    assert failing["name"] == "models::step::rejects_empty"
    assert failing["failure"].startswith("thread 'models::step::rejects_empty' panicked at src/step.rs:42:9:")
    assert "left: 1" in failing["failure"]
    assert ignored["reason"] == "needs database"


def test_convert_writes_expected_counts_and_elements(tmp_path):
    root = _convert(tmp_path, CARGO_OUTPUT)

    assert root.tag == "testsuites"
    assert root.get("tests") == "5"
    assert root.get("failures") == "1"
    assert root.get("skipped") == "1"
    assert root.get("errors") == "0"

    cases = root.findall("./testsuite/testcase")
    failed = [case for case in cases if case.find("failure") is not None]
    assert len(failed) == 1
    assert failed[0].get("name") == "models::step::rejects_empty"
    assert failed[0].get("classname") == "testcase_models"
    assert failed[0].find("failure").get("message") == "thread 'models::step::rejects_empty' panicked at src/step.rs:42:9:"
    assert "left: 1" in failed[0].find("failure").text

    skipped = [case for case in cases if case.find("skipped") is not None]
    assert len(skipped) == 1
    assert skipped[0].find("skipped").get("message") == "needs database"


def test_convert_escapes_markup_in_names_and_failure_text(tmp_path):
    content = """\
     Running unittests src/lib.rs (/t/deps/escape-0123456789abcdef)

running 1 tests
test parses <tag> & "quotes" ... FAILED

failures:

---- parses <tag> & "quotes" stdout ----
expected <a> & "b"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"""
    root = _convert(tmp_path, content)

    case = root.find("./testsuite/testcase")
    assert case.get("name") == 'parses <tag> & "quotes"'
    assert case.find("failure").text.strip() == 'expected <a> & "b"'


def test_convert_strips_control_characters(tmp_path):
    content = (
        "     Running unittests src/lib.rs (/t/deps/ctrl-0123456789abcdef)\n\n"
        "running 1 tests\n"
        "test noisy ... FAILED\n\n"
        "failures:\n\n"
        "---- noisy stdout ----\n"
        "bad\x1bchar\x07here\n\n"
        "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"
    )
    root = _convert(tmp_path, content)

    text = root.find("./testsuite/testcase/failure").text
    assert "badcharhere" in text


def test_empty_input_produces_valid_empty_report(tmp_path):
    root = _convert(tmp_path, "No crates to test - skipping tests\n")

    assert root.tag == "testsuites"
    assert root.get("tests") == "0"
    assert root.findall("testsuite") == []


def test_failure_without_captured_output_uses_default_message(tmp_path):
    content = (
        "     Running unittests src/lib.rs (/t/deps/bare-0123456789abcdef)\n\n"
        "running 1 tests\n"
        "test quiet ... FAILED\n\n"
        "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"
    )
    root = _convert(tmp_path, content)

    failure = root.find("./testsuite/testcase/failure")
    assert failure.get("message") == "Test failed"
    assert failure.text == "Test failed"


def test_multiple_inputs_are_merged(tmp_path):
    first = tmp_path / "test_output.txt"
    second = tmp_path / "doc_test_output.txt"
    first.write_text(CARGO_OUTPUT, encoding="utf-8")
    second.write_text("   Doc-tests other\n\nrunning 1 tests\ntest src/a.rs - a (line 1) ... ok\n\n"
                      "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s\n",
                      encoding="utf-8")
    output = tmp_path / "junit.xml"

    assert report.main([str(first), str(second), "--output", str(output)]) == 0

    root = ET.parse(output).getroot()
    assert root.get("tests") == "6"
    suite_names = [suite.get("name") for suite in root.findall("testsuite")]
    assert suite_names[-1] == "doc-tests other"
