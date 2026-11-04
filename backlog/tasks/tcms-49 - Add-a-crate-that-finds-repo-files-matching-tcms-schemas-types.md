---
id: TCMS-49
title: Add a crate that finds repo files matching tcms-schemas types
status: To Do
assignee: []
created_date: '2026-11-04T07:35:00Z'
updated_date: '2026-11-04T07:36:00Z'
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
- [ ] #1 A new binary crate named `find-files-by-schema` under crates/ is added to the workspace members. Its dependencies respect the layer rules in AGENTS.md (it may depend only on lower layers, and it is self-contained as a binary).
- [ ] #2 By default the crate scans the current directory recursively. A `--root <DIR>` option overrides the scan root.
- [ ] #3 By default the scan covers *.yaml, *.yml and *.json files and respects .gitignore.
- [ ] #4 A file is reported when its `schema:` URI resolves to a vendored tcms-schemas file under schemas/tcms-schemas/tcms/. This includes the versioned aliases listed in testcase-common (for example tcms/test-case.schema.v1.json). Files without a `schema:` field, or with one that is not a tcms-schemas type, are not reported.
- [ ] #5 Each reported file carries its path, its resolved schema file, and its declared `type:` value (null when absent).
- [ ] #6 By default every tcms-schemas type is matched. The `--only <type>` option is repeatable and restricts matches to the named types. An unknown type name is a usage error with a non-zero exit code and a message listing the valid types.
- [ ] #7 `--output=plaintext` is the default. It prints one human-readable line per match to stdout.
- [ ] #8 `--output=jsonl` prints one JSON object per match to stdout, one per line, with fields `path`, `schema` and `type`. Nothing else goes to stdout in this mode.
- [ ] #9 Diagnostics (malformed YAML/JSON, unreadable files) go to stderr as warnings. A malformed file is skipped and the scan continues.
- [ ] #10 Output is sorted by path so that repeated runs produce identical output.
- [ ] #11 The exit code is 0 for a completed scan, including a scan with zero matches. It is non-zero for usage errors and for an unreadable root.
- [ ] #12 Tests cover: each output format; the default and `--only` type filters; the versioned aliases; a file with an unknown or missing `schema:`; a malformed file that is skipped with a warning; and a .gitignored file that is not scanned.
- [ ] #13 crates/README.md is updated with the new crate, and the crate has its own README covering usage and both output formats.
- [ ] #14 `cargo test --all-features` and `make lint` pass, and the coverage threshold in AGENTS.md is still met.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
