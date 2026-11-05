"""Renders and validates the traceability documents (AsciiDoc and Markdown) of every VV case fixture.

For each case folder under crates/tcms/tests/fixtures/vv-cases/<binary>/<case>/ that carries test results:
  1. build the test-results container: `combine` folders use their per-case results; the other run-based folders
     use the newest recorded run (test-run.yaml, with the step counts of verification-results.yaml when present);
  2. validate the container against the sut-1 container schema;
  3. render with tpdg and the sut-1 templates, in the same order as sut-1 scripts/traceability-document.sh:
     test cases, traceability matrix, then the outer document, for each format;
  4. validate: tpdg exits 0; no template syntax left; a traceability matrix; a PASS or FAIL verdict;
     the AsciiDoc compiles with asciidoctor and no warning.

`report` folders hold a metrics JSON and no test cases, so they have nothing to render and are listed as skipped.

Usage:
  render_fixtures.py --tpdg TPDG --templates SUT1_TEMPLATES --asciidoctor ASCIIDOCTOR --out OUT [--only BINARY]

Outputs go to OUT (not into the repository): OUT/<binary>/<case>/{document.adoc, document.md, container.yaml,
validation.json}, and OUT/summary.json.
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import yaml
from jsonschema import validate

REPO = Path(__file__).resolve().parents[3]
FIXTURES = REPO / "crates/tcms/tests/fixtures/vv-cases"
TEST_DATE = "2026-11-05T00:00:00Z"
TEMPLATE_RE = re.compile(r"\{\{|\}\}|\{%|%\}")
RUN_BINARIES = ("metrics", "history", "trends")


def load(path: Path):
    return yaml.safe_load(path.read_text())


def case_requirements(folder: Path) -> dict:
    """test_case_id -> requirement, from the test case files at the folder root."""
    out = {}
    for f in folder.glob("*.yml"):
        doc = load(f)
        if isinstance(doc, dict) and doc.get("type") == "test_case":
            out[doc.get("id")] = doc.get("requirement", "UNLINKED")
    return out


def steps_of(verdict: str, total: int, passed, failed_hint=None):
    """(total, passed, failed, not_executed) for one result."""
    if passed is None:
        passed = total if verdict == "pass" else 0
    not_exec = total if verdict in ("not_executed", "untested", "skipped") else 0
    failed = max(total - passed - not_exec, 0)
    return total, passed, failed, not_exec


def step_results(total, passed, failed):
    out = []
    for i in range(1, total + 1):
        if i <= passed:
            out.append({"Pass": {"step": i, "description": f"step {i}"}})
        elif i <= passed + failed:
            out.append({"Fail": {"step": i, "description": f"step {i}", "reason": "failed"}})
        else:
            out.append({"NotExecuted": {"step": i, "description": f"step {i}"}})
    return out


def normalize_steps(result: dict) -> dict:
    """The sut-1 templates read these fields of a failed step; fill the ones a fixture leaves out with empty
    defaults, as the verifier writes them for a failed step with no recorded output."""
    for seq in result.get("sequences", []):
        for sr in seq.get("step_results", []):
            fail = sr.get("Fail")
            if fail is not None:
                fail.setdefault("reason", "")
                fail.setdefault("actual_output", "")
                fail.setdefault("actual_result", "")
                fail.setdefault("expected", {"success": True, "output": "", "result": ""})
                fail.setdefault("description", "")
    return result


def results_from_run(folder: Path, binary: str):
    """Test results of the newest recorded run of a run-based folder, or None when there is none."""
    runs = []
    for d in sorted((folder / "runs").glob("*")):
        f = d / "test-run.yaml"
        if f.exists():
            doc = load(f)
            if doc.get("results"):
                runs.append((str(doc.get("started_at")), d.name, d, doc))
    if not runs:
        return None
    runs.sort(key=lambda r: (r[0], r[1]))
    _, _, d, doc = runs[-1]
    steps = {}
    vr = d / "verification-results.yaml"
    if vr.exists():
        for t in (load(vr) or {}).get("test_results", []) or []:
            steps[t["test_case_id"]] = (t.get("total_steps"), t.get("passed_steps"))
    reqs = case_requirements(folder)
    out = []
    for r in doc["results"]:
        cid, verdict = r["test_case_id"], r["verdict"]
        total0, passed0 = steps.get(cid, (None, None))
        total, passed, failed, ne = steps_of(verdict, total0 or 1, passed0)
        sr = step_results(total, passed, failed)
        out.append(normalize_steps({
            "test_case_id": cid,
            "description": f"VV fixture {binary}/{folder.name}: verdict {verdict}",
            "requirement": reqs.get(cid, "UNLINKED"),
            "sequences": [{"sequence_id": 1, "name": "VV fixture steps",
                           "step_results": sr, "all_steps_passed": verdict == "pass"}],
            "total_steps": total, "passed_steps": passed, "failed_steps": failed,
            "not_executed_steps": ne, "overall_pass": verdict == "pass",
        }))
    return out


def results_from_combine(folder: Path):
    per_case = sorted(folder.glob("runs/*/per-case/*_result.yaml"))
    if not per_case:
        return None
    out = []
    for f in per_case:
        doc = load(f)
        doc.pop("type", None)
        doc.pop("schema", None)
        doc.setdefault("requirement", "UNLINKED")
        out.append(normalize_steps(doc))
    return out


def build_container(binary: str, folder: Path, results):
    passed = sum(1 for r in results if r["overall_pass"])
    return {
        "type": "test_results_container",
        "schema": "tcms/test-results-container.schema.v1.json",
        "title": f"VV fixture {binary}/{folder.name}",
        "project": "tcms-cli VV fixtures",
        "test_date": TEST_DATE,
        "test_results": results,
        "metadata": {
            "environment": "VV fixture",
            "platform": "linux",
            "executor": "tcms-cli fixtures",
            "total_test_cases": len(results),
            "passed_test_cases": passed,
            "failed_test_cases": len(results) - passed,
        },
    }


def tpdg(bin_path, args, cwd):
    p = subprocess.run([str(bin_path), *map(str, args)], cwd=cwd, capture_output=True, text=True)
    return p.returncode == 0, (p.stdout + p.stderr).strip()


def render(tpdg_bin, templates: Path, container: dict, fmt: str, work: Path):
    """The sut-1 pipeline's render_document, for one format. Returns (ok, document text or error)."""
    work.mkdir(parents=True, exist_ok=True)
    suffix = f"{fmt}.j2"
    vm = templates / "verification_methods"
    tr = templates / "test_results"
    container_schema = tr / "container_schema.json"
    results_dir = work / "results"
    results_dir.mkdir(exist_ok=True)
    for r in container["test_results"]:
        (results_dir / f"{r['test_case_id']}.yml").write_text(
            yaml.safe_dump({"type": "result", **r}, sort_keys=False))
    ok, msg = tpdg(tpdg_bin, ["--multiple-by-type", "type", vm, *sorted(results_dir.glob("*.yml")),
                              "--format", fmt, "--output", work / f"test_cases.{fmt}"], work)
    if not ok:
        return False, f"test_cases: {msg}"
    reqs = sorted({r.get("requirement", "UNLINKED") for r in container["test_results"]})
    matrix_input = {**container, "requirements": reqs}
    (work / "matrix_input.json").write_text(json.dumps(matrix_input))
    ok, msg = tpdg(tpdg_bin, ["--single", container_schema, tr / f"traceability_matrix.{suffix}",
                              work / "matrix_input.json", "--format", fmt,
                              "--output", work / f"traceability_matrix.{fmt}"], work)
    if not ok:
        return False, f"traceability_matrix: {msg}"
    document = {**container,
                "test_cases_md": (work / f"test_cases.{fmt}").read_text(),
                "traceability_matrix_md": (work / f"traceability_matrix.{fmt}").read_text()}
    (work / "document.json").write_text(json.dumps(document))
    ok, msg = tpdg(tpdg_bin, ["--single", container_schema, tr / f"test_container_result.{suffix}",
                              work / "document.json", "--format", fmt, "--output", work / f"document.{fmt}"], work)
    if not ok:
        return False, f"document: {msg}"
    return True, (work / f"document.{fmt}").read_text()


