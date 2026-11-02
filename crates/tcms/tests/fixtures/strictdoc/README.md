# StrictDoc test fixtures

These files are the input for the requirement-locate tests
(`crates/tcms/tests/requirements_locate_strictdoc_e2e.rs` and
`crates/tcms/tests/requirements_export_ids_strictdoc_e2e.rs`).

## What the files are

Each case is a directory with two parts:

- `<case>/src/*.sdoc`: small StrictDoc sources, written by hand and checked in.
- `<case>/export.json`: the JSON that the real StrictDoc tool writes from those
  sources (`<output>/json/index.json`). It is a generated file.

Cases:

| Case | Purpose |
|------|---------|
| `locate` | Accepted. Items at levels `system`, `HLR` and `LLR` with a Parent relation chain. |
| `refused-level` | Refused. HLR-8 has `LEVEL: chapter`, which is not a requirement type. |
| `no-level` | Refused. HLR-9 has no `LEVEL` field. |

The `refused-level` and `no-level` cases exist so the reader's refusals are tested
against real StrictDoc output, not hand-made JSON.

## Regeneration command

For each case, the exact command is:

```bash
uv run strictdoc export --formats json --output-dir <dir> crates/tcms/tests/fixtures/strictdoc/<case>/src
```

Then copy `<dir>/json/index.json` to `crates/tcms/tests/fixtures/strictdoc/<case>/export.json`.

Shortcuts (both run the same steps for all three cases):

```bash
make strictdoc-regen-test-fixtures          # write the three export.json files
make strictdoc-check-test-fixtures          # regenerate into a temp dir and diff; exit 1 on drift
bash crates/tcms/tests/fixtures/strictdoc/regenerate.sh [--check]
```

`regenerate.sh` runs from the repository root, so `uv run` uses the project's
`pyproject.toml` and `uv.lock`. It writes into a `mktemp -d` directory that it
removes on exit, and it never copies the `_cache` directory.

## Version

- The exports were generated with StrictDoc **0.20.0**, the version pinned in `uv.lock`
  (`uv run strictdoc --version` prints `0.20.0`).
- The planner also checked StrictDoc 0.30.4 on these sources (recorded 2026-10-31) and
  reported byte-identical `index.json`. That check is not repeated here.

## Running the regeneration

- Regeneration needs `uv`, Python and network access on a cold uv cache.
- `cargo test` never runs `regenerate.sh` or the make targets. The tests read the checked-in
  `export.json` files, need no network and no Python, and do not call StrictDoc.
- Do not add these targets to `make test` or CI (D-07-2).

## Rules for the exports

- Do not edit an `export.json` by hand. To change its content, change the `.sdoc` source,
  run the regeneration, and check the result.
- `regenerate.sh --check` (or `make strictdoc-check-test-fixtures`) fails when a checked-in
  export differs from a fresh export. Run it after changing any `.sdoc` file.

## Expected content of `locate/export.json`

The tests rely on this content:

- Two documents, in this order: `Locate Fixture Software`, then `Locate Fixture System`.
- `Locate Fixture Software` holds the sections "High-level requirements" (HLR-1) and
  "Low-level requirements" > "Parsing" (LLR-1).
- `Locate Fixture System` holds SYS-1.
- Requirements:
  - SYS-1: level `system`, status `approved`, title "Remote profile management".
  - HLR-1: level `HLR`, status `approved`, title "Update profile metadata", Parent relation to SYS-1.
  - LLR-1: level `LLR`, status `in_review`, title "Parse the update frame", Parent relation to HLR-1.
- The file starts with the StrictDoc metadata comment (`_COMMENT`), which shows it is the
  tool's own output.
