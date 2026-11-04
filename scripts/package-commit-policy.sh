#!/usr/bin/env bash
# Packs the commit-policy toolkit into a self-contained tarball that can be
# copied to any machine and installed with its install.sh.
#
#   scripts/package-commit-policy.sh                 # writes dist/commit-policy-<stamp>.tar.gz
#   scripts/package-commit-policy.sh --output FILE   # choose the tarball path
#
# Bundle layout (paths are kept so install-git-wrapper.sh can find its templates):
#   commit-policy/README.md
#   commit-policy/install.sh
#   commit-policy/scripts/commit_policy.py
#   commit-policy/scripts/install-git-wrapper.sh
#   commit-policy/scripts/git-wrapper/{git,commit-msg}
#   commit-policy/scripts/git-hooks/pre-push
#   commit-policy/scripts/tests/test_commit_policy.py

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NAME="commit-policy"
STAMP="$(date -u +%Y%m%d-%H%M%S)"
OUTPUT="$REPO_ROOT/dist/$NAME-$STAMP.tar.gz"

usage() {
	cat <<EOF
Usage: $(basename "$0") [--output FILE]

Options:
  --output FILE   tarball to write (default: dist/$NAME-<timestamp>.tar.gz)
  -h, --help      show this help
EOF
}

die() { echo "error: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
	case "$1" in
		--output) [ $# -ge 2 ] || die "--output needs a value"; OUTPUT="$2"; shift 2 ;;
		-h|--help) usage; exit 0 ;;
		*) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
	esac
done

SOURCES="
scripts/commit_policy.py
scripts/install-git-wrapper.sh
scripts/git-wrapper/git
scripts/git-wrapper/commit-msg
scripts/git-hooks/pre-push
scripts/tests/test_commit_policy.py
scripts/bundle/install.sh
scripts/bundle/README.md
"

for rel in $SOURCES; do
	[ -f "$REPO_ROOT/$rel" ] || die "missing $rel"
done

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

for rel in $SOURCES; do
	mkdir -p "$STAGE/$NAME/$(dirname "$rel")"
	cp "$REPO_ROOT/$rel" "$STAGE/$NAME/$rel"
done
# install.sh lives at the bundle root, next to scripts/.
mv "$STAGE/$NAME/scripts/bundle/install.sh" "$STAGE/$NAME/install.sh"
mv "$STAGE/$NAME/scripts/bundle/README.md" "$STAGE/$NAME/README.md"
rmdir "$STAGE/$NAME/scripts/bundle"

chmod 755 "$STAGE/$NAME/install.sh" \
	"$STAGE/$NAME/scripts/commit_policy.py" \
	"$STAGE/$NAME/scripts/install-git-wrapper.sh" \
	"$STAGE/$NAME/scripts/git-hooks/pre-push" \
	"$STAGE/$NAME/scripts/git-wrapper/git" \
	"$STAGE/$NAME/scripts/git-wrapper/commit-msg"
chmod 644 "$STAGE/$NAME/scripts/tests/test_commit_policy.py"

mkdir -p "$(dirname "$OUTPUT")"
tar -czf "$OUTPUT" -C "$STAGE" "$NAME"

echo "wrote $OUTPUT"
echo "contents:"
tar -tzf "$OUTPUT" | sed 's/^/  /'
