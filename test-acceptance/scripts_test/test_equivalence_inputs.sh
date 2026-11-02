#!/usr/bin/env bash
# Equivalence of inputs (08-SPEC section 2.1, D-08-1).
#
# Rules checked through the real binaries (no mocks):
#   - every valid fixture (passes JSON schema validation) exits 0
#     (or a documented non-zero code other than 1; none are listed yet);
#   - every invalid fixture (fails JSON schema validation) exits exactly 1
#     (INVALID_INPUT_FILES) and the binary reports a schema violation.
#
# Python cross-check (spec 2.1): every fixture is also validated with Python
# jsonschema against the schema its `schema:` field resolves to (same rules as
# the Rust loader). Runs with `python3 -I`, or `uv run --no-sync python -I` when
# PyYAML and jsonschema are only importable there. If neither runner can import
# them, every skipped check is counted and the run fails unless SKIP_PYTHON_CHECK=1.
#
# Test cases are run through `test-executor generate`; run records through
# `validate-yaml` (the same schema loader the report path uses).
#
# Usage: test-acceptance/scripts_test/test_equivalence_inputs.sh
# Override BIN_DIR to point at another build of the binaries.
# bash 3.2 and BSD/GNU safe.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../../scripts/lib/logger.sh
source "$ROOT/scripts/lib/logger.sh" || exit 1

FIXTURES="$ROOT/test-acceptance/equivalence"
BIN_DIR="${BIN_DIR:-$ROOT/target/debug}"
TEST_EXECUTOR="$BIN_DIR/test-executor"
VALIDATE_YAML="$BIN_DIR/validate-yaml"

# Documented non-zero exit codes allowed for VALID inputs. Code 1 is never allowed
# (INVALID_INPUT_FILES). Empty: no valid-input code other than 0 is documented yet.
DOCUMENTED_VALID_CODES=""

INVALID_INPUT_FILES=1

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/equivalence.XXXXXX")" || exit 1
setup_cleanup "$WORK_DIR"

for bin in "$TEST_EXECUTOR" "$VALIDATE_YAML"; do
    if [ ! -x "$bin" ]; then
        log_error "Binary not found: $bin (build with: env -u CARGO_TARGET_DIR cargo build -p test-executor -p validate-yaml)"
        exit 1
    fi
done

PASSED=0
FAILED=0
SKIPPED=0
COUNT=0

# SKIP_PYTHON_CHECK=1 explicitly allows skipping the Python cross-check when no runner is
# available. By default a missing runner fails the test (skipped fixtures are counted as failures
# of the run, not silently dropped).
SKIP_PYTHON_CHECK="${SKIP_PYTHON_CHECK:-0}"

# Python cross-check runner: "system" (python3 -I), "uv" (uv run --no-sync python -I), or "" (skipped).
# --no-sync keeps uv from syncing the environment or touching the network.
PY_MODE=""
if command -v python3 >/dev/null 2>&1 && python3 -I -c "import yaml, jsonschema" >/dev/null 2>&1; then
    PY_MODE="system"
elif command -v uv >/dev/null 2>&1 && (cd "$ROOT" && uv run --no-sync python -I -c "import yaml, jsonschema" >/dev/null 2>&1); then
    PY_MODE="uv"
fi

# Loads a fixture with PyYAML (timestamps kept as strings, as the Rust loader reads them),
# resolves its `schema:` URI like resolve_schema_uri, checks the envelope type of the versioned
# URIs, and validates with jsonschema. argv: ROOT FILE EXPECT(valid|invalid).
# Exit 0 when jsonschema agrees with EXPECT, 1 when it disagrees, 2 on a load/resolve error.
PY_SRC='
import json
import os
import sys

import jsonschema
import yaml

