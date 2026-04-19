# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GitNexus is a Rust-based code intelligence system that builds a knowledge graph from source code and exposes it via MCP (Model Context Protocol) for AI-powered code analysis. It supports 14 programming languages via tree-sitter parsing, with deep ASP.NET MVC 5 / Entity Framework 6 enrichment.

## Build Commands

```bash
# Build (debug)
cargo build

# Build (release, with thin LTO)
cargo build --release

# Build with KuzuDB backend
cargo build --features gitnexus-cli/kuzu-backend

# Build with semantic search (ONNX embeddings)
cargo build --features gitnexus-search/embeddings

# Build the desktop app (Tauri v2)
cd crates/gitnexus-desktop/ui && npm install && npm run build && cd ../../..
cargo build -p gitnexus-desktop

# Run the desktop app in dev mode (with hot reload)
# Dev server on port 1421 (Tauri's default 1420 is commonly used by other
# projects on this machine — kept at 1421 to avoid collisions; see
# tauri.conf.json devUrl/beforeDevCommand).
cd crates/gitnexus-desktop && cargo tauri dev

# Frontend only (from ui/ dir)
cd crates/gitnexus-desktop/ui && npm run dev    # Vite dev server
cd crates/gitnexus-desktop/ui && npm run lint   # ESLint
cd crates/gitnexus-desktop/ui && npm run build  # tsc + vite build

# Build NexusBrain (separate Tauri app)
cd nexus-brain && npm install && npm run build && cd ..
cd nexus-brain/src-tauri && cargo build && cd ../..

# Run NexusBrain in dev mode
cd nexus-brain/src-tauri && cargo tauri dev

# Run the CLI
cargo run -p gitnexus-cli -- <command>

# Run tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p gitnexus-ingest

# Run a single test by name
cargo test -p gitnexus-ingest -- test_name

# Check all crates compile
cargo check --workspace

# Clippy
cargo clippy --workspace
```

## Workspace Architecture

12 active crates in `crates/` (gitnexus-lsp, gitnexus-storage, and gitnexus-types are excluded from the workspace), with a strict dependency flow:

```
gitnexus-cli (binary: "gitnexus")
  ├── gitnexus-mcp        (MCP server: 13 tools, stdio/HTTP transport, JSON-RPC 2.0)
  ├── gitnexus-search      (Hybrid search: BM25 + optional ONNX semantic + RRF fusion)
  ├── gitnexus-db          (Database adapter: InMemory backend or optional KuzuDB)
  ├── gitnexus-ingest      (8-phase ingestion pipeline, parallel with rayon)
  │     └── gitnexus-lang  (14 language providers: tree-sitter queries, import resolution, type extraction)
  ├── gitnexus-query       (Query execution)
  ├── gitnexus-output      (Output formatting)
  ├── gitnexus-git         (Git analytics: hotspots, coupling, ownership)
  ├── gitnexus-rag         (GraphRAG: doc chunking and semantic anchoring into the graph)
  └── gitnexus-core        (Core types: KnowledgeGraph, NodeLabel, SymbolTable, config)
       └── gitnexus-types  (Shared type definitions)

gitnexus-desktop (Tauri v2 desktop app)
  ├── gitnexus-db          (direct graph/index/FTS access, NOT via MCP envelope)
  ├── gitnexus-git
  ├── gitnexus-search
  └── gitnexus-core

nexus-brain (separate Tauri v2 app — "Knowledge IDE", Obsidian-like vault editor)
  └── standalone — reads Markdown Vaults exported by GitNexus, not a workspace member
```

**Core** (`gitnexus-core`): In-memory knowledge graph with HashMap-based O(1) node/relationship lookup. Defines `NodeLabel` (52 variants), `RelationshipType` (27 variants including `Calls`, `HasMethod`, `HasProperty`, `HasAction`), `SymbolDefinition`, and pipeline types. Node properties include `is_traced`, `is_dead_candidate`, `trace_call_count`. Not thread-safe on its own; wrapped in `Arc<RwLock<>>` when shared.

