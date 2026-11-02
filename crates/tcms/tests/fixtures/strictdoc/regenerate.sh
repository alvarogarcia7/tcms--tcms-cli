#!/usr/bin/env bash
# Regenerate (or check) the checked-in StrictDoc JSON exports used by the
# requirement-locate tests. See README.md next to this script.
#
#   regenerate.sh          write <case>/export.json for every case
#   regenerate.sh --check  regenerate into a temp dir and diff; exit 1 on drift
#
# Exit codes: 0 ok, 1 drift (--check) or tool failure, 2 usage or missing uv.
# Bash 3.2 and BSD/GNU compatible. Never run by cargo test.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# This directory is five levels below the repository root.
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
FIXTURE_REL="crates/tcms/tests/fixtures/strictdoc"

# The logger path is computed at runtime, so shellcheck cannot follow it.
# shellcheck disable=SC1091
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1

CASES="locate refused-level no-level"

usage() {
    echo "Usage: $(basename "$0") [--check]" >&2
    echo "  (no argument)  regenerate <case>/export.json for: $CASES" >&2
    echo "  --check        regenerate into a temp dir and fail if any export differs" >&2
}

CHECK=0
if [ "$#" -eq 1 ] && [ "$1" = "--check" ]; then
    CHECK=1
elif [ "$#" -ne 0 ]; then
    usage
    exit 2
fi

if ! command -v uv >/dev/null 2>&1; then
    log_error "uv is required: StrictDoc is run through uv (uv run strictdoc), pinned in uv.lock"
    exit 2
fi

cd "$REPO_ROOT"

TMP_DIR="$(mktemp -d)"
setup_cleanup "$TMP_DIR"

DRIFT=0
for case_name in $CASES; do
    src_dir="$FIXTURE_REL/$case_name/src"
    out_dir="$TMP_DIR/$case_name"
    log_file="$TMP_DIR/$case_name.log"
    fresh="$out_dir/json/index.json"
    checked_in="$FIXTURE_REL/$case_name/export.json"

    log_info "Exporting $case_name from $src_dir"
    if ! uv run strictdoc export --formats json --output-dir "$out_dir" "$src_dir" >"$log_file" 2>&1; then
        log_error "StrictDoc export failed for $case_name:"
        cat "$log_file" >&2
        exit 1
    fi

    if [ ! -f "$fresh" ]; then
        log_error "StrictDoc wrote no json/index.json for $case_name"
        cat "$log_file" >&2
        exit 1
    fi

    if [ "$CHECK" -eq 1 ]; then
        if [ ! -f "$checked_in" ]; then
            log_error "missing checked-in export: $checked_in"
            DRIFT=1
        elif ! diff -u "$checked_in" "$fresh"; then
            log_error "checked-in export differs from a fresh StrictDoc export: $checked_in"
            DRIFT=1
        else
            log_info "$case_name: up to date"
        fi
    else
        cp "$fresh" "$checked_in"
        log_info "Wrote $checked_in"
    fi
done

if [ "$CHECK" -eq 1 ]; then
    if [ "$DRIFT" -ne 0 ]; then
        log_error "Drift detected. Run make strictdoc-regen-test-fixtures, never edit the exports by hand."
        exit 1
    fi
    log_info "All exports match a fresh StrictDoc export"
fi
exit 0
