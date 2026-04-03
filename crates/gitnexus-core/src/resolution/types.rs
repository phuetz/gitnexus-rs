use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::symbol::SymbolDefinition;

/// Resolution tier indicating how a name was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionTier {
    /// Tier 1: Same-file exact match
    SameFile,
    /// Tier 2a: Named import binding chain
    NamedImport,
    /// Tier 2a: Import-scoped fuzzy match
    ImportScoped,
    /// Tier 2b: Package-scoped fuzzy match
    PackageScoped,
    /// Tier 3: Global fuzzy match
    Global,
}

impl ResolutionTier {
    /// Confidence score for each resolution tier.
    pub fn confidence(&self) -> f64 {
        match self {
            Self::SameFile => 1.0,
            Self::NamedImport => 0.95,
            Self::ImportScoped => 0.8,
            Self::PackageScoped => 0.7,
            Self::Global => 0.5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SameFile => "same-file",
            Self::NamedImport => "named-import",
            Self::ImportScoped => "import-scoped",
            Self::PackageScoped => "package-scoped",
            Self::Global => "global",
        }
    }
}

impl std::fmt::Display for ResolutionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Result of a tiered name resolution.
#[derive(Debug, Clone)]
pub struct TieredCandidates {
    pub tier: ResolutionTier,
    pub candidates: Vec<Arc<SymbolDefinition>>,
}

/// Named import binding: tracks `import { X as Y }`.
#[derive(Debug, Clone)]
pub struct NamedImportBinding {
    /// File path of the source module
    pub source_path: String,
    /// The name exported by the source module
    pub exported_name: String,
}

/// Import map: file_path → set of imported file paths.
pub type ImportMap = HashMap<String, HashSet<String>>;

/// Package map: file_path → set of package directory suffixes (Go/C#).
pub type PackageMap = HashMap<String, HashSet<String>>;

/// Named import map: file_path → (local_name → NamedImportBinding).
pub type NamedImportMap = HashMap<String, HashMap<String, NamedImportBinding>>;

/// Module alias map: file_path → (alias → source_file_path).
/// Used for Python namespace imports: `import models` → `models` → `models.py`.
pub type ModuleAliasMap = HashMap<String, HashMap<String, String>>;