**Lang** (`gitnexus-lang`): `LanguageProvider` trait with 14 implementations (JS, TS, Python, Java, C, C++, C#, Go, Rust, PHP, Ruby, Kotlin, Swift, Razor). Each provider supplies tree-sitter query strings (`queries/`), import resolvers (`import_resolvers/`), named binding extractors (`named_bindings/`), type extractors, export detection, and call routing. Static dispatch via `registry.rs` (match on language variant, zero-cost).

**Ingest** (`gitnexus-ingest`): Pipeline orchestrator in `pipeline.rs` runs 7 phases (some with sub-phases) sequentially:
1. **Structure** — filesystem walk, File/Folder nodes
2. **Parsing** — tree-sitter AST extraction, creates `HasMethod`/`HasProperty` nesting edges via `find_enclosing_class_id` parent-chain walk. Sub-phase 2b detects `.csproj` component libraries
3. **Imports** — import resolution, File→File edges
4. **Calls** — Method→Method resolution with 4 tiers: DI field-type (0.85 confidence), static-call (0.80), same-file, global fuzzy. Handles C# `using` and `var = new` patterns
5. **Heritage** — class inheritance/implementation edges. Sub-phase 5b: ASP.NET MVC 5 / EF6 enrichment (14 passes: controllers, actions, views, entities, DbContexts, services, AJAX, UI components, StackLogger tracing propagation)
6. **Community** (6a) + **Process detection** (6b) — cluster and process grouping
7. **Dead Code** — marks methods with 0 incoming Calls as `is_dead_candidate` (entry points excluded)

Uses rayon for parallel file processing with a 20MB chunk budget and LRU AST cache (cap 50).

**DB** (`gitnexus-db`): `DatabaseBackend` trait with `InMemoryBackend` (default, includes simple Cypher executor and BM25 FTS) and `KuzuDbBackend` (feature-gated via `kuzu-backend`). Schema defines 56 node tables with a unified `CodeRelation` relationship table. Persistence via bincode snapshots (`graph.bin`). Query results returned as `Vec<serde_json::Value>`.

**Search** (`gitnexus-search`): Reciprocal Rank Fusion (K=60) merging BM25 lexical results with optional ONNX-based semantic embeddings. Gracefully degrades without the `embeddings` feature.

**MCP** (`gitnexus-mcp`): Implements MCP protocol version 2024-11-05. Nineteen tools dispatched in `backend/local.rs`:
- **Graph & query**: `list_repos`, `query`, `context`, `impact`, `detect_changes`, `rename`, `cypher`, `search_code`, `read_file`
- **Analytics**: `hotspots`, `coupling`, `ownership`, `coverage`, `diagram`, `report`, `business`, `analyze_execution_trace`
- **Agent support**: `get_insights`, `save_memory`

Stdio and HTTP transports. `LocalBackend` coordinates registry loading and tool dispatch.

**Git** (`gitnexus-git`): Git history analysis: `analyze_hotspots` (file churn scoring), `analyze_coupling` (temporal coupling between files), `analyze_ownership` (author distribution per file). Used by CLI and desktop app.

**RAG** (`gitnexus-rag`): GraphRAG integration — ingests external documentation (Markdown, PDF, DOCX), chunks it into `DocChunk` structs, and anchors chunks semantically into the knowledge graph. Uses pulldown-cmark for Markdown parsing.

**Desktop** (`gitnexus-desktop`): Tauri v2 desktop app with React 19 frontend. Accesses `KnowledgeGraph` + `GraphIndexes` + `FtsIndex` directly via Tauri IPC (not via MCP envelope — this is a deliberate performance choice). Frontend uses Sigma.js + Graphology for graph visualization, Zustand + TanStack Query for state, Tailwind CSS v4 + framer-motion for styling/animations. 35 Tauri command modules in `src/commands/` bridge frontend↔Rust. Four app modes: Explorer (graph + lenses), Analyze (hotspots/coupling/ownership/coverage/diagram/report/health), Chat (LLM Q&A with context), Manage (repo CRUD). State in `ui/src/stores/app-store.ts` (Zustand) and `ui/src/hooks/use-tauri-query.ts` (TanStack Query wrapper for IPC). Graph rendering: `ui/src/components/graph/GraphExplorer.tsx` (hot path).

**NexusBrain** (`nexus-brain/`): Separate Tauri v2 app (not a workspace member) — an Obsidian-like "Knowledge IDE" that reads Markdown Vaults exported by the GitNexus desktop app's "Digital Brain" export feature. React 18 + Tailwind CSS 3 frontend, md-editor-rt for Markdown editing, react-force-graph-2d for graph visualization. Independent from the Rust workspace — has its own `Cargo.toml` under `src-tauri/`.

## Feature Flags

| Flag | Crate | Default | Purpose |
|------|-------|---------|---------|
| `kuzu-backend` | gitnexus-db, gitnexus-cli | off | Real KuzuDB graph database backend |
| `kotlin` | gitnexus-ingest | on | Kotlin tree-sitter grammar |
| `swift` | gitnexus-ingest | on | Swift tree-sitter grammar |
| `embeddings` | gitnexus-search | off | ONNX Runtime semantic search |

## Key Design Patterns

- **Node ID format**: Deterministic `"${Label}:${qualifiedName}"` (e.g., `"Function:src/main.ts:handleLogin"`). Stable across serialization.
- **Method→Method Calls**: `extract_call` uses `find_enclosing_method_id` to walk the tree-sitter parent chain and set the enclosing method as the Calls edge source (not the File node).
- **Class→Method nesting**: `create_definition_node` uses `find_enclosing_class_id` to emit `HasMethod`/`HasProperty` edges from the parent class to each member.
- **Controller→Service traversal**: Commands seed BFS with the sibling Class node (same name/file) to access `HasMethod` children that carry the Calls edges.
- **Tracing coverage**: `extract_tracing_info` detects `StackLogger.BeginMethodScope()` per method, propagated to Method nodes as `is_traced`.
- **Enum-based edge filtering**: Commands use `matches!(rel.rel_type, RelationshipType::...)` instead of string comparison. Note: `as_str()` on RelationshipType returns `SCREAMING_SNAKE_CASE`.
- **Cypher WHERE**: Supports `AND`, `OR`, `NOT` with correct precedence (NOT > AND > OR). Operators: `=`, `<>`, `!=`, `CONTAINS`, `STARTS WITH`, `ENDS WITH`. Example: `WHERE n.name STARTS WITH 'handle' AND NOT n.filePath ENDS WITH '.test.ts'`.
- **Cypher DISTINCT**: `RETURN DISTINCT n.name` deduplicates results before ORDER BY/LIMIT.
- **Pipeline phases**: Each phase in `crates/gitnexus-ingest/src/phases/` takes the graph and enriches it. Phases are sequential but file processing within each phase is parallel via rayon.
- **Database adapter pattern**: `DatabaseBackend` trait in `crates/gitnexus-db/src/adapter.rs` abstracts storage. The in-memory backend is always available; KuzuDB is opt-in.
- **Runtime data**: Indexed repos store data in `.gitnexus/` (meta.json, graph.bin, csv/).

## Gotchas

- **MCP stdout is sacred**: MCP mode logs to stderr to avoid polluting the stdout JSON-RPC stream. Never add `println!` in MCP code paths.
- **Snapshot is JSON, not bincode**: Despite the filename `graph.bin`, snapshots are JSON. Bincode is incompatible because `NodeProperties` uses `#[serde(skip_serializing_if)]` on ~40 optional fields, which breaks bincode's positional format. Migrating requires either removing skip attributes (breaking JSON API), using bincode 2.x `Encode`/`Decode` traits, or switching to MessagePack (`rmp-serde`).
- **cxx-build pin on Windows**: `cxx-build` is pinned to `=1.0.138` to match kuzu's `cxx` version. Newer cxx-build encodes patch versions into bridge symbol names, causing `LNK2019` linker errors on Windows. Do not bump without verifying kuzu compatibility.
- **Adding a new language**: Implement `LanguageProvider` in `crates/gitnexus-lang/src/languages/`, add query strings in `queries/`, an import resolver in `import_resolvers/`, and register in `registry.rs`.
- **LLM enrichment Authorization header**: `call_structured_llm` in `crates/gitnexus-cli/src/commands/generate/enrichment.rs` MUST set the `Authorization: Bearer <key>` header for OpenAI-compatible endpoints (Gemini via `generativelanguage.googleapis.com/v1beta/openai`). Missing header → HTTP 400 "Missing or invalid Authorization header" on section-mode calls (fixed L944-946).
- **Sectioned enrichment anchors**: Pages >50KB are split into multiple LLM calls per anchor (`INTRO`, `SERVICES`, `ENTITIES`, etc.). Before the Phase A fix, only `INTRO` anchor was supported — non-INTRO anchors never triggered sectioned mode, causing systematic truncation on modules like `dossiers.md`. Fix is in `enrichment.rs` L1266-1331.
- **LLM response cache**: Enrichment responses are cached in `<repo>/.gitnexus/docs/_meta/cache/llm/*.txt` keyed by MD5 of the full request body. A re-run reuses all cached responses gratis — extremely useful for retry with different models/settings without re-burning tokens.
- **Gemini Flash output ceiling**: Gemini 2.5 Flash truncates at ~65K output tokens (`finish_reason: length`). On large pages this fires constantly. The fallback freeform parser recovers ~60% of truncated responses; the rest go to the auto retry queue with reduced scope. For quality runs on large repos, prefer Gemini 3.1 Pro Preview (65K native, fewer truncations).

## Rust Version and Toolchain

- MSRV: 1.75
- Edition: 2021
- Release profile: thin LTO, single codegen unit, stripped binaries, opt-level 3
