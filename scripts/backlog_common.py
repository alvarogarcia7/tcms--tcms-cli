"""Shared helpers for the Backlog.md scripts in this directory.

The scripts talk to Backlog.md through its CLI (`backlog ... --json`) rather
than editing task files directly, so the CLI stays the single writer.

Task IDs can come from positional arguments, from stdin, or from the output
of another script in this directory (e.g. backlog_changed_tasks.py), so the
scripts can be chained with pipes.
"""

import json
import re
import subprocess
import sys

# TCMS-41, TCMS-40.1, TCMS-40.10 ...
ID_RE = re.compile(r"^[A-Za-z][A-Za-z0-9]*-\d+(?:\.\d+)*$")

# Matches the "old identity" line backlog_changed_tasks.py prints for renamed
# tasks. That ID no longer exists, so it is skipped rather than reported.
RENAMED_FROM_MARKER = "[renamed-from]"


class BacklogError(Exception):
    """A `backlog` CLI call failed."""


def run_backlog(*args):
    result = subprocess.run(["backlog", *args], capture_output=True, text=True)
    if result.returncode != 0:
        message = (result.stderr or result.stdout).strip()
        raise BacklogError(f"backlog {' '.join(args)}: {message}")
    return result.stdout


def backlog_json(*args):
    return json.loads(run_backlog(*args, "--json"))


def configured_statuses():
    """Statuses from backlog/config.yml, in configured order."""
    raw = run_backlog("config", "get", "statuses").strip()
    return [status.strip() for status in raw.split(",") if status.strip()]


def view_task(task_id):
    """Full task record (status, subtasks, acceptance criteria) for one ID."""
    return backlog_json("task", "view", task_id)["task"]


def all_task_ids():
    return [task["id"] for task in backlog_json("task", "list")["tasks"]]


def parse_ids_from_text(text):
    """IDs from either the JSON `task list`-style output or line-based output.

    Line-based: the first whitespace-separated token of each line is taken
    if it looks like a task ID. Other lines (headers, file paths) are ignored.
    """
    stripped = text.strip()
    if stripped.startswith("{"):
        data = json.loads(stripped)
        return [task["id"] for task in data.get("tasks", [])]

    ids = []
    for line in text.splitlines():
        if RENAMED_FROM_MARKER in line:
            continue
        tokens = line.split()
        if tokens and ID_RE.match(tokens[0]):
            ids.append(tokens[0])
    return ids


def dedupe(ids):
    seen = set()
    unique = []
    for task_id in ids:
        if task_id not in seen:
            seen.add(task_id)
            unique.append(task_id)
    return unique


def resolve_ids(positional, use_all, stdin_text):
    """Pick the task IDs in scope for a script run.

    Precedence: --all, then positional IDs, then stdin. Raises ValueError if
    nothing was given.
    """
    if use_all:
        return all_task_ids()
    if positional:
        return dedupe(positional)
    if stdin_text is not None and stdin_text.strip():
        ids = dedupe(parse_ids_from_text(stdin_text))
        if ids:
            return ids
    raise ValueError("no task IDs given: pass IDs, --all, or pipe IDs on stdin")


def read_stdin_if_piped():
    """Stdin text when it is piped or redirected, otherwise None."""
    if sys.stdin is None or sys.stdin.isatty():
        return None
    return sys.stdin.read()
