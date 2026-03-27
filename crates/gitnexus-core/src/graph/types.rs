use serde::{Deserialize, Serialize};

use crate::config::languages::SupportedLanguage;

// ─── Node Labels ─────────────────────────────────────────────────────────

/// All possible node types in the knowledge graph.
/// Matches the TypeScript `NodeLabel` union type exactly (38 variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeLabel {
    Project,
    Package,
    Module,
    Folder,
    File,
    Class,
    Function,
    Method,
    Variable,
    Interface,
    Enum,
    Decorator,
    Import,
    Type,
    CodeElement,
    Community,
    Process,
    // Multi-language node types
    Struct,
    Macro,
    Typedef,
    Union,
    Namespace,
    Trait,
    Impl,
    TypeAlias,
    Const,
    Static,
    Property,
    Record,
    Delegate,
    Annotation,
    Constructor,
    Template,
    Section,
    /// API route endpoint (e.g., /api/grants)
    Route,
    /// MCP tool definition
    Tool,
    /// External library / UI component library
    Library,
}

impl NodeLabel {
    /// Returns the string representation matching the TypeScript enum.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Package => "Package",
            Self::Module => "Module",
            Self::Folder => "Folder",
            Self::File => "File",
            Self::Class => "Class",
            Self::Function => "Function",
            Self::Method => "Method",
            Self::Variable => "Variable",
            Self::Interface => "Interface",
            Self::Enum => "Enum",
            Self::Decorator => "Decorator",
            Self::Import => "Import",
            Self::Type => "Type",
            Self::CodeElement => "CodeElement",
            Self::Community => "Community",
            Self::Process => "Process",
            Self::Struct => "Struct",
            Self::Macro => "Macro",
            Self::Typedef => "Typedef",
            Self::Union => "Union",
            Self::Namespace => "Namespace",
            Self::Trait => "Trait",
            Self::Impl => "Impl",
            Self::TypeAlias => "TypeAlias",
            Self::Const => "Const",
            Self::Static => "Static",
            Self::Property => "Property",
            Self::Record => "Record",
            Self::Delegate => "Delegate",
            Self::Annotation => "Annotation",
            Self::Constructor => "Constructor",
            Self::Template => "Template",
            Self::Section => "Section",
            Self::Route => "Route",
            Self::Tool => "Tool",
            Self::Library => "Library",
        }
    }

    /// Parse from string, matching TypeScript values.
    pub fn from_str_label(s: &str) -> Option<Self> {
        match s {
            "Project" => Some(Self::Project),
            "Package" => Some(Self::Package),
            "Module" => Some(Self::Module),
            "Folder" => Some(Self::Folder),
            "File" => Some(Self::File),
            "Class" => Some(Self::Class),
            "Function" => Some(Self::Function),
            "Method" => Some(Self::Method),
            "Variable" => Some(Self::Variable),
            "Interface" => Some(Self::Interface),
            "Enum" => Some(Self::Enum),
            "Decorator" => Some(Self::Decorator),
            "Import" => Some(Self::Import),
            "Type" => Some(Self::Type),
            "CodeElement" => Some(Self::CodeElement),
            "Community" => Some(Self::Community),
            "Process" => Some(Self::Process),
            "Struct" => Some(Self::Struct),
            "Macro" => Some(Self::Macro),
            "Typedef" => Some(Self::Typedef),
            "Union" => Some(Self::Union),
            "Namespace" => Some(Self::Namespace),
            "Trait" => Some(Self::Trait),
            "Impl" => Some(Self::Impl),
            "TypeAlias" => Some(Self::TypeAlias),
            "Const" => Some(Self::Const),
            "Static" => Some(Self::Static),
            "Property" => Some(Self::Property),
            "Record" => Some(Self::Record),
            "Delegate" => Some(Self::Delegate),
            "Annotation" => Some(Self::Annotation),
            "Constructor" => Some(Self::Constructor),
            "Template" => Some(Self::Template),
            "Section" => Some(Self::Section),
            "Route" => Some(Self::Route),
            "Tool" => Some(Self::Tool),
            "Library" => Some(Self::Library),
            _ => None,
        }
    }
}

