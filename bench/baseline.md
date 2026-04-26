# Baseline BM25 — gitnexus-rs repo

Indexed at:   "indexedAt": "2026-04-24T06:36:26Z",
Nodes: 12480
Search: FtsIndex (BM25 full-text search on name + file_path + description)

Generated: 2026-04-24T12:55:52Z

## Analyse qualitative (avant reranker)

| Q  | Query | BM25 top-1 est juste ? | Problème observé |
|----|-------|---|---|
| 1  | RRF fusion | ✅ merge_with_rrf | Tests noient les résultats (4/5 = tests) |
| 2  | reciprocal rank fusion | ❌ trouve `rank` et `todo_rank` | Query-term mismatch : "reciprocal" absent des noms |
| 3  | how is the call graph built | ⚠ build_function_call_graph #1 OK mais bruit | `is_empty` en #2, non pertinent |
| 4  | feature flag embeddings | ✅ generate_embeddings | Bon |
| 5  | cypher parser | ⚠ execute_cypher #1 mais parse_cql en #4 | Ordre sous-optimal |
| 6  | ASP.NET MVC controller action extraction | ⚠ resolve_action_node_id #1, `enrich_aspnet_mvc` #4 | La vraie fonction principale est #4 |
| 7  | tree-sitter parsing | ❌ `tree_sitter_queries` (string holders) | Rate la vraie fonction de parsing |
| 8  | LLM configuration authorization header | ❌ `build_skeleton_flowchart_returns_header_for_empty_graph` en #1 | Bruit puis enrich_with_llm |
| 9  | BM25 implementation | ✅ search_fts | Bon |
| 10 | detect dead code functions | ✅ mark_dead_code | Bon |
| 11 | ingest pipeline orchestrate phases | ✅ run_pipeline | Bon |
| 12 | C# DI resolver | ✅ extract_di_registrations | Bon |
| 13 | snapshot persistence format | ✅ save_snapshot | Bon |
| 14 | chat streaming cancellation | ✅ chat_ask + chat_cancel | Bon |
| 15 | why is cxx-build version pinned | ❌ rien de pertinent | Recall problem : info dans comment Cargo.toml non indexé |

**Scorecard BM25 pur :**
- Clair wins : 8/15 (53%)
- Partiel / bruit à nettoyer : 4/15 (27%)
- Rate total : 3/15 (20%)

**Ce qu'un reranker LLM devrait améliorer :**
- Q2, Q3, Q5, Q6, Q7, Q8 — le bon résultat est dans le top-20 BM25 mais mal classé. Precision.

**Ce qu'un reranker ne peut PAS résoudre :**
- Q15 — le bon contenu est un comment dans Cargo.toml jamais indexé. Recall problem.
- Il faudrait indexer le body/content ou utiliser des embeddings sur le code source.

**Cible Phase 1 (reranker) :** passer de 8/15 à 12/15 wins clairs, sans dégrader les wins existants.

## Q1 — "RRF fusion"

```
Found 5 results for 'RRF fusion':

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
    3. [Function  ] test_rrf_score_formula          crates/gitnexus-search/src/hybrid.rs:213-220
    4. [Function  ] test_rrf_merge_basic            crates/gitnexus-search/src/hybrid.rs:166-186
    5. [Function  ] test_rrf_empty_inputs           crates/gitnexus-search/src/hybrid.rs:189-192
```

## Q2 — "reciprocal rank fusion"

```
Found 2 results for 'reciprocal rank fusion':

    1. [Function  ] rank                            crates/gitnexus-desktop/src/commands/quality.rs:120-128
    2. [Function  ] todo_rank                       crates/gitnexus-mcp/src/backend/local.rs:2733-2741
```

## Q3 — "how is the call graph built"

