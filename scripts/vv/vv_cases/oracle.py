"""Independent reference model of the tcms vv rules, written from docs/VV_METRICS.md only.

It never calls the binary. Expected values for generated cases come from here.
"""

from math import floor

# verdict -> gate bucket (docs: VV_METRICS.md "Gate and pass rate", TCMS-50.2)
GATE_BUCKET = {
    "pass": "P", "fail": "F", "error": "F", "blocked": "F",
    "not_executed": "NE", "untested": "NE", "skipped": "S",
}
# verdict -> status string (history, requirements, failingCases)
STATUS = {
    "pass": "PASSED", "fail": "FAILED", "error": "FAILED", "blocked": "FAILED",
    "not_executed": "NOT_EXECUTED", "untested": "NOT_EXECUTED", "skipped": "SKIPPED",
}
# verdict -> stability input (D-97): only PASSED / FAILED count
STAB = {"pass": "PASSED", "fail": "FAILED", "error": "FAILED", "blocked": "FAILED"}


class Invalid(Exception):
    pass


def half_up(num, den):
    """floor(num/den + 1/2) with integers."""
    return (2 * num + den) // (2 * den)


def pct_display(num, den):
    return (num * 100) // den


def scope_runs(runs, plan):
    rs = [r for r in runs if plan is None or r.get("plan") == plan]
    rs.sort(key=lambda r: (r["started"], r["id"]))
    return rs


def gate_of(run, gate):
    """Gate figure of one run: (passed, den, display, exact, verdict)."""
    counts = {"P": 0, "F": 0, "NE": 0, "S": 0}
    for _, v, _ in run["results"]:
        counts[GATE_BUCKET[v]] += 1
    den = counts["P"] + counts["F"] + counts["NE"]
    if den == 0:
        return {"counts": counts, "den": 0, "display": None, "exact": None, "verdict": "NO_DATA"}
    verdict = "PASSED" if counts["P"] * 100 >= gate * den else "FAILED"
    return {"counts": counts, "den": den, "display": pct_display(counts["P"], den),
            "exact": counts["P"] * 100 / den, "verdict": verdict}


def stability_label(seq, thr):
    """seq: kept statuses oldest first (only PASSED/FAILED)."""
    flips = sum(1 for a, b in zip(seq, seq[1:]) if a != b)
    if flips >= thr:
        return "FLAKY", flips
    if seq and seq[-1] == "FAILED":
        return "FAILING", flips
    return "STABLE", flips


