#!/usr/bin/env bash
# Run the baseline queries with --rerank to measure the LLM reranker impact.
# Output: bench/rerank.md with top-5 for each query (LLM-reranked from BM25 top-20).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"
BIN="$REPO_ROOT/target/release/gitnexus"

[ -x "$BIN.exe" ] && BIN="$BIN.exe"
if [ ! -x "$BIN" ]; then
  echo "ERROR: $BIN not found — cargo build --release -p gitnexus-cli first" >&2
  exit 1
fi

OUT="$HERE/rerank.md"
QUERIES="$HERE/queries.txt"

{
  echo "# Rerank (LLM) — gitnexus-rs repo"
  echo ""
  echo "Indexed at: $(cat "$REPO_ROOT/.gitnexus/meta.json" | grep indexedAt | head -1)"
  echo "Nodes: $(cat "$REPO_ROOT/.gitnexus/meta.json" | grep -o '"nodes":[[:space:]]*[0-9]*' | grep -o '[0-9]*')"
  echo "Search: BM25 top-20 pool -> LlmReranker (Gemini 2.5 Flash) -> top-5"
  echo ""
  echo "Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo ""
} > "$OUT"

i=0
while IFS= read -r query; do
  [ -z "$query" ] && continue
  i=$((i+1))
  echo "## Q$i — \"$query\"" >> "$OUT"
  echo '' >> "$OUT"
  echo '```' >> "$OUT"
  # stdout = results; stderr = tracing warnings (503 retries). We capture both
  # so readers see when the reranker fell back or retried.
  "$BIN" query "$query" --rerank --limit 5 --repo "$REPO_ROOT" 2>&1 | tail -25 >> "$OUT" || echo "(query failed)" >> "$OUT"
  echo '```' >> "$OUT"
  echo '' >> "$OUT"
  echo "Q$i done" >&2
done < "$QUERIES"

echo "Rerank run written to $OUT" >&2
