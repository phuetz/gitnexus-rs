# Rerank (LLM) — gitnexus-rs repo

Indexed at:   "indexedAt": "2026-04-24T06:36:26Z",
Nodes: 12480
Search: BM25 top-20 pool -> LlmReranker (Gemini 2.5 Flash) -> top-5

Generated: 2026-04-24T13:08:49Z

## Q1 — "RRF fusion"

```
[2m2026-04-24T13:08:51.280581Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:08:54.066191Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
[2m2026-04-24T13:09:00.074057Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m3 [3mbackoff_ms[0m[2m=[0m4000
Warning: reranker failed, falling back to BM25 order: reranker HTTP 503 Service Unavailable: [{
  "error": {
    "code": 503,
    "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
    "status": "UNAVAILABLE"
  }
}
]
Found 5 results for 'RRF fusion':
  (reranked by LLM from top-6 BM25 pool)

    1. [Function  ] merge_with_rrf                  crates/gitnexus-search/src/hybrid.rs:50-133
    2. [Function  ] test_rrf_limit                  crates/gitnexus-search/src/hybrid.rs:204-210
    3. [Function  ] test_rrf_empty_inputs           crates/gitnexus-search/src/hybrid.rs:189-192
    4. [Function  ] test_rrf_merge_basic            crates/gitnexus-search/src/hybrid.rs:166-186
    5. [Function  ] test_rrf_only_bm25              crates/gitnexus-search/src/hybrid.rs:195-201
```

## Q2 — "reciprocal rank fusion"

```
[2m2026-04-24T13:09:06.644224Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:09:10.435274Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
[2m2026-04-24T13:09:13.793697Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m3 [3mbackoff_ms[0m[2m=[0m4000
Warning: reranker failed, falling back to BM25 order: reranker HTTP 503 Service Unavailable: [{
  "error": {
    "code": 503,
    "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
    "status": "UNAVAILABLE"
  }
}
]
Found 2 results for 'reciprocal rank fusion':
  (reranked by LLM from top-2 BM25 pool)

    1. [Function  ] rank                            crates/gitnexus-desktop/src/commands/quality.rs:120-128
    2. [Function  ] todo_rank                       crates/gitnexus-mcp/src/backend/local.rs:2733-2741
```

## Q3 — "how is the call graph built"

```
[2m2026-04-24T13:09:20.551693Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:09:22.625518Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
Found 5 results for 'how is the call graph built':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] build_function_call_graph       crates/gitnexus-ingest/src/phases/process.rs:190-265
    2. [Function  ] build_call_adjacency            crates/gitnexus-ingest/src/phases/process.rs:161-181
    3. [Function  ] extract_call                    crates/gitnexus-ingest/src/phases/parsing.rs:1369-1470
    4. [Function  ] extract_new_call                crates/gitnexus-ingest/src/phases/parsing.rs:1473-1507
    5. [Function  ] is_empty                        crates/gitnexus-db/src/analytics/graph_diff.rs:67-73
```

## Q4 — "feature flag embeddings"

```
Found 5 results for 'feature flag embeddings':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] generate_embeddings             crates/gitnexus-search/src/embeddings/mod.rs:60-91
    2. [Function  ] embed                           crates/gitnexus-search/src/embeddings/mod.rs:37-49
    3. [Function  ] search_semantic                 crates/gitnexus-search/src/embeddings/mod.rs:94-128
    4. [Function  ] cosine_similarity               crates/gitnexus-search/src/embeddings/mod.rs:130-153
    5. [Function  ] new                             crates/gitnexus-search/src/embeddings/mod.rs:30-35
```

## Q5 — "cypher parser"

```
[2m2026-04-24T13:09:33.667991Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:09:35.644867Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
Found 5 results for 'cypher parser':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] parse_cql                       crates/gitnexus-query/src/parser.rs:37-45
    2. [Function  ] parse_statement                 crates/gitnexus-query/src/parser.rs:49-56
    3. [Function  ] from                            crates/gitnexus-query/src/parser.rs:29-31
    4. [Function  ] parse_match_statement           crates/gitnexus-query/src/parser.rs:84-144
    5. [Function  ] parse_call_statement            crates/gitnexus-query/src/parser.rs:58-64
```

## Q6 — "ASP.NET MVC controller action extraction"

