#!/usr/bin/env bash
# ============================================================================
# remove-quarantine.sh
# ----------------------------------------------------------------------------
# Remove the macOS Gatekeeper quarantine attribute (com.apple.quarantine) from
# the files of a directory. Files downloaded through a browser get this
# attribute, and macOS then refuses to run the binaries.
#
# Usage:
#   scripts/remove-quarantine.sh [DIR]     (DIR defaults to this script's directory)
#
# The files are discovered at run time: every regular file directly inside DIR
# that currently carries the attribute is cleaned. Nothing is hardcoded, so the
# script also works for binaries added to a release later.
#
# Notes:
#   - macOS only; on other systems the script does nothing and exits 0.
#   - Only the quarantine attribute is removed; other xattrs are left alone.
#   - Portable across BSD and GNU userland; bash 3.2+ compatible.
# ============================================================================
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ATTR="com.apple.quarantine"
DIR="${1:-$SCRIPT_DIR}"

if [ "$(uname -s)" != "Darwin" ]; then
    echo "[INFO] Not macOS; nothing to do."
    exit 0
fi

if [ ! -d "$DIR" ]; then
    echo "[ERROR] Not a directory: $DIR" >&2
    exit 1
fi

echo "=== Removing $ATTR from files in $DIR ==="
failed=0
cleaned=0
while IFS= read -r file; do
    if xattr -p "$ATTR" "$file" >/dev/null 2>&1; then
        if xattr -d "$ATTR" "$file"; then
            echo "[OK] cleaned: $(basename "$file")"
            cleaned=$((cleaned + 1))
        else
            echo "[ERROR] could not remove $ATTR from: $file" >&2
            failed=$((failed + 1))
        fi
    fi
done <<EOF
$(find "$DIR" -maxdepth 1 -type f)
EOF

if [ "$failed" -ne 0 ]; then
    echo "[ERROR] $failed file(s) could not be cleaned." >&2
    exit 1
fi
echo "[INFO] Done. Files cleaned: $cleaned"
exit 0
