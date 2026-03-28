use serde::{Deserialize, Serialize};

/// Pipeline execution phase.
/// Matches the TypeScript `PipelinePhase` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelinePhase {
    Idle,
    Extracting,
    Structure,
    Parsing,
    Imports,
    Calls,
    Heritage,
    Communities,
    Processes,
    /// ASP.NET MVC 5 / EF6 enrichment (controllers, actions, entities, views, .edmx)
    AspNetMvc,
    Enriching,
    Complete,
    Error,
}

impl PipelinePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Extracting => "extracting",
            Self::Structure => "structure",
            Self::Parsing => "parsing",
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::Heritage => "heritage",
            Self::Communities => "communities",
            Self::Processes => "processes",
            Self::AspNetMvc => "aspnet_mvc",
            Self::Enriching => "enriching",
            Self::Complete => "complete",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for PipelinePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Progress report from the ingestion pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    pub phase: PipelinePhase,
    /// Completion percentage (0-100)
    pub percent: f64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<PipelineStats>,
}

/// Statistics reported during pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStats {
    pub files_processed: usize,
    pub total_files: usize,
    pub nodes_created: usize,
}
