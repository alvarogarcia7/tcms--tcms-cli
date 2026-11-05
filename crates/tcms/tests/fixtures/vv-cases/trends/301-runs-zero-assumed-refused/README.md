# 301-runs-zero-assumed-refused

ASSUMPTION: --runs 0 is not a valid count (the other numeric flags state a minimum of 1). The binary currently prints one row (the newest run) instead. Expected exit 1.

**Expected:** exit 1 (refused; nothing valid on stdout).

Arguments and expected values are in `case.json`; see the index `../README.md`.