```
Found 5 results for 'how is the call graph built':

    1. [Function  ] build_function_call_graph       crates/gitnexus-ingest/src/phases/process.rs:190-265
    2. [Function  ] is_empty                        crates/gitnexus-db/src/analytics/graph_diff.rs:67-73
    3. [Function  ] route_ruby_call                 crates/gitnexus-lang/src/call_routing.rs:47-93
    4. [Function  ] parse_function_call_as_call     crates/gitnexus-query/src/parser.rs:66-82
    5. [Function  ] route_call                      crates/gitnexus-lang/src/provider.rs:82-84
```

## Q4 — "feature flag embeddings"

```
Found 5 results for 'feature flag embeddings':

    1. [Function  ] generate_embeddings             crates/gitnexus-search/src/embeddings/mod.rs:60-91
    2. [Function  ] feature_dev_run                 crates/gitnexus-desktop/src/commands/feature_dev.rs:36-196
    3. [Function  ] test_generate_embeddings_fallback  crates/gitnexus-search/src/embeddings/mod.rs:260-269
    4. [Function  ] default                         crates/gitnexus-search/src/embeddings/types.rs:24-33
    5. [Function  ] new                             crates/gitnexus-search/src/embeddings/mod.rs:30-35
```

## Q5 — "cypher parser"

```
Found 5 results for 'cypher parser':

    1. [Function  ] execute_cypher                  crates/gitnexus-desktop/src/commands/cypher.rs:9-19
    2. [Function  ] from                            crates/gitnexus-query/src/parser.rs:29-31
    3. [Function  ] parse_value                     crates/gitnexus-query/src/parser.rs:472-508
    4. [Function  ] parse_cql                       crates/gitnexus-query/src/parser.rs:37-45
    5. [Function  ] parse_aggregation               crates/gitnexus-query/src/parser.rs:519-560
```

## Q6 — "ASP.NET MVC controller action extraction"

```
Found 5 results for 'ASP.NET MVC controller action extraction':

    1. [Function  ] resolve_action_node_id          crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1621-1675
    2. [Function  ] infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1592-1614
    3. [Function  ] test_infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1715-1722
    4. [Function  ] enrich_aspnet_mvc               crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:55-1237
    5. [Function  ] test_spring_controller          crates/gitnexus-lang/src/framework_detection.rs:340-346
```

## Q7 — "tree-sitter parsing"

```
Found 5 results for 'tree-sitter parsing':

    1. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/swift.rs:23-25
    2. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/go.rs:20-22
    3. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/python.rs:21-23
    4. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/c.rs:21-23
    5. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/javascript.rs:21-23
```

## Q8 — "LLM configuration and authorization header"

```
Found 5 results for 'LLM configuration and authorization header':

    1. [Function  ] build_skeleton_flowchart_returns_header_for_empty_graph  crates/gitnexus-desktop/src/commands/diagram.rs:794-801
    2. [Function  ] enrich_with_llm                 crates/gitnexus-ingest/src/phases/llm_enrichment.rs:512-647
    3. [Function  ] parse_llm_response              crates/gitnexus-ingest/src/phases/llm_enrichment.rs:425-444
    4. [Function  ] test_parse_llm_response_direct  crates/gitnexus-ingest/src/phases/llm_enrichment.rs:668-674
    5. [Function  ] parse_and_expr                  crates/gitnexus-query/src/parser.rs:301-321
```

## Q9 — "BM25 implementation"

```
Found 5 results for 'BM25 implementation':

    1. [Function  ] search_fts                      crates/gitnexus-search/src/bm25.rs:55-97
    2. [Function  ] make_bm25                       crates/gitnexus-search/src/hybrid.rs:139-150
    3. [Function  ] build_fts_query                 crates/gitnexus-search/src/bm25.rs:108-110
    4. [Function  ] parse_fts_row                   crates/gitnexus-search/src/bm25.rs:113-145
    5. [Function  ] test_rrf_only_bm25              crates/gitnexus-search/src/hybrid.rs:195-201
```

## Q10 — "detect dead code functions"

