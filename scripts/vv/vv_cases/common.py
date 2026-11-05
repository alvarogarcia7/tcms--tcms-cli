import json
import re
import shutil
from pathlib import Path

REPO = Path("/home/orca/tcms-cli")
CASES = REPO / "crates/tcms/tests/fixtures/vv-cases"
TEMPLATE_CASE = (REPO / "crates/tcms/tests/fixtures/vv-metrics/sut-1/SUT1_PASS_001.yml").read_text()
SCHEMAS = str(REPO / "schemas")

GATE_RE = re.compile(r"^\d+$")


def reset(binary):
    d = CASES / binary
    if d.exists():
        shutil.rmtree(d)
    d.mkdir(parents=True)
    return d


def put(base: Path, rel: str, text: str):
    p = base / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)


def run_yaml(run, results_block=None):
    """test-run.yaml text. run: dict with id, started, finished, plan, build, results."""
    lines = [
        "type: test_run",
        "schema: tcms/test-run.schema.json",
        f"run_id: {run.get('run_id', run['id'])}",
        f"build_id: {run.get('build', 'b-1')}",
        f"environment: {run.get('env', 'Bench A')}",
        f"tester: {run.get('tester', 'tester-1')}",
        f'started_at: "{run["started"]}"',
    ]
    if run.get("finished"):
        lines.append(f'finished_at: "{run["finished"]}"')
    if run.get("plan") is not None:
        lines.append(f'plan_id: "{run["plan"]}"')
    if not run["results"]:
        lines.append("results: []")
    else:
        lines.append("results:")
        for c, v, prp in run["results"]:
            lines.append(f"- test_case_id: {c}")
            lines.append(f"  verdict: {v}")
            if prp:
                lines.append(f"  problem_report_id: {prp}")
    return "\n".join(lines) + "\n"


def verif_yaml(run):
    lines = [
        "type: test_results_container",
        "schema: tcms/test-results-container.schema.v1.json",
        "title: Verification results",
        "project: VV cases",
        f'test_date: "{run["started"]}"',
        "test_results:",
    ]
    for c, _, _ in run["results"]:
        if c not in run["steps"]:
            continue
        t, p = run["steps"][c]
        lines += [f"- test_case_id: {c}", f"  total_steps: {t}", f"  passed_steps: {p}"]
    return "\n".join(lines) + "\n"


def case_file(cid, req):
    text = TEMPLATE_CASE.replace("id: SUT1_PASS_001", f"id: {cid}").replace(
        "requirement: SUT1-2", f"requirement: {req}")
    assert f"id: {cid}" in text and f"requirement: {req}" in text
    return text


def write_case(base: Path, binary, args, expect_exit, expect, readme_title, what, extra_notes=""):
    base.mkdir(parents=True, exist_ok=True)
    (base / "case.json").write_text(json.dumps({
        "binary": binary, "args": args, "expect_exit": expect_exit, "expect": expect,
    }, indent=2, sort_keys=True) + "\n")
    lines = [f"# {readme_title}", "", what, ""]
    if expect_exit == 0:
        lines.append("**Expected:** exit 0.")
    else:
        lines.append(f"**Expected:** exit {expect_exit} (refused; nothing valid on stdout).")
    if extra_notes:
        lines += ["", extra_notes]
    lines += ["", "Arguments and expected values are in `case.json`; see the index `../README.md`.", ""]
    (base / "README.md").write_text("\n".join(lines))
