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
    #[command(after_help = "Examples:\n  gitnexus analyze\n  gitnexus analyze D:\\taf\\MyProject\n  gitnexus analyze --force --verbose")]
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
        /// Use incremental indexing (only re-parse changed files)
        #[arg(long)]
        incremental: bool,
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
    #[command(after_help = "Examples:\n  gitnexus query \"authentication middleware\"\n  gitnexus query \"user service\" --limit 5")]
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
    #[command(after_help = "Examples:\n  gitnexus context UserService\n  gitnexus context handleRequest --repo my-project")]
    Context {
        /// Symbol name to look up
        name: String,
        /// Repository name or path
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Blast radius analysis
    #[command(after_help = "Examples:\n  gitnexus impact handleRequest --direction both\n  gitnexus impact UserService --direction upstream")]
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
    #[command(after_help = "Examples:\n  gitnexus generate html --path D:\\taf\\MyProject\n  gitnexus generate html --enrich --enrich-profile strict\n  gitnexus generate all --path D:\\taf\\MyProject")]
    Generate {
        /// Target: context, agents, wiki, skills, docs, docx, html, all
        #[arg(help = "Target: context | agents | wiki | skills | docs | docx | html | all")]
        what: String,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Enrich documentation with LLM-generated prose (requires configured LLM)
        #[arg(long, default_value_t = false)]
        enrich: bool,
        /// Enrichment profile: fast, quality, or strict
        #[arg(long, default_value = "quality")]
        enrich_profile: String,
        /// Documentation language: auto, fr, or en
        #[arg(long, default_value = "fr")]
        enrich_lang: String,
        /// Include source citations in enriched pages
        #[arg(long, default_value_t = true)]
        enrich_citations: bool,
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
    /// Ask a question about the codebase using the knowledge graph + LLM
    #[command(after_help = "Examples:\n  gitnexus ask \"how does authentication work?\"\n  gitnexus ask \"quels controllers appellent le WebAPI?\" --path D:\\taf\\MyProject")]
    Ask {
        /// The question to ask
        question: String,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Generate a combined code health report (hotspots + coupling + ownership + graph stats)
    #[command(after_help = "Examples:\n  gitnexus report\n  gitnexus report --path D:\\taf\\MyProject\n  gitnexus report --json")]
    Report {
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Test LLM configuration (validate API key and connectivity)
    #[command(after_help = "Examples:\n  gitnexus config test")]
    Config {
        /// Subcommand: test
        action: String,
    },
    /// List all source files involved in a feature (BFS traversal from a symbol)
    #[command(after_help = "Examples:\n  gitnexus trace-files CourrierController\n  gitnexus trace-files BenefService --depth 3 --json")]
    TraceFiles {
        /// Symbol name to trace from (controller, service, class, method)
        target: String,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Maximum traversal depth (default: 3)
        #[arg(long, default_value = "3")]
        depth: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate Mermaid diagrams from the knowledge graph
    #[command(after_help = "Examples:\n  gitnexus diagram CourrierController --type flowchart\n  gitnexus diagram BenefService --type sequence\n  gitnexus diagram CourrierController --type class --output diagram.md")]
    Diagram {
        /// Symbol name to diagram
        target: String,
        /// Diagram type: flowchart, sequence, or class
        #[arg(long, default_value = "flowchart")]
        r#type: String,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import execution traces (log files) to enrich the knowledge graph
    #[command(after_help = "Examples:\n  gitnexus trace-import D:\\logs\\production.log\n  gitnexus trace-import trace.csv --path D:\\taf\\MyProject")]
    TraceImport {
        /// Path to the log/trace file
        file: String,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Analyze tracing coverage and detect dead code
    #[command(after_help = "Examples:\n  gitnexus coverage CourriersService\n  gitnexus coverage --json\n  gitnexus coverage CourriersService --path D:\\taf\\Alise_v2")]
    Coverage {
        /// Class/Service name to analyze (omit for global report)
        target: Option<String>,
        /// Path to the repository (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Trace the full call flow and show coverage along the chain
        #[arg(long)]
        trace: bool,
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
            incremental,
        } => {
            commands::analyze::run(&path, force, embeddings, verbose, skip_git, incremental).await
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
        Commands::Generate { what, path, enrich, enrich_profile, enrich_lang, enrich_citations } => {
            commands::generate::run(&what, path.as_deref(), enrich, &enrich_profile, &enrich_lang, enrich_citations)
        }
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
        Commands::Ask { question, path } => {
            commands::ask::run(&question, path.as_deref())
        }
        Commands::Report { path, json } => {
            commands::report::run(path.as_deref(), json)
        }
        Commands::Config { action } => {
            match action.as_str() {
                "test" => tokio::task::spawn_blocking(commands::config_cmd::run_test)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?,
                _ => {
                    println!("Unknown config action: {}. Use 'gitnexus config test'.", action);
                    Ok(())
                }
            }
        }
        Commands::TraceFiles { target, path, depth, json } => {
            commands::trace_files::run(&target, path.as_deref(), depth, json)
        }
        Commands::Diagram { target, r#type, path, output } => {
            commands::diagram::run(&target, &r#type, path.as_deref(), output.as_deref())
        }
        Commands::TraceImport { file, path } => {
            commands::trace_import::run(&file, path.as_deref())
        }
        Commands::Coverage { target, path, json, trace } => {
            commands::coverage::run(target.as_deref(), path.as_deref(), json, trace)
        }
    }
}
