---
name: gitnexus
description: Query and analyze codebases using the GitNexus knowledge graph. Use for finding symbols, analyzing impact, understanding architecture, exploring code relationships, generating documentation, and asking questions about code.
argument-hint: "[command] [arguments]"
allowed-tools: Bash(gitnexus *), Bash(*/gitnexus *), Bash(*/gitnexus.exe *), Bash(cargo run -p gitnexus-cli -- *), Read, Grep, Glob
---

# GitNexus — Knowledge Graph Code Intelligence

You have access to GitNexus, a code intelligence tool that builds a knowledge graph from source code. Use it to answer questions about codebases, find symbols, trace dependencies, and analyze impact.

## Binary location

Use whichever is available:
- Release: `target/release/gitnexus.exe` (Windows) or `target/release/gitnexus`
- Dev: `cargo run -p gitnexus-cli --`

## Available commands

### 1. Analyze a codebase (must run first)

```bash
gitnexus analyze [path]              # Index a repository
gitnexus analyze [path] --force      # Force re-index
gitnexus analyze [path] --incremental # Only re-parse changed files
gitnexus status                       # Check if index exists
```

### 2. Search the knowledge graph

```bash
gitnexus query "authentication middleware"    # Natural language search
gitnexus query "user service" --limit 5       # Limit results
```

### 3. Symbol context (360-degree view)

```bash
gitnexus context UserService          # Callers, callees, imports, exports, hierarchy
gitnexus context handleRequest --repo my-project
```

### 4. Impact analysis (blast radius)

```bash
gitnexus impact handleRequest --direction both       # Upstream + downstream
gitnexus impact UserService --direction upstream      # Who calls this?
gitnexus impact UserService --direction downstream    # What does this call?
```

### 5. Ask questions (LLM-powered, requires ~/.gitnexus/chat-config.json)

```bash
gitnexus ask "comment fonctionne le calcul des baremes ?" --path D:\taf\Alise_v2
gitnexus ask "which controllers call the external API?"
```

### 6. Code health report

```bash
gitnexus report --path D:\taf\Alise_v2        # Text report with grade A-E
gitnexus report --path D:\taf\Alise_v2 --json # JSON output
```

### 7. Git analytics

```bash
gitnexus hotspots --path [path]       # Most changed files (last 90 days)
gitnexus coupling --path [path]       # Files that change together
gitnexus ownership --path [path]      # Code ownership by author
```

### 8. Tracing coverage & dead code

```bash
gitnexus coverage                             # Global tracing + dead code stats
gitnexus coverage UserService                 # Single class coverage
gitnexus coverage --json                      # JSON output
gitnexus coverage UserService --trace         # Flow trace mode
```

### 9. Raw Cypher queries

```bash
gitnexus cypher "MATCH (n:Function) RETURN n.name LIMIT 10"
gitnexus cypher "MATCH (n:Controller)-[:DEFINES]->(a:ControllerAction) RETURN n.name, a.name"
gitnexus cypher "MATCH (n:Method) WHERE n.name STARTS WITH 'Get' RETURN DISTINCT n.name"
gitnexus cypher "MATCH (n:Function) WHERE n.name CONTAINS 'auth' OR n.name CONTAINS 'login' RETURN n"
gitnexus cypher "MATCH (n:Method) WHERE NOT n.filePath ENDS WITH '.test.cs' RETURN n.name"
gitnexus cypher "MATCH (n:Method) WHERE n.name <> 'Dispose' RETURN n.name LIMIT 20"
```

Supported Cypher operators:
- WHERE: `=`, `<>`, `!=`, `CONTAINS`, `STARTS WITH`, `ENDS WITH`
- Logic: `AND`, `OR`, `NOT` (precedence: NOT > AND > OR)
- RETURN: `DISTINCT`, `count()`
- Clauses: `ORDER BY [ASC|DESC]`, `LIMIT`

### 10. Interactive shell

```bash
gitnexus shell                                # REPL with tab completion
# Inside the shell:
#   query auth           — search symbols
#   context UserService  — 360° view
#   impact handleLogin   — blast radius
#   cypher MATCH ...     — Cypher queries (full operator support)
#   hotspots             — git churn analysis
#   stats                — graph statistics
#   help                 — all commands
```

### 11. MCP server (for AI agents)

```bash
gitnexus mcp                                       # Start MCP server (stdio, JSON-RPC 2.0)
# 13 tools: list_repos, query, context, impact, detect_changes, rename,
#           cypher, hotspots, coupling, ownership, coverage, diagram, report
# 5 prompts: detect_impact, generate_map, analyze_hotspots, find_dead_code, trace_dependencies
```

### 12. HTTP server (REST API)

```bash
gitnexus serve --port 3000                         # Start HTTP server
# Endpoints:
#   POST /mcp                        — JSON-RPC 2.0 MCP bridge
#   GET  /health                     — Liveness check
#   GET  /api/repos                  — List repositories
#   GET  /api/repos/:name/search?q=  — Search symbols
#   GET  /api/repos/:name/stats      — Repository statistics
#   GET  /api/repos/:name/hotspots   — File hotspots
```

### 13. Validate LLM config

```bash
gitnexus config test                              # Check API key + test connection
```

