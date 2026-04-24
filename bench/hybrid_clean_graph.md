# Hybrid (BM25 + semantic RRF) — gitnexus-rs repo

Indexed at:   "indexedAt": "2026-04-24T15:26:50Z",
Embeddings: all-MiniLM-L6-v2 (384d, 5293 symbols, 8.2MB)

Generated: 2026-04-24T15:27:40Z

## Q1 — "RRF fusion"

```
Found 5 results for 'RRF fusion':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_score_formula          crates/gitnexus-search/src/hybrid.rs:213-220
    3. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
    4. [Function  ] test_rrf_merge_basic            crates/gitnexus-search/src/hybrid.rs:166-186
    5. [Function  ] test_rrf_empty_inputs           crates/gitnexus-search/src/hybrid.rs:189-192
```

## Q2 — "reciprocal rank fusion"

```
Found 5 results for 'reciprocal rank fusion':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] rank                            crates/gitnexus-desktop/src/commands/quality.rs:120-128
    2. [Function  ] rerank                          crates/gitnexus-search/src/reranker/llm.rs:145-242
    3. [Function  ] todo_rank                       crates/gitnexus-mcp/src/backend/local.rs:2936-2944
    4. [Function  ] with_max_candidates             crates/gitnexus-search/src/reranker/llm.rs:48-51
    5. [Trait     ] Reranker                        crates/gitnexus-search/src/reranker/mod.rs:31-33
```

## Q3 — "how is the call graph built"

```
Found 5 results for 'how is the call graph built':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] build_function_call_graph       crates/gitnexus-ingest/src/phases/process.rs:190-265
    2. [Struct    ] GraphRelationship               crates/gitnexus-core/src/graph/types.rs:727-741
    3. [Function  ] is_empty                        crates/gitnexus-db/src/analytics/graph_diff.rs:67-73
    4. [Interface ] ToolbarProps                    crates/gitnexus-desktop/ui/src/components/graph/GraphEmptyStates.tsx:7-17
    5. [Function  ] buildGraphologyGraph            crates/gitnexus-desktop/ui/src/lib/graph-adapter.ts:138-221
```

## Q4 — "feature flag embeddings"

```
Found 5 results for 'feature flag embeddings':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] generate_embeddings             crates/gitnexus-search/src/embeddings/mod.rs:236-294
    2. [Function  ] save_embeddings                 crates/gitnexus-search/src/embeddings/store.rs:39-93
    3. [Function  ] load_embeddings                 crates/gitnexus-search/src/embeddings/store.rs:95-147
    4. [Function  ] test_generate_embeddings_fallback_no_model  crates/gitnexus-search/src/embeddings/mod.rs:463-471
    5. [Function  ] embed                           crates/gitnexus-search/src/embeddings/mod.rs:78-215
```

## Q5 — "cypher parser"

```
Found 5 results for 'cypher parser':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] parse_cql                       crates/gitnexus-query/src/parser.rs:37-45
    2. [Function  ] parse                           crates/gitnexus-db/src/inmemory/cypher.rs:305-316
    3. [Function  ] execute_cypher                  crates/gitnexus-desktop/src/commands/cypher.rs:9-19
    4. [Function  ] parse_patterns                  crates/gitnexus-db/src/inmemory/cypher.rs:435-463
    5. [Function  ] from                            crates/gitnexus-query/src/parser.rs:29-31
```

## Q6 — "ASP.NET MVC controller action extraction"

```
Found 5 results for 'ASP.NET MVC controller action extraction':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1592-1614
    2. [Function  ] extract_action_filters          crates/gitnexus-lang/src/route_extractors/csharp/controllers.rs:265-293
    3. [Function  ] resolve_action_node_id          crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1621-1675
    4. [Function  ] test_infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1715-1722
    5. [Function  ] parse_action_method             crates/gitnexus-lang/src/route_extractors/csharp/controllers.rs:124-258
```

## Q7 — "tree-sitter parsing"

```
Found 5 results for 'tree-sitter parsing':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/java.rs:26-28
    2. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/c.rs:21-23
    3. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/php.rs:22-24
    4. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/javascript.rs:21-23
    5. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/go.rs:20-22
```

## Q8 — "LLM configuration and authorization header"

```
Found 5 results for 'LLM configuration and authorization header':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] load_llm_config                 crates/gitnexus-mcp/src/llm_config.rs:20-41
    2. [Function  ] build_skeleton_flowchart_returns_header_for_empty_graph  crates/gitnexus-desktop/src/commands/diagram.rs:794-801
    3. [Struct    ] LlmConfig                       crates/gitnexus-mcp/src/llm_config.rs:11-17
    4. [Enum      ] LlmResponseChunk                crates/gitnexus-core/src/llm/mod.rs:31-34
    5. [Function  ] parse_and_expr                  crates/gitnexus-query/src/parser.rs:301-321
```

## Q9 — "BM25 implementation"

```
Found 5 results for 'BM25 implementation':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] make_bm25                       crates/gitnexus-search/src/hybrid.rs:139-150
    2. [Function  ] search_fts                      crates/gitnexus-search/src/bm25.rs:55-97
    3. [Function  ] test_rrf_only_bm25              crates/gitnexus-search/src/hybrid.rs:195-201
    4. [Struct    ] BM25SearchResult                crates/gitnexus-search/src/bm25.rs:40-49
    5. [Function  ] test_bm25_scoring               crates/gitnexus-db/src/inmemory/fts.rs:446-454
```

