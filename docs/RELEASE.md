# Releases on GitHub

Before creating a release, the tag must include `-rc` (for example `v2026-11-03_new-crate-rc.1`).
Without it the GitHub release does not work. The release workflow always marks the release as a
prerelease (`--prerelease`), so the tag name is the only thing that needs to follow this rule.

The tag must point at a commit whose message has no `[ci skip]` or `[skip ci]` marker, otherwise
GitHub does not start the release run.