### 14. List indexed repos

```bash
gitnexus list                                      # Show all indexed repositories
```

### 15. Generate documentation

```bash
gitnexus generate html --path [path]                    # HTML site (DeepWiki-style)
gitnexus generate html --path [path] --enrich           # With LLM enrichment
gitnexus generate docs --path [path]                    # Markdown pages
gitnexus generate all --path [path]                     # All formats
```

### 16. Trace files (all sources for a feature)

```bash
gitnexus trace-files CourrierController                  # List all related source files
gitnexus trace-files BenefService --depth 3              # Limit traversal depth
gitnexus trace-files CourrierController --json           # JSON output
```

### 17. Generate diagrams

```bash
gitnexus diagram CourrierController --type flowchart     # Call flow organigramme
gitnexus diagram CourrierController --type sequence      # UML sequence diagram
gitnexus diagram BeneficiaireController --type class     # Class diagram with methods
gitnexus diagram CourrierController --output flow.md     # Write to file
```

### 18. Import execution traces

```bash
gitnexus trace-import D:\logs\production.log             # Enrich graph with runtime data
gitnexus trace-import trace.csv --path D:\taf\MyProject  # Specify repo path
```

## How to use this skill

When the user asks about code structure, architecture, dependencies, or impact:

1. **Check if the repo is indexed**: run `gitnexus status` in the relevant directory
2. **If not indexed**: run `gitnexus analyze [path]` first
3. **Choose the right command** based on the question:
   - "Where is X defined?" → `gitnexus query "X"` or `gitnexus context X`
   - "What calls X?" → `gitnexus impact X --direction upstream`
   - "What does X depend on?" → `gitnexus impact X --direction downstream`
   - "How healthy is the code?" → `gitnexus report`
   - "Which files change most?" → `gitnexus hotspots`
   - "Is this code used?" → `gitnexus coverage` or `gitnexus coverage ClassName`
   - "Explain how X works" → `gitnexus ask "how does X work?"`
   - "Show me the architecture" → `gitnexus generate html` then read the output
   - "Which files are involved in X?" → `gitnexus trace-files X`
   - "Generate a diagram of X" → `gitnexus diagram X --type flowchart`
   - "Show the sequence for X" → `gitnexus diagram X --type sequence`
   - "Import logs to enrich" → `gitnexus trace-import logfile.log`
4. **Parse the output** and present it clearly with file paths and relationships
5. **Combine multiple commands** if needed for complex questions

## Graph node types

The knowledge graph contains these node types (50+):
- **Code**: Function, Method, Class, Interface, Struct, Enum, Module, Namespace
- **ASP.NET**: Controller, ControllerAction, Service, Repository, ExternalService
- **UI**: UiComponent (Telerik/Kendo grids), ScriptFile, AjaxCall
- **Data**: Entity (EF6), Association, NavigationProperty
- **Infrastructure**: File, Directory, Import, Export

## Relationship types

Key relationships in the graph:
- `Calls`, `CallsAction`, `CallsService` — invocation chains
- `Imports`, `Exports` — module dependencies
- `Inherits`, `Implements` — type hierarchy
- `DependsOn`, `RendersComponent`, `IncludesScript` — UI dependencies
- `HasAssociation`, `HasNavigationProperty` — data model links

## Code quality metrics (computed during analysis)

- **Cyclomatic complexity (CC)**: Counted per Method/Function/Constructor via tree-sitter AST (if, for, while, switch case, &&, ||, catch, ternary). Shown in health reports and desktop hover cards.
- **Dead code detection**: Methods with 0 incoming Calls edges (excludes constructors, tests, entry points, interface methods, JS in views)
- **Circular dependencies**: DFS on file-level imports/DependsOn edges
- **Layer violations**: Detects Presentation → Data bypassing Business layer (ASP.NET)
- **Tracing coverage**: StackLogger.BeginMethodScope() detection

## Desktop app (Tauri v2 + React 19)

```bash
# Launch (two terminals):
cd crates/gitnexus-desktop/ui && npm run dev    # Frontend (Vite)
cd crates/gitnexus-desktop && cargo tauri dev    # Backend (Tauri)
```

Features: Sigma.js WebGL graph (ForceAtlas2), fuzzy search, navigation history (Alt+←/→), breadcrumbs, impact overlay, code snippet preview, Cypher panel, coverage/diagram/report views, PNG export (Ctrl+E), dark/light theme.

## Tips

- The `.gitnexus/` directory in the repo root contains the serialized graph (`graph.bin`)
- Supports 14 languages: JS, TS, Python, Java, C, C++, C#, Go, Rust, Ruby, PHP, Kotlin, Swift, Razor
- Skips `obj/`, `bin/`, `node_modules/`, `packages/` directories during analysis
- For ASP.NET MVC projects: controllers, views, EF6 entities, Telerik grids, jQuery AJAX are all in the graph
- Cypher supports: MATCH, WHERE (=, <>, !=, CONTAINS, STARTS WITH, ENDS WITH, AND, OR, NOT), RETURN DISTINCT, count(), ORDER BY, LIMIT
- All `--json` flags output machine-readable JSON
- Graph nodes sorted by importance (connectivity + entry point score + exported/traced status)