## Q10 — "detect dead code functions"

```
Found 5 results for 'detect dead code functions':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] mark_dead_code                  crates/gitnexus-ingest/src/phases/dead_code.rs:18-198
    2. [Function  ] test_extract_csharp_blocks_functions_directive  crates/gitnexus-lang/src/component_detection.rs:914-927
    3. [Module    ] dead_code                       crates/gitnexus-ingest/src/phases/mod.rs:8-8
    4. [Function  ] detect_cycles                   crates/gitnexus-desktop/src/commands/quality.rs:35-44
    5. [Function  ] detect_layer_violations         crates/gitnexus-ingest/src/phases/architecture.rs:178-231
```

## Q11 — "how does the ingest pipeline orchestrate phases"

```
Found 5 results for 'how does the ingest pipeline orchestrate phases':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] run_pipeline                    crates/gitnexus-ingest/src/pipeline.rs:44-478
    2. [Struct    ] ProcessTrace                    crates/gitnexus-ingest/src/phases/process.rs:335-337
    3. [Function  ] test_pipeline_multiple_languages  crates/gitnexus-ingest/src/pipeline.rs:829-859
    4. [Struct    ] PipelineOptions                 crates/gitnexus-ingest/src/pipeline.rs:32-41
    5. [Function  ] classify_process                crates/gitnexus-ingest/src/phases/process.rs:478-497
```

## Q12 — "where is the C# DI resolver"

```
Found 5 results for 'where is the C# DI resolver':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] extract_di_registrations        crates/gitnexus-lang/src/route_extractors/csharp/di.rs:36-140
    2. [Struct    ] ResolveCtx                      crates/gitnexus-lang/src/import_resolvers/types.rs:19-30
    3. [Function  ] resolve                         crates/gitnexus-lang/src/import_resolvers/razor.rs:18-81
    4. [Function  ] test_extract_di_autofac         crates/gitnexus-lang/src/route_extractors/csharp/di.rs:147-166
    5. [Function  ] resolve_by_suffix_insensitive   crates/gitnexus-lang/src/import_resolvers/utils.rs:24-39
```

## Q13 — "snapshot persistence format"

```
Found 5 results for 'snapshot persistence format':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] save_snapshot                   crates/gitnexus-db/src/snapshot.rs:28-110
    2. [Function  ] snapshot_exists                 crates/gitnexus-db/src/snapshot.rs:123-125
    3. [Function  ] snapshot_err                    crates/gitnexus-db/src/snapshot.rs:18-23
    4. [Function  ] snapshot_path                   crates/gitnexus-db/src/snapshot.rs:128-130
    5. [Function  ] write_snapshot_metadata         crates/gitnexus-desktop/src/commands/snapshots.rs:77-84
```

## Q14 — "chat streaming cancellation"

```
Found 5 results for 'chat streaming cancellation':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] chat_cancel                     crates/gitnexus-desktop/src/commands/chat.rs:3149-3154
    2. [Function  ] chat_retry_tool                 crates/gitnexus-desktop/src/commands/chat.rs:1447-1504
    3. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
    4. [Function  ] useChatStream                   crates/gitnexus-desktop/ui/src/hooks/use-chat-stream.ts:12-150
    5. [Function  ] chat_set_config                 crates/gitnexus-desktop/src/commands/chat.rs:1135-1138
```

## Q15 — "why is cxx-build version pinned"

```
Found 5 results for 'why is cxx-build version pinned':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] is_build_artifact               crates/gitnexus-cli/src/commands/embed.rs:225-238
    2. [Function  ] build_name_index                crates/gitnexus-core/src/trace.rs:145-154
    3. [Function  ] test_build_name_index           crates/gitnexus-core/src/trace.rs:307-312
    4. [Function  ] extract_package_version         crates/gitnexus-lang/src/component_detection.rs:454-477
    5. [Function  ] build_digest                    crates/gitnexus-ingest/src/manifest.rs:105-120
```

## Q16 — "comment fonctionne la fusion RRF"

```
Found 5 results for 'comment fonctionne la fusion RRF':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
    3. [Function  ] test_rrf_score_formula          crates/gitnexus-search/src/hybrid.rs:213-220
    4. [Function  ] test_rrf_merge_basic            crates/gitnexus-search/src/hybrid.rs:166-186
    5. [Function  ] test_rrf_empty_inputs           crates/gitnexus-search/src/hybrid.rs:189-192
```

## Q17 — "où est gérée l'annulation du chat streaming"

```
Found 5 results for 'où est gérée l'annulation du chat streaming':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] chat_retry_tool                 crates/gitnexus-desktop/src/commands/chat.rs:1447-1504
    2. [Function  ] chat_cancel                     crates/gitnexus-desktop/src/commands/chat.rs:3149-3154
    3. [Function  ] useChatStream                   crates/gitnexus-desktop/ui/src/hooks/use-chat-stream.ts:12-150
    4. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
    5. [Function  ] french_architecture_questions_route_to_complex  crates/gitnexus-desktop/src/commands/chat_planner.rs:783-797
```

