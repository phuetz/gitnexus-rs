//! MCP resource definitions.
//!
//! Resources expose graph data through URI templates that MCP clients
//! can browse and read.

use serde_json::{json, Value};

/// Return the list of MCP resource templates and static resources.
///
/// Resource templates (6):
/// - gitnexus://repos - List all indexed repositories
/// - gitnexus://repos/{repo}/schema - Graph schema for a repo
/// - gitnexus://repos/{repo}/stats - Statistics for a repo
/// - gitnexus://repos/{repo}/communities - Community summary
/// - gitnexus://repos/{repo}/processes - Process flows
/// - gitnexus://repos/{repo}/files/{path} - File node details
///
/// Static resources (2):
/// - gitnexus://version - GitNexus version info
/// - gitnexus://help - Help and usage information
pub fn resource_definitions() -> Value {
    json!({
        "resources": [
            {
                "uri": "gitnexus://version",
                "name": "GitNexus Version",
                "description": "Current GitNexus version and build information",
                "mimeType": "application/json"
            },
            {
                "uri": "gitnexus://help",
                "name": "GitNexus Help",
                "description": "Usage guide and available tools",
                "mimeType": "text/plain"
            }
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "gitnexus://repos",
                "name": "Repository List",
                "description": "List all indexed repositories with stats"
            },
            {
                "uriTemplate": "gitnexus://repos/{repo}/schema",
                "name": "Graph Schema",
                "description": "Node and relationship types in the knowledge graph for a repository"
            },
            {
                "uriTemplate": "gitnexus://repos/{repo}/stats",
                "name": "Repository Stats",
                "description": "Statistics about the indexed knowledge graph"
            },
            {
                "uriTemplate": "gitnexus://repos/{repo}/communities",
                "name": "Community Summary",
                "description": "Detected code communities and their descriptions"
            },
            {
                "uriTemplate": "gitnexus://repos/{repo}/processes",
                "name": "Process Flows",
                "description": "Detected execution flows and their step sequences"
            },
            {
                "uriTemplate": "gitnexus://repos/{repo}/files/{path}",
                "name": "File Details",
                "description": "Detailed information about a specific file node"
            }
        ]
    })
}

/// Read a resource by URI and return its contents.
pub fn read_resource(uri: &str) -> Option<Value> {
    match uri {
        "gitnexus://version" => Some(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&json!({
                    "name": "gitnexus",
                    "version": env!("CARGO_PKG_VERSION"),
                    "runtime": "rust",
                    "description": "Graph-powered code intelligence for AI agents"
                })).unwrap_or_default()
            }]
        })),
        "gitnexus://help" => Some(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/plain",
                "text": HELP_TEXT
            }]
        })),
        _ => None,
    }
}

const HELP_TEXT: &str = r#"GitNexus - Graph-powered code intelligence for AI agents

Available MCP Tools:
  list_repos     - List all indexed repositories
  query          - Natural language search across the knowledge graph
  context        - 360-degree view of a code symbol (callers, callees, hierarchy)
  impact         - Blast radius analysis for a code change
  detect_changes - Detect uncommitted changes and their graph impact
  rename         - Analyze impact of renaming a symbol
  cypher         - Execute raw Cypher queries (read-only)

Getting Started:
  1. Index a repository: gitnexus analyze /path/to/repo
  2. Start MCP server: gitnexus mcp
  3. Query from your editor or AI agent

Examples:
  query: { "query": "authentication middleware" }
  context: { "name": "handleLogin" }
  impact: { "target": "UserService", "direction": "both" }
  cypher: { "query": "MATCH (n:Function) RETURN n.name LIMIT 10" }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_definitions() {
        let defs = resource_definitions();
        let resources = defs["resources"].as_array().unwrap();
        let templates = defs["resourceTemplates"].as_array().unwrap();
        assert_eq!(resources.len(), 2);
        assert_eq!(templates.len(), 6);
    }

    #[test]
    fn test_read_version_resource() {
        let result = read_resource("gitnexus://version");
        assert!(result.is_some());
        let contents = result.unwrap();
        assert!(contents["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("gitnexus"));
    }

    #[test]
    fn test_read_help_resource() {
        let result = read_resource("gitnexus://help");
        assert!(result.is_some());
    }

    #[test]
    fn test_read_unknown_resource() {
        assert!(read_resource("gitnexus://unknown").is_none());
    }
}
