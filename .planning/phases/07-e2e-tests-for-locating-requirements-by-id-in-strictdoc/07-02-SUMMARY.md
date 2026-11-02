# Phase 7 Plan 02: Real StrictDoc fixture exports Summary

Checked-in real StrictDoc 0.20.0 JSON exports for locate, refused-level and no-level, with sources, a `--check` regeneration script, make targets and a README.

Requirement: LOOK-03 (not marked complete here; that is for a later plan).

## Commits

- `5b0f43a` feat(07-02): add real StrictDoc fixture exports and regeneration [ci skip]

## Files

- Created: `crates/tcms/tests/fixtures/strictdoc/README.md`
- Created: `crates/tcms/tests/fixtures/strictdoc/regenerate.sh` (executable)
- Created: `crates/tcms/tests/fixtures/strictdoc/locate/src/system.sdoc`, `locate/src/software.sdoc`
- Created: `crates/tcms/tests/fixtures/strictdoc/locate/export.json`
- Created: `crates/tcms/tests/fixtures/strictdoc/refused-level/src/refused.sdoc`, `refused-level/export.json`
- Created: `crates/tcms/tests/fixtures/strictdoc/no-level/src/nolevel.sdoc`, `no-level/export.json`
- Modified: `mk/strictdoc.mk` (two targets, two `.PHONY` lines, Targets comment)

The exports were written by `uv run strictdoc export --formats json` (not by hand). Regeneration is run on demand only (D-07-2); `cargo test` does not run it.

## StrictDoc version

`uv run strictdoc --version` printed `0.20.0`, the version pinned in `uv.lock`. The exports were generated with 0.20.0. The plan's README text said "generated with StrictDoc 0.30.4"; that was wrong for this run, so the README records 0.20.0 and notes the planner's 0.30.4 check without repeating it.

## How the script is run

```bash
bash crates/tcms/tests/fixtures/strictdoc/regenerate.sh          # write the three export.json files
bash crates/tcms/tests/fixtures/strictdoc/regenerate.sh --check  # diff a fresh export; exit 1 on drift
make strictdoc-regen-test-fixtures
make strictdoc-check-test-fixtures
```

Exit codes: 0 ok, 1 drift or tool failure, 2 usage or missing `uv`.

## Manual run of the real tcms binary (Task 2)

Config: `product_id: PROD-002`, `requirement_source: strictdoc`, one `strictdoc_html` system named `strictdoc` (base_url `https://pages.example.com/srs`, the plan's id_pattern, url_template and document_map), in a temp directory outside the repository (removed afterwards). Binary: `target/debug/tcms` built with `env -u CARGO_TARGET_DIR cargo build -p tcms`.

| Run | Exit | stdout | stderr |
|-----|------|--------|--------|
| `requirements locate SYS-1 HLR-1 LLR-1 --input locate/export.json --format plaintext` | 0 | three lines: `SYS-1: found in strictdoc (strictdoc) at level system "Remote profile management" [approved] https://pages.example.com/srs/system-requirements.html#SYS-1`, `HLR-1: ... at level HLR "Update profile metadata" [approved] .../software-requirements.html#HLR-1`, `LLR-1: ... at level LLR "Parse the update frame" [in_review] .../low-level-requirements.html#LLR-1` | empty |
| `requirements locate HLR-8 --input refused-level/export.json --format plaintext` | 2 | empty | `error: StrictDoc item HLR-8 in Refused Level: level 'chapter' is not a requirement type (...)` |
| `requirements locate HLR-9 --input no-level/export.json --format plaintext` | 2 | empty | `error: StrictDoc item HLR-9 in No Level: has no LEVEL; set LEVEL to one of the requirement types (...)` |

The locate lines match the Phase 6 contract (the plaintext form has no leading check mark).

## Gates

| Gate | Result |
|------|--------|
| `shellcheck crates/tcms/tests/fixtures/strictdoc/regenerate.sh` | exit 0 (after a `# shellcheck disable=SC1091` for the runtime-computed logger path) |
| `regenerate.sh` then `regenerate.sh --check` | both exit 0 |
| Drift: `echo ' ' >> locate/export.json`, then `--check` | exit 1; restored by running the script again, then `--check` exit 0 |
| `regenerate.sh --bogus` | exit 2 |
| `find ... -name _cache -o -name html` | nothing |
| `make -n strictdoc-regen-test-fixtures`, `make -n strictdoc-check-test-fixtures` | exit 0 both |
| `env -u CARGO_TARGET_DIR cargo test -p tcms --test requirements_locate_cli` | 23 passed, 0 failed |
| `cargo fmt --check` and `cargo test` for other crates | not applicable: no Rust file was touched |
| `make test`, `make acceptance-test`, `make coverage-e2e` | not run (outside the plan's gate rules) |

## Plan criteria

Task 1:
- Scripted generation exits 0 and creates the three export.json files: MET
- `--check` exits 0 right after; exits 1 after drift; `--bogus` exits 2: MET
- No `_cache` or `html` directory in the fixture: MET
- `strictdoc export --formats json` appears in regenerate.sh and README.md: MET
- README states that cargo test does not run the script: MET
- logger.sh sourced; no `declare -A`; shellcheck exit 0: MET
- `make -n` for both targets exits 0: MET

Task 2:
- Document titles equal `["Locate Fixture Software","Locate Fixture System"]`: MET
- Requirements `HLR-1:HLR:approved,LLR-1:LLR:in_review,SYS-1:system:approved`: MET
- LLR-1 Parent relation to HLR-1: MET
- refused-level has `HLR-8:chapter`: MET
- no-level item has no LEVEL field: MET
- `_COMMENT` starts with "Fields with _ are metadata": MET
- Real-binary run: exit 0 (locate), exit 2 (refused-level, message contains "is not a requirement type"), exit 2 (no-level, message contains "has no LEVEL"): MET

Plan-level:
- The reader of 07-01 accepts the locate export and refuses the other two (checked once with the real binary): MET
- `cargo test` needs neither network nor Python for these files: MET (the locate CLI suite runs offline; StrictDoc is not called by any test)

Overall status: all criteria MET. No criteria NOT MET or DEFERRED.

## Deviations

- **[Lint fix, no behaviour change] Shellcheck info finding:** SC1091 (not following a runtime-computed `source` path) made shellcheck exit 1 on an info note; added `# shellcheck disable=SC1091` with a comment. No behaviour change.
- **README version text:** the plan's wording ("generated with StrictDoc 0.30.4") did not match the real run (0.20.0); the README states 0.20.0 and records the planner's 0.30.4 check as the planner's claim only.

## Known Stubs

None.

## Self-Check: PASSED

- Files listed above exist on disk (verified by `find` and `git status`).
- Commit `5b0f43a` exists.
