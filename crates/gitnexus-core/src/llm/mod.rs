use std::pin::Pin;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

pub mod openai;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum LlmResponseChunk {
    Text(String),
    ToolCall(ToolCall),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub trait LlmProvider: Send + Sync {
    fn stream_completion(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> impl std::future::Future<Output = Result<Pin<Box<dyn Stream<Item = Result<LlmResponseChunk, String>> + Send>>, String>> + Send;
}

/// Collect a streaming completion into a single String.
/// Used by pipeline enrichment which doesn't need token-by-token streaming.
pub async fn collect_completion(
    provider: &impl LlmProvider,
    messages: &[Message],
) -> Result<String, String> {
    use futures_util::StreamExt;
    let mut stream = provider.stream_completion(messages, &[]).await?;
    let mut result = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk? {
            LlmResponseChunk::Text(text) => result.push_str(&text),
            LlmResponseChunk::ToolCall(_) => {}
        }
    }
    Ok(result)
}