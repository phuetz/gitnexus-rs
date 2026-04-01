# AGENTS.md — GitNexus Codebase Context

## Project Overview

GitNexus is a Rust-based code intelligence system that builds a knowledge graph from source code via tree-sitter parsing. It supports 14 languages with deep ASP.NET MVC 5 / Entity Framework 6 enrichment. Exposed via CLI, MCP server, and Tauri desktop app.

## Build Commands

```bash
cargo build --release                    # Release build (CLI binary)
cargo test --workspace                   # Run all tests
cargo run -p gitnexus-cli -- <command>   # Run CLI in dev mode
```

Binary: `target/release/gitnexus` (or `gitnexus.exe` on Windows)

## Workspace Architecture (14 crates)

```
gitnexus-cli          CLI binary with 27 commands
gitnexus-mcp          MCP server (7 tools, stdio/HTTP)
gitnexus-ingest       8-phase ingestion pipeline
  gitnexus-lang       14 language providers (tree-sitter queries)
gitnexus-db           Database adapter (InMemory + optional KuzuDB)
gitnexus-search       Hybrid search (BM25 + optional ONNX semantic)
gitnexus-core         Core types: KnowledgeGraph, NodeLabel, RelationshipType
gitnexus-git          Git analytics (hotspots, coupling, ownership)
gitnexus-desktop      Tauri v2 desktop app (React 19 frontend)
gitnexus-types        Shared type definitions
gitnexus-query        Query execution
gitnexus-output       Output formatting
```

## Ingestion Pipeline (8 phases)

| Phase | File | Description |
|-------|------|-------------|
| 1. Structure | `phases/structure.rs` | Filesystem walk, File/Folder nodes |
| 2. Parsing | `phases/parsing.rs` | Tree-sitter AST extraction. Creates `HasMethod`/`HasProperty` nesting edges via `find_enclosing_class_id`. Creates Method→Method `Calls` edges via `find_enclosing_method_id` |
| 3. Imports | `phases/imports.rs` | Import resolution, File→File edges |
| 4. Calls | `phases/calls.rs` | Call resolution with 4 tiers: DI field-type (0.85), static-call (0.80), same-file, global fuzzy. Handles C# `using` and `var = new` patterns |
| 5. Heritage | `phases/heritage.rs` | Inheritance/implementation edges |
| 6. ASP.NET MVC | `phases/aspnet_mvc.rs` | 14 passes: Controllers, Actions, Views, Entities, DbContexts, Services, AJAX, UI components, StackLogger tracing propagation to Method nodes |
| 7. Communities | `phases/community.rs` | Community detection |
| 8. Dead Code | `phases/dead_code.rs` | Marks methods with 0 incoming Calls as `is_dead_candidate` |

## Key Graph Types

**Node Labels** (38 variants): Class, Method, Constructor, Property, Controller, ControllerAction, Service, Repository, View, DbEntity, DbContext, File, Folder, Namespace, Interface, Enum, Struct, ExternalService, AjaxCall, UiComponent, ScriptFile, Community, Process, ...

**Relationship Types** (34 variants): Calls, HasMethod, HasProperty, HasAction, RendersView, CallsAction, CallsService, DependsOn, Inherits, Implements, Defines, Contains, Imports, MemberOf, MapsToEntity, AssociatesWith, HasFilter, BelongsToArea, ...

**Key Node Properties**: `name`, `file_path`, `start_line`, `end_line`, `is_traced` (StackLogger coverage), `is_dead_candidate` (0 incoming Calls), `trace_call_count`, `return_type`, `parameter_count`

## CLI Commands (27)

### Core Analysis
- `analyze [path] [--force]` — Index a repository
- `status` — Check index status
- `query "text" [--limit N]` — Natural language search
- `context <symbol>` — 360-degree symbol view (callers, callees, relationships)
- `impact <symbol> --direction upstream|downstream|both` — Blast radius analysis
- `cypher "MATCH..."` — Raw Cypher queries

### Tracing & Exploration
- `trace-files <symbol> [--depth N]` — List all source files involved in a feature
- `diagram <symbol> --type flowchart|sequence|class` — Generate Mermaid diagrams
- `coverage [class] [--trace] [--json]` — Tracing coverage & dead code detection

### Documentation & Reports
- `generate html|docs|all [--enrich]` — Generate documentation
- `report [--json]` — Code health report (grade A-E)
- `ask "question"` — LLM-powered Q&A

### Git Analytics
- `hotspots` — Most changed files
- `coupling` — Temporal coupling between files
- `ownership` — Code ownership by author

### Infrastructure
- `mcp` — Start MCP server
- `serve` — Start HTTP server
- `shell` — Interactive shell
- `list` — List indexed repos
- `clean` — Remove index
- `config test` — Validate LLM config
- `trace-import <file>` — Import execution traces

## Key Source Files

| File | Purpose |
|------|---------|
| `crates/gitnexus-cli/src/main.rs` | CLI entry point, command dispatch |
| `crates/gitnexus-cli/src/commands/` | 27 command implementations |
| `crates/gitnexus-ingest/src/pipeline.rs` | Pipeline orchestrator |
| `crates/gitnexus-ingest/src/phases/parsing.rs` | AST extraction + HasMethod/Calls edges |
| `crates/gitnexus-ingest/src/phases/calls.rs` | Call resolution (DI/using/static) |
| `crates/gitnexus-ingest/src/phases/aspnet_mvc.rs` | ASP.NET MVC enrichment (14 passes) |
| `crates/gitnexus-ingest/src/phases/dead_code.rs` | Dead code detection |
| `crates/gitnexus-lang/src/queries/csharp.rs` | C# tree-sitter queries |
| `crates/gitnexus-lang/src/route_extractors/csharp.rs` | C# service/controller/tracing extraction |
| `crates/gitnexus-core/src/graph/types.rs` | NodeLabel, RelationshipType, NodeProperties |
| `crates/gitnexus-core/src/id.rs` | Node ID generation: `"{Label}:{filepath}:{name}"` |
| `crates/gitnexus-db/src/inmemory/cypher.rs` | Cypher query executor |
| `crates/gitnexus-db/src/snapshot.rs` | Graph serialization (graph.bin) |

## Design Patterns

- **Method→Method Calls**: `extract_call` uses `find_enclosing_method_id` to walk the tree-sitter parent chain and set the enclosing method as the Calls edge source (not the File node)
- **Class→Method nesting**: `create_definition_node` uses `find_enclosing_class_id` to emit HasMethod/HasProperty edges from the parent class to each member
- **Controller→Service traversal**: Commands seed BFS with the sibling Class node (same name/file) to access HasMethod children that carry the Calls edges
- **Tracing coverage**: `extract_tracing_info` detects `StackLogger.BeginMethodScope()` per method, propagated to Method nodes as `is_traced`
- **Dead code**: Post-pipeline phase marks methods with 0 incoming Calls edges as `is_dead_candidate` (entry points excluded)
- **Enum-based edge filtering**: Commands use `matches!(rel.rel_type, RelationshipType::...)` instead of string comparison (as_str returns SCREAMING_SNAKE_CASE)