def check_text(text: str, fmt: str) -> list:
    problems = []
    if not text.strip():
        problems.append("empty document")
    if TEMPLATE_RE.search(text):
        problems.append("template syntax left over")
    if "Traceability Matrix" not in text and "traceability" not in text.lower():
        problems.append("no traceability matrix")
    if not re.search(r"PASS|FAIL", text):
        problems.append("no PASS or FAIL verdict")
    return problems


def asciidoc_warnings(asciidoctor: str, path: Path) -> str:
    p = subprocess.run([asciidoctor, "-S", "safe", "-o", "/dev/null", str(path)],
                       capture_output=True, text=True)
    if p.returncode != 0:
        return f"exit {p.returncode}: {p.stderr.strip()[:300]}"
    return p.stderr.strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tpdg", required=True)
    ap.add_argument("--templates", required=True, type=Path, help="sut-1 vv-tcms/templates folder")
    ap.add_argument("--asciidoctor", default="asciidoctor")
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--only", default=None, help="one binary folder, e.g. combine")
    args = ap.parse_args()
    schema = json.loads((args.templates / "test_results/container_schema.json").read_text())
    args.out.mkdir(parents=True, exist_ok=True)
    summary = {}
    for bdir in sorted(FIXTURES.iterdir()):
        if not bdir.is_dir() or (args.only and bdir.name != args.only):
            continue
        counts = {"folders": 0, "rendered": 0, "valid": 0, "skipped": 0, "failed": 0}
        for folder in sorted(bdir.iterdir()):
            if not (folder / "case.json").exists():
                continue
            counts["folders"] += 1
            out_dir = args.out / bdir.name / folder.name
            out_dir.mkdir(parents=True, exist_ok=True)
            expect_exit = json.loads((folder / "case.json").read_text()).get("expect_exit", 0)
            if bdir.name == "report":
                reason = "report fixture: metrics JSON only, no test cases"
            elif expect_exit != 0:
                reason = f"refusal fixture (the command exits {expect_exit}): no successful run to document"
            else:
                reason = None
            results = None
            if reason is None:
                try:
                    results = results_from_combine(folder) if bdir.name == "combine" else results_from_run(folder, bdir.name)
                except (yaml.YAMLError, OSError, KeyError, TypeError) as e:
                    # An unreadable input is a validation failure of this folder, not a crash of the run.
                    counts["failed"] += 1
                    msg = f"input does not load: {str(e).splitlines()[0][:200]}"
                    (out_dir / "validation.json").write_text(json.dumps({"problems": [msg]}, indent=2))
                    print(f"FAIL {bdir.name}/{folder.name}: {msg}")
                    continue
                if not results:
                    reason = "no test results in the fixture"
            if reason is not None:
                counts["skipped"] += 1
                (out_dir / "validation.json").write_text(json.dumps({"skipped": reason}, indent=2))
                continue
            container = build_container(bdir.name, folder, results)
            (out_dir / "container.yaml").write_text(yaml.safe_dump(container, sort_keys=False))
            problems = []
            try:
                validate(container, schema)
            except Exception as e:  # noqa: BLE001 - report every validation error as text
                problems.append(f"container invalid: {str(e).splitlines()[0]}")
            docs = {}
            for fmt in ("adoc", "md"):
                ok, text = render(args.tpdg, args.templates, container, fmt, out_dir / f"work-{fmt}")
                if not ok:
                    problems.append(f"{fmt} render failed: {text[:300]}")
                    continue
                docs[fmt] = text
                (out_dir / f"document.{fmt}").write_text(text)
                problems += [f"{fmt}: {p}" for p in check_text(text, fmt)]
            if "adoc" in docs:
                warn = asciidoc_warnings(args.asciidoctor, out_dir / "document.adoc")
                if warn:
                    problems.append(f"adoc compile: {warn[:300]}")
            for w in out_dir.glob("work-*"):
                shutil.rmtree(w, ignore_errors=True)
            (out_dir / "validation.json").write_text(json.dumps(
                {"test_cases": len(results), "problems": problems}, indent=2))
            counts["rendered"] += 1
            if problems:
                counts["failed"] += 1
                print(f"FAIL {bdir.name}/{folder.name}: {problems[0][:200]}")
            else:
                counts["valid"] += 1
        summary[bdir.name] = counts
        print(bdir.name, counts, flush=True)
    (args.out / "summary.json").write_text(json.dumps(summary, indent=2))
    failed = sum(c["failed"] for c in summary.values())
    print(f"TOTAL failed {failed}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
