# GitNexus

Graph-powered code intelligence for AI agents. GitNexus builds a knowledge graph from your codebase and exposes it via [MCP](https://modelcontextprotocol.io/) (Model Context Protocol) for AI-powered code analysis.

Written in Rust. Supports 14 programming languages. Ships with a desktop app and an HTML documentation generator.

[Version française](README.fr.md)

## Features

- **Knowledge Graph** -- Parses source code into a rich graph of symbols (functions, classes, modules, imports, calls, inheritance) with 50+ node types and typed relationships
- **14 Languages** -- JavaScript, TypeScript, Python, Java, C, C++, C#, Go, Rust, Ruby, PHP, Kotlin, Swift, Razor via tree-sitter
- **ASP.NET MVC 5 Deep Support** -- Controllers, actions, Razor views, Entity Framework 6 EDMX, Telerik/Kendo UI grids, jQuery/AJAX mapping, service/repository layer detection (see below)
- **HTML Documentation Generator** -- DeepWiki-style single-page HTML site with full-text search (Ctrl+K), syntax highlighting, copy buttons, callouts, breadcrumbs, Previous/Next navigation, scroll spy TOC, mobile responsive
- **LLM Enrichment** -- Optional `--enrich` mode that augments documentation with grounded LLM prose, structured JSON payloads, evidence citations, provenance tracking, and anti-hallucination validation
- **Ask the Codebase** -- `gitnexus ask "question"` CLI command for graph-powered Q&A with streaming responses
- **Desktop App** -- Tauri v2 desktop application with interactive graph visualization, treemap view, intelligent chat, and command palette (Ctrl+K)
- **Intelligent Chat** -- AI-powered code Q&A with streaming responses, query complexity analysis, multi-step research plans, and deep research mode. Supports Ollama, OpenAI, Anthropic, OpenRouter, and Gemini (with reasoning/thinking mode)
- **MCP Server** -- 7 tools accessible to any MCP-compatible AI agent (Claude, Cursor, VS Code, etc.)
- **Hybrid Search** -- BM25 lexical search + optional ONNX semantic embeddings, fused with Reciprocal Rank Fusion
- **Blast Radius Analysis** -- Trace upstream callers, downstream callees, and transitive impact of any symbol
- **Interactive Modes** -- REPL shell, TUI dashboard, file watcher with auto-reindex
- **Pluggable Storage** -- In-memory backend (default) or KuzuDB graph database

## ASP.NET MVC 5 / Legacy .NET Support

GitNexus has deep support for legacy ASP.NET MVC 5 projects, making it ideal for documenting and understanding complex enterprise applications.

### What it detects

| Feature | Detection |
|---------|-----------|
| **Controllers & Actions** | Class inheritance, `[HttpGet/Post]`, `[GridAction]`, route templates, parameter signatures |
| **Razor Views** (.cshtml) | `@model`, `@layout`, `@Html.Partial`, `@Html.RenderAction`, `@Html.BeginForm` |
| **Entity Framework 6** | DbContext, DbSet, EDMX entities, associations, navigation properties, inheritance (TPH/TPT) |
| **Telerik / Kendo UI** | `Html.Telerik().Grid<T>()`, `Html.Kendo().Grid<T>()`, DataSource bindings (`.Select()`, `.Read()`), grid columns, `ClientEvents`, `DatePickerFor`, `DropDownListFor` |
| **jQuery / AJAX** | `$.ajax()`, `$.getJSON()`, `$.post()`, `$.get()`, `$.load()`, `fetch()`, `@Url.Action()` — linked to controller actions |
| **Service Layer** | `*Service`, `*Repository`, `*Manager`, `*Provider`, `*UnitOfWork` classes with interface detection |
| **Dependency Injection** | Autofac (`RegisterType<T>().As<I>()`), Unity, Ninject, MS DI |
| **Custom Attributes** | `[AuthorizeADAttribute]`, `[VerifActionFilter]`, any `[*Attribute]`, `[*Filter]`, `[*Action]` |
| **External Services** | WebAPI client detection (`new CMCASClient(httpClient)`), WCF service references, HTTP call tracing |
| **StackLogger Tracing** | Coverage analysis — identifies which methods are instrumented with `BeginMethodScope()` |
| **Base Controllers** | Custom controller inheritance (`RootController` → `Controller`) |
| **Web.config** | Configuration file detection |

### Generated documentation

The `generate html` command produces a DeepWiki-style HTML documentation site:

```bash
gitnexus analyze D:\path\to\your\mvc5-project
gitnexus generate --path D:\path\to\your\mvc5-project html
# Open .gitnexus/docs/index.html in your browser
```

The HTML site includes:
- **Overview** with technology stack, project structure, and metrics
- **Architecture diagram** (Mermaid) showing Presentation → Business Logic → Data Access layers
- **Per-controller pages** with action signatures, parameters (linked to data model), callers, and source code
- **Data model pages** with per-entity relationship diagrams and per-domain ER diagrams
- **Functional guide** with business descriptions in French, criticality levels, and Mermaid flow diagrams
- **External services page** with full WebAPI method signatures including all overloads
- **Views & Templates** grouped by screen, filtered by type (grids, forms, partials)
- **Service layer** with descriptions and "Used By" controller links
- **Sequence diagrams** for critical flows (beneficiary search, case creation, accounting export)
- **Dark/light theme** toggle with sidebar search and Previous/Next navigation

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- A C compiler (required for tree-sitter grammar compilation)
- Node.js 18+ (for the desktop app frontend only)

### Build

```bash
git clone https://github.com/phuetz/gitnexus-rs.git
cd gitnexus-rs

# Build the CLI (release mode, optimized)
cargo build --release -p gitnexus-cli

# The binary is at:
# Windows: target\release\gitnexus.exe
# Linux/macOS: target/release/gitnexus
```

Build scripts are also provided:

```bash
# Windows
build-release.bat           # Build CLI + Desktop
build-release.bat cli       # CLI only
build-release.bat desktop   # Desktop only

# Linux/macOS
./build-release.sh          # Build CLI + Desktop
./build-release.sh cli      # CLI only
```

### Build the Desktop App

```bash
cd crates/gitnexus-desktop/ui
npm install
npm run build
cd ../../..
cargo build -p gitnexus-desktop --release
```

Or run in development mode with hot reload:

```bash
cd crates/gitnexus-desktop
cargo tauri dev
```

## CLI Usage

### Analyze a project

```bash
# Index the current directory
gitnexus analyze

# Index a specific path (e.g., a legacy ASP.NET MVC project)
gitnexus analyze D:\path\to\project

# Force re-index (resets the graph)
gitnexus analyze D:\path\to\project --force
```

This creates a `.gitnexus/` directory containing the serialized knowledge graph.

### Generate documentation

```bash
# Generate HTML documentation site (recommended)
gitnexus generate --path D:\path\to\project html
# → Open .gitnexus/docs/index.html in your browser

# Generate with LLM enrichment (requires configured LLM)
gitnexus generate --path D:\path\to\project html --enrich
gitnexus generate --path D:\path\to\project html --enrich --enrich-profile strict
gitnexus generate --path D:\path\to\project html --enrich --enrich-lang en

# Generate everything (AGENTS.md, wiki, skills, docs, DOCX, HTML)
gitnexus generate --path D:\path\to\project all

# Generate specific formats
gitnexus generate --path D:\path\to\project docs     # Markdown pages
gitnexus generate --path D:\path\to\project docx     # Word document
gitnexus generate --path D:\path\to\project context   # AGENTS.md only
gitnexus generate --path D:\path\to\project wiki      # Wiki pages
gitnexus generate --path D:\path\to\project skills    # Skill files
```

### Ask the codebase

```bash
# Ask a question using graph context + LLM (streaming response)
gitnexus ask "comment fonctionne le calcul des barèmes ?"
gitnexus ask "which controllers call the Erable WebAPI?" --path D:\taf\Alise_v2
```

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

### Interactive modes

```bash
gitnexus shell         # Interactive REPL with auto-completion
gitnexus dashboard     # TUI dashboard with graph navigation
gitnexus watch         # Watch & auto-reindex on file changes
```

### MCP Server (for AI agents)

```bash
# Stdio transport (for Claude, Cursor, VS Code, etc.)
gitnexus mcp

# Auto-configure MCP in your editor
gitnexus setup

# HTTP server
gitnexus serve         # Default port 3000
```

### Other commands

```bash
gitnexus list          # List indexed repositories with stats
gitnexus status        # Show index status for current repo
gitnexus clean         # Delete index
gitnexus clean --all   # Delete all indexed repos
```

### Full workflow example (ASP.NET MVC project)

```bash
# 1. Build the CLI
cargo build --release -p gitnexus-cli

# 2. Analyze the project
.\target\release\gitnexus.exe analyze D:\taf\MyLegacyApp

# 3. Generate HTML documentation
.\target\release\gitnexus.exe generate --path D:\taf\MyLegacyApp html

# 4. Optionally enrich with LLM (requires chat-config.json)
.\target\release\gitnexus.exe generate --path D:\taf\MyLegacyApp html --enrich

# 5. Open in browser
start D:\taf\MyLegacyApp\.gitnexus\docs\index.html

# 6. Ask questions about the code
.\target\release\gitnexus.exe ask "how does payment validation work?" --path D:\taf\MyLegacyApp

# 7. Or launch the desktop app for interactive exploration
.\target\release\gitnexus-desktop.exe
```

## Desktop App

The GitNexus desktop app is a Tauri v2 application with a React 19 frontend. It provides a visual interface for exploring your codebase's knowledge graph and an intelligent chat system for AI-powered code analysis.

### Graph Explorer

Interactive graph visualization powered by Cytoscape.js with three zoom levels (package, module, symbol), multiple layout algorithms, and click-to-navigate. Select any node to see its full context: callers, callees, imports, exports, and inheritance chain.

### Intelligent Chat

The chat system is the core feature of the desktop app. It goes beyond simple Q&A by analyzing query complexity and executing multi-step research plans when needed.

**Query Complexity Analysis** -- Every question is classified as Simple (direct lookup), Medium (2-3 operations), or Complex (multi-step DAG). The system detects intent patterns in both French and English: definition lookups, usage analysis, impact assessment, architecture questions, comparison, refactoring, and control flow.

**Multi-Step Research Plans** -- For medium and complex queries, the planner generates a dependency-aware DAG of research steps using five tools: `search_symbols`, `get_symbol_context`, `get_impact_analysis`, `execute_cypher`, and `read_file_content`. Steps execute in order with dependency tracking; if a step fails, dependents are gracefully skipped.

**Deep Research Mode** (Ctrl+Shift+D) -- Forces comprehensive multi-step analysis regardless of query complexity. Inspired by deep research agents, it maximizes coverage across the knowledge graph.

**IDE-Style Filtering** -- Scope your questions to specific parts of the codebase using three quick-pick modals inspired by VS Code and JetBrains:

- **File Picker** (Ctrl+P) -- Fuzzy-search files with language indicators and symbol counts
- **Symbol Picker** (Ctrl+Shift+O) -- Search symbols with kind-specific icons (function, class, interface, etc.)
- **Module Picker** -- Select community clusters by name and member count

Active filters appear as removable pills in the context bar above the chat input.

**Source References** -- Every answer includes expandable source cards with code snippets, relationship context (callers, callees, community), relevance scores, and one-click navigation to the graph node.

**Research Plan Viewer** -- Watch your complex queries execute step by step with a live progress bar, tool icons, status indicators, and expandable results per step.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+K | Open search modal |
| Ctrl+B | Toggle sidebar |
| Ctrl+1-5 | Switch sidebar tab (Repos, Files, Graph, Impact, Docs) |
| Ctrl+\\ | Close detail panel |
| Ctrl+Shift+D | Toggle deep research mode |
| F | Fit graph to screen |
| L | Cycle graph layouts |
| 1 / 2 / 3 | Switch zoom level (package / module / symbol) |
| Escape | Close modals, deselect nodes |

### Tech Stack

The desktop app is built with:

- **Backend**: Tauri v2 + Rust, direct access to KnowledgeGraph and indexes (no MCP envelope overhead)
- **Frontend**: React 19, TypeScript, Tailwind CSS 4, Vite 8
- **State**: Zustand 5 + TanStack React Query 5
- **Graph**: Cytoscape.js with react-cytoscapejs
- **Syntax**: Shiki for code highlighting
- **Markdown**: react-markdown with remark-gfm and rehype-raw

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

Workspace crates with a layered dependency flow:

```
gitnexus-cli              CLI binary ("gitnexus")
  |
  +-- gitnexus-mcp          MCP server (7 tools, stdio/HTTP, JSON-RPC 2.0)
  +-- gitnexus-search        Hybrid search (BM25 + semantic + RRF)
  +-- gitnexus-db            Database adapter (in-memory or KuzuDB)
  +-- gitnexus-ingest        6-phase ingestion pipeline (parallel via rayon)
  |     +-- gitnexus-lang    13 language providers (tree-sitter)
  +-- gitnexus-core          Core types: KnowledgeGraph, NodeLabel, SymbolTable

gitnexus-desktop           Tauri v2 desktop app
  +-- gitnexus-db            Direct graph/index/FTS access
  +-- gitnexus-search        Hybrid search
  +-- gitnexus-core          Core types
```

### Ingestion Pipeline

The pipeline runs 6 sequential phases, with parallel file processing within each phase:

1. **Structure** -- Filesystem walk, create File/Folder nodes
2. **Parsing** -- Tree-sitter AST extraction, symbol creation
3. **Imports** -- Import statement extraction and resolution
4. **Calls** -- Function/method call extraction and linking
5. **Heritage** -- Class inheritance and interface implementation
6. **Community** -- Community detection and clustering

For ASP.NET MVC projects, 14 additional enrichment passes run automatically:
- Controllers, actions, areas, route extraction
- EDMX entity/association parsing with inheritance
- Razor view analysis (@model, @layout, partials, forms)
- Telerik/Kendo grid detection with DataSource bindings and columns
- jQuery/AJAX → controller action mapping
- Service/repository layer with DI detection (Autofac, Unity, Ninject)
- Custom attribute/filter detection
- External service call tracing (WebAPI, WCF)
- StackLogger tracing coverage analysis
- Base controller inheritance tracking

### Desktop IPC Commands

The desktop app communicates with the Rust backend via Tauri IPC:

| Command | Description |
|---------|-------------|
| `list_repos` | List available repositories |
| `open_repo` | Open and load a repository |
| `get_graph_data` | Get full graph for visualization |
| `get_subgraph` | Get a filtered subgraph |
| `get_neighbors` | Get neighbors of a node |
| `search_symbols` | Full-text symbol search |
| `search_autocomplete` | Fast prefix search |
| `get_symbol_context` | 360-degree symbol context |
| `get_impact_analysis` | Blast radius analysis |
| `get_file_tree` | File tree for explorer |
| `read_file_content` | Read file source code |
| `execute_cypher` | Run Cypher queries |
| `chat_ask` | Simple chat Q&A |
| `chat_analyze_query` | Analyze query complexity |
| `chat_plan_research` | Generate research plan DAG |
| `chat_execute_step` | Execute a single research step |
| `chat_execute_plan` | Execute full research plan with synthesis |
| `chat_pick_files` | Quick-pick file search |
| `chat_pick_symbols` | Quick-pick symbol search |
| `chat_pick_modules` | Quick-pick module search |

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
| C# | `.cs` `.cshtml` `.edmx` `.config` |
| Go | `.go` |
| Rust | `.rs` |
| Ruby | `.rb` |
| PHP | `.php` |
| Kotlin | `.kt` `.kts` |
| Swift | `.swift` |
| Razor | `.cshtml` `.razor` |

## License

PolyForm Noncommercial 1.0.0
