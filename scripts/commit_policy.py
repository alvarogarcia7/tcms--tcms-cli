#!/usr/bin/env python3
"""Commit policy: commits must use an allowed author email and carry no Co-Authored-By trailer.

Commands:
  pre-push [REMOTE]   Read git's pre-push ref lines from stdin and check the commits
                      being pushed. Local only, no network.
  github | gitlab     Check the commits between --base and --head on the hosting
                      service using a token (GITHUB_TOKEN / GITLAB_TOKEN), then post a
                      commit status named "commit-policy" on --head. The status only
                      blocks a merge when branch protection (GitHub) or the merge
                      requirements (GitLab) make it required.

Allowed author emails are listed one per line in the policy file (.commit-policy at
the repository root by default). Lines may carry a "# comment". An entry may use
$NAME or ${NAME}; the value comes from the .env file beside the policy file, then from
the environment, so the address can stay out of git. An unset variable is an error. A
missing policy file is an error too, so the check fails closed.

Exit codes: 0 all commits allowed, 1 policy violations found, 2 the check itself failed.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import urllib.parse
import urllib.request
from dataclasses import dataclass

POLICY_FILE = ".commit-policy"
STATUS_CONTEXT = "commit-policy"
ZERO_SHA = "0" * 40
GITHUB_API = "https://api.github.com"
GITLAB_URL = "https://gitlab.com"
TRAILER_RE = re.compile(r"^\s*co-authored-by\s*:", re.IGNORECASE | re.MULTILINE)
ENV_LINE_RE = re.compile(r"^[ \t]*([A-Za-z_][A-Za-z0-9_]*)[ \t]*=[ \t]*(.*)$")
ENV_REF_RE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)")


@dataclass
class Commit:
    sha: str
    author_email: str
    message: str


def read_env_file(path):
    """Return the KEY=VALUE assignments in an env file (last assignment wins)."""
    values = {}
    if not os.path.isfile(path):
        return values
    with open(path, encoding="utf-8") as env_file:
        for line in env_file:
            match = ENV_LINE_RE.match(line)
            if match:
                values[match.group(1)] = unquote(match.group(2))
    return values


def unquote(value):
    value = value.rstrip("\r\n").strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


def expand_entry(entry, env_values):
    """Replace each $NAME or ${NAME} in an entry with its value.

    A value from the policy's .env file wins; the process environment is the fallback.
    An unset or empty variable raises ValueError so the check fails closed.
    """
    def replace(match):
        name = match.group(1) or match.group(2)
        value = env_values.get(name) or os.environ.get(name, "")
        if not value:
            raise ValueError(f"policy entry '{entry}' uses ${name}, which is not set in .env or the environment")
        return value

    return ENV_REF_RE.sub(replace, entry)


def load_allowed(path):
    """Return the lower-cased allowed author emails listed in the policy file.

    Entries may reference variables ($NAME or ${NAME}), which keeps the address out of
    git: the policy file's sibling .env (gitignored) is read first, then the environment.
    """
    env_values = read_env_file(os.path.join(os.path.dirname(path), ".env"))
    allowed = set()
    with open(path, encoding="utf-8") as policy:
        for line in policy:
            raw = line.split("#", 1)[0].strip()
            if raw:
                allowed.add(expand_entry(raw, env_values).strip().lower())
    if not allowed:
        raise ValueError(f"{path} lists no allowed emails")
    return allowed


def problems_for(commit, allowed):
    """Return a list of human-readable policy problems for one commit (empty when it passes)."""
    problems = []
    if commit.author_email.strip().lower() not in allowed:
        problems.append(f"author {commit.author_email} is not listed in {POLICY_FILE}")
    if TRAILER_RE.search(commit.message):
        problems.append("message contains a Co-Authored-By trailer")
    return problems


def git(*args):
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True
    ).stdout


def local_commits(local_sha, remote_name, remote_sha):
    """Commits a push would send: the new ones on the branch, or everything for a new branch."""
    if remote_sha == ZERO_SHA:
        rev_args = [local_sha, "--not", f"--remotes={remote_name}"]
    else:
        rev_args = [f"{remote_sha}..{local_sha}"]
    commits = []
    for sha in git("rev-list", *rev_args).split():
        author_email, _, message = git("show", "-s", "--format=%ae%n%B", sha).partition("\n")
        commits.append(Commit(sha, author_email, message))
    return commits


def run_pre_push(remote_name, ref_lines, allowed):
    failures = []
    for line in ref_lines:
        parts = line.split()
        if len(parts) != 4:
            continue
        local_ref, local_sha, _remote_ref, remote_sha = parts
        if local_sha == ZERO_SHA:
            continue  # deleting a branch sends no commits
        for commit in local_commits(local_sha, remote_name, remote_sha):
            for problem in problems_for(commit, allowed):
                failures.append(f"{commit.sha[:10]} ({local_ref}): {problem}")
    return failures


def http_json(method, url, headers, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    request = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={**headers, "Accept": "application/json", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode("utf-8") or "null")


def github_commits(api, repo, base, head, token):
    headers = {"Authorization": f"Bearer {token}", "X-GitHub-Api-Version": "2022-11-28"}
    url = f"{api}/repos/{repo}/compare/{urllib.parse.quote(base)}...{urllib.parse.quote(head)}"
    data = http_json("GET", url, headers)
    if data["total_commits"] > len(data["commits"]):
        raise ValueError(f"compare returned {len(data['commits'])} of {data['total_commits']} commits")
    return [
        Commit(c["sha"], c["commit"]["author"]["email"], c["commit"]["message"])
        for c in data["commits"]
    ]


def github_post_status(api, repo, sha, state, description, token):
    headers = {"Authorization": f"Bearer {token}", "X-GitHub-Api-Version": "2022-11-28"}
    body = {"state": state, "context": STATUS_CONTEXT, "description": description[:140]}
    http_json("POST", f"{api}/repos/{repo}/statuses/{sha}", headers, body)


def gitlab_commits(api, project, base, head, token):
    headers = {"PRIVATE-TOKEN": token}
    project_id = urllib.parse.quote(project, safe="")
    url = (
        f"{api}/projects/{project_id}/repository/compare"
        f"?from={urllib.parse.quote(base)}&to={urllib.parse.quote(head)}"
    )
    data = http_json("GET", url, headers)
    return [
        Commit(c["id"], c["author_email"], c["message"]) for c in data["commits"]
    ]


def gitlab_post_status(api, project, sha, state, description, token):
    headers = {"PRIVATE-TOKEN": token}
    project_id = urllib.parse.quote(project, safe="")
    body = {"state": state, "name": STATUS_CONTEXT, "description": description[:255]}
    http_json("POST", f"{api}/projects/{project_id}/statuses/{sha}", headers, body)


def require_token(variable):
    token = os.environ.get(variable, "")
    if not token:
        raise ValueError(f"{variable} is not set")
    return token


def run_hosted(args, allowed):
    if args.command == "github":
        token = require_token("GITHUB_TOKEN")
        api = args.api_url or GITHUB_API
        commits = github_commits(api, args.repo, args.base, args.head, token)
    else:
        token = require_token("GITLAB_TOKEN")
        api = args.api_url or f"{os.environ.get('GITLAB_URL', GITLAB_URL).rstrip('/')}/api/v4"
        commits = gitlab_commits(api, args.repo, args.base, args.head, token)

    failures = []
    for commit in commits:
        for problem in problems_for(commit, allowed):
            failures.append(f"{commit.sha[:10]}: {problem}")

    description = (
        f"{len(failures)} policy violation(s)" if failures else f"{len(commits)} commit(s) allowed"
    )
    state = "failure" if failures else "success"
    if args.dry_run:
        print(f"dry run: would post {state} status on {args.head}: {description}")
    elif args.command == "github":
        github_post_status(api, args.repo, args.head, state, description, token)
    else:
        gitlab_post_status(api, args.repo, args.head, state if state == "success" else "failed",
                           description, token)
    return failures


def parse_args(argv):
    parser = argparse.ArgumentParser(description="Check commit author and trailer policy.")
    parser.add_argument("--policy", default=POLICY_FILE, help="allowed-emails file")
    sub = parser.add_subparsers(dest="command", required=True)

    pre_push = sub.add_parser("pre-push", help="check commits being pushed (git hook)")
    pre_push.add_argument("remote", nargs="?", default="origin")

    for name in ("github", "gitlab"):
        hosted = sub.add_parser(name, help=f"check a {name} change and post a commit status")
        hosted.add_argument(
            "--repo", required=True,
            help="owner/repo (GitHub) or project path or numeric ID (GitLab)",
        )
        hosted.add_argument("--base", required=True, help="base branch or sha")
        hosted.add_argument("--head", required=True, help="head sha to check and report on")
        hosted.add_argument("--api-url", help="override the API base URL")
        hosted.add_argument("--dry-run", action="store_true", help="do not post the status")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv if argv is not None else sys.argv[1:])
    try:
        allowed = load_allowed(args.policy)
        if args.command == "pre-push":
            failures = run_pre_push(args.remote, sys.stdin.read().splitlines(), allowed)
        else:
            failures = run_hosted(args, allowed)
    except subprocess.CalledProcessError as exc:
        print(f"commit-policy: error: {exc.cmd[0]} failed: {exc.stderr.strip()}", file=sys.stderr)
        return 2
    except (OSError, ValueError, KeyError) as exc:
        print(f"commit-policy: error: {exc}", file=sys.stderr)
        return 2

    if failures:
        print("commit-policy: rejected", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1
    print("commit-policy: all commits allowed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
