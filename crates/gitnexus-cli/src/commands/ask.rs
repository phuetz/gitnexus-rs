//! The `ask` command: ask questions about the codebase using graph + LLM.

use anyhow::Result;
use colored::Colorize;

use gitnexus_db::snapshot;

pub fn run(question: &str, path: Option<&str>) -> Result<()> {
    let repo_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    // Load config
    let config = super::generate::load_llm_config();
    let config = match config {
        Some(c) => c,
        None => {
            println!(
                "{} No LLM configured. Create ~/.gitnexus/chat-config.json with:",
                "ERROR".red()
            );
            println!();
            println!("  {{");
            println!("    \"provider\": \"gemini\",");
            println!("    \"api_key\": \"YOUR_API_KEY\",");
            println!("    \"base_url\": \"https://generativelanguage.googleapis.com/v1beta/openai\",");
            println!("    \"model\": \"gemini-3.1-flash-lite-preview\",");
            println!("    \"max_tokens\": 8192,");
            println!("    \"reasoning_effort\": \"high\"");
            println!("  }}");
            println!();
            println!("  Supported providers: Gemini, OpenAI, Anthropic, OpenRouter, Ollama");
            println!("  For Ollama: base_url = \"http://localhost:11434/v1\", api_key = \"\"");
            return Ok(());
        }
    };

    // Load graph
    let storage_path = repo_path.join(".gitnexus");
    let snap_path = storage_path.join("graph.bin");
    if !snap_path.exists() {
        println!(
            "{} No index found. Run 'gitnexus analyze' first.",
            "ERROR".red()
        );
        return Ok(());
    }

    let graph = snapshot::load_snapshot(&snap_path)
        .map_err(|e| anyhow::anyhow!("Failed to load graph: {}", e))?;

    println!("{} Searching knowledge graph...", "->".cyan());

    // Search the graph for relevant symbols
    let query_lower = question.to_lowercase();
    let mut relevant_nodes: Vec<(&gitnexus_core::graph::types::GraphNode, f64)> = Vec::new();

    for node in graph.iter_nodes() {
        let name_lower = node.properties.name.to_lowercase();
        let file_lower = node.properties.file_path.to_lowercase();

        let mut score = 0.0;
        for word in query_lower.split_whitespace() {
            if name_lower.contains(word) {
                score += 2.0;
            }
            if file_lower.contains(word) {
                score += 0.5;
            }
            if let Some(desc) = &node.properties.description {
                if desc.to_lowercase().contains(word) {
                    score += 1.0;
                }
            }
        }
        if score > 0.0 {
            relevant_nodes.push((node, score));
        }
    }

    relevant_nodes
        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_nodes = &relevant_nodes[..relevant_nodes.len().min(10)];

    if top_nodes.is_empty() {
        println!(
            "{} No relevant symbols found for: {}",
            "WARN".yellow(),
            question
        );
        return Ok(());
    }

    println!("  Found {} relevant symbols", top_nodes.len());

    // Build context from top nodes
    let mut context = String::new();
    for (node, _score) in top_nodes {
        context.push_str(&format!(
            "**{}** ({}) in `{}`\n",
            node.properties.name,
            node.label.as_str(),
            node.properties.file_path
        ));

        // Read source code snippet
        let source_path = repo_path.join(&node.properties.file_path);
        if let Ok(source) = std::fs::read_to_string(&source_path) {
            let lines: Vec<&str> = source.lines().collect();
            let start = node
                .properties
                .start_line
                .map(|l| l as usize)
                .unwrap_or(1)
                .saturating_sub(1);
            let end = (start + 15).min(lines.len());
            context.push_str("```\n");
            for line in &lines[start..end] {
                context.push_str(line);
                context.push('\n');
            }
            context.push_str("```\n\n");
        }
    }

    println!("{} Asking LLM ({})...", "->".cyan(), config.model);

    // Call LLM
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "Tu es un expert en analyse de code. R\u{00e9}ponds de fa\u{00e7}on pr\u{00e9}cise et concise en te basant UNIQUEMENT sur le contexte fourni. Ne fais pas de suppositions."
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("Question : {}\n\nContexte du code :\n{}", question, context)
        }),
    ];

    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": true
    });

    let effort = config.reasoning_effort.trim().to_lowercase();
    if !effort.is_empty() && effort != "none" {
        body["reasoning_effort"] = serde_json::Value::String(effort);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = request.send()?;
    if !response.status().is_success() {
        println!("{} LLM error: {}", "ERROR".red(), response.status());
        return Ok(());
    }

    println!("\n{}\n", "\u{2500}".repeat(60));

    use std::io::{BufRead, BufReader, Write};

    let mut full_answer = String::new();
    let reader = BufReader::new(response);
    for line in reader.lines() {
        let line = line?;
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" { break; }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                    print!("{}", delta);
                    std::io::stdout().flush()?;
                    full_answer.push_str(delta);
                }
            }
        }
    }

    println!("\n\n{}", "\u{2500}".repeat(60));

    // Show sources
    println!("\n{}", "Sources:".dimmed());
    for (node, _) in top_nodes.iter().take(5) {
        println!(
            "  {} `{}` in {}",
            "->".dimmed(),
            node.properties.name,
            node.properties.file_path
        );
    }

    Ok(())
}