impl std::fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Relationship Types ──────────────────────────────────────────────────

/// All possible relationship types in the knowledge graph.
/// Matches the TypeScript `RelationshipType` union type exactly (20 variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipType {
    Contains,
    Calls,
    Inherits,
    Overrides,
    Imports,
    Uses,
    Defines,
    Decorates,
    Implements,
    Extends,
    HasMethod,
    HasProperty,
    Accesses,
    MemberOf,
    StepInProcess,
    /// Function/File -> Route (handler serves this endpoint)
    HandlesRoute,
    /// Function/File -> Route (consumer calls this endpoint)
    Fetches,
    /// Function/File -> Tool (handler implements this tool)
    HandlesTool,
    /// Route/Tool -> Process (this endpoint starts this execution flow)
    EntryPointOf,
    /// Function -> Function (middleware wrapper chain) — Reserved: future
    Wraps,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contains => "CONTAINS",
            Self::Calls => "CALLS",
            Self::Inherits => "INHERITS",
            Self::Overrides => "OVERRIDES",
            Self::Imports => "IMPORTS",
            Self::Uses => "USES",
            Self::Defines => "DEFINES",
            Self::Decorates => "DECORATES",
            Self::Implements => "IMPLEMENTS",
            Self::Extends => "EXTENDS",
            Self::HasMethod => "HAS_METHOD",
            Self::HasProperty => "HAS_PROPERTY",
            Self::Accesses => "ACCESSES",
            Self::MemberOf => "MEMBER_OF",
            Self::StepInProcess => "STEP_IN_PROCESS",
            Self::HandlesRoute => "HANDLES_ROUTE",
            Self::Fetches => "FETCHES",
            Self::HandlesTool => "HANDLES_TOOL",
            Self::EntryPointOf => "ENTRY_POINT_OF",
            Self::Wraps => "WRAPS",
        }
    }

    pub fn from_str_type(s: &str) -> Option<Self> {
        match s {
            "CONTAINS" => Some(Self::Contains),
            "CALLS" => Some(Self::Calls),
            "INHERITS" => Some(Self::Inherits),
            "OVERRIDES" => Some(Self::Overrides),
            "IMPORTS" => Some(Self::Imports),
            "USES" => Some(Self::Uses),
            "DEFINES" => Some(Self::Defines),
            "DECORATES" => Some(Self::Decorates),
            "IMPLEMENTS" => Some(Self::Implements),
            "EXTENDS" => Some(Self::Extends),
            "HAS_METHOD" => Some(Self::HasMethod),
            "HAS_PROPERTY" => Some(Self::HasProperty),
            "ACCESSES" => Some(Self::Accesses),
            "MEMBER_OF" => Some(Self::MemberOf),
            "STEP_IN_PROCESS" => Some(Self::StepInProcess),
            "HANDLES_ROUTE" => Some(Self::HandlesRoute),
            "FETCHES" => Some(Self::Fetches),
            "HANDLES_TOOL" => Some(Self::HandlesTool),
            "ENTRY_POINT_OF" => Some(Self::EntryPointOf),
            "WRAPS" => Some(Self::Wraps),
            _ => None,
        }
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Enrichment Source ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnrichedBy {
    Heuristic,
    Llm,
}

// ─── Process Type ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessType {
    IntraCommunity,
    CrossCommunity,
}

// ─── Node Properties ─────────────────────────────────────────────────────

/// Properties attached to a graph node.
/// Matches the TypeScript `NodeProperties` type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeProperties {
    pub name: String,
    pub file_path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<SupportedLanguage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_exported: Option<bool>,

    // AST-derived framework hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_framework_multiplier: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_framework_reason: Option<String>,

    // Community properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic_label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohesion: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enriched_by: Option<EnrichedBy>,

    // Process properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_type: Option<ProcessType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub communities: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,

    // Entry point scoring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_score: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_reason: Option<String>,

    // Method signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,

    // Section-specific (markdown heading level, 1-6)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,

    // Response shape
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_keys: Option<Vec<String>>,

    // Error response shape
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_keys: Option<Vec<String>>,

    // Middleware wrapper chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middleware: Option<Vec<String>>,
}

