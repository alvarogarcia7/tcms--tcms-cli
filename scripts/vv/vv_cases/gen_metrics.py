"""Generates tcms vv metrics cases into vv-cases/metrics/<NN-slug>/."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import oracle  # noqa: E402
from common import (CASES, case_file, put, reset, run_yaml, verif_yaml, write_case)  # noqa: E402

BIN = "metrics"


def T(day, hour=10):
    return f"2026-10-{day:02d}T{hour:02d}:00:00Z"


def R(rid, day, results, finished=True, plan=None, steps=None, hour=10):
    """results: list of (case, verdict[, prp]). steps default: 3 steps per case."""
    res = [(x[0], x[1], x[2] if len(x) > 2 else None) for x in results]
    st = {}
    for c, v, _ in res:
        if steps and c in steps:
            st[c] = steps[c]
        else:
            st[c] = (3, {"pass": 3, "skipped": 3, "fail": 1, "error": 1, "blocked": 1}.get(v, 0))
    return {"id": rid, "started": T(day, hour), "finished": T(day, hour + 1) if finished else None,
            "plan": plan, "results": res, "steps": st}


def mix(passed, failed, ne=0, skipped=0, blocked=0, error=0):
    """Build a results list with the given verdict counts, cases TC_001..."""
    out, n = [], 0
    for v, k in [("pass", passed), ("fail", failed), ("not_executed", ne),
                 ("skipped", skipped), ("blocked", blocked), ("error", error)]:
        for _ in range(k):
            n += 1
            out.append((f"TC_{n:03d}", v))
    return out


def cases_of(results, req="SUT1-1"):
    return [(x[0], req) for x in results]


def build(name, title, what, runs, cases=None, reqs=None, flags=(), config=None,
          config_text=None, extra_files=None, expect_exit=0, select=None, write_root_cases=True):
    base = CASES / BIN / name
    base.mkdir(parents=True, exist_ok=True)
    for r in runs:
        put(base, f"runs/{r['id']}/test-run.yaml", run_yaml(r))
        if r["results"] and r.get("steps") is not None:
            put(base, f"runs/{r['id']}/verification-results.yaml", verif_yaml(r))
    cases = cases or []
    for cid, req in cases:
        if write_root_cases:
            put(base, f"{cid}.yml", case_file(cid, req))
    if reqs is not None:
        import json
        put(base, "requirements.json", json.dumps({"requirements": [{"id": x} for x in reqs]}, indent=2) + "\n")
    if config_text is not None:
        put(base, "vv-config.yaml", config_text)
    elif config:
        lines = ["type: vv-config", "schema: tcms/vv-config.schema.v1.json"]
        for k, v in config.items():
            lines.append(f"{k}: {v}")
        put(base, "vv-config.yaml", "\n".join(lines) + "\n")
    for rel, text in (extra_files or {}).items():
        put(base, rel, text)

    # effective parameters: flag > file > default
    def flag(f):
        return flags[flags.index(f) + 1] if f in flags else None

    file_cfg = {}
    if config:
        file_cfg = config
    gate_src = ("flag", int(flag("--gate"))) if flag("--gate") else (
        ("file", int(file_cfg["gate_threshold_percent"])) if "gate_threshold_percent" in file_cfg else ("default", 90))
    flk_src = ("flag", int(flag("--flaky-threshold"))) if flag("--flaky-threshold") else (
        ("file", int(file_cfg["flaky_flip_threshold"])) if "flaky_flip_threshold" in file_cfg else ("default", 4))
    win_src = ("flag", int(flag("--stability-window"))) if flag("--stability-window") else (
        ("file", int(file_cfg["stability_window_runs"])) if "stability_window_runs" in file_cfg else ("default", 12))

    expect = None
    if expect_exit == 0:
        spec = {"runs": runs, "cases": cases, "reqs": reqs}
        plan = flag("--plan")
        expect = oracle.metrics(spec, gate=gate_src[1], flaky=flk_src[1], window=win_src[1],
                                plan=plan, select=select or flag("--run"))
        expect["parameters"] = {
            "gateThresholdPercent": {"value": gate_src[1], "source": gate_src[0]},
            "flakyThreshold": {"value": flk_src[1], "source": flk_src[0]},
            "stability_window_runs": {"value": win_src[1], "source": win_src[0]},
        }
    write_case(base, BIN, list(flags), expect_exit, expect, title, what)
    return base


def main():
    reset(BIN)
    n = 0

    def nm(slug):
        nonlocal n
        n += 1
        return f"{n:03d}-{slug}"

    # ---- A. rounding, single run (valid) -------------------------------------------------
    pairs = [(1, 8), (3, 8), (5, 8), (7, 8), (1, 16), (3, 16), (1, 40), (7, 40), (1, 3), (2, 3),
             (9, 10), (17, 20), (12, 13), (11, 12), (7, 9), (3, 7), (13, 15), (1, 6), (5, 6), (1, 2),
             (19, 20), (39, 40)]
    for p, t in pairs:
        res = mix(p, t - p)
        r = R("RUN-A", 1, res)
        ex = {1: "display is floor, vvPassRate is half up", }
        slug = f"round-{p}-of-{t}"
        what = (f"One run, {p} of {t} test cases passed (the rest fail), gate 90 (default). "
                f"Rounding focus: the exact pass rate is {p * 100 / t:.4g}%; the gate display rounds it down, the "
                f"V&V view (vvRun.vvPassRate) rounds half up, and the verdict compares exact integers.")
        build(nm(slug), f"Rounding {p} of {t}", what, [r], cases_of(res), reqs=["SUT1-1"],
              flags=("--run", "RUN-A"))

    # ---- B. gate boundaries and 100 percent (valid) -----------------------------------------
    bounds = [
        ((2, 3), 66, "exact 66.67 >= 66: PASSED at the floor of the gate"),
        ((2, 3), 67, "exact 66.67 < 67: FAILED, display still 66"),
        ((9, 10), 90, "exactly 90 = gate: PASSED (the boundary is inclusive)"),
        ((9, 10), 91, "90 < 91: FAILED"),
        ((1, 2), 50, "50 = minimum gate: PASSED"),
        ((4, 5), 100, "80 with gate 100: FAILED"),
        ((5, 5), 100, "100 percent with gate 100: PASSED, display 100"),
        ((1, 1), 90, "single case, passes: 100 percent, PASSED"),
    ]
    for (p, t), gate, what in bounds:
        res = mix(p, t - p)
        r = R("RUN-A", 1, res)
        build(nm(f"gate-{p}-of-{t}-at-{gate}"), f"Gate {gate} on {p} of {t}",
              f"Gate boundary: {what}.", [r], cases_of(res), reqs=["SUT1-1"],
              flags=("--run", "RUN-A", "--gate", str(gate)))

    # ---- C. deltas between runs (valid) -------------------------------------------------------
    def two(prev_pt, cur_pt, what, slug):
        p0 = mix(prev_pt[0], prev_pt[1] - prev_pt[0])
        p1 = mix(cur_pt[0], cur_pt[1] - cur_pt[0])
        runs = [R("RUN-P", 1, p0), R("RUN-Q", 2, p1)]
        build(nm(slug), slug.replace("-", " "), what, runs, cases_of(p1), reqs=["SUT1-1"],
              flags=("--run", "RUN-Q"))

    two((1, 8), (9, 10), "Previous 1 of 8 (12.5, floor 12), now 9 of 10 (90): delta +78 on printed rates.", "delta-12-to-90")
    two((2, 3), (5, 8), "Previous 2 of 3 (66.67, floor 66), now 5 of 8 (62.5, floor 62): delta -4.", "delta-66-to-62")
    two((1, 3), (2, 3), "Previous 1 of 3 (33.3, floor 33), now 2 of 3 (66.7, floor 66): delta +33.", "delta-33-to-66")
    two((0, 4), (1, 8), "Previous 0 of 4 (0), now 1 of 8 (12): delta +12; the exact values are not subtracted.", "delta-zero-to-12")
    two((4, 5), (4, 5), "Identical rates: delta 0.", "delta-zero-same")

    # ---- D. stability (valid) -----------------------------------------------------------------
    flips = ["pass", "fail", "pass", "fail", "pass", "fail"]
    runs = []
    for i, v in enumerate(flips):
        runs.append(R(f"RUN-{i + 1:02d}", i + 1, [("TC_FLIP", v), ("TC_STEADY", "pass"), ("TC_FAILS", "fail" if i < 5 else "pass")]))
    cases = [("TC_FLIP", "SUT1-1"), ("TC_STEADY", "SUT1-1"), ("TC_FAILS", "SUT1-2")]
    build(nm("stability-flaky-default"), "Stability: 5 flips is FLAKY",
          "Six runs; TC_FLIP alternates (5 flips, FLAKY at the default threshold 4); TC_STEADY passes "
          "throughout (STABLE); TC_FAILS fails in runs 1-5 and passes in run 6 (1 flip, STABLE).",
          runs, cases, reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-06"))
    build(nm("stability-threshold-6"), "Stability: threshold 6 makes TC_FLIP not flaky",
          "Same six runs as the previous case with --flaky-threshold 6: 5 flips are below 6, so TC_FLIP is FAILING "
          "(its last kept result is FAILED).",
          runs, cases, reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-06", "--flaky-threshold", "6"))
    build(nm("stability-window-3"), "Stability window of 3 runs",
          "Same six runs with --stability-window 3: only RUN-04..RUN-06 count, TC_FLIP has 2 flips.",
          runs, cases, reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-06", "--stability-window", "3"))
    build(nm("stability-flaky-2"), "Flaky threshold 2",
          "Six runs as before with --flaky-threshold 2: TC_FLIP and TC_FAILS (1 flip) are judged with threshold 2.",
          runs, cases, reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-06", "--flaky-threshold", "2"))

    # ---- E. requirements (valid) --------------------------------------------------------------
    r_cov = R("RUN-R", 1, [("TC_A", "pass"), ("TC_B", "fail", "PR-101"), ("TC_C", "not_executed"), ("TC_D", "skipped")])
    cases = [("TC_A", "SUT1-1"), ("TC_B", "SUT1-2"), ("TC_C", "SUT1-3"), ("TC_D", "SUT1-4")]
    build(nm("requirements-mixed"), "Requirements: covered, failing, not run, skipped",
          "SUT1-1 covered by a pass; SUT1-2 failing with problem report PR-101 (open); SUT1-3 not run "
          "(not_executed is not passed); SUT1-4 skipped only (NOT_RUN with 1 executed); SUT1-5 listed in "
          "requirements.json with no case (NO_CASES).",
          [r_cov], cases, reqs=["SUT1-1", "SUT1-2", "SUT1-3", "SUT1-4", "SUT1-5"], flags=("--run", "RUN-R"))
    build(nm("requirements-no-cases"), "Requirements: none cite the case files",
          "requirements.json lists SUT1-1 and SUT1-2 but no case file cites them: both NO_CASES.",
          [R("RUN-R", 1, [("TC_A", "pass")])], [], reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-R"))

    # ---- F. plan filter (valid) ---------------------------------------------------------------
    runs = [R("RUN-X1", 1, mix(1, 1), plan="PLAN-1"), R("RUN-Y1", 2, mix(1, 3), plan="PLAN-2"),
            R("RUN-X2", 3, mix(9, 1), plan="PLAN-1")]
    build(nm("plan-filter-delta"), "Plan filter: delta against the previous run of the same plan",
          "Three runs over two plans. With --plan PLAN-1 the previous run of RUN-X2 is RUN-X1 (not RUN-Y1): "
          "delta uses 50 -> 90.",
          runs, cases_of(mix(9, 1)), reqs=["SUT1-1"], flags=("--plan", "PLAN-1"))

    # ---- G. config file precedence (valid) ---------------------------------------------------
    r = R("RUN-C", 1, mix(9, 1))
    build(nm("config-file-gate"), "Gate from vv-config.yaml",
          "vv-config.yaml sets gate_threshold_percent 95 and no flag: 90% exact fails the 95 gate; source 'file'.",
          [r], cases_of(mix(9, 1)), config={"gate_threshold_percent": 95}, flags=("--run", "RUN-C"))
    build(nm("config-flag-over-file"), "Flag overrides vv-config.yaml",
          "vv-config.yaml says 95, --gate 80 overrides it: PASSED at 90; parameters source 'flag'.",
          [r], cases_of(mix(9, 1)), config={"gate_threshold_percent": 95}, flags=("--run", "RUN-C", "--gate", "80"))

    # ---- H. no data, provisional, and 0-blocked (valid) -----------------------------------
    build(nm("no-data-skipped-only"), "Skipped only: NO_DATA",
          "Every result is skipped: the denominator is 0, verdict NO_DATA, passRate and delta are null.",
          [R("RUN-S", 1, [("TC_A", "skipped"), ("TC_B", "skipped")])], [("TC_A", "SUT1-1"), ("TC_B", "SUT1-1")],
          reqs=["SUT1-1"], flags=("--run", "RUN-S"))
    build(nm("provisional-run"), "Run without finished_at is provisional",
          "No finished_at in test-run.yaml: run.provisional is true; the figures are otherwise normal (1 of 2 = 50).",
          [R("RUN-PV", 1, mix(1, 1), finished=False)], cases_of(mix(1, 1)), reqs=["SUT1-1"], flags=("--run", "RUN-PV"))
    build(nm("zero-blocked-all-pass"), "Zero blocked, one valid: 100 percent",
          "Three passing cases, 0 blocked, 0 failed: display 100, exact 100, verdict PASSED at gate 100.",
          [R("RUN-Z", 1, mix(3, 0))], cases_of(mix(3, 0)), reqs=["SUT1-1"],
          flags=("--run", "RUN-Z", "--gate", "100"))
    build(nm("blocked-and-error-count-failed"), "error and blocked count as FAILED",
          "One pass, one error, one blocked: denominator 3 (2 failed), display 33, verdict FAILED.",
          [R("RUN-E", 1, mix(1, 0, error=1, blocked=1))], cases_of(mix(1, 0, error=1, blocked=1)), reqs=["SUT1-1"],
          flags=("--run", "RUN-E"))
    build(nm("not-executed-and-skipped"), "not_executed in denominator, skipped outside",
          "One pass, one not_executed, one skipped: denominator 2, display 50, skipped counted separately.",
          [R("RUN-N", 1, mix(1, 0, ne=1, skipped=1))], cases_of(mix(1, 0, ne=1, skipped=1)), reqs=["SUT1-1"],
          flags=("--run", "RUN-N"))

    # ---- I. invalid / mixed inputs ------------------------------------------------------------
    base_run = [R("RUN-A", 1, mix(2, 1))]
    build(nm("invalid-gate-49"), "Gate 49 is refused", "--gate 49 is outside 50..100: exit 1, no JSON.",
          base_run, cases_of(mix(2, 1)), reqs=["SUT1-1"], flags=("--gate", "49"), expect_exit=1)
    build(nm("invalid-gate-101"), "Gate 101 is refused", "--gate 101: exit 1.",
          base_run, cases_of(mix(2, 1)), reqs=["SUT1-1"], flags=("--gate", "101"), expect_exit=1)
    build(nm("invalid-flaky-0"), "Flaky threshold 0 refused", "--flaky-threshold 0 (must be at least 1): exit 1.",
          base_run, cases_of(mix(2, 1)), reqs=["SUT1-1"], flags=("--flaky-threshold", "0"), expect_exit=1)
    build(nm("invalid-window-0"), "Stability window 0 refused", "--stability-window 0 (at least 1): exit 1.",
          base_run, cases_of(mix(2, 1)), reqs=["SUT1-1"], flags=("--stability-window", "0"), expect_exit=1)
    build(nm("run-id-inside-file-not-folder"), "--run takes the folder name, not run_id",
          "Folder RUN-FOLDER-1 contains test-run.yaml with run_id RUN-INSIDE-9. --run RUN-INSIDE-9 must fail "
          "(TCMS-50.6: folder name only). exit 1.",
          [dict(R("RUN-FOLDER-1", 1, mix(1, 1)), run_id="RUN-INSIDE-9")], cases_of(mix(1, 1)), reqs=["SUT1-1"],
          flags=("--run", "RUN-INSIDE-9"), expect_exit=1)
    build(nm("unknown-run"), "Unknown --run", "--run RUN-NOPE is not a folder: exit 1, the message lists valid folders.",
          base_run, cases_of(mix(2, 1)), reqs=["SUT1-1"], flags=("--run", "RUN-NOPE"), expect_exit=1)
    build(nm("unknown-plan"), "Unknown --plan", "--plan PLAN-404: no run has that plan, exit 1.",
          [R("RUN-A", 1, mix(1, 0), plan="PLAN-1")], cases_of(mix(1, 0)), reqs=["SUT1-1"],
          flags=("--plan", "PLAN-404"), expect_exit=1)
    build(nm("selected-run-zero-results"), "Selected run with results: []",
          "The selected run has no results: exit 1 (not NO_DATA).",
          [R("RUN-EMPTY", 1, [])], [], reqs=None, flags=("--run", "RUN-EMPTY"), expect_exit=1)
    build(nm("invalid-verdict-in-history"), "Unknown verdict in an older run",
          "RUN-OLD has verdict 'maybe' for TC_A; the selected RUN-NEW is valid. Any other verdict is an error "
          "naming the case (VV_METRICS.md): exit 1.",
          [R("RUN-OLD", 1, [("TC_A", "maybe")]), R("RUN-NEW", 2, mix(1, 0))], cases_of(mix(1, 0)),
          reqs=["SUT1-1"], flags=("--run", "RUN-NEW"), expect_exit=1)
    build(nm("invalid-config-type"), "vv-config.yaml with a string gate",
          "gate_threshold_percent: \"ninety\" in vv-config.yaml: the file is refused, exit 1, names the key.",
          [R("RUN-C", 1, mix(1, 0))], cases_of(mix(1, 0)), reqs=["SUT1-1"],
          config_text="type: vv-config\nschema: tcms/vv-config.schema.v1.json\ngate_threshold_percent: \"ninety\"\n",
          flags=("--run", "RUN-C"), expect_exit=1)
    build(nm("invalid-config-unknown-key"), "vv-config.yaml with an unknown key",
          "An extra key 'colour: blue' is refused: exit 1.",
          [R("RUN-C", 1, mix(1, 0))], cases_of(mix(1, 0)), reqs=["SUT1-1"],
          config_text="type: vv-config\nschema: tcms/vv-config.schema.v1.json\ncolour: blue\n",
          flags=("--run", "RUN-C"), expect_exit=1)
    build(nm("mixed-invalid-case-file"), "One invalid test case file among valid ones",
          "BAD_001.yml declares type test_case but lacks required fields. It goes to loadSummary.schemaErrors and "
          "is not counted: distinctCases counts only the valid case. Exit 0.",
          [R("RUN-A", 1, mix(1, 0))], cases_of(mix(1, 0)), reqs=["SUT1-1"], flags=("--run", "RUN-A"),
          extra_files={"BAD_001.yml": "type: test_case\nschema: tcms/test-case.schema.v1.json\nid: BAD_001\n"})

    # ---- J. edge cases added after the first pass ------------------------------------------
    runs = [R("RUN-P1", 1, mix(1, 0), plan="PLAN-1"), R("RUN-P2", 2, mix(1, 1), plan="PLAN-2")]
    build(nm("run-not-of-plan"), "--run must belong to --plan",
          "RUN-P2 exists but has PLAN-2; --plan PLAN-1 --run RUN-P2 must fail (TCMS-50.6 and the --plan rule).",
          runs, cases_of(mix(1, 1)), reqs=["SUT1-1"], flags=("--plan", "PLAN-1", "--run", "RUN-P2"), expect_exit=1)
    build(nm("invalid-config-gate-range"), "vv-config.yaml gate 40 out of range",
          "gate_threshold_percent: 40 is below 50: the file fails, exit 1.",
          [R("RUN-C", 1, mix(1, 0))], cases_of(mix(1, 0)), reqs=["SUT1-1"],
          config_text="type: vv-config\nschema: tcms/vv-config.schema.v1.json\ngate_threshold_percent: 40\n",
          flags=("--run", "RUN-C"), expect_exit=1)
    many = {f"BAD_{i:03d}.yml": f"type: test_case\nschema: tcms/test-case.schema.v1.json\nid: BAD_{i:03d}\n"
            for i in range(25)}
    b = build(nm("schema-errors-capped-at-20"), "25 invalid case files: count 25, list capped at 20",
              "25 files declare type test_case and fail schema validation. loadSummary.schemaErrorCount must be 25 "
              "and schemaErrors must list at most 20 (VV_METRICS.md). distinctCases counts only the valid case TC_001.",
              [R("RUN-A", 1, mix(1, 0))], [("TC_001", "SUT1-1")], reqs=["SUT1-1"],
              flags=("--run", "RUN-A"), extra_files=many)
    # patch the expectation: this runner key checks the length of schemaErrors
    import json as _json
    cj = _json.loads((b / "case.json").read_text())
    cj["expect"]["loadSummary"]["schemaErrorCount"] = 25
    cj["expect"]["_schemaErrorsLen"] = 20
    (b / "case.json").write_text(_json.dumps(cj, indent=2, sort_keys=True) + "\n")
    runs = [R("RUN-B", 1, mix(1, 0)), R("RUN-A", 1, mix(0, 1))]
    build(nm("same-start-folder-tiebreak"), "Equal started_at: folder name breaks the tie",
          "RUN-A and RUN-B both start at 10:00. Ordered by folder name, RUN-B is last, so the default selection is "
          "RUN-B (1 of 1 = 100) and its previous run is RUN-A (0 of 1 = 0): deltaFromPrevious +100.",
          runs, cases_of(mix(1, 0)), reqs=["SUT1-1"], flags=())
    build(nm("steps-coverage-half-up"), "Step coverage 1 of 8 rounds half up to 13",
          "One failing case with 8 steps of which 1 passed: stepCoverage display 13 (12.5 rounded half up), "
          "gate display 0.",
          [R("RUN-S", 1, [("TC_001", "fail")], steps={"TC_001": (8, 1)})], [("TC_001", "SUT1-1")], reqs=["SUT1-1"],
          flags=("--run", "RUN-S"))
    seq = [R("RUN-01", 1, [("TC_X", "pass"), ("TC_Y", "pass")]),
           R("RUN-02", 2, [("TC_X", "not_executed"), ("TC_Y", "pass")]),
           R("RUN-03", 3, [("TC_X", "fail"), ("TC_Y", "pass")])]
    build(nm("stability-not-executed-between"), "not_executed between two results is ignored by stability",
          "TC_X: pass, not_executed, fail. The not_executed run is not a kept result, so the kept sequence is "
          "PASSED, FAILED: 1 flip, FAILING (threshold 4).",
          seq, [("TC_X", "SUT1-1"), ("TC_Y", "SUT1-2")], reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-03"))
    seq2 = [R("RUN-01", 1, [("TC_X", "pass")]), R("RUN-02", 2, [("TC_X", "error")])]
    build(nm("stability-error-counts-failed"), "error is FAILED for stability: one flip",
          "TC_X passes then errors: kept sequence PASSED, FAILED, 1 flip, label FAILING.",
          seq2, [("TC_X", "SUT1-1")], reqs=["SUT1-1"], flags=("--run", "RUN-02"))
    build(nm("delta-previous-no-data"), "Previous run NO_DATA: delta is null, previousRunId is set",
          "RUN-01 has only skipped results (denominator 0). RUN-02 is 1 of 2 = 50. deltaFromPrevious null because "
          "one denominator is 0; previousRunId is RUN-01.",
          [R("RUN-01", 1, [("TC_A", "skipped"), ("TC_B", "skipped")]), R("RUN-02", 2, mix(1, 1))],
          cases_of(mix(1, 1)), reqs=["SUT1-1"], flags=("--run", "RUN-02"))
    build(nm("exact-gate-hit-7-of-10-at-70"), "Exact gate hit 7 of 10 at 70",
          "7 of 10 = 70 exactly with gate 70: PASSED (inclusive boundary, exact integer compare).",
          [R("RUN-A", 1, mix(7, 3))], cases_of(mix(7, 3)), reqs=["SUT1-1"], flags=("--run", "RUN-A", "--gate", "70"))
    build(nm("untested-legacy-is-not-executed"), "Legacy untested counts as not executed",
          "One pass and one 'untested' (legacy): denominator 2, notExecuted 1, display 50, verdict FAILED at 90.",
          [R("RUN-A", 1, mix(1, 0, ne=0, skipped=0) + [("TC_002", "untested")])],
          [("TC_001", "SUT1-1"), ("TC_002", "SUT1-1")], reqs=["SUT1-1"], flags=("--run", "RUN-A"))

    print(f"metrics cases: {n}")


if __name__ == "__main__":
    main()


def extra():
    """Probes of rules the docs state precisely (added after the first run)."""
    from common import put as _put
    n = 71
    def nx(slug):
        nonlocal_counter.append(1)
        return f"{71 + len(nonlocal_counter):03d}-{slug}"
    nonlocal_counter = []
    # no verification-results.yaml at all: 0 steps, stepCoverage null
    r = R("RUN-NV", 1, mix(1, 1)); r["steps"] = None
    build(nx("no-verification-file-step-null"), "No verification-results.yaml: step coverage is null",
          "Run with no verification-results.yaml (optional). vvRun.totalSteps 0, passedSteps 0, stepCoverage null "
          "(VV_METRICS.md: null when the total is 0).", [r], cases_of(mix(1, 1)), reqs=["SUT1-1"], flags=("--run", "RUN-NV"))
    # a case without an entry in verification-results: 0 steps for it
    r = R("RUN-PART", 1, mix(2, 0)); r["steps"] = {"TC_001": (3, 3)}
    build(nx("case-without-step-entry-counts-zero"), "A case without a verification entry has 0 steps",
          "TC_002 has no entry in verification-results.yaml (VV_METRICS.md: 0 steps). totalSteps 3 (TC_001 only), "
          "passedSteps 3; stepCoverage 100.", [r], cases_of(mix(2, 0)), reqs=["SUT1-1"], flags=("--run", "RUN-PART"))
    # problem report on a passing case: withDefect 1, openDefects 0
    r = R("RUN-PRP", 1, [("TC_A", "pass", "PR-77"), ("TC_B", "fail")])
    build(nx("pass-with-problem-report"), "Problem report on a passing result is not open",
          "TC_A passes with PR-77 attached: the requirement SUT1-1 has withDefect 1 and openDefects 0 "
          "(openDefects counts reports on results that did not pass).", [r],
          [("TC_A", "SUT1-1"), ("TC_B", "SUT1-2")], reqs=["SUT1-1", "SUT1-2"], flags=("--run", "RUN-PRP"))
    # case file in a nested folder under the root is counted
    build(nx("case-file-in-subfolder-counted"), "A case file in a subfolder under the root counts",
          "The test case file is in sub/deeper/ under the root (VV_METRICS.md: any YAML under the root except runs/).",
          [R("RUN-SUB", 1, mix(1, 0))], [("TC_001", "SUT1-1")], reqs=["SUT1-1"], flags=("--run", "RUN-SUB"),
          extra_files={"sub/deeper/TC_001.yml": case_file("TC_001", "SUT1-1")}, write_root_cases=False)
    # a non-test_case YAML under the root is ignored, not an error
    build(nx("other-yaml-type-ignored"), "A YAML of another type under the root is ignored",
          "result.yml declares type test_result: it is not a test case, is not listed as schemaErrors and does not "
          "count in distinctCases.",
          [R("RUN-OT", 1, mix(1, 0))], [("TC_001", "SUT1-1")], reqs=["SUT1-1"], flags=("--run", "RUN-OT"),
          extra_files={"result.yml": "type: test_result\nschema: tcms/test-result.schema.v1.json\ntest_case_id: TC_ZZ\n"})
    # vv-config out-of-range values (docs: fails, names the key)
    build(nx("config-stability-window-0"), "vv-config stability_window_runs 0 refused",
          "stability_window_runs: 0 (must be at least 1): the file fails, exit 1.",
          [R("RUN-C", 1, mix(1, 0))], cases_of(mix(1, 0)), reqs=["SUT1-1"],
          config_text="type: vv-config\nschema: tcms/vv-config.schema.v1.json\nstability_window_runs: 0\n",
          flags=("--run", "RUN-C"), expect_exit=1)
    build(nx("config-type-key-not-checked"), "vv-config with type test_case is accepted",
          "DECISION (Opus, D-01): the type and schema keys are accepted with any value and not checked until the schema "
          "is vendored (TCMS-47.2 AC 7). Exit 0. --gate 90 is passed so the gate parameter source is 'flag'.",
          [R("RUN-C", 1, mix(1, 0))], cases_of(mix(1, 0)), reqs=["SUT1-1"],
          config_text="type: test_case\nschema: tcms/vv-config.schema.v1.json\ngate_threshold_percent: 90\n",
          flags=("--run", "RUN-C", "--gate", "90"))
    build(nx("flag-gate-over-file-stability"), "Flags override each key of vv-config independently",
          "vv-config sets gate 95, flaky 3, window 2; --flaky-threshold 5 only overrides flaky: gate source file, "
          "flaky source flag, window source file.",
          [R("RUN-C", 1, mix(9, 1))], cases_of(mix(9, 1)), reqs=["SUT1-1"],
          config={"gate_threshold_percent": 95, "flaky_flip_threshold": 3, "stability_window_runs": 2},
          flags=("--run", "RUN-C", "--flaky-threshold", "5"))
    build(nx("not-executed-only-is-failed-not-no-data"), "Only not_executed results: FAILED with denominator 1, not NO_DATA",
          "One case, not_executed. VV_METRICS.md: not_executed is in the denominator as not passed, so denominator 1, "
          "display 0, verdict FAILED. (report/README.md says a null passRate happens when no passed, failed or blocked "
          "result exists: that is inconsistent; see DISCREPANCES_TEST_CASES.md.)",
          [R("RUN-NE", 1, [("TC_001", "not_executed")])], [("TC_001", "SUT1-1")], reqs=["SUT1-1"], flags=("--run", "RUN-NE"))
    runs82=[R("RUN-0b",1,[],plan="P1"), R("RUN-1a",2,[("TC_01","pass")],plan="P2"), R("RUN-2b",3,[("TC_02","fail")])]
    build(nx("plan-filter-distinct-cases-all-runs"), "distinctCases counts all runs, not only the plan's",
          "DOCS (VV_METRICS.md Output): loadSummary.distinctCases counts the case files and the test cases of the verification "
          "results 'in all runs'. Here --plan P2 selects RUN-1a, but RUN-2b (no plan) has TC_02 and must still count: "
          "distinctCases 2. Logged as D-08 in DISCREPANCES_TEST_CASES.md.",
          runs82, [("TC_01","SUT1-1")], reqs=["SUT1-1"], flags=("--plan","P2"))
    print("extra metrics cases added")


if __name__ == "__main__" and "extra" not in sys.argv:
    pass
