# Stability: 5 flips is FLAKY

Six runs; TC_FLIP alternates (5 flips, FLAKY at the default threshold 4); TC_STEADY passes throughout (STABLE); TC_FAILS fails in runs 1-5 and passes in run 6 (1 flip, STABLE).

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
