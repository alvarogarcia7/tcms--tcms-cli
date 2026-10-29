# Independence between author and verifier

Some objectives require that whoever checks the results is not the person who wrote the test
case (TCMS-40.8). The test case records its `author`; the person who checks its results is the
`reviewer` of the test run. The tester may be the author; the reviewer may not.

## The check on a test run

The verifier (`--reviewer NAME`; it writes the test run by default, see `docs/RUN_METADATA.md`) refuses to write the run
when a test case in it has `independence_required: true` and

- the run has no reviewer, or
- the reviewer is the test case's `author` (names compare ignoring case and surrounding spaces), or
- the test case records no author, so the reviewer cannot be checked.

The error names each offending test case and no record is written. Test cases without
`independence_required` are unaffected: no reviewer is needed, and the tester may be the author.

```yaml
# test case
author: alice
independence_required: true
```

```bash
verifier ... --reviewer alice --test-run run.yaml   # refused: reviewer alice is the author
verifier ... --reviewer bob   --test-run run.yaml   # written
```

## Independence assignment records

An `independence-assignment` (tcms-schemas) records that an artifact was checked by someone other
than its author. JSON Schema cannot compare two fields, so `test-executor
independence-assignment` enforces the rule that a record with `verifier_is_independent: true`
must name a verifier other than the author.

```bash
# verifier_is_independent follows from author and verifier
test-executor independence-assignment create --id IA-001 \
    --objective "Test results reviewed by someone other than the author" \
    --artifact RUN-001 --author alice --verifier bob -o ia-001.yaml

# schema and cross-field check (exit 1 on any invalid record)
test-executor independence-assignment check ia-001.yaml records/*.yml
```

A verifier who is the author gives an honest record with `verifier_is_independent: false` and a
note; a record that claims otherwise is rejected by `check`.
