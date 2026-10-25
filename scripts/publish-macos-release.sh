#!/usr/bin/env bash
# ============================================================================
# publish-macos-release.sh
# ----------------------------------------------------------------------------
# Build the macOS arm64 release binaries locally and publish them to a GitLab
# release for a given tag, from the current local commit.
#
# Usage:
#   make publish-macos TAG=v0.0.1
#   scripts/publish-macos-release.sh v0.0.1
#
# Why local: the GitLab CI runners are Linux-only and cannot reliably
# cross-compile the macOS binaries (openssl-sys, pulled in via git2/native-tls,
# needs a working macOS SDK + headers). So the macOS archive is produced on a
# developer's Mac and uploaded to the SAME GitLab release that CI creates for
# the tag with the linux-amd64 archive.
#
# Requirements (run on macOS, ideally Apple Silicon):
#   - Rust toolchain (rustup/cargo); the aarch64-apple-darwin target is added
#     automatically if missing
#   - jq, zip, curl
#   - GITLAB_TOKEN: a personal access token with the 'api' scope
#
# Behaviour:
#   1. Builds every workspace binary for aarch64-apple-darwin (release).
#   2. Zips them into testcase-manager-<TAG>-macos-arm64.zip.
#   3. Uploads the zip to the project's generic package registry under <TAG>.
#   4. Ensures a GitLab release exists for <TAG> (creates it from the local
#      commit if absent), then attaches/refreshes the macOS asset link.
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/logger.sh
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

TAG="${1:-}"
PKG_NAME="testcase-manager"
ARCH_LABEL="macos-arm64"
TARGET="aarch64-apple-darwin"

# ---------------------------------------------------------------------------
# Validate arguments and environment
# ---------------------------------------------------------------------------
if [ -z "$TAG" ]; then
    log_error "No tag supplied. Usage: make publish-macos TAG=v0.0.1"
    exit 1
fi
case "$TAG" in
    v*) : ;;
    *) log_error "Tag must start with 'v' (e.g. v0.0.1); got '$TAG'"; exit 1 ;;
esac
if [ -z "${GITLAB_TOKEN:-}" ]; then
    log_error "GITLAB_TOKEN is not set (needs a personal access token with 'api' scope)."
    exit 1
fi
for tool in cargo jq zip curl git; do
    command -v "$tool" >/dev/null 2>&1 || { log_error "Required tool not found: $tool"; exit 1; }
done

OS="$(uname -s)"
if [ "$OS" != "Darwin" ]; then
    log_warning "Not running on macOS (uname=$OS). The $TARGET binaries may not be"
    log_warning "produced correctly from this host; continuing anyway."
fi

# ---------------------------------------------------------------------------
# Resolve project and API endpoint from the origin remote
# ---------------------------------------------------------------------------
REMOTE_URL="$(git config --get remote.origin.url)"
if [ -z "$REMOTE_URL" ]; then
    log_error "Could not determine origin remote URL."
    exit 1
fi
# Host, e.g. gitlab.crypto.tii.ae  (handles https://host/... and git@host:...)
GITLAB_HOST="$(printf '%s' "$REMOTE_URL" | sed -E 's#^[a-z]+://##; s#^git@##; s#[:/].*$##')"
# Project path (incl. any subgroups), e.g. int/test-case-management-system/tcms-cli
# Strip scheme and user, then drop the host (everything up to the first '/' or ':').
PROJECT_PATH="$(printf '%s' "$REMOTE_URL" | sed -E 's#^[a-z]+://##; s#^git@##; s#^[^/:]*[:/]##; s#\.git$##')"
# URL-encode the slashes for the GitLab "id or URL-encoded path" API form.
PROJECT_ENC="$(printf '%s' "$PROJECT_PATH" | sed 's#/#%2F#g')"
API="https://$GITLAB_HOST/api/v4"
COMMIT="$(git rev-parse HEAD)"

