//! Next-step hints for MCP tool responses.
//!
//! Each tool returns a hint suggesting what the AI agent should do next,
//! helping guide the conversation flow.

/// Get the next-step hint for a tool.
pub fn hint_for(tool_name: &str) -> &'static str {
    match tool_name {
        "list_repos" => {
            "Now that you see the indexed repos, you can use `query` to search \
             for code patterns, or `context` to explore a specific symbol."
        }
        "query" => {
            "Use `context` on any result to see its full 360-degree view \
             (callers, callees, imports, class hierarchy). \
             Use `impact` to analyze the blast radius of changing a result."
        }
        "context" => {
            "To explore further: use `impact` to see the blast radius, \
             or `context` on any connected symbol. \
             Use `cypher` for custom graph queries."
        }
        "impact" => {
            "Review the affected files and symbols. \
             Use `context` on high-impact nodes to understand them better. \
             Use `rename` if you're planning a rename refactor."
        }
        "detect_changes" => {
            "Use `impact` on changed files to understand the blast radius. \
             Use `query` to find related tests that should be run."
        }
        "rename" => {
            "Review all references that need updating. \
             The `filesAffected` count shows how many files need changes. \
             Use `context` on the target to verify you have the right symbol."
        }
        "cypher" => {
            "You can refine your query or use the tool-specific commands \
             (query, context, impact) for structured analysis. \
             Common patterns: MATCH (n:Function) WHERE n.isExported = true RETURN n"
        }
        _ => "Use `list_repos` to see indexed repositories, or `query` to search.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_have_hints() {
        let tools = [
            "list_repos",
            "query",
            "context",
            "impact",
            "detect_changes",
            "rename",
            "cypher",
        ];
        for tool in &tools {
            let hint = hint_for(tool);
            assert!(!hint.is_empty(), "Hint for {tool} is empty");
        }
    }

    #[test]
    fn test_unknown_tool_has_fallback() {
        let hint = hint_for("nonexistent");
        assert!(!hint.is_empty());
    }
}
