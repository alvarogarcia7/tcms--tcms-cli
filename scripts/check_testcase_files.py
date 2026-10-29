#!/usr/bin/env python3
"""Check the naming of test case files.

For every test case file (a YAML file whose top-level `type` is `test_case` or `test-case`):

  1. the file name (without extension) equals the top-level `id`
     (TC_MANUAL_ALL_001.yml must contain `id: TC_MANUAL_ALL_001`);
  2. the extension is `.yml`, not `.yaml`.

Other YAML files (results, configs, fragments) are ignored.

Output. Everything printed is shell: `#` comment lines (folders reviewed, summary, each VIOLATION,
and the files that need a MANUAL fix) and one `[ -e DEST ] || git mv SRC DEST` (`mv` for untracked files) per file that
can be renamed; the guard skips a rename whose destination exists when the line runs. Errors are `# ERROR` comments on stderr. So the renames can be applied with

  scripts/check_testcase_files.py | bash

and reviewed first by running it without `| bash`. The new name is `<id>.yml`. A file is not renamed
(a `# MANUAL` comment says why) when its id is not a plain name (letters, digits, `_`, `.`, `-`,
starting with a letter), when the destination is taken by a file that stays (it is judged after all
renames: a file that is itself renamed away does not count, and the commands are ordered for that), when
two files would get the same name, or in a rename cycle.
Other files that mention a renamed path are not updated.

Modes:
  check_testcase_files.py                    check every file below the default folders
  check_testcase_files.py PATH...            check every file below the given folders or files
  check_testcase_files.py --changed-since REF
      CI mode. Looks at the files changed since REF (git diff REF...HEAD, added, copied,
      modified or renamed). If none of them is below the folders, it prints a notice and exits 0
      without checking anything; otherwise it checks only the changed files, so files that were
      already wrong before the change do not fail the pipeline.

Default folders: testcases/ and test-acceptance/ (relative to the repository root).

Exit codes: 0 all good or nothing to check, 1 violations found, 2 usage or git error.
Stdlib only; works with Python 3.8+.
"""

import argparse
import os
import re
import shlex
import subprocess
import sys
from typing import Dict, List, Optional, Set, Tuple

DEFAULT_FOLDERS = ["testcases", "test-acceptance"]
TEST_CASE_TYPES = ("test_case", "test-case")
YAML_EXTENSIONS = (".yml", ".yaml")

_TOP_LEVEL = r"^%s[ \t]*:[ \t]*(.*?)[ \t]*$"
_PLAIN_ID = re.compile(r"^[A-Za-z][A-Za-z0-9_.-]*$")


def top_level_value(text: str, key: str) -> Optional[str]:
    """Value of a top-level scalar `key:` (column 0), without quotes and trailing comment."""
    m = re.search(_TOP_LEVEL % re.escape(key), text, re.M)
    if not m:
        return None
    raw = m.group(1)
    if raw[:1] in ("'", '"'):
        end = raw.find(raw[0], 1)
        return raw[1:end] if end > 0 else raw[1:]
    return re.sub(r"[ \t]+#.*$", "", raw).strip()


def check_file(path: str) -> List[str]:
    """Violations of one file; empty for a file that is fine or is not a test case."""
    try:
        with open(path, encoding="utf-8") as f:
            text = f.read()
    except (OSError, UnicodeDecodeError) as e:
        return ["%s: cannot read: %s" % (path, e)]
    if top_level_value(text, "type") not in TEST_CASE_TYPES:
        return []
    problems = []
    base = os.path.basename(path)
    stem, ext = os.path.splitext(base)
    if ext != ".yml":
        problems.append("%s: extension must be .yml, not %s (rename to %s.yml)" % (path, ext, stem))
    tc_id = top_level_value(text, "id")
    if tc_id is None or tc_id == "":
        problems.append("%s: no top-level `id:`; expected `id: %s`" % (path, stem))
    elif tc_id != stem:
        problems.append("%s: file name says %s but the content has `id: %s`" % (path, stem, tc_id))
    return problems