```
Found 5 results for 'ASP.NET MVC controller action extraction':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] enrich_aspnet_mvc               crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:55-1237
    2. [Function  ] resolve_action_node_id          crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1621-1675
    3. [Function  ] parse_action_method             crates/gitnexus-lang/src/route_extractors/csharp/controllers.rs:124-258
    4. [Function  ] infer_controller_from_view_path  crates/gitnexus-ingest/src/phases/aspnet_mvc.rs:1592-1614
    5. [Function  ] extract_action_filters          crates/gitnexus-lang/src/route_extractors/csharp/controllers.rs:265-293
```

## Q7 — "tree-sitter parsing"

```
[2m2026-04-24T13:09:44.642979Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
Found 5 results for 'tree-sitter parsing':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] walk_tree_for_complexity        crates/gitnexus-ingest/src/phases/parsing.rs:1088-1140
    2. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/javascript.rs:21-23
    3. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/csharp.rs:26-28
    4. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/java.rs:26-28
    5. [Function  ] tree_sitter_queries             crates/gitnexus-lang/src/languages/c.rs:21-23
```

## Q8 — "LLM configuration and authorization header"

```
Found 5 results for 'LLM configuration and authorization header':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] enrich_with_llm                 crates/gitnexus-ingest/src/phases/llm_enrichment.rs:512-647
    2. [Function  ] parse_llm_response              crates/gitnexus-ingest/src/phases/llm_enrichment.rs:425-444
    3. [Function  ] build_skeleton_flowchart_returns_header_for_empty_graph  crates/gitnexus-desktop/src/commands/diagram.rs:794-801
    4. [Function  ] test_parse_llm_response_direct  crates/gitnexus-ingest/src/phases/llm_enrichment.rs:668-674
    5. [Function  ] parse_and_expr                  crates/gitnexus-query/src/parser.rs:301-321
```

## Q9 — "BM25 implementation"

```
[2m2026-04-24T13:09:54.650085Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:09:56.188809Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
[2m2026-04-24T13:10:00.995257Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m3 [3mbackoff_ms[0m[2m=[0m4000
Warning: reranker failed, falling back to BM25 order: reranker HTTP 503 Service Unavailable: [{
  "error": {
    "code": 503,
    "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
    "status": "UNAVAILABLE"
  }
}
]
Found 5 results for 'BM25 implementation':
  (reranked by LLM from top-15 BM25 pool)

    1. [Function  ] search_fts                      crates/gitnexus-search/src/bm25.rs:55-97
    2. [Function  ] make_bm25                       crates/gitnexus-search/src/hybrid.rs:139-150
    3. [Function  ] build_fts_query                 crates/gitnexus-search/src/bm25.rs:108-110
    4. [Function  ] parse_fts_row                   crates/gitnexus-search/src/bm25.rs:113-145
    5. [Function  ] test_rrf_only_bm25              crates/gitnexus-search/src/hybrid.rs:195-201
```

## Q10 — "detect dead code functions"

```
[2m2026-04-24T13:10:07.576822Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:10:08.912152Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
[2m2026-04-24T13:10:11.764368Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m3 [3mbackoff_ms[0m[2m=[0m4000
Warning: reranker failed, falling back to BM25 order: reranker HTTP 503 Service Unavailable: [{
  "error": {
    "code": 503,
    "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
    "status": "UNAVAILABLE"
  }
}
]
Found 5 results for 'detect dead code functions':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] mark_dead_code                  crates/gitnexus-ingest/src/phases/dead_code.rs:18-198
    2. [File      ] dead_code.rs                    crates/gitnexus-ingest/src/phases/dead_code.rs
    3. [Module    ] dead_code                       crates/gitnexus-ingest/src/phases/mod.rs:8-8
    4. [Function  ] test_pipeline_javascript_functions  crates/gitnexus-ingest/src/pipeline.rs:741-785
    5. [Function  ] code_review_run                 crates/gitnexus-desktop/src/commands/code_review.rs:28-189
```

## Q11 — "how does the ingest pipeline orchestrate phases"

