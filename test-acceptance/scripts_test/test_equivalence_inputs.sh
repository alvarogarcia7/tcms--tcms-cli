#!/usr/bin/env bash
# Equivalence of inputs (08-SPEC section 2.1, D-08-1).
#
# Rules checked through the real binaries (no mocks):
#   - every valid fixture (passes JSON schema validation) exits 0
#     (or a documented non-zero code other than 1; none are listed yet);
#   - every invalid fixture (fails JSON schema validation) exits exactly 1
#     (INVALID_INPUT_FILES) and the binary reports a schema violation.
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
COUNT=0

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
    # record STATUS DESCRIPTION
    COUNT=$((COUNT + 1))
    if [ "$1" = "ok" ]; then
        PASSED=$((PASSED + 1))
        pass "$2"
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
    elif ! grep -q "Schema" "$out"; then
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
done

for f in "$FIXTURES"/valid/runs/*.yml; do
    [ -f "$f" ] || continue
    check_valid "run record" "$f" "$VALIDATE_YAML" "$f"
done

for f in "$FIXTURES"/invalid/test-cases/*.yml; do
    [ -f "$f" ] || continue
    check_invalid "test case" "$f" "$TEST_EXECUTOR" generate "$f"
done

for f in "$FIXTURES"/invalid/runs/*.yml; do
    [ -f "$f" ] || continue
    check_invalid "run record" "$f" "$VALIDATE_YAML" "$f"
done

if [ "$COUNT" -eq 0 ]; then
    log_error "No fixtures found under $FIXTURES"
    exit 1
fi

info "Equivalence checks: $COUNT, passed: $PASSED, failed: $FAILED"
if [ "$FAILED" -ne 0 ]; then
    exit 1
fi
exit 0
