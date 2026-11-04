#!/usr/bin/env bash
# Installer for the commit-policy bundle (built by scripts/package-commit-policy.sh).
#
#   sudo ./install.sh                       # install the git wrapper and commit-msg hook
#   sudo ./install.sh --uninstall           # restore the original git
#   ./install.sh --repo /path/to/repo       # also enable the pre-push policy check in a repo
#   ./install.sh --dry-run                  # show what would happen
#
# Without --repo, options are passed to scripts/install-git-wrapper.sh.
# With --repo, only that repository is changed: commit_policy.py and the pre-push
# hook are copied into it and core.hooksPath is set for that repo only. It needs
# a .commit-policy file there. Run the system install and the repo step separately.

set -eu

BUNDLE_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO=""
PASS_ARGS=""
DRY_RUN=0

usage() {
	cat <<EOF
Usage: $(basename "$0") [--repo DIR] [installer options]

  --repo DIR     install the pre-push policy check into the git repository DIR
  --dry-run      print the actions without changing anything
  --uninstall    restore the original git (system-wide installer option)
  --bin-dir DIR  directory holding the git binary (default: /usr/bin)
  --hook-dir DIR where the commit-msg hook is installed
  -h, --help     show this help
EOF
}

say() { echo "==> $*"; }
die() { echo "error: $*" >&2; exit 1; }

command -v python3 >/dev/null 2>&1 || die "python3 is required by commit_policy.py"
command -v git >/dev/null 2>&1 || die "git is required"

# Collect the arguments for install-git-wrapper.sh, keeping --repo for this script.
ARGS=""
UNINSTALL=0
while [ $# -gt 0 ]; do
	case "$1" in
		--repo) [ $# -ge 2 ] || die "--repo needs a value"; REPO="$2"; shift 2 ;;
		--dry-run) DRY_RUN=1; ARGS="$ARGS --dry-run"; shift ;;
		--uninstall) UNINSTALL=1; ARGS="$ARGS --uninstall"; shift ;;
		-h|--help) usage; exit 0 ;;
		*) ARGS="$ARGS $1"; shift ;;
	esac
done

# A --repo run only touches that repository; run again without it for the system install.
if [ -z "$REPO" ]; then
	# Word-split on purpose: ARGS holds only flags and their simple values.
	# shellcheck disable=SC2086
	"$BUNDLE_DIR/scripts/install-git-wrapper.sh" $ARGS
	if [ "$UNINSTALL" -eq 1 ]; then
		exit 0
	fi
	say "to enable the pre-push policy in a repo, run: $0 --repo /path/to/repo"
	exit 0
fi

[ "$UNINSTALL" -eq 0 ] || die "--uninstall applies to the system install; do not combine it with --repo"
[ -d "$REPO" ] || die "$REPO is not a directory"
git -C "$REPO" rev-parse --show-toplevel >/dev/null 2>&1 || die "$REPO is not a git repository"
REPO_ROOT="$(git -C "$REPO" rev-parse --show-toplevel)"

if [ "$DRY_RUN" -eq 1 ]; then
	echo "[dry-run] copy commit_policy.py and pre-push into $REPO_ROOT/scripts"
	echo "[dry-run] git -C $REPO_ROOT config core.hooksPath scripts/git-hooks"
	exit 0
fi

say "copying policy checker and pre-push hook into $REPO_ROOT/scripts"
mkdir -p "$REPO_ROOT/scripts/git-hooks"
cp "$BUNDLE_DIR/scripts/commit_policy.py" "$REPO_ROOT/scripts/commit_policy.py"
cp "$BUNDLE_DIR/scripts/git-hooks/pre-push" "$REPO_ROOT/scripts/git-hooks/pre-push"
chmod 755 "$REPO_ROOT/scripts/commit_policy.py" "$REPO_ROOT/scripts/git-hooks/pre-push"

say "setting core.hooksPath for this repository"
git -C "$REPO_ROOT" config core.hooksPath scripts/git-hooks

if [ ! -f "$REPO_ROOT/.commit-policy" ]; then
	echo "warning: $REPO_ROOT/.commit-policy is missing; the pre-push check fails until you add it" >&2
	echo "         one allowed author email or \$VARIABLE per line" >&2
fi

say "done: pre-push policy enabled in $REPO_ROOT"
