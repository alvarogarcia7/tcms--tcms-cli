# AGENTS.md

## Workspace Structure

This project uses a **Cargo workspace** to organize multiple related crates under the `crates/` directory. This provides better modularity, shared dependencies, and parallel compilation.

### Directory Organization

```
.
├── Cargo.toml                 # Workspace root configuration
├── crates/                    # All Rust crates organized here
│   ├── bash-eval/             # Bash evaluation library
│   ├── editor/                # Interactive editor
│   ├── json-escape/           # JSON escaping utility
│   ├── json-to-yaml/          # JSON to YAML converter
│   ├── script-cleanup/        # Script cleanup tool
│   ├── testcase-cli/          # CLI utilities
│   ├── testcase-common/       # Shared common code
│   ├── testcase-execution/    # Test execution logic
│   ├── testcase-git/          # Git integration
│   ├── testcase-manager/      # Main test case manager
│   ├── testcase-models/       # Data models
│   ├── testcase-orchestration/# Test orchestration
│   ├── testcase-storage/      # Storage and persistence
│   ├── testcase-ui/           # UI components
│   ├── testcase-validation/   # Validation logic
│   ├── testcase-verification/ # Verification logic
│   ├── test-executor/         # Test execution engine
│   ├── test-orchestrator/     # Test orchestrator
│   ├── test-run-manager/      # Test run management
│   ├── test-verify/           # Test verification tool
│   ├── tpdg-compat/           # TPDG compatibility layer
│   ├── validate-json/         # JSON validator
│   ├── validate-yaml/         # YAML validator
│   └── verifier/              # Verifier binary
├── testcases/                 # Test case YAML files
├── schemas/                   # JSON schemas
├── scripts/                   # Build and utility scripts
├── tests/                     # Integration tests
├── data/                      # Sample data
└── examples/                  # Example code
```

### Workspace Benefits

- **Shared Dependencies**: Common dependencies defined once in workspace root `[workspace.dependencies]`
- **Consistent Versions**: All crates use identical dependency versions
- **Parallel Builds**: Cargo can build multiple crates simultaneously
- **Better Organization**: Clear separation between binaries, libraries, and shared code
- **Easier Maintenance**: Dependency updates managed centrally
- **Incremental Compilation**: Only changed crates and their dependents rebuild

## Data Flow and Architecture

Understanding the data flow and architecture is essential for navigating the codebase and understanding how different components interact.

### Documentation References

- **Data Flow Diagram**: See [docs/data_flow/2026-01-20_diagram_steps.mermaid](docs/data_flow/2026-01-20_diagram_steps.mermaid) for a detailed Mermaid sequence diagram showing the complete test execution workflow from test case generation through verification and documentation.
- **Architecture Guide**: See [crates/README.md](crates/README.md) for comprehensive documentation on crate dependencies, layer architecture, and design principles.

### Five-Layer Architecture

The workspace follows a **strict bottom-up layered architecture** where higher-level crates depend on lower-level crates, but never the reverse. This prevents circular dependencies and ensures clean separation of concerns.

```
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 5: BINARY CRATES                                          │
│  (Autonomous microservices - independently deployable)           │
│                                                                   │
│  editor, test-executor, test-orchestrator, test-verify,          │
│  test-run-manager, verifier, validate-yaml, validate-json,       │
│  script-cleanup, json-escape, json-to-yaml, tpdg-compat          │
└───────────────────────────┬───────────────────────────────────────┘
                            │ depends on
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 4: ORCHESTRATION                                          │
│  (High-level coordination and workflow management)               │
│                                                                   │
│  testcase-orchestration, testcase-manager                        │
└───────────────────────────┬───────────────────────────────────────┘
                            │ depends on
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 3: CORE FUNCTIONALITY                                     │
│  (Domain-specific logic)                                         │
│                                                                   │
│  testcase-validation, testcase-execution, testcase-verification, │
│  testcase-storage, testcase-ui, testcase-git, testcase-cli       │
└───────────────────────────┬───────────────────────────────────────┘
                            │ depends on
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 2: FOUNDATION                                             │
│  (Shared utilities and data structures)                          │
│                                                                   │
│  testcase-common                                                 │
└───────────────────────────┬───────────────────────────────────────┘
                            │ depends on
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 1: BASE                                                   │
│  (Core data models and primitive utilities)                      │
│                                                                   │
│  testcase-models, bash-eval                                      │
└─────────────────────────────────────────────────────────────────┘
```

**Layer Responsibilities**:

1. **Base (Layer 1)**: Core data structures (`TestCase`, `TestSequence`, `TestStep`, `Verification`, `Hook`) and bash expression evaluation
2. **Foundation (Layer 2)**: Shared utilities (file I/O, YAML/JSON parsing, configuration management, path resolution)
3. **Core Functionality (Layer 3)**: Domain-specific logic (validation, execution, verification, storage, UI components, Git integration)
4. **Orchestration (Layer 4)**: High-level workflows that coordinate multiple core functionality crates
5. **Binary (Layer 5)**: Self-contained microservices with CLI interfaces, each performing one well-defined task

### Common Data Types and Flow

The following table shows the primary data types that flow between crates:

| Data Type | Produced By | Consumed By | Format | Purpose |
|-----------|-------------|-------------|--------|---------|
| **TestCase** | testcase-models, testcase-storage | All crates (directly or indirectly) | Rust struct | Core test case definition with sequences, steps, verifications |
| **Test Case YAML** | User (editor), testcase-storage | testcase-validation, testcase-storage, testcase-execution | YAML file | Declarative test case definition |
| **Bash Script** | testcase-execution, test-executor | Bash interpreter, test-orchestrator | .sh file | Executable test script generated from YAML |
| **Execution Log** | testcase-execution, Bash scripts | testcase-verification, verifier | JSON file | Captured test execution output with timestamps |
| **Verification Result** | testcase-verification, verifier | testcase-orchestration, test-verify, Reports | YAML/JSON | Pass/fail status with detailed comparison results |
| **Container YAML** | testcase-verification, verifier | test-plan-documentation-generator, tpdg-compat | YAML file | Structured verification results for documentation |
| **Documentation** | test-plan-documentation-generator | User, CI/CD systems | AsciiDoc/Markdown/HTML | Human-readable test reports and documentation |
| **Configuration** | testcase-common | testcase-execution, testcase-validation | YAML/TOML/.env | Environment variables, settings, metadata |
| **Schema** | Project (schemas/ directory) | testcase-validation, validate-yaml, validate-json | JSON Schema | Validation rules for YAML/JSON files |

**Key Data Flow Patterns**:

- **YAML → TestCase**: `testcase-storage` deserializes YAML files into `TestCase` structs
- **TestCase → Bash Script**: `testcase-execution` generates executable scripts from test case definitions
- **Execution → Log**: Bash scripts output JSON execution logs
- **Log + TestCase → Verification**: `testcase-verification` compares actual vs. expected outputs
- **Verification → Container**: Results are serialized into container YAML format for documentation
- **Container → Documentation**: External `test-plan-documentation-generator` renders reports

### Navigation Tips for Agents

When working with the codebase:

1. **Start with the layer diagram**: Identify which layer your change affects
2. **Check dependencies**: Use `cargo tree -p <crate> --invert` to see what depends on a crate
3. **Follow data flow**: Trace data types from their source (`testcase-models`) through transformations
4. **Read architecture docs**: Consult [crates/README.md](crates/README.md) for detailed crate descriptions
5. **View sequence diagram**: See [docs/data_flow/2026-01-20_diagram_steps.mermaid](docs/data_flow/2026-01-20_diagram_steps.mermaid) for the complete test execution workflow
6. **Respect layer boundaries**: Never add dependencies from lower layers to higher layers
7. **Keep binaries autonomous**: Binary crates should be self-contained microservices

### Build Commands

The project supports both workspace-wide and per-crate build commands:

**Workspace-wide builds:**
```bash
# Build all crates in workspace
make build                    # Debug mode
make build-release            # Release mode

# Equivalent cargo commands
cargo build --workspace       # Debug mode
cargo build --workspace --release
```

**Per-crate builds:**
```bash
# Build specific crate by package name
cargo build -p <crate-name>
cargo build --package <crate-name>

# Build specific binary
cargo build --bin <binary-name>

# Make targets for common crates
make build-validate-yaml      # cargo build -p validate-yaml
make build-verifier           # cargo build -p verifier
make build-test-executor      # cargo build -p test-executor
make build-json-escape        # cargo build -p json-escape
make build-testcase-manager   # cargo build -p testcase-manager
```

See **Per-Crate Development Workflow** section below for detailed per-crate commands.

### Selective Compilation in CI/CD

The workspace structure enables intelligent selective compilation when dependencies change:

**When to Rebuild:**
- **Crate source changes**: Only rebuild the modified crate and its dependents
- **Shared dependency changes**: Rebuild all crates using that dependency
- **Workspace dependency version bump**: Rebuild affected crates only

**CI/CD Optimization Strategies:**

1. **Dependency-Based Rebuilding:**
```bash
# Check which crates depend on a specific crate
cargo tree -p testcase-models --invert

# Build only affected crates after testcase-models changes
cargo build -p testcase-models
cargo build -p testcase-manager  # Depends on testcase-models
cargo build -p test-executor     # Depends on testcase-models
```

2. **Change Detection:**
```yaml
# Example GitLab CI strategy
build:
  script:
    # Detect changed crates
    - CHANGED_CRATES=$(git diff --name-only HEAD~1 | grep "^crates/" | cut -d'/' -f2 | sort -u)
    # Build only changed crates and workspace
    - for crate in $CHANGED_CRATES; do cargo build -p $crate; done
    - cargo build --workspace  # Full workspace build to verify integration
```

3. **Caching Strategy:**
```yaml
# Cache cargo target directory between builds
cache:
  key: ${CI_COMMIT_REF_SLUG}
  paths:
    - target/
    - .cargo/
```

**Common Scenarios:**

| Change Type | What to Rebuild | Command |
|-------------|----------------|---------|
| Single crate source | Changed crate + dependents | `cargo build -p <crate>` |
| Shared code (testcase-common) | All crates | `cargo build --workspace` |
| Workspace dependency version | Affected crates only | Automatic via Cargo |
| New crate added | New crate + workspace | `cargo build --workspace` |
| Integration test changes | No recompilation needed | `cargo test` |

**Build Time Optimization:**
- Use `sccache` for compilation caching: `make install-sccache`
- Leverage parallel builds: `cargo build --workspace -j<N>`
- Build only necessary targets: `cargo build --workspace --tests` (excludes examples)

### Per-Crate Development Workflow

When working on a specific crate, use targeted commands for faster iteration:

**Building:**
```bash
# Build specific crate
cargo build -p testcase-models
cargo build -p verifier --release

# Build with features
cargo build -p testcase-validation --all-features
```

**Testing:**
```bash
# Run tests for specific crate
cargo test -p testcase-models
cargo test -p verifier --all-features

# Run specific test
cargo test -p testcase-validation test_schema_validation
```

**Linting and Formatting:**
```bash
# Lint specific crate
cargo clippy -p testcase-manager -- -D warnings

# Format specific crate
cargo fmt -p testcase-models

# Workspace-wide (recommended before commit)
make lint     # Runs fmt + clippy on all crates
```

**Running Binaries:**
```bash
# Run binary from specific crate
cargo run -p validate-yaml -- --help
cargo run -p test-executor -- generate test.yml
cargo run -p verifier -- -f logs/ --format yaml

# Or use make targets
make run-verifier
make run-test-executor
```

**Checking Dependencies:**
```bash
# View crate dependency tree
cargo tree -p testcase-models

# View reverse dependencies (what depends on this crate)
cargo tree -p testcase-models --invert

# Check for duplicate dependencies
cargo tree --duplicates
```

**Development Cycle Example:**
```bash
# 1. Make changes to testcase-models crate
vim crates/testcase-models/src/lib.rs

# 2. Build and test only that crate
cargo build -p testcase-models
cargo test -p testcase-models

# 3. Build dependent crates to verify integration
cargo build -p testcase-manager
cargo build -p test-executor

# 4. Run full workspace tests before commit
make test

# 5. Lint and format
make lint
```

### Selective Compilation and Testing

The project provides intelligent incremental build and test workflows that detect changes since a base reference (branch or commit) and only build/test affected crates and their dependents. This significantly speeds up local development and CI/CD pipelines.

#### Overview

Three main workflow targets are available:

1. **`test-from`** - Incremental testing for local development (builds + unit tests + E2E tests for affected crates)
2. **`build-from`** - Incremental build-only for affected crates
3. **`test-all`** - Full unconditional clean build and test (comprehensive CI-style testing)
4. **`build-all`** - Full unconditional build of all workspace crates

Additionally, `list-affected-crates` is a debugging target that shows which crates would be affected by your changes.

#### Target Comparison

| Target | When to Use | What It Does | Speed | Comprehensiveness |
|--------|-------------|--------------|-------|-------------------|
| **test-from** | Local development, feature branches | Builds + tests only affected crates | Fast | Targeted |
| **test** | Default testing, pre-commit | Full build + comprehensive unit/E2E tests | Moderate | Comprehensive |
| **test-all** | Clean validation, CI/CD | Full clean build + all tests | Slow | Maximum |
| **build-from** | Quick compilation check | Builds only affected crates | Very Fast | Build-only |
| **build-all** | Full workspace build | Builds all crates unconditionally | Moderate | Build-only |

**Key Differences:**

- **`test-from`** (incremental): Fast, change-aware, skips unaffected crates, ideal for rapid iteration
- **`test`** (default): Comprehensive workspace testing with unit tests, E2E tests, and validation
- **`test-all`** (full clean): Unconditional full rebuild and test, ensures clean slate, catches edge cases

**Recommendation**: Use `test-from` during development for fast feedback, use `test` before committing, and use `test-all` for critical validation or when you suspect cache issues.

#### Incremental Build: `build-from`

Builds only the crates affected by changes since a base reference.

**Usage:**
```bash
# Build crates changed since main branch
make build-from BASE_REF=main

# Build crates changed since develop branch
make build-from BASE_REF=develop

# Build crates changed in last 3 commits
make build-from BASE_REF=HEAD~3

# Build crates changed since specific commit
make build-from BASE_REF=abc123
```

**Default**: If `BASE_REF` is not specified, defaults to `main`.

**Behavior:**
1. Detects all changed files between `BASE_REF` and current state (working directory, staged, committed)
2. Maps changed files to affected crates
3. Identifies all reverse dependencies (crates that depend on changed crates)
4. Builds each affected crate with `cargo build -p <crate>`
5. Skips tests (build-only mode)

**Exit Codes:**
- `0` - All builds succeeded or no changes detected
- `1` - One or more builds failed

**Example Output:**
```
========================================
Incremental Build (from BASE_REF)
========================================
Base reference: main

Building affected crates...

Building: testcase-models
  ✓ Build succeeded: testcase-models

Building: testcase-manager
  ✓ Build succeeded: testcase-manager

Building: test-executor
  ✓ Build succeeded: test-executor

========================================
All builds completed successfully
========================================
```

#### Incremental Test: `test-from`

Tests only the crates affected by changes since a base reference, including unit tests and relevant E2E integration tests.

