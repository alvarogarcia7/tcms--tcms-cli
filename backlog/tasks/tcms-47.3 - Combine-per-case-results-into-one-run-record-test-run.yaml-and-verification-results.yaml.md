---
id: TCMS-47.3
title: >-
  Combine per-case results into one run record (test-run.yaml and
  verification-results.yaml)
status: To Do
assignee: []
created_date: '2026-11-03T05:07:00Z'
labels:
  - tcms-cli
  - vv
  - converter
dependencies: []
parent_task_id: TCMS-47
ordinal: 91100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Context: tcms vv reads one run folder per run (runs/<RUN_ID>/test-run.yaml with a results list, plus an optional verification-results.yaml). The 63 per-case results (type: test_result, from convert_verification_to_result_yaml.py) are separate files, so the CLI cannot read them as one run. This task adds the combinator. Comparison with MR 31 of tcms-cli (TCMS-47.2: read the per-product vv-config.yaml and add a stability-window option to tcms vv metrics): TCMS-47.2 is the consumer side. It reads runs/<RUN_ID>/test-run.yaml, the case files and the config, and computes the metrics. TCMS-47.3 is the producer side: it writes the one runs/<RUN_ID>/test-run.yaml and verification-results.yaml that TCMS-47.2 and tcms vv metrics read. The two must agree on the run record layout, so TCMS-47.3 should land before TCMS-47.2 is tested end to end. Run records must follow the tcms-schemas test-run schema, as ORCH-033.01 requires. Steps: read the per-case files; take run_id and plan_id from the shared run metadata; start/finish from the earliest and latest case timestamps (execution logs in 10_test_results/execution_logs); map overall_pass to pass or fail and a missing run to not_executed; write one test-run.yaml with one results entry per test_case_id and one verification-results.yaml keyed by test_case_id; on duplicates the latest started_at wins and not_executed never overrides an executed result; validate both outputs against their schemas before writing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One runs/<RUN_ID>/test-run.yaml with one results entry per test_case_id, validated against the test-run schema
- [ ] #2 One verification-results.yaml keyed by test_case_id, validated against its schema
- [ ] #3 Duplicate case ids: latest started_at wins; not_executed never overrides an executed result
- [ ] #4 Verdict mapping: overall_pass true=pass, false=fail, missing run=not_executed
- [ ] #5 Equivalent to the legacy plain-folder single-run model (MR 31 ORCH-012.02): the folder is read as one run without history
- [ ] #6 A test reads the 63-file layout and checks the combined counts match the per-case inputs
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
