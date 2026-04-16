use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::{LlmProvider, LlmResponseChunk, LlmStream, Message, ToolCall, ToolDefinition};

pub struct OpenAILlmProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: u32,
    reasoning_effort: String,
}

impl OpenAILlmProvider {
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        max_tokens: u32,
        reasoning_effort: String,
    ) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            base_url,
            api_key,
            model,
            max_tokens,
            reasoning_effort,
        })
    }
}

impl LlmProvider for OpenAILlmProvider {
    async fn stream_completion(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LlmStream, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "temperature": 0.3,
            "stream": true,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        let effort = self.reasoning_effort.trim().to_lowercase();
        if !effort.is_empty() && effort != "none" {
            body["reasoning_effort"] = Value::String(effort);
        }

        let mut request = self.client.post(&url).json(&body);

        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("LLM API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("LLM API error ({}): {}", status, error_text.chars().take(300).collect::<String>()));
        }

        let (tx, rx) = mpsc::channel::<Result<LlmResponseChunk, String>>(100);

        let mut stream = response.bytes_stream();
        
        tokio::spawn(async move {
            let mut byte_buffer: Vec<u8> = Vec::new();
            let mut active_tool_calls: std::collections::HashMap<usize, ToolCall> = std::collections::HashMap::new();
            const MAX_LINE_BUFFER: usize = 1_048_576;

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Stream error: {}", e))).await;
                        return;
                    }
                };
                if byte_buffer.len() + chunk.len() > MAX_LINE_BUFFER {
                    let _ = tx.send(Err("SSE stream partial line exceeded 1MB — aborting".to_string())).await;
                    return;
                }
                byte_buffer.extend_from_slice(&chunk);

                while let Some(newline_pos) = byte_buffer.iter().position(|&b| b == b'\n') {
                    let line_bytes: Vec<u8> = byte_buffer.drain(..=newline_pos).collect();
                    let mut end = line_bytes.len() - 1;
                    if end > 0 && line_bytes[end - 1] == b'\r' {
                        end -= 1;
                    }
                    let line = String::from_utf8_lossy(&line_bytes[..end]);
                    
                    let Some(data) = line.strip_prefix("data: ") else { continue };
                    let data = data.trim();
                    if data == "[DONE]" {
                        continue;
                    }
                    let json = match serde_json::from_str::<Value>(data) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("SSE JSON parse error: {} — data: {}", e, &data[..data.len().min(200)]);
                            continue;
                        }
                    };
                    if let Some(choices) = json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            let delta = &choice["delta"];

                            if let Some(content) = delta["content"].as_str() {
                                if !content.is_empty() && tx.send(Ok(LlmResponseChunk::Text(content.to_string()))).await.is_err() {
                                    return;
                                }
                            }

                            if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                for tc in tool_calls {
                                    if let Some(index) = tc["index"].as_u64() {
                                        let idx = index as usize;

                                        if let Some(id) = tc["id"].as_str() {
                                            active_tool_calls.insert(idx, ToolCall {
                                                id: id.to_string(),
                                                name: tc["function"]["name"].as_str().unwrap_or_default().to_string(),
                                                arguments: String::new(),
                                            });
                                        }

                                        if let Some(active_tc) = active_tool_calls.get_mut(&idx) {
                                            if let Some(args) = tc["function"]["arguments"].as_str() {
                                                active_tc.arguments.push_str(args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

            }

            let mut indices: Vec<usize> = active_tool_calls.keys().copied().collect();
            indices.sort_unstable();
            for idx in indices {
                if let Some(tc) = active_tool_calls.remove(&idx) {
                    if tx.send(Ok(LlmResponseChunk::ToolCall(tc))).await.is_err() {
                        return;
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}