---
id: TCMS-48.1
title: 'Phase 0: run the build and test baseline when the machine is idle'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run scripts/dx/measure-baseline.sh on a warm and, if disk allows, a cold target/. Record the report location and the top compile units, test binaries and slow test executables in this task. Needs about 17 GB free for a cold build.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Measurements (warm target/, git d83bb4c, 5 uncommitted files, 32 cores, machine idle)

Report: `/tmp/claude-1001/-home-paseo--paseo-worktrees-1m3ly0hg-paltry-turtle/751c11bd-4cc2-48b6-99cc-153c6db6f7d1/scratchpad/baseline-warm/` (summary.txt, timings/cargo-timing.html, unit-times.tsv, crate-times.tsv, test-binaries.tsv, test-runs.tsv)

- Workspace build (`cargo build --workspace --timings`): 8.67 s wall, 34 crates recompiled, exit 0.
- Slowest compile units: openssl 2.10 s, tcms 1.97 s, reqwest 1.53 s, jsonschema 1.40 s, test-executor 1.28 s, verifier 1.06 s.
- `cargo test --workspace --no-run`: 213.8 s wall, 111 test executables, 3.53 GB total.
- Largest test binaries: testcase-manager (39 exe, 1.66 GB), tcms (9 exe, 236 MB), test-backends (7 exe, 293 MB).
- Test-executable runs: 748.8 s total, of which 705.1 s is the `mock-jira` server binary (see below).
- Slowest test executables: json_escape_test 21.5 s, verifier_cli_test 4.1 s, hydration_test 0.8 s, edge_cases_test 0.8 s.

## Re-run after the script fix (warm target/, same commit)

Report: `/tmp/claude-1001/-home-paseo--paseo-worktrees-1m3ly0hg-paltry-turtle/751c11bd-4cc2-48b6-99cc-153c6db6f7d1/scratchpad/baseline-warm2/`

- Workspace build: 8.83 s wall, 34 crates recompiled.
- `cargo test --workspace --no-run`: 176.4 s wall, 111 executables. 100 are test harnesses; 11 plain bin targets are skipped.
- Test runs: 37.6 s total (was 748.8 s, of which 705 s was the mock-jira server).
- Slowest: json_escape_test 18.3 s, verifier_cli_test 4.2 s, hydration_test 1.2 s, edge_cases_test 0.9 s.
- One failing executable: `executor_test` (test_command_escaping_for_json_with_backslashes, see below).

## Problems found while measuring

- `scripts/dx/measure-baseline.sh` runs every `bin` target in step 3, including `mock-jira`, a long-running server. It blocked the run until it was killed manually, which is why its row shows 705 s and exit 143. Step 3 should run test kinds only, or wrap each run in a timeout.
- Failing test: `testcase-manager` `executor_test::test_command_escaping_for_json_with_backslashes` panics at `crates/testcase-manager/tests/executor_test.rs:5295` ("Backslashes should be properly escaped in JSON (doubled)"). 160 pass, 1 fails. Not investigated here; it is a correctness failure, not a timing one.
- Other non-zero exits from running binaries that take arguments without them: verifier, validate-yaml, test-executor, tcms (exit 1 or 2), and `tty_fallback_demo` (exit 101, not investigated).

## Not done

- Cold build: not run. `/home` had about 5.7 GB free at the start, and a cold build needs about 17 GB.
- `make test`: not run, so the definition-of-done item stays open.
