# Incremental Build and Test Targets
# This file contains targets for incremental builds and tests based on git changes
# These targets detect changed crates and only build/test what's necessary

# ============================================================================
# Helper Targets
# ============================================================================

# list-changed-files: Lists all files that have changed between BASE_REF and HEAD
# Parameters:
#   BASE_REF - Git reference to compare against (default: main)
#              Examples: main, develop, HEAD~1, abc123
# Usage:
#   make list-changed-files BASE_REF=main
#   make list-changed-files BASE_REF=abc123
list-changed-files:
	@BASE_REF=$(or $(BASE_REF),main); \
	echo "Changed files between $$BASE_REF and current state:"; \
	echo ""; \
	if [ "$$BASE_REF" = "HEAD" ]; then \
		git diff --name-only 2>/dev/null || echo ""; \
		git diff --cached --name-only 2>/dev/null || echo ""; \
	else \
		git diff --name-only "$$BASE_REF...HEAD" 2>/dev/null || echo ""; \
		git diff --name-only 2>/dev/null || echo ""; \
		git diff --cached --name-only 2>/dev/null || echo ""; \
	fi | sort -u
.PHONY: list-changed-files

# list-affected-crates: Lists crates affected by changes (including reverse dependencies)
# Parameters:
#   BASE_REF - Git reference to compare against (default: main)
#              Examples: main, develop, HEAD~1, abc123
# Behavior:
#   - Detects changed files in the working directory
#   - Maps changed files to crates
#   - Finds all crates that depend on changed crates (transitive)
#   - Outputs space-separated list of affected crate names
# Usage:
#   make list-affected-crates BASE_REF=main
#   make list-affected-crates BASE_REF=abc123
list-affected-crates:
	@BASE_REF=$(or $(BASE_REF),main); \
	echo "Detecting affected crates compared to $$BASE_REF..."; \
	echo ""; \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No affected crates detected"; \
	else \
		echo "Affected crates:"; \
		echo "$$AFFECTED" | tr ' ' '\n' | sed 's/^/  - /'; \
	fi
.PHONY: list-affected-crates

# ============================================================================
# Build Targets
# ============================================================================

# build-from: Incrementally build only affected crates
# Parameters:
#   BASE_REF - Git reference to compare against (default: main)
#              Examples: main, develop, HEAD~1, abc123
# Behavior:
#   1. Detects changed files between BASE_REF and current state
#   2. Identifies affected crates (including reverse dependencies)
#   3. Builds each affected crate using cargo build -p <crate>
#   4. Skips tests (build-only mode)
# Exit codes:
#   0 - All builds succeeded or no changes detected
#   1 - One or more builds failed
# Usage:
#   make build-from BASE_REF=main
#   make build-from BASE_REF=develop
#   make build-from BASE_REF=HEAD~3
build-from:
	@echo "========================================"
	@echo "Incremental Build (from BASE_REF)"
	@echo "========================================"
	@BASE_REF=$(or $(BASE_REF),main); \
	echo "Base reference: $$BASE_REF"; \
	echo ""; \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No affected crates detected - nothing to build"; \
		exit 0; \
	fi; \
	echo "Building affected crates..."; \
	echo ""; \
	FAILED=0; \
	for crate in $$AFFECTED; do \
		echo "Building: $$crate"; \
		if cargo build -p "$$crate" 2>&1 | tail -20; then \
			echo "  ✓ Build succeeded: $$crate"; \
		else \
			echo "  ✗ Build failed: $$crate"; \
			FAILED=1; \
		fi; \
		echo ""; \
	done; \
	if [ $$FAILED -eq 0 ]; then \
		echo "========================================"; \
		echo "All builds completed successfully"; \
		echo "========================================"; \
	else \
		echo "========================================"; \
		echo "Some builds failed"; \
		echo "========================================"; \
		exit 1; \
	fi
.PHONY: build-from

