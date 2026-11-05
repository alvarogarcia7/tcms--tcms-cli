"""Generates combine, history, trends and report cases."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import oracle  # noqa: E402
from common import put, reset, run_yaml, write_case, CASES  # noqa: E402
from gen_metrics import R, mix  # noqa: E402

# ---------------------------------------------------------------- combine
RUN_FLAGS = ["--run", "RUN-FIX-1", "--plan", "PLAN-7", "--build-id", "fw-1.4.2",
             "--environment", "Staging bench", "--tester", "ci-bot"]


def A(key, tid, total, passed, failed, overall, start=None, end=None):
    return {"key": key, "tid": tid, "total": total, "passed": passed, "failed": failed,
            "overall": overall, "start": start, "end": end}


def ts(hour, minute=0, sec=0, day=2):
    return f"2026-11-{day:02d}T{hour:02d}:{minute:02d}:{sec:02d}Z"


def log(start, end):
    return (start, end)


def result_yaml(a):
    ne = a["total"] - a["passed"] - a["failed"]
    lines = ["type: test_result", "schema: tcms/test-result.schema.v1.json",
             f"test_case_id: {a['tid']}", "description: generated case", "sequences:",
             "  - sequence_id: 1", "    name: Generated sequence", "    step_results:"]
    step = 0
    for kind, n in (("Pass", a["passed"]), ("Fail", a["failed"]), ("NotExecuted", ne)):
        for _ in range(n):
            step += 1
            lines += [f"      - {kind}:", f"          step: {step}", f"          description: step {step}"]
    lines += [f"    all_steps_passed: {str(a['overall']).lower()}",
              f"total_steps: {a['total']}", f"passed_steps: {a['passed']}",
              f"failed_steps: {a['failed']}", f"not_executed_steps: {ne}",
              f"overall_pass: {str(a['overall']).lower()}"]
    return "\n".join(lines) + "\n"


def verdict_of(a):
    if a["passed"] + a["failed"] == 0 and a["total"] > 0:
        return "not_executed"
    return "pass" if a["overall"] else "fail"


def combine_oracle(attempts, plan=""):
    by = {}
    for a in attempts:
        by.setdefault(a["tid"], []).append(a)
    results = {}
    for tid, lst in by.items():
        lst.sort(key=lambda a: (a["start"] or "", a["key"]))
        decided = "not_executed"
        for a in reversed(lst):
            v = verdict_of(a)
            if v != "not_executed":
                decided = v
                break
        results[tid] = decided
    starts = [a["start"] for a in attempts if a["start"]]
    ends = [a["end"] for a in attempts if a["end"]]
    return {"results": results, "started_at": min(starts) if starts else None,
            "finished_at": max(ends) if ends else None, "plan_id": plan,
            "build_id": "fw-1.4.2", "environment": "Staging bench", "tester": "ci-bot",
            "verification_count": len(by)}


def combine_case(name, attempts, what, flags=None, expect_exit=0, no_write=False, raw_extra=None, plan=""):
    base = CASES / "combine" / name
    base.mkdir(parents=True, exist_ok=True)
    for a in attempts:
        put(base, f"runs/RUN-FIX-1/per-case/{a['key']}_result.yaml", result_yaml(a))
        if a["start"]:
            entries = [{"test_case_id": a["tid"], "step": 1, "timestamp": a["start"]},
                       {"test_case_id": a["tid"], "step": 2, "timestamp": a["end"]}]
            put(base, f"runs/RUN-FIX-1/execution_logs/{a['key']}_execution_log.json", json.dumps(entries, indent=2) + "\n")
    for rel, text in (raw_extra or {}).items():
        put(base, rel, text)
    args = flags if flags is not None else RUN_FLAGS
    args_plan = args[args.index("--plan") + 1] if "--plan" in args else ""
    expect = None
    if expect_exit == 0:
        expect = combine_oracle(attempts, plan=args_plan)
        for flag, field in (("--build-id", "build_id"), ("--environment", "environment"), ("--tester", "tester")):
            if flag in args:
                expect[field] = args[args.index(flag) + 1]
    if no_write:
        expect = {"_no_write": True}
    write_case(base, "combine", list(args), expect_exit, expect, name, what)
    return base


def gen_combine():
    n = 0

    def nm(slug):
        nonlocal n
        n += 1
        return f"{n:03d}-{slug}"

    # A. all pass, N cases
    for N in [1, 2, 3, 5, 8, 12, 20, 30, 62]:
        att = [A(f"TC_P{i:02d}", f"TC_P{i:02d}", 2, 2, 0, True, ts(7, i % 60), ts(7, i % 60, 30)) for i in range(1, N + 1)]
        combine_case(nm(f"all-pass-{N}"), att, f"{N} cases, each 2 of 2 steps passed: all pass; run times from the earliest and latest log entries.")
    # B. pass/fail/not-executed patterns
    for pat in ["PFN", "FFP", "NNP", "PPF", "NPF", "FNN"]:
        att = []
        for i, c in enumerate(pat):
            tid = f"TC_{pat}_{i}"
            if c == "P":
                att.append(A(tid, tid, 2, 2, 0, True, ts(8, i), ts(8, i, 20)))
            elif c == "F":
                att.append(A(tid, tid, 2, 1, 1, False, ts(8, i), ts(8, i, 20)))
            else:
                att.append(A(tid, tid, 2, 0, 0, False, ts(8, i), ts(8, i, 5)))
        combine_case(nm(f"pattern-{pat}"), att, f"Three cases in the pattern {pat} (P pass, F fail, N not executed with no step run): verdicts follow the pattern.")
    # C. duplicates (TCMS-50.1)
    dup = [
        ("newer-pass-beats-older-fail", [A("TC_D__a1", "TC_D", 2, 0, 2, False, ts(6), ts(6, 5)), A("TC_D__a2", "TC_D", 2, 2, 0, True, ts(9), ts(9, 5))], "pass"),
        ("newer-fail-beats-older-pass", [A("TC_D__a1", "TC_D", 2, 2, 0, True, ts(6), ts(6, 5)), A("TC_D__a2", "TC_D", 2, 1, 1, False, ts(9), ts(9, 5))], "fail"),
        ("newer-not-executed-ignored", [A("TC_D__a1", "TC_D", 2, 2, 0, True, ts(6), ts(6, 5)), A("TC_D__a2", "TC_D", 2, 0, 0, False, ts(9), ts(9, 5))], "pass"),
        ("older-fail-newer-not-executed", [A("TC_D__a1", "TC_D", 2, 0, 2, False, ts(6), ts(6, 5)), A("TC_D__a2", "TC_D", 2, 0, 0, False, ts(9), ts(9, 5))], "fail"),
        ("all-not-executed", [A("TC_D__a1", "TC_D", 2, 0, 0, False, ts(6), ts(6, 5)), A("TC_D__a2", "TC_D", 2, 0, 0, False, ts(9), ts(9, 5))], "not_executed"),
        ("tie-start-greater-key-later", [A("TC_D__a1", "TC_D", 2, 2, 0, True, ts(7), ts(7, 5)), A("TC_D__a2", "TC_D", 2, 0, 2, False, ts(7), ts(7, 5))], "fail"),
        ("three-attempts-newest-fail", [A("TC_D__a1", "TC_D", 2, 2, 0, True, ts(5), ts(5, 5)), A("TC_D__a2", "TC_D", 2, 2, 0, True, ts(6), ts(6, 5)), A("TC_D__a3", "TC_D", 2, 0, 2, False, ts(7), ts(7, 5))], "fail"),
        ("three-attempts-middle-not-executed", [A("TC_D__a1", "TC_D", 2, 0, 2, False, ts(5), ts(5, 5)), A("TC_D__a2", "TC_D", 2, 0, 0, False, ts(6), ts(6, 5)), A("TC_D__a3", "TC_D", 2, 2, 0, True, ts(7), ts(7, 5))], "pass"),
    ]
    for slug, att, expect_v in dup:
        others = [A("TC_OTHER", "TC_OTHER", 1, 1, 0, True, ts(4), ts(4, 1))]
        assert combine_oracle(att)["results"]["TC_D"] == expect_v, slug  # self-check of the oracle
        combine_case(nm(f"dup-{slug}"), att + others,
                     f"TC_D has several attempts; the newest attempt that is pass or fail decides (expected {expect_v}). "
                     f"not_executed attempts never decide (TCMS-50.1, VV_METRICS.md).")
    # D. partial execution and not_executed
    combine_case(nm("partial-execution-is-fail"), [A("TC_PART", "TC_PART", 3, 1, 0, False, ts(10), ts(10, 3))],
                 "3 steps, 1 passed, 2 not executed, overall_pass false: at least one step executed, so the verdict is fail, not not_executed.")
    combine_case(nm("not-executed-full"), [A("TC_NE", "TC_NE", 3, 0, 0, False, ts(10), ts(10, 3))],
                 "3 steps, none executed: not_executed.")
    combine_case(nm("all-fail"), [A(f"TC_F{i}", f"TC_F{i}", 2, 0, 2, False, ts(11, i), ts(11, i, 10)) for i in range(4)],
                 "Four cases, each 0 of 2 passed: four fail verdicts.")
    # E. plan flag
    combine_case(nm("no-plan-flag-writes-empty-plan"), [A("TC_NP", "TC_NP", 1, 1, 0, True, ts(12), ts(12, 1))],
                 "No --plan: plan_id is written as an empty string (TCMS-47.4 AC 1).",
                 flags=[a for a in RUN_FLAGS if a not in ("--plan", "PLAN-7")])
    combine_case(nm("plan-flag-recorded"), [A("TC_NP", "TC_NP", 1, 1, 0, True, ts(12), ts(12, 1))],
                 "--plan PLAN-7 is recorded as plan_id.")
    # F. run times
    combine_case(nm("run-times-min-max"), [A("TC_T1", "TC_T1", 1, 1, 0, True, ts(13, 30), ts(13, 31)),
                                           A("TC_T2", "TC_T2", 1, 1, 0, True, ts(9, 0), ts(9, 2)),
                                           A("TC_T3", "TC_T3", 1, 1, 0, True, ts(15, 0), ts(15, 45))],
                 "Earliest start 09:00:00 and latest finish 15:45:00 across cases, not the first file read.")
    combine_case(nm("single-case-single-step"), [A("TC_ONE", "TC_ONE", 1, 1, 0, True, ts(1), ts(1, 0, 1))],
                 "One case, one step: start and finish are the two log entries.")
    # G. invalid
    for flag in ["--build-id", "--tester", "--environment"]:
        f = list(RUN_FLAGS)
        i = f.index(flag)
        del f[i:i + 2]
        combine_case(nm(f"missing-{flag.strip('-')}"), [A("TC_M", "TC_M", 1, 1, 0, True, ts(2), ts(2, 1))],
                     f"{flag} omitted: the test-run schema requires it, so the command must refuse (exit 1) and write nothing.",
                     flags=f, expect_exit=1, no_write=True)
    combine_case(nm("unknown-run-folder"), [A("TC_U", "TC_U", 1, 1, 0, True, ts(2), ts(2, 1))],
                 "--run RUN-NOPE: no per-case folder: refused, exit 1.",
                 flags=["--run", "RUN-NOPE"] + RUN_FLAGS[2:], expect_exit=1, no_write=True)
    combine_case(nm("malformed-per-case-yaml"), [A("TC_MAL", "TC_MAL", 1, 1, 0, True, ts(2), ts(2, 1))],
                 "A per-case file that is not YAML is in the folder: refused, exit 1, nothing written (no partial run record).",
                 expect_exit=1, no_write=True, raw_extra={"runs/RUN-FIX-1/per-case/TC_BROKEN_result.yaml": "type: [unclosed\n  : :\n"})
    combine_case(nm("no-per-case-folder"), [], "The run folder has no per-case files: refused, exit 1, nothing written.",
                 expect_exit=1, no_write=True, raw_extra={"runs/RUN-FIX-1/README.txt": "empty run\n"})
    combine_case(nm("overall-pass-missing"), [A("TC_X", "TC_X", 1, 1, 0, True, ts(2), ts(2, 1))],
                 "A per-case file missing overall_pass is refused, exit 1, nothing written.",
                 expect_exit=1, no_write=True,
                 raw_extra={"runs/RUN-FIX-1/per-case/TC_NOOV_result.yaml": "type: test_result\ntest_case_id: TC_NOOV\ntotal_steps: 1\npassed_steps: 1\n"})
    # H. larger, fixture-like
    att = [A(f"TC_B{i:02d}", f"TC_B{i:02d}", 2, 2, 0, True, ts(6, i % 60), ts(6, i % 60, 9)) for i in range(1, 58)]
    att += [A("TC_FIX_010__a1", "TC_FIX_010", 2, 2, 0, True, ts(6), ts(6, 0, 5)),
            A("TC_FIX_010__a2", "TC_FIX_010", 2, 1, 1, False, ts(9), ts(9, 0, 5))]
    att += [A(f"TC_NE{i}", f"TC_NE{i}", 2, 0, 0, False, None, None) for i in range(3)]
    combine_case(nm("fixture-like-62-cases"), att,
                 "62 distinct cases in the layout of the conversion script: one duplicate (fails newest), three without a log "
                 "and with no executed step (not_executed).")
    return n


# ---------------------------------------------------------------- history / trends
def hist_case(name, runs, case, flags, what, expect_exit=0, plan=None):
    base = CASES / "history" / name
    base.mkdir(parents=True, exist_ok=True)
    for r in runs:
        put(base, f"runs/{r['id']}/test-run.yaml", run_yaml(r))
    expect = None
    if expect_exit == 0:
        expect = oracle.history({"runs": runs}, case, plan=plan)
    write_case(base, "history", list(flags), expect_exit, expect, name, what)


def trends_case(name, runs, flags, what, n=12, gate=90, flaky=4, window=12, plan=None, expect_exit=0):
    base = CASES / "trends" / name
    base.mkdir(parents=True, exist_ok=True)
    for r in runs:
        put(base, f"runs/{r['id']}/test-run.yaml", run_yaml(r))
    expect = None
    if expect_exit == 0:
        t = oracle.trends({"runs": runs}, n=n, gate=gate, flaky=flaky, window=window, plan=plan)
        t["runs"] = {r["runId"]: r for r in t["runs"]}
        expect = t
    write_case(base, "trends", list(flags), expect_exit, expect, name, what)


def gen_history():
    n = 0

    def nm(slug):
        nonlocal n
        n += 1
        return f"{n:03d}-{slug}"

    sets = {
        "all-kinds": [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [("TC_A", "fail")]),
                      R("RUN-3", 3, [("TC_A", "error")]), R("RUN-4", 4, [("TC_A", "blocked")]),
                      R("RUN-5", 5, [("TC_A", "not_executed")]), R("RUN-6", 6, [("TC_A", "skipped")]),
                      R("RUN-7", 7, [("TC_A", "untested")]), R("RUN-8", 8, [("TC_A", "pass")])],
        "gaps": [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [("TC_B", "pass")]),
                 R("RUN-3", 3, [("TC_A", "fail"), ("TC_B", "fail")]), R("RUN-4", 4, [("TC_B", "pass")])],
        "two-plans": [R("RUN-1", 1, [("TC_A", "pass")], plan="PLAN-1"), R("RUN-2", 2, [("TC_A", "fail")], plan="PLAN-2"),
                      R("RUN-3", 3, [("TC_A", "pass")], plan="PLAN-1"), R("RUN-4", 4, [("TC_A", "fail")], plan="PLAN-2")],
        "single-run": [R("RUN-ONLY", 1, [("TC_A", "fail")])],
    }
    for case in ["TC_A", "TC_B", "TC_NONE"]:
        for key, runs in sets.items():
            hist_case(nm(f"{key}-{case.lower()}"), runs, case, ["--case", case],
                      f"Set '{key}', case {case}: one row per run, oldest first, status per run; null when the run has no result.")
    for plan in ["PLAN-1", "PLAN-2", "PLAN-9"]:
        unknown = plan not in ("PLAN-1", "PLAN-2")
        hist_case(nm(f"two-plans-{plan.lower()}"), sets["two-plans"], "TC_A", ["--case", "TC_A", "--plan", plan],
                  ("Unknown plan: refused, exit 1 (as vv metrics does). The docs do not say this for history; "
                   "the expectation follows the metrics rule.") if unknown else f"Only runs of {plan} are listed (--plan).",
                  plan=plan, expect_exit=1 if unknown else 0)
    hist_case(nm("missing-case-flag"), sets["all-kinds"], "TC_A", ["--root-only-no-case"],
              "A bad flag: usage error, exit 2.", expect_exit=2)
    hist_case(nm("zero-runs-in-root"), [], "TC_A", ["--case", "TC_A"], "No run folders: an empty list, exit 0 (documented: one row per recorded run).")
    for i in range(12):
        runs = [R(f"RUN-{k + 1:02d}", k + 1, [("TC_A", ["pass", "fail", "skipped"][(k + i) % 3])]) for k in range(i + 3)]
        hist_case(nm(f"long-history-{i}"), runs, "TC_A", ["--case", "TC_A"],
                  f"{len(runs)} runs with a rotating pattern: the list is complete and in run order.")
    hist_case(nm("invalid-verdict-in-run"), [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [("TC_A", "maybe")])],
              "TC_A", ["--case", "TC_A"], "A run has verdict 'maybe': the verdict vocabulary says any other value is an error: exit 1.",
              expect_exit=1)
    return n


def gen_trends():
    n = 0

    def nm(slug):
        nonlocal n
        n += 1
        return f"{n:03d}-{slug}"

    def seq(k, pattern):
        return [R(f"RUN-{i + 1:02d}", i + 1, [(f"TC_{j}", v) for j, v in enumerate(p)]) for i, p in enumerate(pattern)]

    base5 = seq(5, [["pass", "pass"], ["pass", "fail"], ["fail", "fail"], ["pass", "pass"], ["pass", "fail"]])
    trends_case(nm("five-runs-default"), base5, [], "Five runs, default --runs 12: all five rows, oldest first; rates 100, 50, 0, 100, 50.")
    trends_case(nm("runs-2-of-five"), base5, ["--runs", "2"], "--runs 2: the last two runs only, oldest first.", n=2)
    trends_case(nm("runs-1"), base5, ["--runs", "1"], "--runs 1: the newest run only.", n=1)
    trends_case(nm("runs-larger-than-history"), base5, ["--runs", "50"], "--runs 50 with five runs: five rows.", n=50)
    trends_case(nm("gate-95"), base5, ["--gate", "95"], "Gate 95: rates unchanged, verdicts all FAILED except 100s.", gate=95)
    trends_case(nm("gate-50"), base5, ["--gate", "50"], "Gate 50: a 50 percent run passes.", gate=50)
    trends_case(nm("flaky-2"), base5, ["--flaky-threshold", "2"], "Flaky threshold 2 in the heatmap.", flaky=2)
    trends_case(nm("window-2"), base5, ["--stability-window", "2"], "Stability window 2.", window=2)
    trends_case(nm("heatmap-flaky-case"), seq(6, [["pass"], ["fail"], ["pass"], ["fail"], ["pass"], ["fail"]]), [],
                "One case alternating 6 times: 5 flips, FLAKY at the default threshold 4 in the newest run's heatmap.")
    trends_case(nm("heatmap-failing"), seq(3, [["pass"], ["pass"], ["fail"]]), [],
                "Last result FAILED, one flip: heatmap FAILING.")
    trends_case(nm("heatmap-stable"), seq(3, [["pass"], ["pass"], ["pass"]]), [], "All pass: STABLE.")
    trends_case(nm("no-data-run-row"), [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [("TC_A", "skipped")]),
                                        R("RUN-3", 3, [("TC_A", "fail")])], [],
                "Middle run is skipped only: passRate null and verdict NO_DATA in that row.")
    runs_p = [R("RUN-A1", 1, [("TC_A", "pass")], plan="PLAN-1"), R("RUN-B1", 2, [("TC_A", "fail")], plan="PLAN-2"),
              R("RUN-A2", 3, [("TC_A", "fail")], plan="PLAN-1")]
    trends_case(nm("plan-filter"), runs_p, ["--plan", "PLAN-1"], "Only PLAN-1 runs appear as rows.", plan="PLAN-1")
    trends_case(nm("rounding-row"), seq(4, [["pass", "fail", "fail"], ["pass", "pass", "fail"]]), [],
                "Rows show the floored gate rate: 1 of 3 = 33, 2 of 3 = 66.")
    trends_case(nm("exact-gate-hit-row"), seq(2, [["pass", "pass", "pass", "pass", "pass", "fail", "fail", "fail", "fail", "fail"]]), ["--gate", "50"],
                "Gate 50: 5 of 10 passed = exactly 50: PASSED row (inclusive boundary).", gate=50)
    for k in range(3):
        trends_case(nm(f"many-runs-{k}"), seq(12, [["pass" if (i + k) % 2 else "fail"] for i in range(14)]),
                    ["--runs", str(4 + k)], f"14 runs, --runs {4 + k}: newest {4 + k} rows, oldest first.", n=4 + k)
    trends_case(nm("invalid-stability-window"), base5, ["--stability-window", "0"],
                "Stability window 0 is refused (at least 1): exit 1.", expect_exit=1)
    trends_case(nm("invalid-gate"), base5, ["--gate", "40"], "Gate 40 is refused: exit 1.", expect_exit=1)
    trends_case(nm("unknown-plan"), runs_p, ["--plan", "PLAN-404"], "Unknown plan: exit 1. ASSUMPTION: the docs state this for metrics and not for trends; the expectation is by analogy.", expect_exit=1)
    trends_case(nm("invalid-verdict"), [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [("TC_A", "bogus")])], [],
                "Verdict 'bogus' in a run: refused, exit 1.", expect_exit=1)
    return n


# ---------------------------------------------------------------- report
REPORT_BIN = "report"


def binary_shape(orc, gate_src, flk_src, win_src, generated):
    """Convert the oracle model to the JSON shape tcms vv metrics writes."""
    out = json.loads(json.dumps(orc, default=str))
    out["generatedAt"] = generated
    out["parameters"] = {
        "gateThresholdPercent": {"value": gate_src[1], "source": gate_src[0]},
        "flakyThreshold": {"value": flk_src[1], "source": flk_src[0]},
        "stability_window_runs": {"value": win_src[1], "source": win_src[0]},
    }
    out["requirements"]["items"] = [
        {**{k: v for k, v in it.items() if k != "cases"}, "id": rid,
         "cases": [{"caseId": c, "status": s} for c, s in it["cases"].items()]}
        for rid, it in out["requirements"]["items"].items()]
    out["stability"]["cases"] = [{"caseId": c, **v} for c, v in out["stability"]["cases"].items()]
    out["loadSummary"]["schemaErrorCount"] = 0
    out["loadSummary"]["schemaErrors"] = []
    return out


def report_case(name, metrics_obj, flags, what, expect_exit, contains=(), absent=(), files=None, bad_text=None):
    base = CASES / REPORT_BIN / name
    base.mkdir(parents=True, exist_ok=True)
    if bad_text is not None:
        put(base, "metrics.json", bad_text)
    else:
        put(base, "metrics.json", json.dumps(metrics_obj, indent=2) + "\n")
    base.joinpath("case.json").write_text(json.dumps({
        "binary": "report", "args": list(flags), "expect_exit": expect_exit, "expect": None,
        "contains": list(contains), "absent": list(absent), "expect_files": files or []}, indent=2) + "\n")
    write_case(base, "report", list(flags), expect_exit, None, name, what)
    # write_case wrote case.json too; restore the fuller one
    base.joinpath("case.json").write_text(json.dumps({
        "binary": "report", "args": list(flags), "expect_exit": expect_exit, "expect": None,
        "contains": list(contains), "absent": list(absent), "expect_files": files or []}, indent=2) + "\n")


def gen_report():
    n = 0

    def nm(slug):
        nonlocal n
        n += 1
        return f"{n:03d}-{slug}"

    specs = {
        "pass-9-of-10": ({"runs": [R("RUN-A", 1, mix(9, 1))], "cases": [(f"TC_{i:03d}", "SUT1-1") for i in range(1, 11)], "reqs": ["SUT1-1"]}, {}),
        "fail-1-of-8": ({"runs": [R("RUN-A", 1, mix(1, 7))], "cases": [(f"TC_{i:03d}", "SUT1-1") for i in range(1, 9)], "reqs": ["SUT1-1"]}, {}),
        "no-data": ({"runs": [R("RUN-A", 1, [("TC_A", "skipped")])], "cases": [("TC_A", "SUT1-1")], "reqs": ["SUT1-1"]}, {}),
        "delta": ({"runs": [R("RUN-P", 1, mix(1, 7)), R("RUN-Q", 2, mix(9, 1))], "cases": [(f"TC_{i:03d}", "SUT1-1") for i in range(1, 11)], "reqs": ["SUT1-1"]}, {}),
        "flaky": ({"runs": [R(f"RUN-{i + 1:02d}", i + 1, [("TC_FLIP", v)]) for i, v in enumerate(["pass", "fail", "pass", "fail", "pass", "fail"])],
                   "cases": [("TC_FLIP", "SUT1-1")], "reqs": ["SUT1-1"]}, {}),
        "mixed-reqs": ({"runs": [R("RUN-M", 1, [("TC_A", "pass"), ("TC_B", "fail", "PR-9"), ("TC_C", "not_executed")])],
                        "cases": [("TC_A", "SUT1-1"), ("TC_B", "SUT1-2"), ("TC_C", "SUT1-3")], "reqs": ["SUT1-1", "SUT1-2", "SUT1-3", "SUT1-4"]}, {}),
        "provisional": ({"runs": [R("RUN-V", 1, mix(1, 1), finished=False)], "cases": [("TC_001", "SUT1-1"), ("TC_002", "SUT1-1")], "reqs": ["SUT1-1"]}, {}),
    }
    metric_cache = {}
    for key, (spec, _) in specs.items():
        for gate in [90, 50, 100]:
            for fmt in ["markdown", "asciidoc"]:
                gsrc = ("default", 90) if gate == 90 else ("flag", gate)
                orc = oracle.metrics({"runs": spec["runs"], "cases": spec["cases"], "reqs": spec["reqs"]}, gate=gate,
                                     select=spec["runs"][-1]["id"])
                mo = binary_shape(orc, gsrc, ("default", 4), ("default", 12), "2026-01-01T00:00:00Z")
                metric_cache[(key, gate)] = mo
                verdict = orc["gate"]["verdict"]
                contains = [verdict] + ([("n/a")] if orc["gate"]["deltaFromPrevious"] is None else [])
                if verdict == "NO_DATA":
                    contains = ["NO_DATA"]
                report_case(nm(f"{key}-gate-{gate}-{fmt}"), mo, ["--metrics", "metrics.json", "--format", fmt],
                            f"Report of '{key}' at gate {gate} in {fmt}: exit 0; verdict {verdict} shown.",
                            0, contains=contains)
    # fail-on-gate
    for gate in [90, 100]:
        orc = oracle.metrics({"runs": specs["fail-1-of-8"][0]["runs"], "cases": specs["fail-1-of-8"][0]["cases"],
                              "reqs": ["SUT1-1"]}, gate=gate, select="RUN-A")
        mo = binary_shape(orc, ("flag", gate), ("default", 4), ("default", 12), "2026-01-01T00:00:00Z")
        report_case(nm(f"fail-on-gate-failed-{gate}"), mo, ["--metrics", "metrics.json", "--fail-on-gate"],
                    "FAILED with --fail-on-gate: the report is written first, then exit 2.", 2, contains=["FAILED"])
    orc = oracle.metrics({"runs": specs["pass-9-of-10"][0]["runs"], "cases": specs["pass-9-of-10"][0]["cases"],
                          "reqs": ["SUT1-1"]}, gate=90, select="RUN-A")
    mo = binary_shape(orc, ("default", 90), ("default", 4), ("default", 12), "2026-01-01T00:00:00Z")
    report_case(nm("pass-with-fail-on-gate"), mo, ["--metrics", "metrics.json", "--fail-on-gate"],
                "PASSED with --fail-on-gate: exit 0.", 0, contains=["PASSED"])
    orc = oracle.metrics({"runs": specs["no-data"][0]["runs"], "cases": specs["no-data"][0]["cases"], "reqs": ["SUT1-1"]},
                         gate=90, select="RUN-A")
    mo = binary_shape(orc, ("default", 90), ("default", 4), ("default", 12), "2026-01-01T00:00:00Z")
    report_case(nm("no-data-with-fail-on-gate"), mo, ["--metrics", "metrics.json", "--fail-on-gate"],
                "NO_DATA alone does not exit 2 (README): exit 0.", 0, contains=["NO_DATA"])
    # output file with nested parent folder
    orc = oracle.metrics({"runs": specs["delta"][0]["runs"], "cases": specs["delta"][0]["cases"], "reqs": ["SUT1-1"]}, select="RUN-Q")
    mo = binary_shape(orc, ("default", 90), ("default", 4), ("default", 12), "2026-01-01T00:00:00Z")
    report_case(nm("out-file-nested-dirs"), mo, ["--metrics", "metrics.json", "--out", "deep/er/report.md"],
                "--out creates parent folders; stdout empty; file deep/er/report.md exists.", 0,
                absent=["|"], files=["deep/er/report.md"])
    # invalid inputs
    base_orc = oracle.metrics({"runs": specs["pass-9-of-10"][0]["runs"], "cases": specs["pass-9-of-10"][0]["cases"], "reqs": ["SUT1-1"]}, select="RUN-A")
    base_mo = binary_shape(base_orc, ("default", 90), ("default", 4), ("default", 12), "2026-01-01T00:00:00Z")
    for field in ["gate", "vvRun", "requirements", "stability", "failingCases", "loadSummary", "run", "parameters"]:
        m = json.loads(json.dumps(base_mo))
        m.pop(field, None)
        if field == "failingCases":
            report_case(nm(f"missing-{field}"), m, ["--metrics", "metrics.json"],
                        "DECISION (Opus, D-02): the report requires only the fields it renders. failingCases is not "
                        "rendered, so its absence is not an error: exit 0.", 0, contains=["PASSED"])
        else:
            report_case(nm(f"missing-{field}"), m, ["--metrics", "metrics.json"],
                        f"The metrics JSON lacks the rendered field '{field}': exit 1 and the message names it.", 1)
    report_case(nm("invalid-json"), None, ["--metrics", "metrics.json"], "Metrics file is not JSON: exit 1.", 1,
                bad_text="{ not json")
    report_case(nm("bad-format-value"), base_mo, ["--metrics", "metrics.json", "--format", "html"],
                "--format html is not a report format: usage error, exit 2.", 2)
    report_case(nm("json-is-not-a-format"), base_mo, ["--metrics", "metrics.json", "--format", "json"],
                "--format json: JSON is not a report format: exit 2.", 2)
    report_case(nm("missing-metrics-flag"), base_mo, [], "No --metrics: usage error, exit 2.", 2)
    report_case(nm("empty-object"), {}, ["--metrics", "metrics.json"], "An empty object lacks every field: exit 1.", 1)
    return n


def main():
    for b in ["combine", "history", "trends", "report"]:
        reset(b)
    counts = {"combine": gen_combine(), "history": gen_history(), "trends": gen_trends(), "report": gen_report()}
    print(counts)




# ---------------------------------------------------------------- additional cases (numbered 100+)
def gen_more():
    import itertools
    cnt = {"combine": 0, "history": 0, "trends": 0}

    def nm(b, slug):
        cnt[b] += 1
        return f"{100 + cnt[b]:03d}-{slug}"

    # combine: more patterns of three attempts, and step-count variants
    have = {"PFN", "FFP", "NNP", "PPF", "NPF", "FNN"}
    pats = [p for p in ("".join(t) for t in itertools.product("PFN", repeat=3)) if p not in have][:8]
    for pat in pats:
        att = []
        for i, c in enumerate(pat):
            tid = f"TC_{pat}_{i}"
            if c == "P":
                att.append(A(tid, tid, 2, 2, 0, True, ts(8, i), ts(8, i, 20)))
            elif c == "F":
                att.append(A(tid, tid, 2, 1, 1, False, ts(8, i), ts(8, i, 20)))
            else:
                att.append(A(tid, tid, 2, 0, 0, False, ts(8, i), ts(8, i, 5)))
        combine_case(nm("combine", f"pattern-{pat}"), att, f"Pattern {pat} (P/F/N) across three cases.")
    combine_case(nm("combine", "steps-4-of-4"), [A("TC_S4", "TC_S4", 4, 4, 0, True, ts(5), ts(5, 9))],
                 "4 of 4 steps passed: pass.")
    combine_case(nm("combine", "steps-3-passed-2-failed"), [A("TC_S5", "TC_S5", 5, 3, 2, False, ts(5), ts(5, 9))],
                 "5 steps: 3 passed, 2 failed, overall_pass false: fail.")
    combine_case(nm("combine", "not-executed-x5"), [A(f"TC_N{i}", f"TC_N{i}", 3, 0, 0, False, ts(5, i), ts(5, i, 3)) for i in range(5)],
                 "Five cases, none executed: five not_executed verdicts.")
    combine_case(nm("combine", "single-not-executed-with-no-log"), [A("TC_NL", "TC_NL", 2, 0, 0, False, None, None)],
                 "One not_executed case and no execution log at all: started_at cannot be derived, and the test-run schema "
                 "requires it (RUN_RECORD_FORMAT.md). Refused, exit 1, nothing written.", expect_exit=1, no_write=True)
    combine_case(nm("combine", "dup-across-two-runs-same-folder-keys"),
                 [A("TC_Q__a1", "TC_Q", 1, 1, 0, True, ts(3), ts(3, 1)), A("TC_Q__a2", "TC_Q", 1, 1, 0, True, ts(4), ts(4, 1))],
                 "Two passing attempts of TC_Q: pass, one result.")
    combine_case(nm("combine", "many-cases-unicode-env"), [A("TC_U1", "TC_U1", 1, 1, 0, True, ts(6), ts(6, 1))],
                 "Environment with non-ASCII characters: accepted if the schema allows it.",
                 flags=[("Labor à Dubaï" if a == "Staging bench" else a) for a in RUN_FLAGS])
    cnt["combine"] += 0

    # history: one case per verdict kind, and variations
    kinds = ["pass", "fail", "error", "blocked", "not_executed", "skipped", "untested"]
    for k in kinds:
        hist_case(nm("history", f"single-{k}"), [R("RUN-1", 1, [("TC_K", k)])], "TC_K", ["--case", "TC_K"],
                  f"One run whose result is '{k}': the row status follows the table in VV_METRICS.md.")
    hist_case(nm("history", "case-only-in-last-run"), [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [("TC_A", "pass")]),
                                                    R("RUN-3", 3, [("TC_NEW", "fail")])], "TC_NEW", ["--case", "TC_NEW"],
              "TC_NEW appears only in RUN-3: nulls for the first two runs.")
    hist_case(nm("history", "case-only-in-first-run"), [R("RUN-1", 1, [("TC_OLD", "fail")]), R("RUN-2", 2, [("TC_A", "pass")])],
              "TC_OLD", ["--case", "TC_OLD"], "TC_OLD only in RUN-1: FAILED then null.")
    for i in range(6):
        runs = [R(f"RUN-{k + 1:02d}", k + 1, [("TC_A", ["pass", "fail", "pass", "pass", "not_executed", "error"][(k + i) % 6])],
                  plan="PLAN-1" if (k + i) % 2 == 0 else "PLAN-2") for k in range(7)]
        hist_case(nm("history", f"plan-mix-{i}"), runs, "TC_A", ["--case", "TC_A", "--plan", "PLAN-1"],
                  "Seven runs over two plans; only PLAN-1 runs are listed, oldest first.", plan="PLAN-1")
    hist_case(nm("history", "many-runs-30"), [R(f"RUN-{k + 1:02d}", k + 1, [("TC_A", "pass" if k % 3 else "fail")]) for k in range(30)],
              "TC_A", ["--case", "TC_A"], "Thirty runs: thirty rows in order.")
    hist_case(nm("history", "case-id-with-dash"), [R("RUN-1", 1, [("TC-A-1", "pass")])], "TC-A-1", ["--case", "TC-A-1"],
              "Case id with dashes is matched exactly.")
    hist_case(nm("history", "run-with-no-results"), [R("RUN-1", 1, [("TC_A", "pass")]), R("RUN-2", 2, [])],
              "TC_A", ["--case", "TC_A"], "RUN-2 has results: [] (no cases at all): a null row.")

    # trends: variations of run count, gate, threshold, window
    pattern6 = [["pass", "fail"], ["fail", "fail"], ["pass", "pass"], ["fail", "pass"], ["pass", "fail"], ["pass", "pass"]]
    def seq(pat):
        return [R(f"RUN-{i + 1:02d}", i + 1, [(f"TC_{j}", v) for j, v in enumerate(p)]) for i, p in enumerate(pat)]
    for k in range(1, 7):
        trends_case(nm("trends", f"runs-{k}-of-six"), seq(pattern6), ["--runs", str(k)],
                    f"Six runs, --runs {k}: the last {k} rows oldest first.", n=k)
    for g in [50, 60, 75, 100]:
        trends_case(nm("trends", f"gate-{g}"), seq(pattern6), ["--gate", str(g)],
                    f"Gate {g} applied to every row.", gate=g)
    for f in [1, 2, 3, 6]:
        trends_case(nm("trends", f"flaky-{f}"), seq(pattern6), ["--flaky-threshold", str(f)],
                    f"Flaky threshold {f} in the heatmap of the newest run.", flaky=f)
    for w in [1, 2, 3, 6]:
        trends_case(nm("trends", f"window-{w}"), seq(pattern6), ["--stability-window", str(w)],
                    f"Stability window {w} in the heatmap.", window=w)
    trends_case(nm("trends", "plan-2-only"), [R("RUN-A", 1, [("TC_A", "pass")], plan="PLAN-1"),
                                              R("RUN-B", 2, [("TC_A", "fail")], plan="PLAN-2"),
                                              R("RUN-C", 3, [("TC_A", "fail")], plan="PLAN-2")],
                ["--plan", "PLAN-2"], "Only PLAN-2 runs: rows RUN-B and RUN-C.", plan="PLAN-2")
    trends_case(nm("trends", "single-run-no-data"), [R("RUN-1", 1, [("TC_A", "skipped")])], [],
                "Single run with only skipped: row verdict NO_DATA, passRate null.")
    trends_case(nm("trends", "invalid-gate-101"), seq(pattern6), ["--gate", "101"], "Gate 101 refused.", expect_exit=1)
    trends_case(nm("trends", "invalid-flaky-0"), seq(pattern6), ["--flaky-threshold", "0"], "Flaky 0 refused (at least 1).", expect_exit=1)
    trends_case(nm("trends", "invalid-runs-word"), seq(pattern6), ["--runs", "many"], "--runs must be a number: usage error.", expect_exit=2)
    trends_case(nm("trends", "eight-runs-heatmap-stable"), seq([["pass"]] * 8), [], "Eight passing runs: heatmap all STABLE, rows all 100.")
    print("additional:", cnt)




def gen_more_trends():
    def seq(pat):
        return [R(f"RUN-{i + 1:02d}", i + 1, [(f"TC_{j}", v) for j, v in enumerate(p)]) for i, p in enumerate(pat)]
    cnt = 200
    cases = [
        ("alternating-two-cases", seq([[("pass")], ["fail"], ["pass"], ["fail"], ["pass"]]), [], "Five runs of one case alternating pass/fail: 4 flips, which is FLAKY at the default threshold 4 (flips >= threshold).", {}),
        ("one-case-error-then-pass", seq([["error"], ["pass"], ["pass"]]), [], "Error then pass: error is FAILED for stability: 1 flip, newest STABLE (last kept is PASSED).", {}),
        ("ninety-exact-row", seq([["pass"] * 9 + ["fail"]]), [], "Nine of ten passing = 90 exactly: PASSED at the default gate 90.", {}),
        ("not-executed-middle", seq([["pass"], ["not_executed"], ["fail"]]), [], "Not-executed middle run is not a kept result: kept PASSED, FAILED: 1 flip, FAILING.", {}),
    ]
    for slug, runs, flags, what, kw in cases:
        cnt += 1
        trends_case(f"{cnt:03d}-{slug}", runs, flags, what, **kw)


if __name__ == "__main__":
    gen_more_trends()


def gen_probes():
    """Probes found while reviewing the binary against the docs."""
    seq = lambda pat: [R(f"RUN-{i + 1:02d}", i + 1, [(f"TC_{j}", v) for j, v in enumerate(p)]) for i, p in enumerate(pat)]
    trends_case("301-runs-zero-assumed-refused", seq([["pass"], ["fail"], ["pass"], ["fail"], ["pass"]]), ["--runs", "0"],
                "ASSUMPTION: --runs 0 is not a valid count (the other numeric flags state a minimum of 1). The binary "
                "currently prints one row (the newest run) instead. Expected exit 1.", expect_exit=1)
    pipe_metrics = {"generatedAt": "2026-01-01T00:00:00Z",
                    "parameters": {"gateThresholdPercent": {"value": 90, "source": "default"},
                                   "flakyThreshold": {"value": 4, "source": "default"},
                                   "stability_window_runs": {"value": 12, "source": "default"}},
                    "run": {"runId": "RUN-A", "startedAt": "2026-10-01T10:00:00Z", "finishedAt": "2026-10-01T11:00:00Z",
                            "provisional": False},
                    "gate": {"gateThresholdPercent": 90, "denominator": 1,
                             "counts": {"passed": 0, "failed": 1, "notExecuted": 0, "skipped": 0},
                             "passRate": {"display": 0, "exact": 0.0}, "verdict": "FAILED", "provisional": False,
                             "deltaFromPrevious": None, "previousRunId": None},
                    "vvRun": {"totalTestCases": 1, "passedTestCases": 0, "vvPassRate": {"display": 0, "exact": 0.0},
                              "totalSteps": 0, "passedSteps": 0, "stepCoverage": None},
                    "requirements": {"summary": {"total": 1, "covered": 0, "failing": 1, "notRun": 0, "noCases": 0,
                                                 "withDefect": 0, "openDefects": 0},
                                     "items": [{"id": "SUT|1", "coverage": "FAILING", "executed": 1, "total": 1,
                                                "problemReports": [], "openProblemReports": [],
                                                "cases": [{"caseId": "TC|1", "status": "FAILED"}]}]},
                    "stability": {"flakyThreshold": 4, "flakyCount": 0, "cases": []},
                    "failingCases": [{"caseId": "TC|1", "failureStartRunId": "RUN-A"}],
                    "loadSummary": {"distinctCases": 1, "runsRecorded": 1, "schemaErrorCount": 0, "schemaErrors": []}}
    report_case("061-pipe-escaped-in-table", pipe_metrics, ["--metrics", "metrics.json", "--format", "markdown"],
                "A '|' in a requirement id is written \\| in the table (report README, Escaping): the row stays one line.",
                0, contains=["SUT\\|1"])
    report_case("062-pipe-escaped-asciidoc", pipe_metrics, ["--metrics", "metrics.json", "--format", "asciidoc"],
                "Same pipe check in AsciiDoc.", 0, contains=["SUT\\|1"])
