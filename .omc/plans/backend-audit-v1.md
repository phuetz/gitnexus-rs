# GitNexus Backend — Full Audit Report

**Date**: 2026-04-03
**Scope**: core, types, db, search, ingest, lang, mcp, cli, git, query, output
**Total findings**: 70 (4 CRITICAL, 14 HIGH, 31 MEDIUM, 21 LOW)

---

## CRITICAL (4) — Must fix immediately

### C1. ReDoS vulnerability via user-supplied regex in CQL `=~` operator
**File**: `crates/gitnexus-query/src/executor.rs:733`
**Risk**: A Cypher query `WHERE n.name =~ '(a+)+$'` causes catastrophic backtracking, hanging the process. Reachable via MCP `cypher` tool.
**Fix**: Use `RegexBuilder::new(pattern).size_limit(1 << 20).build()` to cap compiled NFA size.

### C2. HTTP server binds 0.0.0.0 with permissive CORS, no authentication
**Files**: `crates/gitnexus-cli/src/commands/serve.rs:19`, `crates/gitnexus-mcp/src/transport/http.rs:44`
**Risk**: Full knowledge graph + Cypher queries exposed to entire network. Data exfiltration vector.
**Fix**: Bind `127.0.0.1` by default, add `--host` flag, restrictive CORS (localhost only), optional API key.

### C3. Divergent parallel type systems (gitnexus-types vs gitnexus-core)
**Files**: `crates/gitnexus-types/src/node.rs` (42 variants), `crates/gitnexus-core/src/graph/types.rs` (53 variants)
**Risk**: Two independent NodeLabel/RelationshipType/NodeProperties definitions. gitnexus-types is dead code, but schema references it. Silent deserialization failures if mixed.
**Fix**: Delete gitnexus-types or consolidate. All crates already use gitnexus-core.

### C4. `remove_node()` does not remove dangling relationships
**File**: `crates/gitnexus-core/src/graph/knowledge_graph.rs:49-62`
**Risk**: Graph corruption — relationships with dangling source/target references persist after node removal.
**Fix**: Add `self.relationships.retain(|_, rel| rel.source_id != id && rel.target_id != id)`.

---

## HIGH (14)

| # | Issue | File(s) | Fix |
|---|-------|---------|-----|
| H1 | Cypher ORDER BY uses string comparison for numeric fields | `cypher.rs:898-912` | Parse as f64 first, fallback to string |
| H2 | Snapshot rename fails on Windows (destination exists) | `snapshot.rs:28-55` | `remove_file` before `rename` on Windows |
| H3 | ConnectionPool `Arc<DbAdapter>` prevents `close()` | `pool.rs:27-82` | Use `Arc<Mutex<DbAdapter>>` or `Drop` impl |
| H4 | `is_write_query` bypass via comments/newlines | `query.rs:33-61` | Strip comments, use word-boundary regex |
| H5 | O(E^2) performance in diagram tool | `local.rs:743-748` | Pre-build HashSet of child IDs |
| H6 | No Content-Length limit on stdio transport | `stdio.rs:99-113` | Add 64MB max, reject larger |
| H7 | Coupling analysis loads full git history (no time bound) | `coupling.rs:31-39` | Add `since_days` parameter |
| H8 | Ownership `lines` field stores commit count (misleading) | `ownership.rs:92-96` | Rename to `commits` or use `--numstat` |
| H9 | Regex compiled per call in `build_field_type_map` | `calls.rs:21-37` | Use `Lazy<Regex>` statics |
| H10 | 4 pipeline phases have zero unit tests | `calls.rs`, `imports.rs`, `heritage.rs`, `dead_code.rs` | Add targeted unit tests |
| H11 | No tests for 14 language providers or query strings | `crates/gitnexus-lang/src/languages/*.rs` | Per-language query validation tests |
| H12 | `process_match` is 450 lines with 22+ branches | `parsing.rs:485-940` | Refactor to dispatch table |
| H13 | FTS label filter applied AFTER full BM25 scoring | `fts.rs:129-193` | Pre-filter by label or per-label index |
| H14 | FTS only indexes name+filePath (missing description) | `fts.rs:88-89` | Add description, keywords to indexed text |

---

## MEDIUM (31) — Grouped by area

### Core/Types
- M1: NodeProperties 40+ optional fields God Object (`types.rs:392-564`)
- M2: `add_node` doesn't deduplicate file_index entries (`knowledge_graph.rs:34-39`)
- M3: `SymbolTable::add()` clones for both indexes — use Arc (`table.rs:30-47`)
- M4: ResolutionContext cache cleared on file switch (`context.rs:50-55`)
- M5: Schema NODE_LABELS out of sync with NodeLabel enum (`schema.rs:9-68`)

