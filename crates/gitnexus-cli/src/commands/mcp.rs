//! The `mcp` command: starts the MCP server on stdio transport.

use gitnexus_mcp::backend::local::LocalBackend;
use gitnexus_mcp::server::start_mcp_server;

pub async fn run() -> anyhow::Result<()> {
    let backend = LocalBackend::new();
    start_mcp_server(backend)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
