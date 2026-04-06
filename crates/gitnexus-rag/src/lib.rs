//! GraphRAG integration for GitNexus.
//!
//! This crate handles the ingestion, chunking, and semantic anchoring
//! of external documentation (Markdown, PDF, DOCX) into the knowledge graph.

pub mod chunker;

use serde::{Deserialize, Serialize};

/// Represents a semantic chunk of a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocChunk {
    /// The original file path or URL
    pub source_path: String,
    /// Document title or header
    pub title: String,
    /// The extracted text content for this chunk
    pub content: String,
    /// Page number or section index
    pub index: u32,
}
