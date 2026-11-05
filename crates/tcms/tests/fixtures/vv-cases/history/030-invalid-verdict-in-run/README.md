# 030-invalid-verdict-in-run

A run has verdict 'maybe': the verdict vocabulary says any other value is an error: exit 1.

**Expected:** exit 1 (refused; nothing valid on stdout).

Arguments and expected values are in `case.json`; see the index `../README.md`.
