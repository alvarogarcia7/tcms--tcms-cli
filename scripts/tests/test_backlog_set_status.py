"""Tests for scripts/backlog_set_status.py and the shared ID parsing.

Run with:  python3 -m unittest discover -s scripts/tests -v
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from backlog_common import dedupe, parse_ids_from_text, resolve_ids  # noqa: E402
from backlog_set_status import validate_status  # noqa: E402

STATUSES = ["To Do", "In Progress", "In Review", "Done"]


class ValidateStatusTest(unittest.TestCase):
    def test_accepts_each_configured_status(self):
        for status in STATUSES:
            self.assertEqual(validate_status(status, STATUSES), status)

    def test_rejects_free_text(self):
        with self.assertRaises(ValueError) as ctx:
            validate_status("Finished", STATUSES)
        self.assertIn("Finished", str(ctx.exception))
        self.assertIn("Done", str(ctx.exception))

    def test_match_is_exact_not_case_insensitive(self):
        for bad in ["done", "DONE", "in progress", "To-Do", " Done", ""]:
            with self.subTest(status=bad):
                with self.assertRaises(ValueError):
                    validate_status(bad, STATUSES)


class ParseIdsTest(unittest.TestCase):
    def test_reads_ids_from_previous_script_text_output(self):
        text = (
            "Range: abc..HEAD\n"
            "Changed tasks: 2\n"
            "TCMS-40 - Support the glossary  [added]\n"
            "    backlog/tasks/tcms-40 - Support-the-glossary.md\n"
            "TCMS-40.1 - Consume schemas  [modified]\n"
            "    backlog/tasks/tcms-40.1 - Consume-schemas.md\n"
        )
        self.assertEqual(parse_ids_from_text(text), ["TCMS-40", "TCMS-40.1"])

    def test_skips_renamed_from_old_identity(self):
        text = "TCMS-17  [renamed-from]\nTCMS-41 - Verifier  [renamed]\n"
        self.assertEqual(parse_ids_from_text(text), ["TCMS-41"])

    def test_reads_ids_from_json_task_list(self):
        text = '{"schemaVersion": 1, "tasks": [{"id": "TCMS-1"}, {"id": "TCMS-2"}]}'
        self.assertEqual(parse_ids_from_text(text), ["TCMS-1", "TCMS-2"])

    def test_one_id_per_line(self):
        self.assertEqual(parse_ids_from_text("TCMS-1\nTCMS-2\n"), ["TCMS-1", "TCMS-2"])

    def test_dedupe_keeps_first_occurrence_order(self):
        self.assertEqual(dedupe(["B-1", "A-1", "B-1"]), ["B-1", "A-1"])

    def test_positional_ids_take_precedence_over_stdin(self):
        self.assertEqual(resolve_ids(["X-1"], False, "TCMS-9\n"), ["X-1"])

    def test_stdin_used_when_no_positional_ids(self):
        self.assertEqual(resolve_ids([], False, "TCMS-9\n"), ["TCMS-9"])

    def test_no_ids_at_all_is_an_error(self):
        with self.assertRaises(ValueError):
            resolve_ids([], False, None)
        with self.assertRaises(ValueError):
            resolve_ids([], False, "   \n")


if __name__ == "__main__":
    unittest.main()
