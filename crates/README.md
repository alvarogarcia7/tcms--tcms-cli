# Crate Architecture

This document describes the architecture of the testcase-generator workspace, including the dependency graph, design principles, and how to extend the workspace with new crates.

## Table of Contents

- [Overview](#overview)
- [Dependency Graph](#dependency-graph)
- [Library Crates](#library-crates)
- [Binary Crates](#binary-crates)
- [Autonomous Microservice Design](#autonomous-microservice-design)
- [Adding New Crates](#adding-new-crates)
- [Workspace Dependencies](#workspace-dependencies)

## Overview

The testcase-generator workspace follows a **layered architecture** with clear separation between library crates (shared functionality) and binary crates (autonomous microservices). This design promotes:

- **Modularity**: Each crate has a single, well-defined responsibility
- **Reusability**: Shared logic is centralized in library crates
- **Independence**: Binary crates are self-contained and can be deployed independently
- **Maintainability**: Clear dependency flow prevents circular dependencies
- **Testability**: Small, focused crates are easier to test in isolation

## Dependency Graph

The workspace follows a **bottom-up dependency hierarchy** where higher-level crates depend on lower-level crates, but never the reverse.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           BINARY CRATES                                  │
│  (Autonomous microservices - self-contained, independently deployable)   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  editor              test-orchestrator      test-executor                │
│  test-verify         test-run-manager       script-cleanup               │
│  validate-yaml       validate-json          verifier                     │
│  json-escape         json-to-yaml           tpdg-compat                  │
│                                                                           │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ depends on
                                ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                        ORCHESTRATION LAYER                               │
│         (High-level coordination and workflow management)                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│                    testcase-orchestration                                │
│                    testcase-manager                                      │
│                                                                           │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ depends on
                                ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                      CORE FUNCTIONALITY LAYER                            │
│      (Domain-specific logic: validation, execution, verification)        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  testcase-validation    testcase-execution    testcase-verification      │
│  testcase-storage       testcase-ui           testcase-git               │
│  testcase-cli           testcase-reporting                               │
│                                                                           │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ depends on
                                ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                        FOUNDATION LAYER                                  │
│          (Shared utilities and data structures)                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│                      testcase-common                                     │
│                                                                           │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ depends on
                                ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                          BASE LAYER                                      │
│        (Core data models and primitive utilities)                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│                      testcase-models                                     │
│                      bash-eval                                           │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### Detailed Dependency Flow

```
testcase-models (base data structures)
    ↓
testcase-common (shared utilities, I/O, configuration)
    ↓
┌───────────────────┬──────────────────────┬─────────────────────┐
│                   │                      │                     │
testcase-validation testcase-execution  testcase-verification  testcase-ui
testcase-storage    testcase-git         testcase-cli  testcase-reporting
│                   │                      │                     │
└───────────────────┴──────────────────────┴─────────────────────┘
    ↓
testcase-orchestration (coordinates execution, validation, verification)
testcase-manager (high-level workflows)
    ↓
BINARY CRATES (autonomous microservices)
    - editor (test case editing)
    - test-executor (execute test scripts)
    - test-orchestrator (orchestrate test runs)
    - test-verify (verify test results)
    - test-run-manager (manage test run history)
    - verifier (verify test outputs)
    - validate-yaml (validate YAML files)
    - validate-json (validate JSON files)
    - script-cleanup (clean up generated scripts)
    - json-escape (JSON string escaping utility)
    - json-to-yaml (JSON to YAML conversion)
    - tpdg-compat (test-plan-documentation-generator compatibility)
```

## Library Crates

Library crates provide shared functionality and are organized by layer.

### Base Layer

#### `testcase-models`
**Purpose**: Core data structures and domain models

**Key Types**:
- `TestCase`: Root test case structure
- `TestSequence`: Test sequence with steps
- `TestStep`: Individual test step
- `Verification`: Expected output definitions
- `Hook`: Lifecycle hook definitions
- `Prerequisites`: Prerequisite definitions

**Dependencies**: `bash-eval`, `serde`, `serde_json`, `serde_yaml`, `chrono`, `indexmap`

**Consumers**: All other crates directly or indirectly

#### `bash-eval`
**Purpose**: Bash expression evaluation and variable substitution

**Key Functionality**:
- Evaluate bash expressions
- Variable substitution in strings
- Template rendering with context

**Dependencies**: `serde`, `anyhow`, `log`, `regex`

**Consumers**: `testcase-models`, `testcase-execution`, `testcase-verification`, `testcase-manager`

### Foundation Layer

#### `testcase-common`
**Purpose**: Shared utilities, I/O, and configuration management

**Key Functionality**:
- File I/O utilities
- YAML/JSON/TOML parsing
- Environment variable management
- Configuration loading
- Path resolution

**Dependencies**: `testcase-models`, `serde`, `serde_json`, `serde_yaml`, `anyhow`, `toml`, `dotenvy`, `log`

**Consumers**: `testcase-validation`, `testcase-execution`, `testcase-verification`, `testcase-storage`, `testcase-ui`

### Core Functionality Layer

#### `testcase-validation`
**Purpose**: Schema validation for test cases and outputs

**Key Functionality**:
- JSON schema validation
- YAML structure validation
- XML validation
- Test case schema compliance checking

**Dependencies**: `testcase-models`, `testcase-common`, `anyhow`, `jsonschema`, `roxmltree`, `serde`, `serde_json`, `serde_yaml`, `log`

**Consumers**: `testcase-storage`, `test-executor`, `validate-yaml`, `validate-json`

#### `testcase-execution`
**Purpose**: Test script generation and execution

**Key Functionality**:
- Generate bash scripts from test cases
- Execute test scripts
- Capture execution output
- Handle lifecycle hooks
- Environment variable hydration

**Dependencies**: `bash-eval`, `testcase-models`, `testcase-common`, `anyhow`, `chrono`, `regex`, `tempfile`, `serde`, `serde_json`

**Consumers**: `testcase-orchestration`, `testcase-manager`, `test-executor`

#### `testcase-verification`
**Purpose**: Verify test execution results against expectations

**Key Functionality**:
- Compare actual vs. expected output
- Parse verification results
- Generate verification reports
- Clean up generated scripts

**Dependencies**: `testcase-models`, `testcase-common`, `bash-eval`, `regex`, `strip-ansi-escapes`, `serde_json`, `serde`, `serde_yaml`, `anyhow`

**Consumers**: `testcase-orchestration`, `testcase-ui`, `test-orchestrator`, `test-verify`, `verifier`, `script-cleanup`

#### `testcase-reporting`
**Purpose**: Strict input loading, run selection, report model and renderers for `test-executor report`

**Key Functionality**:
- Lenient loaders shared by `trace-matrix` and `regression-select` (`legacy`)
- `LoadedRun`: run record with its source file and parsed RFC 3339 start instant
- Strict loaders and run selection (added in later plans)

**Dependencies**: `testcase-models`, `testcase-common`, `testcase-verification`, `anyhow`, `chrono`, `serde_yaml`

**Consumers**: `test-executor` only, by design

#### `testcase-storage`
**Purpose**: File system storage and retrieval of test cases

**Key Functionality**:
- Load test cases from YAML files
- Save test cases to disk
- Discover test cases in directories
- Manage test run results

**Dependencies**: `testcase-models`, `testcase-common`, `testcase-validation`, `walkdir`, `serde_yaml`, `serde_json`, `serde`, `anyhow`, `chrono`

**Consumers**: `testcase-orchestration`, `testcase-ui`, `testcase-manager`, `test-orchestrator`, `test-executor`, `test-run-manager`, `test-verify`, `verifier`

#### `testcase-ui`
**Purpose**: Terminal user interface components

**Key Functionality**:
- Interactive test case selection (skim fuzzy finder)
- Confirmation dialogs (dialoguer)
- Text editor integration (edit)
- Git commit message composition
- Progress indicators

**Dependencies**: `testcase-models`, `testcase-storage`, `testcase-common`, `testcase-verification`, `testcase-git`, `skim`, `dialoguer`, `edit`, `anyhow`

**Consumers**: `testcase-manager`, `editor`, `test-orchestrator`, `test-run-manager`

#### `testcase-git`
**Purpose**: Git integration for test cases

**Key Functionality**:
- Repository status checking
- Commit creation
- Diff generation
- Branch management

**Dependencies**: `anyhow`, `git2`, `chrono`, `log`

**Consumers**: `testcase-manager`, `testcase-ui`, `editor`

#### `testcase-cli`
**Purpose**: Common CLI argument definitions

**Key Functionality**:
- Shared CLI argument structures
- Command-line parsing utilities

**Dependencies**: `testcase-models`, `clap`

**Consumers**: `testcase-manager`, `editor`

### Orchestration Layer

#### `testcase-orchestration`
**Purpose**: High-level coordination of test execution and verification

**Key Functionality**:
- Orchestrate test execution workflows
- Coordinate validation → execution → verification
- Manage test run lifecycle
- Aggregate results

**Dependencies**: `testcase-models`, `testcase-execution`, `testcase-storage`, `testcase-verification`, `anyhow`, `chrono`, `serde`, `serde_json`, `serde_yaml`

**Consumers**: `testcase-manager`, `test-orchestrator`

#### `testcase-manager`
**Purpose**: High-level test management workflows

**Key Functionality**:
- End-to-end test case management
- Interactive editing workflows
- Git integration workflows
- Complete test execution pipelines

**Dependencies**: `bash-eval`, `testcase-models`, `testcase-cli`, `testcase-common`, `testcase-execution`, `testcase-git`, `testcase-orchestration`, `testcase-storage`, `testcase-ui`, `testcase-validation`, `testcase-verification`, `clap`, `anyhow`, `serde`, `serde_json`, `serde_yaml`, `chrono`, `log`, `env_logger`

**Consumers**: `editor`, `test-verify`, `validate-yaml`

**Note**: `testcase-manager` is feature-gated to prevent circular dependencies. Some binaries use `default-features = false` to avoid pulling in unnecessary UI dependencies.

## Binary Crates

Binary crates are **autonomous microservices** - self-contained applications that can be built, deployed, and run independently.

### Autonomous Microservice Design

Each binary crate follows these principles:

1. **Self-Contained**: Has its own `main.rs` and complete CLI interface
2. **Single Responsibility**: Performs one well-defined task
3. **Minimal Dependencies**: Only depends on necessary library crates
4. **Stateless**: Does not maintain long-running state (except file I/O)
5. **Composable**: Can be used standalone or as part of larger workflows

### Test Case Management

#### `editor`
**Binary**: `editor`

**Purpose**: Interactive test case editor with Git integration

**Key Features**:
- Fuzzy search test case selection
- Edit test cases in your preferred editor
- Interactive Git commit workflow
- Automatic validation on save

**Dependencies**: `testcase-manager`, `testcase-ui`, `testcase-cli`, `testcase-git`, `testcase-storage`

**Usage**:
```bash
editor [OPTIONS]
```

### Test Execution

#### `test-executor`
**Binary**: `test-executor`

**Purpose**: Execute test cases and generate bash scripts

**Key Features**:
- Generate bash scripts from YAML test cases
- Execute test scripts
- Save execution results
- Support for environment variable hydration

**Dependencies**: `testcase-execution`, `testcase-storage`, `testcase-models`, `testcase-validation`, `clap`

**Usage**:
```bash
test-executor --testcase <FILE> [OPTIONS]
```

#### `test-orchestrator`
**Binary**: `test-orchestrator`

**Purpose**: Orchestrate complete test execution workflows

**Key Features**:
- Execute multiple test cases
- Coordinate validation → execution → verification
- Generate comprehensive reports
- Interactive test selection

**Dependencies**: `testcase-orchestration`, `testcase-storage`, `testcase-ui`, `testcase-verification`

**Usage**:
```bash
test-orchestrator [OPTIONS]
```

### Test Verification

#### `test-verify`
**Binary**: `test-verify`

**Purpose**: Verify test execution results against expected outputs

**Key Features**:
- Compare actual vs. expected output
- Generate verification reports
- Support for multiple verification formats
- Interactive result browsing

**Dependencies**: `testcase-manager`, `testcase-verification`, `testcase-storage`, `testcase-models`, `clap`

**Usage**:
```bash
test-verify --testcase <FILE> [OPTIONS]
```

#### `verifier`
**Binary**: `verifier`

**Purpose**: Standalone verification of test outputs

**Key Features**:
- Verify JSON/YAML output files
- Schema validation
- Generate verification reports

**Dependencies**: `testcase-verification`, `testcase-storage`, `testcase-models`, `clap`, `jsonschema`

**Usage**:
```bash
verifier --testcase <FILE> --result <FILE> [OPTIONS]
```

### Test Run Management

#### `test-run-manager` (trm)
**Binary**: `trm`

**Purpose**: Manage test run history and results

**Key Features**:
- List test runs
- View test run details
- Export test results
- Clean up old test runs

**Dependencies**: `testcase-storage`, `testcase-ui`, `testcase-models`, `clap`, `chrono`

**Usage**:
```bash
trm [COMMAND] [OPTIONS]
```

### Validation Utilities

#### `validate-yaml`
**Binary**: `validate-yaml`

**Purpose**: Validate YAML test case files against schema

**Key Features**:
- Schema validation
- Syntax checking
- Detailed error reporting

**Dependencies**: `testcase-validation`, `testcase-models`, `testcase-common`, `testcase-manager`, `anyhow`

**Usage**:
```bash
validate-yaml <FILE>
```

#### `validate-json`
**Binary**: `validate-json`

**Purpose**: Validate JSON files against JSON schemas

**Key Features**:
- JSON schema validation
- Detailed error messages
- Support for external schemas

**Dependencies**: `testcase-validation`, `testcase-common`, `anyhow`, `clap`, `serde_json`

**Usage**:
```bash
validate-json --schema <SCHEMA> <FILE>
```

### Utility Tools

#### `script-cleanup`
**Binary**: `script-cleanup`

**Purpose**: Clean up generated test scripts

**Key Features**:
- Remove generated bash scripts
- Clean up temporary files
- Configurable cleanup policies

**Dependencies**: `testcase-verification`, `clap`, `anyhow`, `log`, `env_logger`

**Usage**:
```bash
script-cleanup [OPTIONS]
```

#### `json-escape`
**Binary**: `json-escape`

**Purpose**: Escape strings for JSON

**Key Features**:
- Escape special characters
- Handle newlines and quotes
- Stdin/stdout piping support

**Dependencies**: `clap`, `serde_json`, `anyhow`, `env_logger`, `log`

**Usage**:
```bash
echo 'text with "quotes"' | json-escape
```

#### `json-to-yaml`
**Binary**: `json-to-yaml`

**Purpose**: Convert JSON files to YAML format

**Key Features**:
- JSON to YAML conversion
- Preserves structure
- Pretty formatting

**Dependencies**: `clap`, `serde_json`, `serde_yaml`, `anyhow`

**Usage**:
```bash
json-to-yaml input.json output.yaml
```

#### `tpdg-compat`
**Binary**: `test-plan-documentation-generator-compat`

**Purpose**: Compatibility shim for test-plan-documentation-generator

**Key Features**:
- Provides compatible CLI interface
- Delegates to external `test-plan-documentation-generator`
- Fallback for missing external tool

**Dependencies**: `anyhow`, `clap`, `serde`, `serde_json`, `serde_yaml`

**Usage**:
```bash
test-plan-documentation-generator-compat [OPTIONS]
```

## Adding New Crates

### Adding a New Library Crate

1. **Create the crate directory**:
   ```bash
   cargo new --lib crates/my-new-lib
   ```

2. **Add to workspace** in root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       # ... existing members ...
       "crates/my-new-lib",
   ]
   ```

3. **Define dependencies** in `crates/my-new-lib/Cargo.toml`:
   ```toml
   [package]
   name = "my-new-lib"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   # Only depend on lower-layer crates
   testcase-models = { path = "../testcase-models" }
   testcase-common = { path = "../testcase-common" }
   
   # Use workspace dependencies
   serde = { workspace = true }
   anyhow = { workspace = true }
   ```

4. **Follow dependency rules**:
   - Only depend on crates in lower layers
   - Never create circular dependencies
   - Use workspace dependencies for external crates
   - Document the crate's layer and purpose

5. **Update this README**:
   - Add to the dependency graph
   - Document in the appropriate layer section
   - List key functionality and consumers

### Adding a New Binary Crate

1. **Create the crate directory**:
   ```bash
   cargo new crates/my-new-tool
   ```

2. **Configure as binary** in `crates/my-new-tool/Cargo.toml`:
   ```toml
   [package]
   name = "my-new-tool"
   version = "0.1.0"
   edition = "2021"

   [[bin]]
   name = "my-new-tool"
   path = "src/main.rs"

   [dependencies]
   # Depend on necessary library crates
   testcase-models = { path = "../testcase-models" }
   testcase-common = { path = "../testcase-common" }
   
   # CLI dependencies
   clap = { workspace = true }
   anyhow = { workspace = true }
   ```

3. **Add to workspace** in root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       # ... existing members ...
       "crates/my-new-tool",
   ]
   ```

4. **Implement the microservice**:
   - Create `src/main.rs` with a focused CLI interface
   - Keep the binary self-contained
   - Follow single responsibility principle
   - Use library crates for shared logic

5. **Build and test**:
   ```bash
   cargo build -p my-new-tool
   cargo test -p my-new-tool
   cargo run -p my-new-tool -- --help
   ```

6. **Update this README**:
   - Add to the Binary Crates section
   - Document purpose, features, and usage
   - List dependencies

### Dependency Guidelines

**DO**:
- ✅ Depend on crates in lower layers
- ✅ Use workspace dependencies for external crates
- ✅ Keep binary crates independent and focused
- ✅ Document dependencies and their purpose
- ✅ Follow the layer hierarchy

**DON'T**:
- ❌ Create circular dependencies
- ❌ Depend on crates in higher layers
- ❌ Add duplicate functionality across crates
- ❌ Create deep dependency chains (prefer flat dependencies)
- ❌ Mix library and binary code in the same crate

## Workspace Dependencies

The workspace uses centralized dependency management through `[workspace.dependencies]` in the root `Cargo.toml`. This ensures:

- **Version Consistency**: All crates use the same versions of external dependencies
- **Reduced Duplication**: Version numbers specified once
- **Easier Updates**: Update dependencies in one place

### Common Workspace Dependencies

**Serialization**:
- `serde` - Serialization framework
- `serde_json` - JSON support
- `serde_yaml` - YAML support
- `serde_path_to_error` - Better error messages

**CLI & UI**:
- `clap` - Command-line argument parsing
- `skim` - Fuzzy finder
- `dialoguer` - Interactive prompts
- `edit` - Editor integration

**Error Handling**:
- `anyhow` - Error handling (pinned to 1.0.99)
- `log` - Logging facade
- `env_logger` - Logger implementation

**Utilities**:
- `regex` - Regular expressions
- `chrono` - Date and time
- `indexmap` - Ordered hash maps
- `walkdir` - Recursive directory walking
- `tempfile` - Temporary file creation

**Validation**:
- `jsonschema` - JSON schema validation
- `roxmltree` - XML parsing

**Configuration**:
- `dotenvy` - .env file support
- `toml` - TOML parsing

**Git**:
- `git2` - Git integration

See the root `Cargo.toml` for the complete list and version constraints.

## Best Practices

1. **Keep crates focused**: Each crate should have a single, clear purpose
2. **Minimize dependencies**: Only depend on what you need
3. **Respect the layer hierarchy**: Never depend on higher-layer crates
4. **Document thoroughly**: Update this README when adding crates
5. **Test in isolation**: Each crate should have its own test suite
6. **Use workspace dependencies**: For consistency and easier maintenance
7. **Follow naming conventions**:
   - Libraries: `testcase-*` for domain crates
   - Utilities: Descriptive names like `bash-eval`, `json-escape`
   - Binaries: Short, memorable names reflecting their purpose

## Building and Testing

### Build Everything
```bash
cargo build --workspace
```

### Build Specific Crate
```bash
cargo build -p testcase-models
cargo build -p editor
```

### Test Everything
```bash
cargo test --workspace
```

### Test Specific Crate
```bash
cargo test -p testcase-validation
```

### Run Binary
```bash
cargo run -p editor
cargo run -p test-executor -- --help
```

### Check Dependencies
```bash
# View dependency tree
cargo tree -p testcase-orchestration

# View reverse dependencies (what depends on this crate)
cargo tree -p testcase-models --invert
```

## Architecture Evolution

As the project grows, consider:

- **Feature Flags**: Use Cargo features to make crates more modular
- **Async Support**: Add async versions of library functions where beneficial
- **Plugin System**: Allow binary crates to load plugins dynamically
- **API Stabilization**: Mark stable APIs with semantic versioning
- **Performance**: Profile and optimize hot paths in library crates
- **Documentation**: Generate API docs with `cargo doc --workspace --open`

---

**Last Updated**: 2024
**Maintained By**: testcase-generator project maintainers
