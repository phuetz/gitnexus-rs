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

// ─── Documentation ──────────────────────────────────────────────────────────

/// A documentation page in the navigation tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocPage {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DocPage>>,
}

/// Documentation index with navigation tree and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocIndex {
    pub title: String,
    pub generated_at: String,
    pub stats: DocStats,
    pub pages: Vec<DocPage>,
}

/// Documentation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocStats {
    pub files: usize,
    pub nodes: usize,
    pub edges: usize,
    pub modules: usize,
}

/// Content of a documentation page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocContent {
    pub path: String,
    pub content: String,
    pub title: String,
}

// ─── Chat Q&A ───────────────────────────────────────────────────────────

/// Chat message (user or assistant).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat request from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub question: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
}

/// Chat response returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<ChatSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// A source citation in a chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSource {
    pub node_id: String,
    pub symbol_name: String,
    pub symbol_type: String,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callees: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community: Option<String>,
    pub relevance_score: f64,
}

/// LLM chat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    /// Reasoning effort level for models that support thinking (e.g. Gemini).
    /// Values: "none", "low", "medium", "high". Empty or "none" means disabled.
    #[serde(default)]
    pub reasoning_effort: String,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            api_key: String::new(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3.2".to_string(),
            max_tokens: 4096,
            reasoning_effort: String::new(),
        }
    }
}

// ─── Chat Intelligence (Planner & Executor) ─────────────────────────

/// Query complexity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryComplexity {
    /// Direct lookup — single search or symbol resolution
    Simple,
    /// 2-3 operations — search + context or search + impact
    Medium,
    /// Multi-step research — DAG of operations
    Complex,
}

/// Filters for scoping chat context to specific files/symbols/modules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatContextFilter {
    /// Filter to specific file paths or glob patterns
    #[serde(default)]
    pub files: Vec<String>,
    /// Filter to specific symbol names
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Filter to specific module/community names
    #[serde(default)]
    pub modules: Vec<String>,
    /// Filter to specific languages
    #[serde(default)]
    pub languages: Vec<String>,
    /// Filter to specific node labels (Function, Class, etc.)
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Enhanced chat request with optional context filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSmartRequest {
    pub question: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<ChatContextFilter>,
    /// If true, execute a full research plan (complex mode)
    #[serde(default)]
    pub deep_research: bool,
}

/// Result of analyzing a query's complexity and required tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryAnalysis {
    pub complexity: QueryComplexity,
    pub suggested_tools: Vec<String>,
    pub estimated_steps: u32,
    pub reasoning: String,
    /// Keywords extracted from the query
    pub keywords: Vec<String>,
    /// Whether the query needs cross-file analysis
    pub needs_cross_file: bool,
    /// Whether the query needs impact/dependency analysis
    pub needs_impact: bool,
}

/// A research plan — a DAG of steps to answer a complex question.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchPlan {
    pub id: String,
    pub query: String,
    pub analysis: QueryAnalysis,
    pub steps: Vec<ResearchStep>,
    pub status: PlanStatus,
}

/// Status of the overall research plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// A single step in a research plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchStep {
    pub id: String,
    pub order: u32,
    /// Tool to use: search_symbols, get_symbol_context, get_impact_analysis, execute_cypher, read_file_content
    pub tool: String,
    /// Description of what this step does
    pub description: String,
    /// Parameters for the tool
    pub params: serde_json::Value,
    /// IDs of steps this depends on
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub status: StepStatus,
    /// Result of executing this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<StepResult>,
}

/// Status of an individual research step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Result of executing a research step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Summary of what was found
    pub summary: String,
    /// Sources discovered in this step
    #[serde(default)]
    pub sources: Vec<ChatSource>,
    /// Raw data (tool-specific JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Enhanced chat response with optional research plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSmartResponse {
    pub answer: String,
    pub sources: Vec<ChatSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<ResearchPlan>,
    pub complexity: QueryComplexity,
}

/// File quick-pick result (for Ctrl+P style file picker).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileQuickPick {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub symbol_count: u32,
}

/// Symbol quick-pick result (for Ctrl+Shift+O style symbol picker).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolQuickPick {
    pub node_id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
}

/// Module quick-pick result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleQuickPick {
    pub community_id: String,
    pub name: String,
    pub member_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
