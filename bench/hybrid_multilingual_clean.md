# Hybrid (BM25 + semantic RRF) — gitnexus-rs repo

Indexed at:   "indexedAt": "2026-04-24T06:36:26Z",
Embeddings: all-MiniLM-L6-v2 (384d, 5293 symbols, 8.2MB)

Generated: 2026-04-24T14:54:26Z

## Q1 — "RRF fusion"

```
Found 5 results for 'RRF fusion':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_score_formula          crates/gitnexus-search/src/hybrid.rs:213-220
    3. [Function  ] test_rrf_merge_basic            crates/gitnexus-search/src/hybrid.rs:166-186
    4. [Function  ] test_rrf_only_bm25              crates/gitnexus-search/src/hybrid.rs:195-201
    5. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
```

## Q2 — "reciprocal rank fusion"

```
Found 5 results for 'reciprocal rank fusion':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] todo_rank                       crates/gitnexus-mcp/src/backend/local.rs:2733-2741
    2. [Function  ] full_props_equal                crates/gitnexus-db/src/analytics/graph_diff.rs:173-178
    3. [Function  ] rank                            crates/gitnexus-desktop/src/commands/quality.rs:120-128
    4. [Function  ] values_equal                    crates/gitnexus-query/src/executor.rs:837-853
    5. [Function  ] ranges_overlap                  crates/gitnexus-mcp/src/backend/local.rs:2842-2844
```

## Q3 — "how is the call graph built"

```
Found 5 results for 'how is the call graph built':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] build_function_call_graph       crates/gitnexus-ingest/src/phases/process.rs:190-265
    2. [Function  ] is_empty                        crates/gitnexus-db/src/analytics/graph_diff.rs:67-73
    3. [Interface ] GraphEffectsOptions             crates/gitnexus-desktop/ui/src/components/graph/useGraphEffects.ts:10-29
    4. [Function  ] make_graph                      crates/gitnexus-core/src/trace.rs:227-263
    5. [Function  ] route_ruby_call                 crates/gitnexus-lang/src/call_routing.rs:47-93
```

## Q4 — "feature flag embeddings"

```
Found 5 results for 'feature flag embeddings':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] generate_embeddings             crates/gitnexus-search/src/embeddings/mod.rs:60-91
    2. [Struct    ] FeatureInfo                     crates/gitnexus-desktop/src/types.rs:550-558
    3. [Function  ] feature_dev_run                 crates/gitnexus-desktop/src/commands/feature_dev.rs:36-196
    4. [Struct    ] FeatureDevSectionEvent          crates/gitnexus-desktop/src/types.rs:722-725
    5. [Function  ] test_generate_embeddings_fallback  crates/gitnexus-search/src/embeddings/mod.rs:260-269
```

## Q5 — "cypher parser"

```
Found 5 results for 'cypher parser':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] parse_atom                      crates/gitnexus-query/src/parser.rs:418-470
    2. [Function  ] parse_expression                crates/gitnexus-query/src/parser.rs:271-277
    3. [Function  ] parse_comparison                crates/gitnexus-query/src/parser.rs:343-361
    4. [Function  ] parse_string_literal            crates/gitnexus-query/src/parser.rs:510-517
    5. [Function  ] execute_cypher                  crates/gitnexus-desktop/src/commands/cypher.rs:9-19
```

## Q6 — "ASP.NET MVC controller action extraction"

```
Found 5 results for 'ASP.NET MVC controller action extraction':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] extract_action_filters          crates/gitnexus-lang/src/route_extractors/csharp/controllers.rs:265-293
    2. [Function  ] infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1592-1614
    3. [Function  ] test_extract_html_helpers_action_link  crates/gitnexus-lang/src/component_detection.rs:1012-1028
    4. [Function  ] test_infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1715-1722
    5. [Function  ] parse_action_method             crates/gitnexus-lang/src/route_extractors/csharp/controllers.rs:124-258
```

## Q7 — "tree-sitter parsing"

```
Found 5 results for 'tree-sitter parsing':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/csharp.rs:26-28
    2. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/cpp.rs:22-24
    3. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/java.rs:26-28
    4. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/razor.rs:36-38
    5. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/go.rs:20-22
```

## Q8 — "LLM configuration and authorization header"

```
Found 5 results for 'LLM configuration and authorization header':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] cmd_ownership                   crates/gitnexus-cli/src/commands/shell.rs:1802-1836
    2. [Function  ] build_skeleton_flowchart_returns_header_for_empty_graph  crates/gitnexus-desktop/src/commands/diagram.rs:794-801
    3. [Function  ] enrich_with_llm                 crates/gitnexus-ingest/src/phases/llm_enrichment.rs:512-647
    4. [Module    ] ownership_cmd                   crates/gitnexus-cli/src/commands/mod.rs:19-19
    5. [Function  ] heading                         crates/gitnexus-desktop/src/commands/export.rs:535-544
```

## Q9 — "BM25 implementation"

```
Found 5 results for 'BM25 implementation':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] make_bm25                       crates/gitnexus-search/src/hybrid.rs:139-150
    2. [Function  ] test_bm25_scoring               crates/gitnexus-db/src/inmemory/fts.rs:446-454
    3. [Struct    ] BM25SearchResult                crates/gitnexus-search/src/bm25.rs:40-49
    4. [Module    ] bm25                            crates/gitnexus-search/src/lib.rs:1-1
    5. [Module    ] tests                           crates/gitnexus-search/src/bm25.rs:179-262
```

