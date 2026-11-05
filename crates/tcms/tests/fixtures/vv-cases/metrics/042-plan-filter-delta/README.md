# Plan filter: delta against the previous run of the same plan

Three runs over two plans. With --plan PLAN-1 the previous run of RUN-X2 is RUN-X1 (not RUN-Y1): delta uses 50 -> 90.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
