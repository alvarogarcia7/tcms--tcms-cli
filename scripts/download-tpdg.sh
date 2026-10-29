#!/usr/bin/env bash
# download-tpdg.sh - Download the test-plan-documentation-generator (TPDG) release
#
# DESCRIPTION:
#   Downloads the TPDG linux-amd64 zip and SHA256SUMS from the generic package registry of the
#   TPDG project (the assets of its GitLab release), checks the checksum, and unpacks the binary
#   `tpdg` into the install directory. The acceptance suite uses it through TEST_PLAN_DOC_GEN.
#
# USAGE:
#   TPDG_TOKEN=<token> scripts/download-tpdg.sh [--version vX.Y.Z] [--dir DIR]
#
# ENVIRONMENT:
#   TPDG_TOKEN       Access token with read_api (or read_package_registry) on the TPDG project.
#                    Needed locally. In GitLab CI, CI_JOB_TOKEN is used when TPDG_TOKEN is not set;
#                    that works only if the TPDG project allows this project's job token
#                    (Settings > CI/CD > Job token permissions); otherwise set TPDG_TOKEN as a
#                    masked CI/CD variable.
#   TPDG_VERSION     Release tag (default v0.1.0-rc.1)
#   TPDG_DIR         Install directory (default tools/tpdg)
#   TPDG_HOST        GitLab host (default gitlab.crypto.tii.ae)
#   TPDG_PROJECT_ID  Project id of the TPDG project (default 350)
#
# EXIT: 0 downloaded and verified (or already there), 1 failure
# Note: only linux-amd64 is published.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

TPDG_VERSION="${TPDG_VERSION:-v0.1.0-rc.1}"
TPDG_DIR="${TPDG_DIR:-tools/tpdg}"
TPDG_HOST="${TPDG_HOST:-gitlab.crypto.tii.ae}"
TPDG_PROJECT_ID="${TPDG_PROJECT_ID:-350}"

while [ $# -gt 0 ]; do
    case "$1" in
        --version) TPDG_VERSION="$2"; shift 2 ;;
        --dir) TPDG_DIR="$2"; shift 2 ;;
        -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
        *) log_error "unknown argument: $1"; exit 1 ;;
    esac
done

if [ -n "${TPDG_TOKEN:-}" ]; then
    AUTH_HEADER="PRIVATE-TOKEN: $TPDG_TOKEN"
elif [ -n "${CI_JOB_TOKEN:-}" ]; then
    AUTH_HEADER="JOB-TOKEN: $CI_JOB_TOKEN"
else
    log_error "TPDG_TOKEN is not set (a GitLab token with read_api on project $TPDG_PROJECT_ID)"
    log_info "Example: TPDG_TOKEN=glpat-... make download-tpdg"
    exit 1
fi

os="$(uname -s)"; arch="$(uname -m)"
case "$os-$arch" in
    Linux-x86_64) ;;
    *) log_error "no TPDG build for $os-$arch; only linux-amd64 is published"
       log_info "Build it from source or set TEST_PLAN_DOC_GEN to your own binary"
       exit 1 ;;
esac

ZIP_NAME="tpdg-${TPDG_VERSION}-linux-amd64.zip"
BASE_URL="https://${TPDG_HOST}/api/v4/projects/${TPDG_PROJECT_ID}/packages/generic/tpdg/${TPDG_VERSION}"

if [ -x "$TPDG_DIR/tpdg" ] && [ "$(cat "$TPDG_DIR/.version" 2>/dev/null)" = "$TPDG_VERSION" ]; then
    log_info "TPDG $TPDG_VERSION already installed: $TPDG_DIR/tpdg"
    exit 0
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fetch() {
    # fetch NAME: the token goes in a header read from stdin, never on the command line
    printf 'header = "%s"\n' "$AUTH_HEADER" | \
        curl --fail --silent --show-error --location --config - \
            --output "$TMP_DIR/$1" "$BASE_URL/$1" || {
        log_error "download failed: $BASE_URL/$1 (check the token and its access to project $TPDG_PROJECT_ID)"
        exit 1
    }
}

log_info "Downloading TPDG $TPDG_VERSION from $TPDG_HOST"
fetch SHA256SUMS
fetch "$ZIP_NAME"

expected="$(awk -v f="$ZIP_NAME" '$2 == f {print $1}' "$TMP_DIR/SHA256SUMS")"
if [ -z "$expected" ]; then
    log_error "$ZIP_NAME is not listed in SHA256SUMS"
    exit 1
fi
if command -v sha256sum > /dev/null 2>&1; then
    actual="$(sha256sum "$TMP_DIR/$ZIP_NAME" | awk '{print $1}')"
else
    actual="$(shasum -a 256 "$TMP_DIR/$ZIP_NAME" | awk '{print $1}')"
fi
if [ "$expected" != "$actual" ]; then
    log_error "checksum mismatch for $ZIP_NAME: expected $expected, got $actual"
    exit 1
fi
pass "checksum verified ($actual)"

mkdir -p "$TPDG_DIR"
if command -v unzip > /dev/null 2>&1; then
    unzip -o -q "$TMP_DIR/$ZIP_NAME" tpdg -d "$TPDG_DIR"
else
    python3 -c "import sys,zipfile; zipfile.ZipFile(sys.argv[1]).extract('tpdg', sys.argv[2])" \
        "$TMP_DIR/$ZIP_NAME" "$TPDG_DIR"
fi
chmod +x "$TPDG_DIR/tpdg"
echo "$TPDG_VERSION" > "$TPDG_DIR/.version"
pass "installed $TPDG_DIR/tpdg"
log_info "Use it: export TEST_PLAN_DOC_GEN=\"$(cd "$TPDG_DIR" && pwd)/tpdg\""
