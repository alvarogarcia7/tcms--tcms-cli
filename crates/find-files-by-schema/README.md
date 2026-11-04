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
```

| Option | Default | Meaning |
|--------|---------|---------|
| `--root <DIR>` | `.` | Directory to scan recursively |
| `--schemas-root <DIR>` | `schemas` | Directory that contains `tcms-schemas/tcms/` |
| `--only <TYPE>` | all types | Restrict matches to a type; repeatable; unknown names are an error |
| `--output=plaintext\|jsonl` | `plaintext` | Output format |

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

Matches are sorted by path. Diagnostics (malformed files) go to stderr as `warning:` lines and never to stdout.

## Exit codes

- `0`: scan completed, including zero matches
- `1`: unknown `--only` type, missing or unreadable schemas root, or unreadable scan root
- `2`: invalid command-line arguments (clap)

## Tests

```bash
cargo test -p find-files-by-schema
```
