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

### 8. Raw Cypher queries

```bash
gitnexus cypher "MATCH (n:Function) RETURN n.name LIMIT 10"
gitnexus cypher "MATCH (n:Controller)-[:DEFINES]->(a:ControllerAction) RETURN n.name, a.name"
```

### 9. Validate LLM config

```bash
gitnexus config test                              # Check API key + test connection
```

### 10. List indexed repos

```bash
gitnexus list                                      # Show all indexed repositories
```

### 11. Generate documentation

```bash
gitnexus generate html --path [path]                    # HTML site (DeepWiki-style)
gitnexus generate html --path [path] --enrich           # With LLM enrichment
gitnexus generate docs --path [path]                    # Markdown pages
gitnexus generate all --path [path]                     # All formats
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
   - "Explain how X works" → `gitnexus ask "how does X work?"`
   - "Show me the architecture" → `gitnexus generate html` then read the output
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

## Tips

- The `.gitnexus/` directory in the repo root contains the serialized graph (`graph.bin`)
- Supports 14 languages: JS, TS, Python, Java, C, C++, C#, Go, Rust, Ruby, PHP, Kotlin, Swift, Razor
- For ASP.NET MVC projects: controllers, views, EF6 entities, Telerik grids, jQuery AJAX are all in the graph
- Cypher queries use a simplified subset (MATCH/WHERE/RETURN/LIMIT)
- All `--json` flags output machine-readable JSON
