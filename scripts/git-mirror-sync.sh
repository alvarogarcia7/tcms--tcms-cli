#!/usr/bin/env bash
# Sync a git repository across three places, with the local bare repo as the hub:
#
#   origin       the remote origin
#   bare         a local bare repo
#   bare remote  a second, remote bare repo
#
#   fetch: origin -> bare -> bare remote (commits that exist only on the bare
#          remote are rebased onto origin's branch first)
#   push:  bare -> origin
#   verify: check that a commit is present in all three
#
# Branches and tags only are pushed (with pruning, never forced: non-fast-forward
# updates are rejected), with one exception: a branch whose bare-remote-only
# commits were rebased is updated on the bare remote with --force-with-lease,
# pinned to the tip that was rebased, so a concurrent push is never overwritten.
# Hosting-specific refs such as
# refs/merge-requests/* are fetched into the mirror but never pushed.
#
# Compatible with bash 3.2+.
set -e

REFSPECS=("refs/heads/*:refs/heads/*" "refs/tags/*:refs/tags/*")

usage() {
    cat <<USAGE
Usage:
  $(basename "$0") fetch --origin <url> --bare <path> --bare-remote <url|path>
  $(basename "$0") push  --origin <url> --bare <path>
  $(basename "$0") verify --commit <rev> --origin <url> --bare <path> --bare-remote <url|path>

Commands:
  fetch   Fetch the remote origin into the local bare repo (cloned on first
          use), then push its branches and tags to the bare remote (created
          if it is a missing local path). Before pushing, work found only on
          the bare remote is reconciled with the origin:
            - branch diverged: its bare-remote-only commits are rebased onto
              the origin's branch and the bare remote is updated with
              --force-with-lease; a rebase conflict aborts with nothing pushed
            - bare remote ahead: its commits are kept, so `push` can send them
              to the origin
            - branch only on the bare remote and not merged anywhere in the
              origin: kept, so it is neither pruned nor lost
  push    Push branches and tags from the local bare repo to the remote origin.
  verify  Check that a commit is reachable from a branch or tag in the remote
          origin, the local bare repo and the bare remote. Exits 0 if it is
          present in all three, 1 otherwise. Repos that are not local
          directories are fetched (branches and tags, blobs skipped where the
          server supports it) into a temporary bare repo first.

Options:
  -o, --origin <url>         The remote origin (both)
  -b, --bare <path>          The local bare repository (both)
  -r, --bare-remote <url|path>  The remote bare repository (fetch, verify)
  -c, --commit <rev>         Commit to look for: a SHA, or a branch/tag name
                             resolved in the local bare repo (verify)
  -h, --help           Show this help
USAGE
}

require() {
    if [ -z "$2" ]; then
        echo "Missing required option: $1" >&2
        usage >&2
        exit 2
    fi
}

push_refs() {
    # $1 = bare repo, $2 = destination (path or URL)
    git -C "$1" push --prune "$2" "${REFSPECS[@]}"
}

BARE_REMOTE_NS="refs/mirror-sync/bare-remote"

# Reconcile branches that have commits only on the bare remote with the local
# bare repo (which already holds the origin's branches). Sets REBASED to
# "<branch> <bare remote tip that was rebased>" lines for the lease push.
reconcile_bare_remote() {
    local ref branch remote_tip local_tip base tmp new_tip

    REBASED=""
    git -C "$BARE" fetch --quiet "$BARE_REMOTE" "+refs/heads/*:$BARE_REMOTE_NS/*"

    while read -r ref; do
        [ -n "$ref" ] || continue
        branch="${ref#"$BARE_REMOTE_NS"/}"
        remote_tip="$(git -C "$BARE" rev-parse "$ref")"

        if ! local_tip="$(git -C "$BARE" rev-parse --verify --quiet "refs/heads/$branch")"; then
            # Only on the bare remote. Keep it if it has work the origin lacks;
            # if it is already merged into an origin branch it is stale and the
            # prune on push removes it.
            if [ -z "$(git -C "$BARE" for-each-ref --count=1 --contains "$remote_tip" refs/heads)" ]; then
                echo "Branch $branch exists only on the bare remote, keeping it"
                git -C "$BARE" update-ref "refs/heads/$branch" "$remote_tip"
            fi
        elif [ "$local_tip" = "$remote_tip" ] || git -C "$BARE" merge-base --is-ancestor "$remote_tip" "$local_tip"; then
            : # in sync, or behind: the push fast-forwards it
        elif git -C "$BARE" merge-base --is-ancestor "$local_tip" "$remote_tip"; then
            echo "Branch $branch: bare remote is ahead of the origin, keeping its commits"
            git -C "$BARE" update-ref "refs/heads/$branch" "$remote_tip" "$local_tip"
        else
            base="$(git -C "$BARE" merge-base "$local_tip" "$remote_tip")"
            echo "Branch $branch: rebasing bare-remote-only commits onto the origin"
            tmp="$(mktemp -d)"
            git -C "$BARE" worktree add --quiet --detach "$tmp" "$remote_tip"
            if ! GIT_COMMITTER_NAME="${GIT_COMMITTER_NAME:-$(git config user.name || echo git-mirror-sync)}" \
                GIT_COMMITTER_EMAIL="${GIT_COMMITTER_EMAIL:-$(git config user.email || echo git-mirror-sync@localhost)}" \
                git -C "$tmp" rebase --quiet --rebase-merges --onto "$local_tip" "$base" >/dev/null 2>&1; then
                git -C "$tmp" rebase --abort >/dev/null 2>&1 || true
                git -C "$BARE" worktree remove --force "$tmp"
                git -C "$BARE" for-each-ref --format='delete %(refname)' "$BARE_REMOTE_NS" | git -C "$BARE" update-ref --stdin
                echo "Rebase of $branch onto the origin has conflicts; resolve them by hand." >&2
                echo "Nothing was pushed to the bare remote." >&2
                exit 1
            fi
            new_tip="$(git -C "$tmp" rev-parse HEAD)"
            git -C "$BARE" worktree remove --force "$tmp"
            git -C "$BARE" update-ref "refs/heads/$branch" "$new_tip" "$local_tip"
            REBASED="$REBASED$branch $remote_tip
"
        fi
    done < <(git -C "$BARE" for-each-ref --format='%(refname)' "$BARE_REMOTE_NS")

    git -C "$BARE" for-each-ref --format='delete %(refname)' "$BARE_REMOTE_NS" | git -C "$BARE" update-ref --stdin
}

