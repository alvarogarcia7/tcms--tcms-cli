#!/usr/bin/env python3
"""Regenerate the derived V&V fixtures from the recorded SUT-1 run (TCMS-47).

The derived folders under crates/tcms/tests/fixtures/vv-metrics/derived/ are produced from sut-1/ by the
edits below. The script is deterministic: running it twice gives the same bytes, and running it with
--dest into a temporary directory and comparing with the committed folders detects drift
(scripts/vv/test_vv_metrics.sh does exactly that).

Usage: generate_derived_fixtures.py [--dest DIR]   (default: the committed derived/ folder)
"""

import argparse
import os
import re
import shutil
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
FIXTURES = os.path.join(REPO, "crates", "tcms", "tests", "fixtures", "vv-metrics")
SUT = os.path.join(FIXTURES, "sut-1")
SRC_RUN = "RUN-32921"
DERIVED_STARTED = "2026-10-30T12:00:00Z"


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def set_verdicts(text, verdicts):
    for case, verdict in verdicts.items():
        text, n = re.subn(rf"(test_case_id: {case}\n  verdict: )\w+", rf"\g<1>{verdict}", text)
        if n != 1:
            raise SystemExit(f"verdict of {case} not found exactly once")
    return text


def edit_block(text, case, subs):
    """Apply (pattern, replacement, expected_count) substitutions inside one case block."""
    start = text.index(f"- test_case_id: {case}\n")
    nxt = text.find("\n- test_case_id:", start + 1)
    end = len(text) if nxt == -1 else nxt
    block = text[start:end]
    for pattern, replacement, count in subs:
        block, n = re.subn(pattern, replacement, block)
        if n != count:
            raise SystemExit(f"{case}: pattern {pattern!r} matched {n} times, expected {count}")
    return text[:start] + block + text[end:]


def make_run(dest, new_id, verdicts, block_edits):
    run_dir = os.path.join(dest, "runs", new_id)
    tr = read(os.path.join(SUT, "runs", SRC_RUN, "test-run.yaml"))
    tr = tr.replace(f"run_id: {SRC_RUN}", f"run_id: {new_id}")
    tr = re.sub(r'started_at: "[^"]+"', f'started_at: "{DERIVED_STARTED}"', tr)
    tr = re.sub(r'finished_at: "[^"]+"', f'finished_at: "{DERIVED_STARTED}"', tr)
    tr = tr.replace(SRC_RUN, new_id)
    tr = set_verdicts(tr, verdicts)
    vr = read(os.path.join(SUT, "runs", SRC_RUN, "verification-results.yaml"))
    vr = vr.replace(f"run_id: {SRC_RUN}", f"run_id: {new_id}")
    for case, subs in block_edits.items():
        vr = edit_block(vr, case, subs)
    write(os.path.join(run_dir, "test-run.yaml"), tr)
    write(os.path.join(run_dir, "verification-results.yaml"), vr)


def base(dest, name):
    folder = os.path.join(dest, name)
    if os.path.isdir(folder):
        shutil.rmtree(folder)
    os.makedirs(folder)
    for name_ in ["SUT1_PASS_001.yml", "SUT1_CALC_001.yml", "requirements.json"]:
        shutil.copy(os.path.join(SUT, name_), folder)
    shutil.copytree(os.path.join(SUT, "runs", SRC_RUN), os.path.join(folder, "runs", SRC_RUN))
    return folder


def derived_from(folder, run_id, lines):
    text = (
        f"Derived from SUT-1 run {SRC_RUN} (copied unchanged into runs/{SRC_RUN}).\n"
        f"Added run {run_id}, copied from {SRC_RUN} with these edits:\n"
    )
    text += "".join(f"- {line}\n" for line in lines)
    text += "Requirements and case files are unchanged copies.\n"
    write(os.path.join(folder, "DERIVED_FROM"), text)


FAIL_SUBS = [
    (r"overall_pass: true", "overall_pass: false", 1),
    (r"passed_steps: 3", "passed_steps: 2", 1),
    (r"failed_steps: 0", "failed_steps: 1", 1),
]
NOT_PASS_SUBS = [
    (r"overall_pass: true", "overall_pass: false", 1),
    (r"passed_steps: 3", "passed_steps: 0", 1),
    (r"not_executed_steps: 0", "not_executed_steps: 3", 1),
]
ALL_STEPS_ZERO = [
    (r"(total_steps|passed_steps|failed_steps|not_executed_steps): \d+", r"\g<1>: 0", 4),
]


