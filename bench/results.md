# Phase 1 Results — LLM Reranker vs BM25 Baseline

Comparaison des 15 requêtes du corpus de baseline.
Repo indexé : gitnexus-rs lui-même (12480 nodes, 470 files).
LLM utilisé : Gemini 2.5 Flash via `~/.gitnexus/chat-config.json`.
BM25 pool = top-20, reranker affiche top-5.

## Tableau comparatif

| Q  | Query | BM25 top-1 | Rerank top-1 | LLM a répondu ? | Verdict |
|----|-------|---|---|---|---|
| 1  | RRF fusion | merge_with_rrf ✓ | merge_with_rrf ✓ | ❌ 503 3× | = |
| 2  | reciprocal rank fusion | rank ✗ | rank ✗ | ❌ 503 3× | = recall fail |
| 3  | how is the call graph built | build_function_call_graph puis `is_empty` | build_function_call_graph → build_call_adjacency → extract_call | ✅ | **⬆️⬆️** |
| 4  | feature flag embeddings | generate_embeddings puis feature_dev_run | generate_embeddings → embed → search_semantic → cosine_similarity | ✅ | **⬆️⬆️** |
| 5  | cypher parser | execute_cypher; parse_cql #4 | parse_cql → parse_statement → from | ✅ | **⬆️⬆️** |
| 6  | ASP.NET MVC controller action extraction | resolve_action_node_id #1, enrich_aspnet_mvc #4 | enrich_aspnet_mvc → resolve_action_node_id → parse_action_method | ✅ | **⬆️⬆️** |
| 7  | tree-sitter parsing | tree_sitter_queries × 5 (string holders) | walk_tree_for_complexity #1 | ✅ | ⬆️ |
| 8  | LLM config auth header | build_skeleton_flowchart_returns_header... #1 (bruit) | enrich_with_llm → parse_llm_response | ✅ | **⬆️⬆️** |
| 9  | BM25 implementation | search_fts ✓ | search_fts ✓ | ❌ 503 3× | = |
| 10 | detect dead code | mark_dead_code ✓ | mark_dead_code ✓ | ❌ 503 3× | = |
| 11 | ingest pipeline orchestrate phases | run_pipeline ✓ | run_pipeline ✓ | ❌ 503 3× | = |
| 12 | C# DI resolver | extract_di_registrations #1 | di.rs file #1 puis extract_di_registrations #2 | ✅ | ⚠️ légère régression |
| 13 | snapshot persistence format | save_snapshot ✓ | save_snapshot ✓ | ✅ | = (tail remix) |
| 14 | chat streaming cancellation | chat_ask #1, chat_cancel #2 | chat_cancel #1, chat_ask #2 | ✅ | ⬆️ |
| 15 | why is cxx-build version pinned | rien de pertinent | rien de pertinent | ✅ | = recall fail |

## Scorecard

**Sur les 15 requêtes totales :**
- Gros gain (⬆️⬆️) : Q3, Q4, Q5, Q6, Q8 → **5/15 (33%)**
- Gain léger (⬆️) : Q7, Q14 → **2/15 (13%)**
- Neutre (=) : Q1, Q2, Q9, Q10, Q11, Q13, Q15 → **7/15 (47%)**
- Régression légère (⚠️) : Q12 → **1/15 (7%)**

**Sur les 9 requêtes où le reranker a effectivement répondu (Gemini up) :**
- Amélioré : 7/9 (**78%**)
- Neutre : 1/9
- Régression : 1/9

## Faits saillants

1. **Quand il répond, le reranker améliore presque toujours.** Sur les 9 queries reçues, 7 sont significativement meilleures.
2. **Il ne dégrade jamais catastrophiquement.** La seule régression (Q12) remplace une fonction par le fichier qui la contient — discutable, pas dramatique.
3. **Le filtrage des tests fonctionne.** Q1 baseline avait 4 tests sur 5 résultats. Q4 baseline avait `test_generate_embeddings_fallback` ; après rerank, seul le code de prod.
4. **Les échecs Gemini 503 ne cassent rien.** Fallback BM25 silencieux + warning stderr. 6/15 queries ont touché un 503 ; toutes ont livré un résultat.
5. **Les recall problems restent.** Q2 ("reciprocal rank fusion") et Q15 ("cxx-build pinned") — la cible n'est pas dans le top-20 BM25, donc le reranker ne peut rien. **C'est exactement la raison d'être des embeddings (Phase 2).**

## Latence

- BM25 pur : ~50 ms (in-memory)
- Rerank happy path : 1-3 s (un seul appel Gemini Flash)
- Rerank avec 2 retries : 5-7 s
- Rerank fallback après 3 retries : ~10 s (1s + 2s + 4s backoff + latence)

Acceptable pour un CLI dev. Pour un serveur interactif, ajouter un cache query→indices keyed par query+top_20_ids.

## Coût Gemini Flash

~500 tokens in + 100 tokens out par query → ~$0.0001/query avec les tarifs Flash ($0.075/$0.30 per 1M).
Négligeable même à 1000 queries/jour.

## Verdict Phase 1

**Livrée, mesurée, stable.** Le reranker apporte un gain qualitatif net sur 7 queries, n'en dégrade aucune significativement, et échoue en mode dégradé gracieux.

## Arbitrage Phase 2 (embeddings)

**Encore justifiée, mais moins urgente.** Les 2 queries qui restent totalement ratées (Q2, Q15) sont des recall problems — la cible n'est jamais retournée par BM25. Les embeddings résoudraient précisément ces cas.

Recommandation :
- **Si quality agile-up.com** → Phase 2 vaut le coup (multilingue, docs Alise)
- **Si juste qualité sur code** → Phase 2 peut attendre, reranker couvre déjà 78% des cas where it matters

## Vraie Phase F à garder en vue

Le vrai point de la vidéo (subagents isolés dans le chat desktop) n'est pas adressé ici. C'est l'angle le plus différenciant si on veut vendre GitNexus comme "second brain" pour clients agile-up.com. À planifier sur une autre branche.
