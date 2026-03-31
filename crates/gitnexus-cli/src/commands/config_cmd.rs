//! The `config` command: validate LLM configuration.

use anyhow::Result;
use colored::Colorize;

pub fn run_test() -> Result<()> {
    let config = super::generate::load_llm_config();
    let config = match config {
        Some(c) => c,
        None => {
            println!("{} No LLM config found.", "ERROR".red());
            println!();
            println!("  Create ~/.gitnexus/chat-config.json with:");
            println!();
            println!("  {{");
            println!("    \"provider\": \"gemini\",");
            println!("    \"api_key\": \"YOUR_API_KEY\",");
            println!("    \"base_url\": \"https://generativelanguage.googleapis.com/v1beta/openai\",");
            println!("    \"model\": \"gemini-2.5-flash\",");
            println!("    \"max_tokens\": 8192,");
            println!("    \"reasoning_effort\": \"high\"");
            println!("  }}");
            return Ok(());
        }
    };

    println!("{} Config loaded:", "OK".green());
    println!("  Provider:  {}", config.provider);
    println!("  Model:     {}", config.model);
    println!("  Base URL:  {}", config.base_url);
    println!("  Max tokens: {}", config.max_tokens);
    println!(
        "  API key:   {}...{}",
        &config.api_key[..config.api_key.len().min(8)],
        if config.api_key.len() > 8 {
            &config.api_key[config.api_key.len() - 4..]
        } else {
            ""
        }
    );

    // Test connectivity
    println!();
    println!("{} Testing connection...", "->".cyan());

    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": "Say hello in one word."}],
        "max_tokens": 10,
        "temperature": 0.0,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    match request.send() {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                println!("{} Connection successful (HTTP {})", "OK".green(), status);
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        println!("  Response: {}", content.trim());
                    }
                }
            } else {
                println!("{} HTTP {} — {}", "ERROR".red(), status, status.canonical_reason().unwrap_or(""));
                if let Ok(body) = resp.text() {
                    let preview: String = body.chars().take(200).collect();
                    println!("  {}", preview);
                }
            }
        }
        Err(e) => {
            println!("{} Connection failed: {}", "ERROR".red(), e);
        }
    }

    Ok(())
}
