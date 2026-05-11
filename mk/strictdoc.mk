# ============================================================================
# StrictDoc Makefile targets
# ============================================================================
# This file contains targets for managing StrictDoc DO-178C documentation.
#
# Targets:
#   strictdoc-download-template  - Download DO-178C template from GitHub
#   strictdoc-export             - Export documentation to HTML
#   strictdoc-validate           - Validate that export succeeds
#   strictdoc-server             - Start web server for editing
#   strictdoc-clean              - Remove generated output

STRICTDOC_SOURCE_DIR := docs/strictdoc
STRICTDOC_OUTPUT_DIR := output/strictdoc
STRICTDOC_TEMPLATE_URL := https://raw.githubusercontent.com/strictdoc-project/strictdoc-templates/main/templates/DO-178C

# List of DO-178C template files to download
TEMPLATE_FILES := \
	system/SR.sdoc \
	software/SAS.sdoc \
	software/plans/PSAC.sdoc \
	software/plans/SDP.sdoc \
	software/plans/SVP.sdoc \
	software/plans/SCMP.sdoc \
	software/plans/SQAP.sdoc \
	software/requirements/SRD.sdoc \
	software/requirements/SDD.sdoc \
	software/requirements/DD.rst \
	software/standard/SRS.sdoc \
	software/standard/SDS.sdoc \
	software/standard/SCS.sdoc \
	software/verification/SVCP.sdoc \
	software/verification/SVR.sdoc

.PHONY: strictdoc-download-template
strictdoc-download-template:
	@echo "Downloading DO-178C template from GitHub..."
	@mkdir -p $(STRICTDOC_SOURCE_DIR)
	@for file in $(TEMPLATE_FILES); do \
		dir=$$(dirname "$(STRICTDOC_SOURCE_DIR)/$$file"); \
		mkdir -p "$$dir"; \
		echo "  Downloading $$file..."; \
		curl -sS "$(STRICTDOC_TEMPLATE_URL)/$$file" -o "$(STRICTDOC_SOURCE_DIR)/$$file" || { echo "Failed to download $$file"; exit 1; }; \
	done
	@echo "✓ DO-178C template downloaded successfully"
	@echo "Run: git add $(STRICTDOC_SOURCE_DIR)/ && git commit -m 'Add DO-178C strictdoc template'"

.PHONY: strictdoc-export
strictdoc-export:
	@echo "Exporting StrictDoc documentation to HTML..."
	@uv run strictdoc export $(STRICTDOC_SOURCE_DIR) --formats=html --output-dir $(STRICTDOC_OUTPUT_DIR)
	@echo "✓ Export completed: $(STRICTDOC_OUTPUT_DIR)/html/"

.PHONY: strictdoc-validate
strictdoc-validate: strictdoc-clean
	@./scripts/validate-strictdoc-export.sh

.PHONY: strictdoc-server
strictdoc-server:
	@echo "Starting StrictDoc server..."
	@echo "Open http://localhost:5111 in your browser"
	@uv run strictdoc server $(STRICTDOC_SOURCE_DIR)

.PHONY: strictdoc-clean
strictdoc-clean:
	@if [ -d "$(STRICTDOC_OUTPUT_DIR)" ]; then \
		echo "Cleaning StrictDoc output..."; \
		rm -rf "$(STRICTDOC_OUTPUT_DIR)"; \
	fi
