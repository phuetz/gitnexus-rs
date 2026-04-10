use std::collections::HashMap;

use rayon::prelude::*;
use streaming_iterator::StreamingIterator;
use tracing::warn;
use tree_sitter::{Parser, Query, QueryCursor};

use gitnexus_core::config::languages::SupportedLanguage;
use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::id::generate_id;
use gitnexus_core::symbol::{SymbolDefinition, SymbolTable};

use crate::grammar;
use crate::phases::structure::FileEntry;
use crate::pipeline::ProgressSender;

/// Data extracted from parsing phase (before resolution).
#[derive(Debug, Default)]
pub struct ExtractedData {
    pub imports: Vec<ExtractedImport>,
    pub calls: Vec<ExtractedCall>,
    pub assignments: Vec<ExtractedAssignment>,
    pub heritage: Vec<ExtractedHeritage>,
}

impl ExtractedData {
    fn merge(&mut self, other: ExtractedData) {
        self.imports.extend(other.imports);
        self.calls.extend(other.calls);
        self.assignments.extend(other.assignments);
        self.heritage.extend(other.heritage);
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedImport {
    pub file_path: String,
    pub raw_import_path: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedCall {
    pub file_path: String,
    pub called_name: String,
    pub source_id: String,
    pub arg_count: Option<u32>,
    pub call_form: CallForm,
    pub receiver_name: Option<String>,
    pub receiver_type_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallForm {
    Free,
    Member,
    Constructor,
}

#[derive(Debug, Clone)]
pub struct ExtractedAssignment {
    pub file_path: String,
    pub source_id: String,
    pub receiver_text: String,
    pub property_name: String,
    pub receiver_type_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedHeritage {
    pub file_path: String,
    pub class_name: String,
    pub parent_name: String,
    pub kind: String,
}

/// Result of parsing a single file (graph nodes + extracted data).
struct FileParsed {
    nodes: Vec<GraphNode>,
    relationships: Vec<GraphRelationship>,
    extracted: ExtractedData,
}

/// Parse all files and extract symbols, imports, calls, heritage.
pub fn parse_files(
    graph: &mut KnowledgeGraph,
    files: &[FileEntry],
    _progress_tx: Option<&ProgressSender>,
) -> Result<ExtractedData, crate::IngestError> {
    // Parse all files in parallel using rayon
    let results: Vec<FileParsed> = files
        .par_iter()
        .filter_map(|file| {
            let lang = file.language?;
            if !grammar::is_language_available(lang) {
                return None;
            }
            Some(parse_single_file(file, lang))
        })
        .collect();

    // Merge results into the graph (single-threaded for graph mutation)
    let mut extracted = ExtractedData::default();
    for result in results {
        for node in result.nodes {
            graph.add_node(node);
        }
        for rel in result.relationships {
            graph.add_relationship(rel);
        }
        extracted.merge(result.extracted);
    }

    Ok(extracted)
}

/// Parse a single file with tree-sitter and extract all symbols.
fn parse_single_file(file: &FileEntry, lang: SupportedLanguage) -> FileParsed {
    let ts_language = grammar::get_language(lang);
    let provider = gitnexus_lang::registry::get_provider(lang);
    let query_str = provider.tree_sitter_queries();

    // Create parser and parse the content
    let mut parser = Parser::new();
    if parser.set_language(&ts_language).is_err() {
        warn!("Failed to set language for {}", file.path);
        return FileParsed {
            nodes: Vec::new(),
            relationships: Vec::new(),
            extracted: ExtractedData::default(),
        };
    }

    let tree = match parser.parse(&file.content, None) {
        Some(t) => t,
        None => {
            warn!("Failed to parse {}", file.path);
            return FileParsed {
                nodes: Vec::new(),
                relationships: Vec::new(),
                extracted: ExtractedData::default(),
            };
        }
    };

    // Compile query
    let query = match Query::new(&ts_language, query_str) {
        Ok(q) => q,
        Err(e) => {
            warn!("Query compilation failed for {} ({}): {}", file.path, lang.as_str(), e);
            return FileParsed {
                nodes: Vec::new(),
                relationships: Vec::new(),
                extracted: ExtractedData::default(),
            };
        }
    };

    let content_bytes = file.content.as_bytes();
    let capture_names = query.capture_names();
    let file_node_id = generate_id("File", &file.path);

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut relationships: Vec<GraphRelationship> = Vec::new();
    let mut extracted = ExtractedData::default();

    // Build a capture index for fast lookup: capture_name -> index
    // Execute query
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), content_bytes);

    while let Some(m) = matches.next() {
        // Collect captures for this match into a map: capture_name -> text.
        // We also collect multi-value captures separately for capture names
        // where one match legitimately produces several captures (e.g.
        // `class Foo implements IBar, IBaz` emits multiple
        // `@heritage.implements` captures in a single match — `HashMap::insert`
        // would silently keep only the last one).
        let mut captures: HashMap<&str, (&str, tree_sitter::Node)> = HashMap::new();
        let mut multi_captures: HashMap<&str, Vec<&str>> = HashMap::new();
        for capture in m.captures {
            let Some(name) = capture_names.get(capture.index as usize) else { continue };
            if let Ok(text) = capture.node.utf8_text(content_bytes) {
                captures.insert(name, (text, capture.node));
                multi_captures.entry(name).or_default().push(text);
            }
        }

        // Determine the pattern category from the outermost capture name
        // The pattern type is determined by which captures are present
        process_match(
            &captures,
            &multi_captures,
            file,
            lang,
            &file_node_id,
            &mut nodes,
            &mut relationships,
            &mut extracted,
        );
    }

    // ── Razor-specific post-processing ─────────────────────────────────
    // Extract Razor directives, embedded JavaScript, and detect UI
    // component libraries from .cshtml/.razor files.
    if lang == SupportedLanguage::Razor {
        process_razor_extras(
            file,
            &file_node_id,
            &mut nodes,
            &mut relationships,
            &mut extracted,
        );
    }

    FileParsed {
        nodes,
        relationships,
        extracted,
    }
}

/// Razor-specific post-processing: extract directives, script blocks,
/// and detect UI component library usage.
fn process_razor_extras(
    file: &FileEntry,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    relationships: &mut Vec<GraphRelationship>,
    extracted: &mut ExtractedData,
) {
    use gitnexus_lang::component_detection::{
        extract_html_helpers, extract_razor_directives, extract_script_blocks, ComponentDetector,
    };

    // 1. Extract Razor directives (@page, @model, @inject, @using, etc.)
    let directives = extract_razor_directives(&file.content);
    for directive in &directives {
        match directive.directive.as_str() {
            "page" => {
                // Create a Route node for @page directives
                let route_id = generate_id("Route", &format!("{}:{}", file.path, directive.value));
                let edge_id = format!("handles_route_{}_{}", file_node_id, route_id);
                nodes.push(GraphNode {
                    id: route_id.clone(),
                    label: NodeLabel::Route,
                    properties: NodeProperties {
                        name: directive.value.clone(),
                        file_path: file.path.clone(),
                        start_line: Some(directive.line as u32 + 1),
                        description: Some("Razor page route".to_string()),
                        ..Default::default()
                    },
                });
                relationships.push(GraphRelationship {
                    id: edge_id,
                    source_id: file_node_id.to_string(),
                    target_id: route_id,
                    rel_type: RelationshipType::HandlesRoute,
                    confidence: 1.0,
                    reason: "razor_page_directive".to_string(),
                    step: None,
                });
            }
            "model" => {
                // Create an import reference for the @model type
                extracted.imports.push(ExtractedImport {
                    file_path: file.path.clone(),
                    raw_import_path: directive.value.clone(),
                    language: "razor".to_string(),
                });
            }
            "inject" => {
                // Parse "@inject TypeName FieldName" as an import + dependency
                let parts: Vec<&str> = directive.value.split_whitespace().collect();
                if !parts.is_empty() {
                    extracted.imports.push(ExtractedImport {
                        file_path: file.path.clone(),
                        raw_import_path: format!("@inject {}", directive.value),
                        language: "razor".to_string(),
                    });
                }
            }
            "using" => {
                // Capture @using directives as imports
                extracted.imports.push(ExtractedImport {
                    file_path: file.path.clone(),
                    raw_import_path: directive.value.clone(),
                    language: "razor".to_string(),
                });
            }
            "inherits" => {
                let filename = std::path::Path::new(&file.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown");
                extracted.heritage.push(ExtractedHeritage {
                    file_path: file.path.clone(),
                    class_name: filename.to_string(),
                    parent_name: directive.value.clone(),
                    kind: "extends".to_string(),
                });
            }
            "implements" => {
                let filename = std::path::Path::new(&file.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown");
                extracted.heritage.push(ExtractedHeritage {
                    file_path: file.path.clone(),
                    class_name: filename.to_string(),
                    parent_name: directive.value.clone(),
                    kind: "implements".to_string(),
                });
            }
            "layout" => {
                // @layout defines which layout this component uses (Blazor).
                // Create a heritage relationship: this component "extends" the layout.
                let filename = std::path::Path::new(&file.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown");
                extracted.heritage.push(ExtractedHeritage {
                    file_path: file.path.clone(),
                    class_name: filename.to_string(),
                    parent_name: directive.value.clone(),
                    kind: "extends".to_string(),
                });
                // Also track as an import so the layout file can be resolved
                extracted.imports.push(ExtractedImport {
                    file_path: file.path.clone(),
                    raw_import_path: directive.value.clone(),
                    language: "razor".to_string(),
                });
            }
            "namespace" => {
                // @namespace sets the namespace for the component.
                // Store as metadata on the file node (useful for symbol resolution).
                // We don't create a separate node — just an import for the namespace.
                extracted.imports.push(ExtractedImport {
                    file_path: file.path.clone(),
                    raw_import_path: directive.value.clone(),
                    language: "razor".to_string(),
                });
            }
            _ => {}
        }
    }

    // 2. Extract JavaScript from <script> blocks
    let script_blocks = extract_script_blocks(&file.content);
    if !script_blocks.is_empty() {
        let js_lang = grammar::get_language(SupportedLanguage::JavaScript);
        let js_provider = gitnexus_lang::registry::get_provider(SupportedLanguage::JavaScript);
        let js_query_str = js_provider.tree_sitter_queries();

        let mut js_parser = Parser::new();
        if js_parser.set_language(&js_lang).is_ok() {
            if let Ok(js_query) = Query::new(&js_lang, js_query_str) {
                for (block_idx, (_line_num, script_content)) in script_blocks.iter().enumerate() {
                    let virtual_path = format!("{}#script-{}", file.path, block_idx);
                    let virtual_file_id = generate_id("File", &virtual_path);

                    if let Some(tree) = js_parser.parse(script_content, None) {
                        let content_bytes = script_content.as_bytes();
                        let capture_names = js_query.capture_names();
                        let mut cursor = QueryCursor::new();
                        let mut matches =
                            cursor.matches(&js_query, tree.root_node(), content_bytes);

                        while let Some(m) = matches.next() {
                            let mut captures: HashMap<&str, (&str, tree_sitter::Node)> =
                                HashMap::new();
                            let mut multi_captures: HashMap<&str, Vec<&str>> = HashMap::new();
                            for capture in m.captures {
                                let Some(name) = capture_names.get(capture.index as usize) else { continue };
                                if let Ok(text) = capture.node.utf8_text(content_bytes) {
                                    captures.insert(name, (text, capture.node));
                                    multi_captures.entry(name).or_default().push(text);
                                }
                            }

                            let virtual_file = FileEntry {
                                path: virtual_path.clone(),
                                content: script_content.clone(),
                                language: Some(SupportedLanguage::JavaScript),
                                size: script_content.len(),
                            };

                            process_match(
                                &captures,
                                &multi_captures,
                                &virtual_file,
                                SupportedLanguage::JavaScript,
                                &virtual_file_id,
                                nodes,
                                relationships,
                                extracted,
                            );
                        }
                    }

                    // Link the virtual script file to the Razor file
                    let edge_id = format!("contains_script_{}_{}", file_node_id, virtual_file_id);
                    relationships.push(GraphRelationship {
                        id: edge_id,
                        source_id: file_node_id.to_string(),
                        target_id: virtual_file_id,
                        rel_type: RelationshipType::Contains,
                        confidence: 1.0,
                        reason: "embedded_script_block".to_string(),
                        step: None,
                    });
                }
            }
        }
    }

    // 3. Extract MVC HtmlHelper calls (@Html.Partial, @Html.ActionLink, etc.)
    let helpers = extract_html_helpers(&file.content);
    for helper in &helpers {
        match helper.helper_type.as_str() {
            "Partial" | "RenderPartial" | "PartialAsync" | "RenderPartialAsync" => {
                // Partial view reference → create a call to the partial view file
                extracted.calls.push(ExtractedCall {
                    file_path: file.path.clone(),
                    called_name: helper.target.clone(),
                    source_id: file_node_id.to_string(),
                    arg_count: None,
                    call_form: CallForm::Member,
                    receiver_name: Some("Html".to_string()),
                    receiver_type_name: Some("IHtmlHelper".to_string()),
                });
            }
            "ActionLink" | "Action" | "RenderAction" | "RouteUrl" => {
                // Controller action reference → create a call to the action method
                let target_name = if let Some(ref controller) = helper.controller {
                    format!("{}.{}", controller, helper.target)
                } else {
                    helper.target.clone()
                };
                extracted.calls.push(ExtractedCall {
                    file_path: file.path.clone(),
                    called_name: target_name,
                    source_id: file_node_id.to_string(),
                    arg_count: None,
                    call_form: CallForm::Member,
                    receiver_name: helper.controller.clone(),
                    receiver_type_name: helper
                        .controller
                        .as_ref()
                        .map(|c| format!("{}Controller", c)),
                });
            }
            _ => {}
        }
    }

    // 4. Detect UI component libraries (use shared detector to avoid re-init per file)
    let detector = ComponentDetector::shared();
    let detected = detector.detect_in_file(&file.content, &file.path);
    for component in &detected {
        let lib_id = generate_id("Library", &component.library_name);
        // Only add the library node once (check by ID)
        if !nodes.iter().any(|n| n.id == lib_id) {
            nodes.push(GraphNode {
                id: lib_id.clone(),
                label: NodeLabel::Library,
                properties: NodeProperties {
                    name: component.library_name.clone(),
                    file_path: String::new(), // Library is project-level, not file-specific
                    description: Some(format!(
                        "{} — {} (detected via {:?})",
                        component.vendor, component.category, component.detected_by
                    )),
                    ..Default::default()
                },
            });
        }

        let edge_id = format!("uses_lib_{}_{}", file_node_id, lib_id);
        relationships.push(GraphRelationship {
            id: edge_id,
            source_id: file_node_id.to_string(),
            target_id: lib_id,
            rel_type: RelationshipType::Uses,
            confidence: component.confidence,
            reason: format!("{:?}", component.detected_by),
            step: None,
        });
    }
}

/// Process a single query match and extract nodes/edges/data.
#[allow(clippy::too_many_arguments)]
fn process_match(
    captures: &HashMap<&str, (&str, tree_sitter::Node)>,
    multi_captures: &HashMap<&str, Vec<&str>>,
    file: &FileEntry,
    lang: SupportedLanguage,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    relationships: &mut Vec<GraphRelationship>,
    extracted: &mut ExtractedData,
) {
    // --- Original TS capture pattern: @name + @definition.X ---
    // The original GitNexus queries use @name for the symbol name and
    // @definition.class, @definition.function, etc. as the match pattern.
    if let Some((name, name_node)) = captures.get("name") {
        // Determine label from which @definition.X captures are present
        let label = if captures.contains_key("definition.class") {
            Some(NodeLabel::Class)
        } else if captures.contains_key("definition.function") {
            Some(NodeLabel::Function)
        } else if captures.contains_key("definition.method") {
            Some(NodeLabel::Method)
        } else if captures.contains_key("definition.interface") {
            Some(NodeLabel::Interface)
        } else if captures.contains_key("definition.struct") {
            Some(NodeLabel::Struct)
        } else if captures.contains_key("definition.enum") {
            Some(NodeLabel::Enum)
        } else if captures.contains_key("definition.property") {
            Some(NodeLabel::Property)
        } else if captures.contains_key("definition.constructor") {
            Some(NodeLabel::Constructor)
        } else if captures.contains_key("definition.trait") {
            Some(NodeLabel::Trait)
        } else if captures.contains_key("definition.impl") {
            Some(NodeLabel::Impl)
        } else if captures.contains_key("definition.module") {
            Some(NodeLabel::Module)
        } else if captures.contains_key("definition.namespace") {
            Some(NodeLabel::Namespace)
        } else if captures.contains_key("definition.type") {
            Some(NodeLabel::TypeAlias)
        } else if captures.contains_key("definition.const") {
            Some(NodeLabel::Const)
        } else if captures.contains_key("definition.static") {
            Some(NodeLabel::Static)
        } else if captures.contains_key("definition.macro") {
            Some(NodeLabel::Macro)
        } else if captures.contains_key("definition.typedef") {
            Some(NodeLabel::Typedef)
        } else if captures.contains_key("definition.union") {
            Some(NodeLabel::Union)
        } else if captures.contains_key("definition.record") {
            Some(NodeLabel::Record)
        } else if captures.contains_key("definition.delegate") {
            Some(NodeLabel::Delegate)
        } else if captures.contains_key("definition.annotation") {
            Some(NodeLabel::Annotation)
        } else if captures.contains_key("definition.template") {
            Some(NodeLabel::Template)
        } else {
            None
        };

        if let Some(label) = label {
            create_definition_node(
                label,
                name,
                name_node,
                None,
                file,
                lang,
                file_node_id,
                nodes,
                relationships,
            );
            return;
        }
        // Fall through if @name present but no @definition.X (could be import/call/heritage)
    }

    // --- Original TS: @import with @import.source ---
    if captures.contains_key("import") || captures.contains_key("import.source") {
        extract_import(captures, file, lang, extracted);
        return;
    }

    // --- Original TS: @call with @call.name ---
    if captures.contains_key("call") && captures.contains_key("call.name") {
        extract_call(captures, file, lang, file_node_id, extracted);
        return;
    }

    // --- Original TS: @heritage with @heritage.extends / @heritage.implements / @heritage.trait ---
    if captures.contains_key("heritage") || captures.contains_key("heritage.impl") {
        extract_heritage(captures, multi_captures, file, extracted);
        return;
    }

    // --- Original TS: @assignment with @assignment.receiver / @assignment.property ---
    if captures.contains_key("assignment") && captures.contains_key("assignment.property") {
        extract_assignment(captures, file, file_node_id, extracted);
        return;
    }

    // --- Fallback: agent-style capture names (class.name, function.name, etc.) ---
    // Functions
    if let Some((name, node)) = captures.get("function.name") {
        create_definition_node(
            NodeLabel::Function,
            name,
            node,
            captures.get("function.params").map(|(t, _)| *t),
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Variable functions (arrow / function expressions)
    else if let Some((name, node)) = captures.get("variable_function.name") {
        create_definition_node(
            NodeLabel::Function,
            name,
            node,
            captures.get("variable_function.params").map(|(t, _)| *t),
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Classes
    else if let Some((name, node)) = captures.get("class.name") {
        create_definition_node(
            NodeLabel::Class,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Methods
    else if let Some((name, node)) = captures.get("method.name") {
        create_definition_node(
            NodeLabel::Method,
            name,
            node,
            captures.get("method.params").map(|(t, _)| *t),
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Interfaces
    else if let Some((name, node)) = captures.get("interface.name") {
        create_definition_node(
            NodeLabel::Interface,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Structs
    else if let Some((name, node)) = captures.get("struct.name") {
        create_definition_node(
            NodeLabel::Struct,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Enums
    else if let Some((name, node)) = captures.get("enum.name") {
        create_definition_node(
            NodeLabel::Enum,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Traits
    else if let Some((name, node)) = captures.get("trait.name") {
        create_definition_node(
            NodeLabel::Trait,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Constructors
    else if let Some((name, node)) = captures.get("constructor.name") {
        create_definition_node(
            NodeLabel::Constructor,
            name,
            node,
            captures.get("constructor.params").map(|(t, _)| *t),
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Type aliases
    else if let Some((name, node)) = captures.get("type_alias.name") {
        create_definition_node(
            NodeLabel::TypeAlias,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Constants
    else if let Some((name, node)) = captures.get("const.name") {
        create_definition_node(
            NodeLabel::Const,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Statics
    else if let Some((name, node)) = captures.get("static.name") {
        create_definition_node(
            NodeLabel::Static,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Macros
    else if let Some((name, node)) = captures.get("macro.name") {
        create_definition_node(
            NodeLabel::Macro,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Modules
    else if let Some((name, node)) = captures.get("module.name") {
        create_definition_node(
            NodeLabel::Module,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Namespaces
    else if let Some((name, node)) = captures.get("namespace.name") {
        create_definition_node(
            NodeLabel::Namespace,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Typedefs
    else if let Some((name, node)) = captures.get("typedef.name") {
        create_definition_node(
            NodeLabel::Typedef,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Unions
    else if let Some((name, node)) = captures.get("union.name") {
        create_definition_node(
            NodeLabel::Union,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Records
    else if let Some((name, node)) = captures.get("record.name") {
        create_definition_node(
            NodeLabel::Record,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Annotation types
    else if let Some((name, node)) = captures.get("annotation_type.name") {
        create_definition_node(
            NodeLabel::Annotation,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Delegates
    else if let Some((name, node)) = captures.get("delegate.name") {
        create_definition_node(
            NodeLabel::Delegate,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Protocols (Swift - treated as Interface)
    else if let Some((name, node)) = captures.get("protocol.name") {
        create_definition_node(
            NodeLabel::Interface,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // Function signatures (TypeScript overloads)
    else if let Some((name, node)) = captures.get("function_signature.name") {
        create_definition_node(
            NodeLabel::Function,
            name,
            node,
            captures.get("function_signature.params").map(|(t, _)| *t),
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
    // --- Imports ---
    else if captures.contains_key("import") || captures.contains_key("import.source")
        || captures.contains_key("import.path") || captures.contains_key("import.name")
    {
        extract_import(captures, file, lang, extracted);
    }
    // --- Function calls ---
    else if captures.contains_key("call.function") || captures.contains_key("call.method") {
        extract_call(captures, file, lang, file_node_id, extracted);
    }
    // --- Constructor calls (new expressions) ---
    else if captures.contains_key("new.constructor") || captures.contains_key("new.type") {
        extract_new_call(captures, file, file_node_id, extracted);
    }
    // --- Heritage ---
    else if captures.contains_key("heritage.extends") || captures.contains_key("heritage.implements")
        || captures.contains_key("heritage.trait") || captures.contains_key("heritage.embeds")
        || captures.contains_key("heritage.conforms") || captures.contains_key("heritage.protocol")
        || captures.contains_key("heritage.uses_trait")
    {
        extract_heritage(captures, multi_captures, file, extracted);
    }
    // --- Assignments (member/field) ---
    else if captures.contains_key("assignment.property") {
        extract_assignment(captures, file, file_node_id, extracted);
    }
    // --- Properties (field definitions) - create Property nodes ---
    else if let Some((name, node)) = captures.get("property.name") {
        create_definition_node(
            NodeLabel::Property,
            name,
            node,
            None,
            file,
            lang,
            file_node_id,
            nodes,
            relationships,
        );
    }
}

/// Create a definition node and a DEFINES edge from the file to it.
#[allow(clippy::too_many_arguments)]
fn create_definition_node(
    label: NodeLabel,
    name: &str,
    node: &tree_sitter::Node,
    params_text: Option<&str>,
    file: &FileEntry,
    lang: SupportedLanguage,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    relationships: &mut Vec<GraphRelationship>,
) {
    let qualified_name = format!("{}:{}", file.path, name);
    let node_id = generate_id(label.as_str(), &qualified_name);

    // Count parameters if we have params text
    let parameter_count = params_text.map(count_parameters);

    // Check export status using the language provider
    let provider = gitnexus_lang::registry::get_provider(lang);
    // Approximate ancestors check: look at the node's parent chain
    let parent_type = node
        .parent()
        .map(|p| p.kind().to_string())
        .unwrap_or_default();
    let grandparent_type = node
        .parent()
        .and_then(|p| p.parent())
        .map(|gp| gp.kind().to_string())
        .unwrap_or_default();

    let ancestors = [parent_type.as_str(), grandparent_type.as_str()];
    let is_exported = provider.check_export(name, node.kind(), &ancestors);

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node
        .parent()
        .map(|p| p.end_position().row as u32 + 1)
        .unwrap_or(start_line);

    // Compute cyclomatic complexity for callable nodes
    let complexity = if matches!(label, NodeLabel::Method | NodeLabel::Function | NodeLabel::Constructor) {
        // Walk up to the definition node (parent of the name node) to get the full body
        let def_node = node.parent().unwrap_or(*node);
        Some(compute_complexity(def_node, file.content.as_bytes()))
    } else {
        None
    };

    let graph_node = GraphNode {
        id: node_id.clone(),
        label,
        properties: NodeProperties {
            name: name.to_string(),
            file_path: file.path.clone(),
            start_line: Some(start_line),
            end_line: Some(end_line),
            language: Some(lang),
            is_exported: Some(is_exported),
            parameter_count,
            complexity,
            ..Default::default()
        },
    };
    nodes.push(graph_node);

    // Create nesting edges: Class -> Method/Property/Constructor
    if matches!(label, NodeLabel::Method | NodeLabel::Property | NodeLabel::Constructor) {
        if let Some(class_node_id) = find_enclosing_class_id(node, &file.path, file.content.as_bytes()) {
            let rel_type = if label == NodeLabel::Property {
                RelationshipType::HasProperty
            } else {
                RelationshipType::HasMethod
            };
            let nesting_edge_id = format!("{}_{}", rel_type.as_str().to_lowercase(), node_id);
            relationships.push(GraphRelationship {
                id: nesting_edge_id,
                source_id: class_node_id,
                target_id: node_id.clone(),
                rel_type,
                confidence: 1.0,
                reason: "ast_nesting".to_string(),
                step: None,
            });
        }
    }

    // Create DEFINES edge: File -> Symbol
    let edge_id = format!("defines_{}_{}", file_node_id, node_id);
    relationships.push(GraphRelationship {
        id: edge_id,
        source_id: file_node_id.to_string(),
        target_id: node_id,
        rel_type: RelationshipType::Defines,
        confidence: 1.0,
        reason: "ast".to_string(),
        step: None,
    });
}

/// Compute cyclomatic complexity (CC) for a tree-sitter AST node.
///
/// CC = 1 + number of decision points found in the subtree.
/// Decision points: if, for, foreach, while, do, case/switch-arm, catch,
/// ternary/conditional expressions, and `&&`/`||` binary operators.
fn compute_complexity(node: tree_sitter::Node, content: &[u8]) -> u32 {
    let mut cc = 1u32;
    let mut cursor = node.walk();
    walk_tree_for_complexity(&mut cursor, content, &mut cc);
    cc
}

/// Recursively walk the AST via TreeCursor counting decision points.
fn walk_tree_for_complexity(
    cursor: &mut tree_sitter::TreeCursor,
    content: &[u8],
    cc: &mut u32,
) {
    let kind = cursor.node().kind();
    match kind {
        // Branching
        "if_statement" | "if_expression" => *cc += 1,

        // Loops
        "for_statement" | "for_expression" | "foreach_statement"
        | "for_in_statement" | "for_each_statement"
        | "enhanced_for_statement" => *cc += 1,

        "while_statement" | "while_expression" => *cc += 1,

        "do_statement" => *cc += 1,

        // Case clauses (NOT the switch/match itself)
        "case_clause" | "switch_expression_arm" | "match_arm" => *cc += 1,

        // Exception handling
        "catch_clause" | "catch_declaration" => *cc += 1,

        // Ternary / conditional expressions
        "conditional_expression" | "ternary_expression" => *cc += 1,

        // Logical operators in binary expressions
        "binary_expression" | "logical_expression" => {
            // Check if the operator is && or ||
            if let Some(op_node) = cursor.node().child_by_field_name("operator") {
                if let Ok(op_text) = op_node.utf8_text(content) {
                    if op_text == "&&" || op_text == "||" || op_text == "and" || op_text == "or" {
                        *cc += 1;
                    }
                }
            }
        }

        _ => {}
    }

    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            walk_tree_for_complexity(cursor, content, cc);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Walk up the tree-sitter AST from `node` to find the nearest enclosing class/struct/interface
/// container, and return its graph node ID so we can create HasMethod/HasProperty edges.
fn find_enclosing_class_id(
    node: &tree_sitter::Node,
    file_path: &str,
    content: &[u8],
) -> Option<String> {
    // Container node kinds across all supported languages
    const CONTAINER_KINDS: &[&str] = &[
        // C#
        "class_declaration",
        "struct_declaration",
        "interface_declaration",
        "record_declaration",
        // Java
        "annotation_type_declaration",
        // Python
        "class_definition",
        // Rust
        "struct_item",
        "impl_item",
        "enum_item",
        "trait_item",
        // C / C++
        "class_specifier",
        "struct_specifier",
        // PHP
        "trait_declaration",
        // Note: "class_declaration" / "interface_declaration" / "enum_declaration"
        // are shared across C#, Java, TS/JS, Kotlin, PHP, Swift — no duplicates needed
    ];

    let mut cursor = node.parent();
    while let Some(ancestor) = cursor {
        let kind = ancestor.kind();
        if CONTAINER_KINDS.contains(&kind) {
            // Extract the class/struct/interface name via the "name" field child
            if let Some(name_node) = ancestor.child_by_field_name("name") {
                let class_name = name_node.utf8_text(content).ok()?;
                let label_str = match kind {
                    k if k.contains("interface") => "Interface",
                    k if k.contains("struct") => "Struct",
                    k if k.contains("record") => "Record",
                    k if k.contains("trait") => "Trait",
                    k if k.contains("impl") => "Impl",
                    k if k.contains("enum") => "Enum",
                    _ => "Class",
                };
                let qualified = format!("{}:{}", file_path, class_name);
                return Some(generate_id(label_str, &qualified));
            }
        }
        cursor = ancestor.parent();
    }
    None
}

/// Walk up the tree-sitter AST from `node` to find the nearest enclosing method/function/constructor
/// and return its graph node ID. This enables Method→Method Calls edges instead of File→Method.
fn find_enclosing_method_id(
    node: &tree_sitter::Node,
    file_path: &str,
    content: &[u8],
) -> Option<String> {
    const METHOD_KINDS: &[&str] = &[
        // C#
        "method_declaration",
        "constructor_declaration",
        "local_function_statement",
        // Java
        // (method_declaration, constructor_declaration already listed)
        // Python
        "function_definition",
        // Rust
        "function_item",
        // JavaScript / TypeScript
        "method_definition",
        "function_declaration",
        // Anonymous JS/TS function-like nodes (named via parent variable_declarator
        // or property assignment).
        "arrow_function",
        "function_expression",
        // Kotlin / Swift / generic lambdas
        "lambda_expression",
        // C / C++
        // (function_definition already listed)
        // Generic
        "function",
    ];

    let mut cursor = node.parent();
    while let Some(ancestor) = cursor {
        let kind = ancestor.kind();
        if METHOD_KINDS.contains(&kind) {
            // 1. Direct `name` field — works for declarations and method_definition.
            // 2. Fallback for arrow_function / function_expression: walk up to a
            //    `variable_declarator` (e.g. `const foo = () => {}`) or `pair` /
            //    `property_assignment` (object literal `{ foo: () => {} }`) and
            //    grab its name.
            let name_node = ancestor.child_by_field_name("name").or_else(|| {
                let parent = ancestor.parent()?;
                match parent.kind() {
                    "variable_declarator"
                    | "pair"
                    | "property_assignment"
                    | "field_definition"
                    | "public_field_definition" => parent.child_by_field_name("name"),
                    _ => None,
                }
            });
            if let Some(name_node) = name_node {
                let method_name = name_node.utf8_text(content).ok()?;
                let label_str = if kind.contains("constructor") {
                    "Constructor"
                } else if kind == "function_declaration"
                    || kind == "function_definition"
                    || kind == "function_item"
                    || kind == "arrow_function"
                    || kind == "function_expression"
                    || kind == "lambda_expression"
                    // C# `local_function_statement` is registered as
                    // `@definition.function` by the C# query, which generates
                    // a Function node ID. If we classify it as "Method" here,
                    // every CALLS edge from inside a C# local function points
                    // to a phantom `Method:...` node and the call disappears
                    // from impact analysis.
                    || kind == "local_function_statement"
                {
                    "Function"
                } else {
                    "Method"
                };
                let qualified = format!("{}:{}", file_path, method_name);
                return Some(generate_id(label_str, &qualified));
            }
        }
        cursor = ancestor.parent();
    }
    None
}

/// Count parameters from a params string like "(a, b, c)" or "(a: int, b: str)".
///
/// Splits on top-level commas and discards empty segments so trailing commas
/// (e.g. `"(a, b, )"`) and whitespace-only argument lists do not inflate the
/// arity. A pure empty list `"()"` returns 0.
fn count_parameters(params: &str) -> u32 {
    let trimmed = params.trim();
    // Remove surrounding parens
    let inner = if trimmed.starts_with('(') && trimmed.ends_with(')') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    let inner = inner.trim();
    if inner.is_empty() {
        return 0;
    }
    // Walk the string, splitting at top-level commas (not inside nested
    // parens/brackets/braces). Angle brackets are tracked separately because
    // `<` / `>` are ambiguous between comparison operators and generics.
    let mut depth = 0i32;
    let mut angle_depth = 0i32;
    let mut count = 0u32;
    let mut current_has_content = false;
    for ch in inner.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                current_has_content = true;
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current_has_content = true;
            }
            '<' => {
                angle_depth += 1;
                current_has_content = true;
            }
            '>' if angle_depth > 0 => {
                angle_depth -= 1;
                current_has_content = true;
            }
            ',' if depth == 0 && angle_depth == 0 => {
                if current_has_content {
                    count += 1;
                }
                current_has_content = false;
            }
            c if c.is_whitespace() => {}
            _ => {
                current_has_content = true;
            }
        }
    }
    if current_has_content {
        count += 1;
    }
    count
}

/// Extract import information from match captures.
fn extract_import(
    captures: &HashMap<&str, (&str, tree_sitter::Node)>,
    file: &FileEntry,
    lang: SupportedLanguage,
    extracted: &mut ExtractedData,
) {
    // Try different capture names for the import path/source
    let raw_path = captures
        .get("import.source")
        .or_else(|| captures.get("import.path"))
        .or_else(|| captures.get("import.name"))
        .map(|(text, _)| *text);

    if let Some(path) = raw_path {
        // Clean quotes from import path
        let cleaned = path.trim_matches(|c| c == '"' || c == '\'' || c == '`');
        extracted.imports.push(ExtractedImport {
            file_path: file.path.clone(),
            raw_import_path: cleaned.to_string(),
            language: lang.as_str().to_string(),
        });
    }
}

/// Extract function call information from match captures.
fn extract_call(
    captures: &HashMap<&str, (&str, tree_sitter::Node)>,
    file: &FileEntry,
    lang: SupportedLanguage,
    file_node_id: &str,
    extracted: &mut ExtractedData,
) {
    // Determine call form and name
    // Original TS queries use @call.name for both free and member calls
    // Agent-style queries use @call.method + @call.object or @call.function
    let (called_name, call_form, receiver_name) =
        if let Some((call_name, _)) = captures.get("call.name") {
            // Original capture pattern - determine form from context
            // If there's a receiver/object capture, it's a member call
            let receiver = captures.get("call.object")
                .or_else(|| captures.get("assignment.receiver"))
                .map(|(t, _)| t.to_string());
            let form = if receiver.is_some() { CallForm::Member } else { CallForm::Free };
            (call_name.to_string(), form, receiver)
        } else if let Some((method_name, _)) = captures.get("call.method") {
            let receiver = captures.get("call.object").map(|(t, _)| t.to_string());
            (method_name.to_string(), CallForm::Member, receiver)
        } else if let Some((func_name, _)) = captures.get("call.function") {
            (func_name.to_string(), CallForm::Free, None)
        } else {
            return;
        };

    // Language-specific call routing. The Ruby provider redirects calls like
    // `require 'foo'`, `include Bar`, and `attr_accessor :baz` to imports,
    // heritage, and property declarations respectively. Without this hook,
    // every `require` in a Ruby project produced an unresolved Calls edge
    // and zero Imports edges existed for any Ruby file. The route is opt-in
    // — `route_call` returns `None` for languages that don't override it.
    let provider = gitnexus_lang::registry::get_provider(lang);
    let call_text = captures
        .get("call")
        .and_then(|(_, node)| node.utf8_text(file.content.as_bytes()).ok())
        .unwrap_or("");
    if let Some(routed) = provider.route_call(&called_name, call_text) {
        use gitnexus_lang::call_routing::CallRoutingResult;
        match routed {
            CallRoutingResult::Import { import_path, is_relative } => {
                // The Ruby resolver treats a path as relative only if it
                // starts with `./` or `../`. `require_relative 'models/user'`
                // (without the dot prefix) is also relative in Ruby — anchor
                // it to the calling file by injecting `./` when the call
                // form is `require_relative` but the path is bare.
                let normalized = if is_relative
                    && !import_path.starts_with("./")
                    && !import_path.starts_with("../")
                {
                    format!("./{import_path}")
                } else {
                    import_path
                };
                extracted.imports.push(ExtractedImport {
                    file_path: file.path.clone(),
                    raw_import_path: normalized,
                    language: lang.as_str().to_string(),
                });
                return;
            }
            CallRoutingResult::Skip => return,
            // Heritage / Properties / Call routing not yet wired here — the
            // Ruby `include` / `attr_accessor` patterns currently still flow
            // through as plain calls so name-based resolution can pick them
            // up. Treat them like normal calls for now.
            _ => {}
        }
    }

    // Count args
    let arg_count = captures.get("call.args").map(|(text, _)| count_parameters(text));

    // Resolve enclosing method as call source (fallback to file node)
    let source_id = captures
        .get("call")
        .or_else(|| captures.get("call.name"))
        .and_then(|(_, node)| find_enclosing_method_id(node, &file.path, file.content.as_bytes()))
        .unwrap_or_else(|| file_node_id.to_string());

    extracted.calls.push(ExtractedCall {
        file_path: file.path.clone(),
        called_name,
        source_id,
        arg_count,
        call_form,
        receiver_name,
        receiver_type_name: None,
    });
}

/// Extract constructor call (new expression) information.
fn extract_new_call(
    captures: &HashMap<&str, (&str, tree_sitter::Node)>,
    file: &FileEntry,
    file_node_id: &str,
    extracted: &mut ExtractedData,
) {
    let constructor_name = captures
        .get("new.constructor")
        .or_else(|| captures.get("new.type"))
        .map(|(text, _)| text.to_string());

    if let Some(name) = constructor_name {
        let arg_count = captures.get("new.args").map(|(text, _)| count_parameters(text));

        let source_id = captures
            .get("new.constructor")
            .or_else(|| captures.get("new.type"))
            .and_then(|(_, node)| find_enclosing_method_id(node, &file.path, file.content.as_bytes()))
            .unwrap_or_else(|| file_node_id.to_string());

        extracted.calls.push(ExtractedCall {
            file_path: file.path.clone(),
            called_name: name,
            source_id,
            arg_count,
            call_form: CallForm::Constructor,
            receiver_name: None,
            receiver_type_name: None,
        });
    }
}

/// Extract heritage (extends/implements/trait) information.
fn extract_heritage(
    captures: &HashMap<&str, (&str, tree_sitter::Node)>,
    multi_captures: &HashMap<&str, Vec<&str>>,
    file: &FileEntry,
    extracted: &mut ExtractedData,
) {
    let class_name = captures
        .get("heritage.class")
        .or_else(|| captures.get("heritage.type"))
        .or_else(|| captures.get("heritage.struct"))
        .or_else(|| captures.get("heritage.record"))
        .or_else(|| captures.get("heritage.extension"))
        .map(|(text, _)| text.to_string());

    let push_all = |key: &str, kind: &str, extracted: &mut ExtractedData| {
        let Some(ref cls) = class_name else { return };
        let Some(items) = multi_captures.get(key) else { return };
        for item in items {
            extracted.heritage.push(ExtractedHeritage {
                file_path: file.path.clone(),
                class_name: cls.clone(),
                parent_name: (*item).to_string(),
                kind: kind.to_string(),
            });
        }
    };

    // For every heritage capture name, iterate over ALL matched values, not
    // just the last one stored in `captures`. The HashMap-based `captures`
    // silently overwrites repeats, so a class implementing multiple interfaces
    // (`class Foo : IBar, IBaz`) used to record only the last interface.
    push_all("heritage.extends", "extends", extracted);
    push_all("heritage.implements", "implements", extracted);
    push_all("heritage.trait", "implements", extracted);
    push_all("heritage.embeds", "extends", extracted);
    push_all("heritage.conforms", "implements", extracted);
    push_all("heritage.protocol", "implements", extracted);
    push_all("heritage.uses_trait", "uses", extracted);
}

/// Extract assignment (member/field) information.
fn extract_assignment(
    captures: &HashMap<&str, (&str, tree_sitter::Node)>,
    file: &FileEntry,
    file_node_id: &str,
    extracted: &mut ExtractedData,
) {
    let receiver = captures
        .get("assignment.object")
        .map(|(text, _)| text.to_string())
        .unwrap_or_default();

    let property = captures
        .get("assignment.property")
        .map(|(text, _)| text.to_string())
        .unwrap_or_default();

    if !property.is_empty() {
        extracted.assignments.push(ExtractedAssignment {
            file_path: file.path.clone(),
            source_id: file_node_id.to_string(),
            receiver_text: receiver,
            property_name: property,
            receiver_type_name: None,
        });
    }
}

/// Scan for .csproj files and detect component libraries from NuGet PackageReferences.
///
/// This provides higher-confidence library detection than source-level patterns because
/// .csproj files contain the definitive list of NuGet dependencies with exact versions.
pub fn detect_csproj_components(
    graph: &mut KnowledgeGraph,
    repo_path: &std::path::Path,
) {
    use gitnexus_lang::component_detection::ComponentDetector;
    use ignore::WalkBuilder;

    let detector = ComponentDetector::shared();

    // Walk the repo looking for .csproj files (they may not be in the file_entries
    // since walk_repository only picks up source files with known languages).
    let walker = WalkBuilder::new(repo_path)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .max_depth(Some(8)) // .csproj files shouldn't be deeply nested
        .build();

    for result in walker.flatten() {
        if !result.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = result.path();
        let path_str = path.to_string_lossy();

        let is_project_file = path_str.ends_with(".csproj")
            || path_str.ends_with("packages.config")
            || path_str.ends_with("web.config");

        if !is_project_file {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = path
            .strip_prefix(repo_path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let detected = detector.detect_in_csproj(&content);
        if detected.is_empty() {
            continue;
        }

        // Create a Project node for the .csproj if it doesn't exist
        let project_id = generate_id("File", &rel_path);

        for component in &detected {
            let lib_id = generate_id("Library", &component.library_name);

            // Add Library node if not present
            if graph.get_node(&lib_id).is_none() {
                let mut desc = format!(
                    "{} — {}",
                    component.vendor, component.category
                );
                if let Some(ref ver) = component.detected_version {
                    desc.push_str(&format!(" (v{})", ver));
                }
                graph.add_node(GraphNode {
                    id: lib_id.clone(),
                    label: NodeLabel::Library,
                    properties: NodeProperties {
                        name: component.library_name.clone(),
                        file_path: rel_path.clone(),
                        description: Some(desc),
                        ..Default::default()
                    },
                });
            }

            // Link project → library
            let edge_id = format!("uses_lib_{}_{}", project_id, lib_id);
            graph.add_relationship(GraphRelationship {
                id: edge_id,
                source_id: project_id.clone(),
                target_id: lib_id,
                rel_type: RelationshipType::Uses,
                confidence: component.confidence,
                reason: format!("csproj_{:?}", component.detected_by),
                step: None,
            });
        }
    }
}

/// Build symbol table from the current graph state.
pub fn build_symbol_table(graph: &KnowledgeGraph, table: &mut SymbolTable) {
    graph.for_each_node(|node| {
        match node.label {
            NodeLabel::Function
            | NodeLabel::Method
            | NodeLabel::Constructor
            | NodeLabel::Class
            | NodeLabel::Interface
            | NodeLabel::Struct
            | NodeLabel::Trait
            | NodeLabel::Enum
            | NodeLabel::Variable
            | NodeLabel::Property
            | NodeLabel::TypeAlias
            | NodeLabel::Const
            | NodeLabel::Static
            | NodeLabel::Macro => {
                let def = SymbolDefinition {
                    node_id: node.id.clone(),
                    file_path: node.properties.file_path.clone(),
                    symbol_type: node.label,
                    parameter_count: node.properties.parameter_count,
                    required_parameter_count: None,
                    parameter_types: None,
                    return_type: node.properties.return_type.clone(),
                    declared_type: None,
                    owner_id: None,
                    is_exported: node.properties.is_exported.unwrap_or(false),
                };
                table.add(node.properties.name.clone(), def);
            }
            _ => {}
        }
    });

    // Populate owner_id from HasMethod / HasProperty edges so that
    // call resolution can match methods to their containing class.
    for rel in graph.iter_relationships() {
        if !matches!(
            rel.rel_type,
            RelationshipType::HasMethod | RelationshipType::HasProperty
        ) {
            continue;
        }
        let owner_id = rel.source_id.clone();
        let target_id = &rel.target_id;
        table.set_owner_id(target_id, owner_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_parameters_empty() {
        assert_eq!(count_parameters("()"), 0);
        assert_eq!(count_parameters("(  )"), 0);
    }

    #[test]
    fn test_count_parameters_simple() {
        assert_eq!(count_parameters("(a, b, c)"), 3);
        assert_eq!(count_parameters("(x)"), 1);
    }

    #[test]
    fn test_count_parameters_with_types() {
        assert_eq!(count_parameters("(a: number, b: string)"), 2);
    }

    #[test]
    fn test_count_parameters_nested() {
        // Nested generics should not count extra commas
        assert_eq!(count_parameters("(a: Map<K, V>, b: int)"), 2);
        assert_eq!(count_parameters("(f: Fn(a, b) -> c, d: int)"), 2);
    }

    #[test]
    fn test_parse_javascript_function() {
        let file = FileEntry {
            path: "test.js".to_string(),
            content: "function greet(name) { return 'hello ' + name; }".to_string(),
            size: 49,
            language: Some(SupportedLanguage::JavaScript),
        };

        let mut graph = KnowledgeGraph::new();
        // Add file node first
        graph.add_node(GraphNode {
            id: "File:test.js".to_string(),
            label: NodeLabel::File,
            properties: NodeProperties {
                name: "test.js".to_string(),
                file_path: "test.js".to_string(),
                ..Default::default()
            },
        });

        let _extracted = parse_files(&mut graph, &[file], None).unwrap();

        // Should have created a Function node for greet
        let func_node = graph.get_node("Function:test.js:greet");
        assert!(func_node.is_some(), "Should create Function node for greet");
        let func = func_node.unwrap();
        assert_eq!(func.properties.name, "greet");
        // Note: original queries don't capture parameter count directly
        // Parameter count extraction happens via AST analysis in full implementation
    }

    #[test]
    fn test_parse_javascript_class() {
        let file = FileEntry {
            path: "test.js".to_string(),
            content: "class UserService { constructor() {} getUser(id) { } }".to_string(),
            size: 54,
            language: Some(SupportedLanguage::JavaScript),
        };

        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "File:test.js".to_string(),
            label: NodeLabel::File,
            properties: NodeProperties {
                name: "test.js".to_string(),
                file_path: "test.js".to_string(),
                ..Default::default()
            },
        });

        let _extracted = parse_files(&mut graph, &[file], None).unwrap();

        assert!(
            graph.get_node("Class:test.js:UserService").is_some(),
            "Should create Class node"
        );
    }

    #[test]
    fn test_parse_javascript_imports() {
        let file = FileEntry {
            path: "test.js".to_string(),
            content: r#"import { foo } from './utils';"#.to_string(),
            size: 30,
            language: Some(SupportedLanguage::JavaScript),
        };

        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "File:test.js".to_string(),
            label: NodeLabel::File,
            properties: NodeProperties {
                name: "test.js".to_string(),
                file_path: "test.js".to_string(),
                ..Default::default()
            },
        });

        let extracted = parse_files(&mut graph, &[file], None).unwrap();
        assert!(!extracted.imports.is_empty(), "Should extract import");
        assert_eq!(extracted.imports[0].raw_import_path, "./utils");
    }

    #[test]
    fn test_parse_python_function() {
        let file = FileEntry {
            path: "test.py".to_string(),
            content: "def hello(name, age):\n    return name".to_string(),
            size: 38,
            language: Some(SupportedLanguage::Python),
        };

        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "File:test.py".to_string(),
            label: NodeLabel::File,
            properties: NodeProperties {
                name: "test.py".to_string(),
                file_path: "test.py".to_string(),
                ..Default::default()
            },
        });

        let _extracted = parse_files(&mut graph, &[file], None).unwrap();

        let func_node = graph.get_node("Function:test.py:hello");
        assert!(func_node.is_some(), "Should create Function node for hello");
        assert_eq!(func_node.unwrap().properties.name, "hello");
    }

    #[test]
    fn test_parse_empty_files() {
        let extracted = parse_files(&mut KnowledgeGraph::new(), &[], None).unwrap();
        assert!(extracted.imports.is_empty());
        assert!(extracted.calls.is_empty());
    }

    #[test]
    fn test_parse_unsupported_language_skipped() {
        let file = FileEntry {
            path: "test.kt".to_string(),
            content: "fun main() {}".to_string(),
            size: 14,
            language: Some(SupportedLanguage::Kotlin),
        };

        let mut graph = KnowledgeGraph::new();
        let extracted = parse_files(&mut graph, &[file], None).unwrap();
        // Kotlin uses fallback grammar, so it's skipped
        assert!(extracted.imports.is_empty());
    }

    #[test]
    fn test_build_symbol_table() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "Function:src/main.ts:handleLogin".to_string(),
            label: NodeLabel::Function,
            properties: NodeProperties {
                name: "handleLogin".to_string(),
                file_path: "src/main.ts".to_string(),
                is_exported: Some(true),
                parameter_count: Some(2),
                ..Default::default()
            },
        });

        let mut table = SymbolTable::new();
        build_symbol_table(&graph, &mut table);

        let results = table.lookup_global("handleLogin");
        assert!(results.is_some());
        assert_eq!(results.unwrap().len(), 1);
    }
}
