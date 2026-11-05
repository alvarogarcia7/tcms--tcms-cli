# Flags override each key of vv-config independently

vv-config sets gate 95, flaky 3, window 2; --flaky-threshold 5 only overrides flaky: gate source file, flaky source flag, window source file.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
