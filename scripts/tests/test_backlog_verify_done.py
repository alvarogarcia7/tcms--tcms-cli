"""Tests for scripts/backlog_verify_done.py.

Run with:  python3 -m unittest discover -s scripts/tests -v
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from backlog_verify_done import find_problems, verify  # noqa: E402


def task(task_id, status="To Do", criteria=(), subtasks=()):
    """A record shaped like `backlog task view --json` output's `task`."""
    return {
        "id": task_id,
        "status": status,
        "acceptanceCriteria": [
            {"index": i, "text": text, "checked": checked}
            for i, (text, checked) in enumerate(criteria, start=1)
        ],
        "subtasks": [{"id": sub_id} for sub_id in subtasks],
    }


def fetcher(*records):
    by_id = {record["id"]: record for record in records}
    return lambda task_id: by_id[task_id]


class FindProblemsTest(unittest.TestCase):
    def test_done_task_with_everything_complete_has_no_problems(self):
        fetch = fetcher(
            task("T-1", "Done", criteria=[("a", True), ("b", True)], subtasks=["T-1.1"]),
            task("T-1.1", "Done", criteria=[("c", True)]),
        )
        self.assertEqual(find_problems("T-1", fetch), [])

    def test_done_task_with_unchecked_acceptance_criterion_is_reported(self):
        fetch = fetcher(task("T-1", "Done", criteria=[("a", True), ("b", False)]))
        problems = find_problems("T-1", fetch)
        self.assertEqual(len(problems), 1)
        self.assertIn("T-1", problems[0])
        self.assertIn("#2", problems[0])
        self.assertIn("b", problems[0])

    def test_done_task_with_direct_child_not_done_is_reported(self):
        fetch = fetcher(
            task("T-1", "Done", subtasks=["T-1.1"]),
            task("T-1.1", "In Progress"),
        )
        problems = find_problems("T-1", fetch)
        self.assertEqual(len(problems), 1)
        self.assertIn("T-1.1", problems[0])
        self.assertIn("In Progress", problems[0])

    def test_not_done_grandchild_is_reported_recursively(self):
        fetch = fetcher(
            task("T-1", "Done", subtasks=["T-1.1"]),
            task("T-1.1", "Done", subtasks=["T-1.1.1"]),
            task("T-1.1.1", "To Do"),
        )
        problems = find_problems("T-1", fetch)
        self.assertEqual(len(problems), 1)
        self.assertIn("T-1.1.1", problems[0])

    def test_problem_below_a_done_child_is_reported(self):
        # The child is Done, but its own child is not, so the walk must go
        # past the first level.
        fetch = fetcher(
            task("T-1", "Done", subtasks=["T-1.1"]),
            task("T-1.1", "Done", subtasks=["T-1.1.1"]),
            task("T-1.1.1", "In Review"),
        )
        problems = find_problems("T-1", fetch)
        self.assertEqual(len(problems), 1)
        self.assertIn("T-1.1.1", problems[0])

    def test_every_non_done_descendant_is_reported(self):
        fetch = fetcher(
            task("T-1", "Done", subtasks=["T-1.1", "T-1.2"]),
            task("T-1.1", "To Do"),
            task("T-1.2", "Done", subtasks=["T-1.2.1"]),
            task("T-1.2.1", "In Progress"),
        )
        problems = find_problems("T-1", fetch)
        self.assertEqual(len(problems), 2)
        self.assertTrue(any("T-1.1 " in p for p in problems))
        self.assertTrue(any("T-1.2.1" in p for p in problems))

    def test_not_done_task_is_never_reported(self):
        fetch = fetcher(
            task("T-1", "In Progress", criteria=[("a", False)], subtasks=["T-1.1"]),
            task("T-1.1", "To Do"),
        )
        self.assertEqual(find_problems("T-1", fetch), [])

    def test_cycle_in_subtasks_does_not_hang(self):
        fetch = fetcher(
            task("T-1", "Done", subtasks=["T-1.1"]),
            task("T-1.1", "Done", subtasks=["T-1", "T-1.1"]),
        )
        self.assertEqual(find_problems("T-1", fetch), [])

    def test_combined_criteria_and_child_problems_are_all_reported(self):
        fetch = fetcher(
            task("T-1", "Done", criteria=[("a", False)], subtasks=["T-1.1"]),
            task("T-1.1", "To Do"),
        )
        self.assertEqual(len(find_problems("T-1", fetch)), 2)


class VerifyTest(unittest.TestCase):
    def test_splits_passed_and_failed_ids_in_input_order(self):
        fetch = fetcher(
            task("T-1", "Done"),
            task("T-2", "Done", criteria=[("a", False)]),
            task("T-3", "To Do"),
        )
        passed, problems_by_id = verify(["T-2", "T-1", "T-3"], fetch)
        self.assertEqual(passed, ["T-1", "T-3"])
        self.assertEqual(list(problems_by_id), ["T-2"])

    def test_no_ids_gives_no_results(self):
        self.assertEqual(verify([], fetcher()), ([], {}))


if __name__ == "__main__":
    unittest.main()
