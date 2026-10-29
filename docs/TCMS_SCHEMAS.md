# tcms-schemas in the CLI

The record types of the verification glossary (test run, trace matrix, baseline, review,
coverage report, ...) are defined in the
[tcms-schemas](https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas)
project. The CLI keeps a **pinned, vendored copy**.

## How the CLI obtains tcms-schemas

| Item | Value |
|------|-------|
| Mechanism | Vendored copy at a pinned commit (no submodule, no build-time download) |
| Pinned version | `schemas/tcms-schemas/TCMS_SCHEMAS_VERSION` (repository and full commit SHA) |
| Schemas | `schemas/tcms-schemas/tcms/<name>.schema.json` |
| Samples | `schemas/tcms-schemas/samples/<category>/` |
| Update | `scripts/sync-tcms-schemas.sh [REF]`, then review `git diff`, `make test`, commit |

Why vendored: it works offline, in Docker and in CI without network or submodule setup, and a
schema change reaches the CLI only through a reviewed commit. The copy has its own directory,
mirroring the upstream paths, so it never mixes with the CLI's own `schemas/*.schema.json` and
`schemas/tcms/*.schema.v1.json`.

## Using a schema

A document selects its schema with the envelope field `schema`:

```yaml
type: test_run
schema: tcms/test-run.schema.json
```

`resolve_schema_uri` looks for the URI under the schemas root first (the CLI's own versioned
schemas, `tcms/test-case.schema.v1.json`) and then under `schemas/tcms-schemas/`
(`tcms/test-run.schema.json`). The CLI's own schema wins when both exist.

### The envelope

`schemas/tcms-envelope.schema.json` accepts `schema` URIs `tcms/<name>.schema.v<n>.json` (CLI) and
`tcms/<name>.schema.json` (tcms-schemas), and every tcms-schemas schema as `type`
(kebab-case name with underscores, `test-run` is `test_run`).

Some entities have their own domain field `type` (requirement, design-element, plan-document,
qa-record). A document of such an entity carries **only `schema:`**: that is all auto-resolution
reads, and `type` keeps its domain meaning.

The tcms-schemas samples carry no envelope. Validate them with an explicit schema
(`validate-yaml FILE --schema schemas/tcms-schemas/tcms/<name>.schema.json`) or add the
`schema:` line.

## Not yet unified (TCMS-40.1, criterion "no duplicate local copy")

The CLI still keeps its own `test-case`, `execution-log` and `verification-result` schemas
because the tcms-schemas versions are **less strict** than the CLI needs. Replacing them now
would silently drop validation. tcms-schemas must first absorb:

| Schema | Missing in tcms-schemas |
|--------|-------------------------|
| `test-case-main` | `hooks`, `hydration_vars`, `prerequisites`; the `include:` dependency list (`{id, test_sequence}`) under `general_initial_conditions` / `initial_conditions`, which today must be strings; boolean `verification.output` (manual steps) |
| `execution-log` | `hook_type`, `hook_path`, `result_verification_pass`, `output_verification_pass` (the CLI requires the two flags) |
| `verification-result` | `requirement`, `item`, `tc`, `source_yaml_sha256` as documented properties |

Until then both sets coexist; the outputs of the CLI validate against both.
