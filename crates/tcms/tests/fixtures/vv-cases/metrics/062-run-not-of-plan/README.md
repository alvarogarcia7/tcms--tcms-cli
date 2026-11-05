# --run must belong to --plan

RUN-P2 exists but has PLAN-2; --plan PLAN-1 --run RUN-P2 must fail (TCMS-50.6 and the --plan rule).

**Expected:** exit 1 (refused; nothing valid on stdout).

Arguments and expected values are in `case.json`; see the index `../README.md`.
