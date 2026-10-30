# ============================================================================
# Testcase Manager Makefile
# ============================================================================
#
# This Makefile uses an INCREMENTAL BUILD STRATEGY by default for everyday
# development workflows. Common targets like `make build`, `make test`,
# `make lint`, and `make clippy` automatically detect which crates have changed
# since a base reference (default: main branch) and only build/test those crates
# and their reverse dependencies.
#
# INCREMENTAL TARGETS (default for everyday use):
#   make build              - Build only affected crates
#   make test               - Test only affected crates
#   make lint               - Lint only affected crates
#   make clippy             - Clippy only affected crates
#   make test-unit          - Unit tests for affected crates only
#   make test-e2e           - E2E tests for affected crates only
#
# FULL BUILD TARGETS (for CI/CD or comprehensive validation):
#   make build-all          - Build all workspace crates unconditionally
#   make test-all           - Full build + all unit tests + all E2E tests
#   make test-e2e-all       - Run all E2E tests unconditionally (with build)
#
# CUSTOMIZING BASE REFERENCE:
#   By default, incremental targets compare against 'main' branch.
#   You can override this with BASE_REF parameter:
#
#   make build BASE_REF=develop          # Compare against develop branch
#   make test BASE_REF=HEAD~3            # Compare against 3 commits ago
#   make build BASE_REF=abc123           # Compare against specific commit
#
# DEBUGGING:
#   make list-affected-crates BASE_REF=main  # See which crates would be built
#
# Include modular makefiles
include mk/python.mk
-include mk/incremental.mk
include mk/strictdoc.mk
-include mk/projector.mk
-include mk/backlog.mk

pre-commit:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	@${MAKE} lint BASE_REF=$(or $(BASE_REF),main)
	@${MAKE} test BASE_REF=$(or $(BASE_REF),main)
.PHONY: pre-commit

README_INSTALL_AUTOMATED.md:
	echo "" > README_INSTALL_AUTOMATED.md
	@for bin in $(shell cargo run --bin 2>&1| grep "^    "|awk '{print $1}'); do \
		cargo build --bin $$bin ; \
		echo "## $$bin " >> README_INSTALL_AUTOMATED.md ; \
		$$bin -- --help >> README_INSTALL_AUTOMATED.md; \
		echo "\`\`\`" >> README_INSTALL_AUTOMATED.md ; \
  		cargo run --bin $$bin -- --help >> README_INSTALL_AUTOMATED.md; \
		echo "\`\`\`" >> README_INSTALL_AUTOMATED.md ; \
  	done
.PHONY: README_INSTALL_AUTOMATED.md

lint:
	@${MAKE} fmt BASE_REF=$(or $(BASE_REF),main)
	@${MAKE} clippy BASE_REF=$(or $(BASE_REF),main)
.PHONY: lint

fmt:
	cargo fmt
.PHONY: fmt

build:
	@BASE_REF=$(or $(BASE_REF),main); \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No changes detected - skipping build"; \
	else \
		echo "Building affected crates: $$AFFECTED"; \
		for crate in $$AFFECTED; do \
			cargo build -p "$$crate" || exit 1; \
		done; \
	fi
.PHONY: build

build-debug:
	@BASE_REF=$(or $(BASE_REF),main); \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No changes detected - skipping build"; \
	else \
		echo "Building affected crates: $$AFFECTED"; \
		for crate in $$AFFECTED; do \
			cargo build -p "$$crate" || exit 1; \
		done; \
	fi
.PHONY: build-debug

build-release:
	@BASE_REF=$(or $(BASE_REF),main); \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No changes detected - skipping build"; \
	else \
		echo "Building affected crates (release mode): $$AFFECTED"; \
		for crate in $$AFFECTED; do \
			cargo build -p "$$crate" --release || exit 1; \
		done; \
	fi
.PHONY: build-release

# Publish macOS arm64 binaries, built locally, to a GitLab release.
# Manual goal: run on a macOS machine (the CI runners are Linux-only and cannot
# reliably cross-compile the macOS binaries). Builds the current local commit.
# Usage:
#   make publish-macos TAG=v0.0.1
# Requires: GITLAB_TOKEN env var (personal access token with 'api' scope),
#           plus cargo, jq, zip, curl.
publish-macos:
	@scripts/publish-macos-release.sh "$(TAG)-mac-local"
.PHONY: publish-macos

# Per-crate build targets
build-validate-yaml:
	cargo build --package validate-yaml
.PHONY: build-validate-yaml

build-validate-json:
	cargo build --package validate-json
.PHONY: build-validate-json

build-verifier:
	cargo build --package verifier
