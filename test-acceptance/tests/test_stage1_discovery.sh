#!/usr/bin/env bash
# Count guard and extension tests for the shared test case discovery used by
# acceptance suite Stage 1 and Stage 2 (TCMS-45 AC 1, decisions D-45-1 and D-45-2).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_CASES_DIR="$PROJECT_ROOT/test-acceptance/00_test_cases"

source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1
source "$PROJECT_ROOT/test-acceptance/lib/discover_test_cases.sh" || {
    echo "FAIL: discovery library test-acceptance/lib/discover_test_cases.sh is missing" >&2
    exit 1
}

FAILED=0
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/stage1_discovery.XXXXXX")" || exit 1
trap 'rm -rf "$SCRATCH"' EXIT

# Fills the global array DISCOVERED with the NUL-delimited output of discover_test_case_files.
collect() {
    DISCOVERED=()
    while IFS= read -r -d '' entry; do
        DISCOVERED+=("$entry")
    done < <(discover_test_case_files "$1")
}

check_eq() {
    local label="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        pass "$label"
    else
        fail "$label (expected '$expected', got '$actual')"
        FAILED=$((FAILED + 1))
    fi
}

# Test A: fixture with a known count. 4 test case files (3 .yml, 1 .yaml, one in a
# subdirectory) plus one ignored .txt file; discovery must return exactly 4.
FIX_A="$SCRATCH/a"
mkdir -p "$FIX_A/sub"
: > "$FIX_A/one.yml"
: > "$FIX_A/two.yaml"
: > "$FIX_A/four.yml"
: > "$FIX_A/sub/three.yml"
: > "$FIX_A/notes.txt"
KNOWN_FIXTURE_COUNT=4
collect "$FIX_A"
check_eq "A1: discovered count equals the fixture's known count ($KNOWN_FIXTURE_COUNT)" "$KNOWN_FIXTURE_COUNT" "${#DISCOVERED[@]}"

# Test A2/A3: the real tree yields 89 files, and the .yaml extension is present in none of them.
collect "$TEST_CASES_DIR"
check_eq "A2: discovery count on current tree is 89" "89" "${#DISCOVERED[@]}"

YAML_COUNT=0
for entry in "${DISCOVERED[@]}"; do
    case "$entry" in
        *.yaml) YAML_COUNT=$((YAML_COUNT + 1)) ;;
    esac
done
check_eq "A3: current tree has 0 .yaml files discovered" "0" "$YAML_COUNT"

# Test B: both extensions are matched; other extensions are ignored.
FIX_B="$SCRATCH/b"
mkdir -p "$FIX_B"
: > "$FIX_B/a.yml"
: > "$FIX_B/b.yaml"
: > "$FIX_B/c.txt"
collect "$FIX_B"
check_eq "B1: fixture with a.yml, b.yaml, c.txt yields 2 files" "2" "${#DISCOVERED[@]}"
check_eq "B2: first match is a.yml" "a.yml" "$(basename "${DISCOVERED[0]}")"
check_eq "B3: second match is b.yaml" "b.yaml" "$(basename "${DISCOVERED[1]}")"

# Test C: an empty discovery is a failure for the stage.
FIX_C="$SCRATCH/c"
mkdir -p "$FIX_C"
collect "$FIX_C"
check_eq "C1: empty directory yields 0 files" "0" "${#DISCOVERED[@]}"
if require_test_case_files "${#DISCOVERED[@]}" 2>/dev/null; then
    fail "C2: require_test_case_files 0 must return non-zero"
    FAILED=$((FAILED + 1))
else
    pass "C2: require_test_case_files 0 returns non-zero"
fi
if require_test_case_files 3 2>/dev/null; then
    pass "C3: require_test_case_files 3 returns zero"
else
    fail "C3: require_test_case_files 3 must return zero"
    FAILED=$((FAILED + 1))
fi

# Test D: output is sorted and unique across both extensions.
FIX_D="$SCRATCH/d"
mkdir -p "$FIX_D"
: > "$FIX_D/z.yml"
: > "$FIX_D/a.yaml"
: > "$FIX_D/m.yml"
collect "$FIX_D"
ORDER=""
for entry in "${DISCOVERED[@]}"; do
    ORDER="$ORDER $(basename "$entry")"
done
check_eq "D1: output is sorted with no duplicates" " a.yaml m.yml z.yml" "$ORDER"

# Test E: names containing spaces are returned intact (NUL-safe).
FIX_E="$SCRATCH/e"
mkdir -p "$FIX_E"
: > "$FIX_E/with space.yml"
collect "$FIX_E"
check_eq "E1: fixture with a space in the name yields 1 file" "1" "${#DISCOVERED[@]}"
check_eq "E2: space in name is preserved" "with space.yml" "$(basename "${DISCOVERED[0]}")"

# Test S: Stage 1 through the suite's real code path. The suite runs main() when sourced,
# so the validate_test_cases function is extracted from the suite source and run in a
# subshell against a fixture tree. The stub validator is `true` (every file passes).
SUITE="$PROJECT_ROOT/test-acceptance/run_acceptance_suite.sh"
S_DIR="$SCRATCH/stage1"
mkdir -p "$S_DIR/cases/sub" "$S_DIR/cases_empty" "$S_DIR/schemas/tcms-schemas/tcms" "$S_DIR/tmp"
: > "$S_DIR/schemas/tcms-schemas/tcms/test-case-main.schema.json"
: > "$S_DIR/cases/one.yml"
: > "$S_DIR/cases/two.yaml"
: > "$S_DIR/cases/sub/three.yaml"
: > "$S_DIR/cases/four.yml"
: > "$S_DIR/cases/readme.txt"
S_KNOWN_COUNT=4

# Prints "<return code> <TOTAL_TEST_CASES>" for validate_test_cases run on DIR.
run_stage1() {
    (
        eval "$(awk '/^validate_test_cases\(\) \{/{p=1} p{print} p && /^\}/{exit}' "$SUITE")"
        SCHEMA_DIR="$S_DIR/schemas"
        TEST_CASES_DIR="$1"
        VALIDATE_YAML=true
        TEMP_DIR="$S_DIR/tmp"
        VALIDATION_FAILURES="$S_DIR/tmp/failures.txt"
        VALIDATION_PASSED=0
        VALIDATION_FAILED=0
        VERBOSE=0
        TOTAL_TEST_CASES=0
        validate_test_cases >"$S_DIR/stage1.log" 2>&1
        echo "$? $TOTAL_TEST_CASES"
    )
}

if ! grep -q '^validate_test_cases() {' "$SUITE"; then
    fail "S0: validate_test_cases not found in $SUITE"
    FAILED=$((FAILED + 1))
else
    read -r S_RC S_TOTAL <<EOF
$(run_stage1 "$S_DIR/cases")
EOF
    check_eq "S1: Stage 1 discovers the fixture's known count ($S_KNOWN_COUNT)" "$S_KNOWN_COUNT" "$S_TOTAL"
    check_eq "S2: Stage 1 passes on the fixture" "0" "$S_RC"

    read -r S_RC S_TOTAL <<EOF
$(run_stage1 "$S_DIR/cases_empty")
EOF
    check_eq "S3: Stage 1 fails when no test case files are discovered" "1" "$S_RC"
fi

if [ "$FAILED" -ne 0 ]; then
    log_error "Stage 1 discovery tests: $FAILED check(s) failed"
    exit 1
fi
pass "Stage 1 discovery tests: all checks passed"
exit 0
