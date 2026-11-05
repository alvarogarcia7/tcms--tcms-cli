"""Random trends datasets compared with oracle.trends (rows and heatmap)."""
import json, random, shutil, subprocess, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
import oracle
from common import put, run_yaml, SCHEMAS
from run_vv_cases import TCMS, compare, normalize
SCR = Path("/tmp/claude-1001/-home-orca-tcms-cli/3f56caab-1ffd-4097-894d-9a2fd30e62fc/scratchpad")
seed = int(sys.argv[1]); n_iter = int(sys.argv[2])
rng = random.Random(seed)
VERD = ["pass", "pass", "fail", "error", "blocked", "not_executed", "untested", "skipped"]
work = SCR / "fuzz_trends_work"; stats = {"ok": 0, "mismatch": 0, "both_refuse": 0, "other": 0}; bad = []
for it in range(n_iter):
    runs = []
    cases = [f"TC_{i}" for i in range(1, rng.randint(1, 4) + 1)]
    for k in range(rng.randint(1, 9)):
        plan = rng.choice([None, "P1", "P2"]) if rng.random() < 0.5 else None
        res = [(c, rng.choice(VERD), None) for c in cases if rng.random() < 0.8]
        steps = {c: (3, 1) for c, _, _ in res}
        runs.append({"id": f"RUN-{k:02d}", "started": f"2026-10-{k + 1:02d}T10:00:00Z", "finished": "2026-10-01T11:00:00Z",
                     "plan": plan, "results": res, "steps": steps})
    flags, gate, flaky, window, plan_f = [], 90, 4, 12, None
    if rng.random() < 0.6:
        gate = rng.randint(50, 100); flags += ["--gate", str(gate)]
    if rng.random() < 0.5:
        flaky = rng.randint(1, 6); flags += ["--flaky-threshold", str(flaky)]
    if rng.random() < 0.5:
        window = rng.randint(1, 8); flags += ["--stability-window", str(window)]
    n = 12
    if rng.random() < 0.7:
        n = rng.randint(1, 10); flags += ["--runs", str(n)]
    if rng.random() < 0.3:
        plan_f = rng.choice(["P1", "P2"]); flags += ["--plan", plan_f]
    if work.exists(): shutil.rmtree(work)
    work.mkdir(parents=True)
    for r in runs:
        put(work, f"runs/{r['id']}/test-run.yaml", run_yaml(r))
    try:
        exp = oracle.trends({"runs": runs}, n=n, gate=gate, flaky=flaky, window=window, plan=plan_f)
    except Exception as e:
        exp = None; exp_err = str(e)
    p = subprocess.run([TCMS, "--schemas-root", SCHEMAS, "vv", "trends", "--root", str(work), *flags], capture_output=True, text=True)
    if exp is None:
        if p.returncode != 0:
            stats["both_refuse"] += 1
        else:
            stats["other"] += 1
        continue
    if p.returncode != 0:
        stats["other"] += 1; bad.append((it, flags, "binary refuses: " + p.stderr.strip()[:150])); continue
    act = normalize("trends", json.loads(p.stdout))
    exp["runs"] = {r["runId"]: r for r in exp["runs"]}
    probs = []
    compare(exp, act, "", probs)
    if probs:
        stats["mismatch"] += 1; bad.append((it, flags, "; ".join(probs[:2])[:220]))
    else:
        stats["ok"] += 1
print(stats)
for b in bad[:12]: print(b)