**Usage:**
```bash
# Test crates changed since main branch
make test-from BASE_REF=main

# Test crates changed since develop branch
make test-from BASE_REF=develop

# Test crates changed in last commit
make test-from BASE_REF=HEAD~1

# Test crates changed since specific commit
make test-from BASE_REF=abc123
```

**Default**: If `BASE_REF` is not specified, defaults to `main`.

**Behavior:**
1. Detects all changed files between `BASE_REF` and current state
2. Maps changed files to affected crates (including reverse dependencies)
3. For each affected crate:
   - **[1/3]** Builds the crate: `cargo build -p <crate>`
   - **[2/3]** Runs unit tests: `cargo test -p <crate>`
   - **[3/3]** Collects relevant E2E integration tests based on binary mapping
4. Runs all collected E2E tests (deduplicated)
5. Reports aggregated results

**Exit Codes:**
- `0` - All builds and tests passed or no changes detected
- `1` - One or more builds or tests failed

**Crate to E2E Test Mapping:**

The `test-from` target intelligently maps affected crates to their corresponding E2E integration tests:

| Crate | Binaries | E2E Tests |
|-------|----------|-----------|
| `validate-yaml` | validate-yaml | test_validate_yaml_*.sh (5 tests) |
| `test-executor` | test-executor | test_executor_e2e.sh, test_variable_passing_e2e.sh, etc. (4 tests) |
| `test-orchestrator` | test-orchestrator | test_orchestrator_e2e.sh (1 test) |
| `verifier` | verifier | test_verifier_*.sh, run_verifier_and_generate_reports.sh (4 tests) |
| `audit-verifier` | audit-verifier, sign-audit-log, verify-audit-log | audit-verifier integration tests (4 tests) |
| `json-escape` | json-escape | test_json_escape_e2e.sh (1 test) |
| `testcase-manager` | testcase-manager | smoke_test.sh, test_bdd_e2e.sh, etc. (5 tests) |

Library crates (e.g., `testcase-models`, `testcase-common`) have no direct E2E test mapping, but their reverse dependencies will be tested if affected.

**Example Output:**
```
========================================
Incremental Test (from BASE_REF)
========================================
Base reference: main

Testing affected crates...

========================================
Processing crate: testcase-models
========================================

[1/3] Building testcase-models...
  ✓ Build succeeded: testcase-models

[2/3] Running unit tests for testcase-models...
  ✓ Unit tests passed: testcase-models

[3/3] Collecting E2E tests for testcase-models...
  No E2E test mapping for testcase-models (library crate or no tests defined)

========================================
Processing crate: test-executor
========================================

[1/3] Building test-executor...
  ✓ Build succeeded: test-executor

[2/3] Running unit tests for test-executor...
  ✓ Unit tests passed: test-executor

[3/3] Collecting E2E tests for test-executor...
  test-executor binary -> 4 E2E tests

========================================
Running E2E Integration Tests
========================================

Found 4 unique E2E test script(s) to run

Running: crates/testcase-manager/tests/integration/test_executor_e2e.sh
  ✓ E2E test passed: test_executor_e2e.sh

...

========================================
All tests completed successfully
========================================
```

#### Full Clean Build and Test: `build-all`, `test-all`, and `test-e2e-all`

These targets provide unconditional full workspace builds and tests, regardless of changes. They ensure a clean slate and comprehensive validation.

**`build-all`:**
```bash
# Build all workspace crates with all features
make build-all

# Equivalent to:
cargo build --workspace --all-features
```

**`test-all`:**
```bash
# Full clean build and comprehensive test suite
make test-all

# Equivalent to:
cargo build --workspace --all-features
cargo test --workspace --all-features --tests
make test-e2e-all
```

**`test-e2e-all`:**
```bash
# Run all E2E integration tests unconditionally (includes build)
make test-e2e-all

# This runs the complete E2E test suite:
# - All validate-yaml tests
# - All test-executor tests (including manual step tests)
# - All test-orchestrator tests
# - All verifier tests
# - All testcase-manager tests
# - All audit-verifier tests
# - Documentation generation tests
# - Output schema validation
```

**When to Use:**
- **CI/CD pipelines**: Ensure no caching issues or missed dependencies
- **Pre-release validation**: Comprehensive check before tagging releases
- **Clean slate testing**: When you suspect incremental build cache issues
- **Merge validation**: Final check before merging to main/develop

**Note:** `test-all` sets up Python environment automatically (`setup-python-for-test`) before running tests. All E2E tests in `test-e2e-all` are designed to run in non-interactive mode and will not block on user input.

#### Detecting Changes: Examples

**Detect changes since main branch:**
```bash
# See which files changed
make list-changed-files BASE_REF=main

# See which crates are affected (including reverse dependencies)
make list-affected-crates BASE_REF=main
```

**Detect changes since specific commit:**
```bash
# Using commit SHA
make list-affected-crates BASE_REF=abc123def

# Using relative commit reference
make list-affected-crates BASE_REF=HEAD~5
```

**Detect changes in current working directory:**
```bash
# Uncommitted changes only
make list-affected-crates BASE_REF=HEAD
```

#### Debugging: `list-affected-crates`

This target helps you understand which crates will be built/tested by `build-from` or `test-from` without actually building or testing them.

**Usage:**
```bash
# List affected crates compared to main
make list-affected-crates BASE_REF=main

# List affected crates compared to specific commit
make list-affected-crates BASE_REF=abc123
```

**Example Output:**
```
Detecting affected crates compared to main...

Affected crates:
  - testcase-models
  - testcase-manager
  - test-executor
  - verifier
```

**Interpretation:**
- **Direct changes**: Crates where files were modified
- **Reverse dependencies**: Crates that depend on directly changed crates (transitive)
- **Empty output**: No changes detected or changes don't affect any crates

**Use Cases:**
1. **Preview impact**: See what will be built/tested before running `test-from`
2. **Verify detection**: Confirm change detection is working correctly
3. **Debug issues**: Troubleshoot why expected crates aren't being built
4. **Estimate time**: Gauge how long incremental builds/tests will take

#### Change Detection Details

The incremental build system uses `scripts/detect-local-changes.sh` to detect changes:

**Detection Sources:**
1. **Committed changes**: `git diff BASE_REF...HEAD`
2. **Staged changes**: `git diff --cached`
3. **Unstaged changes**: `git diff` (working directory modifications)

**File-to-Crate Mapping:**
- Files under `crates/<crate-name>/` are mapped to `<crate-name>`
- Changes to workspace files (root `Cargo.toml`, `Cargo.lock`) affect all crates
- Changes to non-crate files are ignored

**Reverse Dependency Resolution:**
- Uses `cargo metadata` to build dependency graph
- Identifies all crates that transitively depend on changed crates
- Ensures dependent crates are rebuilt when dependencies change

**Example Scenarios:**

| Change | Affected Crates | Reason |
|--------|----------------|--------|
| `crates/testcase-models/src/lib.rs` | testcase-models, testcase-manager, test-executor, verifier, ... | All crates that depend on testcase-models |
| `crates/verifier/src/main.rs` | verifier | Only the verifier binary crate |
| `Cargo.toml` (workspace root) | All workspace crates | Workspace-wide dependency change |
| `README.md` | None | Documentation change, no code impact |
| `crates/testcase-common/src/utils.rs` | testcase-common, all dependents | Shared utility library affects many crates |

#### Performance Comparison

Typical performance improvements when using incremental builds (single crate change):

| Target | Time | What Gets Built/Tested |
|--------|------|------------------------|
| `build-from BASE_REF=main` | ~10-30s | 1-5 affected crates |
| `build-all` | ~2-5m | All 24+ workspace crates |
| `test-from BASE_REF=main` | ~30s-2m | 1-5 crates + mapped E2E tests |
| `test-all` | ~5-10m | All crates + all E2E tests |

**Note:** Actual times vary based on:
- Number of affected crates
- Extent of changes (API vs implementation)
- Whether sccache is enabled
- Machine specifications

#### Best Practices

**Local Development Workflow:**
```bash
# 1. Create feature branch from main
git checkout -b feature/my-feature main

# 2. Make changes to code
vim crates/testcase-models/src/lib.rs

# 3. Quick incremental test during development
make test-from BASE_REF=main

# 4. Iterate rapidly
# Edit code -> make test-from -> repeat

# 5. Before committing: run full test suite
make test

# 6. Final validation before pushing (optional)
make test-all
```

**CI/CD Integration:**
```bash
# Feature branch CI: Use incremental testing for fast feedback
make test-from BASE_REF=origin/main

# Main branch CI: Use full testing for comprehensive validation
make test-all

# Release CI: Always use test-all for maximum confidence
make test-all
```

**Troubleshooting:**

| Issue | Solution |
|-------|----------|
| No crates detected when changes exist | Check `git status`, ensure changes are in `crates/` directory |
| Wrong crates being built | Verify `BASE_REF` points to correct branch/commit |
| E2E tests not running | Check crate-to-test mapping in `mk/incremental.mk` |
| Builds fail but `cargo build` works | Run `make build-all` to ensure clean slate |

#### Implementation Details

**Files:**
- **`mk/incremental.mk`**: Makefile definitions for incremental targets
- **`scripts/detect-local-changes.sh`**: Change detection and crate resolution script
- **`Makefile`**: Includes `mk/incremental.mk` via `include` directive

**Customization:**

To add E2E test mappings for new crates, edit `mk/incremental.mk`:

```makefile
# In the test-from target's case statement:
your-new-crate) \
    echo "path/to/test_your_crate_e2e.sh" >> $$E2E_TESTS_FILE; \
    echo "  your-new-crate binary -> 1 E2E test"; \
    ;; \
```

**Dependencies:**
- Requires Git repository (uses `git diff` for change detection)
- Requires `cargo metadata` for dependency graph analysis
- Requires workspace structure (all crates under `crates/` directory)

### Workspace Troubleshooting

Common workspace-related issues and solutions:

#### Issue: Dependency Version Conflicts

**Symptoms:**
```
error: failed to select a version for `serde`
  required by package `testcase-models v0.1.0`
  versions that meet the requirements `^1.0` are: 1.0.210, 1.0.209, ...
```

**Solution:**
```bash
# Check workspace dependency definition
grep -A2 "\[workspace.dependencies\]" Cargo.toml

# Ensure crate uses workspace dependency
# In crate Cargo.toml:
[dependencies]
serde = { workspace = true }  # ✓ Correct
# NOT: serde = "1.0"          # ✗ Wrong - version conflict

# Update all workspace dependencies
cargo update
```

#### Issue: Crate Not Found

**Symptoms:**
```
error: package ID specification `testcase-foo` did not match any packages
```

**Solution:**
```bash
# List all workspace members
cargo metadata --format-version 1 | jq '.workspace_members'

# Or check Cargo.toml
grep -A30 "\[workspace\]" Cargo.toml

# Verify crate is listed in workspace members
# Add if missing:
[workspace]
members = [
    "crates/testcase-foo",  # Add this line
]
```

#### Issue: Build Cache Issues

**Symptoms:**
```
error: failed to compile `testcase-manager`, intermediate artifacts are out of date
```

**Solution:**
```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Or clean specific crate
cargo clean -p testcase-manager
cargo build -p testcase-manager

# Clean coverage artifacts if present
make coverage-clean
```

#### Issue: Circular Dependencies

**Symptoms:**
```
error: cyclic package dependency: package `testcase-a` depends on itself
```

**Solution:**
```bash
# Visualize dependency graph
cargo tree -p testcase-a

# Check for circular references in Cargo.toml files
# Refactor to break circular dependency:
# Option 1: Extract shared code to new crate
# Option 2: Use dependency injection or traits
# Option 3: Reorganize crate boundaries
```

#### Issue: Feature Flag Conflicts

**Symptoms:**
```
error: feature `my-feature` is not present in package `testcase-models`
```

**Solution:**
```bash
# Check available features in target crate
grep -A10 "\[features\]" crates/testcase-models/Cargo.toml

# Use correct feature name in dependent crate
[dependencies]
testcase-models = { workspace = true, features = ["correct-feature-name"] }

# Build with all features to test
cargo build -p testcase-models --all-features
```

#### Issue: Workspace vs Package Confusion

**Symptoms:**
```bash
# Command runs on wrong scope
cargo build  # Only builds root package, not workspace
```

**Solution:**
```bash
# Explicit workspace commands
cargo build --workspace        # ✓ Builds all crates
cargo test --workspace         # ✓ Tests all crates
cargo clippy --workspace       # ✓ Lints all crates

# Use make targets (already workspace-aware)
make build    # Runs: cargo build --workspace
make test     # Runs: cargo test --workspace
make lint     # Runs: cargo clippy --workspace
```

#### Issue: Binary Name Conflicts

**Symptoms:**
```
warning: output filename collision: bin `validate-yaml` already exists
```

**Solution:**
```bash
# Check for duplicate binary names in workspace
find crates -name Cargo.toml -exec grep -H "name.*=.*\"validate-yaml\"" {} \;

# Rename one of the binaries
[[bin]]
name = "validate-yaml-v2"  # Give unique name
path = "src/bin/validate_yaml.rs"
```

#### Issue: Path Dependency Errors

**Symptoms:**
```
error: failed to load manifest for workspace member `crates/testcase-foo`
```

**Solution:**
```bash
# Verify directory structure
ls -la crates/testcase-foo/Cargo.toml

# Check path in workspace members list
[workspace]
members = [
    "crates/testcase-foo",  # Must match actual directory name
]

# Verify no typos in path
find crates -name Cargo.toml -type f
```

#### Issue: Incremental Compilation Issues

**Symptoms:**
```
error: could not compile `testcase-manager` due to previous error
# But no clear error shown
```

**Solution:**
```bash
# Disable incremental compilation temporarily
CARGO_INCREMENTAL=0 cargo build --workspace

# Or clean incremental artifacts
rm -rf target/debug/incremental/
cargo build --workspace

# Permanently disable in .cargo/config.toml (not recommended)
[build]
incremental = false
```

#### Issue: Out of Sync Cargo.lock

**Symptoms:**
```
error: the lock file needs to be updated
```

**Solution:**
```bash
# Update lock file
cargo update

# Or regenerate from scratch
rm Cargo.lock
cargo build --workspace

# Commit updated Cargo.lock
git add Cargo.lock
git commit -m "Update Cargo.lock"
```

#### Issue: Documentation Build Failures

**Symptoms:**
```
error[E0432]: unresolved import `testcase_models::TestCase`
# When running: cargo doc --workspace
```

**Solution:**
```bash
# Build docs for specific crate first
cargo doc -p testcase-models

# Check for doc-only dependencies
[dev-dependencies]
# May need to add:
testcase-models = { workspace = true }

# Build workspace docs with private items
cargo doc --workspace --document-private-items --no-deps
```

#### Getting Help

If workspace issues persist:

1. **Check workspace metadata:**
```bash
cargo metadata --format-version 1 | jq '.'
```

2. **Verify workspace structure:**
```bash
cargo tree --workspace
```

3. **Review build logs:**
```bash
cargo build --workspace -vv  # Very verbose output
```

