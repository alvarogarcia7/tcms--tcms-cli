#!/usr/bin/env bash
#
# sync-tcms-schemas.sh - Vendor the tcms-schemas JSON schemas and samples at a pinned ref
#
# The CLI keeps a pinned copy of https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas
# instead of a git submodule (CI/Docker friction) or a build-time download (needs network).
#
#   tcms-schemas schemas/tcms/*.schema.json -> schemas/tcms-schemas/tcms/
#   tcms-schemas samples/<category>/*       -> schemas/tcms-schemas/samples/<category>/
#   pinned commit                           -> schemas/tcms-schemas/TCMS_SCHEMAS_VERSION
#
# The copy lives in its own directory, mirroring the upstream paths, so it never
# mixes with the CLI's own schemas/*.schema.json and schemas/tcms/*.schema.v1.json.
# A document with `schema: tcms/<name>.schema.json` resolves to it.
#
# Usage:
#   scripts/sync-tcms-schemas.sh [REF]      REF defaults to the pinned commit; use "main" to update
#   TCMS_SCHEMAS_REPO=/path/or/url scripts/sync-tcms-schemas.sh REF
#
# After updating, review `git diff`, run `make test`, and commit.
#
# Compatible with bash 3.2+ and BSD/GNU tools.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

DEFAULT_REPO="https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas.git"
REPO="${TCMS_SCHEMAS_REPO:-$DEFAULT_REPO}"
VENDOR_DIR="$REPO_ROOT/schemas/tcms-schemas"
VERSION_FILE="$VENDOR_DIR/TCMS_SCHEMAS_VERSION"
SCHEMAS_DST="$VENDOR_DIR/tcms"
SAMPLES_DST="$VENDOR_DIR/samples"

if [ -n "$1" ]; then
    REF="$1"
elif [ -f "$VERSION_FILE" ]; then
    REF="$(sed -n 's/^commit: //p' "$VERSION_FILE")"
fi
if [ -z "$REF" ]; then
    log_error "No ref given and no pinned commit in $VERSION_FILE"
    exit 2
fi

TMP="$(mktemp -d)"
setup_cleanup "$TMP"

log_info "Fetching $REPO ($REF)"
git clone --quiet "$REPO" "$TMP/src"
git -C "$TMP/src" checkout --quiet "$REF"
COMMIT="$(git -C "$TMP/src" rev-parse HEAD)"

log_info "Copying schemas to $SCHEMAS_DST"
mkdir -p "$SCHEMAS_DST"
# Remove schemas that no longer exist upstream
for f in "$SCHEMAS_DST"/*.schema.json; do
    [ -e "$f" ] || continue
    [ -e "$TMP/src/schemas/tcms/$(basename "$f")" ] || rm -f "$f"
done
cp "$TMP"/src/schemas/tcms/*.schema.json "$SCHEMAS_DST/"

log_info "Copying samples to $SAMPLES_DST"
rm -rf "$SAMPLES_DST"
mkdir -p "$SAMPLES_DST"
cp -R "$TMP"/src/samples/. "$SAMPLES_DST/"

cat > "$VERSION_FILE" <<EOF
# Pinned copy of tcms-schemas. Update with scripts/sync-tcms-schemas.sh <ref>.
repository: https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas
commit: $COMMIT
EOF

pass "tcms-schemas synced at $COMMIT"
