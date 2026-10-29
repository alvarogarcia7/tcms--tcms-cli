#!/usr/bin/env python3
"""Tests for check_testcase_files.py. Run: python3 -m unittest scripts/test_check_testcase_files.py"""

import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check_testcase_files as c  # noqa: E402

TC = "type: test_case\nschema: tcms/test-case.schema.v1.json\nid: %s\n"


def write(root, rel, text):
    path = os.path.join(root, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)
    return path


class CheckFile(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = self.tmp.name

    def tearDown(self):
        self.tmp.cleanup()

    def test_good_file(self):
        p = write(self.root, "testcases/TC_A_001.yml", TC % "TC_A_001")
        self.assertEqual(c.check_file(p), [])

    def test_id_differs_from_name(self):
        p = write(self.root, "testcases/TC_A_001.yml", TC % "TC_B_002")
        r = c.check_file(p)
        self.assertEqual(len(r), 1)
        self.assertIn("TC_B_002", r[0])

    def test_yaml_extension(self):
        p = write(self.root, "testcases/TC_A_001.yaml", TC % "TC_A_001")
        r = c.check_file(p)
        self.assertEqual(len(r), 1)
        self.assertIn(".yml", r[0])

    def test_both_problems(self):
        p = write(self.root, "x/TC_A_001.yaml", TC % "OTHER")
        self.assertEqual(len(c.check_file(p)), 2)

    def test_missing_id(self):
        p = write(self.root, "x/TC_A_001.yml", "type: test_case\n")
        self.assertIn("no top-level `id:`", c.check_file(p)[0])

    def test_quoted_id_and_comment(self):
        p = write(self.root, "x/TC_A_001.yml", 'type: "test-case"\nid: "TC_A_001"  # note\n')
        self.assertEqual(c.check_file(p), [])
        q = write(self.root, "x/TC_A_002.yml", "type: test_case # t\nid: TC_A_002 # note\n")
        self.assertEqual(c.check_file(q), [])

    def test_nested_id_is_not_the_id(self):
        p = write(self.root, "x/TC_A_001.yml", "type: test_case\nsequences:\n  - id: TC_A_001\n")
        self.assertIn("no top-level", c.check_file(p)[0])

    def test_not_a_test_case_is_ignored(self):
        p = write(self.root, "x/result.yaml", "type: container\nid: whatever\n")
        self.assertEqual(c.check_file(p), [])
        q = write(self.root, "x/frag.yaml", "foo: bar\n")
        self.assertEqual(c.check_file(q), [])

    def test_numeric_id(self):
        p = write(self.root, "x/gsma.yml", "type: test_case\nid: 1\n")
        self.assertEqual(len(c.check_file(p)), 1)


class Modes(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = self.tmp.name
        self.old = os.getcwd()
        os.chdir(self.root)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.org")
        self.git("config", "user.name", "t")
        write(self.root, "testcases/TC_OLD_001.yaml", TC % "WRONG")  # legacy violation
        write(self.root, "src/readme.txt", "x")
        write(self.root, "test-acceptance/.keep", "")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "base")
        self.git("checkout", "-q", "-b", "feature")

    def tearDown(self):
        os.chdir(self.old)
        self.tmp.cleanup()

    def git(self, *a):
        subprocess.run(["git"] + list(a), check=True, capture_output=True)

    def commit(self, msg="c"):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", msg)

    def run_main(self, *argv):
        return c.main(list(argv))

    def test_full_mode_reports_legacy(self):
        self.assertEqual(self.run_main(), 1)

    def test_full_mode_missing_folder(self):
        os.makedirs("empty")
        self.assertEqual(self.run_main("nope"), 2)

    def test_changed_mode_skips_when_folders_untouched(self):
        write(self.root, "src/other.txt", "y")
        self.commit()
        self.assertEqual(self.run_main("--changed-since", "main"), 0)

    def test_changed_mode_ignores_legacy_checks_new_file(self):
        write(self.root, "test-acceptance/TC_NEW_001.yml", TC % "TC_NEW_001")
        self.commit()
        self.assertEqual(self.run_main("--changed-since", "main"), 0)

    def test_changed_mode_fails_on_bad_new_file(self):
        write(self.root, "test-acceptance/TC_NEW_001.yaml", TC % "TC_NEW_001")
        self.commit()
        self.assertEqual(self.run_main("--changed-since", "main"), 1)

    def test_changed_mode_deleted_file_is_not_checked(self):
        os.remove("testcases/TC_OLD_001.yaml")
        self.commit()
        self.assertEqual(self.run_main("--changed-since", "main"), 0)

    def run_captured(self, *argv):
        import contextlib
        import io
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = c.main(list(argv))
        return rc, out.getvalue(), err.getvalue()

    def test_stdout_is_a_script_with_renames_and_folder_comment(self):
        write(self.root, "testcases/TC_NEW_001.yaml", TC % "TC_NEW_001")
        write(self.root, "testcases/TC_ODD_001.yml", TC % "TC_OTHER_002")
        rc, out, err = self.run_captured("testcases")
        self.assertEqual(rc, 1)
        self.assertIn("# check-testcase-files: reviewed folders: testcases", out)
        self.assertIn("\n[ -e testcases/TC_NEW_001.yml ] || mv -- testcases/TC_NEW_001.yaml testcases/TC_NEW_001.yml", out)  # untracked: plain mv
        self.assertIn("[ -e testcases/WRONG.yml ] || git mv -- testcases/TC_OLD_001.yaml testcases/WRONG.yml", out)  # tracked: git mv
        self.assertIn("[ -e testcases/TC_OTHER_002.yml ] || mv -- testcases/TC_ODD_001.yml testcases/TC_OTHER_002.yml", out)
        self.assertIn("# VIOLATION testcases/TC_NEW_001.yaml: extension must be .yml", out)
        self.assertEqual(err, "")
        for line in out.splitlines():
            self.assertTrue(line.startswith(("#", "[ -e ")), line)  # never plain text

    def test_piped_to_bash_fixes_everything_it_can(self):
        write(self.root, "testcases/TC_NEW_001.yaml", TC % "TC_NEW_001")
        write(self.root, "testcases/TC_ODD_001.yml", TC % "TC_OTHER_002")
        write(self.root, "testcases/sp ace.yml", TC % "TC_SP_001")
        rc, out, _ = self.run_captured("testcases")
        subprocess.run(["bash", "-e"], input=out, text=True, check=True)
        rc2, _, _ = self.run_captured("testcases")
        self.assertEqual(rc2, 0)
        self.assertTrue(os.path.exists("testcases/TC_SP_001.yml"))

    def test_manual_cases_are_comments_not_commands(self):
        write(self.root, "testcases/one.yml", "type: test_case\nid: 1\n")
        write(self.root, "testcases/TC_X_001.yml", TC % "TC_X_001")
        write(self.root, "testcases/TC_X_001.yaml", TC % "TC_X_001")
        _, out, _ = self.run_captured("testcases")
        self.assertIn("# MANUAL testcases/one.yml: id '1' is not a plain name", out)
        self.assertIn("# MANUAL testcases/TC_X_001.yaml: target testcases/TC_X_001.yml already exists", out)
        self.assertNotIn("\nmv -- testcases/one.yml", out)

    def test_destination_is_judged_after_all_renames(self):
        # A.yaml -> B.yml needs B.yml out of the way first, and B.yml -> C.yml does that
        write(self.root, "testcases/A_001.yaml", TC % "B_001")
        write(self.root, "testcases/B_001.yml", TC % "C_001")
        os.remove("testcases/TC_OLD_001.yaml")
        rc, out, _ = self.run_captured("testcases")
        cmds = [l for l in out.splitlines() if l.startswith("[ -e ")]
        self.assertEqual(cmds, ["[ -e testcases/C_001.yml ] || mv -- testcases/B_001.yml testcases/C_001.yml",
                                "[ -e testcases/B_001.yml ] || mv -- testcases/A_001.yaml testcases/B_001.yml"])
        subprocess.run(["bash", "-e"], input=out, text=True, check=True)
        self.assertEqual(self.run_captured("testcases")[0], 0)

    def test_destination_taken_by_a_file_that_stays(self):
        write(self.root, "testcases/B_001.yml", TC % "B_001")
        write(self.root, "testcases/A_001.yml", TC % "B_001")
        _, out, _ = self.run_captured("testcases")
        self.assertIn("# MANUAL testcases/A_001.yml: target testcases/B_001.yml already exists and stays", out)

    def test_rename_cycle_is_manual(self):
        write(self.root, "testcases/A_001.yml", TC % "B_001")
        write(self.root, "testcases/B_001.yml", TC % "A_001")
        _, out, _ = self.run_captured("testcases")
        self.assertIn("rename cycle", out)
        self.assertFalse([l for l in out.splitlines() if l.startswith("[ -e ") and ("A_001" in l or "B_001" in l)])

    def test_guard_skips_a_destination_that_appeared_meanwhile(self):
        write(self.root, "testcases/A_001.yaml", TC % "A_001")
        os.remove("testcases/TC_OLD_001.yaml")
        _, out, _ = self.run_captured("testcases")
        write(self.root, "testcases/A_001.yml", "other\n")  # appears after the plan was made
        subprocess.run(["bash", "-e"], input=out, text=True, check=True)
        self.assertTrue(os.path.exists("testcases/A_001.yaml"))
        self.assertEqual(open("testcases/A_001.yml").read(), "other\n")

    def test_clean_run_prints_only_comments(self):
        os.remove("testcases/TC_OLD_001.yaml")
        write(self.root, "testcases/TC_OK_001.yml", TC % "TC_OK_001")
        rc, out, err = self.run_captured("testcases")
        self.assertEqual((rc, err), (0, ""))
        self.assertTrue(all(l.startswith("#") for l in out.splitlines()))

    def test_changed_mode_bad_ref(self):
        with self.assertRaises(SystemExit):
            self.run_main("--changed-since", "does-not-exist")


if __name__ == "__main__":
    unittest.main()
