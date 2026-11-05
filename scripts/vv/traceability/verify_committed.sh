#!/usr/bin/env bash
# Verifies the committed traceability documents of the VV fixtures (crates/tcms/tests/fixtures/vv-cases/<binary>/
# <case>/traceability/), without regenerating them:
#   - every document.adoc compiles with asciidoctor and prints no warning;
#   - every document.md compiles with markdown-it (compile_markdown.js: parse, table rows, excess cells);
#   - every validation.json reports no problem, and a skipped folder names its reason;
#   - the counts match: one .adoc and one .md per rendered folder.
#
# Environment:
#   NODE_MODULES   folder with asciidoctor and markdown-it (default: <repo>/.ci-tools/node_modules)
#   ASCIIDOCTOR    asciidoctor command (default: $NODE_MODULES/.bin/asciidoctor)
# Exit code: 0 when everything is verified, 1 otherwise.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
FIXTURES="$REPO_ROOT/crates/tcms/tests/fixtures/vv-cases"
NODE_MODULES="${NODE_MODULES:-$REPO_ROOT/.ci-tools/node_modules}"
ASCIIDOCTOR="${ASCIIDOCTOR:-$NODE_MODULES/.bin/asciidoctor}"

[ -x "$ASCIIDOCTOR" ] || { echo "error: asciidoctor not found at $ASCIIDOCTOR" >&2; exit 1; }
[ -d "$NODE_MODULES/markdown-it" ] || { echo "error: markdown-it not found in $NODE_MODULES" >&2; exit 1; }

failures=0
fail() { echo "FAIL  $1"; failures=$((failures + 1)); }

adoc_files=()
md_files=()
while IFS= read -r f; do adoc_files+=("$f"); done < <(find "$FIXTURES" -path '*/traceability/document.adoc' | sort)
while IFS= read -r f; do md_files+=("$f"); done < <(find "$FIXTURES" -path '*/traceability/document.md' | sort)
validations=()
while IFS= read -r f; do validations+=("$f"); done < <(find "$FIXTURES" -path '*/traceability/validation.json' | sort)

echo "Documents: ${#adoc_files[@]} .adoc, ${#md_files[@]} .md, ${#validations[@]} validation records"
[ "${#adoc_files[@]}" -gt 0 ] || fail "no committed .adoc documents found"
[ "${#adoc_files[@]}" -eq "${#md_files[@]}" ] || fail "${#adoc_files[@]} .adoc but ${#md_files[@]} .md documents"

for f in "${adoc_files[@]}"; do
  err="$(mktemp)"
  if "$ASCIIDOCTOR" -S safe -o /dev/null "$f" 2> "$err" && [ ! -s "$err" ]; then
    :
  else
    fail "asciidoctor: ${f#"$FIXTURES"/}: $(head -c 300 "$err")"
  fi
  rm -f "$err"
done

if [ "${#md_files[@]}" -gt 0 ]; then
  if ! NODE_PATH="$NODE_MODULES" node "$REPO_ROOT/scripts/vv/traceability/compile_markdown.js" "$NODE_MODULES" "${md_files[@]}" > "$REPO_ROOT/.ci-tools/markdown-report.txt" 2>&1; then
    grep '^FAIL' "$REPO_ROOT/.ci-tools/markdown-report.txt" | sed 's/^/FAIL  markdown: /' || true
    failures=$((failures + 1))
  else
    echo "ok    $(tail -1 "$REPO_ROOT/.ci-tools/markdown-report.txt")"
  fi
fi

for v in "${validations[@]}"; do
  if ! python3 - "$v" <<'EOF'
import json, sys
doc = json.load(open(sys.argv[1]))
if "skipped" in doc:
    sys.exit(0)
sys.exit(1 if doc.get("problems") else 0)
EOF
  then
    fail "validation: ${v#"$FIXTURES"/}: $(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('problems'))" "$v" | head -c 300)"
  fi
done

if [ "$failures" -eq 0 ]; then
  echo "OK    all committed traceability documents compile and validate"
else
  echo "$failures check(s) failed" >&2
  exit 1
fi
