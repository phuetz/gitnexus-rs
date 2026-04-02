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
cd crates/gitnexus-desktop && cargo tauri dev

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

12 active crates in `crates/` (gitnexus-lsp and gitnexus-storage are excluded from the workspace), with a strict dependency flow:

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
  └── gitnexus-core        (Core types: KnowledgeGraph, NodeLabel, SymbolTable, config)
       └── gitnexus-types  (Shared type definitions)

gitnexus-desktop (Tauri v2 desktop app)
  ├── gitnexus-db          (direct graph/index/FTS access, NOT via MCP envelope)
  ├── gitnexus-git
  ├── gitnexus-search
  └── gitnexus-core
```

**Core** (`gitnexus-core`): In-memory knowledge graph with HashMap-based O(1) node/relationship lookup. Defines `NodeLabel` (38 variants), `RelationshipType` (34 variants including `Calls`, `HasMethod`, `HasProperty`, `HasAction`), `SymbolDefinition`, and pipeline types. Node properties include `is_traced`, `is_dead_candidate`, `trace_call_count`. Not thread-safe on its own; wrapped in `Arc<RwLock<>>` when shared.

**Lang** (`gitnexus-lang`): `LanguageProvider` trait with 14 implementations (JS, TS, Python, Java, C, C++, C#, Go, Rust, PHP, Ruby, Kotlin, Swift, Razor). Each provider supplies tree-sitter query strings (`queries/`), import resolvers (`import_resolvers/`), named binding extractors (`named_bindings/`), type extractors, export detection, and call routing. Static dispatch via `registry.rs` (match on language variant, zero-cost).

**Ingest** (`gitnexus-ingest`): Pipeline orchestrator in `pipeline.rs` runs 8 phases sequentially:
1. **Structure** — filesystem walk, File/Folder nodes
2. **Parsing** — tree-sitter AST extraction, creates `HasMethod`/`HasProperty` nesting edges via `find_enclosing_class_id` parent-chain walk
3. **Imports** — import resolution, File→File edges
4. **Calls** — Method→Method resolution with 4 tiers: DI field-type (0.85 confidence), static-call (0.80), same-file, global fuzzy. Handles C# `using` and `var = new` patterns
5. **Heritage** — class inheritance/implementation edges
6. **ASP.NET MVC** — 14 passes: controllers, actions, views, entities, DbContexts, services, AJAX, UI components, StackLogger tracing propagation to Method nodes
7. **Community** — cluster detection
8. **Dead Code** — marks methods with 0 incoming Calls as `is_dead_candidate` (entry points excluded)

Uses rayon for parallel file processing with a 20MB chunk budget and LRU AST cache (cap 50).

**DB** (`gitnexus-db`): `DatabaseBackend` trait with `InMemoryBackend` (default, includes simple Cypher executor and BM25 FTS) and `KuzuDbBackend` (feature-gated via `kuzu-backend`). Schema defines 35 node tables with a unified `CodeRelation` relationship table. Persistence via bincode snapshots (`graph.bin`). Query results returned as `Vec<serde_json::Value>`.

**Search** (`gitnexus-search`): Reciprocal Rank Fusion (K=60) merging BM25 lexical results with optional ONNX-based semantic embeddings. Gracefully degrades without the `embeddings` feature.

**MCP** (`gitnexus-mcp`): Implements MCP protocol version 2024-11-05. Thirteen tools: `list_repos`, `query`, `context`, `impact`, `detect_changes`, `rename`, `cypher`, `hotspots`, `coupling`, `ownership`, `coverage`, `diagram`, `report`. Stdio and HTTP transports. `LocalBackend` coordinates registry loading and tool dispatch.

**Git** (`gitnexus-git`): Git history analysis: `analyze_hotspots` (file churn scoring), `analyze_coupling` (temporal coupling between files), `analyze_ownership` (author distribution per file). Used by CLI and desktop app.

**Desktop** (`gitnexus-desktop`): Tauri v2 desktop app with React 19 frontend. Accesses `KnowledgeGraph` + `GraphIndexes` + `FtsIndex` directly via Tauri IPC (not via MCP envelope — this is a deliberate performance choice). Frontend uses Cytoscape.js for graph visualization, Zustand + TanStack Query for state, Tailwind CSS + framer-motion for styling/animations.

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

## Rust Version and Toolchain

- MSRV: 1.75
- Edition: 2021
- Release profile: thin LTO, single codegen unit, stripped binaries, opt-level 3
