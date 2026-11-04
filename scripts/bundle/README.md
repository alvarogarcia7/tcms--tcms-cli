# commit-policy

Enforces two rules on commits:

- the author email must be on the repository's allow-list (`.commit-policy`)
- commits must not carry a `Co-Authored-By:` trailer

The toolkit has two parts:

| Part | Where it runs | Installed by |
|------|---------------|--------------|
| git wrapper and `commit-msg` hook | every repository on the machine | `sudo ./install.sh` |
| `pre-push` policy check | one repository | `./install.sh --repo DIR` |

The wrapper sets the commit identity from the repository's `.env` (falling back to `GIT_*` environment variables) and strips `Co-Authored-By` trailers. The pre-push check rejects pushes that contain disallowed commits.

## Requirements

- `git`
- `python3` (the policy checker has no third-party dependencies)
- `sudo` for the system-wide install

## 1. Install the git wrapper (once per machine)

```bash
tar -xzf commit-policy-<stamp>.tar.gz
cd commit-policy
sudo ./install.sh
```

This moves the real git binary from `/usr/bin/git` to `/usr/bin/.git-real`, installs the wrapper at `/usr/bin/git`, and installs the commit-msg hook at `/usr/local/share/git-wrap/hooks/commit-msg`.

Check the result:

```bash
git --version
```

Options:

| Option | Effect |
|--------|--------|
| `--dry-run` | print the actions without changing anything |
| `--bin-dir DIR` | directory holding the git binary (default `/usr/bin`) |
| `--hook-dir DIR` | where the commit-msg hook is installed (default `/usr/local/share/git-wrap/hooks`) |

Preview first with `./install.sh --dry-run` (no sudo needed).

## 2. Enable the pre-push check (each repository)

Add the allow-list to the repository root, one entry per line:

```text
# .commit-policy
alice@users.noreply.github.com
${GIT_AUTHOR_EMAIL}
```

An entry is either a literal email or a variable reference, `$NAME` or `${NAME}`. To keep an address out of git, reference a variable instead of writing the address. The value is read from the `.env` file next to `.commit-policy` first, then from the environment. `.env` is gitignored in this project. Lines may end with `# comment`. An unset variable fails the check, so the check still fails closed.

Then enable the check:

```bash
./install.sh --repo /path/to/repo
```

This copies `commit_policy.py` and `scripts/git-hooks/pre-push` into `<repo>/scripts/` and sets `core.hooksPath=scripts/git-hooks` for that repository only. Commit `scripts/commit_policy.py` and `scripts/git-hooks/pre-push` to the repository so that other clones get them too.

A missing `.commit-policy` makes the pre-push check fail, so the check is closed by default.

## 3. Check the policy by hand

```bash
cd /path/to/repo
scripts/git-hooks/pre-push origin <<< "refs/heads/main $(git rev-parse HEAD) refs/heads/main 0000000000000000000000000000000000000000"
```

Exit code `0` means every commit passes. `1` means a policy violation, and the offending commits are listed. `2` means the check itself failed.

## Uninstall

```bash
sudo ./install.sh --uninstall      # restores the original git and removes the hook
```

To disable the pre-push check in one repository:

```bash
git -C /path/to/repo config --unset core.hooksPath
```

## Troubleshooting

- **`/usr/bin is not writable`**: run the system install with `sudo`.
- **`.git-real already exists`**: an earlier install did not finish cleanly. Compare `/usr/bin/git` with `/usr/bin/.git-real` before removing anything.
- **Commits still carry trailers or the wrong author**: another `git` earlier on PATH may be shadowing the wrapper. Run `which -a git`. Remove or reorder any extra wrapper, for example `~/.local/bin/git`, so that the system wrapper is found first.
- **Pre-push fails on every push with `.commit-policy` missing**: add the file to the repository root (step 2).
- **Tests fail with a Co-Authored-By mismatch**: the tests locate a real git binary automatically. Set `TEST_GIT=/path/to/git` to force a specific binary.

## Running the tests

The tests need `pytest`:

```bash
python3 -m pip install pytest
python3 -m pytest scripts/tests -q
```
