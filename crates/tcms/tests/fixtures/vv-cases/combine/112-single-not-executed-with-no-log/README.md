# 112-single-not-executed-with-no-log

One not_executed case and no execution log at all: started_at cannot be derived, and the test-run schema requires it (RUN_RECORD_FORMAT.md). Refused, exit 1, nothing written.

**Expected:** exit 1 (refused; nothing valid on stdout).

Arguments and expected values are in `case.json`; see the index `../README.md`.
