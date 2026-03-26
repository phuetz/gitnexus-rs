# GitNexus

Graph-powered code intelligence for AI agents. GitNexus builds a knowledge graph from your codebase and exposes it via [MCP](https://modelcontextprotocol.io/) (Model Context Protocol) for AI-powered code analysis.

Written in Rust. Supports 13 programming languages.

## Features

- **Knowledge Graph** -- Parses source code into a rich graph of symbols (functions, classes, modules, imports, calls, inheritance) with 38 node types and typed relationships
- **13 Languages** -- JavaScript, TypeScript, Python, Java, C, C++, C#, Go, Rust, Ruby, PHP, Kotlin, Swift via tree-sitter
- **MCP Server** -- 7 tools accessible to any MCP-compatible AI agent (Claude, Cursor, VS Code, etc.)
- **Hybrid Search** -- BM25 lexical search + optional ONNX semantic embeddings, fused with Reciprocal Rank Fusion
- **Blast Radius Analysis** -- Trace upstream callers, downstream callees, and transitive impact of any symbol
- **Interactive Modes** -- REPL shell, TUI dashboard, file watcher with auto-reindex
- **Pluggable Storage** -- In-memory backend (default) or KuzuDB graph database

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- A C compiler (required for tree-sitter grammar compilation)

### Build

```bash
git clone https://github.com/anthropics/gitnexus-rs.git
cd gitnexus-rs
cargo build --release
```

The binary is at `target/release/gitnexus`.

### Index a Repository

```bash
# Index the current directory
gitnexus analyze

# Index a specific path
gitnexus analyze /path/to/repo

# Force re-index
gitnexus analyze --force
```

This creates a `.gitnexus/` directory containing the serialized knowledge graph.

### Search & Explore

```bash
# Natural language search
gitnexus query "authentication middleware"

# 360-degree symbol context (callers, callees, imports, hierarchy)
gitnexus context UserService

# Blast radius analysis
gitnexus impact handleRequest --direction both

# Raw Cypher query
gitnexus cypher "MATCH (n:Function) RETURN n.name LIMIT 10"
```

### Start the MCP Server

```bash
# Stdio transport (for Claude, Cursor, etc.)
gitnexus mcp

# Auto-configure MCP in your editor
gitnexus setup
```

### Other Commands

```bash
gitnexus list          # List indexed repositories
gitnexus status        # Show index status
gitnexus shell         # Interactive REPL
gitnexus dashboard     # TUI dashboard
gitnexus watch         # Watch & auto-reindex on file changes
gitnexus serve         # HTTP server (default port 3000)
gitnexus generate all  # Generate AGENTS.md, wiki/, skills/
gitnexus clean         # Delete index
```

## MCP Tools

When running as an MCP server, GitNexus exposes these tools:

| Tool | Description |
|------|-------------|
| `list_repos` | List indexed repositories with stats |
| `query` | Natural language search across the knowledge graph |
| `context` | 360-degree view of a symbol: callers, callees, imports, exports, hierarchy |
| `impact` | Blast radius analysis -- upstream, downstream, or both |
| `detect_changes` | Analyze uncommitted changes and their impact |
| `rename` | Find all references that would need updating for a symbol rename |
| `cypher` | Execute a raw read-only Cypher query |

## Architecture

Seven workspace crates with a layered dependency flow:

```
gitnexus-cli          CLI binary ("gitnexus")
  |
  +-- gitnexus-mcp      MCP server (7 tools, stdio/HTTP, JSON-RPC 2.0)
  +-- gitnexus-search    Hybrid search (BM25 + semantic + RRF)
  +-- gitnexus-db        Database adapter (in-memory or KuzuDB)
  +-- gitnexus-ingest    6-phase ingestion pipeline (parallel via rayon)
  |     +-- gitnexus-lang  13 language providers (tree-sitter)
  +-- gitnexus-core      Core types: KnowledgeGraph, NodeLabel, SymbolTable
```

### Ingestion Pipeline

The pipeline runs 6 sequential phases, with parallel file processing within each phase:

1. **Structure** -- Filesystem walk, create File/Folder nodes
2. **Parsing** -- Tree-sitter AST extraction, symbol creation
3. **Imports** -- Import statement extraction and resolution
4. **Calls** -- Function/method call extraction and linking
5. **Heritage** -- Class inheritance and interface implementation
6. **Community** -- Community detection and clustering

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `kuzu-backend` | off | Enable KuzuDB graph database backend |
| `embeddings` | off | Enable ONNX Runtime semantic search |
| `kotlin` | on | Kotlin tree-sitter grammar |
| `swift` | on | Swift tree-sitter grammar |

```bash
# Build with KuzuDB support
cargo build --release --features gitnexus-cli/kuzu-backend

# Build with semantic search
cargo build --release --features gitnexus-search/embeddings
```

## Supported Languages

| Language | Extensions |
|----------|------------|
| JavaScript | `.js` `.jsx` `.mjs` `.cjs` |
| TypeScript | `.ts` `.tsx` `.mts` `.cts` |
| Python | `.py` `.pyi` |
| Java | `.java` |
| C | `.c` `.h` |
| C++ | `.cpp` `.hpp` `.cc` `.hh` `.cxx` `.hxx` |
| C# | `.cs` |
| Go | `.go` |
| Rust | `.rs` |
| Ruby | `.rb` |
| PHP | `.php` |
| Kotlin | `.kt` `.kts` |
| Swift | `.swift` |

## License

PolyForm Noncommercial 1.0.0