def metrics(spec, gate=90, flaky=4, window=12, plan=None, select=None, config=None):
    """spec: runs, cases (case files: list of (id, req)), reqs (list of ids or None)."""
    runs = spec["runs"]
    cases = spec.get("cases", [])
    reqs = spec.get("reqs")
    scope = scope_runs(runs, plan)
    if plan is not None and not scope:
        raise Invalid("no run has plan")
    if not scope:
        raise Invalid("no runs")
    if select is None:
        sel = scope[-1]
    else:
        m = [r for r in scope if r["id"] == select]
        if not m:
            raise Invalid("unknown run")
        sel = m[0]
    i = scope.index(sel)
    if not sel["results"]:
        raise Invalid("selected run has zero results")
    g = gate_of(sel, gate)
    prev = scope[i - 1] if i > 0 else None
    delta, prev_id = None, None
    if prev is not None:
        prev_id = prev["id"]
        pg = gate_of(prev, gate)
        if g["den"] and pg["den"]:
            delta = g["display"] - pg["display"]

    # vvRun
    total_tc = len(sel["results"])
    passed_tc = sum(1 for _, v, _ in sel["results"] if v == "pass")
    ts = sum((sel["steps"] or {}).get(c, (0, 0))[0] for c, _, _ in sel["results"])
    ps = sum((sel["steps"] or {}).get(c, (0, 0))[1] for c, _, _ in sel["results"])
    vv = {"totalTestCases": total_tc, "passedTestCases": passed_tc,
          "vvPassRate": {"display": half_up(passed_tc * 100, total_tc), "exact": passed_tc * 100 / total_tc},
          "totalSteps": ts, "passedSteps": ps,
          "stepCoverage": ({"display": half_up(ps * 100, ts), "exact": ps * 100 / ts} if ts else None)}

    # requirements
    res_sel = {c: (v, prp) for c, v, prp in sel["results"]}
    req_ids = sorted(set(reqs or []) | {r for _, r in cases})
    items = {}
    for rid in req_ids:
        linked = [c for c, r in cases if r == rid]
        if not linked:
            cov = "NO_CASES"
        else:
            sts = [STATUS[res_sel[c][0]] if c in res_sel else None for c in linked]
            if any(s == "FAILED" for s in sts):
                cov = "FAILING"
            elif all(s == "PASSED" for s in sts):
                cov = "COVERED"
            else:
                cov = "NOT_RUN"
        executed = sum(1 for c in linked if c in res_sel and STATUS[res_sel[c][0]] != "NOT_EXECUTED")
        prps = sorted({res_sel[c][1] for c in linked if c in res_sel and res_sel[c][1]})
        open_prps = sorted({res_sel[c][1] for c in linked if c in res_sel and res_sel[c][1]
                            and STATUS[res_sel[c][0]] != "PASSED"})
        items[rid] = {"coverage": cov, "executed": executed, "total": len(linked),
                      "problemReports": prps, "openProblemReports": open_prps,
                      "cases": {c: (STATUS[res_sel[c][0]] if c in res_sel else None) for c in linked}}
    summ = {"total": len(items), "covered": 0, "failing": 0, "notRun": 0, "noCases": 0}
    for it in items.values():
        k = {"COVERED": "covered", "FAILING": "failing", "NOT_RUN": "notRun", "NO_CASES": "noCases"}[it["coverage"]]
        summ[k] += 1
    summ["withDefect"] = sum(1 for it in items.values() if it["problemReports"])
    summ["openDefects"] = len({p for it in items.values() for p in it["openProblemReports"]})

    # stability over the window ending at the selected run
    win = scope[max(0, i - window + 1): i + 1]
    stab = {}
    for c, v, _ in sel["results"]:
        seq = []
        for r in win:
            rv = {x[0]: x[1] for x in r["results"]}.get(c)
            if rv is not None and rv in STAB:
                seq.append(STAB[rv])
        label, flips = stability_label(seq, flaky)
        stab[c] = {"stability": label, "flips": flips}
    flaky_count = sum(1 for s in stab.values() if s["stability"] == "FLAKY")

    # failing cases and streak start
    failing = []
    for c, v, _ in sel["results"]:
        if STAB.get(v) == "FAILED":
            start = sel["id"]
            for r in reversed(scope[:i]):
                rv = {x[0]: x[1] for x in r["results"]}.get(c)
                if rv is None:
                    continue
                if STAB.get(rv) == "FAILED":
                    start = r["id"]
                elif STAB.get(rv) == "PASSED":
                    break
            failing.append({"caseId": c, "failureStartRunId": start})
    failing.sort(key=lambda x: x["caseId"])

    # VV_METRICS.md: distinctCases = ids of the valid case files and the test cases of the verification results
    # in ALL runs (not only the plan's runs); verification results are the steps entries (verification-results.yaml)
    distinct = set(c for c, _ in cases) | {c for r in runs if r["results"] and r.get("steps") is not None
                                           for c in r["steps"]}
    run_out = {"runId": sel["id"], "startedAt": sel["started"], "finishedAt": sel.get("finished"),
               "provisional": sel.get("finished") is None}

    return {
        "run": run_out,
        "gate": {"gateThresholdPercent": gate, "denominator": g["den"],
                 "counts": {"passed": g["counts"]["P"], "failed": g["counts"]["F"],
                            "notExecuted": g["counts"]["NE"], "skipped": g["counts"]["S"]},
                 "passRate": ({"display": g["display"], "exact": g["exact"]} if g["den"] else None),
                 "verdict": g["verdict"], "provisional": sel.get("finished") is None,
                 "deltaFromPrevious": delta, "previousRunId": prev_id},
        "vvRun": vv,
        "requirements": {"summary": summ, "items": items},
        "stability": {"flakyThreshold": flaky, "flakyCount": flaky_count, "cases": stab},
        "failingCases": failing,
        "loadSummary": {"distinctCases": len(distinct), "runsRecorded": len(scope)},
    }


def history(spec, case, plan=None):
    scope = scope_runs(spec["runs"], plan)
    out = []
    for r in scope:
        v = {x[0]: x[1] for x in r["results"]}.get(case)
        out.append({"runId": r["id"], "status": STATUS[v] if v is not None else None})
    return out


def trends(spec, n=12, gate=90, flaky=4, window=12, plan=None):
    scope = scope_runs(spec["runs"], plan)
    rows_src = scope[-n:]
    rows = []
    for r in rows_src:
        g = gate_of(r, gate)
        rows.append({"runId": r["id"], "startedAt": r["started"],
                     "passRate": ({"display": g["display"], "exact": g["exact"]} if g["den"] else None),
                     "verdict": g["verdict"]})
    newest = scope[-1]
    i = len(scope) - 1
    win = scope[max(0, i - window + 1): i + 1]
    heat = {}
    for c, _, _ in newest["results"]:
        seq = []
        for r in win:
            rv = {x[0]: x[1] for x in r["results"]}.get(c)
            if rv is not None and rv in STAB:
                seq.append(STAB[rv])
        label, flips = stability_label(seq, flaky)
        heat[c] = {"stability": label, "flips": flips}
    flaky_n = sum(1 for v in heat.values() if v["stability"] == "FLAKY")
    return {"runs": rows, "heatmap": {"cases": heat, "flakyCount": flaky_n, "flakyThreshold": flaky}}
