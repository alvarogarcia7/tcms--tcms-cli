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

`resolve_schema_uri` resolves the URI in this order:

1. The three **versioned URIs** are aliases of vendored files. The CLI keeps no copy of these
   schemas (TCMS-40.1):

   | URI in the document | Vendored file |
   |---------------------|---------------|
   | `tcms/test-case.schema.v1.json` | `schemas/tcms-schemas/tcms/test-case-main.schema.json` |
   | `tcms/test-execution.schema.v1.json` | `schemas/tcms-schemas/tcms/execution-log.schema.json` |
   | `tcms/test-verification.schema.v1.json` | `schemas/tcms-schemas/tcms/verification-result.schema.json` |

   A file that exists at the URI under the given `--schemas-root` still wins, so a schemas root of
   its own (tests, projects) can define them.
2. Any other URI is looked up under the schemas root (the CLI's own `tcms/*.schema.v1.json` such
   as `test-result`, `test-results-container`, `container-config`), and then under
   `schemas/tcms-schemas/` (`tcms/test-run.schema.json`).

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

## What is still the CLI's own

`test-case`, `execution-log` and `verification-result` come from tcms-schemas since SCH-006 (merge
request !11 of tcms-schemas, pinned at the head of its branch until it is merged). The CLI still
owns `test-result`, `test-results-container`, `container-config`, `verification-output`, the
`verification_methods` and the envelope meta-schema. TCMS-40.12 decides for each whether it moves
upstream or stays CLI-only.

## Two things that changed with the move

- **The envelope is no longer part of the test-case, execution-log and verification-result
  schemas.** The CLI copies required `type` and `schema` inside the schema; the upstream schemas
  are envelope-free (SCH-006.07). The envelope is checked where the schema is chosen:
  `schema:` must be present to resolve a schema, and `tcms-envelope.schema.json` accepts the
  `type` values.
- **`result_verification_pass` and `output_verification_pass` are optional** in the upstream
  execution-log schema (the CLI copy required them). The generated scripts always write them.

Invalid samples in tcms-schemas (`samples-invalid/`) keep the new constraints from being lost.