4. **Consult workspace documentation:**
- [Cargo Workspaces Documentation](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- Project-specific: `WORKSPACE_STRUCTURE.md`

## Feature Overview

This project is a YAML-based test harness that converts declarative test case definitions into executable bash scripts. Key features include:

- **Declarative Test Cases**: Define test sequences, steps, and expectations in YAML
- **Variable Capture**: Extract values from command output using regex patterns or commands
- **Conditional Verification**: Support for if/then/else logic in verification expressions
- **Prerequisites**: Define manual and automatic prerequisites with verification commands
- **Environment Variables**: Hydration support with required/optional environment variables
- **Test Execution Lifecycle Hooks**: Optional hooks for custom setup, teardown, logging, and resource management at eight different lifecycle points
- **Shell Script Generation**: Generate portable bash 3.2+ compatible scripts from test cases
- **Comprehensive Validation**: Built-in schema validation and test execution verification

### Test Execution Lifecycle Hooks

Hooks provide optional extensibility points throughout the test execution lifecycle. **Hooks are entirely optional** - test cases work perfectly without them. When defined, hooks enable:

- **Custom Setup/Teardown**: Initialize and clean up resources at various lifecycle points
- **Logging and Monitoring**: Track test execution with custom logging at any stage
- **Resource Management**: Create temporary directories, files, and external resources
- **Integration with External Systems**: Connect to databases, APIs, or monitoring systems
- **Context-Aware Operations**: Access test execution context (sequences, steps, variables)
- **Error Handling**: Choose between strict (fail on error) or lenient (continue on error) modes

**Eight Hook Types Available:**
1. `script_start` - Once at script beginning (global initialization)
2. `setup_test` - Once after script_start (test-wide setup)
3. `before_sequence` - Before each test sequence (sequence initialization)
4. `after_sequence` - After each test sequence (sequence cleanup)
5. `before_step` - Before each test step (step preparation)
6. `after_step` - After each test step (step validation)
7. `teardown_test` - Once before script_end (test-wide cleanup)
8. `script_end` - Once at script end (final logging/cleanup)

See the [Hooks](#hooks) section for detailed documentation and examples.

## Commands

### Workspace Build Commands
- **Build**: make build (builds all workspace crates: `cargo build --workspace`)
- **Build Release**: make build-release (release mode: `cargo build --workspace --release`)
- **Build Debug**: make build-debug (debug mode, same as `make build`)
- **Build All**: make build-all (unconditional full build with all features: `cargo build --workspace --all-features`)
- **Lint**: make lint (runs fmt + clippy on all workspace crates)
- **Test**: make test (runs unit tests, e2e tests, and verification across workspace)
- **Test All**: make test-all (unconditional full clean build and comprehensive test suite with all features)
- **Test E2E All**: make test-e2e-all (unconditional full E2E test suite, all tests run in non-interactive mode)

### Incremental Build and Test Commands
These commands provide intelligent change detection for faster local development:
- **Build From**: make build-from BASE_REF=main (incrementally build only affected crates since BASE_REF)
- **Test From**: make test-from BASE_REF=main (incrementally test only affected crates with unit and E2E tests since BASE_REF)
- **List Changed Files**: make list-changed-files BASE_REF=main (show files changed since BASE_REF)
- **List Affected Crates**: make list-affected-crates BASE_REF=main (show crates affected by changes, including reverse dependencies)

See the [Selective Compilation and Testing](#selective-compilation-and-testing) section for detailed documentation.

### Per-Crate Build Commands
Build individual crates for faster development iteration:
- **Build Validate YAML**: make build-validate-yaml (`cargo build -p validate-yaml`)
- **Build Validate JSON**: make build-validate-json (`cargo build -p validate-json`)
- **Build Verifier**: make build-verifier (`cargo build -p verifier`)
- **Build Test Executor**: make build-test-executor (`cargo build -p test-executor`)
- **Build Test Orchestrator**: make build-test-orchestrator (`cargo build -p test-orchestrator`)
- **Build Test Run Manager**: make build-test-run-manager (`cargo build -p test-run-manager`)
- **Build Test Verify**: make build-test-verify (`cargo build -p test-verify`)
- **Build Script Cleanup**: make build-script-cleanup (`cargo build -p script-cleanup`)
- **Build JSON Escape**: make build-json-escape (`cargo build -p json-escape`)
- **Build JSON to YAML**: make build-json-to-yaml (`cargo build -p json-to-yaml`)
- **Build Editor**: make build-editor (`cargo build -p editor`)
- **Build Testcase Manager**: make build-testcase-manager (`cargo build -p testcase-manager`)
- **Build TPDG Compat**: make build-tpdg-compat (`cargo build -p tpdg-compat`)
- **Build Bash Eval**: make build-bash-eval (`cargo build -p bash-eval`)

### Test Commands
- **Test Verifier Edge Cases**: make test-verifier-edge-cases (run verifier edge case unit tests and integration tests)
- **Coverage**: make coverage (run unit tests with workspace coverage analysis, 50% threshold)
- **Coverage E2E**: make coverage-e2e (run unit + e2e tests with workspace coverage, 70% threshold)
- **Coverage HTML**: make coverage-html (generate HTML coverage report for workspace)
- **Coverage HTML E2E**: make coverage-html-e2e (generate HTML coverage report with e2e tests)
- **Coverage Report**: make coverage-report (display workspace coverage summary)
- **Coverage Report E2E**: make coverage-report-e2e (display workspace coverage summary with e2e tests)

### Build Tool Commands
- **Install Coverage Tools**: make install-coverage-tools (install cargo-llvm-cov and related tools)
- **Install sccache**: make install-sccache (install sccache compilation cache)
- **Enable sccache**: make enable-sccache (show instructions to enable sccache for current session)
- **Disable sccache**: make disable-sccache (show instructions to disable sccache if causing compilation issues)
- **Check sccache**: make sccache-check (verify if sccache is properly configured and enabled)
- **sccache Stats**: make sccache-stats (display sccache compilation cache statistics)
- **sccache Clean**: make sccache-clean (stop sccache server, preserve cache)
- **Install sccache**: make install-sccache (install sccache compilation cache for faster workspace builds)
- **sccache Stats**: make sccache-stats (display sccache compilation cache statistics)
- **sccache Clean**: make sccache-clean (clear sccache compilation cache)

### Validation and Verification Commands
- **Verify Scripts**: make verify-scripts (verify syntax of all shell scripts)
- **Validate Output Schemas**: make validate-output-schemas (validate expected output samples against schemas)
- **Validate Test Cases Report**: make validate-testcases-report (generate detailed validation report for all test case YAML files in testcases/ directory, output saved to reports/validation_report.txt)
- **Watch Mode**: make watch (monitors testcases/ for changes and auto-validates)
- **Generate Docs**: make generate-docs (generate documentation reports using test-plan-documentation-generator)
- **Generate Docs All**: make generate-docs-all (generate documentation reports for all test scenarios using test-plan-documentation-generator)
- **Generate Docs Coverage**: make generate-docs-coverage (run documentation generation with tarpaulin coverage analysis)
- **Test Container Compatibility**: make test-container-compat (verify container YAML compatibility with test-plan-doc-gen)
- **Acceptance Tests**: make acceptance-test (run full acceptance test suite with 7 stages: validation, generation, execution, verification, container validation, per-test documentation, and consolidated documentation)
- **Acceptance Suite E2E Tests**: make test-e2e-acceptance (run E2E integration tests for the acceptance suite orchestrator)
- **Install LOC**: make install-loc (install tokei/loc lines of code counter)
- **LOC Statistics**: make loc (compute lines of code statistics for Rust, Python, Shell, and documentation)
- **LOC Verbose**: make loc-verbose (compute lines of code statistics with verbose output)
- **LOC JSON**: make loc-json (compute lines of code statistics in JSON format)
- **LOC YAML**: make loc-yaml (compute lines of code statistics in YAML format)
- **LOC Report**: make loc-report (generate lines of code statistics report to reports/loc/loc_statistics.txt)
- **Setup Python**: make setup-python (install and configure Python 3.14 with uv package manager)
- **Verify Python**: make verify-python (verify Python 3.14 environment is properly configured)
- **Test Comparison Report**: make test-comparison-report (generate JSON report comparing test execution before and after crate splitting, includes test counts, execution times, and per-crate test organization)
- **Test Comparison From Files**: make test-comparison-from-files BEFORE=before.txt AFTER=after.txt (generate test comparison report from pre-saved cargo test outputs)
- **Dev Server**: N/A

### Pipeline Testing

The project includes comprehensive pipeline testing tools that validate test cases through a complete 5-stage pipeline:

**Pipeline E2E Test** (single test case validation):
```bash
make test-e2e-pipeline
```
Runs the pipeline validation on two test cases (one passing, one failing) to verify all stages work correctly.

**Pipeline Batch Processor** (all test cases):
```bash
make test-e2e-pipeline-batch
```
Applies the 5-stage pipeline to all test case YAMLs in the `testcases/` directory and reports grand totals of success/failure per stage.

**Direct Script Execution**:
```bash
# Single test case pipeline
./crates/testcase-manager/tests/integration/test_pipeline_e2e.sh

# Batch processor with options
./crates/testcase-manager/tests/integration/test_pipeline_batch.sh [--no-remove] [--testcases-dir DIR]
```

**Options**:
- `--no-remove`: Preserve temporary files for debugging
- `--testcases-dir DIR`: Process YAMLs from a custom directory (default: testcases/)

**5-Stage Pipeline**:
1. **Stage 1**: YAML validation against test-case.schema.json
2. **Stage 2**: Script generation with bash syntax and shellcheck validation
3. **Stage 3**: Script execution with JSON output validation
4. **Stage 4**: Verification with YAML report generation
5. **Stage 5**: Result summary generation in JSON format

**Batch Processor Output**:
- Per-stage statistics (total, success, failure, success rate %)
- Overall test case statistics (total, completed, passed, failed)
- Detailed lists of passed and failed test cases with failure stage information
- Organized output directories: logs/, scripts/, executions/, verifications/, results/

**Requirements**:
- bash 3.2+ compatible
- jq (JSON processing)
- shellcheck (optional, for enhanced validation)

**Documentation**:
- User Guide: `PIPELINE_BATCH_PROCESSOR.md`
- Implementation Details: `IMPLEMENTATION_PIPELINE_BATCH.md`

### Sccache Configuration

**⚠️ IMPORTANT**: If you're experiencing build failures with exit status 254, immediately disable sccache:
```bash
unset RUSTC_WRAPPER
cargo clean && cargo build
```

The project uses **sccache** (Shared Compilation Cache) to accelerate Rust compilation by caching build artifacts. The cache is configured to use a **global directory in the user's home folder** to enable cache sharing across multiple worktrees and git checkouts.

**Global Cache Directory**: `~/.cache/sccache/testcase-manager`

**Benefits**:
- **Worktree Sharing**: Multiple worktrees of the same repository share the same cache
- **Persistent Cache**: Cache survives repository deletion and recreation
- **Faster Builds**: Reduced compilation time across all worktrees
- **Disk Efficiency**: Single cache location instead of per-worktree caches

**Configuration**:
The cache directory is exported by `scripts/enable-sccache.sh`:
```bash
export SCCACHE_DIR="$HOME/.cache/sccache/testcase-manager"
```

It is deliberately **not** set in `.cargo/config.toml`: Cargo's `[env]` table
performs no shell or variable expansion, so `"$HOME/.cache/sccache/..."` reaches
sccache literally and makes it create a directory *named* `$HOME` in the build
working directory.

**Setup (Required for sccache to work)**:
```bash
# 1. Install sccache (one-time)
make install-sccache

# 2. Enable sccache for current shell session
source ./scripts/enable-sccache.sh

# 3. (Optional) Enable permanently (adds to ~/.bashrc or ~/.zshrc)
source ./scripts/enable-sccache.sh --permanent

# 4. Verify sccache is enabled
make sccache-check
```

**Usage Commands**:
```bash
# Check if sccache is enabled
make sccache-check

# View cache statistics
make sccache-stats

# Disable sccache (if causing compilation issues)
source ./scripts/disable-sccache.sh

# Stop sccache server (preserves cache)
make sccache-clean

# Manually clear cache (if needed)
rm -rf ~/.cache/sccache/testcase-manager
```

**Troubleshooting Compilation Issues**:

If you encounter compilation errors with sccache (exit status 254), disable it:
```bash
# Disable sccache for current session
source ./scripts/disable-sccache.sh

# Clean and rebuild
cargo clean
cargo build
```

See [docs/SCCACHE_SETUP.md](docs/SCCACHE_SETUP.md) for detailed troubleshooting.

**Docker Environment**:
In Docker builds, sccache uses `/root/.cache/sccache/testcase-manager` as the cache directory. This can be persisted across builds using Docker cache mounts.

**Manual Override**:
To use a custom cache location, set `SCCACHE_DIR` before sourcing the enable script (it honours an existing value):
```bash
export SCCACHE_DIR=/my/custom/cache
source ./scripts/enable-sccache.sh
```

### Report Generation

All report generation now uses the Rust-based **test-plan-documentation-generator** (tpdg) tool, which generates AsciiDoc, Markdown, and HTML reports from test cases and verification results.

**Python PDF Generation Removed**: The legacy Python-based PDF generation (scripts/generate_verifier_reports.py) has been removed. The reportlab dependency has been removed from pyproject.toml. The only remaining Python dependency is pyyaml, which is required for the convert_verification_to_result_yaml.py script.

**Report Formats Supported**:
- AsciiDoc (.adoc) - Structured documentation format
- Markdown (.md) - GitHub-compatible documentation
- HTML - Generated from AsciiDoc (requires asciidoctor for conversion)

**Installation**:
```bash
# Install test-plan-documentation-generator globally
cargo install test-plan-documentation-generator

# Or use custom path
export TEST_PLAN_DOC_GEN=/path/to/test-plan-documentation-generator/binary
```

**Usage**:
```bash
# Generate documentation reports
make generate-docs          # Verifier scenarios only
make generate-docs-all      # All test cases
```

**Benefits of test-plan-documentation-generator**:
- Better performance and maintainability
- Native integration with the Rust test framework
- Consistent report generation across all test scenarios
- Support for multiple output formats (AsciiDoc, Markdown, HTML)
- No external Python dependencies for report generation
- Schema validation for container YAML compatibility

**Troubleshooting**:
See [Report Generation Documentation](docs/report_generation.md) for detailed installation, configuration, schema compatibility requirements, and troubleshooting steps.

### Python 3.14 Environment Setup

The project requires Python 3.14 for various utility scripts and CI/CD tools. Python 3.14 is managed using the **`uv`** package manager, which provides fast, reliable Python environment management.

#### Quick Start

**Local Setup**:
```bash
# Install and configure Python 3.14 environment
make setup-python

# Verify Python 3.14 is properly configured
make verify-python
```

**Docker Setup**:
The Docker image automatically installs and configures Python 3.14 during build. No manual setup required.

#### Understanding uv Package Manager

**What is uv?**
- Modern, fast Python package and project manager written in Rust
- Replaces pip, pip-tools, virtualenv, and pyenv functionality
- Provides deterministic dependency resolution with lock files
- Handles Python version management (installation and switching)

**Key Benefits**:
- **Speed**: 10-100x faster than pip for dependency resolution and installation
- **Reliability**: Lock files ensure reproducible environments across machines
- **Simplicity**: Single tool for all Python environment needs
- **Version Management**: Automatically downloads and manages Python versions

**Installation**:
```bash
# Install uv package manager
curl -LsSf https://astral.sh/uv/install.sh | sh

# Add uv to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"
```

**Official Documentation**: https://docs.astral.sh/uv/

#### Environment Setup Process

The setup process performs the following steps:

1. **Verify uv Installation**: Checks that `uv` is available in PATH
2. **Sync Dependencies**: Reads `pyproject.toml` and `uv.lock` to install dependencies
3. **Install Python 3.14**: Downloads and installs Python 3.14 if not already present
4. **Set Default Version**: Configures Python 3.14 as the default version for the project
5. **Create Virtual Environment**: Sets up `.venv/` directory with Python 3.14
6. **Re-sync Dependencies**: Ensures all dependencies are installed for Python 3.14
7. **Verify Installation**: Confirms Python 3.14 is working correctly

**Setup Script Location**: `scripts/setup_python_env.sh`

**What Gets Created**:
- `.venv/` - Virtual environment directory containing Python 3.14 and packages
- `uv.lock` - Lock file with pinned dependency versions (if not already present)
- Python binaries available via `uv run` or by activating virtual environment

#### Available Python Commands

After setup, you can use Python 3.14 in several ways:

**1. Via uv run (Recommended)**:
```bash
# Run Python scripts with uv
uv run python3.14 script.py
uv run python3.14 -c "import yaml; print(yaml.__version__)"

# uv automatically uses the project's virtual environment
uv run python script.py  # Uses Python 3.14 from .venv
```

**2. Activate Virtual Environment**:
```bash
# Activate the virtual environment
source .venv/bin/activate

# Now python3.14 points to the virtual environment
python3.14 --version
python3 --version  # Also points to 3.14 in activated venv

# Deactivate when done
deactivate
```

**3. Direct Invocation (Docker Only)**:
```bash
# In Docker, global symlinks are created
python3.14 --version
python3 --version
python --version  # All point to Python 3.14
```

**4. Check Python Version**:
```bash
# Verify Python 3.14 is active
uv run python3.14 --version  # Python 3.14.x
uv python find 3.14          # Shows path to Python 3.14
```

#### Python Dependencies

The project uses minimal Python dependencies defined in `pyproject.toml`:

| Package | Version | Purpose |
|---------|---------|---------|
| `pyyaml` | >=6.0.3 | YAML parsing (convert_verification_to_result_yaml.py) |
| `jsonschema` | >=4.26.0 | JSON schema validation |
| `mypy` | >=1.19.1 | Static type checking for Python scripts |
| `ruff` | >=0.15.6 | Fast Python linting and code formatting |

**Note**: The project previously used `reportlab` for PDF generation, but this has been removed in favor of the Rust-based `test-plan-documentation-generator` tool.

#### Adding New Dependencies

To add a new Python dependency to the project:

**1. Add to pyproject.toml**:
```bash
# Add a new dependency with version constraint
uv add "package-name>=1.0.0"

# Add a development dependency
uv add --dev "pytest>=7.0.0"

# Add with specific version
uv add "requests==2.31.0"
```

**2. Manual Edit (Alternative)**:
```toml
# Edit pyproject.toml manually
[project]
dependencies = [
    "jsonschema>=4.26.0",
    "mypy>=1.19.1",
    "pyyaml>=6.0.3",
    "ruff>=0.15.6",
    "requests>=2.31.0",  # Add your new dependency
]
```

Then sync:
```bash
uv sync
```

**3. Update Lock File**:
```bash
# Regenerate uv.lock with new dependencies
uv lock

# Or sync (which also updates lock file if needed)
uv sync
```

**4. Verify Installation**:
```bash
# Check that the new package is available
uv run python3.14 -c "import requests; print(requests.__version__)"
```

**5. Update Docker Image**:
After adding dependencies, rebuild the Docker image to include them:
```bash
docker build -t your-image-name .
```

**Best Practices**:
- Use version constraints (`>=`, `~=`) rather than exact versions for flexibility
- Run `uv lock` after adding dependencies to update the lock file
- Commit both `pyproject.toml` and `uv.lock` to version control
- Test that dependencies work in both local and Docker environments

#### Removing Dependencies

**1. Remove from pyproject.toml**:
```bash
# Remove a dependency
uv remove package-name
```

**2. Manual Edit (Alternative)**:
```toml
# Edit pyproject.toml and remove the dependency line
[project]
dependencies = [
    "jsonschema>=4.26.0",
    "mypy>=1.19.1",
    "pyyaml>=6.0.3",
    # "removed-package>=1.0.0",  # Remove this line
]
```

Then sync:
```bash
uv sync
```

**3. Clean Virtual Environment (Optional)**:
```bash
# Remove and recreate virtual environment
rm -rf .venv
uv sync
```

#### Upgrading Dependencies

**Update All Dependencies**:
```bash
# Update all dependencies to latest compatible versions
uv lock --upgrade

# Sync to apply updates
uv sync
```

**Update Specific Package**:
```bash
# Update specific package to latest compatible version
uv lock --upgrade-package pyyaml

# Sync to apply
uv sync
```

**Update to Specific Version**:
```bash
# Edit pyproject.toml to change version constraint
# Then sync
uv sync
```

#### Docker Environment

The Docker image automatically sets up Python 3.14 during build:

**Dockerfile Setup Process**:
1. Copies `uv` binary from official `ghcr.io/astral-sh/uv` image
2. Copies `pyproject.toml` and `uv.lock` to container
3. Runs `uv sync --frozen` to install exact locked versions
4. Installs Python 3.14 and sets as default: `uv python install --default 3.14`
5. Creates global symlinks for `python3.14`, `python3`, and `python`
6. Re-syncs dependencies with Python 3.14: `uv sync --frozen --python 3.14`

**Docker Verification**:
```bash
# Verify Python 3.14 is available in container
docker run your-image python3.14 --version
docker run your-image python3 --version
docker run your-image python --version

# All should output: Python 3.14.x
```

**Rebuild After Dependency Changes**:
```bash
# After modifying pyproject.toml or uv.lock
docker build -t your-image-name .
```

#### Verification Process

The verification script (`scripts/verify_python_env.sh`) performs comprehensive checks:

**Local Environment Tests**:
1. ✓ `python3.14` is available in PATH
2. ✓ `python3.14 --version` returns Python 3.14.x
3. ✓ `python3` points to Python 3.14 (optional)
4. ✓ `uv` package manager is available
5. ✓ `uv python find 3.14` locates Python 3.14
6. ✓ `uv run python3.14 --version` works
7. ✓ Required Python packages (pyyaml, jsonschema) are importable

**Docker Environment Tests**:
1. ✓ `python3.14` is available globally
2. ✓ `python3.14 --version` returns Python 3.14.x
3. ✓ `python3` symlink points to Python 3.14
4. ✓ `python` symlink points to Python 3.14
5. ✓ Required Python packages are importable

**Run Verification**:
```bash
# Local environment
make verify-python

# Or run script directly
./scripts/verify_python_env.sh

# Docker environment
docker run your-image make verify-python
```

#### Troubleshooting Python 3.14 Migration

##### Issue: uv not found

**Symptoms**:
```
Error: uv is not installed
```

**Solution**:
```bash
# Install uv package manager
curl -LsSf https://astral.sh/uv/install.sh | sh

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"

# Reload shell configuration
source ~/.bashrc  # or source ~/.zshrc
```

##### Issue: Python 3.14 not found after setup

**Symptoms**:
```
Error: Failed to find Python 3.14 after installation
```

**Solution**:
```bash
# Manually install Python 3.14
uv python install 3.14

# Verify installation
uv python find 3.14

# Re-run setup
make setup-python
```

##### Issue: Module import errors in Python 3.14

**Symptoms**:
```python
ModuleNotFoundError: No module named 'yaml'
```

**Solution**:
```bash
# Re-sync dependencies
uv sync

# Verify packages are installed
uv run python3.14 -c "import yaml; print('OK')"

# If still failing, recreate virtual environment
rm -rf .venv
uv sync
```

##### Issue: Different Python version in virtual environment

**Symptoms**:
```bash
$ source .venv/bin/activate
$ python --version
Python 3.12.0  # Wrong version!
```

**Solution**:
```bash
# Deactivate current environment
deactivate

# Remove virtual environment
rm -rf .venv

# Reinstall with Python 3.14 explicitly
uv sync --python 3.14

# Verify
source .venv/bin/activate
python --version  # Should show Python 3.14.x
```

##### Issue: uv.lock conflicts after git merge

**Symptoms**:
```
Error: Failed to parse uv.lock
Git merge conflict in uv.lock
```

**Solution**:
```bash
# Resolve git conflicts in uv.lock manually, or:

# Regenerate lock file from pyproject.toml
rm uv.lock
uv lock

# Sync to install dependencies
uv sync
```

##### Issue: Dependency resolution conflicts

**Symptoms**:
```
Error: dependency resolution failed
```

**Solution 1 - Check version constraints**:
```bash
# Review pyproject.toml for conflicting version constraints
cat pyproject.toml

# Try loosening version constraints
# Change: package>=2.0.0,<2.1.0
# To:     package>=2.0.0
```

**Solution 2 - Update dependencies**:
```bash
# Update all dependencies to latest compatible versions
uv lock --upgrade
uv sync
```

**Solution 3 - Fresh install**:
```bash
# Remove lock file and virtual environment
rm uv.lock
rm -rf .venv

# Regenerate from scratch
uv lock
uv sync
```

##### Issue: Docker build fails with Python errors

**Symptoms**:
```
ERROR: Failed to sync Python dependencies in Docker
```

**Solution 1 - Verify lock file is committed**:
```bash
# Ensure uv.lock is in git
git add uv.lock
git commit -m "Add uv.lock"

# Rebuild Docker image
docker build -t your-image-name .
```

**Solution 2 - Update Docker base image**:
```bash
# Check Dockerfile uses recent uv version
# Ensure this line is present:
# COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv
```

**Solution 3 - Clean Docker build**:
```bash
# Build without cache
docker build --no-cache -t your-image-name .
```

##### Issue: Permission denied when running uv

**Symptoms**:
```
Permission denied: cannot create .venv
```

**Solution**:
```bash
# Check directory permissions
ls -la .venv

# Remove and recreate with correct permissions
rm -rf .venv
uv sync

# For Docker, ensure correct user/permissions in Dockerfile
```

##### Issue: Old Python version still active

**Symptoms**:
```bash
$ python3 --version
Python 3.12.0  # Old version
```

**Solution 1 - Use uv run**:
```bash
# Use uv run to ensure correct version
uv run python3.14 --version
```

**Solution 2 - Activate virtual environment**:
```bash
source .venv/bin/activate
python3.14 --version
```

**Solution 3 - Update PATH (local development)**:
```bash
# Find Python 3.14 path
PYTHON_314_PATH=$(uv python find 3.14)

# Add to PATH temporarily
export PATH="$(dirname $PYTHON_314_PATH):$PATH"

# Or add to ~/.bashrc for permanent change
```

##### Issue: Scripts fail with Python syntax errors

**Symptoms**:
```python
SyntaxError: invalid syntax
# Using Python 3.14+ features in older Python
```

**Solution**:
```bash
# Verify script is running with Python 3.14
uv run python3.14 script.py

# Check shebang in script
# Should be: #!/usr/bin/env python3.14
# Or use:    #!/usr/bin/env python3
```

##### Issue: Missing dependencies after migration

**Symptoms**:
```
ImportError: cannot import name 'X' from 'package'
```

**Solution**:
```bash
# Check if package version is compatible with Python 3.14
uv tree | grep package-name

# Update to compatible version
uv add "package-name>=compatible-version"
uv sync

# Or update all dependencies
uv lock --upgrade
uv sync
```

##### Issue: CI/CD fails with Python errors

**Symptoms**:
```
CI Error: Python 3.14 not found in CI environment
```

**Solution**:
```bash
# Ensure CI uses proper setup commands
# Add to CI configuration:

# For local CI:
make setup-python
make verify-python

# For Docker-based CI:
# Ensure Docker image is built with Python 3.14 support
```

#### Advanced uv Commands

**Check Installed Packages**:
```bash
# List all installed packages
uv pip list

# Show dependency tree
uv tree

# Show package information
uv pip show pyyaml
```

**Python Version Management**:
```bash
# List installed Python versions
uv python list

# Install specific Python version
uv python install 3.14.1

# Set default Python version for project
uv python pin 3.14
```

**Virtual Environment Management**:
```bash
# Create virtual environment manually
uv venv --python 3.14

# Remove virtual environment
rm -rf .venv

# Recreate from pyproject.toml
uv sync
```

**Lock File Operations**:
```bash
# Generate lock file without installing
uv lock

# Update lock file with latest compatible versions
uv lock --upgrade

# Update specific packages
uv lock --upgrade-package pyyaml --upgrade-package jsonschema

# Check if lock file is up to date
uv lock --check
```

**Debugging**:
```bash
# Verbose output for debugging
uv sync --verbose

# Very verbose output
uv sync -vv

# Show what would be installed without installing
uv sync --dry-run
```

#### Migration Checklist

When migrating existing Python scripts to Python 3.14:

- [ ] Install uv package manager (`curl -LsSf https://astral.sh/uv/install.sh | sh`)
- [ ] Run `make setup-python` to set up Python 3.14
- [ ] Run `make verify-python` to confirm setup
- [ ] Update scripts to use `uv run python3.14` or activate virtual environment
- [ ] Test all Python scripts with Python 3.14
- [ ] Check for deprecated Python features (if migrating from much older versions)
- [ ] Update CI/CD configuration to use Python 3.14
- [ ] Rebuild Docker images with Python 3.14
- [ ] Update documentation to reflect Python 3.14 requirement
- [ ] Commit `pyproject.toml` and `uv.lock` to version control

#### Additional Resources

- **uv Documentation**: https://docs.astral.sh/uv/
- **uv GitHub Repository**: https://github.com/astral-sh/uv
- **Python 3.14 Release Notes**: https://docs.python.org/3.14/whatsnew/3.14.html
- **pyproject.toml Specification**: https://packaging.python.org/en/latest/specifications/pyproject-toml/

#### Getting Help

If you encounter issues not covered in this documentation:

1. Run `make verify-python` to diagnose the problem
2. Check `uv sync --verbose` output for detailed error messages
3. Review `uv.lock` for dependency conflicts
4. Consult uv documentation: https://docs.astral.sh/uv/
5. Check project-specific scripts: `scripts/setup_python_env.sh` and `scripts/verify_python_env.sh`

You must build, test, lint, verify coverage, and run acceptance tests before committing

## Acceptance Test Suite

The acceptance test suite provides comprehensive end-to-end testing of the entire test execution workflow, from YAML validation through to documentation generation.

### Running Acceptance Tests

**Command**: `make acceptance-test`

This target executes the full acceptance test suite, which includes:
1. Building all required binaries (test-executor, verifier, validate-yaml)
2. Validating TPDG (test-plan-documentation-generator) availability
3. Running all acceptance test stages
4. Capturing output to both console and log file
5. Generating final summary report
6. Displaying statistics and results

**Exit Codes**:
- `0` - All tests passed successfully
- `1` - One or more tests failed

**Prerequisites**:
- TPDG must be installed and available:
  - Install globally: `cargo install test-plan-documentation-generator`
  - Or set environment variable: `export TEST_PLAN_DOC_GEN=/path/to/tpdg`

**Output Files**:
- Execution log: `test-acceptance/reports/acceptance_suite_execution.log`
- Summary report: `test-acceptance/reports/acceptance_suite_summary.txt`
- Test results: `test-acceptance/verification_results/`
- Documentation: `test-acceptance/reports/asciidoc/` and `test-acceptance/reports/markdown/`

### Acceptance Test Stages

The acceptance test suite runs seven stages:

1. **YAML Validation** - Validates all test case YAMLs against schema
2. **Script Generation** - Generates executable bash scripts from test cases
3. **Test Execution** - Executes all automated tests (skips manual tests by default)
4. **Verification** - Runs verifier on execution logs to generate container YAMLs
5. **Container Validation** - Validates container YAMLs against schema
6. **Per-Test Documentation** - Generates individual AsciiDoc and Markdown documentation for each test using TPDG
7. **Consolidated Documentation** - Generates unified AsciiDoc and Markdown documentation combining all test results using TPDG

### Manual Test Suite Execution

For advanced usage, run the acceptance suite script directly:

```bash
# Run with default settings
./test-acceptance/run_acceptance_suite.sh

# Run with verbose output
./test-acceptance/run_acceptance_suite.sh --verbose

# Include manual tests in execution
./test-acceptance/run_acceptance_suite.sh --include-manual

# Skip specific stages
./test-acceptance/run_acceptance_suite.sh --skip-generation
./test-acceptance/run_acceptance_suite.sh --skip-execution
./test-acceptance/run_acceptance_suite.sh --skip-verification
./test-acceptance/run_acceptance_suite.sh --skip-documentation
./test-acceptance/run_acceptance_suite.sh --skip-consolidated-docs
```

### CI/CD Integration

The `acceptance-test` target is included in the `pre-commit` checks to ensure all code changes pass the full acceptance test suite before commit. This provides comprehensive validation of:
- Code functionality
- Test execution workflow
- Documentation generation
- Schema compliance
- End-to-end integration

### Acceptance Suite E2E Tests

**Command**: `make test-e2e-acceptance`

The acceptance suite E2E tests validate that the `run_acceptance_suite.sh` orchestrator works correctly by running it on a subset of test cases and verifying all stages complete successfully.

**Test Coverage**:
- Validates all 7 stages complete successfully
- Checks expected files are created at each stage (scripts, logs, containers, per-test documentation, consolidated documentation)
- Validates final report is generated with correct statistics
- Tests all `--skip-*` flags work correctly (generation, execution, verification, documentation, consolidated-docs)
- Ensures `--verbose` flag increases logging detail
- Verifies error handling for missing dependencies (TPDG not available)
- Tests timeout handling for long-running scripts
- Confirms cleanup of temporary files after completion
- Tests combining multiple `--skip-*` flags

**Test Subset**:
- 5 success scenarios
- 3 failure scenarios
- 2 hook scenarios

**Documentation**: See `test-acceptance/tests/README.md` for detailed information on the E2E test implementation and adding new tests.

## Binaries

The project includes several binary utilities:

- **json-escape**: A utility that reads from stdin and performs JSON string escaping. Supports a test mode (`--test`) to validate that escaped output is valid JSON when wrapped in quotes, and verbose mode (`--verbose`) for detailed logging.
  - Build: `make build-json-escape`
  - Run: `make run-json-escape` or `cargo run --bin json-escape`
  - Usage: `echo "text" | json-escape`

- **test-plan-documentation-generator-compat**: A compatibility checker that verifies container YAML files are compatible with the test-plan-doc-gen tool. Validates schema compliance, generates compatibility reports, and tests against verifier scenarios.
  - Build: `cargo build --bin test-plan-documentation-generator-compat`
  - Run: `cargo run --bin test-plan-documentation-generator-compat -- <command>`
  - Usage: `test-plan-documentation-generator-compat validate container.yaml`
  - Commands: `validate`, `batch`, `test-verifier-scenarios`, `report`
  - Documentation: `docs/TEST_PLAN_DOC_GEN_COMPATIBILITY.md`

- **test-verify** (verifier): Test verification tool for validating test execution logs against test case definitions.
  - Build: `make build` or `cargo build --bin verifier`
  - Run: `cargo run --bin verifier` or `./target/release/verifier`
  - Features: Batch processing, YAML/JSON output formats with rich metadata, aggregated reports
  - Output Formats:
    - **yaml**: YAML format with rich metadata (title, project, environment, platform, executor)
    - **json**: JSON format with rich metadata
  - Configuration File:
    - `--config <PATH>`: Path to YAML configuration file (optional)
    - Schema: `schemas/container_config.schema.json`
    - Configuration file format (see `verifier-config.example.yaml`):
      ```yaml
      title: "Test Execution Results"
      project: "Test Case Manager - Verification Results"
      environment: "Staging"
      platform: "Linux x86_64"
      executor: "Jenkins v3.2"
      ```
    - All fields are optional with sensible defaults
  - CLI Flags (override config file):
    - `--title`: Report title (default: "Test Execution Results")
    - `--project`: Project name (default: "Test Case Manager - Verification Results")
    - `--environment`: Environment information (e.g., "Staging", "Production")
    - `--platform`: Platform information (e.g., "Linux x86_64")
    - `--executor`: Executor information (e.g., "CI Pipeline v2.1")
  - Precedence: CLI flags > Configuration file > Default values
  - Examples:
    - Using defaults:
      ```bash
      verifier -f logs/ --format yaml --output report.yaml
      ```
    - Using config file:
      ```bash
      verifier -f logs/ --format yaml --output report.yaml --config verifier-config.yaml
      ```
    - Using only CLI flags:
      ```bash
      verifier -f logs/ --format yaml --output report.yaml \
        --title "Nightly Test Run" \
        --environment "Production" \
        --platform "Linux x86_64"
      ```
    - Combining config file and CLI flags (CLI overrides config):
      ```bash
      verifier -f logs/ --format yaml --output report.yaml \
        --config verifier-config.yaml \
        --title "Nightly Test Run" \
        --environment "Production"
      ```

## JSON Escaping Methods

When capturing command output for JSON formatting in generated bash scripts, the test execution framework supports multiple JSON escaping methods with automatic fallback. This ensures robust handling of special characters, newlines, and control codes in command output.

### Overview

**Preferred Method**: `jq` is now the preferred method for JSON output capture due to its robustness, native JSON handling, and widespread availability in modern environments.

**Auto Mode Priority**: When using `auto` mode (default), the framework automatically selects the best available method using this fallback chain:
1. **jq** - JSON processor (most robust, preferred)
2. **json-escape** - Custom Rust binary (fast, reliable, works when jq unavailable)
3. **Shell fallback** - Native bash string escaping (least robust, always available)

### Configuration

JSON escaping method is configured in the test case configuration TOML file under the `json_escaping` section.

#### Configuration Location

Create a configuration file (e.g., `config.toml` or `testcase-config.toml`) with the following structure:

```toml
[json_escaping]
# Method: "auto", "jq", "json-escape", or "shell"
method = "auto"
```

#### Available Methods

| Method | Description | Requirements | Robustness |
|--------|-------------|--------------|------------|
| `auto` | Automatic selection (jq > json-escape > shell) | None (always works) | High |
| `jq` | Use jq JSON processor | jq must be installed and in PATH | Highest |
| `json-escape` | Use custom Rust binary | json-escape binary in PATH or target/ | High |
| `shell` | Use bash string escaping | None (bash built-in) | Moderate |

#### Configuration Examples

**Example 1: Auto mode (recommended)**
```toml
[json_escaping]
method = "auto"
```

**Example 2: Force jq (production environments)**
```toml
[json_escaping]
method = "jq"
```

**Example 3: Force json-escape binary**
```toml
[json_escaping]
method = "json-escape"
```

**Example 4: Force shell fallback (constrained environments)**
```toml
[json_escaping]
method = "shell"
```

#### Using Configuration File

Pass the configuration file to test-executor using the `--config` flag:

```bash
# Generate script with configuration
test-executor generate testcases/TC_001.yaml --config config.toml

# Execute test with configuration
test-executor execute testcases/TC_001.yaml --config config.toml
```

**Default Behavior**: If no configuration file is provided or the `json_escaping` section is missing, the framework uses `auto` mode by default.

### Method Details

#### 1. jq (Preferred)

**Why jq is Preferred**:
- **Native JSON handling**: Correctly escapes all special characters, Unicode, and control codes
- **Standard compliance**: Produces valid JSON strings according to RFC 8259
- **Robustness**: Handles edge cases (null bytes, binary data, invalid UTF-8) gracefully
- **Widespread availability**: Pre-installed on most modern Linux distributions and CI/CD environments
- **Performance**: Fast and efficient for JSON processing
- **Reliability**: Battle-tested in production environments worldwide

**Usage in Generated Scripts**:
```bash
# Example: Capture output with jq escaping
OUTPUT=$(command_to_run 2>&1)
ESCAPED_OUTPUT=$(printf '%s' "$OUTPUT" | jq -Rs .)
echo "{\"output\": $ESCAPED_OUTPUT}" >> result.json
```

**Requirements**:
- jq must be installed: `apt-get install jq` (Debian/Ubuntu) or `brew install jq` (macOS)
- jq must be in PATH

**Verification**:
```bash
# Check if jq is available
command -v jq >/dev/null 2>&1 && echo "jq available" || echo "jq not found"

# Test jq escaping
echo 'Hello "World"' | jq -Rs .
# Output: "Hello \"World\"\n"
```

#### 2. json-escape (Custom Binary)

**When to Use**:
- jq is not available in the environment
- Need consistent behavior across different environments
- Prefer Rust-based tooling

**Usage in Generated Scripts**:
```bash
# Example: Capture output with json-escape binary
OUTPUT=$(command_to_run 2>&1)
ESCAPED_OUTPUT=$(printf '%s' "$OUTPUT" | json-escape)
echo "{\"output\": \"$ESCAPED_OUTPUT\"}" >> result.json
```

**Requirements**:
- json-escape binary must be built: `make build-json-escape`
- json-escape must be in PATH or discoverable in `target/release/` or `target/debug/`

**Building json-escape**:
```bash
# Build in release mode
make build-json-escape

# Or build manually
cargo build --release -p json-escape

# Add to PATH (optional)
export PATH="$PWD/target/release:$PATH"
```

**Verification**:
```bash
# Check if json-escape is available
command -v json-escape >/dev/null 2>&1 && echo "json-escape available" || echo "json-escape not found"

# Test json-escape
echo 'Hello "World"' | json-escape
# Output: Hello \"World\"
```

#### 3. Shell Fallback

**When Used**:
- Automatically when neither jq nor json-escape is available (in `auto` mode)
- Explicitly configured with `method = "shell"`
- Constrained environments without external dependencies

**Usage in Generated Scripts**:
```bash
# Example: Capture output with shell escaping
OUTPUT=$(command_to_run 2>&1)
ESCAPED_OUTPUT=$(printf '%s' "$OUTPUT" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\n/\\n/g')
echo "{\"output\": \"$ESCAPED_OUTPUT\"}" >> result.json
```

**Limitations**:
- Less robust than jq or json-escape
- May not handle all edge cases (binary data, complex Unicode)
- Relies on sed availability and BSD/GNU compatibility

**Best Practices**:
- Use only when jq and json-escape are unavailable
- Test generated output with JSON validators
- Prefer `auto` mode to avoid manual fallback selection

### Production Environment Requirements

#### jq Availability in Production

**MANDATORY for Production**: Production environments should have `jq` installed to ensure robust JSON output capture and avoid potential parsing errors.

**Why jq is Required**:
- **Reliability**: jq provides the most reliable JSON escaping for production workloads
- **Error Prevention**: Reduces risk of JSON parsing errors from improperly escaped output
- **Standard Compliance**: Ensures all JSON output is RFC 8259 compliant
- **Maintainability**: Simplifies troubleshooting and reduces edge case handling

**Installation Instructions**:

**Debian/Ubuntu**:
```bash
sudo apt-get update
sudo apt-get install -y jq
```

**RHEL/CentOS/Fedora**:
```bash
sudo yum install -y jq
# or
sudo dnf install -y jq
```

**macOS**:
```bash
brew install jq
```

**Alpine Linux** (Docker):
```dockerfile
RUN apk add --no-cache jq
```

**Docker Images**:
```dockerfile
# Add to Dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y jq && rm -rf /var/lib/apt/lists/*
```

**Verification**:
```bash
# Check jq installation
jq --version
# Expected output: jq-1.6 (or later)

# Verify jq is in PATH
which jq
# Expected output: /usr/bin/jq (or similar)
```

**CI/CD Integration**:
```yaml
# Example: GitLab CI
before_script:
  - apt-get update && apt-get install -y jq

# Example: GitHub Actions
- name: Install jq
  run: sudo apt-get install -y jq

# Example: Jenkins
sh 'apt-get update && apt-get install -y jq'
```

### Troubleshooting JSON Parsing Errors

#### Common Issue: Invalid Output Characters

**Symptoms**:
- JSON parsing errors: `Unexpected character`, `Invalid escape sequence`, `Unterminated string`
- Verification fails with JSON schema violations
- Container YAML contains malformed JSON strings
- Output contains unescaped special characters: `"`, `\`, newlines, tabs, control codes

**Example Error**:
```
Error: Failed to parse JSON output
  Unexpected character '"' at position 42
  JSON string not properly escaped
```

**Root Cause**:
Command output contains special characters that were not properly escaped for JSON string encoding.

**Characters That Require Escaping**:
| Character | JSON Escape | Description |
|-----------|-------------|-------------|
| `"` | `\"` | Double quote |
| `\` | `\\` | Backslash |
| `/` | `\/` | Forward slash (optional) |
| Newline (`\n`) | `\\n` | Line feed |
| Tab (`\t`) | `\\t` | Horizontal tab |
| Carriage return (`\r`) | `\\r` | Carriage return |
| Backspace (`\b`) | `\\b` | Backspace |
| Form feed (`\f`) | `\\f` | Form feed |
| Control codes (U+0000 to U+001F) | `\uXXXX` | Unicode escape sequences |

#### Diagnosis Steps

**Step 1: Identify the problematic output**
```bash
# Check execution log for the failing test step
cat execution_log.txt

# Look for JSON output sections
grep -A 10 "output" execution_log.txt
```

**Step 2: Check which escaping method was used**
```bash
# Review generated test script
cat generated_test_script.sh

# Look for output capture commands
grep -A 5 "ESCAPED_OUTPUT" generated_test_script.sh
```

**Step 3: Verify current configuration**
```bash
# Check configuration file
cat config.toml

# Look for json_escaping section
grep -A 3 "\[json_escaping\]" config.toml
```

**Step 4: Test escaping manually**
```bash
# Test problematic output with different methods
SAMPLE_OUTPUT='Line with "quotes" and \backslashes and
newlines'

# Test with jq
echo "$SAMPLE_OUTPUT" | jq -Rs .

# Test with json-escape
echo "$SAMPLE_OUTPUT" | json-escape

# Test with shell fallback
echo "$SAMPLE_OUTPUT" | sed 's/\\/\\\\/g; s/"/\\"/g'
```

#### Solutions

**Solution 1: Use jq (Recommended)**

Install jq and configure to use jq method:

```bash
# Install jq
sudo apt-get install -y jq  # Debian/Ubuntu
# or
brew install jq  # macOS

# Update configuration
cat > config.toml <<EOF
[json_escaping]
method = "jq"
EOF

# Regenerate test script
test-executor generate testcases/TC_001.yaml --config config.toml

# Re-execute test
./generated_test_script.sh
```

**Solution 2: Use json-escape binary**

Build and configure json-escape:

```bash
# Build json-escape
make build-json-escape

# Add to PATH
export PATH="$PWD/target/release:$PATH"

# Update configuration
cat > config.toml <<EOF
[json_escaping]
method = "json-escape"
EOF

# Regenerate and execute
test-executor generate testcases/TC_001.yaml --config config.toml
./generated_test_script.sh
```

**Solution 3: Use auto mode (Default)**

Let the framework automatically select the best available method:

```bash
# Update configuration
cat > config.toml <<EOF
[json_escaping]
method = "auto"
EOF

# Framework will try: jq > json-escape > shell fallback
test-executor generate testcases/TC_001.yaml --config config.toml
```

**Solution 4: Validate JSON output**

After fixing escaping, validate the JSON output:

```bash
# Extract JSON from output file
cat result.json | jq .

# If jq parses successfully, escaping is correct
# If jq fails, escaping is still incorrect
```

#### Prevention Best Practices

**1. Always Use Auto Mode or jq in Production**
```toml
[json_escaping]
method = "auto"  # or "jq" for production
```

**2. Install jq in All Environments**
```bash
# Add to Dockerfile
RUN apt-get update && apt-get install -y jq

# Add to CI/CD setup scripts
sudo apt-get install -y jq
```

**3. Test with Complex Output**

Test your test cases with output containing special characters:

```yaml
sequences:
  - sequence_id: 1
    steps:
      - description: Test special characters
        command: |
          cat <<'EOF'
          Line with "quotes"
          Line with \backslashes
          Line with 	tabs
          Line with $variables
          EOF
        expected:
          exit_code: 0
```

**4. Validate Generated JSON**

Always validate JSON output before using in production:

```bash
# After test execution
cat verification_result.json | jq . > /dev/null && echo "Valid JSON" || echo "Invalid JSON"
```

**5. Monitor Escaping Method Usage**

Log which escaping method is being used:

```bash
# Check generated script for escaping method
grep -o "jq\|json-escape\|sed.*\\\\\\\\g" generated_test_script.sh | head -1
```

#### Edge Cases and Special Considerations

**Binary Data**: 
- jq handles binary data by converting to base64 or escaping as Unicode
- json-escape filters non-UTF-8 sequences
- Shell fallback may produce invalid results

**Large Output** (> 1MB):
- All methods handle large output efficiently
- jq may use more memory for very large inputs
- json-escape is optimized for streaming

**Null Bytes** (`\0`):
- jq escapes as `\u0000`
- json-escape filters null bytes
- Shell fallback may truncate at null byte

**Invalid UTF-8**:
- jq replaces invalid sequences with replacement character
- json-escape filters invalid UTF-8
- Shell fallback may corrupt data

### Migration Guide

#### Migrating to jq

If you're currently using `json-escape` or `shell` method and want to migrate to jq:

**Step 1: Install jq in all environments**
```bash
# Development environment
sudo apt-get install -y jq

# Update CI/CD configuration
# Update Dockerfile
# Update deployment scripts
```

**Step 2: Update configuration files**
```toml
# Before
[json_escaping]
method = "json-escape"

# After
[json_escaping]
method = "jq"
```

**Step 3: Regenerate all test scripts**
```bash
# Batch regeneration
find testcases/ -name "*.yaml" -type f -exec \
  test-executor generate {} --config config.toml \;
```

**Step 4: Validate changes**
```bash
# Run tests and verify output
make test-e2e

# Check JSON output validity
find test-results/ -name "*.json" -exec sh -c 'jq . {} > /dev/null' \;
```

**Step 5: Update documentation**
- Update deployment guides to include jq installation
- Update CI/CD documentation
- Update troubleshooting guides

### Examples

#### Example 1: Configuration with jq

**config.toml**:
```toml
[json_escaping]
method = "jq"
```

**Generated script snippet**:
```bash
# Capture command output with jq escaping
STEP_OUTPUT=$(echo "Testing jq escaping" 2>&1)
ESCAPED_OUTPUT=$(printf '%s' "$STEP_OUTPUT" | jq -Rs .)
echo "{\"output\": $ESCAPED_OUTPUT, \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" >> result.json
```

#### Example 2: Configuration with auto mode

**config.toml**:
```toml
[json_escaping]
method = "auto"
```

**Generated script snippet**:
```bash
# Auto-detect best escaping method
if command -v jq >/dev/null 2>&1; then
    ESCAPED_OUTPUT=$(printf '%s' "$STEP_OUTPUT" | jq -Rs .)
elif command -v json-escape >/dev/null 2>&1; then
    ESCAPED_OUTPUT=$(printf '%s' "$STEP_OUTPUT" | json-escape)
else
    # Shell fallback
    ESCAPED_OUTPUT=$(printf '%s' "$STEP_OUTPUT" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g')
fi
```

#### Example 3: Testing different methods

**test-json-escaping.sh**:
```bash
#!/usr/bin/env bash
set -e

# Sample output with special characters
SAMPLE='Line with "quotes" and \backslashes and
newlines and 	tabs'

echo "Testing JSON escaping methods:"
echo "=============================="

# Test jq
echo -e "\n1. jq method:"
printf '%s' "$SAMPLE" | jq -Rs .

# Test json-escape
echo -e "\n2. json-escape method:"
printf '%s' "$SAMPLE" | json-escape

# Test shell fallback
echo -e "\n3. shell fallback method:"
printf '%s' "$SAMPLE" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\n/\\n/g'

echo -e "\n=============================="
echo "All methods tested"
```

### Reference

#### Configuration Schema

```toml
[json_escaping]
# Method selection: "auto", "jq", "json-escape", or "shell"
# Default: "auto"
method = "auto"
```

#### Method Selection Logic (Auto Mode)

```
1. Check if jq is available in PATH
   YES → Use jq
   NO → Continue to step 2

2. Check if json-escape binary is available in PATH or target/
   YES → Use json-escape
   NO → Continue to step 3

3. Use shell fallback (always available)
```

#### Command Reference

| Operation | Command |
|-----------|---------|
| Install jq (Debian/Ubuntu) | `sudo apt-get install -y jq` |
| Install jq (macOS) | `brew install jq` |
| Build json-escape | `make build-json-escape` |
| Test jq escaping | `echo 'test' \| jq -Rs .` |
| Test json-escape | `echo 'test' \| json-escape` |
| Validate JSON output | `cat output.json \| jq .` |
| Check jq availability | `command -v jq` |
| Check json-escape availability | `command -v json-escape` |

## Shell Script Compatibility

**MANDATORY**: All shell scripts and generated bash scripts must be compatible with both BSD and GNU variants of command-line tools, and must work with bash 3.2+ (the default on macOS).

### Key Requirements:
- Scripts must work on macOS (BSD) and Linux (GNU) without modification
- Scripts must be compatible with bash 3.2+ (macOS ships with bash 3.2 by default)
- Avoid GNU-specific flags or options that don't exist in BSD variants
- Avoid bash 4.0+ features like associative arrays (`declare -A`)
- Test commands like `sed`, `grep`, `awk`, `find`, etc. must use portable syntax
- When using regex, ensure patterns are compatible with both POSIX and GNU extended regex
- Use POSIX-compliant shell constructs where possible

### Logging Library

**MANDATORY**: All shell scripts must use the centralized logging library for consistent output formatting.

**Location**: `scripts/lib/logger.sh`

**Usage**:
```bash
#!/usr/bin/env bash
set -e

# Get script directory and source logger
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

# Use logging functions
log_info "Informational message"
log_warning "Warning message"
log_error "Error message"
log_debug "Debug message (only shown if VERBOSE=1)"
log_verbose "Verbose message (only shown if VERBOSE=1)"

# Use color-coded test helpers
pass "Test passed"
fail "Test failed"
info "Information"
section "Section Header"
```

**Available Functions**:
- `log_info "message"` - Standard informational message
- `log_warning "message"` - Warning message
- `log_error "message"` - Error message (outputs to stderr)
- `log_debug "message"` - Debug message (only shown when VERBOSE=1)
- `log_verbose "message"` - Verbose message (only shown when VERBOSE=1)
- `pass "message"` - Success message with green checkmark (✓)
- `fail "message"` - Failure message with red X (✗)
- `info "message"` - Info message with blue info symbol (ℹ)
- `section "title"` - Section header with yellow highlighting

**Cleanup Management**:
The logger library also provides cleanup management for temporary files and background processes:
- `setup_cleanup "/path/to/temp/dir"` - Register temporary directory for cleanup
- `register_background_pid $PID` - Register background process for cleanup
- `disable_cleanup` - Disable automatic cleanup (for debugging)
- `enable_cleanup` - Re-enable automatic cleanup

**Benefits**:
- Consistent formatting across all scripts
- Color-coded output for better readability
- Automatic cleanup of temporary resources
- Easy integration with CI/CD pipelines

### Common Pitfalls:
- `grep -P` (Perl regex) is GNU-only - use `sed -n` with capture groups instead
- `sed -r` is GNU-only - use `sed -E` for BSD/macOS compatibility
- `date` formatting differs between BSD and GNU
- `readlink -f` is GNU-only - use alternative methods for BSD
- `declare -A` (associative arrays) requires bash 4.0+ - use eval with dynamic variable names for bash 3.2+

### Testing:
- Test generated scripts on both macOS and Linux when possible
- Use portable regex patterns that work with both implementations
- Verify scripts work with bash 3.2 (default on macOS)
- Verify script syntax using `make verify-scripts`

## YAML Validation

All YAML files in this project must include required metadata fields and are automatically validated against their JSON schemas before parsing. This section explains the validation system, how to use it in code, and how to troubleshoot validation errors.

### Required YAML Fields

**MANDATORY**: Every YAML file in this project must include two required fields at the root level:

1. **`type`** - Identifies the type of YAML content (e.g., `"test-case"`, `"verification-result"`, `"container"`)
2. **`schema`** - Specifies the JSON schema URI for validation (e.g., `"tcms/test-case.schema.v1.json"`)

**Example:**
```yaml
type: test-case
schema: tcms/test-case.schema.v1.json
test_case_id: TC_EXAMPLE_001
title: Example Test Case
# ... rest of test case content
```

**Why These Fields Are Required:**
- **Schema Resolution**: The `schema` field enables automatic schema resolution without manual specification
- **Type Safety**: The `type` field provides explicit type identification for documentation and tooling
- **Validation**: Files are validated against their schema before parsing to catch errors early
- **Consistency**: Ensures all YAML files follow a consistent structure across the project

### Automatic Schema Validation

YAML files are automatically validated against their schema **before** parsing in all project binaries and libraries:

**Validation Process:**
1. Read YAML file content
2. Extract `schema` field from YAML
3. Resolve schema path from URI (e.g., `tcms/test-case.schema.v1.json` → `schemas/test-case.schema.json`)
4. Validate YAML content against resolved JSON schema
5. If validation passes, deserialize YAML to Rust struct
6. If validation fails, return detailed error with constraint violations

**Benefits:**
- **Early Error Detection**: Schema violations are caught before parsing
- **Detailed Error Messages**: Validation errors show exact path and constraint that failed
- **Type Safety**: Ensures YAML structure matches expected data model
- **Automatic**: No manual validation code needed in application logic

### Using `testcase_common::load_and_validate_yaml`

**MANDATORY**: All new code that loads YAML files must use the `testcase_common::load_and_validate_yaml` function for automatic schema validation and deserialization.

#### Basic Usage

```rust
use testcase_common::load_and_validate_yaml;
use testcase_models::TestCase;

// Load and validate a test case YAML file
let test_case: TestCase = load_and_validate_yaml("testcases/TC_001.yaml", "schemas/")?;

// File is automatically:
// 1. Read from disk
// 2. Schema resolved from 'schema' field
// 3. Validated against schema
// 4. Deserialized to TestCase struct
```

#### Function Signature

```rust
pub fn load_and_validate_yaml<T, P, S>(file_path: P, schemas_root: S) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
    S: AsRef<str>,
```

**Parameters:**
- `file_path` - Path to YAML file (relative or absolute)
- `schemas_root` - Root directory containing schema files (usually `"schemas/"`)

**Returns:**
- `Ok(T)` - Successfully validated and deserialized object
- `Err(anyhow::Error)` - Error with context about what failed

**Type Parameter:**
- `T` - Target Rust struct implementing `serde::Deserialize`

#### Loading Different YAML Types

```rust
use testcase_common::load_and_validate_yaml;
use testcase_models::{TestCase, VerificationResult};

// Load test case
let test_case: TestCase = load_and_validate_yaml(
    "testcases/TC_001.yaml",
    "schemas/"
)?;

// Load verification result
let result: VerificationResult = load_and_validate_yaml(
    "results/verification_001.yaml",
    "schemas/"
)?;

// Load configuration
use testcase_common::Config;
let config: Config = load_and_validate_yaml(
    "config.yaml",
    "schemas/"
)?;
```

#### Error Handling

```rust
use testcase_common::load_and_validate_yaml;
use testcase_models::TestCase;

match load_and_validate_yaml::<TestCase, _, _>("test.yaml", "schemas/") {
    Ok(test_case) => {
        println!("Loaded test case: {}", test_case.test_case_id);
        // Process test case
    }
    Err(e) => {
        eprintln!("Failed to load test case: {}", e);
        // Error contains full context:
        // - File read errors
        // - Schema resolution errors
        // - Schema validation errors with constraint violations
        // - YAML parsing errors with line numbers
        return Err(e);
    }
}
```

#### Using `parse_and_validate_yaml_string`

For YAML content from sources other than files (e.g., editor buffers, network responses):

```rust
use testcase_common::parse_and_validate_yaml_string;
use testcase_models::TestCase;

let yaml_content = r#"
type: test-case
schema: tcms/test-case.schema.v1.json
test_case_id: TC_001
title: Example Test
sequences: []
"#;

let test_case: TestCase = parse_and_validate_yaml_string(
    yaml_content,
    "schemas/",
    "editor buffer"  // Source name for error messages
)?;
```

#### Integration in Binaries

All project binaries use `load_and_validate_yaml`:

**test-executor:**
```rust
// Load test case with automatic validation
let test_case: TestCase = load_and_validate_yaml(&yaml_file, "schemas/")?;
```

**verifier:**
```rust
// Load verification result with automatic validation
let result: VerificationResult = load_and_validate_yaml(&log_file, "schemas/")?;
```

**validate-yaml binary:**
```rust
// Uses internal validation for detailed error reporting
let validator = YamlValidator::new();
validator.validate_file(&yaml_file, &schema_path)?;
```

### Manual Validation with `validate-yaml` Binary

The `validate-yaml` binary provides command-line YAML validation with detailed error reporting.

#### Basic Usage

```bash
# Validate single YAML file (auto-resolves schema from 'schema' field)
cargo run -p validate-yaml -- testcases/TC_001.yaml

# Validate multiple YAML files
cargo run -p validate-yaml -- testcases/*.yaml

# Validate with explicit schema (overrides 'schema' field)
cargo run -p validate-yaml -- testcases/TC_001.yaml --schema schemas/test-case.schema.json

# Specify custom schemas root directory
cargo run -p validate-yaml -- testcases/TC_001.yaml --schemas-root /custom/schemas/
```

#### Output Format

**Successful validation:**
```
✓ testcases/TC_001.yaml (schema: schemas/test-case.schema.json)
✓ testcases/TC_002.yaml (schema: schemas/test-case.schema.json)

Summary:
  Total files validated: 2
  Passed: 2
  Failed: 0
```

**Failed validation:**
```
✗ testcases/TC_001.yaml
  Schema constraint violations:
    Error #1: Path '/sequences/0/steps/0/expected/exit_code'
      Constraint: "0" is not of type "integer"
      Found value: "0"

Summary:
  Total files validated: 1
  Passed: 0
  Failed: 1
```

#### Watch Mode

Monitor YAML files for changes and auto-validate (Linux/macOS only):

```bash
# Watch single file
cargo run -p validate-yaml -- testcases/TC_001.yaml --watch

# Watch multiple files
cargo run -p validate-yaml -- testcases/*.yaml --watch
```

**Watch mode features:**
- Monitors YAML files for changes
- Monitors schema files (including transitive dependencies via `$ref`)
- Debounces rapid changes (300ms)
- Shows which files changed
- Re-validates automatically on file save

#### Verbose Output

Enable detailed logging for debugging:

```bash
# Verbose mode (info level)
cargo run -p validate-yaml -- testcases/TC_001.yaml --verbose

# Custom log level
cargo run -p validate-yaml -- testcases/TC_001.yaml --log-level debug

# Trace level for maximum detail
cargo run -p validate-yaml -- testcases/TC_001.yaml --log-level trace
```

#### Batch Validation

Validate all test case YAML files and generate a detailed report:

```bash
# Generate validation report for all test cases
make validate-testcases-report

# View the report
cat reports/validation_report.txt
```

See **Test Case Validation Report** section below for details.

### Troubleshooting Validation Errors

#### Common Validation Errors

**1. Missing Required Field: `schema`**

**Error:**
```
Failed to resolve schema for file: testcases/TC_001.yaml
```

**Cause:** YAML file is missing the required `schema` field.

**Solution:**
```yaml
# Add schema field at root level
schema: tcms/test-case.schema.v1.json
type: test-case
test_case_id: TC_001
# ... rest of content
```

**2. Missing Required Field: `type`**

**Error:**
```
Schema constraint violations:
  Error #1: Path 'root'
    Constraint: 'type' is a required property
    Found value: {...}
```

**Cause:** YAML file is missing the required `type` field.

**Solution:**
```yaml
# Add type field at root level
type: test-case
schema: tcms/test-case.schema.v1.json
test_case_id: TC_001
# ... rest of content
```

**3. Schema File Not Found**

**Error:**
```
Schema file not found: schemas/test-case.schema.json (resolved from URI: 'tcms/test-case.schema.v1.json')
```

**Cause:** Schema URI in YAML file cannot be resolved to an existing schema file.

**Solution - Check schema URI:**
```yaml
# Incorrect URI
schema: tcms/wrong-name.schema.v1.json  # ✗ Wrong

# Correct URI
schema: tcms/test-case.schema.v1.json   # ✓ Correct
```

**Solution - Check schemas directory:**
```bash
# Verify schema file exists
ls -la schemas/test-case.schema.json

# If missing, check schemas directory structure
tree schemas/
```

**4. Invalid Data Type**

**Error:**
```
Schema constraint violations:
  Error #1: Path '/sequences/0/steps/0/expected/exit_code'
    Constraint: "0" is not of type "integer"
    Found value: "0"
```

**Cause:** Field has wrong data type (string instead of integer).

**Solution:**
```yaml
# Incorrect (string)
expected:
  exit_code: "0"  # ✗ Wrong - quoted

# Correct (integer)
expected:
  exit_code: 0    # ✓ Correct - unquoted
```

**5. Missing Required Property**

**Error:**
```
Schema constraint violations:
  Error #1: Path '/sequences/0/steps/0'
    Constraint: 'command' is a required property
    Found value: {"description": "Test step", "expected": {...}}
```

**Cause:** Required field is missing from object.

**Solution:**
```yaml
steps:
  - description: Test step
    command: echo "Hello"  # ✓ Add required field
    expected:
      exit_code: 0
```

**6. Invalid Enum Value**

**Error:**
```
Schema constraint violations:
  Error #1: Path '/prerequisites/0/type'
    Constraint: "invalid_type" is not one of ["manual", "automatic"]
    Found value: "invalid_type"
```

**Cause:** Field value is not one of the allowed enum values.

**Solution:**
```yaml
prerequisites:
  - type: manual      # ✓ Use valid enum value
    description: Install required software
```

**7. Array Length Constraint Violation**

**Error:**
```
Schema constraint violations:
  Error #1: Path '/sequences'
    Constraint: [] should have at least 1 item
    Found value: []
```

**Cause:** Array must have minimum number of items but is empty.

**Solution:**
```yaml
sequences:
  - sequence_id: 1   # ✓ Add at least one sequence
    name: Test Sequence
    steps: []
```

**8. Pattern/Regex Mismatch**

**Error:**
```
Schema constraint violations:
  Error #1: Path '/test_case_id'
    Constraint: "invalid id" does not match pattern "^[A-Z][A-Z0-9_]*$"
    Found value: "invalid id"
```

**Cause:** String value doesn't match required regex pattern.

**Solution:**
```yaml
# Incorrect (spaces, lowercase)
test_case_id: "invalid id"     # ✗ Wrong

# Correct (uppercase, underscores)
test_case_id: TC_EXAMPLE_001   # ✓ Correct
```

#### Schema Path Resolution

The validation system resolves schema URIs to file paths using these rules:

**URI Format:** `namespace/schema-name.schema.version.json`

**Examples:**
- `tcms/test-case.schema.v1.json` → `schemas/test-case.schema.json`
- `tcms/verification-result.schema.v1.json` → `schemas/verification-result.schema.json`
- `tcms/container.schema.v1.json` → `schemas/container.schema.json`

**Resolution Process:**
1. Extract schema name from URI (remove namespace and version)
2. Append to schemas root directory
3. Verify file exists
4. Return absolute path to schema file

**Custom Schemas Root:**
```bash
# Use custom schemas directory
cargo run -p validate-yaml -- test.yaml --schemas-root /custom/path/schemas/
```

**Debugging Schema Resolution:**
```bash
# Enable debug logging to see resolved paths
cargo run -p validate-yaml -- test.yaml --log-level debug

# Output shows:
# DEBUG Resolved schema 'tcms/test-case.schema.v1.json' for file 'test.yaml' to 'schemas/test-case.schema.json'
```

#### Debugging Validation Failures

**Step 1: Enable verbose output**
```bash
cargo run -p validate-yaml -- testcases/TC_001.yaml --verbose
```

**Step 2: Check schema file exists**
```bash
# Extract schema URI from YAML
grep "^schema:" testcases/TC_001.yaml

# Verify schema file exists
ls -la schemas/test-case.schema.json
```

**Step 3: Validate schema file is valid JSON**
```bash
# Parse schema JSON
cat schemas/test-case.schema.json | jq .

# If jq fails, schema JSON is malformed
```

**Step 4: Compare YAML structure to schema**
```bash
# View schema requirements
cat schemas/test-case.schema.json | jq '.required'
cat schemas/test-case.schema.json | jq '.properties'

# View YAML structure
cat testcases/TC_001.yaml
```

**Step 5: Test with minimal valid YAML**
```bash
# Create minimal YAML matching schema
cat > /tmp/minimal.yaml <<EOF
type: test-case
schema: tcms/test-case.schema.v1.json
test_case_id: TC_MINIMAL
title: Minimal Test
sequences: []
EOF

# Validate minimal YAML
cargo run -p validate-yaml -- /tmp/minimal.yaml --verbose

# If minimal YAML passes, incrementally add fields from failing YAML
```

**Step 6: Use JSON schema validators**
```bash
# Convert YAML to JSON
cat testcases/TC_001.yaml | yq -o json > /tmp/test.json

# Validate with external tool (if available)
jsonschema -i /tmp/test.json schemas/test-case.schema.json
```

#### Common Schema Violations

| Violation | Description | Fix |
|-----------|-------------|-----|
| Missing `type` field | Root-level `type` field not present | Add `type: test-case` |
| Missing `schema` field | Root-level `schema` field not present | Add `schema: tcms/test-case.schema.v1.json` |
| Wrong data type | String instead of integer, etc. | Remove quotes from numbers/booleans |
| Missing required property | Required field not in object | Add missing field with valid value |
| Invalid enum value | Value not in allowed list | Use one of the allowed enum values |
| Empty array | Array must have min items | Add at least one item to array |
| Invalid pattern | String doesn't match regex | Fix string format to match pattern |
| Extra properties | Schema has `additionalProperties: false` | Remove unknown properties |

#### Getting Help

If validation errors persist after troubleshooting:

1. **Check Schema Documentation**
   - Review schema file: `schemas/test-case.schema.json`
   - Check schema comments and descriptions
   - Look at `required` and `properties` sections

2. **Review Example Files**
   - Valid examples: `testcases/examples/`
   - Verifier scenarios: `testcases/verifier-scenarios/`
   - Copy structure from working examples

3. **Use Validation Report**
   - Generate full report: `make validate-testcases-report`
   - Compare passing and failing files
   - Identify common patterns in errors

4. **Check Recent Changes**
   - Review schema changes in git history
   - Check if schema was recently updated
   - Verify YAML files are compatible with new schema

5. **Enable Maximum Logging**
   ```bash
   cargo run -p validate-yaml -- file.yaml --log-level trace
   ```

## Schema Validation

The project includes comprehensive schema validation for both test case inputs and verification outputs.

### Output Schema Validation

Expected output samples in `testcases/examples/expected_test_results/` are validated against their respective JSON schemas to ensure correctness and consistency.

**Command**: `make validate-output-schemas`

**Requirements**:
- Python 3 with `pyyaml` and `jsonschema` modules
- Install with: `pip3 install pyyaml jsonschema`

**Validated Files**:
1. **Test Case Results** (`test_case_result/*.yml`):
   - Validated against `test_case_result/schema.json`
   - Individual test case verification results
   - Contains sequences, steps, pass/fail status, and metadata

2. **Container Files** (`container/*.yml`):
   - Validated against `container/container_schema.json`
   - Aggregated test results with rich metadata
   - Multiple test case results in a single container

**Integration**: The `validate-output-schemas` target is automatically included in the `test-e2e` target, ensuring schema compliance is tested in CI.

**Manual Validation**:
```bash
# Validate all expected output samples
make validate-output-schemas

# Or run the script directly
./scripts/validate-output-schemas.sh
```

**Schema Files**:
- `schemas/verification-result.schema.json` - Individual test case verification result schema
- `schemas/container_config.schema.json` - Container configuration metadata schema
- `schemas/verification-output.schema.json` - Complete verification output schema
- `schemas/execution-log.schema.json` - Test execution log schema
- `schemas/test-case.schema.json` - Test case definition schema

### Test Case Validation Report

The project provides a comprehensive validation reporting tool that validates all test case YAML files and generates a detailed report.

**Command**: `make validate-testcases-report`

**Purpose**:
- Validates all test case YAML files in the `testcases/` directory against the JSON schema
- Generates a detailed validation report with file-by-file results
- Provides summary statistics showing total files, valid files, and invalid files
- Helps identify schema compliance issues across the entire test suite

**Output Location**: `reports/validation_report.txt`

**Usage**:
```bash
# Generate validation report for all test cases
make validate-testcases-report

# View the generated report
cat reports/validation_report.txt
```

**Report Contents**:
The validation report includes:
1. **Header** - Report title and timestamp
2. **File-by-File Results** - For each test case YAML file:
   - File path relative to project root
   - Validation status (✓ VALID or ✗ INVALID)
   - Detailed error messages for invalid files (schema violations, parsing errors)
3. **Summary Statistics**:
   - Total number of YAML files validated
   - Number of valid files
   - Number of invalid files
   - Overall validation success rate

**Interpreting the Report**:
- **Valid Files**: Files marked with `✓ VALID` comply with the test case schema and are ready for execution
- **Invalid Files**: Files marked with `✗ INVALID` have schema violations or syntax errors that must be fixed
- **Error Messages**: Detailed error messages indicate the specific schema property or constraint that was violated
- **Summary**: The summary section provides a quick overview of validation health across the entire test suite

**Example Report Format**:
```
========================================
Test Case Validation Report
Generated: 2024-01-15 14:30:00
========================================

Validating YAML files in: testcases/

File: testcases/verifier-scenarios/TEST_SUCCESS_001.yaml
Status: ✓ VALID

File: testcases/examples/TC_EXAMPLE_001.yaml
Status: ✗ INVALID
Errors:
  - Missing required property: 'test_case_id'
  - Invalid value for 'sequences[0].steps[0].expected.exit_code': expected integer, got string

========================================
Summary
========================================
Total files validated: 42
Valid files: 40
Invalid files: 2
Success rate: 95.24%
```

**Integration with Workflow**:
- Run this command before committing changes to test case YAML files
- Use in CI/CD pipelines to catch schema violations early
- Helps maintain consistency and quality across all test cases
- Complements the `validate-output-schemas` command which validates verification output

**Common Validation Errors**:
- Missing required properties (test_case_id, title, sequences)
- Invalid data types (string instead of integer for exit_code)
- Invalid enum values (unknown prerequisite types)
- Malformed YAML syntax (indentation, quotes, special characters)
- Schema constraint violations (empty sequences, invalid regex patterns)

**Fixing Validation Errors**:
1. Locate the invalid file in the report
2. Review the specific error messages
3. Consult the test case schema: `schemas/test-case.schema.json`
4. Make corrections to the YAML file
5. Re-run validation to confirm fixes: `make validate-testcases-report`

## Testing Requirements

**MANDATORY**: All agents must run the full test suite before considering any task complete. Testing is a critical step that cannot be skipped.

### Test Execution
- Run tests using: `cargo test --all-features`
- This ensures comprehensive validation across the entire codebase with all feature flags enabled
- Alternative basic test command: `cargo test`

### Test Requirements
- **All tests must pass** before any code changes can be committed
- If tests fail, investigate and fix the failures before proceeding
- Never commit code with failing tests
- Update or add tests as needed when modifying functionality

## Documentation Generation Coverage

The project includes a specialized coverage reporting tool for analyzing code coverage of the documentation generation workflow.

### Coverage Report Generation

**Command**: `make generate-docs-coverage`

This command executes cargo-tarpaulin across all document generation code paths exercised by sample test cases, generating:
- Coverage report showing which functions and branches were executed
- Total coverage percentage for documentation-related modules
- Detailed HTML and JSON reports (optional)

### Modules Tracked

The coverage analysis focuses on:
- `src/lib.rs` - Library exports
- `src/verification.rs` - Verification and report generation
- `src/verification_templates.rs` - Template rendering
- `src/parser.rs` - YAML parsing
- `src/models.rs` - Data models
- `src/bin/verifier.rs` - Verifier binary
- `src/bin/test-plan-documentation-generator-compat.rs` - Documentation generator compatibility

### Usage

**Basic Coverage Report**:
```bash
make generate-docs-coverage
```

**With HTML Report**:
```bash
./scripts/generate_documentation_coverage_report.sh --html
```

**Custom Output Directory**:
```bash
./scripts/generate_documentation_coverage_report.sh --output-dir /path/to/reports
```

### Output Files

Reports are generated in `reports/coverage/documentation/` (default):
- `tarpaulin-report.json` - Coverage data in JSON format
- `coverage_summary.txt` - Human-readable coverage summary
- `coverage_run.log` - Detailed execution log
- `html/` - HTML coverage report (if `--html` flag used)

### Coverage Workflow

The tool automatically:
1. Checks for and installs cargo-tarpaulin if needed
2. Builds verifier and documentation generator binaries
3. Runs sample test scenarios under coverage instrumentation
4. Processes verification logs and container YAML files
5. Generates comprehensive coverage reports
6. Prints total coverage percentage to stdout

### Sample Scenarios

Coverage analysis runs against these test scenarios:
- `TEST_SUCCESS_001` - Successful test execution
- `TEST_FAILED_FIRST_001` - Failed first step scenario
- `TEST_MULTI_SEQ_001` - Multiple sequences scenario

## Hooks

Hooks provide optional extensibility points in the test execution lifecycle, enabling custom setup, teardown, logging, and resource management. **Hooks are entirely optional** - all test cases function normally without defining any hooks.

### Overview

Hooks allow you to inject custom scripts at eight different points in the test execution lifecycle:

1. **script_start** - Executes once at the very beginning of the generated test script
2. **setup_test** - Executes once after script_start, before any test sequences run
3. **before_sequence** - Executes before each test sequence starts
4. **after_sequence** - Executes after each test sequence completes
5. **before_step** - Executes before each test step
6. **after_step** - Executes after each test step completes
7. **teardown_test** - Executes once after all test sequences, before script_end
8. **script_end** - Executes once at the very end of the test script

### Configuration

Hooks are defined in the test case YAML under the `hooks` key:

```yaml
hooks:
  script_start:
    command: "scripts/script_start.sh"
    on_error: "fail"
  setup_test:
    command: "scripts/setup_test.sh"
  before_sequence:
    command: "scripts/before_sequence.sh"
  after_sequence:
    command: "scripts/after_sequence.sh"
  before_step:
    command: "scripts/before_step.sh"
  after_step:
    command: "scripts/after_step.sh"
    on_error: "continue"
  teardown_test:
    command: "scripts/teardown_test.sh"
  script_end:
    command: "scripts/script_end.sh"
```

#### Hook Configuration Fields

- **command** (required): Path to the script or command to execute. Paths are relative to the test case YAML file location, or can be absolute paths.
- **on_error** (optional): Error handling behavior when the hook fails
  - `fail` (default): Test execution stops immediately if the hook fails
  - `continue`: Hook errors are logged but test execution continues

### Available Environment Variables

Hooks have access to the test execution context through environment variables:

#### All Hooks
- Standard environment variables from the test execution context
- Any environment variables defined in the test case's `hydration_vars`

#### before_sequence and after_sequence
- `TEST_SEQUENCE_ID`: The sequence ID (e.g., "1", "2")
- `TEST_SEQUENCE_NAME`: The sequence name

#### before_step and after_step
- `TEST_SEQUENCE_ID`: The sequence ID
- `TEST_STEP_NUMBER`: The step number
- `TEST_STEP_DESCRIPTION`: The step description
- All sequence-scoped variables defined in the test sequence
- All captured variables from previous steps (in after_step)

#### after_step only
- `STEP_EXIT_CODE`: The exit code of the step command
- `COMMAND_OUTPUT`: The output from the step command

### Common Use Cases

#### 1. Test Environment Setup

Create temporary directories and initialize resources:

```yaml
hooks:
  setup_test:
    command: "scripts/setup_environment.sh"
  teardown_test:
    command: "scripts/cleanup_environment.sh"
```

**setup_environment.sh:**
```bash
#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

# Create test workspace
TEST_WORKSPACE="/tmp/test_workspace_$$"
mkdir -p "$TEST_WORKSPACE"
echo "$TEST_WORKSPACE" > /tmp/test_workspace_path.txt

log_info "Created test workspace: $TEST_WORKSPACE"

# Initialize test database
log_info "Initializing test database..."
sqlite3 "$TEST_WORKSPACE/test.db" "CREATE TABLE tests (id INTEGER, name TEXT);"

log_info "Test environment setup complete"
```

**cleanup_environment.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

# Read workspace path
if [ -f /tmp/test_workspace_path.txt ]; then
    TEST_WORKSPACE=$(cat /tmp/test_workspace_path.txt)
    if [ -d "$TEST_WORKSPACE" ]; then
        rm -rf "$TEST_WORKSPACE"
        log_info "Removed test workspace: $TEST_WORKSPACE"
    fi
    rm -f /tmp/test_workspace_path.txt
fi

log_info "Test environment cleanup complete"
```

#### 2. Custom Logging

Track test execution with detailed logging:

```yaml
hooks:
  script_start:
    command: "scripts/log_start.sh"
  before_sequence:
    command: "scripts/log_sequence_start.sh"
  after_step:
    command: "scripts/log_step_result.sh"
  script_end:
    command: "scripts/log_completion.sh"
```

**log_start.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

START_TIME=$(date +%s)
echo "$START_TIME" > /tmp/test_start_time.txt

log_info "========================================="
log_info "Test Execution Started: $(date)"
log_info "========================================="
```

**log_sequence_start.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-unknown}"
SEQUENCE_NAME="${TEST_SEQUENCE_NAME:-unknown}"

section "Sequence $SEQUENCE_ID: $SEQUENCE_NAME"
log_info "Starting test sequence: $SEQUENCE_NAME (ID: $SEQUENCE_ID)"
```

**log_step_result.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

STEP_NUMBER="${TEST_STEP_NUMBER:-unknown}"
EXIT_CODE="${STEP_EXIT_CODE:-unknown}"

if [ "$EXIT_CODE" = "0" ]; then
    pass "Step $STEP_NUMBER completed successfully"
else
    fail "Step $STEP_NUMBER failed with exit code: $EXIT_CODE"
fi
```

**log_completion.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

if [ -f /tmp/test_start_time.txt ]; then
    START_TIME=$(cat /tmp/test_start_time.txt)
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    
    log_info "========================================="
    log_info "Test Execution Completed: $(date)"
    log_info "Total Duration: ${DURATION}s"
    log_info "========================================="
    
    rm -f /tmp/test_start_time.txt
fi
```

#### 3. Resource Cleanup

Ensure proper cleanup even when tests fail:

```yaml
hooks:
  before_sequence:
    command: "scripts/allocate_resources.sh"
  after_sequence:
    command: "scripts/release_resources.sh"
    on_error: "continue"  # Always try to clean up
```

**allocate_resources.sh:**
```bash
#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-1}"
RESOURCE_DIR="/tmp/test_resources_seq_${SEQUENCE_ID}"

mkdir -p "$RESOURCE_DIR"
echo "$RESOURCE_DIR" > "/tmp/resource_dir_seq_${SEQUENCE_ID}.txt"

# Allocate test resources
log_info "Allocated resources for sequence $SEQUENCE_ID: $RESOURCE_DIR"

# Create lock file to track active resources
echo "$$" > "$RESOURCE_DIR/lock"
```

**release_resources.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-1}"
RESOURCE_FILE="/tmp/resource_dir_seq_${SEQUENCE_ID}.txt"

if [ -f "$RESOURCE_FILE" ]; then
    RESOURCE_DIR=$(cat "$RESOURCE_FILE")
    if [ -d "$RESOURCE_DIR" ]; then
        rm -rf "$RESOURCE_DIR"
        log_info "Released resources for sequence $SEQUENCE_ID"
    fi
    rm -f "$RESOURCE_FILE"
fi
```

#### 4. Integration with External Systems

Connect to external monitoring or reporting systems:

```yaml
hooks:
  script_start:
    command: "scripts/notify_test_start.sh"
    on_error: "continue"  # Don't fail if monitoring unavailable
  after_step:
    command: "scripts/report_step_metrics.sh"
    on_error: "continue"
  script_end:
    command: "scripts/notify_test_complete.sh"
    on_error: "continue"
```

**notify_test_start.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

TEST_ID="${TEST_CASE_ID:-unknown}"
MONITORING_URL="${MONITORING_ENDPOINT:-http://localhost:8080/api/tests}"

# Send test start notification
curl -s -X POST "$MONITORING_URL/start" \
    -H "Content-Type: application/json" \
    -d "{\"test_id\":\"$TEST_ID\",\"status\":\"started\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" \
    > /dev/null 2>&1 || log_warning "Failed to notify monitoring system"

log_info "Notified monitoring system: test started"
```

**report_step_metrics.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

STEP_NUMBER="${TEST_STEP_NUMBER:-unknown}"
EXIT_CODE="${STEP_EXIT_CODE:-unknown}"
METRICS_DB="${METRICS_DATABASE:-/tmp/test_metrics.db}"

# Record step metrics to database
if command -v sqlite3 > /dev/null 2>&1; then
    sqlite3 "$METRICS_DB" \
        "INSERT INTO step_metrics (step_number, exit_code, timestamp) \
         VALUES ($STEP_NUMBER, $EXIT_CODE, datetime('now'));" \
        2>/dev/null || log_verbose "Metrics database not available"
fi

log_verbose "Reported metrics for step $STEP_NUMBER"
```

**notify_test_complete.sh:**
```bash
#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../scripts/lib/logger.sh" || exit 1

TEST_ID="${TEST_CASE_ID:-unknown}"
MONITORING_URL="${MONITORING_ENDPOINT:-http://localhost:8080/api/tests}"

# Calculate test duration
if [ -f /tmp/test_start_time.txt ]; then
    START_TIME=$(cat /tmp/test_start_time.txt)
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
else
    DURATION=0
fi

# Send completion notification
curl -s -X POST "$MONITORING_URL/complete" \
    -H "Content-Type: application/json" \
    -d "{\"test_id\":\"$TEST_ID\",\"status\":\"completed\",\"duration\":$DURATION,\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" \
    > /dev/null 2>&1 || log_warning "Failed to notify monitoring system"

log_info "Notified monitoring system: test completed (${DURATION}s)"
```

### Best Practices

1. **Use the Logger Library**: All hook scripts should use the centralized `scripts/lib/logger.sh` for consistent output formatting
2. **Handle Errors Gracefully**: Use `on_error: "continue"` for cleanup hooks to ensure they always run
3. **Shell Compatibility**: Hook scripts must be compatible with bash 3.2+ (BSD and GNU variants)
4. **Resource Tracking**: Use temporary files to track resources created by hooks for proper cleanup
5. **Minimal Side Effects**: Hooks should be lightweight and not significantly impact test execution time
6. **Environment Variable Access**: Use `${VAR:-default}` syntax to provide defaults for optional variables
7. **Idempotent Operations**: Design hooks to be safely re-runnable when possible

### Example: Complete Test Case with Hooks

See `testcases/examples/hooks/TC_HOOKS_001.yaml` for a comprehensive example demonstrating all eight hook types with:
- Resource management (temporary directories)
- Logging integration (centralized logger library)
- Variable access (sequence and step context)
- Error handling (both fail and continue modes)
- Timing and duration tracking

Full documentation and example hook scripts are available in `testcases/examples/hooks/README.md`.

## Coverage Testing

**MANDATORY**: Code coverage testing is required to ensure comprehensive test coverage across the codebase.

### Installation

Install coverage tools using the provided installation script:

```bash
make install-coverage-tools
```

Or manually install `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
```

For more details on coverage tool installation, see `scripts/README_COVERAGE_TOOLS.md`.

### Coverage Commands

- **Run unit tests with coverage**: `make coverage`
  - Executes unit tests with coverage analysis enabled
  - Minimum threshold: 50% line coverage
  - Excludes: fuzzy.rs, prompts.rs, main_editor.rs

- **Run all tests with coverage (including e2e)**: `make coverage-e2e`
  - Executes unit tests and e2e integration tests with coverage analysis
  - Minimum threshold: 70% line coverage
  - Excludes: fuzzy.rs, prompts.rs, main_editor.rs

- **Generate HTML coverage report**: `make coverage-html`
  - Creates an interactive HTML report showing line-by-line coverage (unit tests only)
  - Opens automatically in your default browser
  - Useful for identifying untested code paths

- **Generate HTML coverage report with e2e**: `make coverage-html-e2e`
  - Creates an interactive HTML report including e2e test coverage
  - Opens automatically in your default browser

- **Display coverage summary**: `make coverage-report`
  - Shows a summary of coverage statistics in the terminal (unit tests only)
  - Provides quick overview of coverage percentages

- **Display coverage summary with e2e**: `make coverage-report-e2e`
  - Shows a summary of coverage statistics including e2e tests
  - Provides quick overview of coverage percentages

### Coverage Exclusions

The following files are excluded from coverage analysis:
- `src/fuzzy.rs` - Interactive fuzzy finder UI components
- `src/prompts.rs` - Interactive prompt UI components
- `src/main_editor.rs` - Main editor binary entry point

### Coverage Requirements

- **Minimum coverage threshold (unit tests)**: 50% line coverage
- **Minimum coverage threshold (unit + e2e tests)**: 70% line coverage
- Coverage must be maintained or improved with each commit
- New code should strive for higher coverage (80%+) when possible
- Review coverage reports to identify critical untested paths

### Pre-Commit Workflow

Before committing any code changes, complete the following steps in order:

1. **Build**: `make build` - Ensure code compiles without errors
2. **Lint**: `make lint` - Fix any style or quality issues
3. **Test**: `make test` - Verify all tests pass
4. **Coverage**: `make coverage-e2e` - Verify coverage meets 70% threshold with e2e tests

All steps must complete successfully before committing changes.


<!-- BACKLOG.MD MCP GUIDELINES START -->

<CRITICAL_INSTRUCTION>

## BACKLOG WORKFLOW INSTRUCTIONS

This project uses Backlog.md MCP for all task and project management activities.

**CRITICAL GUIDANCE**

- If your client supports MCP resources, read `backlog://workflow/overview` to understand when and how to use Backlog for this project.
- If your client only supports tools or the above request fails, call `backlog.get_workflow_overview()` tool to load the tool-oriented overview (it lists the matching guide tools).

- **First time working here?** Read the overview resource IMMEDIATELY to learn the workflow
- **Already familiar?** You should have the overview cached ("## Backlog.md Overview (MCP)")
- **When to read it**: BEFORE creating tasks, or when you're unsure whether to track work

These guides cover:
- Decision framework for when to create tasks
- Search-first workflow to avoid duplicates
- Links to detailed guides for task creation, execution, and finalization
- MCP tools reference

You MUST read the overview resource to understand the complete workflow. The information is NOT summarized here.

</CRITICAL_INSTRUCTION>

<!-- BACKLOG.MD MCP GUIDELINES END -->