ALIASES = {
    "tcms/test-case.schema.v1.json": "tcms/test-case-main.schema.json",
    "tcms/test-execution.schema.v1.json": "tcms/execution-log.schema.json",
    "tcms/test-verification.schema.v1.json": "tcms/verification-result.schema.json",
}
ALIAS_TYPES = {
    "tcms/test-case.schema.v1.json": "test_case",
    "tcms/test-execution.schema.v1.json": "test_execution",
    "tcms/test-verification.schema.v1.json": "test_verification",
}


class Loader(yaml.SafeLoader):
    pass


Loader.yaml_implicit_resolvers = {
    key: [(tag, rx) for tag, rx in resolvers if tag != "tag:yaml.org,2002:timestamp"]
    for key, resolvers in yaml.SafeLoader.yaml_implicit_resolvers.items()
}

root, path, expect = sys.argv[1], sys.argv[2], sys.argv[3]
with open(path, encoding="utf-8") as fh:
    doc = yaml.load(fh, Loader=Loader)
if not isinstance(doc, dict):
    print("document is not a mapping")
    sys.exit(2)
uri = doc.get("schema")
if not isinstance(uri, str):
    print("document has no schema field")
    sys.exit(2)

own = os.path.join(root, "schemas", uri)
if os.path.exists(own):
    schema_path = own
elif uri in ALIASES:
    schema_path = os.path.join(root, "schemas", "tcms-schemas", ALIASES[uri])
else:
    schema_path = os.path.join(root, "schemas", "tcms-schemas", uri)
if not os.path.exists(schema_path):
    print("schema not found: " + schema_path)
    sys.exit(2)
with open(schema_path, encoding="utf-8") as fh:
    schema = json.load(fh)

errors = []
if uri in ALIAS_TYPES and doc.get("type") != ALIAS_TYPES[uri]:
    errors.append("envelope type is not " + ALIAS_TYPES[uri])
validator = jsonschema.validators.validator_for(schema)(schema)
errors.extend(e.message for e in validator.iter_errors(doc))

verdict = "valid" if not errors else "invalid"
if verdict == expect:
    sys.exit(0)
print("expected " + expect + ", jsonschema says " + verdict + ": " + "; ".join(errors))
sys.exit(1)
'

py_run() {
    if [ "$PY_MODE" = "uv" ]; then
        (cd "$ROOT" && uv run --no-sync python -I "$@")
    else
        python3 -I "$@"
    fi
}

# check_python LABEL FILE EXPECT: jsonschema (via Python) must agree with EXPECT.
# With no runner, the fixture is recorded as skipped (counted, never silently dropped).
check_python() {
    local label="$1"
    local file="$2"
    local expect="$3"
    if [ -z "$PY_MODE" ]; then
        record skip "python jsonschema $expect: $label $file (no Python runner)"
        return 0
    fi
    local out
    out="$(py_run -c "$PY_SRC" "$ROOT" "$file" "$expect" 2>&1)"
    local rc=$?
    if [ "$rc" -eq 0 ]; then
        record ok "python jsonschema $expect: $label $file"
    else
        record fail "python jsonschema $expect: $label $file ($out)"
    fi
}

if [ -z "$PY_MODE" ]; then
    if [ "$SKIP_PYTHON_CHECK" = "1" ]; then
        log_warning "SKIP_PYTHON_CHECK=1: Python jsonschema cross-check will be SKIPPED. PyYAML and jsonschema are not importable with python3 -I or uv run --no-sync python -I. This run does not verify the Python cross-check."
    else
        log_warning "No Python runner: PyYAML and jsonschema are not importable with python3 -I or uv run --no-sync python -I. Skipped checks will fail this run (set SKIP_PYTHON_CHECK=1 to allow skipping)."
    fi
else
    log_info "Python jsonschema cross-check runner: $PY_MODE"
fi

# is_documented_valid_code CODE: true when CODE is 0 or listed in DOCUMENTED_VALID_CODES.
is_documented_valid_code() {
    if [ "$1" -eq 0 ]; then
        return 0
    fi
    for code in $DOCUMENTED_VALID_CODES; do
        if [ "$1" -eq "$code" ]; then
            return 0
        fi
    done
    return 1
}

