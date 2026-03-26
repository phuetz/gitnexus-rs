//! Response types for Tauri IPC commands.
//!
//! These types are Cytoscape.js-native and designed for direct consumption
//! by the React frontend. They avoid the MCP JSON envelope format.

use serde::{Deserialize, Serialize};

// ─── Graph Payload (Cytoscape-native) ────────────────────────────────────

/// A graph payload ready for Cytoscape.js rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPayload {
    pub nodes: Vec<CytoNode>,
    pub edges: Vec<CytoEdge>,
    pub stats: GraphStats,
}

/// A node in Cytoscape format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CytoNode {
    pub id: String,
    pub label: String,
    pub name: String,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_exported: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

/// An edge in Cytoscape format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CytoEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub rel_type: String,
    pub confidence: f64,
}

/// Graph statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub truncated: bool,
}

// ─── Graph Filters ───────────────────────────────────────────────────────

/// Zoom level for progressive disclosure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ZoomLevel {
    /// Show Folder/Package nodes with CONTAINS edges
    Package,
    /// Show File/Module nodes with IMPORTS edges
    Module,
    /// Show Function/Class/Method nodes with CALLS/USES edges
    Symbol,
}

/// Filter for graph data requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphFilter {
    pub zoom_level: ZoomLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_nodes: Option<usize>,
}

impl Default for GraphFilter {
    fn default() -> Self {
        Self {
            zoom_level: ZoomLevel::Package,
            labels: None,
            file_paths: None,
            max_nodes: Some(500),
        }
    }
}

// ─── Repository Info ─────────────────────────────────────────────────────

/// Repository info for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,
    pub path: String,
    pub indexed_at: String,
    pub last_commit: String,
    pub files: Option<usize>,
    pub nodes: Option<usize>,
    pub edges: Option<usize>,
    pub communities: Option<usize>,
}

// ─── File Tree ───────────────────────────────────────────────────────────

/// A file tree node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<FileTreeNode>,
}

// ─── Search Results ──────────────────────────────────────────────────────

/// A search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub node_id: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

// ─── Symbol Context ──────────────────────────────────────────────────────

/// 360-degree context for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolContext {
    pub node: CytoNode,
    pub callers: Vec<RelatedNode>,
    pub callees: Vec<RelatedNode>,
    pub imports: Vec<RelatedNode>,
    pub imported_by: Vec<RelatedNode>,
    pub inherits: Vec<RelatedNode>,
    pub inherited_by: Vec<RelatedNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community: Option<CommunityInfo>,
}

/// A related node in context view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedNode {
    pub id: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
}

/// Community info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohesion: Option<f64>,
}

// ─── Impact Analysis ─────────────────────────────────────────────────────

/// Blast radius analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactResult {
    pub target: CytoNode,
    pub upstream: Vec<ImpactNode>,
    pub downstream: Vec<ImpactNode>,
    pub graph: GraphPayload,
    pub affected_files: Vec<String>,
    pub summary: ImpactSummary,
}

/// A node in the impact graph with depth info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactNode {
    pub node: CytoNode,
    pub depth: u32,
    pub path: Vec<String>,
}

/// Summary statistics for impact analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactSummary {
    pub upstream_count: usize,
    pub downstream_count: usize,
    pub affected_files_count: usize,
    pub max_depth: u32,
}

// ─── File Content ────────────────────────────────────────────────────────

/// File content with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub language: Option<String>,
    pub total_lines: usize,
}
