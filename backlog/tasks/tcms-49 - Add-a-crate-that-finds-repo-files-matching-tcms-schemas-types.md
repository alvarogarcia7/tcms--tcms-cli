---
id: TCMS-49
title: Add a crate that finds repo files matching tcms-schemas types
status: In Review
assignee: []
created_date: '2026-11-04T07:35:00Z'
updated_date: '2026-11-04T08:02:00Z'
labels: []
dependencies: []
ordinal: 92400
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users and agents need to know which YAML/JSON files in a repo are tcms-schemas documents (test cases, test runs, trace matrices, and so on) without grepping for schema URIs by hand. Tooling and agents also need a machine-readable listing. Matching is by the document's declared `schema:` URI, resolved against the vendored schemas in schemas/tcms-schemas/tcms/, and the document's `type:` is reported alongside it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A new binary crate named `find-files-by-schema` under crates/ is added to the workspace members. Its dependencies respect the layer rules in AGENTS.md (it may depend only on lower layers, and it is self-contained as a binary).
- [x] #2 By default the crate scans the current directory recursively. A `--root <DIR>` option overrides the scan root.
- [x] #3 By default the scan covers *.yaml, *.yml and *.json files and respects .gitignore.
- [x] #4 A file is reported when its `schema:` URI resolves to a vendored tcms-schemas file under schemas/tcms-schemas/tcms/. This includes the versioned aliases listed in testcase-common (for example tcms/test-case.schema.v1.json). Files without a `schema:` field, or with one that is not a tcms-schemas type, are not reported.
- [x] #5 Each reported file carries its path, its resolved schema file, and its declared `type:` value (null when absent).
- [x] #6 By default every tcms-schemas type is matched. The `--only <type>` option is repeatable and restricts matches to the named types. An unknown type name is a usage error with a non-zero exit code and a message listing the valid types.
- [x] #7 `--output=plaintext` is the default. It prints one human-readable line per match to stdout.
- [x] #8 `--output=jsonl` prints one JSON object per match to stdout, one per line, with fields `path`, `schema` and `type`. Nothing else goes to stdout in this mode.
- [x] #9 Diagnostics (malformed YAML/JSON, unreadable files) go to stderr as warnings. A malformed file is skipped and the scan continues.
- [x] #10 Output is sorted by path so that repeated runs produce identical output.
- [x] #11 The exit code is 0 for a completed scan, including a scan with zero matches. It is non-zero for usage errors and for an unreadable root.
- [x] #12 Tests cover: each output format; the default and `--only` type filters; the versioned aliases; a file with an unknown or missing `schema:`; a malformed file that is skipped with a warning; and a .gitignored file that is not scanned.
- [x] #13 crates/README.md is updated with the new crate, and the crate has its own README covering usage and both output formats.
- [ ] #14 `cargo test --all-features` and `make lint` pass, and the coverage threshold in AGENTS.md is still met.
- [x] #15 A `--verify` flag is added. It is off by default. When it is off, the output is exactly as without the flag.
- [x] #16 With `--verify`, each matched document's content is validated against the schema it declares (the schema its `schema:` URI resolves to, under the schemas root). Validation is local and uses no network.
- [x] #17 With `--verify` and `--output=jsonl`, each object gains `verified` (boolean) and `errors` (array of strings, each `<pointer>: <message>`, with `(root)` as the pointer for the document root). Without `--verify`, these fields are absent.
- [x] #18 With `--verify` and `--output=plaintext`, each match line gains `verified: ok` or `verified: FAILED (<n> errors)`, and failed matches list each error on an indented line below.
- [x] #19 With `--verify`, the exit code is 3 when at least one matched document fails verification. Output still lists every match. Exit codes 0, 1 and 2 keep their meaning.
- [x] #20 A declared schema that fails to compile is an error (exit 1) that names the schema file.
- [x] #21 The JSON Schema for the JSONL output accepts the optional `verified` and `errors` fields.
- [x] #22 Tests cover: a valid document; an invalid document with its error pointers; exit code 3; `--verify` absent leaving the output unchanged; a schema that fails to compile; and validation of the verified JSONL output against the published schema.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in crates/find-files-by-schema (commits 9b0aea8, 1ce4e4a, 1 more for --verify). Verified: cargo test -p find-files-by-schema (17 pass), cargo clippy -D warnings and cargo fmt --check clean for the crate, cargo llvm-cov 93% lines. Independent check: jsonschema 4.26 (via uv) validates all 281 verified JSONL lines of this repo against the published schema. Against this repo, --verify exits 3: 12 of 281 documents fail their declared schema. Not verified: AC #14 (make lint / make test / coverage-e2e for the workspace), blocked because openssl-sys needs system OpenSSL headers that this machine lacks (pulled in by git2 in other crates).
<!-- SECTION:NOTES:END -->
