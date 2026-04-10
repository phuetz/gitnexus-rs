# Gemini Context: GitNexus

## Project Overview
GitNexus is a Rust-based code intelligence system that builds a knowledge graph from source code and exposes it via MCP (Model Context Protocol) for AI-powered code analysis. It supports 14 programming languages via tree-sitter parsing, with deep ASP.NET MVC 5 / Entity Framework 6 enrichment. The project includes a CLI, an HTML documentation generator, and a Tauri v2 desktop application.

## Workspace Architecture
The project is structured as a Cargo workspace with 11 active crates in the `crates/` directory:
- `gitnexus-cli`: The main binary.
- `gitnexus-core`: In-memory knowledge graph and core types.
- `gitnexus-db`: Database adapter (InMemory or optional KuzuDB).
- `gitnexus-desktop`: Tauri v2 desktop application (React 19, Tailwind CSS v4, Sigma.js).
- `gitnexus-git`: Git history analysis.
- `gitnexus-ingest`: 8-phase ingestion pipeline orchestrator.
- `gitnexus-lang`: Language providers (14 languages).
- `gitnexus-mcp`: MCP server implementation (13 tools).
- `gitnexus-output`: Output formatting.
- `gitnexus-query`: Query execution.
- `gitnexus-search`: Hybrid search (BM25 + ONNX semantic embeddings).

## Building and Running

### Rust / Backend
- **Build (debug):** `cargo build`
- **Build (release):** `cargo build --release`
- **Build CLI (release):** `cargo build --release -p gitnexus-cli`
- **Run the CLI:** `cargo run -p gitnexus-cli -- <command>`
- **Run tests:** `cargo test --workspace`
- **Linting:** `cargo clippy --workspace`

### Optional Features
- **With KuzuDB backend:** `cargo build --features gitnexus-cli/kuzu-backend`
- **With Semantic Search:** `cargo build --features gitnexus-search/embeddings`

### Desktop Application (Tauri + React)
- **Install dependencies:** `cd crates/gitnexus-desktop/ui && npm install`
- **Run dev server (Frontend):** `cd crates/gitnexus-desktop/ui && npm run dev`
- **Run dev app (Tauri):** `cd crates/gitnexus-desktop && cargo tauri dev`
- **Build Desktop App:** `build-release.bat desktop` (Windows) or `./build-release.sh desktop` (Linux/macOS)

## Development Conventions & Gotchas
- **MCP Stdout:** MCP mode logs to `stderr` to avoid polluting the `stdout` JSON-RPC stream. **Never add `println!` in MCP code paths.**
- **Snapshot Format:** Despite the filename `graph.bin`, snapshots are saved as JSON, not bincode.
- **cxx-build Pin (Windows):** `cxx-build` is pinned to `=1.0.138` to match kuzu's `cxx` version and prevent linker errors (`LNK2019`). Do not bump without verifying kuzu compatibility.
- **Node IDs:** Deterministic format `"${Label}:${qualifiedName}"` (e.g., `"Function:src/main.ts:handleLogin"`).
- **Rust Toolchain:** MSRV is 1.75, Edition 2021. Release profile uses thin LTO, single codegen unit, and stripped binaries.
