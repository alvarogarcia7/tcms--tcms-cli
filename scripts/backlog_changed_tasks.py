#!/usr/bin/env python3
"""List Backlog.md tasks whose files changed in a git commit range.

By default the range runs from the merge-base of HEAD and the main branch
(the point where the current branch departed from its parent) to HEAD.

Examples:
    scripts/backlog_changed_tasks.py
    scripts/backlog_changed_tasks.py --base abc123 --head HEAD
    scripts/backlog_changed_tasks.py --main develop --json
"""

import argparse
import json
import re
import subprocess
import sys
from collections import OrderedDict

# Task-bearing locations in the Backlog.md layout (see backlog/config.yml).
TASK_DIRS = ["backlog/tasks", "backlog/archive/tasks", "backlog/drafts"]

ID_RE = re.compile(r"^id:\s*['\"]?([^'\"\s]+)['\"]?\s*$", re.MULTILINE)
TITLE_RE = re.compile(r"^title:\s*(.+?)\s*$", re.MULTILINE)
FILENAME_ID_RE = re.compile(r"^([A-Za-z][A-Za-z0-9]*-\d+)\b")


def git(*args, check=True):
    result = subprocess.run(
        ["git", *args], capture_output=True, text=True, check=False
    )
    if check and result.returncode != 0:
        sys.exit(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout


def resolve_default_base(main_branch, head):
    for candidate in (main_branch, f"origin/{main_branch}"):
        base = git("merge-base", head, candidate, check=False).strip()
        if base:
            return base
    sys.exit(f"Could not find a merge-base with '{main_branch}' or 'origin/{main_branch}'.")


def changed_entries(base, head):
    """Yield (status, path, old_path) for task files changed between base and head."""
    output = git(
        "diff", "--name-status", "-z", "-M", f"{base}...{head}", "--", *TASK_DIRS
    )
    fields = [f for f in output.split("\0") if f]
    i = 0
    while i < len(fields):
        status = fields[i][0]
        if status in ("R", "C"):
            old_path, new_path = fields[i + 1], fields[i + 2]
            yield status, new_path, old_path
            i += 3
        else:
            yield status, fields[i + 1], None
            i += 2


def read_task_meta(rev, path):
    """Return (task_id, title) for a task file at a revision, or (None, None)."""
    text = git("show", f"{rev}:{path}", check=False)
    id_match = ID_RE.search(text)
    title_match = TITLE_RE.search(text)
    task_id = id_match.group(1) if id_match else None
    title = title_match.group(1).strip("'\"") if title_match else None
    if task_id is None:
        filename_match = FILENAME_ID_RE.match(path.rsplit("/", 1)[-1])
        if filename_match:
            task_id = filename_match.group(1)
    return task_id, title


def collect(base, head):
    tasks = OrderedDict()

    def record(task_id, title, change, path):
        entry = tasks.setdefault(
            task_id, {"id": task_id, "title": None, "changes": [], "files": []}
        )
        if title and not entry["title"]:
            entry["title"] = title
        if change not in entry["changes"]:
            entry["changes"].append(change)
        if path not in entry["files"]:
            entry["files"].append(path)

    for status, path, old_path in changed_entries(base, head):
        if status == "A":
            task_id, title = read_task_meta(head, path)
            change = "added"
        elif status == "D":
            task_id, title = read_task_meta(base, path)
            change = "deleted"
        elif status in ("R", "C"):
            task_id, title = read_task_meta(head, path)
            change = "renamed" if status == "R" else "copied"
            old_id, _ = read_task_meta(base, old_path)
            if old_id and task_id and old_id != task_id:
                record(old_id, None, "renamed-from", old_path)
        else:
            task_id, title = read_task_meta(head, path)
            change = "modified"

        if task_id is None:
            print(f"warning: could not identify task for {path}", file=sys.stderr)
            continue
        record(task_id, title, change, path)

    return list(tasks.values())


def main():
    parser = argparse.ArgumentParser(
        description="List Backlog.md tasks changed in a git commit range."
    )
    parser.add_argument(
        "--base",
        help="Start of the range (default: merge-base of --head and --main)",
    )
    parser.add_argument("--head", default="HEAD", help="End of the range (default: HEAD)")
    parser.add_argument(
        "--main", default="main", help="Main branch used for the default base (default: main)"
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON instead of text")
    args = parser.parse_args()

    base = args.base or resolve_default_base(args.main, args.head)
    tasks = collect(base, args.head)

    if args.json:
        print(json.dumps({"base": base, "head": args.head, "tasks": tasks}, indent=2))
        return

    short_base = base[:12]
    print(f"Range: {short_base}..{args.head}")
    print(f"Changed tasks: {len(tasks)}")
    for task in tasks:
        title = f" - {task['title']}" if task["title"] else ""
        print(f"{task['id']}{title}  [{', '.join(task['changes'])}]")
        for path in task["files"]:
            print(f"    {path}")


if __name__ == "__main__":
    main()
