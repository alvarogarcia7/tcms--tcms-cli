"""Loop over every vv-cases/<binary>/<case>/ folder: copy it to a scratch dir, run the binary, compare.

Usage: run_vv_cases.py [binary ...]   (default: all binaries present)
Writes results to <scratch>/results/<binary>.jsonl and prints a summary.
"""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from common import CASES, SCHEMAS  # noqa: E402

SCR = Path("/tmp/claude-1001/-home-orca-tcms-cli/3f56caab-1ffd-4097-894d-9a2fd30e62fc/scratchpad")
TCMS = SCR / "repo/target/debug/tcms"
REPORT = SCR / "repo/target/debug/tcms-vv-report"
WORK = SCR / "work"
RESULTS = SCR / "results"
NOW = "2026-01-01T00:00:00Z"


def argv_for(binary, args):
    if binary == "metrics":
        return [str(TCMS), "--schemas-root", SCHEMAS, "vv", "metrics", "--root", ".", "--now", NOW, *args]
    if binary == "combine":
        return [str(TCMS), "--schemas-root", SCHEMAS, "vv", "combine", "--root", ".", *args]
    if binary == "history":
        return [str(TCMS), "--schemas-root", SCHEMAS, "vv", "history", "--root", ".", *args]
    if binary == "trends":
        return [str(TCMS), "--schemas-root", SCHEMAS, "vv", "trends", "--root", ".", *args]
    if binary == "report":
        return [str(REPORT), *args]
    raise SystemExit(f"unknown binary {binary}")