cmd_fetch() {
    require --origin "$ORIGIN"
    require --bare "$BARE"
    require --bare-remote "$BARE_REMOTE"

    if [ -d "$BARE" ]; then
        echo "Fetching $ORIGIN into $BARE"
        # Explicit URL and refspecs: do not depend on the remotes configured in
        # the bare repo, which may have no (or a refs/remotes/*) fetch refspec.
        git -C "$BARE" fetch --prune "$ORIGIN" "+refs/heads/*:refs/heads/*" "+refs/tags/*:refs/tags/*"
    else
        echo "Cloning $ORIGIN into new bare mirror $BARE"
        git clone --mirror "$ORIGIN" "$BARE"
    fi

    if [ -d "$(dirname "$BARE_REMOTE")" ] && [ ! -e "$BARE_REMOTE" ]; then
        echo "Creating bare repository $BARE_REMOTE"
        git init --bare "$BARE_REMOTE"
    fi

    reconcile_bare_remote

    # Rebased branches are the only forced push, pinned to the tip that was rebased
    while read -r branch old_tip; do
        [ -n "$branch" ] || continue
        echo "Updating rebased branch $branch on $BARE_REMOTE (force-with-lease)"
        git -C "$BARE" push "$BARE_REMOTE" \
            --force-with-lease="refs/heads/$branch:$old_tip" "refs/heads/$branch:refs/heads/$branch"
    done <<EOF
$REBASED
EOF

    echo "Pushing $BARE to $BARE_REMOTE"
    push_refs "$BARE" "$BARE_REMOTE"
    echo "fetch complete"
}

cmd_push() {
    require --bare "$BARE"
    require --origin "$ORIGIN"

    if [ ! -d "$BARE" ]; then
        echo "Bare repository not found: $BARE" >&2
        exit 1
    fi

    echo "Pushing $BARE to $ORIGIN"
    push_refs "$BARE" "$ORIGIN"
    echo "push complete"
}

# $1 = label, $2 = repo (directory or URL), $3 = full commit SHA
# Prints the result; returns 0 if the commit is on a branch or tag.
check_repo() {
    local label="$1" repo="$2" sha="$3" dir tmp="" found=1

    if [ -d "$repo" ]; then
        dir="$repo"
    else
        tmp="$(mktemp -d)"
        dir="$tmp"
        git init --bare --quiet "$dir"
        if ! git -C "$dir" fetch --quiet --filter=blob:none "$repo" "${REFSPECS[@]}" 2>/dev/null; then
            echo "  MISSING  $label: could not fetch $repo"
            rm -rf "$tmp"
            return 1
        fi
    fi

    if git -C "$dir" cat-file -e "$sha^{commit}" 2>/dev/null \
        && [ -n "$(git -C "$dir" for-each-ref --count=1 --contains "$sha" refs/heads refs/tags)" ]; then
        echo "  PRESENT  $label: $repo"
        found=0
    else
        echo "  MISSING  $label: $repo"
    fi

    [ -n "$tmp" ] && rm -rf "$tmp"
    return "$found"
}

cmd_verify() {
    require --commit "$COMMIT"
    require --origin "$ORIGIN"
    require --bare "$BARE"
    require --bare-remote "$BARE_REMOTE"

    local sha missing=0
    case "$COMMIT" in
        # A full SHA is used as given, so it can be reported missing from any repo
        *[!0-9a-f]*) sha="" ;;
        *) [ "${#COMMIT}" -eq 40 ] || [ "${#COMMIT}" -eq 64 ] && sha="$COMMIT" || sha="" ;;
    esac
    if [ -z "$sha" ] && ! sha="$(git -C "$BARE" rev-parse --verify --quiet "$COMMIT^{commit}")"; then
        echo "Cannot resolve '$COMMIT' to a commit in $BARE" >&2
        exit 1
    fi

    echo "Verifying commit $sha"
    check_repo "origin     " "$ORIGIN" "$sha" || missing=1
    check_repo "bare       " "$BARE" "$sha" || missing=1
    check_repo "bare remote" "$BARE_REMOTE" "$sha" || missing=1

    if [ "$missing" -ne 0 ]; then
        echo "verify failed: commit is not present in all three repositories" >&2
        exit 1
    fi
    echo "verify complete: commit is present in all three repositories"
}

COMMAND="${1:-}"
case "$COMMAND" in
    fetch|push|verify) shift ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac

ORIGIN="" BARE="" BARE_REMOTE="" COMMIT=""
while [ $# -gt 0 ]; do
    case "$1" in
        -o|--origin) ORIGIN="${2:-}"; shift 2 ;;
        -b|--bare)   BARE="${2:-}"; shift 2 ;;
        -r|--bare-remote) BARE_REMOTE="${2:-}"; shift 2 ;;
        -c|--commit) COMMIT="${2:-}"; shift 2 ;;
        -h|--help)   usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
done

"cmd_$COMMAND"