```
[2m2026-04-24T13:10:18.815136Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:10:21.473333Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
[2m2026-04-24T13:10:24.564141Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m3 [3mbackoff_ms[0m[2m=[0m4000
Warning: reranker failed, falling back to BM25 order: reranker HTTP 503 Service Unavailable: [{
  "error": {
    "code": 503,
    "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
    "status": "UNAVAILABLE"
  }
}
]
Found 5 results for 'how does the ingest pipeline orchestrate phases':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] run_pipeline                    crates/gitnexus-ingest/src/pipeline.rs:44-478
    2. [Function  ] test_pipeline_python_classes    crates/gitnexus-ingest/src/pipeline.rs:862-905
    3. [Function  ] test_pipeline_multiple_languages  crates/gitnexus-ingest/src/pipeline.rs:829-859
    4. [Function  ] test_pipeline_error_recovery    crates/gitnexus-ingest/src/pipeline.rs:806-826
    5. [Function  ] test_pipeline_empty_project     crates/gitnexus-ingest/src/pipeline.rs:788-803
```

## Q12 — "where is the C# DI resolver"

```
[2m2026-04-24T13:10:32.409088Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m1 [3mbackoff_ms[0m[2m=[0m1000
[2m2026-04-24T13:10:34.488100Z[0m [33m WARN[0m [2mgitnexus_search::reranker::llm[0m[2m:[0m reranker transient error, retrying [3mstatus[0m[2m=[0m503 Service Unavailable [3mattempt[0m[2m=[0m2 [3mbackoff_ms[0m[2m=[0m2000
Found 5 results for 'where is the C# DI resolver':
  (reranked by LLM from top-20 BM25 pool)

    1. [File      ] di.rs                           crates/gitnexus-lang/src/route_extractors/csharp/di.rs
    2. [Function  ] extract_di_registrations        crates/gitnexus-lang/src/route_extractors/csharp/di.rs:36-140
    3. [Function  ] test_extract_di_autofac         crates/gitnexus-lang/src/route_extractors/csharp/di.rs:147-166
    4. [Function  ] main                            target-codex/debug/build/tree-sitter-c-sharp-40e041a8cda36db4/out/flag_check.c:1-1
    5. [Function  ] eval_where                      crates/gitnexus-db/src/inmemory/cypher.rs:1037-1144
```

## Q13 — "snapshot persistence format"

```
Found 5 results for 'snapshot persistence format':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] save_snapshot                   crates/gitnexus-db/src/snapshot.rs:28-110
    2. [Function  ] load_snapshot                   crates/gitnexus-db/src/snapshot.rs:113-120
    3. [Function  ] snapshot_create                 crates/gitnexus-desktop/src/commands/snapshots.rs:106-162
    4. [Function  ] write_snapshot_metadata         crates/gitnexus-desktop/src/commands/snapshots.rs:77-84
    5. [Function  ] snapshot_meta_path              crates/gitnexus-desktop/src/commands/snapshots.rs:66-72
```

## Q14 — "chat streaming cancellation"

```
Found 5 results for 'chat streaming cancellation':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] chat_cancel                     crates/gitnexus-desktop/src/commands/chat.rs:3149-3154
    2. [Function  ] chat_ask                        crates/gitnexus-desktop/src/commands/chat.rs:646-1125
    3. [Function  ] chat_get_config                 crates/gitnexus-desktop/src/commands/chat.rs:1129-1131
    4. [Function  ] chat_test_connection            crates/gitnexus-desktop/src/commands/chat.rs:1156-1231
    5. [Function  ] chat_retry_tool                 crates/gitnexus-desktop/src/commands/chat.rs:1447-1504
```

## Q15 — "why is cxx-build version pinned"

```
Found 5 results for 'why is cxx-build version pinned':
  (reranked by LLM from top-20 BM25 pool)

    1. [Function  ] extract_package_version         crates/gitnexus-lang/src/component_detection.rs:454-477
    2. [Function  ] test_read_version_resource      crates/gitnexus-mcp/src/resources.rs:292-300
    3. [Function  ] test_extract_package_version    crates/gitnexus-lang/src/component_detection.rs:862-868
    4. [Function  ] is_eof                          target-codex/debug/build/cssparser-9957a2a9eb2a811c/out/tokenizer.rs:64-66
    5. [Function  ] is_ident_start                  target-codex/debug/build/cssparser-9957a2a9eb2a811c/out/tokenizer.rs:539-581
```

