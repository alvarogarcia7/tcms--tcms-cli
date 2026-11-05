"""Random metrics datasets, compared with the oracle. Mismatches are written to <scratch>/fuzz_mismatch/."""
import json, random, shutil, subprocess, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
import oracle
from common import case_file, put, run_yaml, verif_yaml, SCHEMAS
from run_vv_cases import compare, normalize, TCMS, NOW

SCR = Path("/tmp/claude-1001/-home-orca-tcms-cli/3f56caab-1ffd-4097-894d-9a2fd30e62fc/scratchpad")
VERDICTS = ["pass", "pass", "fail", "error", "blocked", "not_executed", "untested", "skipped"]
seed = int(sys.argv[1]) if len(sys.argv) > 1 else 1
n_iter = int(sys.argv[2]) if len(sys.argv) > 2 else 300
rng = random.Random(seed)
work = SCR / "fuzz_work"
mism_dir = SCR / "fuzz_mismatch"
mism_dir.mkdir(exist_ok=True)
stats = {"ok": 0, "mismatch": 0, "both_refuse": 0, "oracle_refuse_binary_ok": 0, "binary_refuse_oracle_ok": 0}
details = []
for it in range(n_iter):
    cases = [f"TC_{i:02d}" for i in range(1, rng.randint(2, 6) + 1)]
    reqs = [f"SUT1-{k}" for k in range(1, rng.randint(1, 4) + 1)]
    case_map = [(c, rng.choice(reqs)) for c in cases if rng.random() < 0.9]
    plans = [None, "P1", "P2"]
    runs = []
    for k in range(rng.randint(1, 5)):
        plan = rng.choice(plans) if rng.random() < 0.6 else None
        res = []
        for c in rng.sample(cases, rng.randint(0, len(cases))):
            v = rng.choice(VERDICTS)
            prp = f"PR-{rng.randint(1, 3)}" if v in ("fail", "error") and rng.random() < 0.3 else None
            res.append((c, v, prp))
        steps = {c: (x := rng.randint(1, 6), rng.randint(0, x)) for c, _, _ in res}
        day = rng.randint(1, 4)
        runs.append({"id": f"RUN-{k}{rng.choice('ab')}", "started": f"2026-10-0{day}T10:00:00Z",
                     "finished": None if rng.random() < 0.15 else f"2026-10-0{day}T11:00:00Z",
                     "plan": plan, "results": res, "steps": steps if rng.random() < 0.9 else None})
    ids = [r["id"] for r in runs]
    if len(set(ids)) != len(ids):
        continue
    gate = rng.randint(50, 100)
    flaky = rng.randint(1, 6)
    window = rng.randint(1, 5)
    plan_f = rng.choice([None, None, "P1", "P2", "P3"])
    select = rng.choice([None, None] + ids)
    flags = ["--gate", str(gate), "--flaky-threshold", str(flaky), "--stability-window", str(window)]
    if plan_f:
        flags += ["--plan", plan_f]
    if select:
        flags += ["--run", select]
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    for r in runs:
        put(work, f"runs/{r['id']}/test-run.yaml", run_yaml(r))
        if r["results"] and r["steps"] is not None:
            put(work, f"runs/{r['id']}/verification-results.yaml", verif_yaml(r))
    for c, req in case_map:
        put(work, f"{c}.yml", case_file(c, req))
    put(work, "requirements.json", json.dumps({"requirements": [{"id": x} for x in reqs]}))
    spec = {"runs": runs, "cases": case_map, "reqs": reqs}
    try:
        exp = oracle.metrics(spec, gate=gate, flaky=flaky, window=window, plan=plan_f, select=select)
        exp_err = None
    except oracle.Invalid as e:
        exp, exp_err = None, str(e)
    p = subprocess.run([TCMS, "--schemas-root", SCHEMAS, "vv", "metrics", "--root", str(work), "--now", NOW, *flags],
                       capture_output=True, text=True)
    if exp_err is not None and p.returncode != 0:
        stats["both_refuse"] += 1
        continue
    if exp_err is not None:
        stats["oracle_refuse_binary_ok"] += 1
        status = "oracle refuses, binary succeeds"
    elif p.returncode != 0:
        stats["binary_refuse_oracle_ok"] += 1
        status = "binary refuses, oracle succeeds: " + p.stderr.strip()[:200]
    else:
        act = normalize("metrics", json.loads(p.stdout))
        probs = []
        exp2 = {k: v for k, v in exp.items()}
        compare(exp2, act, "", probs)
        if not probs:
            stats["ok"] += 1
            continue
        status = "; ".join(probs[:3])
    stats["mismatch"] += 1
    d = mism_dir / f"fuzz-{seed}-{it:03d}"
    if d.exists():
        shutil.rmtree(d)
    shutil.copytree(work, d)
    (d / "case.json").write_text(json.dumps({"flags": flags, "status": status, "spec": spec}, indent=1, default=str))
    details.append((d.name, status[:260]))
print(stats)
for name, st in details[:25]:
    print(name, "|", st)
