#!/usr/bin/env python3
"""Set the status of many Backlog.md tasks at once.

The status must be one of the statuses configured in backlog/config.yml
(exact match). Free text is rejected. Every task ID is checked to exist before
any task is changed, so a typo does not leave a half-applied update.

Task IDs come from arguments, from --all, or from stdin. Stdin can be the
output of another script here (e.g. backlog_changed_tasks.py).

stdout: the IDs that were set (one per line), so the output can be piped on.
stderr: progress and errors.

Examples:
    scripts/backlog_set_status.py --status "In Progress" TCMS-40 TCMS-41
    scripts/backlog_changed_tasks.py | scripts/backlog_set_status.py --status "In Review"
    scripts/backlog_set_status.py --status "To Do" --all --dry-run
    scripts/backlog_set_status.py --sir TCMS-40 TCMS-41
"""

import argparse
import sys

from backlog_common import (
    BacklogError,
    configured_statuses,
    read_stdin_if_piped,
    resolve_ids,
    run_backlog,
    view_task,
)

# Shorthand flags for the common statuses. Each one is the same as
# --status with the status shown.
STATUS_ALIASES = {
    "--st": "To Do",
    "--sip": "In Progress",
    "--sir": "In Review",
    "--sd": "Done",
}


def validate_status(status, valid_statuses):
    """Return the status if it is exactly one of the configured statuses."""
    if status in valid_statuses:
        return status
    raise ValueError(
        f"invalid status {status!r}; valid statuses: {', '.join(valid_statuses)}"
    )


def main():
    parser = argparse.ArgumentParser(
        description="Set the status of Backlog.md tasks to a configured status."
    )
    parser.add_argument("task_ids", nargs="*", metavar="TASK_ID", help="task IDs to update")
    status_group = parser.add_mutually_exclusive_group(required=True)
    alias_help = ", ".join(f"{flag} = {status!r}" for flag, status in STATUS_ALIASES.items())
    status_group.add_argument(
        "-s",
        "--status",
        help=f"target status (must be configured). Aliases: {alias_help}",
    )
    for flag, status in STATUS_ALIASES.items():
        status_group.add_argument(
            flag,
            dest="status",
            action="store_const",
            const=status,
            help=f"same as --status {status!r}",
        )
    parser.add_argument("--all", action="store_true", help="update every task")
    parser.add_argument("-n", "--dry-run", action="store_true", help="show changes without applying")
    args = parser.parse_args()

    try:
        status = validate_status(args.status, configured_statuses())
        task_ids = resolve_ids(args.task_ids, args.all, read_stdin_if_piped())
        # Look every task up before editing any, so a bad ID aborts the run.
        current = {task_id: view_task(task_id)["status"] for task_id in task_ids}
    except (ValueError, BacklogError) as error:
        sys.exit(f"error: {error}")

    for task_id in task_ids:
        print(f"{task_id}: {current[task_id]} -> {status}", file=sys.stderr)
        if not args.dry_run:
            try:
                run_backlog("task", "edit", task_id, "-s", status)
            except BacklogError as error:
                sys.exit(f"error: {error}")
        print(task_id)

    if args.dry_run:
        print(f"dry run: {len(task_ids)} task(s) not changed", file=sys.stderr)


if __name__ == "__main__":
    main()
