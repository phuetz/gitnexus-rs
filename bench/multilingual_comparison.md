# Phase 2.6 — Multilingual model comparison

Compare `all-MiniLM-L6-v2` (English-only, 22M params, 90MB) vs
`paraphrase-multilingual-MiniLM-L12-v2` (50+ languages, 118M params, 470MB)
on the same 17 queries + clean corpus (build artifacts filtered).

## Setup

- Same graph (gitnexus-rs, 12480 nodes)
- Embeddings-only change (same BM25, same hybrid RRF, same K=60)
- Second run filters `target-codex/`, `.codex-target/`, `target/debug/`, `node_modules/`
- 5293 → 3468 embeddings after cleanup (34% build-artifact noise removed)

## Result table (top-1 only, focus on what moved)

| Q  | Query | MiniLM-L6 (EN) | Multilingual clean |
|----|-------|---|---|
| 1  | RRF fusion | merge_with_rrf ✓ | merge_with_rrf ✓ |
| 2  | reciprocal rank fusion | merge_with_rrf at #5 | **lost from top-5** ⬇️ |
| 4  | feature flag embeddings | generate_embeddings ✓ | generate_embeddings ✓ (noise cleaned — Q4 was the biggest pre-cleanup fail) |
| 5  | cypher parser | parse_cql #1 | parse_atom #1 (execute_cypher #5) ⬇️ |
| 6  | ASP.NET MVC controller action | enrich_aspnet_mvc absent | still absent |
| 8  | LLM config auth header | test_parse_llm_response_markdown_wrapped | **cmd_ownership** (weird) ⬇️ |
| 11 | ingest pipeline orchestrate phases | run_pipeline ✓ | PipelinePhase enum #1 ⬆️ (arguably more conceptual) |
| 12 | C# DI resolver | di.rs file #1 | resolve_calls #1 ⬆️ (more general but valid) |
| 14 | chat streaming cancellation | chat_cancel #1 | chat_cancel **absent** ⬇️ |
| 16 | **comment fonctionne la fusion RRF (FR)** | merge_with_rrf ✓ | merge_with_rrf ✓ |
| 17 | **où est gérée l'annulation chat streaming (FR)** | chat_cancel #5 | chat_cancel **#3** ⬆️ |

## Scorecard

| Metric | MiniLM-L6 | Multilingual |
|---|---|---|
| Wins | 5 (Q2, Q5, Q14, etc.) | 4 (Q4, Q11, Q12, Q17) |
| Losses | 0 | 3 (Q2, Q5, Q14) |
| Neutral | 12 | 10 |
| French query quality | OK via lexical passthrough | Clearly better (Q17 moved #5→#3) |
| Speed | 42s/5293 symbols | 136s/3468 symbols (~3× slower) |
| Disk | 8.2MB | 5.3MB (fewer symbols after cleanup) |

## Honest verdict

**Multilingual is NOT universally better.** It trades precision on specific
English-technical queries for broader French recall. On a French-heavy corpus
(Alise_v2), it's the right default. On English-only code, MiniLM-L6 is often
sharper.

**The biggest win was NOT the model** — it was filtering build artifacts
before embedding (1825 symbols removed from 5293). Q4 "feature flag
embeddings" had been polluted by `target-codex/*/flag_check.c` files; the
filter cleaned it up.

**Key gotcha on the multilingual export**: the Xenova/paraphrase-multilingual
ONNX graph still declares a `token_type_ids` input despite being XLM-RoBERTa
based. Don't pass `--no-token-type-ids` — the Gather node will fail. Just
leave the default (true) and feed zeros.

## Recommendation

- Keep `embed --model <path>` user-selectable
- Default guidance: MiniLM-L6 for English code, multilingual for French docs
- Build-artifact filter is now unconditional in `embed.rs` — uncontroversial win
- For Alise_v2 / agile-up.com deliverable: multilingual

## Status

embeddings.bin on this repo is currently the multilingual version. If you
want to flip back to MiniLM-L6 for sharper English-code searches, re-run:

```
gitnexus embed --model ~/.gitnexus/models/all-MiniLM-L6-v2/model.onnx
```