.PHONY: build-verifier

build-test-executor:
	cargo build --package test-executor
.PHONY: build-test-executor

build-test-orchestrator:
	cargo build --package test-orchestrator
.PHONY: build-test-orchestrator

build-test-run-manager:
	cargo build --package test-run-manager
.PHONY: build-test-run-manager

build-test-verify:
	cargo build --package test-verify
.PHONY: build-test-verify

build-script-cleanup:
	cargo build --package script-cleanup
.PHONY: build-script-cleanup

build-json-escape:
	cargo build --package json-escape
.PHONY: build-json-escape

build-json-to-yaml:
	cargo build --package json-to-yaml
.PHONY: build-json-to-yaml

build-editor:
	cargo build --package editor
.PHONY: build-editor

build-testcase-manager:
	cargo build --package testcase-manager
.PHONY: build-testcase-manager

build-tpdg-compat:
	cargo build --package tpdg-compat
.PHONY: build-tpdg-compat

build-bash-eval:
	cargo build --package bash-eval
.PHONY: build-bash-eval

build-audit-verifier:
	cargo build --package audit-verifier
.PHONY: build-audit-verifier

audit-verify: build-audit-verifier
	./target/debug/audit-verifier --yaml testcases/SELF_VALIDATED_EXAMPLE_001.yml --log testcases/self_validated_example_execution_log_with_hash.json
.PHONY: audit-verify

# audit-verifier integration tests
test-audit-verifier: build-audit-verifier
	./crates/audit-verifier/tests/run_all_tests.sh
.PHONY: test-audit-verifier

test-audit-verifier-simple: build-audit-verifier
	./crates/audit-verifier/tests/integration/test_simple_workflow.sh
.PHONY: test-audit-verifier-simple

test-audit-verifier-e2e: build-audit-verifier
	./crates/audit-verifier/tests/integration/test_audit_verifier_e2e.sh
.PHONY: test-audit-verifier-e2e

test-audit-verifier-keys: build-audit-verifier
	./crates/audit-verifier/tests/integration/test_audit_key_scenarios.sh
.PHONY: test-audit-verifier-keys

test: setup-python-for-test
	${MAKE} test-unit
	${MAKE} test-e2e
	${MAKE} verify-testcases
	${MAKE} strictdoc-validate
# 	${MAKE} generate-docs-coverage
	${MAKE} coverage-clean
.PHONY: test

test-unit: build
	@BASE_REF=$(or $(BASE_REF),main); \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No changes detected - skipping unit tests"; \
	else \
		echo "Running unit tests for affected crates: $$AFFECTED"; \
		for crate in $$AFFECTED; do \
			cargo test -p "$$crate" --all-features --tests || exit 1; \
		done; \
	fi
.PHONY: test-unit

test-doc:
	cargo test --workspace --doc
.PHONY: test-doc

clippy:
	@BASE_REF=$(or $(BASE_REF),main); \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No changes detected - skipping clippy"; \
	else \
		echo "Running clippy for affected crates: $$AFFECTED"; \
		for crate in $$AFFECTED; do \
			cargo clippy -p "$$crate" --all-features --tests -- -D warnings || exit 1; \
		done; \
	fi
.PHONY: clippy

run:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./target/debug/testcase-manager
.PHONY: run

run-trm: build-test-run-manager
	./target/debug/trm
.PHONY: run-trm

run-test-verify: build-test-verify
	./target/debug/test-verify
.PHONY: run-test-verify

run-script-cleanup: build-script-cleanup
	./target/debug/script-cleanup
.PHONY: run-script-cleanup

run-json-escape: build-json-escape
	./target/debug/json-escape
.PHONY: run-json-escape

run-verifier: build-verifier
	./target/debug/verifier
.PHONY: run-verifier

# Audit logging targets
run-sign-audit-log: build-audit-verifier
	./target/debug/sign-audit-log
.PHONY: run-sign-audit-log

run-verify-audit-log: build-audit-verifier
	./target/debug/verify-audit-log
.PHONY: run-verify-audit-log

run-audit-verifier: build-audit-verifier
	./target/debug/audit-verifier
.PHONY: run-audit-verifier

# Demo for audit logging
demo-audit-logging: build-audit-verifier build-test-executor
	@echo "Running audit logging demonstration..."
	@chmod +x examples/audit_logging_demo.sh
	@bash examples/audit_logging_demo.sh
.PHONY: demo-audit-logging

test-e2e-verifier-container:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/test_verifier_container_e2e.sh
.PHONY: test-e2e-verifier-container

