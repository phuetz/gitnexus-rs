# Final Results — GitNexus Semantic Search Sprint

Branche : `feat/semantic-search`, 11 commits.
Repo testé : gitnexus-rs lui-même (12480 nodes, 5293 symboles embeddés).
Queries : 17 (15 anglais + 2 français).

## Comparaison 3-way

| Q  | Query | BM25 baseline | +Rerank (LLM) | +Hybrid (embeddings RRF) |
|----|-------|---|---|---|
| 1  | RRF fusion | merge_with_rrf ✓ | = (503 fallback) | = |
| 2  | reciprocal rank fusion | **❌ rank** | = (503 fallback) | **⬆️⬆️ merge_with_rrf en #5** |
| 3  | how is the call graph built | ⚠️ is_empty en #2 | ⬆️⬆️ filtré | ⬆️ struct GraphRelationship en #2 |
| 4  | feature flag embeddings | ⚠️ feature_dev_run #2 | **⬆️⬆️** embed/search_semantic top-3 | ⬆️ mais test_* à #2 |
| 5  | cypher parser | ⚠️ parse_cql #4 | ⬆️⬆️ parse_cql #1 | **⬆️⬆️ parse_cql #1 + parse #2** |
| 6  | ASP.NET MVC controller action | ⚠️ enrich_aspnet_mvc #4 | ⬆️⬆️ enrich_aspnet_mvc #1 | ⬇️ enrich_aspnet_mvc hors top-5 |
| 7  | tree-sitter parsing | ⚠️ query holders × 5 | ⬆️ walk_tree_for_complexity #1 | = (queries holders) |
| 8  | LLM config auth header | ❌ bruit en #1 | **⬆️⬆️ enrich_with_llm #1** | ⬇️ test en #1, LlmResponseChunk #2 |
| 9  | BM25 implementation | search_fts ✓ | = (503) | ⬆️ make_bm25 #1, struct BM25SearchResult #4 |
| 10 | detect dead code | mark_dead_code ✓ | = (503) | = |
| 11 | ingest pipeline phases | run_pipeline ✓ | = (503) | = + PipelineOptions struct |
| 12 | C# DI resolver | extract_di_registrations ✓ | ⬆️ di.rs file #1 | ⬆️ + ResolveCtx struct #2 |
| 13 | snapshot persistence | save_snapshot ✓ | = | ⬇️ snapshot_err #1 (moins pertinent) |
| 14 | chat streaming cancellation | chat_ask #1 | ⬆️ chat_cancel #1 | ⬆️ chat_cancel #1 + useChatStream (UI) |
| 15 | why cxx-build pinned | ❌ rien | ❌ rien | ❌ rien (recall fail irrécupérable) |
| 16 | **comment fonctionne la fusion RRF (FR)** | n/a | n/a | **⬆️⬆️ merge_with_rrf #1** |
| 17 | **où est gérée l'annulation chat streaming (FR)** | n/a | n/a | ⬆️ chat_cancel #5 (MiniLM English-only) |

## Scorecard

| Stage | Gros gains | Gains légers | Neutres | Régressions |
|-------|---|---|---|---|
| BM25 seul | — | — | 8/15 (ok par défaut) | 3 recall fails |
| +LLM rerank (sur 10 répondues) | 5 | 3 | 2 | 0 |
| +Hybrid BM25+semantic RRF | 3 | 5 | 5 | 4 |

## Lecture

**Le reranker LLM** reste le meilleur gain "quand il répond" — 78% des queries où Gemini n'a pas 503'd sont strictement améliorées. Sur les requêtes courantes où BM25 est déjà bon (Q1, Q9, Q10, Q11), le reranker ne dégrade pas.

**L'hybrid (embeddings RRF)** apporte un recall fix spectaculaire sur Q2 (requête où BM25 ne trouve rien par mismatch de vocabulaire) et ouvre la porte au multilingue (Q16 FR parfait). Mais il introduit 4 régressions légères (Q4, Q6, Q8, Q13) — probablement parce qu'on embed le `content` complet des fonctions, ce qui dilue le signal sémantique pour les très grosses fonctions (ex: `enrich_aspnet_mvc` fait 1000+ lignes).

**Q17 (French)** — MiniLM est English-only, il comprend "RRF" et quelques tokens latins, mais "où est gérée" lui échappe. Pour Alise_v2 / agile-up.com il faudra BGE-M3 ou Qwen3-Embedding.

**Q15** — recall fail irrécupérable : "why is cxx-build version pinned" répond à un commentaire dans `Cargo.toml`, pas indexé ni par FTS ni par embeddings. Il faudrait indexer les `.toml` comments ou faire du chunking de fichiers entiers.

## Recommandations

**Pour merge sur master maintenant :**
- Phase 1 (reranker) est strictement positive → OK merge.
- Phase 2 (embeddings) est net positif mais a 4 régressions → décision Patrice.

**Pour réduire les régressions Phase 2 :**
1. **Truncate `content` à ~500 chars** avant embedding — évite la dilution du signal sur les grosses fonctions. Re-embed.
2. **Upgrader vers BGE-M3** (multilingue fr+en, 1024d, ~500MB) — résout Q17 et probablement améliore la qualité globale.
3. **Option** : hybrid n'apparaît que derrière un flag explicite (`--hybrid`), laisser BM25 + rerank comme chemin par défaut tant qu'on n'a pas tuné.

## Latence

- BM25 seul : ~50 ms
- +Rerank : 1–10 s (selon retries Gemini 503)
- +Hybrid : +3 s pour load model ONNX + embed query + fuse (one-shot cost, amortisable avec cache de l'embedder)
- +Hybrid + Rerank : ~8–12 s

Le cold start du modèle ONNX (3s) est le gros coût — à amortir via un service persistant (MCP daemon, desktop app) plutôt qu'à chaque invocation CLI.

## Cost

- Gemini Flash rerank : ~$0.0001/query (négligeable)
- Embeddings : coût ONE-TIME à l'indexation, 43s CPU pour 5293 symboles gitnexus-rs. Re-embed uniquement à chaque `gitnexus analyze`.

## Livrables

| # | Phase | État | Commit |
|---|-------|------|--------|
| 0 | Baseline BM25 | ✅ | `960f7ac` |
| 1 | LLM reranker module | ✅ | `9f4ed70` |
| 1.5 | MCP search_code rerank | ✅ | `110e624` |
| 1.6 | Desktop Tauri search_symbols rerank | ✅ | `f90e13f` |
| 2.1 | Real ONNX inference | ✅ | `0215dfd` + `9c359e3` |
| 2.2a | Embeddings save/load format | ✅ | `234c888` |
| 2.2b | `gitnexus embed` CLI | ✅ | `513e475` |
| 2.3 | `gitnexus query --hybrid` + bench | ✅ | `ef19ff1` |
| F | Subagents chat (reporté) | 📌 noté | — |
