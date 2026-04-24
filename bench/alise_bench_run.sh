#!/usr/bin/env bash
# Run the Alise_v2 French query set under --hybrid.
# Target repo: D:/taf/Alise_v2 (must be indexed + embedded).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"
BIN="$REPO_ROOT/target/release/gitnexus"
[ -x "$BIN.exe" ] && BIN="$BIN.exe"

ALISE="D:/taf/Alise_v2"
if [ ! -f "$ALISE/.gitnexus/embeddings.bin" ]; then
  echo "ERROR: $ALISE/.gitnexus/embeddings.bin missing — run 'gitnexus embed' first" >&2
  exit 1
fi

OUT="$HERE/alise_hybrid.md"
QUERIES="$HERE/queries_alise.txt"

{
  echo "# Alise_v2 hybrid search bench (French corpus)"
  echo ""
  echo "Repo: $ALISE"
  echo "Model: paraphrase-multilingual-MiniLM-L12-v2 (384d, multilingue)"
  echo "Indexed at: $(cat "$ALISE/.gitnexus/meta.json" | grep indexedAt | head -1)"
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
  echo '### BM25 seul' >> "$OUT"
  echo '```' >> "$OUT"
  "$BIN" query "$query" --limit 5 --repo "$ALISE" 2>&1 \
    | grep -v "ort::logging" \
    | tail -10 >> "$OUT" || echo "(query failed)" >> "$OUT"
  echo '```' >> "$OUT"
  echo '' >> "$OUT"

  echo '### +Hybrid (BM25 + multilingual RRF)' >> "$OUT"
  echo '```' >> "$OUT"
  "$BIN" query "$query" --hybrid --limit 5 --repo "$ALISE" 2>&1 \
    | grep -v "ort::logging" \
    | tail -10 >> "$OUT" || echo "(query failed)" >> "$OUT"
  echo '```' >> "$OUT"
  echo '' >> "$OUT"

  echo "Q$i done" >&2
done < "$QUERIES"

echo "Alise bench written to $OUT" >&2
