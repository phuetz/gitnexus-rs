#!/usr/bin/env bash
# Run the full query set under --hybrid (BM25 + embedding RRF, no LLM rerank).
# Output: bench/hybrid.md
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"
BIN="$REPO_ROOT/target/release/gitnexus"
[ -x "$BIN.exe" ] && BIN="$BIN.exe"

OUT="$HERE/hybrid.md"
QUERIES="$HERE/queries.txt"

{
  echo "# Hybrid (BM25 + semantic RRF) — gitnexus-rs repo"
  echo ""
  echo "Indexed at: $(cat "$REPO_ROOT/.gitnexus/meta.json" | grep indexedAt | head -1)"
  echo "Embeddings: all-MiniLM-L6-v2 (384d, 5293 symbols, 8.2MB)"
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
  # Filter out the ort INFO logs (noisy memory allocator traces) so the file stays readable.
  "$BIN" query "$query" --hybrid --limit 5 --repo "$REPO_ROOT" 2>&1 \
    | grep -v "ort::logging" \
    | tail -15 >> "$OUT" || echo "(query failed)" >> "$OUT"
  echo '```' >> "$OUT"
  echo '' >> "$OUT"
  echo "Q$i done" >&2
done < "$QUERIES"

echo "Hybrid run written to $OUT" >&2
