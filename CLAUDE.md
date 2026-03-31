# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GitNexus is a Rust-based code intelligence system that builds a knowledge graph from source code and exposes it via MCP (Model Context Protocol) for AI-powered code analysis. It supports 14 programming languages via tree-sitter parsing.

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

# Run tests (minimal coverage currently)
cargo test --workspace

# Run tests for a specific crate
cargo test -p gitnexus-ingest

# Check all crates compile
cargo check --workspace

# Clippy
cargo clippy --workspace
```

## Workspace Architecture

14 crates in `crates/`, with a strict dependency flow:

```
gitnexus-cli (binary: "gitnexus")
  ├── gitnexus-mcp        (MCP server: 7 tools, stdio/HTTP transport, JSON-RPC 2.0)
  ├── gitnexus-search      (Hybrid search: BM25 + optional ONNX semantic + RRF fusion)
  ├── gitnexus-db          (Database adapter: InMemory backend or optional KuzuDB)
  ├── gitnexus-ingest      (6-phase ingestion pipeline, parallel with rayon)
  │     └── gitnexus-lang  (13 language providers: tree-sitter queries, import resolution, type extraction)
  └── gitnexus-core        (Core types: KnowledgeGraph, NodeLabel, SymbolTable, config)

gitnexus-desktop (Tauri v2 desktop app)
  ├── gitnexus-db          (direct graph/index/FTS access, NOT via MCP envelope)
  ├── gitnexus-git         (hotspots, coupling, ownership, code health)
  ├── gitnexus-search
  └── gitnexus-core
```

**Core** (`gitnexus-core`): In-memory knowledge graph with HashMap-based O(1) node/relationship lookup. Defines `NodeLabel` (38 variants), `RelationshipType`, `SymbolDefinition`, and pipeline types.

**Lang** (`gitnexus-lang`): `LanguageProvider` trait with 13 implementations (JS, TS, Python, Java, C, C++, C#, Go, Rust, PHP, Ruby, Kotlin, Swift). Each provider supplies tree-sitter query strings (`queries/`), import resolvers (`import_resolvers/`), named binding extractors (`named_bindings/`), type extractors, export detection, and call routing. Static dispatch via `registry.rs`.

**Ingest** (`gitnexus-ingest`): Pipeline orchestrator in `pipeline.rs` runs 6 phases sequentially: Structure (filesystem walk) -> Parsing (tree-sitter AST extraction) -> Imports (resolution) -> Calls (function/method calls) -> Heritage (class hierarchy) -> Community (clustering). Uses rayon for parallel file processing with a 20MB chunk budget and LRU AST cache (cap 50).

**DB** (`gitnexus-db`): `DatabaseBackend` trait with `InMemoryBackend` (default, includes simple Cypher executor and BM25 FTS) and `KuzuDbBackend` (feature-gated via `kuzu-backend`). Schema defines 35 node tables with a unified `CodeRelation` relationship table. Persistence via bincode snapshots (`graph.bin`).

**Search** (`gitnexus-search`): Reciprocal Rank Fusion (K=60) merging BM25 lexical results with optional ONNX-based semantic embeddings. Gracefully degrades without the `embeddings` feature.

**MCP** (`gitnexus-mcp`): Implements MCP protocol version 2024-11-05. Seven tools: `list_repos`, `query`, `context`, `impact`, `definition`, `codeql`, `symbol_stats`. Stdio and HTTP transports. `LocalBackend` coordinates registry loading and tool dispatch.

**Git** (`gitnexus-git`): Git history analysis: `analyze_hotspots` (file churn scoring), `analyze_coupling` (temporal coupling between files), `analyze_ownership` (author distribution per file). Used by CLI (`hotspots`, `coupling`, `ownership`, `report` commands) and desktop app.

**CLI** (`gitnexus-cli`): Binary `gitnexus` with commands: `analyze`, `mcp`, `serve`, `list`, `status`, `clean`, `query`, `context`, `impact`, `cypher`, `setup`, `shell`, `generate`, `watch`, `dashboard`, `hotspots`, `coupling`, `ownership`, `ask`, `report`. MCP mode logs to stderr to avoid polluting stdout JSON-RPC.

**Desktop** (`gitnexus-desktop`): Tauri v2 desktop app with React 19 frontend. Accesses `KnowledgeGraph` + `GraphIndexes` + `FtsIndex` directly (not via MCP envelope). React frontend in `crates/gitnexus-desktop/ui/` uses Cytoscape.js for graph visualization (semantic sizing + glow shadows), Zustand + TanStack Query for state, Tailwind CSS + framer-motion for styling/animations. IPC commands: `list_repos`, `open_repo`, `get_graph_data`, `get_subgraph`, `get_neighbors`, `search_symbols`, `search_autocomplete`, `get_symbol_context`, `get_impact_analysis`, `get_file_tree`, `read_file_content`, `execute_cypher`, `get_process_flows`, `get_hotspots`, `get_coupling`, `get_ownership`, `get_code_health`.

## Feature Flags

| Flag | Crate | Default | Purpose |
|------|-------|---------|---------|
| `kuzu-backend` | gitnexus-db, gitnexus-cli | off | Real KuzuDB graph database backend |
| `kotlin` | gitnexus-ingest | on | Kotlin tree-sitter grammar |
| `swift` | gitnexus-ingest | on | Swift tree-sitter grammar |
| `embeddings` | gitnexus-search | off | ONNX Runtime semantic search |

## Key Design Patterns

- **Trait-based language providers**: Adding a new language means implementing `LanguageProvider` in `crates/gitnexus-lang/src/languages/`, adding query strings in `queries/`, an import resolver in `import_resolvers/`, and registering in `registry.rs`.
- **Pipeline phases**: Each phase in `crates/gitnexus-ingest/src/phases/` takes the graph and enriches it. Phases are sequential but file processing within each phase is parallel via rayon.
- **Database adapter pattern**: `DatabaseBackend` trait in `crates/gitnexus-db/src/adapter.rs` abstracts storage. The in-memory backend is always available; KuzuDB is opt-in.
- **Runtime data**: Indexed repos store data in `.gitnexus/` (meta.json, graph.bin, csv/).

## Rust Version and Toolchain

- MSRV: 1.75
- Edition: 2021
- Release profile: thin LTO, single codegen unit, stripped binaries, opt-level 3
