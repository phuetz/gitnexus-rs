mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gitnexus",
    about = "Graph-powered code intelligence for AI agents",
    version,
    long_about = "GitNexus builds a knowledge graph from your codebase and exposes it \
                  via MCP (Model Context Protocol) for AI-powered code analysis."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a repository into a knowledge graph
    Analyze {
        /// Path to the repository (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,
        /// Force re-index even if up-to-date
        #[arg(long)]
        force: bool,
        /// Generate embeddings for semantic search
        #[arg(long)]
        embeddings: bool,
        /// Enable verbose output
        #[arg(long)]
        verbose: bool,
        /// Skip git operations (index without git context)
        #[arg(long)]
        skip_git: bool,
    },
    /// Start MCP server (stdio transport)
    Mcp,
    /// Start HTTP server for web UI
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// List indexed repositories
    List,
    /// Show index status for the current directory
    Status,
    /// Delete GitNexus index
    Clean {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
        /// Clean all indexed repositories
        #[arg(long)]
        all: bool,
    },
    /// Search the knowledge graph
    Query {
        /// Natural language search query
        query: String,
        /// Repository name or path
        #[arg(short, long)]
        repo: Option<String>,
        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// 360-degree symbol view
    Context {
        /// Symbol name to look up
        name: String,
        /// Repository name or path
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Blast radius analysis
    Impact {
        /// Symbol name or node ID
        target: String,
        /// Repository name or path
        #[arg(short, long)]
        repo: Option<String>,
        /// Analysis direction: upstream, downstream, or both
        #[arg(short, long, default_value = "both")]
        direction: String,
    },
    /// Execute a raw Cypher query
    Cypher {
        /// Cypher query string
        query: String,
        /// Repository name or path
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Configure MCP for supported editors (VS Code, Cursor, etc.)
    Setup,
    /// Launch interactive REPL shell for exploring the knowledge graph
    Shell {
        /// Path to the repository (defaults to current directory)
        path: Option<String>,
    },
    /// Generate documentation from the knowledge graph
    Generate {
        /// Target: context, agents, wiki, skills, docs, docx, html, all
        #[arg(help = "Target: context | agents | wiki | skills | docs | docx | html | all")]
        what: String,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Watch a repository for changes and incrementally update the knowledge graph
    Watch {
        /// Path to the repository (defaults to current directory)
        path: Option<String>,
    },
    /// Launch interactive TUI dashboard for exploring the knowledge graph
    Dashboard {
        /// Path to the repository (defaults to current directory)
        path: Option<String>,
    },
    /// Show file-level hotspots (most changed files)
    Hotspots {
        /// Only consider commits from the last N days
        #[arg(long, default_value = "90")]
        since: u32,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show temporally coupled file pairs
    Coupling {
        /// Minimum number of shared commits
        #[arg(long, default_value = "3")]
        min: u32,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show file ownership by author
    Ownership {
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = match &cli.command {
        Commands::Analyze { verbose, .. } if *verbose => tracing::Level::DEBUG,
        Commands::Mcp => tracing::Level::WARN, // Quiet for stdio MCP
        _ => tracing::Level::INFO,
    };

    // For MCP mode, log to stderr to avoid polluting stdout JSON-RPC
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    match cli.command {
        Commands::Analyze {
            path,
            force,
            embeddings,
            verbose,
            skip_git,
        } => {
            commands::analyze::run(&path, force, embeddings, verbose, skip_git).await
        }
        Commands::Mcp => commands::mcp::run().await,
        Commands::Serve { port } => commands::serve::run(port).await,
        Commands::List => commands::list::run(),
        Commands::Status => commands::status::run(),
        Commands::Clean { force, all } => commands::clean::run(force, all),
        Commands::Query { query, repo, limit } => {
            commands::query_cmd::run(&query, repo.as_deref(), limit).await
        }
        Commands::Context { name, repo } => {
            commands::context_cmd::run(&name, repo.as_deref()).await
        }
        Commands::Impact {
            target,
            repo,
            direction,
        } => commands::impact_cmd::run(&target, repo.as_deref(), &direction).await,
        Commands::Cypher { query, repo } => {
            commands::cypher_cmd::run(&query, repo.as_deref()).await
        }
        Commands::Setup => commands::setup::run(),
        Commands::Shell { path } => commands::shell::run(path.as_deref()).await,
        Commands::Generate { what, path } => commands::generate::run(&what, path.as_deref()),
        Commands::Watch { path } => commands::watch::run(path.as_deref()).await,
        Commands::Dashboard { path } => commands::dashboard::run(path.as_deref()),
        Commands::Hotspots { since, path, json } => {
            commands::hotspots::run(since, path.as_deref(), json)
        }
        Commands::Coupling { min, path, json } => {
            commands::coupling_cmd::run(min, path.as_deref(), json)
        }
        Commands::Ownership { path, json } => {
            commands::ownership_cmd::run(path.as_deref(), json)
        }
    }
}
