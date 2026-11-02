#!/usr/bin/env bash
#
# validate_report_exit_codes.sh - Acceptance stage 10: exit codes of `test-executor report` (plan 05-03)
#
# DESCRIPTION:
#   Runs crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh against the real
#   test-executor binary. The E2E checks that report exits 0 on success, 1 on any input, validation,
#   usage or write error, and 2 only with --fail-on-gaps when gaps or failed results exist; that
#   the report is written before exit 2 (byte-identical to the exit-0 output); and that the other
#   subcommands keep clap's exit code 2 for usage errors. See docs/TRACEABILITY.md, "Exit status
#   and pipeline use".
#
# USAGE:
#   ./test-acceptance/validate_report_exit_codes.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose   Print the output of the E2E script, not only when it fails
#   -h, --help      Show this help message
#
# ENVIRONMENT:
#   TEST_EXECUTOR_BIN  Binary to test (default: found with find_binary "test-executor")
#
# EXIT CODES:
#   0 - All report exit-code checks passed
#   1 - A check failed, or the test-executor binary is missing
#
# NOTES:
#   Scripts stay bash 3.2 compatible.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1
source "$REPO_ROOT/scripts/lib/find-binary.sh" || exit 1

VERBOSE=0

usage() {
    sed -n '/^# USAGE:/,/^# NOTES:/p' "$0" | sed 's/^# \{0,1\}//' | sed '$d'
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

cd "$REPO_ROOT" || exit 1

TEST_EXECUTOR_BIN="${TEST_EXECUTOR_BIN:-$(find_binary "test-executor")}"
if [[ -z "$TEST_EXECUTOR_BIN" ]]; then
    log_error "test-executor binary not found (build it first: cargo build -p test-executor)"
    exit 1
fi
export TEST_EXECUTOR_BIN

E2E_SCRIPT="$REPO_ROOT/crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh"
if [[ ! -x "$E2E_SCRIPT" ]]; then
    log_error "E2E script not found or not executable: $E2E_SCRIPT"
    exit 1
fi

OUTPUT_FILE="$(mktemp "${TMPDIR:-/tmp}/report-exit-codes-stage.XXXXXX")"
setup_cleanup "$OUTPUT_FILE"

log_info "Testing $TEST_EXECUTOR_BIN"
if "$E2E_SCRIPT" > "$OUTPUT_FILE" 2>&1; then
    if [[ $VERBOSE -eq 1 ]]; then
        sed 's/^/  | /' "$OUTPUT_FILE" >&2
    fi
    pass "report exit codes (test_report_exit_codes_e2e.sh)"
    exit 0
fi

sed 's/^/  | /' "$OUTPUT_FILE" >&2
fail "report exit codes (test_report_exit_codes_e2e.sh)"
log_error "report exit-code E2E failed; see the output above"
exit 1
