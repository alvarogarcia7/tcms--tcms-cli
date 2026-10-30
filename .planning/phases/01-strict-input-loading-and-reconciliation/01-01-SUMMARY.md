---
phase: 01-strict-input-loading-and-reconciliation
plan: 01
subsystem: reporting
tags: [rust, crate, loaders, rfc3339]
requires: []
provides:
  - testcase-reporting crate (Layer 3)
  - testcase_reporting::legacy loaders (read_test_runs, load_test_cases_strict, load_test_case)
  - LoadedRun with parsed started instant
affects: [01-02, 01-03]
tech-stack:
  added: []
  patterns: [behaviour-preserving move, RFC 3339 instant parsing once at load]
key-files:
  created:
    - crates/testcase-reporting/Cargo.toml
    - crates/testcase-reporting/src/lib.rs
    - crates/testcase-reporting/src/legacy.rs
    - crates/testcase-reporting/src/run.rs
    - crates/testcase-reporting/src/load.rs
    - crates/testcase-reporting/src/select.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/test-executor/Cargo.toml
    - crates/test-executor/src/main.rs
    - crates/README.md
key-decisions:
  - "load_test_case in test-executor stays as a one-line delegation so other call sites are unchanged"
requirements-completed: [CLI-01]
duration: 20min
completed: 2026-10-30
---

# Phase 1 Plan 01: testcase-reporting crate and legacy loaders Summary

New Layer 3 crate `testcase-reporting` holds the moved lenient loaders (unchanged behaviour) and `LoadedRun`, which parses `started_at` as an RFC 3339 instant and fails naming the file.

## Tasks
1. Crate, legacy loaders, LoadedRun, stub load.rs/select.rs: 6f0e436
2. test-executor rewired, README documented: c167658

## Verification
- testcase-reporting: 5 unit tests pass
- test-executor: all unit and integration tests pass unchanged (trace_matrix_cli 11, regression_select_cli 4, coverage_import_cli 3, independence_cli 3, bin 6)
- clippy -D warnings and fmt clean; only test-executor depends on testcase-reporting

## Deviations from Plan

**[Rule 3 - Blocking] libssl-dev not actually installed.** openssl-sys could not build for test-executor. No root access, so I built with `OPENSSL_DIR=/tmp/ossl OPENSSL_LIB_DIR=/tmp/ossl/lib OPENSSL_INCLUDE_DIR=/usr/include/node` (symlinks to system libssl.so.3/libcrypto.so.3 and the node OpenSSL headers). No repository change. Later plans need the same env vars.

**[Rule 1] Unused import** `testcase_common::load_and_validate_yaml` removed from main.rs after the move.

Not run: full `make test`, coverage and acceptance tests (CLAUDE.md pre-commit workflow); only the plan's verification commands were run.

## Known Stubs
load.rs and select.rs are doc-comment-only by design; owned by plans 01-02 and 01-03.

## Self-Check: PASSED
