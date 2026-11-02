#!/usr/bin/env bash
# Diagram check for the V&V data flow (Phase 8, ACC-01).
# Counts arrows, example invocations, step notes and DATA_FLOW.md table rows.
# Bash 3.2 and BSD/GNU safe. Exit 0 when all checks pass, 1 otherwise.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
# shellcheck source=scripts/lib/logger.sh
. "$REPO_ROOT/scripts/lib/logger.sh" || exit 1

MMD="$REPO_ROOT/docs/data_flow/2026-01-20_diagram_steps.mermaid"
DOC="$REPO_ROOT/docs/data_flow/DATA_FLOW.md"
EXPECTED_ARROWS=7
EXPECTED_STEPS=6
FAILED=0

check_eq() {
    # $1 = check name, $2 = actual, $3 = expected
    if [ "$2" -eq "$3" ]; then
        pass "$1: $2 (expected $3)"
    else
        fail "$1: got $2, expected $3"
        FAILED=$((FAILED + 1))
    fi
}

count() {
    # $1 = ERE pattern, $2 = file
    grep -cE -- "$1" "$2" 2>/dev/null || true
}

# Check 1: arrow markers
N_ARROWS=$(count '^ *%% vv-arrow A[0-9]+' "$MMD")
check_eq "1 arrow markers" "$N_ARROWS" "$EXPECTED_ARROWS"

# Check 2: example notes
N_EXAMPLES=$(grep -c -- ' example: ' "$MMD" 2>/dev/null || true)
check_eq "2 example notes" "$N_EXAMPLES" "$EXPECTED_ARROWS"
check_eq "2b example notes match arrows" "$N_EXAMPLES" "$N_ARROWS"

# Check 3: each example names exactly one tool; verifier and execute present
N_ONE_TOOL=0
N_VERIFIER=0
N_EXECUTE=0
while IFS= read -r line; do
    case "$line" in
        *" example: "*) ;;
        *) continue ;;
    esac
    tools=0
    case "$line" in *"test-executor "*) tools=$((tools + 1)) ;; esac
    case "$line" in *"verifier "*) tools=$((tools + 1)) ;; esac
    case "$line" in *"validate-yaml "*) tools=$((tools + 1)) ;; esac
    if [ "$tools" -eq 1 ]; then
        N_ONE_TOOL=$((N_ONE_TOOL + 1))
    fi
    case "$line" in *"verifier "*) N_VERIFIER=$((N_VERIFIER + 1)) ;; esac
    case "$line" in *"test-executor execute "*) N_EXECUTE=$((N_EXECUTE + 1)) ;; esac
done < "$MMD"
check_eq "3 examples with exactly one tool" "$N_ONE_TOOL" "$EXPECTED_ARROWS"
if [ "$N_VERIFIER" -ge 2 ]; then
    pass "3b verifier examples: $N_VERIFIER (at least 2)"
else
    fail "3b verifier examples: $N_VERIFIER, expected at least 2"
    FAILED=$((FAILED + 1))
fi
if [ "$N_EXECUTE" -ge 1 ]; then
    pass "3c test-executor execute examples: $N_EXECUTE (at least 1)"
else
    fail "3c test-executor execute examples: $N_EXECUTE, expected at least 1"
    FAILED=$((FAILED + 1))
fi

# Check 4: no forbidden old names, no TODO left in the mermaid source
N_RUN_MMD=$(count 'test-executor run' "$MMD")
N_RUN_DOC=$(count 'test-executor run' "$DOC")
N_TODO=$(count 'TODO' "$MMD")
check_eq "4a 'test-executor run' in mermaid" "$N_RUN_MMD" 0
check_eq "4b 'test-executor run' in DATA_FLOW.md" "$N_RUN_DOC" 0
check_eq "4c TODO in mermaid" "$N_TODO" 0

# Check 5: step notes with reads and writes
N_STEPS=$(count 'reads:.*writes:' "$MMD")
check_eq "5 step notes with reads and writes" "$N_STEPS" "$EXPECTED_STEPS"

# Check 6: the five allowed directories appear, and step notes use no other paths
for d in 'plans/' 'runs/' 'requirements/' 'tcms-config/' 'reports/'; do
    N_DIR=$(count "$d" "$MMD")
    if [ "$N_DIR" -ge 1 ]; then
        pass "6 directory $d appears ($N_DIR)"
    else
        fail "6 directory $d does not appear in mermaid"
        FAILED=$((FAILED + 1))
    fi
done
N_BAD_PATH=$(grep -E 'reads:.*writes:' "$MMD" | grep -cE 'testcases/|test-acceptance/|data/' || true)
check_eq "6b step notes using other paths" "$N_BAD_PATH" 0

# Check 7: DATA_FLOW.md table rows
N_A_ROWS=$(count '^\| A[0-9]+ \|' "$DOC")
N_S_ROWS=$(count '^\| S[0-9]+ \|' "$DOC")
check_eq "7a DATA_FLOW.md arrow rows" "$N_A_ROWS" "$EXPECTED_ARROWS"
check_eq "7b DATA_FLOW.md step rows" "$N_S_ROWS" "$EXPECTED_STEPS"

# Check 8: schema paths use the tcms-schemas tree
N_BAD_SCHEMA=$(grep -E 'schemas/tcms/test-run' "$MMD" "$DOC" | grep -c '' || true)
check_eq "8 wrong test-run schema path" "$N_BAD_SCHEMA" 0

# Check 9: mermaid syntax (warning only when mmdc is absent)
if command -v mmdc >/dev/null 2>&1; then
    if [ -z "${SCRATCH_DIR:-}" ]; then
        fail "9 mmdc present but SCRATCH_DIR is not set (use a scratch directory, not /tmp)"
        FAILED=$((FAILED + 1))
    elif mmdc -i "$MMD" -o "$SCRATCH_DIR/diagram_check.svg" >/dev/null 2>&1; then
        pass "9 mermaid syntax check passed (mmdc)"
    else
        fail "9 mermaid syntax check failed (mmdc)"
        FAILED=$((FAILED + 1))
    fi
else
    log_warning "mermaid syntax not checked: mmdc not installed"
fi

if [ "$FAILED" -eq 0 ]; then
    pass "All diagram checks passed"
    exit 0
fi
fail "$FAILED diagram check(s) failed"
exit 1
