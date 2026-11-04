#!/usr/bin/env python3
"""Check that tasks marked Done really are done.

For each in-scope task with status Done, this reports:
  - any acceptance criterion that is not checked, and
  - any subtask, at any depth, whose status is not Done.

Only in-scope tasks are checked as roots. Their subtrees are walked fully, but
the acceptance criteria of descendants are only checked when those descendants
are in scope themselves (e.g. with --all).

Task IDs come from arguments, from --all, or from stdin. Stdin can be the
output of another script here (e.g. backlog_set_status.py).

stdout: IDs that passed (one per line), so the output can be piped on.
stderr: the problem report.
exit:   0 if everything passed, 1 if any problem was found, 2 on usage errors.

Examples:
    scripts/backlog_verify_done.py TCMS-40 TCMS-41
    scripts/backlog_verify_done.py --all
    scripts/backlog_set_status.py --status Done TCMS-40 | scripts/backlog_verify_done.py
"""

import argparse
import sys

from backlog_common import (
    BacklogError,
    read_stdin_if_piped,
    resolve_ids,
    view_task,
)

DONE_STATUS = "Done"


def find_problems(root_id, fetch):
    """Problems with one task marked Done, as human-readable strings.

    `fetch(task_id)` returns the task record (as from `backlog task view`).
    Returns an empty list when the task is not Done or everything is complete.
    """
    root = fetch(root_id)
    if root["status"] != DONE_STATUS:
        return []

    problems = []
    for criterion in root["acceptanceCriteria"]:
        if not criterion["checked"]:
            problems.append(
                f"{root_id}: acceptance criterion #{criterion['index']} not done: "
                f"{criterion['text']}"
            )

    for descendant_id, descendant in walk_subtasks(root, fetch):
        if descendant["status"] != DONE_STATUS:
            problems.append(
                f"{root_id}: subtask {descendant_id} is {descendant['status']!r}, "
                f"not {DONE_STATUS!r}"
            )
    return problems


def walk_subtasks(task, fetch):
    """Yield (id, record) for every descendant of `task`, depth first.

    Guards against cycles, so a malformed parent/child loop cannot hang.
    """
    visited = {task["id"]}
    stack = [subtask["id"] for subtask in reversed(task.get("subtasks", []))]
    while stack:
        task_id = stack.pop()
        if task_id in visited:
            continue
        visited.add(task_id)
        record = fetch(task_id)
        yield task_id, record
        stack.extend(subtask["id"] for subtask in reversed(record.get("subtasks", [])))


def verify(task_ids, fetch):
    """Return (passed_ids, problems_by_id) for the given root IDs."""
    passed = []
    problems_by_id = {}
    for task_id in task_ids:
        problems = find_problems(task_id, fetch)
        if problems:
            problems_by_id[task_id] = problems
        else:
            passed.append(task_id)
    return passed, problems_by_id


def main():
    parser = argparse.ArgumentParser(
        description="Verify that tasks with status Done have all acceptance "
        "criteria checked and all subtasks Done (recursively)."
    )
    parser.add_argument("task_ids", nargs="*", metavar="TASK_ID", help="task IDs to check")
    parser.add_argument("--all", action="store_true", help="check every task")
    args = parser.parse_args()

    try:
        task_ids = resolve_ids(args.task_ids, args.all, read_stdin_if_piped())
    except ValueError as error:
        parser.error(str(error))

    try:
        passed, problems_by_id = verify(task_ids, view_task)
    except BacklogError as error:
        sys.exit(f"error: {error}")

    for task_id in passed:
        print(task_id)

    for problems in problems_by_id.values():
        for problem in problems:
            print(problem, file=sys.stderr)

    print(
        f"checked {len(task_ids)} task(s); {len(problems_by_id)} Done task(s) "
        f"with problems",
        file=sys.stderr,
    )
    sys.exit(1 if problems_by_id else 0)


if __name__ == "__main__":
    main()