def generate(dest):
    # 1. A failing run after a passing one.
    f = base(dest, "failing-run")
    make_run(f, "RUN-DERIVED-FAIL-01",
             {"SUT1_PASS_001": "fail", "SUT1_CALC_001": "pass"},
             {"SUT1_PASS_001": FAIL_SUBS})
    derived_from(f, "RUN-DERIVED-FAIL-01", [
        f"runs/RUN-DERIVED-FAIL-01/test-run.yaml: run_id, started_at and finished_at set to {DERIVED_STARTED};"
        " verdict of SUT1_PASS_001 changed from pass to fail (SUT1_CALC_001 stays pass).",
        "runs/RUN-DERIVED-FAIL-01/verification-results.yaml: SUT1_PASS_001 summary changed to"
        " overall_pass: false, passed_steps: 2, failed_steps: 1 (total_steps stays 3). Step_results are not edited.",
    ])

    # 2. Untested and skipped results only, zero steps.
    f = base(dest, "no-data-run")
    make_run(f, "RUN-DERIVED-NODATA-01",
             {"SUT1_PASS_001": "not_executed", "SUT1_CALC_001": "skipped"},
             {"SUT1_PASS_001": NOT_PASS_SUBS + ALL_STEPS_ZERO,
              "SUT1_CALC_001": NOT_PASS_SUBS + ALL_STEPS_ZERO})
    derived_from(f, "RUN-DERIVED-NODATA-01", [
        f"runs/RUN-DERIVED-NODATA-01/test-run.yaml: run_id, started_at and finished_at set to {DERIVED_STARTED};"
        " verdict of SUT1_PASS_001 changed from pass to not_executed (untested), of SUT1_CALC_001 from pass to skipped.",
        "runs/RUN-DERIVED-NODATA-01/verification-results.yaml: both cases get overall_pass: false,"
        " passed_steps: 0, not_executed_steps: 3 for SUT1_PASS_001, and all four step counters set to 0 for both cases.",
    ])

    # 3. Every result blocked (error).
    f = base(dest, "blocked-only-run")
    make_run(f, "RUN-DERIVED-BLOCKED-01",
             {"SUT1_PASS_001": "error", "SUT1_CALC_001": "error"},
             {"SUT1_PASS_001": NOT_PASS_SUBS[:2], "SUT1_CALC_001": NOT_PASS_SUBS[:2]})
    derived_from(f, "RUN-DERIVED-BLOCKED-01", [
        f"runs/RUN-DERIVED-BLOCKED-01/test-run.yaml: run_id, started_at and finished_at set to {DERIVED_STARTED};"
        " verdict of both SUT1_PASS_001 and SUT1_CALC_001 changed from pass to error (blocked).",
        "runs/RUN-DERIVED-BLOCKED-01/verification-results.yaml: both cases get overall_pass: false and"
        " passed_steps: 0 (total_steps stays 3). Step_results are not edited.",
    ])

    # 4. A skipped result sits outside the gate denominator.
    f = base(dest, "skipped-outside-denominator")
    make_run(f, "RUN-DERIVED-SKIP-01",
             {"SUT1_PASS_001": "pass", "SUT1_CALC_001": "skipped"},
             {"SUT1_CALC_001": [
                 (r"overall_pass: true", "overall_pass: false", 1),
                 (r"passed_steps: 3", "passed_steps: 0", 1),
                 (r"not_executed_steps: 0", "not_executed_steps: 3", 1),
             ]})
    derived_from(f, "RUN-DERIVED-SKIP-01", [
        f"runs/RUN-DERIVED-SKIP-01/test-run.yaml: run_id, started_at and finished_at set to {DERIVED_STARTED};"
        " verdict of SUT1_CALC_001 changed from pass to skipped (SUT1_PASS_001 stays pass).",
        "runs/RUN-DERIVED-SKIP-01/verification-results.yaml: SUT1_CALC_001 summary changed to overall_pass: false,"
        " passed_steps: 0, not_executed_steps: 3 (total_steps stays 3). Step_results are not edited.",
    ])


    # 5. A requirement with no test case citing it (not covered). Results are unchanged.
    f = base(dest, "uncovered-requirement")
    make_run(f, "RUN-DERIVED-UNCOVERED-01", {}, {})
    write(os.path.join(f, "requirements.json"), (
        '{\n'
        '  "product_id": "sut-1",\n'
        '  "requirements": [\n'
        '    { "id": "SUT1-1" },\n'
        '    { "id": "SUT1-2" },\n'
        '    { "id": "SUT1-3" }\n'
        '  ]\n'
        '}\n'
    ))
    derived_from(f, "RUN-DERIVED-UNCOVERED-01", [
        f"runs/RUN-DERIVED-UNCOVERED-01/test-run.yaml: run_id, started_at and finished_at set to {DERIVED_STARTED};"
        " verdicts unchanged (both cases pass).",
        "runs/RUN-DERIVED-UNCOVERED-01/verification-results.yaml: copied unchanged except run_id.",
        "requirements.json: requirement SUT1-3 added; no case file cites it.",
    ])

    # 6. A test case with no result in the run (untested); the other case passes.
    f = base(dest, "missing-case-result")
    make_run(f, "RUN-DERIVED-MISSING-01",
             {"SUT1_PASS_001": "not_executed", "SUT1_CALC_001": "pass"},
             {"SUT1_PASS_001": NOT_PASS_SUBS})
    derived_from(f, "RUN-DERIVED-MISSING-01", [
        f"runs/RUN-DERIVED-MISSING-01/test-run.yaml: run_id, started_at and finished_at set to {DERIVED_STARTED};"
        " verdict of SUT1_PASS_001 changed from pass to not_executed (untested), SUT1_CALC_001 stays pass.",
        "runs/RUN-DERIVED-MISSING-01/verification-results.yaml: SUT1_PASS_001 summary changed to overall_pass: false,"
        " passed_steps: 0, not_executed_steps: 3 (total_steps stays 3). Step_results are not edited.",
    ])


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dest", default=os.path.join(FIXTURES, "derived"),
                        help="directory to write the derived folders into (replaced folder by folder)")
    args = parser.parse_args()
    dest = os.path.abspath(args.dest)
    if not os.path.isdir(SUT):
        sys.exit(f"recorded SUT-1 data not found at {SUT}")
    generate(dest)
    print(f"derived fixtures written to {dest}")


if __name__ == "__main__":
    main()