test-verifier-edge-cases:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	cargo test verification_edge_cases_test
#	./tests/integration/test_verifier_edge_cases_e2e.sh
.PHONY: test-verifier-edge-cases

# E2E test of the verifier behaviour changes of TCMS-41 (empty folder, unverifiable log, zero steps,
# failed hook, manual step without log entry, report on stdout)
test-verifier-behaviour:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./crates/testcase-manager/tests/integration/test_verifier_behaviour_changes_e2e.sh
.PHONY: test-verifier-behaviour

test-e2e-failing:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/run_e2e_test.sh
	./tests/integration/test_variable_passing_e2e.sh

.PHONY: test-e2e-failing

test-e2e-failing-all:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/run_all_tests.sh
.PHONY: test-e2e-failing-all

test-e2e: build
	@BASE_REF=$(or $(BASE_REF),main); \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No changes detected - running minimal E2E tests"; \
		./crates/testcase-manager/tests/integration/check_environment.sh; \
		./crates/testcase-manager/tests/integration/smoke_test.sh; \
	else \
		echo "Running E2E tests for affected crates: $$AFFECTED"; \
		E2E_TESTS_FILE=$$(mktemp); \
		./crates/testcase-manager/tests/integration/check_environment.sh; \
		for crate in $$AFFECTED; do \
			case "$$crate" in \
				validate-yaml) \
					echo "./crates/testcase-manager/tests/integration/test_validate_yaml_watch_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_validate_yaml_multi_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_validate_yaml_schema_watch_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_validate_yaml_transitive_schema_watch_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_auto_schema_validation_e2e.sh" >> $$E2E_TESTS_FILE; \
					;; \
				test-executor) \
					echo "./crates/testcase-manager/tests/integration/test_executor_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_script_generation_acceptance_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_variable_passing_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_conditional_verification_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_manual_steps_e2e.sh" >> $$E2E_TESTS_FILE; \
					;; \
				test-orchestrator) \
					echo "./crates/testcase-manager/tests/integration/test_orchestrator_e2e.sh" >> $$E2E_TESTS_FILE; \
					;; \
				verifier) \
					echo "./crates/testcase-manager/tests/integration/test_verifier_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_verifier_container_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_verifier_edge_cases_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_verifier_behaviour_changes_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./scripts/run_verifier_and_generate_reports.sh" >> $$E2E_TESTS_FILE; \
					;; \
				audit-verifier) \
					echo "./crates/audit-verifier/tests/integration/test_audit_logging_demo_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/audit-verifier/tests/integration/test_audit_verifier_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/audit-verifier/tests/integration/test_audit_key_scenarios.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/audit-verifier/tests/integration/test_simple_workflow.sh" >> $$E2E_TESTS_FILE; \
					;; \
				json-escape) \
					echo "./crates/testcase-manager/tests/integration/test_json_escape_e2e.sh" >> $$E2E_TESTS_FILE; \
					;; \
				testcase-manager) \
					echo "./crates/testcase-manager/tests/integration/smoke_test.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_bdd_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_manual_verification_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_variable_display_e2e.sh" >> $$E2E_TESTS_FILE; \
					echo "./crates/testcase-manager/tests/integration/test_documentation_generation.sh" >> $$E2E_TESTS_FILE; \
					;; \
			esac; \
		done; \
		if [ -s "$$E2E_TESTS_FILE" ]; then \
			cat "$$E2E_TESTS_FILE" | sort -u | while read test_script; do \
				if [ -f "$$test_script" ]; then \
					echo "Running: $$test_script"; \
					bash "$$test_script" || exit 1; \
				fi; \
			done; \
		fi; \
		rm -f "$$E2E_TESTS_FILE"; \
		if echo "$$AFFECTED" | grep -qw "verifier"; then \
			${MAKE} test-verifier-edge-cases; \
			BUILD_VARIANT="" ./scripts/run_verifier_and_generate_reports.sh; \
		fi; \
		${MAKE} validate-output-schemas; \
	fi
.PHONY: test-e2e

test-e2e-f: build
	# Failing/disabled integration tests - work in progress to fix and move back to test-e2e
	./crates/testcase-manager/tests/integration/test_bdd_initial_conditions.sh
	./crates/testcase-manager/tests/integration/test_dependencies_e2e.sh
	./crates/testcase-manager/tests/integration/test_docker_build.sh
	./crates/testcase-manager/tests/integration/test_hooks_e2e.sh
	./crates/testcase-manager/tests/integration/test_json_escape_e2e.sh
	./crates/testcase-manager/tests/integration/test_orchestrator_examples.sh
	./crates/testcase-manager/tests/integration/test_run_manager_e2e.sh
	./crates/testcase-manager/tests/integration/test_verify_e2e.sh
	./crates/testcase-manager/tests/integration/test_container_yaml_compat_e2e.sh
