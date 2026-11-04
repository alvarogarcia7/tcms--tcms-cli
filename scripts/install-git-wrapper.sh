#!/usr/bin/env bash
# Installs the git wrapper in place of the git binary.
#
#   sudo scripts/install-git-wrapper.sh                # install
#   sudo scripts/install-git-wrapper.sh --uninstall    # restore the original git
#   scripts/install-git-wrapper.sh --dry-run           # show what would happen
#
# The original binary is moved to <bin-dir>/.git-real (hidden name). <bin-dir>/git
# becomes the wrapper (scripts/git-wrapper/git): it sets the commit identity from
# the repo's .env (or GIT_* env vars) and strips Co-Authored-By trailers, then runs
# .git-real. The commit-msg hook it uses is installed to <hook-dir>.

set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEMPLATE_DIR="$SCRIPT_DIR/git-wrapper"
HIDDEN_NAME=".git-real"
BIN_DIR="/usr/bin"
HOOK_DIR="/usr/local/share/git-wrap/hooks"
DRY_RUN=0
UNINSTALL=0

usage() {
	cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --bin-dir DIR     directory holding the git binary (default: /usr/bin)
  --hook-dir DIR    where the commit-msg hook is installed
                    (default: /usr/local/share/git-wrap/hooks)
  --uninstall       move the original git back and remove the wrapper and hook
  --dry-run         print the actions without changing anything
  -h, --help        show this help
EOF
}

say() { echo "==> $*"; }
die() { echo "error: $*" >&2; exit 1; }

# Run a command, or only print it in dry-run mode.
run() {
	if [ "$DRY_RUN" -eq 1 ]; then
		echo "[dry-run] $*"
	else
		"$@"
	fi
}

# True when FILE is the wrapper installed by this script.
is_wrapper() {
	[ -f "$1" ] && grep -q '^# git wrapper:' "$1"
}

# Copy TEMPLATE to DEST with the placeholders filled in and the given mode.
render() {
	local template="$1" dest="$2" mode="$3"
	if [ "$DRY_RUN" -eq 1 ]; then
		echo "[dry-run] render $template -> $dest (mode $mode)"
		return 0
	fi
	local tmp
	tmp="$(mktemp)"
	sed -e "s|__REAL_GIT__|$BIN_DIR/$HIDDEN_NAME|g" \
		-e "s|__HOOK_DIR__|$HOOK_DIR|g" "$template" > "$tmp"
	chmod "$mode" "$tmp"
	mv "$tmp" "$dest"
}

while [ $# -gt 0 ]; do
	case "$1" in
		--bin-dir) [ $# -ge 2 ] || die "--bin-dir needs a value"; BIN_DIR="$2"; shift 2 ;;
		--hook-dir) [ $# -ge 2 ] || die "--hook-dir needs a value"; HOOK_DIR="$2"; shift 2 ;;
		--uninstall) UNINSTALL=1; shift ;;
		--dry-run) DRY_RUN=1; shift ;;
		-h|--help) usage; exit 0 ;;
		*) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
	esac
done

TARGET="$BIN_DIR/git"
HIDDEN="$BIN_DIR/$HIDDEN_NAME"

if [ "$DRY_RUN" -eq 0 ] && [ ! -w "$BIN_DIR" ]; then
	die "$BIN_DIR is not writable; run with sudo"
fi

if [ "$UNINSTALL" -eq 1 ]; then
	[ -e "$HIDDEN" ] || die "$HIDDEN not found; nothing to restore"
	if [ -e "$TARGET" ] && ! is_wrapper "$TARGET"; then
		die "$TARGET is not the wrapper; refusing to overwrite it"
	fi
	say "restoring original git: $HIDDEN -> $TARGET"
	run rm -f "$TARGET"
	run mv "$HIDDEN" "$TARGET"
	run rm -f "$HOOK_DIR/commit-msg"
	say "restored"
	exit 0
fi

[ -f "$TEMPLATE_DIR/git" ] || die "missing template $TEMPLATE_DIR/git"
[ -f "$TEMPLATE_DIR/commit-msg" ] || die "missing template $TEMPLATE_DIR/commit-msg"

if is_wrapper "$TARGET"; then
	say "wrapper already installed at $TARGET"
	[ -e "$HIDDEN" ] || die "wrapper present but $HIDDEN is missing; cannot find the real git"
else
	[ -x "$TARGET" ] || die "no git binary at $TARGET"
	[ ! -e "$HIDDEN" ] || die "$HIDDEN already exists; refusing to overwrite it"
	say "moving git binary: $TARGET -> $HIDDEN"
	run mv "$TARGET" "$HIDDEN"
fi

say "installing wrapper: $TARGET"
render "$TEMPLATE_DIR/git" "$TARGET" 755

say "installing commit-msg hook: $HOOK_DIR/commit-msg"
run mkdir -p "$HOOK_DIR"
render "$TEMPLATE_DIR/commit-msg" "$HOOK_DIR/commit-msg" 755

if [ "$DRY_RUN" -eq 0 ]; then
	"$HIDDEN" --version >/dev/null || die "$HIDDEN does not run"
	"$TARGET" --version >/dev/null || die "wrapper $TARGET does not run"
	say "installed: $("$TARGET" --version)"
	if [ -e "$HOME/.local/bin/git" ]; then
		echo "note: $HOME/.local/bin/git is still present and will wrap the wrapper; remove it." >&2
	fi
fi
