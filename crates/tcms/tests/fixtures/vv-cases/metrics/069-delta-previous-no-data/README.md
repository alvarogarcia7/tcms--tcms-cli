# Previous run NO_DATA: delta is null, previousRunId is set

RUN-01 has only skipped results (denominator 0). RUN-02 is 1 of 2 = 50. deltaFromPrevious null because one denominator is 0; previousRunId is RUN-01.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