```
Found 5 results for 'detect dead code functions':

    1. [Function  ] mark_dead_code                  crates/gitnexus-ingest/src/phases/dead_code.rs:18-198
    2. [File      ] dead_code.rs                    crates/gitnexus-ingest/src/phases/dead_code.rs
    3. [Module    ] dead_code                       crates/gitnexus-ingest/src/phases/mod.rs:8-8
    4. [Function  ] test_pipeline_javascript_functions  crates/gitnexus-ingest/src/pipeline.rs:741-785
    5. [Function  ] code_review_run                 crates/gitnexus-desktop/src/commands/code_review.rs:28-189
```

## Q11 — "how does the ingest pipeline orchestrate phases"

```
Found 5 results for 'how does the ingest pipeline orchestrate phases':

    1. [Function  ] run_pipeline                    crates/gitnexus-ingest/src/pipeline.rs:44-478
    2. [Function  ] test_pipeline_javascript_functions  crates/gitnexus-ingest/src/pipeline.rs:741-785
    3. [Function  ] test_pipeline_error_recovery    crates/gitnexus-ingest/src/pipeline.rs:806-826
    4. [Function  ] test_pipeline_python_classes    crates/gitnexus-ingest/src/pipeline.rs:862-905
    5. [Function  ] test_pipeline_empty_project     crates/gitnexus-ingest/src/pipeline.rs:788-803
```

## Q12 — "where is the C# DI resolver"

```
Found 5 results for 'where is the C# DI resolver':

    1. [Function  ] extract_di_registrations        crates/gitnexus-lang/src/route_extractors/csharp/di.rs:36-140
    2. [Function  ] test_extract_di_autofac         crates/gitnexus-lang/src/route_extractors/csharp/di.rs:147-166
    3. [Function  ] main                            target-codex/debug/build/tree-sitter-c-sharp-40e041a8cda36db4/out/flag_check.c:1-1
    4. [File      ] di.rs                           crates/gitnexus-lang/src/route_extractors/csharp/di.rs
    5. [Function  ] eval_where                      crates/gitnexus-db/src/inmemory/cypher.rs:1037-1144
```

## Q13 — "snapshot persistence format"

```
Found 5 results for 'snapshot persistence format':

    1. [Function  ] save_snapshot                   crates/gitnexus-db/src/snapshot.rs:28-110
    2. [Function  ] snapshot_err                    crates/gitnexus-db/src/snapshot.rs:18-23
    3. [Function  ] load_snapshot                   crates/gitnexus-db/src/snapshot.rs:113-120
    4. [Function  ] snapshot_path                   crates/gitnexus-db/src/snapshot.rs:128-130
    5. [Function  ] snapshot_exists                 crates/gitnexus-db/src/snapshot.rs:123-125
```

## Q14 — "chat streaming cancellation"

```
Found 5 results for 'chat streaming cancellation':

    1. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
    2. [Function  ] chat_cancel                     crates/gitnexus-desktop/src/commands/chat.rs:3149-3154
    3. [Function  ] chat_test_connection            crates/gitnexus-desktop/src/commands/chat.rs:1156-1231
    4. [Function  ] chat_set_config                 crates/gitnexus-desktop/src/commands/chat.rs:1135-1138
    5. [Function  ] chat_retry_tool                 crates/gitnexus-desktop/src/commands/chat.rs:1447-1504
```

## Q15 — "why is cxx-build version pinned"

```
Found 5 results for 'why is cxx-build version pinned':

    1. [Function  ] test_read_version_resource      crates/gitnexus-mcp/src/resources.rs:292-300
    2. [Function  ] extract_package_version         crates/gitnexus-lang/src/component_detection.rs:454-477
    3. [Function  ] test_extract_package_version    crates/gitnexus-lang/src/component_detection.rs:862-868
    4. [Function  ] is_eof                          target-codex/debug/build/cssparser-9957a2a9eb2a811c/out/tokenizer.rs:64-66
    5. [Function  ] is_ident_start                  target-codex/debug/build/cssparser-9957a2a9eb2a811c/out/tokenizer.rs:539-581
```

