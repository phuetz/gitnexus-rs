# Hybrid (BM25 + semantic RRF) — gitnexus-rs repo

Indexed at:   "indexedAt": "2026-04-24T06:36:26Z",
Embeddings: all-MiniLM-L6-v2 (384d, 5293 symbols, 8.2MB)

Generated: 2026-04-24T14:01:11Z

## Q1 — "RRF fusion"

```
Found 5 results for 'RRF fusion':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
    3. [Function  ] test_rrf_score_formula          crates/gitnexus-search/src/hybrid.rs:213-220
    4. [Function  ] test_rrf_only_bm25              crates/gitnexus-search/src/hybrid.rs:195-201
    5. [Function  ] test_rrf_empty_inputs           crates/gitnexus-search/src/hybrid.rs:189-192
```

## Q2 — "reciprocal rank fusion"

```
Found 5 results for 'reciprocal rank fusion':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] rank                            crates/gitnexus-desktop/src/commands/quality.rs:120-128
    2. [Function  ] todo_rank                       crates/gitnexus-mcp/src/backend/local.rs:2733-2741
    3. [Function  ] rankResults                     crates/gitnexus-desktop/ui/src/components/search/SearchView.tsx:12-27
    4. [Function  ] rankResults                     crates/gitnexus-desktop/ui/src/components/search/SearchModal.tsx:35-47
    5. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
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

    1. [Function  ] generate_embeddings             crates/gitnexus-search/src/embeddings/mod.rs:60-91
    2. [Function  ] test_generate_embeddings_fallback  crates/gitnexus-search/src/embeddings/mod.rs:260-269
    3. [Function  ] embed                           crates/gitnexus-search/src/embeddings/mod.rs:37-49
    4. [Function  ] search_semantic                 crates/gitnexus-search/src/embeddings/mod.rs:94-128
    5. [Function  ] default                         crates/gitnexus-search/src/embeddings/types.rs:24-33
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

    1. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/c.rs:21-23
    2. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/cpp.rs:22-24
    3. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/java.rs:26-28
    4. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/python.rs:21-23
    5. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/javascript.rs:21-23
```

## Q8 — "LLM configuration and authorization header"

```
Found 5 results for 'LLM configuration and authorization header':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] test_parse_llm_response_markdown_wrapped  crates/gitnexus-ingest/src/phases/llm_enrichment.rs:677-681
    2. [Enum      ] LlmResponseChunk                crates/gitnexus-core/src/llm/mod.rs:31-34
    3. [Function  ] build_skeleton_flowchart_returns_header_for_empty_graph  crates/gitnexus-desktop/src/commands/diagram.rs:794-801
    4. [Function  ] enrich_with_llm                 crates/gitnexus-ingest/src/phases/llm_enrichment.rs:512-647
    5. [Function  ] auth_middleware                 crates/gitnexus-mcp/src/transport/http.rs:44-72
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
    2. [Function  ] merge                           crates/gitnexus-ingest/src/phases/parsing.rs:28-33
    3. [Struct    ] ProcessTrace                    crates/gitnexus-ingest/src/phases/process.rs:335-337
    4. [Function  ] test_pipeline_multiple_languages  crates/gitnexus-ingest/src/pipeline.rs:829-859
    5. [Struct    ] PipelineOptions                 crates/gitnexus-ingest/src/pipeline.rs:32-41
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

    1. [Function  ] snapshot_err                    crates/gitnexus-db/src/snapshot.rs:18-23
    2. [Function  ] save_snapshot                   crates/gitnexus-db/src/snapshot.rs:28-110
    3. [Function  ] snapshot_exists                 crates/gitnexus-db/src/snapshot.rs:123-125
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

    1. [Function  ] build_name_index                crates/gitnexus-core/src/trace.rs:145-154
    2. [Function  ] test_read_version_resource      crates/gitnexus-mcp/src/resources.rs:292-300
    3. [Function  ] extract_package_version         crates/gitnexus-lang/src/component_detection.rs:454-477
    4. [Function  ] get                             target-codex/debug/build/markup5ever-dcae884b9dd8cf7c/out/generated.rs:2904-2911
    5. [Function  ] test_extract_package_version    crates/gitnexus-lang/src/component_detection.rs:862-868
```

## Q16 — "comment fonctionne la fusion RRF"

```
Found 5 results for 'comment fonctionne la fusion RRF':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
    3. [Function  ] test_rrf_score_formula          crates/gitnexus-search/src/hybrid.rs:213-220
    4. [Function  ] test_rrf_empty_inputs           crates/gitnexus-search/src/hybrid.rs:189-192
    5. [Function  ] test_rrf_merge_basic            crates/gitnexus-search/src/hybrid.rs:166-186
```

## Q17 — "où est gérée l'annulation du chat streaming"

```
Found 5 results for 'où est gérée l'annulation du chat streaming':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] chat_retry_tool                 crates/gitnexus-desktop/src/commands/chat.rs:1447-1504
    2. [Function  ] top_processes                   crates/gitnexus-desktop/src/commands/chat.rs:2749-2764
    3. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
    4. [Function  ] useChatStream                   crates/gitnexus-desktop/ui/src/hooks/use-chat-stream.ts:12-150
    5. [Function  ] chat_cancel                     crates/gitnexus-desktop/src/commands/chat.rs:3149-3154
```

