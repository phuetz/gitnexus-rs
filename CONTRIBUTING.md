# Contributing to GitNexus

Thank you for your interest in GitNexus! This document provides guidelines for contributing.

## License

GitNexus is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE). By contributing, you agree that your contributions will be licensed under the same terms.

## Development Setup

### Prerequisites

- **Rust** 1.75+ (`rustup` recommended)
- **Node.js** 20+ (for the desktop app frontend)
- **git** (for git analytics features)

### Build

```bash
# Clone the repo
git clone https://github.com/phuetz/gitnexus-rs.git
cd gitnexus-rs

# Build the CLI
cargo build -p gitnexus-cli

# Build everything
cargo build --workspace

# Build the desktop app frontend
cd crates/gitnexus-desktop/ui && npm install && npm run build && cd ../../..
```

### Test

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p gitnexus-ingest

# Run CLI integration tests
cargo test -p gitnexus-cli --test cli_integration

# Clippy (lint)
cargo clippy --workspace

# TypeScript check
cd crates/gitnexus-desktop/ui && npx tsc --noEmit
```

## Project Structure

14 crates in `crates/`, organized in layers:

| Layer | Crate | Role |
|-------|-------|------|
| **Binary** | `gitnexus-cli` | CLI tool |
| **Desktop** | `gitnexus-desktop` | Tauri v2 + React 19 app |
| **Protocol** | `gitnexus-mcp` | MCP server (7 tools) |
| **Analytics** | `gitnexus-git` | Git history analysis |
| **Search** | `gitnexus-search` | BM25 + semantic search |
| **Storage** | `gitnexus-db` | Database adapter |
| **Pipeline** | `gitnexus-ingest` | 6-phase ingestion |
| **Language** | `gitnexus-lang` | 14 tree-sitter providers |
| **Core** | `gitnexus-core` | Types, graph, config |

See [CLAUDE.md](CLAUDE.md) for detailed architecture documentation.

## Code Style

- **Rust**: Follow `cargo clippy` recommendations. No `unwrap()` in production code.
- **TypeScript**: Follow `tsc --strict`. Use functional components with hooks.
- **Commits**: Use conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).
- **No dead code**: Remove unused functions, variables, and imports.

## Adding a New Language Provider

1. Create `crates/gitnexus-lang/src/languages/your_lang.rs`
2. Implement the `LanguageProvider` trait
3. Add tree-sitter queries in `queries/`
4. Add import resolver in `import_resolvers/`
5. Register in `registry.rs`
6. Add tests

## Pull Requests

1. Fork and create a feature branch
2. Write tests for new functionality
3. Ensure `cargo test --workspace` passes
4. Ensure `cargo clippy --workspace` has no warnings
5. Submit PR with a clear description
