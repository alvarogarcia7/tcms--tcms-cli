# Stage 1: builder - Build the actual application
FROM rust:1.95-bookworm AS builder

WORKDIR /app

RUN apt update && \
    apt install -y sccache && \
    rm -rf /var/lib/apt/lists/*

# Set sccache environment variables
# Use a global cache directory that can be shared across builds via Docker cache mount
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/root/.cache/sccache/testcase-manager

# Create cache directory
RUN mkdir -p /root/.cache/sccache/testcase-manager

# Install coverage tools for CI/CD
RUN rustup component add llvm-tools-preview && \
    cargo install cargo-llvm-cov

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy workspace crates structure
COPY crates ./crates

# Copy scripts directory early so include_str! macros can find the files
COPY scripts ./scripts

# Copy schemas directory early so tests can find schema files
COPY schemas ./schemas

COPY crates ./crates

## Create dummy src files in crates to build dependencies
#RUN mkdir -p crates/bash-eval/src && \
#    echo "pub fn dummy() {}" > crates/bash-eval/src/lib.rs && \
#    mkdir -p crates/testcase-manager/src/bin crates/testcase-manager/examples && \
#    echo "fn main() {}" > crates/testcase-manager/src/lib.rs && \
#    echo "fn main() {}" > crates/testcase-manager/src/main_editor.rs && \
#    for bin in validate-yaml validate-json test-run-manager test-verify test-executor json-escape verifier test-orchestrator script-cleanup test-plan-documentation-generator-compat json-to-yaml; do \
#      echo "fn main() {}" > "crates/testcase-manager/src/bin/${bin}.rs"; \
#    done && \
#    for example in tty_fallback_demo test_verify_demo test_verify_integration junit_export_example; do \
#      echo "fn main() {}" > "crates/testcase-manager/examples/${example}.rs"; \
#    done

RUN mkdir -p ".cargo"; cargo vendor --locked > .cargo/config.toml

# Build the application against cached dependencies
RUN cargo build --all --all-features --release && \
    cargo build --all --all-features  # debug build to populate debug cache

# Copy source code (now in crates directory)
COPY crates ./crates
COPY data ./data
COPY testcases ./testcases

# Build the application against cached dependencies
RUN cargo build --all --all-features --release && \
    cargo build --all --all-features  # debug build to populate debug cache

WORKDIR /app


# Install runtime dependencies: git, inotify-tools for watch mode, and make
RUN apt-get update && \
    apt-get install -y \
      git \
      inotify-tools \
      expect \
      iputils-ping \
      shellcheck \
      make && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Makefile for convenient commands
COPY Makefile ./Makefile

# Build release binaries first
RUN cargo build --all --all-features --release && \
    for bin in target/release/*; do \
      if [ -f "$bin" ] && [ -x "$bin" ]; then \
        cp "$bin" /usr/local/bin/ && chmod +x "/usr/local/bin/$(basename $bin)"; \
      fi; \
    done

# Verify binaries were installed correctly
RUN \
for bin in target/release/*; do \
  if [ -f "$bin" ] && [ -x "$bin" ]; then \
    bin_name="$(basename $bin)"; \
    ls -lah "/usr/local/bin/$bin_name" > /dev/null || (echo "Binary $bin_name not found in /usr/local/bin" && exit 1); \
  fi; \
done

# Run tests to ensure everything compiles and passes
# Run unit tests only (skip integration tests that require binaries in specific PATH)
RUN cargo test --workspace --lib --all-features && \
    cargo test --workspace --doc --all-features

# Make scripts executable
RUN chmod +x scripts/*.sh && \
    find scripts -type f -name "*.sh" -exec chmod +x {} \;

# Create a helper script for easy watch mode usage
#RUN cat > /usr/local/bin/watch-yaml << 'WATCHEOF'
##!/bin/bash
## Helper script to start watch mode easily
#cd /app
#exec ./scripts/watch-yaml-files.sh "$@"
#WATCHEOF

#RUN chmod +x /usr/local/bin/watch-yaml

# Create README guide at ~/README.md
COPY README_INSTALL_AUTOMATED.md /app
COPY README_INSTALL.md /app
RUN cat README_INSTALL.md README_INSTALL_AUTOMATED.md >> README_INSTALL_2.md && mv README_INSTALL_2.md README_INSTALL.md && cp README_INSTALL.md /app/README.md && cp README_INSTALL.md /root/README.md

# Install uv package manager
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# Copy Python project files
COPY pyproject.toml uv.lock ./

# First, sync dependencies. Pin to Python 3.14: requires-python is ">=3.14",
# so an unpinned sync downloads the latest CPython (3.15), which exceeds the
# maximum version supported by PyO3 0.26 when building native deps such as
# watchfiles (via strictdoc -> uvicorn[standard]).
RUN uv sync --python 3.14

# Then install Python 3.14 and set as default
RUN uv python install --default 3.14

# Verify Python 3.14 is available and working
RUN uv run python3.14 --version

# Create symlinks to make python3.14 available globally
RUN ln -sf $(uv python find 3.14) /usr/local/bin/python3.14 && \
    ln -sf /usr/local/bin/python3.14 /usr/local/bin/python3 && \
    ln -sf /usr/local/bin/python3.14 /usr/local/bin/python

# Verify global Python 3.14 is available
RUN python3.14 --version && python3 --version && python --version

# Re-sync to ensure all dependencies are installed with Python 3.14
RUN uv sync --python 3.14

# Set default command
CMD ["tcm"]
