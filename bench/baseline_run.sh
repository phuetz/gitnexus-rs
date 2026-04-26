#!/usr/bin/env bash
# Run the baseline queries against current BM25 (FtsIndex::search via `gitnexus query`).
# Output: bench/baseline.md with top-5 for each query.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"
BIN="$REPO_ROOT/target/release/gitnexus"

if [ ! -x "$BIN" ] && [ ! -x "$BIN.exe" ]; then
  echo "ERROR: $BIN(.exe) not found — cargo build --release -p gitnexus-cli first" >&2
  exit 1
fi

# Use whichever binary form exists on this platform.
[ -x "$BIN.exe" ] && BIN="$BIN.exe"

OUT="$HERE/baseline.md"
QUERIES="$HERE/queries.txt"

{
  echo "# Baseline BM25 — gitnexus-rs repo"
  echo ""
  echo "Indexed at: $(cat "$REPO_ROOT/.gitnexus/meta.json" | grep indexedAt | head -1)"
  echo "Nodes: $(cat "$REPO_ROOT/.gitnexus/meta.json" | grep -o '"nodes":[[:space:]]*[0-9]*' | grep -o '[0-9]*')"
  echo "Search: FtsIndex (BM25 full-text search on name + signature + file_path)"
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
  "$BIN" query "$query" --limit 5 --repo "$REPO_ROOT" 2>&1 | tail -20 >> "$OUT" || echo "(query failed)" >> "$OUT"
  echo '```' >> "$OUT"
  echo '' >> "$OUT"
  echo "Q$i done" >&2
done < "$QUERIES"

echo "Baseline written to $OUT" >&2
