"""Tests for scripts/commit_policy.py."""

import os
import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

import commit_policy  # noqa: E402
from commit_policy import Commit, load_allowed, problems_for  # noqa: E402

ALLOWED = {"allowed@example.com"}


def test_load_allowed_ignores_comments_blank_lines_and_case(tmp_path):
    policy = tmp_path / ".commit-policy"
    policy.write_text("# header\n\nAllowed.User@Example.com  # laptop\n", encoding="utf-8")
    assert load_allowed(policy) == {"allowed.user@example.com"}


def test_load_allowed_expands_variables_from_env_file(tmp_path, monkeypatch):
    monkeypatch.delenv("POLICY_EMAIL", raising=False)
    (tmp_path / ".env").write_text(
        "# local identity\nGIT_AUTHOR_EMAIL=\"Hidden.User@Example.com\"\n", encoding="utf-8"
    )
    policy = tmp_path / ".commit-policy"
    policy.write_text("${GIT_AUTHOR_EMAIL}  # laptop\n$GIT_AUTHOR_EMAIL\n", encoding="utf-8")
    assert load_allowed(policy) == {"hidden.user@example.com"}


def test_load_allowed_env_file_wins_over_environment(tmp_path, monkeypatch):
    monkeypatch.setenv("POLICY_EMAIL", "from-env@example.com")
    (tmp_path / ".env").write_text("POLICY_EMAIL=from-file@example.com\n", encoding="utf-8")
    policy = tmp_path / ".commit-policy"
    policy.write_text("${POLICY_EMAIL}\n", encoding="utf-8")
    assert load_allowed(policy) == {"from-file@example.com"}


def test_load_allowed_falls_back_to_environment(tmp_path, monkeypatch):
    monkeypatch.setenv("POLICY_EMAIL", "from-env@example.com")
    policy = tmp_path / ".commit-policy"
    policy.write_text("${POLICY_EMAIL}\n", encoding="utf-8")
    assert load_allowed(policy) == {"from-env@example.com"}


def test_load_allowed_unset_variable_fails_closed(tmp_path, monkeypatch):
    monkeypatch.delenv("POLICY_EMAIL", raising=False)
    policy = tmp_path / ".commit-policy"
    policy.write_text("${POLICY_EMAIL}\n", encoding="utf-8")
    with pytest.raises(ValueError, match="POLICY_EMAIL"):
        load_allowed(policy)


def test_load_allowed_rejects_empty_policy(tmp_path):
    policy = tmp_path / ".commit-policy"
    policy.write_text("# nothing listed\n", encoding="utf-8")
    with pytest.raises(ValueError):
        load_allowed(policy)


def test_load_allowed_missing_file_fails_closed(tmp_path):
    with pytest.raises(OSError):
        load_allowed(tmp_path / "missing")


@pytest.mark.parametrize(
    "message",
    [
        "Co-Authored-By: Claude <noreply@anthropic.com>",
        "co-authored-by: someone <x@y.z>",
        "CO-AUTHORED-BY: someone <x@y.z>",
        "  Co-Authored-By : someone",
    ],
)
def test_trailer_variants_are_rejected(message):
    problems = problems_for(Commit("a" * 40, "allowed@example.com", f"subject\n\n{message}\n"), ALLOWED)
    assert problems == ["message contains a Co-Authored-By trailer"]


def test_unlisted_author_is_rejected():
    problems = problems_for(Commit("b" * 40, "other@example.com", "subject\n"), ALLOWED)
    assert len(problems) == 1
    assert "other@example.com" in problems[0]


def test_listed_author_clean_message_passes():
    assert problems_for(Commit("c" * 40, "Allowed@Example.com", "subject\n"), ALLOWED) == []


def test_text_mentioning_coauthor_mid_line_is_not_a_trailer():
    message = "Explain why Co-Authored-By is removed by the wrapper\n"
    assert problems_for(Commit("d" * 40, "allowed@example.com", message), ALLOWED) == []


WRAPPER_MARKER = "# git wrapper:"


def _real_git():
    """Return the first git on PATH that is not the scripts/git-wrapper wrapper.

    The wrapper forces the commit identity from .env and strips Co-Authored-By
    trailers, which would hide the author and trailers these tests create.
    TEST_GIT, when set, overrides the search.
    """
    if "TEST_GIT" in os.environ:
        return os.environ["TEST_GIT"]
    for directory in os.environ.get("PATH", "").split(os.pathsep):
        candidate = os.path.join(directory, "git")
        if not os.path.isfile(candidate) or not os.access(candidate, os.X_OK):
            continue
        with open(candidate, "rb") as handle:
            head = handle.read(4096)
        if WRAPPER_MARKER.encode() not in head:
            return candidate
    raise RuntimeError("no real git binary found on PATH")


GIT = _real_git()


