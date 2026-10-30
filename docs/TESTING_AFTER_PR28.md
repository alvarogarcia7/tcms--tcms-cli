# Where to start testing after MR !28

MR !28 (`tcms-40-glossary-impl`) adds the verification glossary entities (TCMS-40), verifier edge-case
handling (TCMS-41) and the Jira integration (TCMS-42). This page lists what to run, in order, and what
each step proves. Details of each feature are in the linked docs.

**Verified before the MR:** `cargo test --workspace --all-features` (2185 passed, 0 failed, 20 ignored),
`cargo fmt --check`, `cargo clippy`. **Not verified:** `make test`, the full acceptance suite, the GitLab
pipeline, and anything against a real Jira, Xray, Zephyr or StrictDoc. Start with steps 1 to 4; steps 5
and 6 are where a first failure is most likely.

## 0. Prerequisites

`jq`, `zip`, `python3` with `pyyaml` and `jsonschema`, `libssl-dev` and `pkg-config`, Rust 1.95, bash.
The acceptance suite also needs `test-plan-documentation-generator` (`cargo install
test-plan-documentation-generator` or `export TEST_PLAN_DOC_GEN=/path`) and `uv` (`make setup-python`).
StrictDoc is only needed to produce a JSON export.

```bash
git fetch && git checkout tcms-40-glossary-impl
```

## 1. Build, format, lint, unit tests

```bash
make build            # cargo build --workspace
make lint             # fmt + clippy
cargo test --workspace --all-features --no-fail-fast
```

Expected: no failures. `clippy::useless_borrows_in_formatting` in `log_cleaner.rs` is a known lint that
existed before this MR. If a test under `crates/testcase-manager/tests/dependencies_test.rs` cannot find
`validate-yaml`, build it first (`cargo build -p validate-yaml`); it looks in `<repo>/target/debug`.

One crate at a time:

```bash
cargo test -p verifier                     # includes the 62 edge-case tests (TCMS-41)
cargo test -p test-executor                # trace-matrix, regression, independence, coverage import
cargo test -p jira-client -p id-resolver -p requirement-import -p test-backends -p tcms
```

## 2. The suite the repository itself requires

```bash
make test                   # unit + e2e + verification
make coverage-e2e           # 70% threshold
make validate-output-schemas
make verify-scripts
```

`make validate-output-schemas` checks the verifier output against the vendored tcms-schemas
(`schemas/tcms-schemas/`, pinned by `scripts/sync-tcms-schemas.sh`; see `docs/TCMS_SCHEMAS.md`).

## 3. Acceptance suite (9 stages)

```bash
make acceptance-test
# or one stage at a time:
./test-acceptance/run_acceptance_suite.sh --verbose
./test-acceptance/validate_glossary_entities.sh     # stage 8: TCMS-40 entities
./test-acceptance/validate_jira_integration.sh      # stage 9: TCMS-42, 82 checks, mock Jira only
```

Reports land in `test-acceptance/reports/`. Stages 8 and 9 use the real binaries and fixtures in
`test-acceptance/glossary_entities/fixtures/`.

## 4. Behaviour changes to check by hand

These changed on purpose; confirm they suit your pipelines (`verifier`, TCMS-41).

| Case | New result |
|------|-----------|
| Empty logs folder | non-zero exit |
| Log that cannot be verified (unknown test case, wrong or missing source hash, a file that is not a log) | non-zero exit, even with `--success-on-completion` |
| Test case with zero steps | not a pass |
| Failed hook | non-zero exit |
| Manual step without a log entry | counts as failed |
| Output without `--output` | report goes to stdout |

Automated: `make test-verifier-behaviour` (or run
`crates/testcase-manager/tests/integration/test_verifier_behaviour_changes_e2e.sh`). It covers every row with
the real binaries, plus a control for each. A file that is not a JSON array and has no text-format line is
unverifiable (decision D-41f); a valid `[]` is still a failed verdict, which the flag forgives.

```bash
mkdir -p /tmp/empty-logs
verifier -f /tmp/empty-logs --test-case-dir testcases/; echo "exit=$?"     # expect non-zero
```

## 5. The TCMS-40 entities, by hand

```bash
# run metadata and a test run record (written by default, next to --output)
TCMS_RUN_ID=RUN-1 TCMS_BUILD_ID=b1 TCMS_TESTER=me \
  verifier -f logs/ --test-case-dir testcases/ --environment Staging \
    --output results.yaml                      # also writes results.test-run.yaml
verifier ... --no-test-run                     # skip the record
validate-yaml results.test-run.yaml

# traceability matrix and coverage check (needs normal-range and robustness tests per requirement)
test-executor trace-matrix testcases/ --requirements requirements/ --runs test-runs/ \
  --id TM-1 --baseline-id BL-001 --output trace-matrix.yaml

# regression selection, independence, coverage import
test-executor regression-select testcases/ --baseline baselines/BL-002.yaml
test-executor independence-assignment create --id IA-001 ...    # see docs/INDEPENDENCE.md
test-executor coverage-import --baseline-id BL-002 --run-id RUN-17 ...   # docs/COVERAGE_IMPORT.md
```

If metadata (run, build, environment, tester) is missing, the verifier warns and skips the record; it
fails only when `--test-run`, `--reviewer` or `--problem-report` was requested. Docs: `RUN_METADATA.md`,
`TRACEABILITY.md`, `REGRESSION.md`, `INDEPENDENCE.md`, `COVERAGE_IMPORT.md`, `GLOSSARY_ENTITIES.md`.

## 6. Jira integration against the mock

```bash
cargo build -p tcms -p jira-client
./target/debug/mock-jira --help                       # starts a local fake Jira
./target/debug/tcms --config examples/integration-config/jira_cloud.yaml jira test-connection
./target/debug/tcms ... jira sync --product P         # dry run by default
./target/debug/tcms ... jira sync --product P --apply # writes, mock only
```

Writes go through a guard and an audit log; nothing is written without `--apply`. See
`docs/JIRA_INTEGRATION.md` for the configuration, `pr push|link`, and the Xray / Zephyr Scale / Zephyr
Squad back ends.

## 7. Real systems (not yet done)

The API shapes for Xray, Zephyr and the StrictDoc export layout are assumptions (`backlog/decisions/
decision-1`). Before trusting them, run the same commands against a Jira test project:

1. `tcms jira test-connection` with a service account token (set the variable named in `auth.secret_ref`).
2. `tcms jira sync --product P` as a dry run and read the plan.
3. `--apply` on a throwaway project; check issues, the `tcms-pr-<id>` labels on defects, and that no token
   appears in the audit or log files.
4. Export StrictDoc to JSON and feed it to `test-executor trace-matrix --requirements export.json`.
5. Push a branch and let the `tcms:jira-sync` job run: it applies only on the default branch, and is a dry
   run elsewhere.

Defects created before the `tcms-pr-<id>` label existed need `tcms pr link`.

## 8. Reporting

Note failures with the command, the exit code and the output. Backlog: TCMS-40.1 stays open until the
upstream schema epic SCH-006 lands.
