# Phase 4: AsciiDoc and Machine-Readable Output - Context

**Gathered:** 2026-10-31
**Status:** Ready for planning
**Source:** REQUIREMENTS.md (OUT-01, OUT-02), SUMMARY.md (native renderers, no TPDG as primary path)

<domain>
## Phase Boundary

Every report kind (traceability, summary, gaps, problems) can be written as AsciiDoc, and as JSON or
YAML validated against a local report schema. The renderers are native, in process. TPDG and
asciidoctor are not used in this phase (HTML is v2, FOUT-01).

</domain>

<decisions>
## Implementation Decisions

- **D-01:** Format is chosen with `--format markdown|asciidoc|json|yaml` on `test-executor report`.
  Markdown stays the default (Phase 2).
- **D-02 (AsciiDoc, OUT-01):** All four kinds render in AsciiDoc. Cell content escapes the characters
  that break an AsciiDoc table (`|`, and line breaks become spaces); markup in test case text is
  escaped, not interpreted.
- **D-03 (JSON and YAML, OUT-02):** The same ReportModel is serialised. The document has the
  `type` and `schema` envelope fields, and the schema is a new local file under `schemas/` named
  for the report. The shared tcms-schemas files are not changed (SUMMARY.md, REQUIREMENTS out of
  scope).
- **D-04 (validation before write, OUT-02):** The JSON or YAML document is validated against the
  local schema with `jsonschema` (already used in the workspace) before the file is written. A
  validation failure writes no file and exits with the input/validation error code of Phase 5
  (until Phase 5 exists, exit 1).
- **D-05 (determinism, OUT-03):** Byte-identical output for the same inputs and `--generated-at`, in
  every format, as in Phase 2 D-12.
- **D-06 (escaping):** Hostile strings (pipes, newlines, AsciiDoc or Markdown markup) in ids, titles
  and texts do not break the table or the document structure. Tests use such strings.

### Claude's Discretion
- Exact AsciiDoc table options (cols, header row) and the schema's field names, provided D-02 to
  D-06 hold.

</decisions>

<canonical_refs>
- `.planning/REQUIREMENTS.md` - OUT-01, OUT-02
- `.planning/research/SUMMARY.md` - renderer choice and schema note (do not change shared schemas)
- `.planning/phases/02-traceability-report-markdown/02-CONTEXT.md` - ReportModel and Markdown renderer
- `.planning/phases/03-summary-gaps-and-problem-reports/03-CONTEXT.md` - the other three kinds
- `schemas/tcms-schemas/tcms/` - shared schemas (read only)
</canonical_refs>

<deferred>
- HTML via asciidoctor or TPDG: v2 (FOUT-01).
- CSV export: v2 (FOUT-02).
</deferred>
