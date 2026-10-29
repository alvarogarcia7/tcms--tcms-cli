# Regression selection from a baseline change impact (TCMS-40.7)

Regression scope follows the change impact analysis recorded on a
[baseline](../schemas/tcms-schemas/tcms/baseline.schema.json), not "run everything".

```bash
test-executor regression-select testcases/ --baseline baselines/BL-002.yaml
test-executor regression-select testcases/ --baseline baselines/ --plan plans/TP-001.yaml \
    --format json -o regression.json
```

## What is selected

For a baseline with a `change_impact`, the scope is the union of

1. the test cases named in `change_impact.regression_test_cases` (matched on the test case `id`), and
2. every test case whose `requirements` list names a requirement in
   `change_impact.impacted_requirements` (the same link the [traceability matrix](TRACEABILITY.md) uses).

Obsolete test cases are never selected. Each selected case carries its reasons: `listed` and/or
the impacted requirement IDs it traces to.

## Regression plans

A test plan with a `regression` block names the baseline (`baseline_id`) and the baseline the
changes are measured from (`regression.previous_baseline_id`). With `--plan`, the command
picks the baseline file whose `id` is the plan's `baseline_id` out of the `--baseline` files or
directories, and requires the baseline's `previous_baseline_id` to equal the plan's
`regression.previous_baseline_id`. Without `--plan`, exactly one baseline must be given.

## Errors (exit code 1, message on stderr)

| Situation | Message contains |
|-----------|------------------|
| Baseline path does not exist | `Baseline not found: <path>` |
| Not YAML / not a mapping | `is not valid YAML` / `is not a YAML mapping` |
| Missing `id`, `build_id`, `created`, `items` | `required property '<name>' is missing` |
| `change_impact` without `previous_baseline_id` | `requires 'previous_baseline_id'` |
| Baseline without `change_impact` | `has no change_impact` |
| Plan without `regression` block | `not a regression plan` |
| Plan names a baseline not given | `no given baseline has that id` |
| Plan and baseline disagree on the previous baseline | `measures baseline ... from ...` |
| Several baselines, no plan | `no --plan to choose between them` |
| Baseline lists test case IDs that do not exist | the IDs; the selection is still written. `--allow-unknown-test-cases` accepts it |

Impacted requirements that no test case traces to produce a warning (a requirement changed
without a test is worth seeing, but it is not an error of the selection).

## Output

`--format ids` (default) writes one test case ID per line, suitable for scripts and list files;
`json` and `yaml` write `baseline_id`, `previous_baseline_id`, `selected` (ID and reasons),
`unknown_listed` and `untested_requirements`. `-o FILE` writes to a file instead of stdout.
