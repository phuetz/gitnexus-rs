//! JSON Schema definitions for the 13 MCP tools.
//!
//! Each tool has a name, description, and inputSchema following the MCP spec.

use serde_json::{json, Value};

/// A single MCP tool definition.
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

/// Return definitions for all 13 MCP tools.
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_repos",
            description: "List all indexed repositories with their stats and last-indexed timestamps.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "query",
            description: "Natural language search across the knowledge graph. Returns matching code symbols with file paths, scores, and context.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query (e.g., 'authentication middleware', 'database connection pool')"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path. Optional if only one repo is indexed."
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum results to return (default: 10)",
                        "default": 10
                    },
                    "goal": {
                        "type": "string",
                        "description": "Additional context about what you're trying to accomplish"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "context",
            description: "Get 360-degree context for a code symbol: callers, callees, imports, exports, class hierarchy, and related community.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Symbol name to look up (function, class, method, etc.)"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    },
                    "uid": {
                        "type": "string",
                        "description": "Exact node ID (e.g., 'Function:src/auth.ts:validateToken')"
                    },
                    "file": {
                        "type": "string",
                        "description": "Filter to symbols in this file path"
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "impact",
            description: "Blast radius analysis: find everything affected by changing a symbol. Shows upstream callers, downstream callees, and transitive impact.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Symbol name or node ID to analyze"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["upstream", "downstream", "both"],
                        "description": "Analysis direction (default: both)",
                        "default": "both"
                    },
                    "max_depth": {
                        "type": "number",
                        "description": "Maximum traversal depth (default: 5)",
                        "default": 5
                    }
                },
                "required": ["target"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "detect_changes",
            description: "Detect uncommitted changes in the repository and analyze their impact on the knowledge graph.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "rename",
            description: "Analyze the impact of renaming a symbol: find all references that would need updating.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Current symbol name or node ID"
                    },
                    "new_name": {
                        "type": "string",
                        "description": "Proposed new name"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    }
                },
                "required": ["target", "new_name"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "cypher",
            description: "Execute a raw Cypher query against the knowledge graph. Read-only queries only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Cypher query to execute (must be read-only, no CREATE/DELETE/SET/MERGE)"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "hotspots",
            description: "Identify file-level hotspots: files with high churn (lines added/removed) and frequent commits. These are often sources of bugs and maintenance burden.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    },
                    "since_days": {
                        "type": "number",
                        "description": "Analyze commits from the last N days (default: 90)",
                        "default": 90
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum results to return (default: 20)",
                        "default": 20
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "coupling",
            description: "Analyze temporal coupling between files: find file pairs that frequently change together in commits, suggesting hidden dependencies.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    },
                    "min_shared": {
                        "type": "number",
                        "description": "Minimum shared commits to report a coupling (default: 3)",
                        "default": 3
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum results to return (default: 20)",
                        "default": 20
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "ownership",
            description: "Analyze code ownership by author: shows primary author, ownership percentage, and contributor distribution for each file.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum results to return (default: 20)",
                        "default": 20
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "coverage",
            description: "Analyze tracing coverage and dead code: shows which methods have tracing instrumentation and which have zero incoming calls (dead code candidates).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Optional class/service name to analyze. If omitted, returns global coverage stats."
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "diagram",
            description: "Generate a Mermaid diagram for a code symbol: flowchart (call graph), sequence (interaction), or class (hierarchy).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Symbol name to generate diagram for (class, controller, service, etc.)"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["flowchart", "sequence", "class"],
                        "description": "Diagram type (default: flowchart)",
                        "default": "flowchart"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    }
                },
                "required": ["target"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "report",
            description: "Generate a code health report combining graph stats, hotspots, coupling, and ownership into a grade (A-E).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository name or path"
                    }
                },
                "additionalProperties": false
            }),
        },
    ]
}

/// Convert tool definitions to the JSON format expected by MCP tools/list.
pub fn tools_list_json() -> Value {
    let tools: Vec<Value> = tool_definitions()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema
            })
        })
        .collect();

    json!({ "tools": tools })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_count() {
        assert_eq!(tool_definitions().len(), 13);
    }

    #[test]
    fn test_tool_names() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        assert!(names.contains(&"list_repos"));
        assert!(names.contains(&"query"));
        assert!(names.contains(&"context"));
        assert!(names.contains(&"impact"));
        assert!(names.contains(&"detect_changes"));
        assert!(names.contains(&"rename"));
        assert!(names.contains(&"cypher"));
        assert!(names.contains(&"hotspots"));
        assert!(names.contains(&"coupling"));
        assert!(names.contains(&"ownership"));
        assert!(names.contains(&"coverage"));
        assert!(names.contains(&"diagram"));
        assert!(names.contains(&"report"));
    }

    #[test]
    fn test_tools_list_json() {
        let json = tools_list_json();
        let tools = json["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 13);
        for tool in tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }

    #[test]
    fn test_query_tool_has_required_field() {
        let tools = tool_definitions();
        let query_tool = tools.iter().find(|t| t.name == "query").unwrap();
        let required = query_tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("query")));
    }
}
