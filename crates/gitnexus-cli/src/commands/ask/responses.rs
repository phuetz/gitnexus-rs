//! Responses API backend (chatgpt.com/backend-api/codex/responses) for tool-loop.
//!
//! This module handles the Responses API format used by ChatGPT's backend,
//! distinct from the classic OpenAI chat/completions format. The key differences:
//! - Request body uses `input` (array of items) instead of `messages`
//! - System prompt goes into `instructions` field
//! - Responses come as SSE events with a different type taxonomy
//! - Tool calls are `{ type: "function_call", name: ..., arguments: ..., call_id: ... }`
//! - Tool results are `{ type: "function_call_output", call_id: ..., output: ... }`
//!
//! References: codex-rs source (github.com/openai/codex)

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use super::StreamEvent;

#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    pub id: String,      // call_id in Responses API
    pub name: String,
    pub args: String,    // JSON arguments as string
}

/// Call the Responses API for one turn, handle streaming SSE, and return text + tool calls.
///
/// Accumulates `output_text.delta` events into the response text.
/// Collects function calls from `output_item.done` events where `item.type == "function_call"`.
/// Modifies `input` in-place to append the assistant's function_call items.
pub async fn call_responses_turn(
    client: &Client,
    token: &str,
    model: &str,
    instructions: &str,
    input: &mut Vec<Value>,
    tools: &[Value],
    stream_cb: Option<&(dyn Fn(StreamEvent) + Send + Sync)>,
) -> Result<(String, Vec<ToolCallRequest>)> {
    let url = "https://chatgpt.com/backend-api/codex/responses";

    // Build request body.
    let body = json!({
        "model": model,
        "instructions": instructions,
        "input": input,
        "tools": tools,
        "tool_choice": "auto",
        "parallel_tool_calls": true,
        "stream": true,
    });

    // Make request with Bearer auth and SSE headers.
    let response = client
        .post(url)
        .bearer_auth(token)
        .header("Accept", "text/event-stream")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Responses API error: {} {}", status, body_text));
    }

    // Stream the SSE response and collect events.
    let mut full_text = String::new();
    let mut tool_calls = Vec::new();
    let mut assistant_items = Vec::new(); // Append to input after parsing.

    let body_text = response.text().await?;
    for line in body_text.lines() {
        if !line.starts_with("data: ") {
            continue;
        }

        let json_str = &line[6..]; // Strip "data: " prefix.
        let event: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => continue, // Malformed JSON, skip.
        };

        let event_type = event["type"].as_str().unwrap_or("");

        match event_type {
            "response.output_text.delta" => {
                if let Some(delta) = event["delta"].as_str() {
                    full_text.push_str(delta);
                    if let Some(cb) = stream_cb {
                        cb(StreamEvent::Delta(delta.to_string()));
                    }
                }
            }

            "response.output_item.done" => {
                if let Some(item) = event["item"].as_object() {
                    if let Some(item_type) = item.get("type").and_then(|v| v.as_str()) {
                        if item_type == "function_call" {
                            // Extract function call details.
                            if let (Some(name), Some(call_id), Some(args)) = (
                                item.get("name").and_then(|v| v.as_str()),
                                item.get("call_id").and_then(|v| v.as_str()),
                                item.get("arguments").and_then(|v| v.as_str()),
                            ) {
                                tool_calls.push(ToolCallRequest {
                                    id: call_id.to_string(),
                                    name: name.to_string(),
                                    args: args.to_string(),
                                });

                                // Also collect the function_call item itself for input history.
                                assistant_items.push(json!({
                                    "type": "function_call",
                                    "name": name,
                                    "arguments": args,
                                    "call_id": call_id,
                                }));
                            }
                        }
                    }
                }
            }

            "response.completed" => {
                // End of turn. Check for response_id if needed (not used here).
            }

            "response.failed" => {
                if let Some(error) = event["response"]["error"].as_object() {
                    let code = error
                        .get("code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let message = error
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("no message");
                    return Err(anyhow!("Responses API failed ({}): {}", code, message));
                }
            }

            _ => {
                // Ignore other event types (metadata, reasoning, etc.).
            }
        }
    }

    // Append function_call items to input for the next turn's context.
    for item in assistant_items {
        input.push(item);
    }

    Ok((full_text, tool_calls))
}

/// Append tool results to the input array in Responses API format.
pub fn append_tool_result(input: &mut Vec<Value>, call_id: &str, output: &str) {
    input.push(json!({
        "type": "function_call_output",
        "call_id": call_id,
        "output": output,
    }));
}