.PHONY: test-e2e-f

# test-e2e-all: Run ALL E2E integration tests unconditionally
# This target runs the complete E2E test suite regardless of changes
# Used by test-all to ensure comprehensive testing in CI/CD
test-e2e-all: setup-python-for-test build-all
	${MAKE} test-e2e-all-no-build
.PHONY: test-e2e-all

# test-e2e-all-no-build: Run ALL E2E integration tests without building
# Internal target used by test-all after build-all has already been run
test-e2e-all-no-build:
	./crates/testcase-manager/tests/integration/check_environment.sh
	./crates/testcase-manager/tests/integration/smoke_test.sh
	./crates/testcase-manager/tests/integration/test_bash_commands_e2e.sh
	./crates/testcase-manager/tests/integration/test_bdd_e2e.sh
	./crates/testcase-manager/tests/integration/test_conditional_verification_e2e.sh
	./crates/testcase-manager/tests/integration/test_executor_e2e.sh
	./crates/testcase-manager/tests/integration/test_manual_steps_e2e.sh
	./crates/testcase-manager/tests/integration/test_manual_verification_e2e.sh
	./crates/testcase-manager/tests/integration/test_orchestrator_e2e.sh
	./crates/testcase-manager/tests/integration/test_pipeline_e2e.sh
	./crates/testcase-manager/tests/integration/test_validate_yaml_watch_e2e.sh
	./crates/testcase-manager/tests/integration/test_validate_yaml_multi_e2e.sh
	./crates/testcase-manager/tests/integration/test_validate_yaml_schema_watch_e2e.sh
	./crates/testcase-manager/tests/integration/test_validate_yaml_transitive_schema_watch_e2e.sh
	./crates/testcase-manager/tests/integration/test_auto_schema_validation_e2e.sh
	./crates/testcase-manager/tests/integration/test_variable_display_e2e.sh
	./crates/testcase-manager/tests/integration/test_variable_passing_e2e.sh
	./crates/testcase-manager/tests/integration/test_verifier_e2e.sh
	./crates/testcase-manager/tests/integration/test_verifier_container_e2e.sh
	./crates/testcase-manager/tests/integration/test_verifier_edge_cases_e2e.sh
	./crates/testcase-manager/tests/integration/test_verifier_behaviour_changes_e2e.sh
	${MAKE} test-verifier-edge-cases
	./crates/testcase-manager/tests/integration/test_documentation_generation.sh
# 	BUILD_VARIANT="" ./scripts/run_verifier_and_generate_reports.sh
# 	${MAKE} validate-output-schemas
.PHONY: test-e2e-all-f so-build

example_export-demo:
	./examples/export_demo.sh
.PHONY: example_export-demo

# build-all: Unconditional full workspace build with all features
# Ignores change detection and always builds the entire workspace
build-all:
	cargo build --workspace --all-features
.PHONY: build-all

# test-all: Unconditional full build and test cycle
# Performs complete build, unit tests, and E2E tests regardless of changes
# This target ignores any incremental build logic and always runs the full suite
test-all: setup-python-for-test build-all
	cargo test --workspace --all-features --tests
	${MAKE} test-e2e-all-no-build
.PHONY: test-all

# Coverage exclusion pattern - escapes dots for regex
COVERAGE_EXCLUDE_REGEX = (crates/testcase-manager/src/fuzzy\\.rs|crates/testcase-manager/src/prompts\\.rs|crates/testcase-manager/src/main_editor\\.rs)

coverage:
	cargo llvm-cov --all-features --workspace --tests --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)' --fail-under-lines 50
.PHONY: coverage

coverage-e2e:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	cargo llvm-cov clean --workspace
	cargo llvm-cov --all-features --workspace --no-report --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)'
	${MAKE} test-e2e BASE_REF=$(or $(BASE_REF),main)
	cargo llvm-cov report --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)' --fail-under-lines 70
.PHONY: coverage-e2e

coverage-html:
	cargo llvm-cov --all-features --workspace --tests --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)' --html --open
.PHONY: coverage-html

coverage-html-e2e:
	${MAKE} coverage-e2e
	cargo llvm-cov report --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)' --html --open
.PHONY: coverage-html-e2e

coverage-lcov:
	cargo llvm-cov --all-features --workspace --tests --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)' --lcov --output-path target/llvm-cov/lcov.info
.PHONY: coverage-lcov

coverage-report:
	cargo llvm-cov report --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)'
.PHONY: coverage-report

