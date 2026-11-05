#!/usr/bin/env bash
# Regenerates the traceability documents (AsciiDoc and Markdown, one per test case, plus the consolidated documents)
# of a sut-1 checkout with the sut-1 pipeline script, checks them, and compiles every AsciiDoc file with asciidoctor.
# The templates are the TPDG ones in sut-1 (vv-tcms/templates/), used as they are.
#
# The documents are written to OUT (default: build/traceability-docs, which is gitignored). Nothing here commits
# them; they are regenerated on demand.
#
# Environment:
#   SUT1_DIR              required: a sut-1 checkout (the test cases and the run with execution logs)
#   TEST_PLAN_DOC_GEN     required: the tpdg binary (scripts/download-tpdg.sh of tcms-cli fetches it)
#   TCMS_BIN_DIR          folder with test-executor, verifier and validate-yaml (see sut-1 scripts/tcms-env.sh)
#   TCMS_SRC_DIR          tcms-cli source tree with schemas/ and data/ (see sut-1 scripts/tcms-env.sh)
#   ASCIIDOCTOR           asciidoctor command (default: asciidoctor on PATH; npm install asciidoctor provides it)
#   OUT                   output folder (default: <tcms-cli>/build/traceability-docs)
#   TCMS_DOC_REQUEST_ID   request id written to the manifest (default: local-traceability)
#   TCMS_DOC_RUN_ID       optional run to document (default: newest run with execution logs)
#
# Exit code: 0 when the documents were generated, verified and compiled; 1 otherwise.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SUT1_DIR="${SUT1_DIR:?set SUT1_DIR to a sut-1 checkout}"
: "${TEST_PLAN_DOC_GEN:?set TEST_PLAN_DOC_GEN to the tpdg binary}"
ASCIIDOCTOR="${ASCIIDOCTOR:-asciidoctor}"
OUT="${OUT:-$REPO_ROOT/build/traceability-docs}"
export TCMS_DOC_REQUEST_ID="${TCMS_DOC_REQUEST_ID:-local-traceability}"

[ -x "$TEST_PLAN_DOC_GEN" ] || { echo "error: tpdg not executable: $TEST_PLAN_DOC_GEN" >&2; exit 1; }
command -v "$ASCIIDOCTOR" > /dev/null || { echo "error: asciidoctor not found ($ASCIIDOCTOR)" >&2; exit 1; }

echo "Generating the traceability documents from $SUT1_DIR"
(
  cd "$SUT1_DIR"
  scripts/traceability-document.sh
  scripts/verify-documents.sh build/traceability/30_documentation_source
)

SOURCE="$SUT1_DIR/build/traceability/30_documentation_source"
failures=0
compile() { # compile <adoc>: asciidoctor must succeed with no warning on stderr
  local file="$1" err
  err="$(mktemp)"
  if "$ASCIIDOCTOR" -S safe -o /dev/null "$file" 2> "$err" && [ ! -s "$err" ]; then
    echo "  ok    compiles: ${file#"$SOURCE"/}"
  else
    echo "  FAIL  compiles: ${file#"$SOURCE"/}"
    sed 's/^/        /' "$err"
    failures=$((failures + 1))
  fi
  rm -f "$err"
}
for adoc in "$SOURCE"/asciidoc/*.adoc "$SOURCE"/consolidated/*.adoc; do
  compile "$adoc"
done

rm -rf "$OUT"
mkdir -p "$(dirname "$OUT")"
cp -R "$SOURCE" "$OUT"
echo "Documents: $OUT"
[ "$failures" -eq 0 ] || { echo "$failures document(s) do not compile" >&2; exit 1; }
