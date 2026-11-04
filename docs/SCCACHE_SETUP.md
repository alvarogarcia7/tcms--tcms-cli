# sccache Setup Guide

Quick guide to set up and troubleshoot sccache compilation cache.

## ⚠️ IMPORTANT: If Build is Currently Failing

If you're seeing compilation errors with sccache right now:

```bash
# IMMEDIATELY disable sccache
unset RUSTC_WRAPPER

# Clean and rebuild
cargo clean
cargo build
```

Then continue reading to set up sccache properly.

## Quick Setup

```bash
# 1. Install sccache (one-time)
make install-sccache

# 2. Enable sccache
source ./scripts/enable-sccache.sh

# 3. Build your project
cargo build

# 4. Verify cache is working
make sccache-check
```

## Quick Fix: If Build Fails with sccache

If you see compilation errors with exit status 254:

```bash
# Disable sccache
source ./scripts/disable-sccache.sh

# Clean and rebuild
cargo clean
cargo build
```

## Troubleshooting: Cache Directory is Empty

If `~/.cache/sccache/testcase-manager` is empty after building, sccache is not enabled.

### Check if sccache is Enabled

```bash
echo $RUSTC_WRAPPER
```

**Expected output**: `sccache`

**If empty or different**: sccache is not enabled.

### Solution: Enable sccache

```bash
# Enable for current session
source ./scripts/enable-sccache.sh

# Or enable permanently (adds to ~/.bashrc or ~/.zshrc)
source ./scripts/enable-sccache.sh --permanent
```

### Verify Setup

```bash
# Check configuration
make sccache-check

# Should output:
# ✅ sccache is enabled (RUSTC_WRAPPER=sccache)
# Cache directory: ~/.cache/sccache/testcase-manager
```

### Test the Cache

```bash
# Clean build (populates cache)
cargo clean
cargo build

# Check cache statistics
sccache --show-stats

# Should show:
# Compile requests: XXX
# Cache hits: 0 (first build)
# Cache misses: XXX

# Rebuild (uses cache)
cargo clean
cargo build

# Check again
sccache --show-stats

# Should show cache hits now
```

## Common Issues

### Issue 1: RUSTC_WRAPPER not set

**Symptom**: Cache directory is empty after building

**Cause**: sccache is not enabled

**Solution**:
```bash
source ./scripts/enable-sccache.sh
```

### Issue 2: Cache directory doesn't exist

**Symptom**: Directory `~/.cache/sccache/testcase-manager` doesn't exist

**Cause**: No builds have been run with sccache enabled

**Solution**: Enable sccache and run a build:
```bash
source ./scripts/enable-sccache.sh
cargo build
ls -la ~/.cache/sccache/testcase-manager  # Should now exist
```

### Issue 3: sccache not installed

**Symptom**: `sccache: command not found`

**Cause**: sccache is not installed

**Solution**:
```bash
make install-sccache
```

### Issue 4: Changes don't persist across terminal sessions

**Symptom**: RUSTC_WRAPPER is empty in new terminal sessions

**Cause**: Not added to shell profile

**Solution**: Enable permanently
```bash
source ./scripts/enable-sccache.sh --permanent
```

This adds `export RUSTC_WRAPPER=sccache` to your `~/.bashrc` or `~/.zshrc`.

### Issue 5: Compilation fails with sccache (exit status 254)

**Symptom**: Build fails with error like:
```
error occurred in cc-rs: command did not execute successfully 
(status code exit status: 254): env ... "sccache" "cc" ...
```

**Cause**: sccache may have issues with certain C compilation scenarios, particularly with:
- Native dependencies (libssh2-sys, openssl-sys, etc.)
- Cross-compilation
- Certain cc-rs build scripts

**Solution 1**: Disable sccache for the current session
```bash
source ./scripts/disable-sccache.sh
# Or manually
unset RUSTC_WRAPPER
```

**Solution 2**: Clean and rebuild
```bash
# Disable sccache
source ./scripts/disable-sccache.sh

# Clean build
cargo clean

# Rebuild without sccache
cargo build
```

**Solution 3**: Stop sccache server and retry
```bash
sccache --stop-server
cargo clean
cargo build
```

**Solution 4**: Clear sccache cache
```bash
rm -rf ~/.cache/sccache/testcase-manager
cargo clean
cargo build
```

**Note**: If sccache consistently causes issues, you can work without it. To remove from shell profile:
```bash
# Edit ~/.bashrc or ~/.zshrc and remove:
# export RUSTC_WRAPPER=sccache
```

## Verification Checklist

- [ ] sccache is installed: `which sccache`
- [ ] RUSTC_WRAPPER is set: `echo $RUSTC_WRAPPER` (should be `sccache`)
- [ ] Cache directory exists: `ls -la ~/.cache/sccache/testcase-manager`
- [ ] Cache has content: `du -sh ~/.cache/sccache/testcase-manager`
- [ ] Cache statistics show activity: `sccache --show-stats`

## Make Targets Reference

```bash
make install-sccache   # Install sccache
make enable-sccache    # Show enable instructions
make sccache-check     # Verify sccache is enabled
make sccache-stats     # Show cache statistics
make sccache-clean     # Stop sccache server (preserves cache)
```

## Additional Help

For comprehensive documentation, see:
- [docs/SCCACHE.md](SCCACHE.md) - Complete sccache guide
- [AGENTS.md](../AGENTS.md) - Project commands reference

## Summary

**Key Point**: The cache directory exported by `scripts/enable-sccache.sh` only specifies WHERE the cache will be stored. For sccache to actually cache compilations, you must:

1. Install sccache: `make install-sccache`
2. Enable it: `source ./scripts/enable-sccache.sh`
3. Build: `cargo build`

Without step 2, cargo will not use sccache, and the cache directory will remain empty.