def _git(repo, *args, env_email="allowed@example.com"):
    return subprocess.run(
        [
            GIT,
            "-c", "user.name=Test",
            "-c", f"user.email={env_email}",
            "-C", str(repo),
            *args,
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


@pytest.fixture
def repo(tmp_path):
    path = tmp_path / "repo"
    path.mkdir()
    subprocess.run([GIT, "init", "-q", "-b", "main", str(path)], check=True)
    (path / "a.txt").write_text("a\n", encoding="utf-8")
    _git(path, "add", "a.txt")
    _git(path, "commit", "-q", "-m", "base")
    return path


def test_pre_push_reports_bad_author_and_trailer(repo, monkeypatch):
    base = _git(repo, "rev-parse", "HEAD").strip()
    (repo / "b.txt").write_text("b\n", encoding="utf-8")
    _git(repo, "add", "b.txt")
    _git(repo, "commit", "-q", "-m", "bad", "-m", "Co-Authored-By: X <x@y.z>", env_email="other@example.com")
    tip = _git(repo, "rev-parse", "HEAD").strip()

    monkeypatch.chdir(repo)
    lines = [f"refs/heads/main {tip} refs/heads/main {base}"]
    failures = commit_policy.run_pre_push("origin", lines, ALLOWED)
    assert any("other@example.com" in f for f in failures)
    assert any("Co-Authored-By" in f for f in failures)


def test_pre_push_skips_branch_deletion(repo):
    lines = [f"(delete) {'0' * 40} refs/heads/gone {'a' * 40}"]
    assert commit_policy.run_pre_push("origin", lines, ALLOWED) == []


def test_github_commits_maps_compare_response(monkeypatch):
    def fake(method, url, headers, body=None):
        assert method == "GET"
        assert url == "https://api.github.com/repos/o/r/compare/main...abc"
        assert headers["Authorization"] == "Bearer tok"
        return {
            "total_commits": 1,
            "commits": [{"sha": "abc", "commit": {"author": {"email": "allowed@example.com"}, "message": "m"}}],
        }

    monkeypatch.setattr(commit_policy, "http_json", fake)
    commits = commit_policy.github_commits("https://api.github.com", "o/r", "main", "abc", "tok")
    assert commits == [Commit("abc", "allowed@example.com", "m")]


def test_github_commits_refuses_truncated_compare(monkeypatch):
    def fake(method, url, headers, body=None):
        return {"total_commits": 300, "commits": [{"sha": "abc", "commit": {"author": {"email": "e"}, "message": "m"}}]}

    monkeypatch.setattr(commit_policy, "http_json", fake)
    with pytest.raises(ValueError):
        commit_policy.github_commits("https://api.github.com", "o/r", "main", "abc", "tok")


def test_gitlab_commits_maps_compare_response(monkeypatch):
    def fake(method, url, headers, body=None):
        assert url.startswith("https://gitlab.com/api/v4/projects/grp%2Fproj/repository/compare")
        assert headers == {"PRIVATE-TOKEN": "tok"}
        return {"commits": [{"id": "abc", "author_email": "allowed@example.com", "message": "m"}]}

    monkeypatch.setattr(commit_policy, "http_json", fake)
    commits = commit_policy.gitlab_commits("https://gitlab.com/api/v4", "grp/proj", "main", "abc", "tok")
    assert commits == [Commit("abc", "allowed@example.com", "m")]


def test_missing_token_fails_closed(monkeypatch):
    monkeypatch.delenv("GITHUB_TOKEN", raising=False)
    with pytest.raises(ValueError):
        commit_policy.require_token("GITHUB_TOKEN")


def test_main_exit_codes_for_hosted_dry_run(tmp_path, monkeypatch, capsys):
    policy = tmp_path / ".commit-policy"
    policy.write_text("allowed@example.com\n", encoding="utf-8")
    monkeypatch.setenv("GITHUB_TOKEN", "tok")

    def fake_bad(method, url, headers, body=None):
        return {"total_commits": 1, "commits": [{"sha": "abc", "commit": {"author": {"email": "other@x.y"}, "message": "m"}}]}

    monkeypatch.setattr(commit_policy, "http_json", fake_bad)
    code = commit_policy.main(["--policy", str(policy), "github", "--repo", "o/r", "--base", "main", "--head", "abc", "--dry-run"])
    assert code == 1
    assert "other@x.y" in capsys.readouterr().err

    def fake_good(method, url, headers, body=None):
        return {"total_commits": 1, "commits": [{"sha": "abc", "commit": {"author": {"email": "allowed@example.com"}, "message": "m"}}]}

    monkeypatch.setattr(commit_policy, "http_json", fake_good)
    assert commit_policy.main(["--policy", str(policy), "github", "--repo", "o/r", "--base", "main", "--head", "abc", "--dry-run"]) == 0


def test_main_returns_2_when_check_cannot_run(tmp_path, monkeypatch):
    monkeypatch.delenv("GITLAB_TOKEN", raising=False)
    policy = tmp_path / ".commit-policy"
    policy.write_text("allowed@example.com\n", encoding="utf-8")
    code = commit_policy.main(["--policy", str(policy), "gitlab", "--repo", "g/p", "--base", "main", "--head", "abc"])
    assert code == 2
