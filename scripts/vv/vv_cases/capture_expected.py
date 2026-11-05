"""Capture the binary output of every vv case as expected output, and verify it.

For each case folder under crates/tcms/tests/fixtures/vv-cases/<binary>/<case>/:
  1. run the binary in a fresh copy and record exit code, stdout, stderr and every file it created;
  2. run it again in another fresh copy and require byte-identical output (equivalence: deterministic);
  3. compare the output with the independent reference expectation in case.json (correctness; run_vv_cases.run_case);
  4. when 1-3 agree, write the output to <case>/expected/ (canonical JSON, sorted keys, 2-space indent);
     when a case diverges from the reference and is listed in KNOWN_DIVERGENCES, do NOT capture the buggy output,
     write expected/KNOWN_DIVERGENCE.txt instead; any other divergence is reported and not captured.

Usage: capture_expected.py [--write]    (without --write it only verifies and prints the report)
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import run_vv_cases as rv  # noqa: E402
from common import CASES  # noqa: E402

# Cases whose binary output differs from the reference (none left after the D-03/D-08 fixes).
KNOWN_DIVERGENCES = {}
SKIP = {"case.json", "README.md", "expected"}


def run_in_fresh(binary, case_dir, tag):
    spec = json.loads((case_dir / "case.json").read_text())
    work = rv.WORK / binary / case_dir.name / tag
    if work.exists():
        shutil.rmtree(work)
    shutil.copytree(case_dir, work, ignore=shutil.ignore_patterns(*SKIP))
    p = subprocess.run(rv.argv_for(binary, spec["args"]), cwd=work, capture_output=True, text=True)
    created = {}
    for f in sorted(work.rglob("*")):
        if f.is_file():
            rel = f.relative_to(work).as_posix()
            if not (case_dir / rel).exists():
                created[rel] = f.read_bytes()
    return {"exit": p.returncode, "stdout": p.stdout, "stderr": p.stderr, "files": created}


def canonical(out):
    """Byte form of a run: JSON stdout re-serialised canonically, everything else as-is.
    The schema root path is replaced by <SCHEMAS> so the output does not depend on the checkout location."""
    from common import SCHEMAS
    parts = {"exit.txt": f"{out['exit']}\n".encode()}
    stdout = out["stdout"].replace(SCHEMAS, "<SCHEMAS>")
    out = {**out, "stderr": out["stderr"].replace(SCHEMAS, "<SCHEMAS>")}
    try:
        parts["stdout.json"] = (json.dumps(json.loads(stdout), indent=2, sort_keys=True) + "\n").encode()
    except json.JSONDecodeError:
        parts["stdout.txt"] = stdout.encode()
    if out["stderr"].strip():
        parts["stderr.txt"] = out["stderr"].strip().encode() + b"\n"
    for rel, data in out["files"].items():
        parts[f"files/{rel}"] = data
    return parts


def check_stored():
    """Re-run every case and compare the output bytes with the stored expected/ files."""
    bad, n, known = [], 0, 0
    for binary in sorted(d.name for d in CASES.iterdir() if d.is_dir()):
        for case_dir in sorted((CASES / binary).iterdir()):
            exp = case_dir / "expected"
            if not exp.exists():
                continue
            if (exp / "KNOWN_DIVERGENCE.txt").exists():
                known += 1
                continue
            n += 1
            a = run_in_fresh(binary, case_dir, "check")
            got = canonical(a)
            stored = {p.relative_to(exp).as_posix(): p.read_bytes() for p in exp.rglob("*") if p.is_file()}
            if got != stored:
                bad.append(f"{binary}/{case_dir.name}: stored output differs from a fresh run")
    print(f"checked {n} stored outputs against fresh runs; {known} known divergences skipped; mismatches: {len(bad)}")
    for b in bad:
        print("  MISMATCH", b)
    sys.exit(1 if bad else 0)


def main():
    if "--check" in sys.argv:
        check_stored()
    write = "--write" in sys.argv
    summary = {}
    problems = []
    for binary in sorted(d.name for d in CASES.iterdir() if d.is_dir()):
        counts = {"cases": 0, "correct": 0, "equivalent": 0, "captured": 0, "known": 0, "unexpected": 0}
        for case_dir in sorted((CASES / binary).iterdir()):
            if not (case_dir / "case.json").exists():
                continue
            counts["cases"] += 1
            key = f"{binary}/{case_dir.name}"
            a = run_in_fresh(binary, case_dir, "a")
            b = run_in_fresh(binary, case_dir, "b")
            equivalent = canonical(a) == canonical(b)
            if equivalent:
                counts["equivalent"] += 1
            else:
                problems.append(f"{key}: two runs differ (not deterministic)")
            rec = rv.run_case(binary, case_dir)
            correct = rec["pass"]
            if correct:
                counts["correct"] += 1
            expected_dir = case_dir / "expected"
            if correct and equivalent:
                if write:
                    if expected_dir.exists():
                        shutil.rmtree(expected_dir)
                    for rel, data in canonical(a).items():
                        p = expected_dir / rel
                        p.parent.mkdir(parents=True, exist_ok=True)
                        p.write_bytes(data)
                counts["captured"] += 1
            elif key in KNOWN_DIVERGENCES:
                counts["known"] += 1
                if write:
                    expected_dir.mkdir(exist_ok=True)
                    (expected_dir / "KNOWN_DIVERGENCE.txt").write_text(
                        f"Binary output differs from the documented expectation: {KNOWN_DIVERGENCES[key]} "
                        f"(see DISCREPANCES_TEST_CASES.md). The documented value is in case.json; the binary's "
                        f"output is not captured.\n")
            else:
                counts["unexpected"] += 1
                problems.append(f"{key}: differs from reference: {rec['problems'][:2]}")
        summary[binary] = counts
    for b, c in summary.items():
        print(f"{b:8} cases {c['cases']:3}  correct {c['correct']:3}  equivalent {c['equivalent']:3}  "
              f"captured {c['captured']:3}  known-divergent {c['known']}  unexpected {c['unexpected']}")
    for p in problems:
        print("  PROBLEM", p)
    if write:
        print("expected output written" if not problems else "expected output written except the problems above")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