### DB/Cypher
- M6: `get_node_field_str` maps only 15 of 40 fields (`cypher.rs:1068-1096`)
- M7: `node_to_json` serializes only 15 of 40 properties (`cypher.rs:1156-1214`)
- M8: Cypher tokenizer silently skips unknown characters (`cypher.rs:216-219`)

### Ingest/Lang
- M9: `enrich_aspnet_mvc` is 1600-line monolith (`aspnet_mvc.rs:57-1620`)
- M10: `read_to_string` failure produces empty content silently (`structure.rs:71`)
- M11: `build_symbol_table` never populates `owner_id` (`parsing.rs:1575-1605`)
- M12: Incremental update skips ASP.NET MVC enrichment (`incremental.rs:145-182`)
- M13: Community detection non-deterministic (HashMap iteration) (`community.rs:72`)
- M14: AST cache defined but never used (`ast_cache.rs`)
- M15: chunk_files worker infrastructure unused (`workers/mod.rs`)
- M16: C# heritage query misses multiple interface implementations (`queries/csharp.rs:55-59`)
- M17: `find_enclosing_method_id` returns Method for Functions (`parsing.rs:1197-1203`)
- M18: `process_match` silently drops unknown capture patterns (`parsing.rs:485-940`)
- M19: `topological_level_sort` computed but never used (`pipeline.rs:362-434`)

### MCP/CLI/Git
- M20: `initialized` handled as request, not notification (`server.rs:115-119`)
- M21: No query timeout or cancellation support (`local.rs:217-219`)
- M22: Resource URI path traversal not validated (`resources.rs:116-117`)
- M23: Hardcoded French strings in report command (`report.rs:137-172`)
- M24: CLI uses `process::exit(1)` instead of returning errors (`query_cmd.rs:15`)
- M25: No RETURN DISTINCT support in CQL executor (`executor.rs:252-273`)
- M26: Report/coverage tools load full snapshot redundantly (`local.rs:618,714,801`)
- M27: Tests use deprecated `set_var` (unsafe multi-threaded) (`terminal.rs:226`)
- M28: Health score formula starts at 70, max unreachable (`local.rs:817-824`)
- M29: ONNX embedder is placeholder returning zero vectors (`embeddings/mod.rs:30-42`)
- M30: `merge_by_file_path` score metadata logic accidental (`bm25.rs:139-149`)
- M31: `bincode` dependency unused (incompatible per CLAUDE.md) (`Cargo.toml`)

---

## Recommended Fix Priority

### Phase 1 — Security (immediate)
1. C1: ReDoS regex size limit
2. C2: HTTP bind localhost + restrictive CORS
3. H4: Write query bypass via comments
4. H6: Content-Length limit
5. M22: Path traversal validation

### Phase 2 — Data correctness
6. C4: `remove_node` dangling relationships
7. H1: Numeric ORDER BY
8. H2: Windows snapshot rename
9. M6+M7: Complete Cypher field mappings (use serde)
10. M17: Function/Method label mismatch
11. M11: Populate owner_id in symbol table

### Phase 3 — Type system cleanup
12. C3: Consolidate or remove gitnexus-types
13. M5: Sync schema NODE_LABELS
14. M31: Remove unused bincode dependency

### Phase 4 — Performance
15. H5: O(E^2) diagram → O(E) with HashSet
16. H7: Coupling `since_days` parameter
17. H13: FTS label pre-filtering
18. H14: FTS index description+keywords
19. M3: Arc<SymbolDefinition> instead of cloning
20. M26: Cache loaded snapshot

### Phase 5 — Testing
21. H10: Unit tests for calls/imports/heritage/dead_code
22. H11: Per-language provider query validation
23. H12: Split process_match into dispatch table

### Phase 6 — Code organization
24. H9: Lazy regex statics
25. M9: Split enrich_aspnet_mvc
26. M14/M15/M19: Remove or wire dead infrastructure
27. M23: Fix French strings in report
28. M24: Return errors instead of process::exit

---

## Positive observations

- Excellent error handling — `thiserror` throughout, no panics, no unsafe, no unwrap in production
- Clean Cypher injection protection with `escape_cypher_string`
- Correct WHERE clause precedence (NOT > AND > OR) with tests
- Atomic snapshot writes (temp+rename pattern)
- Clean BM25 + RRF hybrid search implementation
- Well-designed LanguageProvider trait with static dispatch
- Correct Louvain community detection
- Robust dead code exclusion matrix (8 categories)
- Good MCP protocol compliance (JSON-RPC 2.0, content-length framing)
- Stdout discipline in MCP mode (logs to stderr)
- 137 passing tests across pipeline crates