// ─── Graph Node ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: NodeLabel,
    pub properties: NodeProperties,
}

// ─── Graph Relationship ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRelationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    #[serde(rename = "type")]
    pub rel_type: RelationshipType,
    /// Confidence score 0-1 (1.0 = certain, lower = uncertain resolution)
    pub confidence: f64,
    /// Semantics are edge-type-dependent:
    /// CALLS uses resolution tier, ACCESSES uses 'read'/'write', OVERRIDES uses MRO reason
    pub reason: String,
    /// Step number for STEP_IN_PROCESS relationships (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_label_roundtrip() {
        let label = NodeLabel::Function;
        let json = serde_json::to_string(&label).unwrap();
        assert_eq!(json, "\"Function\"");
        let parsed: NodeLabel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, label);
    }

    #[test]
    fn test_relationship_type_roundtrip() {
        let rt = RelationshipType::StepInProcess;
        let json = serde_json::to_string(&rt).unwrap();
        assert_eq!(json, "\"STEP_IN_PROCESS\"");
        let parsed: RelationshipType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, rt);
    }

    #[test]
    fn test_node_label_all_variants() {
        // Ensure all 36 variants have a str representation
        let labels = [
            NodeLabel::Project, NodeLabel::Package, NodeLabel::Module,
            NodeLabel::Folder, NodeLabel::File, NodeLabel::Class,
            NodeLabel::Function, NodeLabel::Method, NodeLabel::Variable,
            NodeLabel::Interface, NodeLabel::Enum, NodeLabel::Decorator,
            NodeLabel::Import, NodeLabel::Type, NodeLabel::CodeElement,
            NodeLabel::Community, NodeLabel::Process, NodeLabel::Struct,
            NodeLabel::Macro, NodeLabel::Typedef, NodeLabel::Union,
            NodeLabel::Namespace, NodeLabel::Trait, NodeLabel::Impl,
            NodeLabel::TypeAlias, NodeLabel::Const, NodeLabel::Static,
            NodeLabel::Property, NodeLabel::Record, NodeLabel::Delegate,
            NodeLabel::Annotation, NodeLabel::Constructor, NodeLabel::Template,
            NodeLabel::Section, NodeLabel::Route, NodeLabel::Tool,
        ];
        for label in &labels {
            let s = label.as_str();
            assert!(!s.is_empty());
            let parsed = NodeLabel::from_str_label(s).unwrap();
            assert_eq!(*label, parsed);
        }
    }

    #[test]
    fn test_graph_node_serialization() {
        let node = GraphNode {
            id: "Function:src/main.ts:handleLogin".to_string(),
            label: NodeLabel::Function,
            properties: NodeProperties {
                name: "handleLogin".to_string(),
                file_path: "src/main.ts".to_string(),
                start_line: Some(10),
                end_line: Some(25),
                is_exported: Some(true),
                ..Default::default()
            },
        };
        let json = serde_json::to_string_pretty(&node).unwrap();
        assert!(json.contains("\"handleLogin\""));
        assert!(json.contains("\"Function\""));

        // Round-trip
        let parsed: GraphNode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, node.id);
        assert_eq!(parsed.label, node.label);
        assert_eq!(parsed.properties.name, "handleLogin");
    }

    #[test]
    fn test_graph_relationship_serialization() {
        let rel = GraphRelationship {
            id: "rel-1".to_string(),
            source_id: "Function:a".to_string(),
            target_id: "Function:b".to_string(),
            rel_type: RelationshipType::Calls,
            confidence: 0.95,
            reason: "exact".to_string(),
            step: None,
        };
        let json = serde_json::to_string(&rel).unwrap();
        assert!(json.contains("\"CALLS\""));
        let parsed: GraphRelationship = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.rel_type, RelationshipType::Calls);
    }

    #[test]
    fn test_optional_fields_skipped() {
        let props = NodeProperties {
            name: "test".to_string(),
            file_path: "test.ts".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&props).unwrap();
        // Optional None fields should not appear in JSON
        assert!(!json.contains("startLine"));
        assert!(!json.contains("language"));
        assert!(!json.contains("cohesion"));
    }
}