# ============================================================================
# Test Targets
# ============================================================================

# test-from: Incrementally test only affected crates
# Parameters:
#   BASE_REF - Git reference to compare against (default: main)
#              Examples: main, develop, HEAD~1, abc123
# Behavior:
#   1. Detects changed files between BASE_REF and current state
#   2. Identifies affected crates (including reverse dependencies)
#   3. For each affected crate:
#      a. Builds the crate: cargo build -p <crate>
#      b. Runs unit tests: cargo test -p <crate>
#      c. Runs relevant E2E integration tests based on binary mapping
#   4. Aggregates and reports results
# Exit codes:
#   0 - All builds and tests passed or no changes detected
#   1 - One or more builds or tests failed
# Usage:
#   make test-from BASE_REF=main
#   make test-from BASE_REF=develop
#   make test-from BASE_REF=abc123
#
# Crate to Binary Mapping:
#   validate-yaml -> validate-yaml
#   validate-json -> validate-json
#   verifier -> verifier
#   test-executor -> test-executor
#   test-orchestrator -> test-orchestrator
#   test-run-manager -> trm
#   test-verify -> test-verify
#   script-cleanup -> script-cleanup
#   json-escape -> json-escape
#   json-to-yaml -> json-to-yaml
#   editor -> editor
#   audit-verifier -> audit-verifier, sign-audit-log, verify-audit-log, verify-audit-signature
#   testcase-manager -> testcase-manager
#   tpdg-compat -> test-plan-documentation-generator-compat
#
# Binary to E2E Test Mapping:
#   validate-yaml -> test_validate_yaml_*.sh, test_auto_schema_validation_e2e.sh
#   test-executor -> test_executor_e2e.sh, test_variable_passing_e2e.sh, test_jq_json_escaping_e2e.sh, etc.
#   test-orchestrator -> test_orchestrator_e2e.sh
#   verifier -> test_verifier_*.sh, run_verifier_and_generate_reports.sh
#   audit-verifier -> audit-verifier tests in crates/audit-verifier/tests/
#   json-escape -> test_json_escape_e2e.sh
#   testcase-manager -> smoke_test.sh, test_bdd_e2e.sh, etc.
test-from:
	@echo "========================================"
	@echo "Incremental Test (from BASE_REF)"
	@echo "========================================"
	@BASE_REF=$(or $(BASE_REF),main); \
	echo "Base reference: $$BASE_REF"; \
	echo ""; \
	AFFECTED=$$(./scripts/detect-local-changes.sh "$$BASE_REF" 2>/dev/null || echo ""); \
	if [ -z "$$AFFECTED" ]; then \
		echo "No affected crates detected - nothing to test"; \
		exit 0; \
	fi; \
	echo "Testing affected crates..."; \
	echo ""; \
	FAILED=0; \
	E2E_TESTS_FILE=$$(mktemp); \
	for crate in $$AFFECTED; do \
		echo "========================================"; \
		echo "Processing crate: $$crate"; \
		echo "========================================"; \
		echo ""; \
		echo "[1/3] Building $$crate..."; \
		if ! cargo build -p "$$crate" 2>&1 | tail -20; then \
			echo "  ✗ Build failed: $$crate"; \
			FAILED=1; \
			continue; \
		fi; \
		echo "  ✓ Build succeeded: $$crate"; \
		echo ""; \
		echo "[2/3] Running unit tests for $$crate..."; \
		if ! cargo test -p "$$crate" 2>&1 | tail -30; then \
			echo "  ✗ Unit tests failed: $$crate"; \
			FAILED=1; \
		else \
			echo "  ✓ Unit tests passed: $$crate"; \
		fi; \
		echo ""; \
		echo "[3/3] Collecting E2E tests for $$crate..."; \
		case "$$crate" in \
			validate-yaml) \
				echo "crates/testcase-manager/tests/integration/test_validate_yaml_watch_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_validate_yaml_multi_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_validate_yaml_schema_watch_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_validate_yaml_transitive_schema_watch_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_auto_schema_validation_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "  validate-yaml binary -> 5 E2E tests"; \
				;; \
			test-executor) \
				echo "crates/testcase-manager/tests/integration/test_executor_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_variable_passing_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_conditional_verification_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_manual_steps_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_jq_json_escaping_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_pipeline_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "  test-executor binary -> 6 E2E tests"; \
				;; \
			test-orchestrator) \
				echo "crates/testcase-manager/tests/integration/test_orchestrator_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "  test-orchestrator binary -> 1 E2E test"; \
				;; \
			verifier) \
				echo "crates/testcase-manager/tests/integration/test_verifier_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_verifier_container_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_verifier_edge_cases_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_verifier_behaviour_changes_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "scripts/run_verifier_and_generate_reports.sh" >> $$E2E_TESTS_FILE; \
				echo "  verifier binary -> 4 E2E tests"; \
				;; \
			audit-verifier) \
				echo "crates/audit-verifier/tests/integration/test_audit_logging_demo_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/audit-verifier/tests/integration/test_audit_verifier_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/audit-verifier/tests/integration/test_audit_key_scenarios.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/audit-verifier/tests/integration/test_simple_workflow.sh" >> $$E2E_TESTS_FILE; \
				echo "  audit-verifier binaries -> 4 E2E tests"; \
				;; \
			json-escape) \
				echo "crates/testcase-manager/tests/integration/test_json_escape_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "  json-escape binary -> 1 E2E test"; \
				;; \
			testcase-manager) \
				echo "crates/testcase-manager/tests/integration/smoke_test.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_bdd_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_manual_verification_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_variable_display_e2e.sh" >> $$E2E_TESTS_FILE; \
				echo "crates/testcase-manager/tests/integration/test_documentation_generation.sh" >> $$E2E_TESTS_FILE; \
				echo "  testcase-manager binary -> 5 E2E tests"; \
				;; \
			*) \
				echo "  No E2E test mapping for $$crate (library crate or no tests defined)"; \
				;; \
		esac; \
		echo ""; \
	done; \
	if [ -s "$$E2E_TESTS_FILE" ]; then \
		echo "========================================"; \
		echo "Running E2E Integration Tests"; \
		echo "========================================"; \
		echo ""; \
		E2E_COUNT=$$(cat "$$E2E_TESTS_FILE" | sort -u | wc -l | tr -d ' '); \
		echo "Found $$E2E_COUNT unique E2E test script(s) to run"; \
		echo ""; \
		E2E_FAILED=0; \
		cat "$$E2E_TESTS_FILE" | sort -u | while read test_script; do \
			if [ ! -f "$$test_script" ]; then \
				echo "  ⚠ Skipping (not found): $$test_script"; \
				continue; \
			fi; \
			echo "Running: $$test_script"; \
			if bash "$$test_script" 2>&1 | tail -50; then \
				echo "  ✓ E2E test passed: $$test_script"; \
			else \
				echo "  ✗ E2E test failed: $$test_script"; \
				echo "1" > "$$E2E_TESTS_FILE.failed"; \
			fi; \
			echo ""; \
		done; \
		if [ -f "$$E2E_TESTS_FILE.failed" ]; then \
			FAILED=1; \
			rm -f "$$E2E_TESTS_FILE.failed"; \
		fi; \
	else \
		echo "========================================"; \
		echo "No E2E tests to run"; \
		echo "========================================"; \
		echo ""; \
	fi; \
	rm -f "$$E2E_TESTS_FILE"; \
	echo "========================================"; \
	if [ $$FAILED -eq 0 ]; then \
		echo "All tests completed successfully"; \
	else \
		echo "Some tests failed"; \
	fi; \
	echo "========================================"; \
	exit $$FAILED
.PHONY: test-from
