# Importing coverage tool output (TCMS-40.10)

`test-executor coverage-import` converts coverage tool output into a tcms-schemas
[coverage-report](../schemas/tcms-schemas/tcms/coverage-report.schema.json)
(`type: coverage_report`). Structural coverage comes from tools; the CLI does not measure it. It
converts the figures and makes sure every gap has a disposition.

```bash
test-executor coverage-import --baseline-id BL-002 --run-id RUN-17 \
    --lcov build/coverage.info --mcdc mcdc.yaml \
    --trace-matrix reports/trace-matrix.yaml \
    --annotations gap-annotations.yaml \
    --tool gcov --tool-version 13 -o reports/coverage-report.yaml
```

At least one of `--lcov`, `--mcdc`, `--trace-matrix` is needed: the schema wants
requirements-based figures, structural figures, or both. `--baseline-id` is required.

## Input formats

### LCOV (`--lcov`): statement and decision coverage

A standard LCOV tracefile (`lcov`, `gcovr --lcov`, `llvm-cov export -format=lcov`). Used records:
`SF` (source file, becomes the code unit ID), `DA:<line>,<count>` (statements),
`BRDA:<line>,<block>,<branch>,<taken|->` (branch outcomes, treated as **decisions**), and
`end_of_record`. Other records are ignored; totals are computed from `DA` and `BRDA`, not from
`LF`/`LH`/`BRF`/`BRH`. Percentages carry two decimals. Overall figures go to `structural`,
per-file figures to `per_code_unit`.

Gaps found: a run of consecutive uncovered statements becomes one `statement` gap
(`src/a.c:12-15`), each line with a branch not taken becomes a `decision` gap (`src/a.c:12`).

### MC/DC side file (`--mcdc`)

LCOV has no MC/DC and no standard format exists, so the figures come from this YAML file
(unknown keys are rejected):

```yaml
mcdc_percent: 96.2              # overall, optional
units:                          # optional, per code unit (same IDs as LCOV file paths)
  - code_unit_id: src/es6.c
    mcdc_percent: 95.0
gaps:                           # optional, conditions without an independence pair
  - location: src/es6.c:88
```

Each gap becomes an `mcdc` gap.

### Traceability matrix (`--trace-matrix`): requirements-based coverage

The output of `test-executor trace-matrix` ([TRACEABILITY.md](TRACEABILITY.md)). It gives
`requirements_based` (total, covered normal range, covered robustness, uncovered) and
`per_requirement`. A requirement lacking a normal-range or robustness test is a
`requirements` gap whose location is the requirement ID.

## Gap annotations (`--annotations`)

A YAML list (or a mapping with an `annotations` list). Gaps are dispositioned, not patched: each
needs a classification and a rationale.

```yaml
- location: src/es6.c:210
  coverage_type: decision          # optional, narrows the match
  classification: deactivated_code # missing_test | missing_requirement | dead_code |
                                   # deactivated_code | justified | tool_limitation
  rationale: Debug branch disabled by build configuration; shown unable to activate.
  approver: qa.lead                # required for justified
  # optional links: code_unit_id, requirement_id, test_case_id, problem_report_id, action_item_id
```

`location` matches a gap when it is the gap's exact location, a source file path (every gap of
that file, narrowed by `coverage_type`), or `file:LINE` or `file:A-B` lying inside the gap's line
range. The first matching annotation wins. An annotation without classification or rationale,
with an unknown classification, or `justified` without an approver, is an error.

## Unclassified gaps

A gap with no matching annotation cannot be written (the schema requires classification and
rationale on every gap). It is **listed on stderr** with location and coverage type and the
command exits with code 1; the report is still written, with the figures showing the shortfall
but without the unclassified gaps. `--allow-unclassified` accepts them (exit 0). Annotations that
match no gap give a warning, since they are stale or mistyped.

## Not covered

Data and control coupling (`coupling` in the schema) are not imported; MC/DC analysis itself is
the tool's job. Dead code must still be removed and deactivated code shown unable to activate:
the rationale records that argument, the CLI does not check it.
