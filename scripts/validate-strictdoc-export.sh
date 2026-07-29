#!/usr/bin/env bash
set -e

# Get script directory and source logger
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

STRICTDOC_SOURCE_DIR="docs/strictdoc"
STRICTDOC_OUTPUT_DIR="output/strictdoc"
STRICTDOC_HTML_DIR="${STRICTDOC_OUTPUT_DIR}/html"

section "StrictDoc Export Validation"

# Check if source directory exists
if [ ! -d "$STRICTDOC_SOURCE_DIR" ]; then
    fail "Source directory not found: $STRICTDOC_SOURCE_DIR"
    exit 1
fi

log_info "Exporting StrictDoc documentation..."

# Run strictdoc export
if uv run strictdoc export "$STRICTDOC_SOURCE_DIR" --formats=html --output-dir "$STRICTDOC_OUTPUT_DIR" > /dev/null 2>&1; then
    pass "StrictDoc export completed successfully"
else
    fail "StrictDoc export failed"
    exit 1
fi

# Check if output directory was created
if [ ! -d "$STRICTDOC_HTML_DIR" ]; then
    fail "Output HTML directory not found: $STRICTDOC_HTML_DIR"
    exit 1
fi

# Check if index.html was created
if [ ! -f "$STRICTDOC_HTML_DIR/index.html" ]; then
    fail "Index HTML file not found: $STRICTDOC_HTML_DIR/index.html"
    exit 1
fi

# Count generated HTML files
html_count=$(find "$STRICTDOC_HTML_DIR" -name "*.html" -type f | wc -l)
if [ "$html_count" -eq 0 ]; then
    fail "No HTML files were generated"
    exit 1
fi

pass "Generated $html_count HTML files"
pass "StrictDoc documentation is valid"