section "Publishing $PKG_NAME $TAG ($ARCH_LABEL)"
log_info "Project : $PROJECT_PATH"
log_info "Host    : $GITLAB_HOST"
log_info "Commit  : $COMMIT"

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
section "Building $ARCH_LABEL ($TARGET)"
rustup target add "$TARGET" >/dev/null 2>&1 || true
cargo build --release --target "$TARGET" --workspace --bins

# Detect the binaries to ship directly from workspace metadata (same logic as
# the CI release:build job) so the set stays in sync as crates are added.
RELEASE_BINS="$(cargo metadata --format-version 1 --no-deps \
    | jq -r '[.packages[].targets[] | select(.kind[] == "bin") | .name] | unique | join(" ")')"
log_info "Binaries: $RELEASE_BINS"

# ---------------------------------------------------------------------------
# Package
# ---------------------------------------------------------------------------
section "Packaging"
STAGE="${PKG_NAME}-${TAG}-${ARCH_LABEL}"
DIST="dist"
rm -rf "$STAGE" "$DIST/$STAGE.zip"
mkdir -p "$STAGE" "$DIST"
for b in $RELEASE_BINS; do
    cp "target/$TARGET/release/$b" "$STAGE/"
done
cp README.md README_INSTALL.md "$STAGE/" 2>/dev/null || true
( cd "$STAGE" && zip -qr "../$DIST/$STAGE.zip" . )
ZIP="$DIST/$STAGE.zip"
log_info "Created $ZIP"

# ---------------------------------------------------------------------------
# Upload to the generic package registry
# ---------------------------------------------------------------------------
section "Uploading to generic package registry"
REG_URL="$API/projects/$PROJECT_ENC/packages/generic/$PKG_NAME/$TAG/$STAGE.zip"
curl --fail --silent --show-error \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    --upload-file "$ZIP" "$REG_URL"
log_info "Uploaded -> $REG_URL"

# ---------------------------------------------------------------------------
# Ensure a release exists for the tag (create from the local commit if absent)
# ---------------------------------------------------------------------------
section "Ensuring GitLab release $TAG"
code="$(curl --silent --output /dev/null --write-out '%{http_code}' \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    "$API/projects/$PROJECT_ENC/releases/$TAG")"
if [ "$code" = "404" ]; then
    log_info "Release not found; creating it from commit $COMMIT"
    curl --fail --silent --show-error --request POST \
        --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
        --data-urlencode "tag_name=$TAG" \
        --data-urlencode "ref=$COMMIT" \
        --data-urlencode "name=Release $TAG" \
        --data-urlencode "description=Binary release of $PKG_NAME for $TAG" \
        "$API/projects/$PROJECT_ENC/releases" >/dev/null
else
    log_info "Release already exists (HTTP $code)"
fi

# ---------------------------------------------------------------------------
# Attach / refresh the macOS asset link (idempotent)
# ---------------------------------------------------------------------------
section "Linking macOS asset to release $TAG"
LINK_NAME="$STAGE.zip"
existing_id="$(curl --silent --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    "$API/projects/$PROJECT_ENC/releases/$TAG/assets/links" \
    | jq -r --arg n "$LINK_NAME" '.[] | select(.name==$n) | .id' | head -1)"
if [ -n "$existing_id" ] && [ "$existing_id" != "null" ]; then
    log_info "Removing existing asset link #$existing_id for re-publish"
    curl --silent --request DELETE \
        --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
        "$API/projects/$PROJECT_ENC/releases/$TAG/assets/links/$existing_id" >/dev/null || true
fi
curl --fail --silent --show-error --request POST \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    --data-urlencode "name=$LINK_NAME" \
    --data-urlencode "url=$REG_URL" \
    --data-urlencode "link_type=package" \
    "$API/projects/$PROJECT_ENC/releases/$TAG/assets/links" >/dev/null

pass "Published $LINK_NAME to GitLab release $TAG"
log_info "Release: https://$GITLAB_HOST/$PROJECT_PATH/-/releases/$TAG"
