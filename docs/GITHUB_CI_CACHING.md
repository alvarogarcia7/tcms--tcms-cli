# GitHub Actions: Rust Build Caching Decision

**Decision: do not use sccache in the GitHub Actions pipeline (`.github/workflows/workspace.yml`).**
The workflow caches Rust dependencies with `RustCrypto/actions/cargo-cache`.

This applies to GitHub CI only. sccache remains the compiler wrapper for local development
and the GitLab pipeline (see [SCCACHE.md](SCCACHE.md)).

## Why

We measured sccache on this pipeline against the existing cargo-cache setup, on the same commit
range with the same change-detection and test steps:

| Variant | Run | Wall-clock |
|---------|-----|-----------|
| cargo-cache (current) | 41067535360 | 280s |
| cargo-cache (current) | 41071157723 | 251s |
| sccache-action v0.0.11, cold | 41068407869 | 412s |
| sccache-action v0.0.11, warm | 41069465534 | 343s |
| sccache-action v0.0.11, warm | 41070369605 | 325s |

Warm sccache averaged about 334s against about 266s for cargo-cache, so sccache was roughly
25% slower. The per-step breakdown showed the slowdown in Build (79-113s vs 58s), Clippy
(52-83s vs 38s) and E2E (86-97s vs 88s). The cold sccache run was the slowest of all, as expected
for a first cache fill.

Sccache uses the GitHub Actions cache backend, which stores each compiled artifact as a separate
cache entry. On a runner that compiles this workspace only a few times per branch, the upload and
restore overhead appears to outweigh the saved compilation. We did not profile the backend to
confirm this; the measured result is what counts.

The decision follows the project rule that a change to the pipeline is kept only if it improves
pipeline time.

## Rules

- Do not add `mozilla-actions/sccache-action`, `SCCACHE_GHA_ENABLED`, or `RUSTC_WRAPPER: sccache`
  to the GitHub workflow.
- Keep `RustCrypto/actions/cargo-cache` as the Rust dependency cache.
- Revisit only with a new measurement: at least two warm runs on the same commit, compared against
  the current baseline, with the run IDs recorded here.

## Related fixes in the same change

These are not caching changes, but they affect how the pipeline's timings should be read:

- **Push change detection:** on the first push of a new branch, `github.event.before` is all zeros,
  so the change detector used to see no changed files and silently skip build, clippy and unit
  tests. Push events now diff against `origin/main`.
- **E2E fixture:** `test_auto_schema_validation_e2e.sh` used a `TC_VERIFIER_002.yaml` fixture that
  the test-case schema rejects. The verifier now exits 1 on unverifiable logs, which failed the
  "verifier logs schema field" check. The fixture now matches `TC_VERIFIER_001.yaml`.