## Q10 — "detect dead code functions"

```
Found 5 results for 'detect dead code functions':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] mark_dead_code                  crates/gitnexus-ingest/src/phases/dead_code.rs:18-198
    2. [Module    ] dead_code                       crates/gitnexus-ingest/src/phases/mod.rs:8-8
    3. [Function  ] detect_processes                crates/gitnexus-ingest/src/phases/process.rs:43-158
    4. [Function  ] test_extract_csharp_blocks_functions_directive  crates/gitnexus-lang/src/component_detection.rs:914-927
    5. [Function  ] detect_cycles                   crates/gitnexus-desktop/src/commands/quality.rs:35-44
```

## Q11 — "how does the ingest pipeline orchestrate phases"

```
Found 5 results for 'how does the ingest pipeline orchestrate phases':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Enum      ] PipelinePhase                   crates/gitnexus-core/src/pipeline/types.rs:7-22
    2. [Function  ] run_pipeline                    crates/gitnexus-ingest/src/pipeline.rs:44-478
    3. [Function  ] test_pipeline_csharp_controller  crates/gitnexus-ingest/src/pipeline.rs:692-738
    4. [Struct    ] PipelineResult                  crates/gitnexus-ingest/src/pipeline.rs:22-28
    5. [Function  ] test_pipeline_javascript_functions  crates/gitnexus-ingest/src/pipeline.rs:741-785
```

## Q12 — "where is the C# DI resolver"

```
Found 5 results for 'where is the C# DI resolver':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] resolve_calls                   crates/gitnexus-ingest/src/phases/calls.rs:120-308
    2. [Function  ] extract_di_registrations        crates/gitnexus-lang/src/route_extractors/csharp/di.rs:36-140
    3. [Function  ] diff_manifests                  crates/gitnexus-ingest/src/manifest.rs:129-163
    4. [Function  ] test_extract_di_autofac         crates/gitnexus-lang/src/route_extractors/csharp/di.rs:147-166
    5. [Function  ] resolve_scope                   crates/gitnexus-desktop/src/commands/simplify.rs:88-160
```

## Q13 — "snapshot persistence format"

```
Found 5 results for 'snapshot persistence format':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] snapshot_exists                 crates/gitnexus-db/src/snapshot.rs:123-125
    2. [Function  ] snapshot_err                    crates/gitnexus-db/src/snapshot.rs:18-23
    3. [Function  ] save_snapshot                   crates/gitnexus-db/src/snapshot.rs:28-110
    4. [Function  ] snapshot_path                   crates/gitnexus-db/src/snapshot.rs:128-130
    5. [Function  ] snapshot_delete                 crates/gitnexus-desktop/src/commands/snapshots.rs:328-339
```

## Q14 — "chat streaming cancellation"

```
Found 5 results for 'chat streaming cancellation':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] chat_test_connection            crates/gitnexus-desktop/src/commands/chat.rs:1156-1231
    2. [Function  ] chat_execute_plan               crates/gitnexus-desktop/src/commands/chat_executor.rs:720-913
    3. [Function  ] chat_execute_step               crates/gitnexus-desktop/src/commands/chat_executor.rs:79-187
    4. [Function  ] chat_config                     crates/gitnexus-desktop/src/state.rs:225-227
    5. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
```

## Q15 — "why is cxx-build version pinned"

```
Found 5 results for 'why is cxx-build version pinned':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] test_read_version_resource      crates/gitnexus-mcp/src/resources.rs:292-300
    2. [Interface ] ModifiedNodeDiff                crates/gitnexus-desktop/ui/src/lib/tauri-commands.ts:847-850
    3. [Function  ] extract_package_version         crates/gitnexus-lang/src/component_detection.rs:454-477
    4. [Interface ] ModifiedNode                    crates/gitnexus-desktop/ui/src/lib/tauri-commands.ts:688-694
    5. [Function  ] is_binary_content               crates/gitnexus-db/src/csv_generator.rs:31-43
```

## Q16 — "comment fonctionne la fusion RRF"

```
Found 5 results for 'comment fonctionne la fusion RRF':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Interface ] Comment                         crates/gitnexus-desktop/ui/src/lib/tauri-commands.ts:515-521
    3. [Module    ] coupling                        crates/gitnexus-git/src/lib.rs:1-1
    4. [Function  ] merge                           crates/gitnexus-ingest/src/phases/parsing.rs:28-33
    5. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
```

## Q17 — "où est gérée l'annulation du chat streaming"

```
Found 5 results for 'où est gérée l'annulation du chat streaming':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
    2. [Function  ] useChatStream                   crates/gitnexus-desktop/ui/src/hooks/use-chat-stream.ts:12-150
    3. [Function  ] chat_cancel                     crates/gitnexus-desktop/src/commands/chat.rs:3149-3154
    4. [Method    ] isInteractive                   crates/gitnexus-desktop/ui/src/components/chat/ChatMarkdown.tsx:84-86
    5. [Function  ] chat_get_config                 crates/gitnexus-desktop/src/commands/chat.rs:1129-1131
```

