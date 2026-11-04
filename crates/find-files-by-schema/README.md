# find-files-by-schema

Lists the YAML and JSON files of a repository that declare a tcms-schemas type. A file matches when its top-level `schema:` URI resolves to a vendored schema under `<schemas-root>/tcms-schemas/tcms/`, including the versioned aliases such as `tcms/test-case.schema.v1.json`. The declared `type:` value is reported with each match.

## Usage

```bash
# Human-readable (default)
find-files-by-schema --schemas-root schemas --root testcases

# One JSON object per match, one per line (for agents)
find-files-by-schema --schemas-root schemas --root . --output=jsonl

# Only some types (repeatable); names are the schema file name without ".schema.json"
find-files-by-schema --only test-run --only trace-matrix

# Also validate each document's content against the schema it declares
find-files-by-schema --verify --output=jsonl
```

| Option | Default | Meaning |
|--------|---------|---------|
| `--root <DIR>` | `.` | Directory to scan recursively |
| `--schemas-root <DIR>` | `schemas` | Directory that contains `tcms-schemas/tcms/` |
| `--only <TYPE>` | all types | Restrict matches to a type; repeatable; unknown names are an error |
| `--output=plaintext\|jsonl` | `plaintext` | Output format |
| `--verify` | off | Validate each match's content against its declared schema (local, no network) |

Scanned files are `*.yaml`, `*.yml` and `*.json`. Files ignored by `.gitignore` are not scanned.

## Output

Plaintext, one line per match:

```
cases/tc.yaml  type: test-case  schema: tcms/test-case-main.schema.json
matrix.json  type: -  schema: tcms/trace-matrix.schema.json
```

JSONL, one object per match (`type` is `null` when the document has none):

```json
{"path":"cases/tc.yaml","schema":"tcms/test-case-main.schema.json","type":"test-case"}
```

With `--verify`, `jsonl` objects gain `verified` (boolean) and `errors` (array of `<pointer>: <message>`, `(root)` for the document root); plaintext lines gain `verified: ok` or `verified: FAILED (<n> errors)`, with each error on an indented line below. Without `--verify` these fields are absent.

Matches are sorted by path. The JSONL shape is published as JSON Schema (draft-07) in [`schema/find-files-by-schema-match.schema.json`](schema/find-files-by-schema-match.schema.json); the test suite validates the binary's output against it. Diagnostics (malformed files) go to stderr as `warning:` lines and never to stdout.

## Exit codes

- `0`: scan completed, including zero matches
- `1`: unknown `--only` type, missing or unreadable schemas root, unreadable scan root, or a declared schema that fails to compile
- `2`: invalid command-line arguments (clap)
- `3`: `--verify` is on and at least one matched document fails verification (all matches are still listed)

## Tests

```bash
cargo test -p find-files-by-schema
```