coverage-report-e2e:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	cargo llvm-cov clean --workspace
	cargo llvm-cov --all-features --workspace --no-report --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)'
	${MAKE} test-e2e BASE_REF=$(or $(BASE_REF),main)
	cargo llvm-cov report --ignore-filename-regex '$(COVERAGE_EXCLUDE_REGEX)'
.PHONY: coverage-report-e2e

clean:
	${MAKE} coverage-clean
	@if command -v sccache >/dev/null 2>&1; then \
		${MAKE} sccache-clean; \
	fi
.PHONY: clean

coverage-clean:
	@-cargo llvm-cov clean --workspace > /dev/null 2>&1 || true
	mkdir -p "/tmp/coverage/"
	find . -iname "*.profraw" -type f -exec mv "{}" /tmp/coverage \;
	rm -f /tmp/coverage/*
.PHONY: coverage-clean

install-coverage-tools:
	./scripts/install-coverage-tools.sh --local
.PHONY: install-coverage-tools

install-sccache:
	./scripts/install-sccache.sh --local
.PHONY: install-sccache

enable-sccache:
	@echo "To enable sccache, run:"
	@echo "  source ./scripts/enable-sccache.sh"
	@echo ""
	@echo "Or for permanent setup:"
	@echo "  source ./scripts/enable-sccache.sh --permanent"
	@echo ""
	@echo "To check if sccache is enabled:"
	@echo "  source ./scripts/enable-sccache.sh --check"
.PHONY: enable-sccache

disable-sccache:
	@echo "To disable sccache, run:"
	@echo "  source ./scripts/disable-sccache.sh"
	@echo ""
	@echo "Or manually:"
	@echo "  unset RUSTC_WRAPPER"
.PHONY: disable-sccache

sccache-stats:
	@sccache --show-stats
.PHONY: sccache-stats

sccache-check:
	@if [ -z "$$RUSTC_WRAPPER" ]; then \
		echo "❌ sccache is NOT enabled"; \
		echo ""; \
		echo "To enable sccache:"; \
		echo "  source ./scripts/enable-sccache.sh"; \
		echo ""; \
		echo "Or add to your shell profile:"; \
		echo "  export RUSTC_WRAPPER=sccache"; \
		exit 1; \
	elif [ "$$RUSTC_WRAPPER" != "sccache" ]; then \
		echo "⚠️  RUSTC_WRAPPER is set to '$$RUSTC_WRAPPER' (expected: 'sccache')"; \
		exit 1; \
	else \
		echo "✅ sccache is enabled (RUSTC_WRAPPER=sccache)"; \
		echo ""; \
		echo "Cache directory: ~/.cache/sccache/testcase-manager"; \
		if command -v sccache >/dev/null 2>&1; then \
			echo ""; \
			sccache --show-stats; \
		fi; \
	fi
.PHONY: sccache-check

sccache-clean:
	@sccache --stop-server || true
	@echo "sccache server stopped"
	@echo "Note: Global cache directory preserved at ~/.cache/sccache/testcase-manager"
	@echo "To manually remove cache: rm -rf ~/.cache/sccache/testcase-manager"
.PHONY: sccache-clean

# Test case file naming: the file name must equal the `id:` inside, and the extension must be .yml.
# Checks every test case below testcases/ and test-acceptance/ (existing files that break the rule are
# listed). CI runs the changed-files mode: make check-testcase-files-changed BASE_REF=origin/main
check-testcase-files:
	@python3 scripts/check_testcase_files.py
.PHONY: check-testcase-files

check-testcase-files-changed:
	@python3 scripts/check_testcase_files.py --changed-since $(or $(BASE_REF),main)
.PHONY: check-testcase-files-changed

test-check-testcase-files:
	@python3 -m unittest scripts/test_check_testcase_files.py
.PHONY: test-check-testcase-files

verify-scripts:
	@echo "Verifying shell script syntax..."
	@FAILED=0; \
	for script in $$(find scripts tests/integration -type f -name "*.sh" 2>/dev/null); do \
		echo "Checking: $$script"; \
		if bash -n "$$script" 2>&1; then \
			echo "  ✓ PASSED"; \
		else \
			echo "  ✗ FAILED"; \
			FAILED=1; \
		fi; \
	done; \
	if [ $$FAILED -eq 1 ]; then \
		echo "Some script syntax checks failed"; \
		exit 1; \
	else \
		echo "All shell scripts have valid syntax"; \
	fi
	$(MAKE) shellcheck
.PHONY: verify-scripts

shellcheck:
	@echo "Running shellcheck on shell scripts (errors only)..."
	@if ! command -v shellcheck > /dev/null 2>&1; then \
		echo "Warning: shellcheck not installed, skipping"; \
		exit 0; \
	fi; \
	FAILED=0; \
	for script in $$(find scripts tests/integration -type f -name "*.sh" 2>/dev/null); do \
		echo "Checking: $$script"; \
		if shellcheck -S error "$$script" > /dev/null 2>&1; then \
			echo "  ✓ PASSED"; \
		else \
			echo "  ✗ FAILED"; \
			shellcheck -S error "$$script" 2>&1 | head -10; \
			FAILED=1; \
		fi; \
	done; \
	if [ $$FAILED -eq 1 ]; then \
		echo "Some shellcheck validations failed"; \
		exit 1; \
	else \
		echo "All shell scripts pass shellcheck"; \
	fi
.PHONY: shellcheck

# Canonical schema paths:
# - Legacy schemas: schemas/*.schema.json (backward compatible)
# - Versioned schemas: schemas/tcms/*.schema.v1.json (recommended for new development)
test-e2e-validate-yaml: build-validate-yaml
	cargo run --bin validate-yaml -- --schema schemas/tcms-schemas/tcms/test-case-main.schema.json tests/sample/gsma_4.4.2.2_TC.yml >/dev/null 2>&1
	! cargo run --bin validate-yaml -- --schema schemas/tcms-schemas/tcms/test-case-main.schema.json tests/sample/data.yml >/dev/null 2>&1
	./tests/integration/test_validate_yaml_multi_e2e.sh
	./tests/integration/test_validate_yaml_watch_e2e.sh
.PHONY: test-e2e-validate-yaml

test-e2e-auto-schema:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/test_auto_schema_validation_e2e.sh
.PHONY: test-e2e-auto-schema

docker-build:
	${MAKE} README_INSTALL_AUTOMATED.md
	docker build -t testcase-manager:latest .
.PHONY: docker-build

docker-run:
	docker run -v $(PWD)/testcases:/app/testcases testcase-manager:latest
.PHONY: docker-run

test-verify-sample:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/test_verify_e2e.sh
.PHONY: test-verify-sample

# Uses canonical legacy schema path: schemas/tcms-schemas/tcms/test-case-main.schema.json
validate-all-testcases: build-validate-yaml
	SCHEMA_FILE=schemas/tcms-schemas/tcms/test-case-main.schema.json ./scripts/validate-files.sh --pattern '\.ya?ml$$' --validator ./scripts/validate-yaml-wrapper.sh
.PHONY: validate-all-testcases

# Validates all test case YAML files using their declared schema field.
# Each YAML file must have a 'schema' field that identifies the schema to validate against.
# The validate-yaml binary auto-resolves the schema path from the URI in the YAML file.
verify-testcases: build-validate-yaml
	@echo "Verifying test case files against their declared schemas..."
	@FAILED=0; \
	VALIDATED=0; \
	for file in $$(find testcases tests/sample data -type f \( -name "*.yml" -o -name "*.yaml" \) -not \( -path "*/expected_output_reports/*" -o -path "*/testcase_results_container/*" -o -path "*/generated_samples/*" -o -path "*/verifier_scenarios_incorrect/*" -o -name "*te.y*" -o -iname "sample_test_runs.yaml" -o -name "*wrong*" -o -name "data.yml" -o -name "steps-in-json.yml" -o -name "1.yaml" -o -name "SGP.22_4.4.2.yaml" -o -name "CONDITIONAL_VERIFICATION_001.yml" -o -name "doc_gen_*.yml" -o -name "*container*" -o -path "*test_case_result*" -o -path "*test_result_01*" \) 2>/dev/null); do \
		VALIDATED=$$((VALIDATED + 1)); \
		if ! grep -q "^schema:" "$$file" 2>/dev/null; then \
			echo "✗ $$file - Missing required 'schema' field"; \
			FAILED=1; \
			continue; \
		fi; \
		if ./target/debug/validate-yaml "$$file" >/dev/null 2>&1; then \
			echo "✓ $$file"; \
		else \
			echo "✗ $$file - Schema validation failed"; \
			FAILED=1; \
		fi; \
	done; \
	if [ $$FAILED -eq 1 ]; then \
		echo ""; \
		echo "Validation failed: Some test case files have missing or invalid schemas"; \
		exit 1; \
	else \
		echo ""; \
		echo "All $$VALIDATED test case files have valid schemas"; \
	fi; \
	echo ""; \
	echo "Verifying JSON schema files themselves..."; \
	cargo run -p validate-json -- schemas/tcms-schemas/tcms/test-case-main.schema.json >/dev/null 2>&1 || echo "⚠ Schema validation deferred to JSON validator"

.PHONY: verify-testcases

# Validates output samples in schemas/tcms/samples/ and testcases/examples/expected_test_results/
validate-output-schemas:
	@echo "Validating expected output sample files against schemas..."
	./scripts/validate-output-schemas.sh
.PHONY: validate-output-schemas

# Validates versioned schemas in schemas/tcms/*.schema.v1.json
validate-envelope-schemas:
	@echo "Validating TCMS envelope schemas..."
	./scripts/validate_envelope_schemas.sh
.PHONY: validate-envelope-schemas

watch: build-validate-yaml
	./scripts/watch-yaml-files.sh
.PHONY: watch

# Uses canonical legacy schema path: schemas/tcms-schemas/tcms/test-case-main.schema.json
watch-verbose: build-validate-yaml
	SCHEMA_FILE=schemas/tcms-schemas/tcms/test-case-main.schema.json ./scripts/validate-files.sh --pattern '\.ya?ml$$' --validator ./scripts/validate-yaml-wrapper.sh --watch --verbose
.PHONY: watch-verbose

clean-validation-cache:
	rm -rf .validation-cache/
.PHONY: clean-validation-cache

run-test-executor: build-test-executor
	cargo run --bin test-executor
.PHONY: run-test-executor

test-executor-sample: build-test-executor
	@echo "Testing test-executor against sample test cases..."
	@echo "Generating script from gsma_4.4.2.2_TC.yml..."
	cargo run --bin test-executor -- generate tests/sample/gsma_4.4.2.2_TC.yml >/dev/null
	@echo "✓ Script generation verified for gsma_4.4.2.2_TC.yml"
	@echo "Generating script from SGP.22_4.4.2.yaml..."
	cargo run --bin test-executor -- generate tests/sample/SGP.22_4.4.2.yaml >/dev/null
	@echo "✓ Script generation verified for SGP.22_4.4.2.yaml"
	@echo "All test-executor sample verifications passed!"
.PHONY: test-executor-sample

test-e2e-executor:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/test_executor_e2e.sh
.PHONY: test-e2e-executor

test-e2e-orchestrator:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/test_orchestrator_e2e.sh
	cargo run --bin test-orchestrator run testcases/SELF_VALIDATED_EXAMPLE_002.yml --verbose
	! cargo run --bin test-orchestrator run testcases/self_validated_example_wrong.yml
.PHONY: test-e2e-orchestrator

test-e2e-orchestrator-examples:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./tests/integration/test_orchestrator_examples.sh
.PHONY: test-e2e-orchestrator-examples

test-e2e-pipeline:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./crates/testcase-manager/tests/integration/test_pipeline_e2e.sh
.PHONY: test-e2e-pipeline

test-e2e-pipeline-batch:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./crates/testcase-manager/tests/integration/test_pipeline_batch.sh
.PHONY: test-e2e-pipeline-batch

generate-docs:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./scripts/generate_documentation_reports.sh
.PHONY: generate-docs

generate-docs-all:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./scripts/generate_documentation_reports.sh --logs-dir testcases --test-case-dir testcases
.PHONY: generate-docs-all

generate-docs-coverage: setup-python-for-test
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./scripts/generate_documentation_coverage_report.sh
.PHONY: generate-docs-coverage

test-container-compat:
	@${MAKE} build BASE_REF=$(or $(BASE_REF),main)
	./scripts/test_container_yaml_compatibility.sh
.PHONY: test-container-compat

# Download the test-plan-documentation-generator (TPDG) release used by the acceptance suite into
# tools/tpdg/tpdg (linux-amd64 only). Needs a GitLab token with read_api on the TPDG project:
#   TPDG_TOKEN=glpat-... make download-tpdg [TPDG_VERSION=v0.1.0-rc.1]
# acceptance-test uses tools/tpdg/tpdg automatically when TEST_PLAN_DOC_GEN is not set.
download-tpdg:
	@TPDG_VERSION="$(or $(TPDG_VERSION),v0.1.0-rc.1)" ./scripts/download-tpdg.sh
.PHONY: download-tpdg

acceptance-test: build-acceptance-binaries
	@echo "========================================="
	@echo "Running Acceptance Test Suite"
	@echo "========================================="
	@echo ""
	@mkdir -p test-acceptance/reports
	@LOG_FILE="test-acceptance/reports/acceptance_suite_execution.log"; \
	echo "Execution log: $$LOG_FILE"; \
	echo ""; \
	if [ -z "$$TEST_PLAN_DOC_GEN" ] && [ -x tools/tpdg/tpdg ]; then \
		export TEST_PLAN_DOC_GEN="$$PWD/tools/tpdg/tpdg"; \
	fi; \
	if ! command -v test-plan-documentation-generator > /dev/null 2>&1 && [ -z "$$TEST_PLAN_DOC_GEN" ]; then \
		echo "ERROR: test-plan-documentation-generator (TPDG) not found"; \
		echo ""; \
		echo "TPDG is required for acceptance tests."; \
		echo ""; \
		echo "Install options:"; \
		echo "  1. Download the release (needs a token): TPDG_TOKEN=... make download-tpdg"; \
		echo ""; \
		echo "  2. Install globally:"; \
		echo "     cargo install test-plan-documentation-generator"; \
		echo ""; \
		echo "  3. Set TEST_PLAN_DOC_GEN environment variable:"; \
		echo "     export TEST_PLAN_DOC_GEN=/path/to/test-plan-documentation-generator"; \
		echo ""; \
		exit 1; \
	fi; \
	if [ -n "$$TEST_PLAN_DOC_GEN" ]; then \
		echo "Using TPDG from: $$TEST_PLAN_DOC_GEN"; \
	else \
		echo "Using TPDG from PATH: $$(which test-plan-documentation-generator)"; \
	fi; \
	echo ""; \
	if ./test-acceptance/run_acceptance_suite.sh 2>&1 | tee "$$LOG_FILE"; then \
		echo ""; \
		echo "=========================================" | tee -a "$$LOG_FILE"; \
		echo "Acceptance Test Suite: SUCCESS" | tee -a "$$LOG_FILE"; \
		echo "=========================================" | tee -a "$$LOG_FILE"; \
		echo "" | tee -a "$$LOG_FILE"; \
		echo "Full execution log: $$LOG_FILE" | tee -a "$$LOG_FILE"; \
		echo "Summary report: test-acceptance/reports/acceptance_suite_summary.txt" | tee -a "$$LOG_FILE"; \
		exit 0; \
	else \
		EXIT_CODE=$$?; \
		echo ""; \
		echo "=========================================" | tee -a "$$LOG_FILE"; \
		echo "Acceptance Test Suite: FAILED" | tee -a "$$LOG_FILE"; \
		echo "=========================================" | tee -a "$$LOG_FILE"; \
		echo "" | tee -a "$$LOG_FILE"; \
		echo "Full execution log: $$LOG_FILE" | tee -a "$$LOG_FILE"; \
		echo "Summary report: test-acceptance/reports/acceptance_suite_summary.txt" | tee -a "$$LOG_FILE"; \
		echo "" | tee -a "$$LOG_FILE"; \
		echo "Review the logs above for details on failures." | tee -a "$$LOG_FILE"; \
		exit 1; \
	fi
.PHONY: acceptance-test

test-e2e-acceptance: build-acceptance-binaries
	@echo "========================================="
	@echo "Running Acceptance Suite E2E Tests"
	@echo "========================================="
	@echo ""
	./test-acceptance/tests/test_acceptance_suite_e2e.sh
.PHONY: test-e2e-acceptance

build-acceptance-binaries:
	@echo "Building required binaries for acceptance tests..."
	@cargo build --package test-executor
	@cargo build --package verifier
	@cargo build --package validate-yaml
	@echo "✓ All required binaries built successfully"
	@echo ""
.PHONY: build-acceptance-binaries

install-loc:
	./scripts/install-loc.sh --local
.PHONY: install-loc

loc:
	./scripts/compute-loc.sh
.PHONY: loc

loc-verbose:
	./scripts/compute-loc.sh --verbose
.PHONY: loc-verbose

loc-json:
	./scripts/compute-loc.sh --format json
.PHONY: loc-json

loc-yaml:
	./scripts/compute-loc.sh --format yaml
.PHONY: loc-yaml

loc-report:
	@mkdir -p reports/loc
	./scripts/compute-loc.sh --output reports/loc/loc_statistics.txt
.PHONY: loc-report

test-rest:
	touch .test-failing .test-failing-tmp
	if [[ $$(wc -l .test-failing | awk '{print $$1}') -eq 0 ]]; then \
		echo "No test-failing/empty found. Adding all"; \
		rm -f .test-failing && touch .test-failing; \
		find crates -maxdepth 1 -type d | cut -d'/' -f2 | grep -v "^\crates" >> .test-failing; \
	fi
	#cat .test-failing
	while IFS= read -r line; do cargo test -p "$$line" || echo "$$line" >> .test-failing-tmp; done < ".test-failing"
	-mv .test-failing-tmp .test-failing
.PHONY: test-rest
