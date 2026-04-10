use std::collections::{HashMap, HashSet};

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::id::generate_id;
use gitnexus_core::resolution::types::*;
use gitnexus_core::symbol::SymbolTable;

use gitnexus_lang::import_resolvers::types::{ImportConfigs, ImportResult, ResolveCtx, SuffixIndex};
use gitnexus_lang::registry::get_provider;

use crate::phases::parsing::ExtractedData;
use crate::phases::structure::FileEntry;
use crate::IngestError;

/// Resolve all imports and build dependency maps.
pub fn resolve_imports(
    graph: &mut KnowledgeGraph,
    files: &[FileEntry],
    extracted: &ExtractedData,
    _symbol_table: &SymbolTable,
) -> Result<(ImportMap, NamedImportMap, PackageMap, ModuleAliasMap), IngestError> {
    // Build file path sets and suffix index
    let all_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    let all_set: HashSet<String> = all_paths.iter().cloned().collect();
    // Lower-cased copy used by case-insensitive resolvers (e.g. Windows where
    // `Controllers/HomeController.cs` may be referenced as `controllers/...`).
    // The SuffixIndex itself stores both case-sensitive and case-insensitive
    // entries internally, so it builds from `(all_paths, all_paths)` as before.
    // The ResolveCtx now exposes a properly lowercased `normalized_file_list`
    // so resolvers that key off it directly behave correctly.
    let normalized_paths: Vec<String> = all_paths.iter().map(|p| p.to_lowercase()).collect();
    let suffix_index = SuffixIndex::build(&all_paths, &all_paths);
    let configs = ImportConfigs::default();

    let ctx = ResolveCtx {
        all_file_paths: &all_set,
        all_file_list: &all_paths,
        normalized_file_list: &normalized_paths,
        suffix_index: &suffix_index,
        configs: &configs,
    };

    let mut import_map: ImportMap = HashMap::new();
    let mut named_import_map: NamedImportMap = HashMap::new();
    let mut package_map: PackageMap = HashMap::new();
    let module_alias_map: ModuleAliasMap = HashMap::new();

    // Process each extracted import
    for imp in &extracted.imports {
        let lang =
            gitnexus_core::config::languages::SupportedLanguage::from_filename(&imp.file_path);
        let lang = match lang {
            Some(l) => l,
            None => continue,
        };
        let provider = get_provider(lang);

        // Resolve the import path
        let result = provider.resolve_import(&imp.raw_import_path, &imp.file_path, &ctx);

        match result {
            ImportResult::Files(resolved_files) => {
                for resolved in &resolved_files {
                    import_map
                        .entry(imp.file_path.clone())
                        .or_default()
                        .insert(resolved.clone());

                    // Create IMPORTS edge
                    let source_id = generate_id("File", &imp.file_path);
                    let target_id = generate_id("File", resolved);
                    let edge_id = format!("imports_{}_{}", source_id, target_id);
                    graph.add_relationship(GraphRelationship {
                        id: edge_id,
                        source_id,
                        target_id,
                        rel_type: RelationshipType::Imports,
                        confidence: 0.9,
                        reason: "resolved".to_string(),
                        step: None,
                    });
                }

                // Extract named bindings
                if let Some(bindings) = provider.extract_named_bindings(&imp.raw_import_path) {
                    for binding in bindings {
                        if let Some(first_file) = resolved_files.first() {
                            named_import_map
                                .entry(imp.file_path.clone())
                                .or_default()
                                .insert(
                                    binding.local.clone(),
                                    NamedImportBinding {
                                        source_path: first_file.clone(),
                                        exported_name: binding.exported,
                                    },
                                );
                        }
                    }
                }
            }
            ImportResult::Package {
                files: pkg_files,
                dir_suffix,
            } => {
                for f in &pkg_files {
                    import_map
                        .entry(imp.file_path.clone())
                        .or_default()
                        .insert(f.clone());
                }
                package_map
                    .entry(imp.file_path.clone())
                    .or_default()
                    .insert(dir_suffix);
            }
            ImportResult::Unresolved => {
                // Skip unresolved imports
            }
        }
    }

    Ok((import_map, named_import_map, package_map, module_alias_map))
}

/// Build a reverse import map (imported_file -> set of files that import it).
#[allow(dead_code)]
pub fn build_reverse_import_map(import_map: &ImportMap) -> HashMap<String, HashSet<String>> {
    let mut reverse: HashMap<String, HashSet<String>> = HashMap::new();
    for (file, imports) in import_map {
        for imported in imports {
            reverse
                .entry(imported.clone())
                .or_default()
                .insert(file.clone());
        }
    }
    reverse
}
