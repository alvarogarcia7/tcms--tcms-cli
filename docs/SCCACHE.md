# sccache: Shared Compilation Cache

This guide covers everything you need to know about using sccache to accelerate Rust compilation in this project, including installation, configuration, usage, cache management, and troubleshooting.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage with Makefile](#usage-with-makefile)
- [Cache Management](#cache-management)
- [Performance Benefits](#performance-benefits)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

## Overview

sccache (Shared Compiler Cache) is a ccache-like compiler caching tool that dramatically speeds up Rust compilation by caching compilation results and reusing them when building the same code again.

**⚠️ QUICK FIX: If Build is Failing Right Now**

If you're seeing compilation errors with exit status 254:
```bash
unset RUSTC_WRAPPER
cargo clean
cargo build
```
See [Troubleshooting](#issue-compilation-failures-with-sccache-exit-status-254) for details.

### Global Cache Configuration (This Project)

**This project uses a global cache directory** shared across all worktrees and git checkouts. This is configured in `.cargo/config.toml`:

```toml
[env]
SCCACHE_DIR = { value = "$HOME/.cache/sccache/testcase-manager", force = true, relative = false }
```

**Location**: `~/.cache/sccache/testcase-manager`

**Key Benefits**:
1. **Worktree Sharing**: Multiple worktrees share the same compilation cache
2. **Persistent Cache**: Cache survives repository deletion and recreation
3. **Faster Builds**: Dramatically reduced compilation time across all worktrees
4. **Disk Efficiency**: Single cache location instead of per-worktree caches

**Example Use Case**:
```bash
# Worktree 1: Full build (populates global cache)
cd ~/projects/tcms-main
cargo build --release  # Takes 2 minutes

# Worktree 2: Same code, instant cache hit
cd ~/projects/tcms-feature
cargo build --release  # Takes 20 seconds (cache hit!)

# Worktree 3: Similar code, partial cache hit
cd ~/projects/tcms-hotfix
cargo build --release  # Takes 30 seconds (partial cache hit)
```

**Override if Needed**: Set `SCCACHE_DIR` environment variable to use a different location (overrides `.cargo/config.toml`).

**⚠️ IMPORTANT - sccache Must Be Enabled**:
The global cache directory will remain empty until sccache is enabled. After installation, you must set `RUSTC_WRAPPER=sccache`:
```bash
source ./scripts/enable-sccache.sh
# Then run a build
cargo build
```

### What is sccache?

sccache is a compilation cache that:
- **Caches compilation results** to avoid recompiling unchanged code
- **Supports multiple languages** including Rust, C, C++, and more
- **Provides distributed caching** via Redis, S3, GCS, and other backends
- **Works transparently** with cargo and other build systems
- **Reduces compilation time** by 50-90% for incremental builds

### How It Works

1. When cargo compiles a crate, it invokes `rustc` through sccache
2. sccache computes a hash of the source code and compiler flags
3. If the hash exists in the cache, sccache returns the cached result
4. If not, sccache runs the compiler and caches the output
5. Subsequent builds with the same code reuse cached results

### Key Benefits

- **Faster rebuilds**: Avoid recompiling unchanged dependencies
- **Faster CI/CD**: Share cache between pipeline runs
- **Faster team builds**: Share cache across team members
- **Disk-based caching**: Works locally without infrastructure
- **Cloud-based caching**: Optional distributed cache for teams

## Installation

sccache can be installed using the provided installation script or manually.

### Using the Installation Script

The fastest way to install sccache is using the automated installation script:

```bash
make install-sccache
```

This runs `scripts/install-sccache.sh --local` which:
- Auto-detects your platform (Linux/macOS, x86_64/aarch64)
- Downloads the appropriate pre-built binary
- Installs to `~/.cargo/bin/` or `/usr/local/bin/`
- Verifies the installation

### Manual Installation

#### Option 1: Pre-built Binary (Fastest)

Download the latest release from GitHub:

```bash
# Linux x86_64
curl -L https://github.com/mozilla/sccache/releases/download/v0.7.7/sccache-v0.7.7-x86_64-unknown-linux-musl.tar.gz | tar xz
chmod +x sccache-v0.7.7-x86_64-unknown-linux-musl/sccache
mv sccache-v0.7.7-x86_64-unknown-linux-musl/sccache ~/.cargo/bin/

# macOS x86_64
curl -L https://github.com/mozilla/sccache/releases/download/v0.7.7/sccache-v0.7.7-x86_64-apple-darwin.tar.gz | tar xz
chmod +x sccache-v0.7.7-x86_64-apple-darwin/sccache
mv sccache-v0.7.7-x86_64-apple-darwin/sccache ~/.cargo/bin/

# macOS ARM64 (Apple Silicon)
curl -L https://github.com/mozilla/sccache/releases/download/v0.7.7/sccache-v0.7.7-aarch64-apple-darwin.tar.gz | tar xz
chmod +x sccache-v0.7.7-aarch64-apple-darwin/sccache
mv sccache-v0.7.7-aarch64-apple-darwin/sccache ~/.cargo/bin/
```

#### Option 2: Cargo Install (Slower)

Install from source using cargo (takes 5-10 minutes):

```bash
cargo install sccache
```

### Verify Installation

After installation, verify that sccache is available:

```bash
sccache --version
```

Expected output:
```
sccache 0.7.7
```

### Enable sccache for Rust

**IMPORTANT**: After installing sccache, you must enable it by setting the `RUSTC_WRAPPER` environment variable. The global cache directory will remain empty until sccache is enabled and a build is run.

#### Quick Setup (Recommended)

Use the provided script for easy setup:

```bash
# Enable for current shell session
source ./scripts/enable-sccache.sh

# Or enable permanently (adds to ~/.bashrc or ~/.zshrc)
source ./scripts/enable-sccache.sh --permanent

# Verify it's enabled
make sccache-check
```

#### Manual Setup

Alternatively, set the environment variable manually:

```bash
export RUSTC_WRAPPER=sccache
```

Add to your shell profile for persistent use:

```bash
# For bash (~/.bashrc or ~/.bash_profile)
echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
source ~/.bashrc

# For zsh (~/.zshrc)
echo 'export RUSTC_WRAPPER=sccache' >> ~/.zshrc
source ~/.zshrc
```

#### Verify sccache is Enabled

```bash
# Check with Make
make sccache-check

# Or check manually
echo $RUSTC_WRAPPER  # Should output: sccache

# Build and verify cache is being used
cargo build
sccache --show-stats  # Should show compile requests and cache hits
```

## Configuration

sccache can be configured using environment variables to customize cache behavior, size, and backend.

### Basic Configuration

#### Cache Directory

**This project uses a global cache directory** configured in `.cargo/config.toml` to enable cache sharing across multiple worktrees:

```toml
[env]
SCCACHE_DIR = { value = "$HOME/.cache/sccache/testcase-manager", force = true, relative = false }
```

**Global Cache Location**: `~/.cache/sccache/testcase-manager`

**Benefits of Global Cache**:
- **Worktree Sharing**: Multiple worktrees of this repository share the same cache
- **Persistent Cache**: Cache survives repository deletion and recreation  
- **Faster Builds**: Reduced compilation time across all worktrees
- **Disk Efficiency**: Single cache location instead of per-worktree caches

**Default sccache locations** (if not configured):
- **Linux**: `~/.cache/sccache`
- **macOS**: `~/Library/Caches/Mozilla.sccache`
- **Windows**: `%LOCALAPPDATA%\Mozilla\sccache\cache`

**Override global configuration** (if needed):

```bash
# Override .cargo/config.toml setting
export SCCACHE_DIR=/my/custom/cache
```

**Note**: Environment variables override `.cargo/config.toml` settings.

#### Cache Size

The default cache size is 10GB. To change it:

```bash
export SCCACHE_CACHE_SIZE=20G
```

Size format:
- `10G` - 10 gigabytes
- `5000M` - 5000 megabytes
- `100000K` - 100000 kilobytes

#### Logging

Enable debug logging to troubleshoot issues:

```bash
export SCCACHE_LOG=debug
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

### Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `RUSTC_WRAPPER` | Enable sccache for Rust | (none) |
| `SCCACHE_DIR` | Cache directory location | Platform-specific |
| `SCCACHE_CACHE_SIZE` | Maximum cache size | `10G` |
| `SCCACHE_LOG` | Log level | `error` |
| `SCCACHE_REDIS` | Redis server for distributed cache | (none) |
| `SCCACHE_BUCKET` | S3 bucket for distributed cache | (none) |
| `SCCACHE_REGION` | AWS region for S3 | (none) |
| `SCCACHE_ENDPOINT` | Custom S3 endpoint | (none) |

### Example Configuration

Add to your shell profile:

```bash
# Enable sccache for Rust
export RUSTC_WRAPPER=sccache

# Configure cache location and size
export SCCACHE_DIR=~/.cache/sccache
export SCCACHE_CACHE_SIZE=20G

# Enable info logging
export SCCACHE_LOG=info
```

## Usage with Makefile

The project's Makefile includes several targets for working with sccache.

### Available Make Targets

#### `make install-sccache`

**Purpose**: Install sccache on your system.

**Command**: `./scripts/install-sccache.sh --local`

**When to use**:
- First-time setup
- Updating to a newer version
- Installing on a new machine

**Example**:
```bash
$ make install-sccache
Starting sccache installation
Auto-detected local development environment

Detected platform: macos-aarch64
Downloading sccache v0.7.7...
✓ Downloaded sccache archive
✓ Extracted sccache archive
✓ Installed sccache to ~/.cargo/bin/sccache
✓ sccache is now available in PATH

Installation complete!
```

#### `make sccache-stats`

**Purpose**: Display current cache statistics.

**Command**: `sccache --show-stats`

**When to use**:
- Checking cache hit rate
- Monitoring cache effectiveness
- Verifying sccache is working
- Debugging performance issues

**Example**:
```bash
$ make sccache-stats
Compile requests                    456
Compile requests executed           123
Cache hits                          333
Cache hits (Rust)                   333
Cache misses                        123
Cache timeouts                        0
Cache read errors                     0
Forced recaches                       0
Cache write errors                    0
Compilation failures                  0
Cache errors                          0
Non-cacheable compilations            0
Non-cacheable calls                   0
Non-compilation calls                 0
Unsupported compiler calls            0
Average cache write               0.012 s
Average cache read miss           2.345 s
Average cache read hit            0.008 s
Cache location                    Local disk: "/home/user/.cache/sccache"
Cache size                        1.2 GiB
Max cache size                    10.0 GiB
```

**Interpreting the output**:
- **Compile requests**: Total compilation requests received
- **Cache hits**: Requests served from cache (fast)
- **Cache misses**: Requests that required actual compilation (slow)
- **Hit rate**: `(Cache hits / Compile requests) × 100`
- **Cache size**: Current disk usage
- **Max cache size**: Configured limit

**Good cache performance**:
- Cache hit rate: 70-90% for incremental builds
- Cache hit rate: 20-50% for clean builds with dependencies
- Average cache read hit: <0.1s
- Average cache read miss: 1-5s (depends on compilation time)

#### `make sccache-clean`

**Purpose**: Stop the sccache server and preserve cache statistics.

**Command**: `sccache --stop-server`

**When to use**:
- Freeing memory (sccache server runs in background)
- Troubleshooting cache corruption
- Before major system changes

**Example**:
```bash
$ make sccache-clean
sccache server stopped
Note: Global cache directory preserved at ~/.cache/sccache/testcase-manager
To manually remove cache: rm -rf ~/.cache/sccache/testcase-manager
```

**Note**: This stops the server but preserves the global cache directory. The cache is intentionally preserved to benefit all worktrees.

**To manually delete cached files** (if needed):

```bash
# This project's global cache
rm -rf ~/.cache/sccache/testcase-manager

# Or all sccache caches
rm -rf ~/.cache/sccache/*          # Linux
rm -rf ~/Library/Caches/Mozilla.sccache/*  # macOS
```

#### `make clean`

**Purpose**: Clean build artifacts and optionally sccache.

**Command**: Runs `make coverage-clean` and `make sccache-clean` if sccache is installed

**When to use**:
- Full project cleanup
- Before rebuilding from scratch
- Troubleshooting build issues

**Example**:
```bash
$ make clean
sccache cache cleared
```

**Note**: The main `clean` target automatically runs `sccache-clean` if sccache is detected in your PATH.

### Integration with Build Commands

sccache works transparently with all cargo commands when `RUSTC_WRAPPER=sccache` is set:

```bash
# Normal cargo commands automatically use sccache
cargo build
cargo build --release
cargo test
cargo clippy

# Makefile targets automatically use sccache
make build
make test
make clippy
make coverage
```

No special invocation needed - sccache intercepts rustc calls automatically.

## Cache Management

Effective cache management ensures optimal performance and disk usage.

### Viewing Cache Statistics

Check cache effectiveness:

```bash
make sccache-stats
```

or

```bash
sccache --show-stats
```

### Clearing Cache Statistics

Reset statistics counters without deleting cached files:

```bash
sccache --zero-stats
```

After running this command, all counters reset to 0, but cached compilation results remain available.

### Stopping the sccache Server

The sccache server runs in the background to serve cache requests. To stop it:

```bash
make sccache-clean
```

or

```bash
sccache --stop-server
```

**When to stop the server**:
- Freeing system memory
- Before system shutdown/restart
- When troubleshooting issues
- After completing development work

**Note**: The next cargo build will automatically restart the server.

### Clearing Cached Files

To delete all cached compilation results:

```bash
# This project's global cache (recommended for this project)
rm -rf ~/.cache/sccache/testcase-manager

# All sccache caches (if needed)
rm -rf ~/.cache/sccache/*          # Linux
rm -rf ~/Library/Caches/Mozilla.sccache/*  # macOS

# Or use sccache itself to find location
sccache --stop-server
rm -rf $(sccache --show-cache-location)
```

**When to clear cached files**:
- Running out of disk space
- Suspected cache corruption
- After major Rust version upgrade
- Testing clean build performance

**Warning**: Clearing the global cache affects all worktrees. The next build in any worktree will be slow as the cache repopulates.

**Worktree Consideration**: Since this project uses a global cache shared across worktrees, deleting the cache will affect all worktrees. Consider the impact before clearing.

### Cache Maintenance Best Practices

1. **Monitor cache size**: Check `sccache --show-stats` periodically
2. **Adjust cache size**: Set `SCCACHE_CACHE_SIZE` based on available disk space
3. **Clean old cache**: sccache uses LRU eviction automatically
4. **Check hit rate**: Aim for >70% hit rate on incremental builds
5. **Restart periodically**: Stop server weekly to free memory

### Cache Size Recommendations

| Project Size | Dependencies | Recommended Cache Size |
|--------------|--------------|------------------------|
| Small | <50 crates | 5-10GB |
| Medium | 50-150 crates | 10-20GB |
| Large | 150-500 crates | 20-50GB |
| Enterprise | 500+ crates | 50-100GB |

This project is medium-sized, so **10-20GB** is recommended.

## Performance Benefits

sccache can dramatically reduce compilation times, especially for incremental builds and CI/CD pipelines.

### Expected Speedup Metrics

#### Local Development (Incremental Builds)

Typical speedup when changing a few files:

| Build Type | Without sccache | With sccache | Speedup |
|------------|----------------|--------------|---------|
| Clean build | 120s | 120s | 1x (first run) |
| Rebuild (no changes) | 12s | 0.5s | 24x |
| Rebuild (1 file changed) | 18s | 2s | 9x |
| Rebuild (5 files changed) | 35s | 6s | 5.8x |
| Rebuild (deps updated) | 80s | 15s | 5.3x |

**Key insight**: Greatest benefit for rebuilds with minimal changes.

#### Local Development (Clean Builds)

Speedup when building from scratch after initial cache population:

| Scenario | Without sccache | With sccache | Speedup |
|----------|----------------|--------------|---------|
| First clean build | 120s | 120s | 1x (populates cache) |
| Second clean build | 120s | 25s | 4.8x |
| Third clean build | 120s | 22s | 5.5x |

**Key insight**: Repeated clean builds benefit significantly (e.g., switching branches).

#### CI/CD Pipelines

With shared cache (Redis/S3):

| Pipeline Stage | Without sccache | With sccache | Speedup |
|----------------|----------------|--------------|---------|
| First run | 180s | 180s | 1x (populates cache) |
| Subsequent runs | 180s | 30s | 6x |
| Dependency update | 180s | 45s | 4x |
| Clean rebuild | 180s | 35s | 5.1x |

**Key insight**: Dramatic speedup for repeated CI/CD runs.

#### Team Collaboration

With shared distributed cache:

| Scenario | Without sccache | With sccache | Speedup |
|----------|----------------|--------------|---------|
| Team member 1 builds | 120s | 120s | 1x |
| Team member 2 builds (same code) | 120s | 25s | 4.8x |
| Team member 3 builds (same code) | 120s | 23s | 5.2x |

**Key insight**: First person to build populates cache for entire team.

### Real-World Performance Examples

#### Example 1: Daily Development Workflow

```bash
# Monday morning: Clean build
$ time cargo build
real    2m15s
user    18m30s
sys     1m20s

# Enable sccache and rebuild
$ export RUSTC_WRAPPER=sccache
$ cargo clean
$ time cargo build
real    2m18s    # First build, populates cache
user    19m5s
sys     1m25s

# Make a small code change
$ time cargo build
real    0m3s     # 45x faster!
user    0m8s
sys     0m2s

# Switch to different branch and back
$ git checkout feature-branch
$ time cargo build
real    0m25s    # Cache hit on unchanged dependencies
user    2m10s
sys     0m15s
```

#### Example 2: CI/CD Pipeline

```yaml
# Without sccache: 3 minutes per run
# With sccache (shared S3 cache): 30-45 seconds per run
# Savings: 2.5 minutes × 50 builds/day = 125 minutes/day saved
```

#### Example 3: Test Execution

```bash
# Unit tests
$ time cargo test
# Without sccache: 45s (recompile + test)
# With sccache: 5s (cache hit + test)
# Speedup: 9x

# Coverage tests
$ time make coverage
# Without sccache: 180s
# With sccache: 35s
# Speedup: 5.1x
```

### Factors Affecting Performance

**Positive factors** (better speedup):
- Large number of dependencies
- Frequent small changes
- Repeated clean builds
- Multiple developers sharing cache
- CI/CD with many builds

**Negative factors** (less speedup):
- Very few dependencies
- Large code changes
- Different compiler flags between builds
- Cache misses due to platform differences

### Measuring Your Performance

To measure sccache effectiveness in your workflow:

1. **Zero statistics**:
   ```bash
   sccache --zero-stats
   ```

2. **Run your typical workflow** (build, test, etc.)

3. **Check statistics**:
   ```bash
   sccache --show-stats
   ```

4. **Calculate hit rate**:
   ```
   Hit rate = (Cache hits / Compile requests) × 100%
   ```

5. **Target metrics**:
   - Hit rate: >70% for incremental builds
   - Hit rate: >40% for clean builds with cached deps
   - Average cache read hit: <0.1s

## CI/CD Integration

sccache can be integrated into CI/CD pipelines to speed up builds across multiple runs.

### Local Disk Cache (Simple)

The simplest approach uses local disk caching on CI runners.

**Local Development vs CI**:
- **Local Development**: Uses global cache at `~/.cache/sccache/testcase-manager` (configured in `.cargo/config.toml`)
- **CI/CD**: Uses project directory cache at `$CI_PROJECT_DIR/.sccache` (configured via environment variable override)

#### GitLab CI

This project's GitLab CI configuration (`.gitlab-ci.yml`):

```yaml
variables:
  RUSTC_WRAPPER: sccache
  # Override .cargo/config.toml for CI compatibility
  SCCACHE_DIR: $CI_PROJECT_DIR/.sccache
  SCCACHE_CACHE_SIZE: 5G

cache:
  key: sccache-$CI_COMMIT_REF_SLUG
  paths:
    - .sccache/

before_script:
  - ./scripts/install-sccache.sh --ci
  - export PATH="$HOME/.cargo/bin:$PATH"

build:
  stage: build
  script:
    - cargo build --release
    - sccache --show-stats
```

**Benefits**:
- Simple setup
- No external dependencies
- Works immediately
- CI environment variable overrides local global cache setting

**Limitations**:
- Cache not shared across jobs/branches
- Limited by runner disk space

#### GitHub Actions

> **Do not use this section for the GitHub Actions workflow.** sccache was measured slower than
> the existing cargo-cache setup there. See [GITHUB_CI_CACHING.md](GITHUB_CI_CACHING.md) for the
> measurements and the rule. The example below is kept for reference only.

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Cache sccache
        uses: actions/cache@v3
        with:
          path: ~/.cache/sccache
          key: ${{ runner.os }}-sccache-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-sccache-
      
      - name: Install sccache
        run: |
          ./scripts/install-sccache.sh --ci
          echo "$HOME/.cargo/bin" >> $GITHUB_PATH
      
      - name: Build
        env:
          RUSTC_WRAPPER: sccache
          SCCACHE_CACHE_SIZE: 2G
        run: |
          cargo build --release
          sccache --show-stats
```

**Benefits**:
- GitHub-native caching
- Shared across workflow runs
- Automatic cache eviction

**Limitations**:
- 10GB cache limit per repository
- Not shared across forks

### Distributed Cache (Advanced)

For better performance, use distributed caching with Redis, S3, or GCS.

#### Redis Cache

**Setup Redis server** (one-time):

```bash
# Docker
docker run -d -p 6379:6379 redis:alpine

# Or use managed Redis (AWS ElastiCache, etc.)
```

**Configure CI** (GitLab CI example):

```yaml
variables:
  RUSTC_WRAPPER: sccache
  SCCACHE_REDIS: redis://redis-server:6379

build:
  stage: build
  before_script:
    - ./scripts/install-sccache.sh --ci
  script:
    - cargo build --release
    - sccache --show-stats
```

**Benefits**:
- Shared across all jobs and runners
- Fast access (in-memory)
- No disk space concerns

**Limitations**:
- Requires Redis infrastructure
- Cache is volatile (lost on restart)

#### S3 Cache (AWS)

**Configure AWS credentials** (environment or IAM role):

```bash
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
```

**Configure CI** (GitLab CI example):

```yaml
variables:
  RUSTC_WRAPPER: sccache
  SCCACHE_BUCKET: my-rust-cache-bucket
  SCCACHE_REGION: us-east-1
  AWS_ACCESS_KEY_ID: $AWS_KEY_ID
  AWS_SECRET_ACCESS_KEY: $AWS_SECRET_KEY

build:
  stage: build
  before_script:
    - ./scripts/install-sccache.sh --ci
  script:
    - cargo build --release
    - sccache --show-stats
```

**Benefits**:
- Persistent cache
- Shared across entire organization
- Scalable and reliable
- Works with S3-compatible services (MinIO, DigitalOcean Spaces, etc.)

**Limitations**:
- Requires AWS account and setup
- Network latency for cache access
- Storage costs

#### GCS Cache (Google Cloud)

**Configure GCS credentials**:

```bash
export GCS_BUCKET=my-rust-cache-bucket
export GCS_KEY_PATH=/path/to/service-account-key.json
```

**Configure CI** (GitLab CI example):

```yaml
variables:
  RUSTC_WRAPPER: sccache
  SCCACHE_GCS_BUCKET: my-rust-cache-bucket
  SCCACHE_GCS_KEY_PATH: /builds/project/.gcs-key.json

build:
  stage: build
  before_script:
    - echo "$GCS_SERVICE_ACCOUNT_KEY" > .gcs-key.json
    - ./scripts/install-sccache.sh --ci
  script:
    - cargo build --release
    - sccache --show-stats
```

**Benefits**:
- Persistent cache
- Shared across organization
- Google Cloud integration

**Limitations**:
- Requires GCP account and setup
- Network latency
- Storage costs

### CI/CD Best Practices

1. **Install sccache in CI**: Use `scripts/install-sccache.sh --ci`
2. **Show statistics**: Run `sccache --show-stats` after builds
3. **Monitor cache size**: Set appropriate `SCCACHE_CACHE_SIZE`
4. **Use distributed cache**: For multi-runner setups
5. **Set cache keys**: Use `Cargo.lock` hash for cache invalidation
6. **Clean cache periodically**: Reset distributed cache monthly

### Troubleshooting CI/CD Issues

**Issue**: sccache not found in CI

**Solution**:
```yaml
before_script:
  - ./scripts/install-sccache.sh --ci
  - export PATH="$HOME/.cargo/bin:$PATH"  # Ensure sccache is in PATH
```

**Issue**: No cache hits in CI

**Solution**:
- Check that `RUSTC_WRAPPER=sccache` is set
- Verify cache backend configuration
- Check network connectivity to distributed cache
- Review cache key strategy

**Issue**: Slow distributed cache

**Solution**:
- Use Redis for faster cache access
- Use S3 in the same region as CI runners
- Check network bandwidth
- Consider local disk cache for faster iteration

## Troubleshooting

Common issues and solutions when using sccache.

### Issue: sccache command not found

**Symptom**:
```bash
$ sccache --version
bash: sccache: command not found
```

**Cause**: sccache is not installed or not in PATH.

**Solution**:

1. Install sccache:
   ```bash
   make install-sccache
   ```

2. Verify installation:
   ```bash
   sccache --version
   ```

3. If installed but not in PATH:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

4. Add to shell profile:
   ```bash
   echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc
   ```

### Issue: sccache not caching anything (0% hit rate)

**Symptom**:
```bash
$ sccache --show-stats
Cache hits                            0
Cache misses                        456
```

**Cause**: `RUSTC_WRAPPER` not set or sccache not properly configured.

**Solution**:

1. Set `RUSTC_WRAPPER`:
   ```bash
   export RUSTC_WRAPPER=sccache
   ```

2. Verify it's set:
   ```bash
   echo $RUSTC_WRAPPER
   # Should output: sccache
   ```

3. Add to shell profile:
   ```bash
   echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
   source ~/.bashrc
   ```

4. Rebuild and check:
   ```bash
   cargo clean
   cargo build
   sccache --show-stats
   ```

### Issue: Low cache hit rate (<30%)

**Symptom**:
```bash
$ sccache --show-stats
Compile requests                    500
Cache hits                          120  # 24% hit rate
Cache misses                        380
```

**Cause**: Frequent code changes, different build configurations, or cache misses due to environment.

**Possible solutions**:

1. **Check if cache is being cleared**:
   - Cache directory might be deleted between builds
   - CI cache might not be persisting

2. **Review build configuration**:
   - Different compiler flags cause cache misses
   - Ensure consistent RUSTFLAGS across builds

3. **Increase cache size**:
   ```bash
   export SCCACHE_CACHE_SIZE=20G
   ```

4. **Check cache location**:
   ```bash
   sccache --show-stats | grep "Cache location"
   ls -lh ~/.cache/sccache/  # Verify cache exists
   ```

5. **Wait for cache to populate**:
   - First few builds have low hit rate (normal)
   - Hit rate improves over time

### Issue: sccache server errors

**Symptom**:
```
error: failed to execute sccache
error: could not connect to server
```

**Cause**: sccache server crashed or is not running.

**Solution**:

1. Restart sccache server:
   ```bash
   sccache --stop-server
   # Next cargo build will restart it
   cargo build
   ```

2. Check server status:
   ```bash
   sccache --show-stats  # This starts server if needed
   ```

3. Review server logs:
   ```bash
   # Enable debug logging
   export SCCACHE_LOG=debug
   cargo build
   ```

4. Check for port conflicts:
   - sccache uses a local port for IPC
   - Ensure no other process is blocking it

### Issue: Cache filling up disk

**Symptom**:
```bash
$ sccache --show-stats
Cache size                        9.8 GiB
Max cache size                   10.0 GiB
```

**Cause**: Cache approaching or exceeding size limit.

**Solution**:

1. **Increase cache size** (if disk space available):
   ```bash
   export SCCACHE_CACHE_SIZE=20G
   ```

2. **Clean old cache entries** (automatic LRU):
   - sccache automatically evicts old entries
   - No action needed unless cache is corrupted

3. **Manually clear cache**:
   ```bash
   sccache --stop-server
   rm -rf ~/.cache/sccache/*
   ```

4. **Move cache to larger disk**:
   ```bash
   export SCCACHE_DIR=/mnt/large-disk/sccache
   ```

### Issue: Distributed cache not working

**Symptom**:
```bash
$ sccache --show-stats
Cache location                    Local disk
# Expected: Redis or S3
```

**Cause**: Distributed cache not configured or credentials missing.

**Solution for Redis**:

1. Verify Redis connection:
   ```bash
   redis-cli -h redis-server -p 6379 ping
   # Should output: PONG
   ```

2. Set environment variable:
   ```bash
   export SCCACHE_REDIS=redis://redis-server:6379
   ```

3. Restart sccache:
   ```bash
   sccache --stop-server
   cargo build
   sccache --show-stats
   ```

**Solution for S3**:

1. Verify AWS credentials:
   ```bash
   aws s3 ls s3://my-rust-cache-bucket
   ```

2. Set environment variables:
   ```bash
   export SCCACHE_BUCKET=my-rust-cache-bucket
   export SCCACHE_REGION=us-east-1
   export AWS_ACCESS_KEY_ID=your_key
   export AWS_SECRET_ACCESS_KEY=your_secret
   ```

3. Restart sccache:
   ```bash
   sccache --stop-server
   cargo build
   sccache --show-stats
   ```

### Issue: Compilation failures with sccache (exit status 254)

**Symptom**:
```
error occurred in cc-rs: command did not execute successfully 
(status code exit status: 254): env ... "sccache" "cc" ...
```

**Cause**: sccache may have issues with:
- Native C dependencies (libssh2-sys, openssl-sys, libgit2-sys, etc.)
- Cross-compilation scenarios
- Certain cc-rs build configurations
- Corrupted cache entries

**Solution 1 - Disable sccache temporarily**:
   ```bash
   # Use the disable script
   source ./scripts/disable-sccache.sh
   
   # Or manually
   unset RUSTC_WRAPPER
   
   # Clean and rebuild
   cargo clean
   cargo build
   ```

**Solution 2 - Stop sccache server and retry**:
   ```bash
   sccache --stop-server
   cargo clean
   cargo build
   ```

**Solution 3 - Clear sccache cache**:
   ```bash
   sccache --stop-server
   rm -rf ~/.cache/sccache/testcase-manager
   cargo clean
   cargo build
   ```

**Solution 4 - Check sccache logs**:
   ```bash
   export SCCACHE_LOG=debug
   cargo build 2>&1 | grep -i error
   ```

**Solution 5 - Report bug** to sccache if issue persists:
   - https://github.com/mozilla/sccache/issues

**Working Without sccache**:
If sccache consistently causes issues, you can disable it permanently:
```bash
# Remove from shell profile
# Edit ~/.bashrc or ~/.zshrc and remove: export RUSTC_WRAPPER=sccache

# Or keep it disabled for specific projects
unset RUSTC_WRAPPER
```

### Issue: Permission denied errors

**Symptom**:
```
error: Permission denied (os error 13)
error: could not write to cache
```

**Cause**: Cache directory has incorrect permissions.

**Solution**:

1. Check cache directory permissions:
   ```bash
   ls -ld ~/.cache/sccache
   ```

2. Fix permissions:
   ```bash
   chmod -R u+rw ~/.cache/sccache
   ```

3. Recreate cache directory:
   ```bash
   sccache --stop-server
   rm -rf ~/.cache/sccache
   mkdir -p ~/.cache/sccache
   cargo build
   ```

### Debug Mode

For detailed troubleshooting, enable debug logging:

```bash
export SCCACHE_LOG=debug
export RUST_LOG=debug
cargo build 2>&1 | tee build.log
```

Review `build.log` for detailed error messages and cache operations.

## Advanced Configuration

### Multiple Cache Backends

sccache supports fallback cache backends. If the primary cache fails, it falls back to local disk:

```bash
# Try Redis first, fall back to local disk
export SCCACHE_REDIS=redis://cache-server:6379
export SCCACHE_DIR=~/.cache/sccache
```

### Custom Compiler Detection

For custom compilers or cross-compilation:

```bash
# Disable compiler detection for specific tools
export SCCACHE_DIRECT=false
```

### Cache Read-Only Mode

Useful in CI for jobs that should only read from cache:

```bash
export SCCACHE_READONLY=true
```

This prevents jobs from populating cache (use for test jobs, linting, etc.).

### Recache Specific Files

Force recompilation of specific files even if cached:

```bash
# This is handled by cargo - just modify the file
touch src/main.rs
cargo build  # Will recompile main.rs and dependents
```

### Platform-Specific Configuration

#### macOS

```bash
# Use macOS-specific cache location
export SCCACHE_DIR=~/Library/Caches/Mozilla.sccache

# For Apple Silicon, ensure correct architecture
rustc --version --verbose | grep host
```

#### Linux

```bash
# Standard XDG cache location
export SCCACHE_DIR=~/.cache/sccache

# For containers, use mounted volume
export SCCACHE_DIR=/cache/sccache
```

#### Windows

```powershell
# PowerShell
$env:SCCACHE_DIR = "$env:LOCALAPPDATA\Mozilla\sccache\cache"
$env:RUSTC_WRAPPER = "sccache"
```

### Integration with Other Tools

#### rust-analyzer

rust-analyzer can use sccache for builds:

```json
{
  "rust-analyzer.cargo.extraEnv": {
    "RUSTC_WRAPPER": "sccache"
  }
}
```

#### cargo-watch

Automatically use sccache with cargo-watch:

```bash
export RUSTC_WRAPPER=sccache
cargo watch -x build
```

## Additional Resources

- **Project Documentation**:
  - `AGENTS.md`: Build commands and requirements
  - `docs/COVERAGE.md`: Coverage testing with sccache
  - `scripts/install-sccache.sh`: Installation script

- **External Resources**:
  - [sccache GitHub](https://github.com/mozilla/sccache)
  - [sccache Documentation](https://github.com/mozilla/sccache/blob/main/docs/README.md)
  - [Rust Performance Book](https://nnethercote.github.io/perf-book/)
  - [cargo Documentation](https://doc.rust-lang.org/cargo/)

## Best Practices Summary

1. **Always set RUSTC_WRAPPER**: Add to your shell profile
2. **Monitor cache stats**: Run `make sccache-stats` regularly
3. **Set appropriate cache size**: 10-20GB for this project
4. **Use distributed cache in CI**: Redis or S3 for best performance
5. **Clean cache periodically**: Monthly or when issues arise
6. **Check hit rate**: Aim for >70% on incremental builds
7. **Enable in CI/CD**: Massive speedup for pipeline builds
8. **Share cache with team**: Use Redis/S3 for collaborative caching