def combine_actual(work, args):
    import datetime
    import yaml
    run = args[args.index("--run") + 1]
    d = work / "runs" / run
    if not (d / "test-run.yaml").exists():
        return None
    tr = yaml.safe_load((d / "test-run.yaml").read_text())
    def iso(v):
        if isinstance(v, datetime.datetime):
            return v.astimezone(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        return v
    act = {"results": {r["test_case_id"]: r["verdict"] for r in tr.get("results", [])},
           "started_at": iso(tr.get("started_at")), "finished_at": iso(tr.get("finished_at")),
           "plan_id": tr.get("plan_id"), "build_id": tr.get("build_id"),
           "environment": tr.get("environment"), "tester": tr.get("tester")}
    vr = d / "verification-results.yaml"
    act["verification_count"] = len(yaml.safe_load(vr.read_text()).get("test_results", [])) if vr.exists() else None
    return act


def key_of(item):
    for k in ("id", "caseId", "runId"):
        if isinstance(item, dict) and k in item:
            return item[k]
    return None


def normalize(binary, act):
    """Turn lists of records into dicts keyed by their id, where the expectation uses a dict."""
    if binary == "metrics" and isinstance(act, dict):
        items = act.get("requirements", {}).get("items")
        if isinstance(items, list):
            d = {}
            for it in items:
                it = dict(it)
                cs = it.get("cases")
                if isinstance(cs, list):
                    it["cases"] = {c["caseId"]: c.get("status") for c in cs}
                d[it["id"]] = it
            act["requirements"]["items"] = d
        st = act.get("stability", {}).get("cases")
        if isinstance(st, list):
            act["stability"]["cases"] = {c["caseId"]: c for c in st}
    if binary == "trends" and isinstance(act, dict):
        if isinstance(act.get("runs"), list):
            act["runs"] = {r["runId"]: r for r in act["runs"]}
        hm = act.get("heatmap", {})
        if isinstance(hm.get("cases"), list):
            hm["cases"] = {c["caseId"]: c for c in hm["cases"]}
    return act


def same(e, a):
    if isinstance(e, float) or isinstance(a, float):
        if isinstance(e, (int, float)) and isinstance(a, (int, float)) and not isinstance(e, bool):
            return abs(float(e) - float(a)) < 1e-9
    return e == a


def compare(e, a, path, out):
    if isinstance(e, dict):
        if not isinstance(a, dict):
            out.append(f"{path}: expected object, got {a!r}")
            return
        for k, v in e.items():
            if k not in a:
                if v is not None:
                    out.append(f"{path}.{k}: missing (expected {v!r})")
                continue
            compare(v, a[k], f"{path}.{k}", out)
    elif isinstance(e, list):
        if not isinstance(a, list) or len(a) != len(e):
            out.append(f"{path}: expected list {e!r}, got {a!r}")
            return
        for i, (x, y) in enumerate(zip(e, a)):
            compare(x, y, f"{path}[{i}]", out)
    else:
        if not same(e, a):
            out.append(f"{path}: expected {e!r}, got {a!r}")


def run_case(binary, case_dir):
    spec = json.loads((case_dir / "case.json").read_text())
    work = WORK / binary / case_dir.name
    if work.exists():
        shutil.rmtree(work)
    shutil.copytree(case_dir, work, ignore=shutil.ignore_patterns("README.md", "case.json"))
    cmd = argv_for(binary, spec["args"])
    p = subprocess.run(cmd, cwd=work, capture_output=True, text=True, env={**os.environ, "NO_COLOR": "1"})
    rec = {"case": case_dir.name, "exit": p.returncode, "expect_exit": spec["expect_exit"],
           "stderr": p.stderr.strip()[-400:], "problems": []}
    if p.returncode != spec["expect_exit"]:
        rec["problems"].append(f"exit {p.returncode}, expected {spec['expect_exit']}")
    exp = spec.get("expect") or {}
    if binary == "combine" and isinstance(exp, dict) and exp.get("_no_write"):
        run = spec["args"][spec["args"].index("--run") + 1] if "--run" in spec["args"] else None
        if run and (work / "runs" / run / "test-run.yaml").exists():
            rec["problems"].append("test-run.yaml was written although the command should refuse")
        if run and (work / "runs" / run / "verification-results.yaml").exists():
            rec["problems"].append("verification-results.yaml was written although the command should refuse")
    if binary == "report":
        for c in spec.get("contains", []):
            if c not in p.stdout:
                rec["problems"].append(f"stdout lacks {c!r}")
        for c in spec.get("absent", []):
            if c in p.stdout and spec["expect_exit"] == 0 and "--out" in spec["args"]:
                rec["problems"].append(f"stdout should be empty with --out, has {c!r}")
        for f in spec.get("expect_files", []):
            if not (work / f).exists():
                rec["problems"].append(f"expected file {f} not written")
    if binary == "combine" and spec["expect_exit"] == 0 and isinstance(spec.get("expect"), dict):
        act = combine_actual(work, spec["args"])
        if act is None:
            rec["problems"].append("test-run.yaml not written")
        else:
            compare(spec["expect"], act, "", rec["problems"])
        rec["pass"] = not rec["problems"]
        return rec
    if spec["expect_exit"] == 0 and p.returncode == 0 and spec.get("expect") is not None:
        expect = spec["expect"]
        if isinstance(expect, dict):
            expect = {k: v for k, v in expect.items() if not k.startswith("_")}
        try:
            act = json.loads(p.stdout)
        except json.JSONDecodeError as ex:
            rec["problems"].append(f"stdout is not JSON: {ex}")
            act = None
        if act is not None:
            if isinstance(spec["expect"], dict) and "_schemaErrorsLen" in spec["expect"]:
                got = len(act.get("loadSummary", {}).get("schemaErrors", []))
                if got != spec["expect"]["_schemaErrorsLen"]:
                    rec["problems"].append(f"loadSummary.schemaErrors length {got}, expected {spec['expect']['_schemaErrorsLen']}")
            act = normalize(binary, act)
            compare(expect, act, "", rec["problems"])
    if spec["expect_exit"] != 0 and p.returncode == 0:
        rec["problems"].append("expected refusal but command succeeded")
    rec["pass"] = not rec["problems"]
    return rec


def main():
    binaries = sys.argv[1:] or sorted(d.name for d in CASES.iterdir() if d.is_dir())
    RESULTS.mkdir(parents=True, exist_ok=True)
    for b in binaries:
        recs = []
        for case_dir in sorted((CASES / b).iterdir()):
            if (case_dir / "case.json").exists():
                recs.append(run_case(b, case_dir))
        with open(RESULTS / f"{b}.jsonl", "w") as f:
            for r in recs:
                f.write(json.dumps(r) + "\n")
        ok = sum(r["pass"] for r in recs)
        print(f"== {b}: {ok}/{len(recs)} pass")
        for r in recs:
            if not r["pass"]:
                print(f"  FAIL {r['case']}")
                for pr in r["problems"][:4]:
                    print(f"       - {pr[:240]}")
                if r["stderr"]:
                    print(f"       stderr: {r['stderr'][-200:]!r}")


if __name__ == "__main__":
    main()
