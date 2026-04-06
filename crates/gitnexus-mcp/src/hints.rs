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
        "hotspots" => {
            "Review the top hotspots — files with high churn and frequent commits \
             are often sources of bugs. Use `coupling` to find files that change \
             together, or `context` to explore symbols in hotspot files."
        }
        "coupling" => {
            "File pairs with high coupling strength may have hidden dependencies. \
             Use `context` or `impact` to understand why they change together. \
             Use `hotspots` to see which of these files are also churn-heavy."
        }
        "ownership" => {
            "Files with many authors or low ownership percentage may need \
             clearer ownership. Use `hotspots` to see if these files are \
             also frequently modified, or `coupling` to find related files."
        }
        "coverage" => {
            "Review dead code candidates (methods with 0 incoming calls). \
             Use `context` on a dead method to verify it's truly unused. \
             Use `diagram` to visualize call chains around covered methods."
        }
        "diagram" => {
            "The Mermaid diagram can be rendered in any Markdown viewer. \
             Use `context` to explore the symbol further, or `impact` \
             to understand the blast radius of changes."
        }
        "report" => {
            "Review the health grade and focus on the worst metrics. \
             Use `hotspots` for file-level churn detail, `coupling` for \
             hidden dependencies, and `coverage` for dead code."
        }
        "analyze_execution_trace" => {
            "Review the timeline of the execution trace and the provided source code for each step. \
             Use this information to write a comprehensive business process documentation, or use `context` \
             to explore specific methods discovered in the trace."
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
            "hotspots",
            "coupling",
            "ownership",
            "coverage",
            "diagram",
            "report",
            "analyze_execution_trace",
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