record() {
    # record STATUS DESCRIPTION  (STATUS: ok | fail | skip)
    COUNT=$((COUNT + 1))
    if [ "$1" = "ok" ]; then
        PASSED=$((PASSED + 1))
        pass "$2"
    elif [ "$1" = "skip" ]; then
        SKIPPED=$((SKIPPED + 1))
        log_warning "SKIPPED: $2"
    else
        FAILED=$((FAILED + 1))
        fail "$2"
    fi
}

# check_valid LABEL FILE CMD...: the command must exit 0 (or a documented code).
check_valid() {
    local label="$1"
    local file="$2"
    shift 2
    local out="$WORK_DIR/out.$COUNT"
    "$@" >"$out" 2>&1
    local rc=$?
    if [ "$rc" -eq "$INVALID_INPUT_FILES" ] || ! is_documented_valid_code "$rc"; then
        record fail "valid $label: $file exited $rc (expected 0)"
        sed 's/^/    /' "$out" >&2
    else
        record ok "valid $label: $file exited $rc"
    fi
}

# check_invalid LABEL FILE CMD...: exit exactly 1 and report a schema violation.
check_invalid() {
    local label="$1"
    local file="$2"
    shift 2
    local out="$WORK_DIR/out.$COUNT"
    "$@" >"$out" 2>&1
    local rc=$?
    if [ "$rc" -ne "$INVALID_INPUT_FILES" ]; then
        record fail "invalid $label: $file exited $rc (expected $INVALID_INPUT_FILES)"
        sed 's/^/    /' "$out" >&2
    elif ! grep -Eq "Schema (validation failed|constraint violations)" "$out"; then
        record fail "invalid $label: $file exited 1 without a schema violation message"
        sed 's/^/    /' "$out" >&2
    else
        record ok "invalid $label: $file exited $rc (schema violation)"
    fi
}

section "Equivalence of inputs (spec 2.1, D-08-1)"

for f in "$FIXTURES"/valid/test-cases/*.yml; do
    [ -f "$f" ] || continue
    check_valid "test case" "$f" "$TEST_EXECUTOR" generate "$f"
    check_python "test case" "$f" valid
done

for f in "$FIXTURES"/valid/runs/*.yml; do
    [ -f "$f" ] || continue
    check_valid "run record" "$f" "$VALIDATE_YAML" "$f"
    check_python "run record" "$f" valid
done

for f in "$FIXTURES"/invalid/test-cases/*.yml; do
    [ -f "$f" ] || continue
    check_invalid "test case" "$f" "$TEST_EXECUTOR" generate "$f"
    check_python "test case" "$f" invalid
done

for f in "$FIXTURES"/invalid/runs/*.yml; do
    [ -f "$f" ] || continue
    check_invalid "run record" "$f" "$VALIDATE_YAML" "$f"
    check_python "run record" "$f" invalid
done

if [ "$COUNT" -eq 0 ]; then
    log_error "No fixtures found under $FIXTURES"
    exit 1
fi

info "Equivalence checks: $COUNT, passed: $PASSED, failed: $FAILED, skipped: $SKIPPED"
if [ "$FAILED" -ne 0 ]; then
    exit 1
fi
if [ "$SKIPPED" -ne 0 ]; then
    if [ "$SKIP_PYTHON_CHECK" = "1" ]; then
        log_warning "SKIP_PYTHON_CHECK=1: $SKIPPED Python jsonschema check(s) skipped. Exiting 0 only because skipping was explicitly allowed."
        exit 0
    fi
    log_error "$SKIPPED Python jsonschema check(s) skipped: no Python runner (PyYAML + jsonschema). Failing. Install them for python3 or uv, or set SKIP_PYTHON_CHECK=1 to allow skipping."
    exit 1
fi
exit 0
