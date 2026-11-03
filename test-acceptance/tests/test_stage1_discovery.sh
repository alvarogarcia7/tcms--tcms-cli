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

# Test A: the real tree yields 89 files, matching the on-disk count and the literal 89.
collect "$TEST_CASES_DIR"
ON_DISK=$(find "$TEST_CASES_DIR" -type f \( -name '*.yml' -o -name '*.yaml' \) | wc -l | tr -d ' ')
check_eq "A1: discovery count equals on-disk count of *.yml and *.yaml" "$ON_DISK" "${#DISCOVERED[@]}"
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

if [ "$FAILED" -ne 0 ]; then
    log_error "Stage 1 discovery tests: $FAILED check(s) failed"
    exit 1
fi
pass "Stage 1 discovery tests: all checks passed"
exit 0