def tracked_files() -> Set[str]:
    try:
        out = subprocess.run(["git", "ls-files"], check=True, capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return set()
    return set(out.splitlines())


def wanted_name(path: str) -> Tuple[Optional[str], Optional[str]]:
    """(new path, None) for the name `path` should have, (None, reason) when it needs a manual fix."""
    with open(path, encoding="utf-8") as f:
        tc_id = top_level_value(f.read(), "id")
    if not tc_id or not _PLAIN_ID.match(tc_id):
        return None, "id %r is not a plain name; rename by hand" % (tc_id or "")
    return os.path.join(os.path.dirname(path), tc_id + ".yml"), None


def plan_renames(paths: List[str]) -> Tuple[List[Tuple[str, str]], List[Tuple[str, str]]]:
    """([(source, destination)] in an order that is safe to run, [(path, reason)] to fix by hand).

    A destination is judged on the tree as it is AFTER all renames: an existing file in the way is
    fine when it is itself renamed away first, and the commands are ordered for that. Two files
    with the same destination, a destination taken by a file that stays, and cycles go to the
    manual list."""
    manual = []  # type: List[Tuple[str, str]]
    want = {}  # type: Dict[str, str]
    for p in paths:
        new, why = wanted_name(p)
        if new is None:
            manual.append((p, why or ""))
        elif os.path.normpath(new) != os.path.normpath(p):
            want[os.path.normpath(p)] = os.path.normpath(new)

    changed = True
    while changed:  # dropping a rename can block another one, so repeat until stable
        changed = False
        by_dst = {}  # type: Dict[str, List[str]]
        for src, dst in want.items():
            by_dst.setdefault(dst, []).append(src)
        for src in sorted(want):
            dst = want[src]
            if len(by_dst[dst]) > 1:
                why = "%s would get the same name as %s" % (dst, ", ".join(sorted(x for x in by_dst[dst] if x != src)))
            elif os.path.exists(dst) and dst not in want:
                why = "target %s already exists and stays" % dst
            else:
                continue
            manual.append((src, why))
            del want[src]
            changed = True
            break

    ordered = []  # type: List[Tuple[str, str]]
    pending = dict(want)
    while pending:
        # a rename can run once nothing still to be moved sits at its destination
        ready = [s for s, d in sorted(pending.items()) if d not in pending]
        if not ready:
            for s in sorted(pending):
                manual.append((s, "rename cycle with %s" % pending[s]))
            break
        for s in ready:
            ordered.append((s, pending.pop(s)))
    return ordered, sorted(manual)


def move_command(path: str, new: str, tracked: Set[str]) -> str:
    verb = "git mv" if os.path.normpath(path) in tracked else "mv"
    # the guard checks at run time, after the earlier renames, so nothing is ever overwritten
    return "[ -e %s ] || %s -- %s %s" % (shlex.quote(new), verb, shlex.quote(path), shlex.quote(new))


def yaml_files_below(paths: List[str]) -> List[str]:
    found = []
    for p in paths:
        if os.path.isfile(p):
            if p.endswith(YAML_EXTENSIONS):
                found.append(p)
            continue
        for root, _dirs, files in os.walk(p):
            for name in files:
                if name.endswith(YAML_EXTENSIONS):
                    found.append(os.path.join(root, name))
    return sorted(found)


def changed_files(ref: str, folders: List[str]) -> List[str]:
    """Files added, copied, modified or renamed since REF below `folders`."""
    cmd = ["git", "diff", "--name-only", "--diff-filter=ACMR", "%s...HEAD" % ref, "--"] + folders
    try:
        out = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        detail = getattr(e, "stderr", "") or str(e)
        raise SystemExit("# ERROR %s failed: %s" % (" ".join(cmd), " ".join(detail.split())))
    return [line for line in out.splitlines() if line]


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(description="Check test case file names and extensions.")
    ap.add_argument("paths", nargs="*", help="folders or files (default: %s)" % ", ".join(DEFAULT_FOLDERS))
    ap.add_argument("--changed-since", metavar="REF",
                    help="CI mode: do nothing unless files below the folders changed since REF, "
                         "and then check only those files")
    ap.add_argument("-v", "--verbose", action="store_true", help="list the files checked")
    args = ap.parse_args(argv)

    folders = args.paths or DEFAULT_FOLDERS
    reviewed = "reviewed folders: %s" % ", ".join(folders)
    if args.changed_since:
        changed = changed_files(args.changed_since, folders)
        if not changed:
            print("# check-testcase-files: %s" % reviewed)
            print("# nothing changed below them since %s, nothing to check" % args.changed_since)
            return 0
        files = [f for f in changed if f.endswith(YAML_EXTENSIONS) and os.path.isfile(f)]
        scope = "%d changed YAML file(s) since %s" % (len(files), args.changed_since)
    else:
        missing = [p for p in folders if not os.path.exists(p)]
        if missing:
            print("# ERROR not found: %s (run from the repository root)" % ", ".join(missing), file=sys.stderr)
            return 2
        files = yaml_files_below(folders)
        scope = "%d YAML file(s)" % len(files)

    problems = []
    bad = []
    tracked = tracked_files()
    for f in files:
        found = check_file(f)
        if args.verbose:
            print("# %s %s" % ("FAIL" if found else "ok  ", f))
        if found:
            problems.extend(found)
            bad.append(f)

    print("# check-testcase-files: %s" % reviewed)
    if not problems:
        print("# %s checked, all good" % scope)
        return 0
    renames, manual = plan_renames(bad)
    print("# %s checked, %d violation(s) in %d file(s): %d rename(s), %d to fix by hand"
          % (scope, len(problems), len(bad), len(renames), len(manual)))
    for p in problems:
        print("# VIOLATION %s" % p)
    for path, why in manual:
        print("# MANUAL %s: %s" % (path, why))
    for src, dst in renames:
        print(move_command(src, dst, tracked))
    return 1


if __name__ == "__main__":
    sys.exit(main())
