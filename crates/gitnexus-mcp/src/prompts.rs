//! MCP prompt templates.
//!
//! Provides structured prompts that MCP clients can use to guide
//! their interactions with the knowledge graph.

use serde_json::{json, Value};

/// Return the list of available MCP prompts.
pub fn prompt_definitions() -> Value {
    json!({
        "prompts": [
            {
                "name": "detect_impact",
                "description": "Analyze the blast radius of a proposed code change",
                "arguments": [
                    {
                        "name": "target",
                        "description": "The symbol name or file path being changed",
                        "required": true
                    },
                    {
                        "name": "change_description",
                        "description": "Description of the proposed change",
                        "required": false
                    }
                ]
            },
            {
                "name": "generate_map",
                "description": "Generate a high-level map of the codebase architecture",
                "arguments": [
                    {
                        "name": "repo",
                        "description": "Repository name (optional if only one is indexed)",
                        "required": false
                    },
                    {
                        "name": "focus",
                        "description": "Optional area to focus on (e.g., 'authentication', 'database')",
                        "required": false
                    }
                ]
            }
        ]
    })
}

/// Get a prompt by name with the given arguments.
pub fn get_prompt(name: &str, args: &Value) -> Option<Value> {
    match name {
        "detect_impact" => {
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let change_desc = args
                .get("change_description")
                .and_then(|v| v.as_str())
                .unwrap_or("unspecified change");

            Some(json!({
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "I want to analyze the impact of changing `{target}`. \
                                 The proposed change is: {change_desc}\n\n\
                                 Please:\n\
                                 1. Use the `impact` tool to find upstream and downstream dependencies\n\
                                 2. Use the `context` tool to understand the symbol's role\n\
                                 3. Summarize which files and functions would be affected\n\
                                 4. Highlight any high-risk areas (heavily depended-upon code)\n\
                                 5. Suggest a safe order for making the changes"
                            )
                        }
                    }
                ]
            }))
        }
        "generate_map" => {
            let repo = args
                .get("repo")
                .and_then(|v| v.as_str())
                .map(|r| format!(" for repository '{r}'"))
                .unwrap_or_default();
            let focus = args
                .get("focus")
                .and_then(|v| v.as_str())
                .map(|f| format!(" Focus on the '{f}' area."))
                .unwrap_or_default();

            Some(json!({
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "Generate a high-level architecture map{repo}.{focus}\n\n\
                                 Please:\n\
                                 1. Use `list_repos` to see available repositories\n\
                                 2. Use `cypher` to query communities: \
                                    MATCH (c:Community) RETURN c.name, c.description, c.symbolCount ORDER BY c.symbolCount DESC\n\
                                 3. Use `cypher` to query key processes: \
                                    MATCH (p:Process) RETURN p.name, p.description, p.stepCount ORDER BY p.stepCount DESC\n\
                                 4. Summarize the codebase structure as a hierarchical outline\n\
                                 5. Identify the main entry points and data flows"
                            )
                        }
                    }
                ]
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_definitions() {
        let defs = prompt_definitions();
        let prompts = defs["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0]["name"], "detect_impact");
        assert_eq!(prompts[1]["name"], "generate_map");
    }

    #[test]
    fn test_get_detect_impact_prompt() {
        let result = get_prompt(
            "detect_impact",
            &json!({"target": "UserService", "change_description": "add caching"}),
        );
        assert!(result.is_some());
        let messages = result.unwrap();
        let text = messages["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("UserService"));
        assert!(text.contains("add caching"));
    }

    #[test]
    fn test_get_generate_map_prompt() {
        let result = get_prompt("generate_map", &json!({"focus": "auth"}));
        assert!(result.is_some());
        let messages = result.unwrap();
        let text = messages["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("auth"));
    }

    #[test]
    fn test_unknown_prompt() {
        assert!(get_prompt("nonexistent", &json!({})).is_none());
    }
}
