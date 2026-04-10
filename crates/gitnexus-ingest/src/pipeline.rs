use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::pipeline::types::{PipelinePhase, PipelineProgress};
use gitnexus_core::symbol::SymbolTable;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::incremental;
use crate::phases;

pub type ProgressSender = mpsc::UnboundedSender<PipelineProgress>;

/// Pipeline configuration constants
pub const CHUNK_BYTE_BUDGET: usize = 20 * 1024 * 1024; // 20MB per chunk
pub const AST_CACHE_CAP: usize = 50;
pub const MAX_SYNTHETIC_BINDINGS_PER_FILE: usize = 1000;
pub const MIN_FILES_FOR_WORKERS: usize = 15;
pub const MIN_BYTES_FOR_WORKERS: usize = 512 * 1024;

/// Result of running the full pipeline.
pub struct PipelineResult {
    pub graph: KnowledgeGraph,
    pub repo_path: String,
    pub total_file_count: usize,
    pub community_count: usize,
    pub process_count: usize,
}

/// Options for pipeline execution.
#[derive(Debug, Default)]
pub struct PipelineOptions {
    pub force: bool,
    pub embeddings: bool,
    pub verbose: bool,
    pub skip_git: bool,
    /// If true and a manifest exists, use incremental indexing instead of full.
    pub incremental: bool,
}

/// Run the full ingestion pipeline on a repository.
pub async fn run_pipeline(
    repo_path: &Path,
    progress_tx: Option<ProgressSender>,
    options: PipelineOptions,
) -> Result<PipelineResult, crate::IngestError> {
    let pipeline_start = Instant::now();
    let repo_path_str = repo_path.display().to_string();

    // Helper to send progress
    let send_progress = |phase, percent, message: &str| {
        if let Some(tx) = &progress_tx {
            let _ = tx.send(PipelineProgress {
                phase,
                percent,
                message: message.to_string(),
                detail: None,
                stats: None,
            });
        }
    };

    // Incremental mode: if a manifest and graph snapshot exist, only re-parse changed files
    let storage_path = repo_path.join(".gitnexus");
    let snap_path = storage_path.join("graph.bin");
    let manifest_path = storage_path.join("manifest.json");
    if options.incremental && !options.force && snap_path.exists() && manifest_path.exists() {
        send_progress(PipelinePhase::Structure, 0.0, "Incremental update...");

        let mut graph = gitnexus_db::snapshot::load_snapshot(&snap_path)
            .map_err(|e| crate::IngestError::PhaseError {
                phase: "incremental".into(),
                message: format!("Failed to load snapshot: {e}"),
            })?;

        let inc_result = incremental::incremental_update(repo_path, &storage_path, &mut graph)?;

        if inc_result.total_changed() > 0 {
            // Capture the new manifest now; we will only persist it AFTER
            // the snapshot is durable, so a crash between the two writes
            // cannot leave the manifest ahead of the snapshot.
            let new_manifest_to_save = inc_result.new_manifest.clone();
            // Purge stale Community/Process nodes (and their `MemberOf` /
            // `StepInProcess` edges) from the previous run before re-running
            // detection. Louvain is deterministic in structure only up to a
            // renumbering, so `Community:community_0` from run N and run
            // N+1 may represent different clusters — without this cleanup,
            // the re-run's `add_node` would overwrite the node while
            // leaving stale membership edges pointing at it. The same
            // applies to `Process:process_*` nodes and `StepInProcess`
            // edges. `remove_nodes_by_label` cascades incident edges.
            use gitnexus_core::graph::types::NodeLabel;
            graph.remove_nodes_by_label(NodeLabel::Community);
            graph.remove_nodes_by_label(NodeLabel::Process);

            // Re-run community + process detection on the updated graph
            let community_count = phases::community::detect_communities(&mut graph)?;
            let process_count = phases::process::detect_processes(&mut graph)?;
            phases::dead_code::mark_dead_code(&mut graph);

            // Save updated snapshot to disk FIRST, then the manifest. This
            // ordering matters: if the snapshot save fails, we leave both the
            // old snapshot and old manifest on disk so the next run can simply
            // re-detect the same changes and try again. Persisting the
            // manifest before the snapshot would silently bake in a stale
            // graph (manifest claims everything is current, but the on-disk
            // snapshot doesn't reflect the changes we just applied).
            gitnexus_db::snapshot::save_snapshot(&graph, &snap_path)
                .map_err(|e| crate::IngestError::PhaseError {
                    phase: "incremental".into(),
                    message: format!("Failed to save snapshot: {e}"),
                })?;

            crate::manifest::save_manifest(&new_manifest_to_save, &manifest_path).map_err(|e| {
                crate::IngestError::PhaseError {
                    phase: "incremental".into(),
                    message: format!("Failed to save manifest: {e}"),
                }
            })?;

            tracing::info!(
                added = inc_result.added,
                modified = inc_result.modified,
                removed = inc_result.removed,
                total_duration_ms = pipeline_start.elapsed().as_millis() as u64,
                "Incremental pipeline complete"
            );

            send_progress(PipelinePhase::Complete, 100.0, &format!(
                "Incremental: +{} ~{} -{} files",
                inc_result.added, inc_result.modified, inc_result.removed,
            ));

            return Ok(PipelineResult {
                graph,
                repo_path: repo_path_str,
                total_file_count: inc_result.unchanged + inc_result.added + inc_result.modified,
                community_count,
                process_count,
            });
        } else {
            tracing::info!("No changes detected, graph is up to date");
            send_progress(PipelinePhase::Complete, 100.0, "No changes detected");

            let community_count = graph.iter_nodes()
                .filter(|n| n.label == gitnexus_core::graph::types::NodeLabel::Community)
                .count();
            let process_count = graph.iter_nodes()
                .filter(|n| n.label == gitnexus_core::graph::types::NodeLabel::Process)
                .count();

            return Ok(PipelineResult {
                graph,
                repo_path: repo_path_str,
                total_file_count: inc_result.unchanged,
                community_count,
                process_count,
            });
        }
    }

    send_progress(PipelinePhase::Structure, 0.0, "Scanning repository...");

    // Phase 1: Structure - walk filesystem
    let phase_start = Instant::now();
    let file_entries = phases::structure::walk_repository(repo_path)?;
    let total_files = file_entries.len();

    let mut graph = KnowledgeGraph::with_capacity(total_files * 5, total_files * 10);

    // Phase 1b: Create File/Folder nodes
    phases::structure::create_structure_nodes(&mut graph, &file_entries);
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "structure",
        duration_ms = duration.as_millis() as u64,
        files = total_files,
        "Phase complete"
    );
    send_progress(
        PipelinePhase::Structure,
        100.0,
        &format!("Found {total_files} files"),
    );

    // Phase 2: Parsing - extract symbols from AST
    send_progress(PipelinePhase::Parsing, 0.0, "Parsing files...");
    let phase_start = Instant::now();
    let extracted = phases::parsing::parse_files(&mut graph, &file_entries, progress_tx.as_ref())?;

    // Phase 2b: Detect component libraries from .csproj project files.
    // This runs after parsing to enrich the graph with NuGet package-level detections,
    // which have higher confidence than source-level pattern matching.
    let has_razor_files = file_entries
        .iter()
        .any(|f| f.path.ends_with(".cshtml") || f.path.ends_with(".razor"));
    if has_razor_files {
        phases::parsing::detect_csproj_components(&mut graph, repo_path);
    }

    // Build symbol table from graph
    let mut symbol_table = SymbolTable::new();
    phases::parsing::build_symbol_table(&graph, &mut symbol_table);
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "parsing",
        duration_ms = duration.as_millis() as u64,
        files = total_files,
        symbols = symbol_table.len(),
        "Phase complete"
    );
    send_progress(
        PipelinePhase::Parsing,
        100.0,
        &format!("Parsed {total_files} files"),
    );

    // Phase 3: Import resolution
    send_progress(PipelinePhase::Imports, 0.0, "Resolving imports...");
    let phase_start = Instant::now();
    let (import_map, named_import_map, package_map, module_alias_map) =
        phases::imports::resolve_imports(
            &mut graph,
            &file_entries,
            &extracted,
            &symbol_table,
        )?;
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "imports",
        duration_ms = duration.as_millis() as u64,
        import_edges = import_map.len(),
        "Phase complete"
    );
    send_progress(PipelinePhase::Imports, 100.0, "Imports resolved");

    // Phase 4: Call resolution
    send_progress(PipelinePhase::Calls, 0.0, "Resolving calls...");
    let phase_start = Instant::now();
    phases::calls::resolve_calls(
        &mut graph,
        &extracted,
        &symbol_table,
        &import_map,
        &named_import_map,
        &package_map,
        &module_alias_map,
        &file_entries,
    )?;
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "calls",
        duration_ms = duration.as_millis() as u64,
        total_edges = graph.relationship_count(),
        "Phase complete"
    );
    send_progress(PipelinePhase::Calls, 100.0, "Calls resolved");

    // Phase 5: Heritage
    send_progress(PipelinePhase::Heritage, 0.0, "Processing inheritance...");
    let phase_start = Instant::now();
    phases::heritage::process_heritage(
        &mut graph,
        &extracted,
        &symbol_table,
        &import_map,
        &named_import_map,
    )?;
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "heritage",
        duration_ms = duration.as_millis() as u64,
        total_edges = graph.relationship_count(),
        "Phase complete"
    );
    send_progress(PipelinePhase::Heritage, 100.0, "Heritage processed");

    // Phase 5b: ASP.NET MVC 5 / EF6 enrichment
    // Runs after heritage (needs class hierarchy) and before communities
    send_progress(
        PipelinePhase::AspNetMvc,
        0.0,
        "Detecting ASP.NET MVC patterns...",
    );
    let phase_start = Instant::now();
    let aspnet_stats = phases::aspnet_mvc::enrich_aspnet_mvc(&mut graph, &file_entries)?;
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "aspnet_mvc",
        duration_ms = duration.as_millis() as u64,
        controllers = aspnet_stats.controllers,
        actions = aspnet_stats.actions,
        entities = aspnet_stats.db_entities,
        views = aspnet_stats.views,
        "Phase complete"
    );
    if aspnet_stats.controllers > 0 || aspnet_stats.db_entities > 0 {
        send_progress(
            PipelinePhase::AspNetMvc,
            100.0,
            &format!(
                "ASP.NET: {} controllers, {} actions, {} entities, {} views",
                aspnet_stats.controllers,
                aspnet_stats.actions + aspnet_stats.api_endpoints,
                aspnet_stats.db_entities,
                aspnet_stats.views,
            ),
        );
    } else {
        send_progress(
            PipelinePhase::AspNetMvc,
            100.0,
            "No ASP.NET MVC patterns detected",
        );
    }

    // Phase 6a: Community detection
    send_progress(
        PipelinePhase::Communities,
        0.0,
        "Detecting communities...",
    );
    let phase_start = Instant::now();
    let community_count = phases::community::detect_communities(&mut graph)?;
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "communities",
        duration_ms = duration.as_millis() as u64,
        communities = community_count,
        "Phase complete"
    );
    send_progress(
        PipelinePhase::Communities,
        100.0,
        &format!("Found {community_count} communities"),
    );

    // Phase 6b: Process detection
    send_progress(
        PipelinePhase::Processes,
        0.0,
        "Tracing execution flows...",
    );
    let phase_start = Instant::now();
    let process_count = phases::process::detect_processes(&mut graph)?;
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "processes",
        duration_ms = duration.as_millis() as u64,
        processes = process_count,
        "Phase complete"
    );
    send_progress(
        PipelinePhase::Processes,
        100.0,
        &format!("Found {process_count} processes"),
    );

    // Phase 7: Dead code detection
    let phase_start = Instant::now();
    phases::dead_code::mark_dead_code(&mut graph);
    let duration = phase_start.elapsed();
    tracing::info!(
        phase = "dead_code",
        duration_ms = duration.as_millis() as u64,
        "Phase complete"
    );

    send_progress(PipelinePhase::Complete, 100.0, "Pipeline complete");

    tracing::info!(
        total_duration_ms = pipeline_start.elapsed().as_millis() as u64,
        total_files = total_files,
        total_nodes = graph.node_count(),
        total_edges = graph.relationship_count(),
        total_communities = community_count,
        total_processes = process_count,
        "Pipeline complete"
    );

    Ok(PipelineResult {
        graph,
        repo_path: repo_path_str,
        total_file_count: total_files,
        community_count,
        process_count,
    })
}

/// Topological level sort using Kahn's algorithm.
/// Groups files by dependency level for parallel processing.
pub fn topological_level_sort(import_map: &HashMap<String, HashSet<String>>) -> TopologicalResult {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut reverse_deps: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_files: HashSet<&str> = HashSet::new();

    // Build in-degree and reverse dependency maps
    for (file, imports) in import_map {
        all_files.insert(file);
        in_degree.entry(file).or_insert(0);
        for imported in imports {
            all_files.insert(imported);
            *in_degree.entry(file.as_str()).or_insert(0) += 1;
            reverse_deps
                .entry(imported.as_str())
                .or_default()
                .push(file);
        }
    }

    // Initialize with zero-degree nodes
    for file in &all_files {
        in_degree.entry(file).or_insert(0);
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(file, _)| *file)
        .collect();
    queue.sort(); // Deterministic ordering

    let mut levels: Vec<Vec<String>> = Vec::new();
    let mut processed = 0;

    while !queue.is_empty() {
        let current_level: Vec<String> = queue.iter().map(|s| s.to_string()).collect();
        let mut next_queue = Vec::new();

        for file in &queue {
            if let Some(dependents) = reverse_deps.get(file) {
                for dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            next_queue.push(*dep);
                        }
                    }
                }
            }
        }

        processed += current_level.len();
        levels.push(current_level);
        next_queue.sort();
        queue = next_queue;
    }

    // Remaining nodes are in cycles
    let cycle_count = all_files.len() - processed;
    if cycle_count > 0 {
        let cycle_files: Vec<String> = in_degree
            .iter()
            .filter(|(_, deg)| **deg > 0)
            .map(|(file, _)| file.to_string())
            .collect();
        levels.push(cycle_files);
    }

    TopologicalResult {
        levels,
        cycle_count,
    }
}

pub struct TopologicalResult {
    pub levels: Vec<Vec<String>>,
    pub cycle_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_linear_chain() {
        // a.ts -> b.ts -> c.ts (a imports b, b imports c)
        let mut import_map: HashMap<String, HashSet<String>> = HashMap::new();
        import_map.insert(
            "a.ts".to_string(),
            HashSet::from(["b.ts".to_string()]),
        );
        import_map.insert(
            "b.ts".to_string(),
            HashSet::from(["c.ts".to_string()]),
        );

        let result = topological_level_sort(&import_map);
        assert_eq!(result.cycle_count, 0);
        assert_eq!(result.levels.len(), 3);
        // c.ts has no imports, so it should be in level 0
        assert!(result.levels[0].contains(&"c.ts".to_string()));
        // b.ts depends on c.ts, so level 1
        assert!(result.levels[1].contains(&"b.ts".to_string()));
        // a.ts depends on b.ts, so level 2
        assert!(result.levels[2].contains(&"a.ts".to_string()));
    }

    #[test]
    fn test_topological_sort_parallel_deps() {
        // a.ts -> c.ts, b.ts -> c.ts (both a and b import c)
        let mut import_map: HashMap<String, HashSet<String>> = HashMap::new();
        import_map.insert(
            "a.ts".to_string(),
            HashSet::from(["c.ts".to_string()]),
        );
        import_map.insert(
            "b.ts".to_string(),
            HashSet::from(["c.ts".to_string()]),
        );

        let result = topological_level_sort(&import_map);
        assert_eq!(result.cycle_count, 0);
        assert_eq!(result.levels.len(), 2);
        // c.ts in level 0
        assert!(result.levels[0].contains(&"c.ts".to_string()));
        // a.ts and b.ts in level 1
        assert!(result.levels[1].contains(&"a.ts".to_string()));
        assert!(result.levels[1].contains(&"b.ts".to_string()));
    }

    #[test]
    fn test_topological_sort_cycle() {
        // a.ts -> b.ts -> a.ts (cycle)
        let mut import_map: HashMap<String, HashSet<String>> = HashMap::new();
        import_map.insert(
            "a.ts".to_string(),
            HashSet::from(["b.ts".to_string()]),
        );
        import_map.insert(
            "b.ts".to_string(),
            HashSet::from(["a.ts".to_string()]),
        );

        let result = topological_level_sort(&import_map);
        assert_eq!(result.cycle_count, 2);
        // The cycle files should still appear in the levels (as the last level)
        let all_files: HashSet<String> = result
            .levels
            .iter()
            .flat_map(|level| level.iter().cloned())
            .collect();
        assert!(all_files.contains("a.ts"));
        assert!(all_files.contains("b.ts"));
    }

    #[test]
    fn test_topological_sort_empty() {
        let import_map: HashMap<String, HashSet<String>> = HashMap::new();
        let result = topological_level_sort(&import_map);
        assert_eq!(result.cycle_count, 0);
        assert!(result.levels.is_empty());
    }

    #[test]
    fn test_topological_sort_no_deps() {
        // All files independent
        let mut import_map: HashMap<String, HashSet<String>> = HashMap::new();
        import_map.insert("a.ts".to_string(), HashSet::new());
        import_map.insert("b.ts".to_string(), HashSet::new());
        import_map.insert("c.ts".to_string(), HashSet::new());

        let result = topological_level_sort(&import_map);
        assert_eq!(result.cycle_count, 0);
        assert_eq!(result.levels.len(), 1);
        assert_eq!(result.levels[0].len(), 3);
    }

    #[test]
    fn test_topological_sort_diamond() {
        // Diamond: a -> b, a -> c, b -> d, c -> d
        let mut import_map: HashMap<String, HashSet<String>> = HashMap::new();
        import_map.insert(
            "a.ts".to_string(),
            HashSet::from(["b.ts".to_string(), "c.ts".to_string()]),
        );
        import_map.insert(
            "b.ts".to_string(),
            HashSet::from(["d.ts".to_string()]),
        );
        import_map.insert(
            "c.ts".to_string(),
            HashSet::from(["d.ts".to_string()]),
        );

        let result = topological_level_sort(&import_map);
        assert_eq!(result.cycle_count, 0);
        assert_eq!(result.levels.len(), 3);
        assert!(result.levels[0].contains(&"d.ts".to_string()));
        assert!(result.levels[1].contains(&"b.ts".to_string()));
        assert!(result.levels[1].contains(&"c.ts".to_string()));
        assert!(result.levels[2].contains(&"a.ts".to_string()));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use gitnexus_core::graph::NodeLabel;
    use std::fs;
    use std::path::PathBuf;

    fn create_test_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "gitnexus_test_{}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn test_pipeline_csharp_controller() {
        let dir = create_test_dir();
        let cs_file = dir.join("HomeController.cs");
        fs::write(
            &cs_file,
            r#"
using System.Web.Mvc;

public class HomeController : Controller
{
    public ActionResult Index()
    {
        return View();
    }

    [HttpPost]
    public ActionResult Login(string username, string password)
    {
        return RedirectToAction("Index");
    }
}
"#,
        )
        .unwrap();

        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(
            result.is_ok(),
            "Pipeline should succeed: {:?}",
            result.err()
        );

        let result = result.unwrap();
        let graph = &result.graph;

        // Verify nodes exist
        assert!(graph.node_count() > 0, "Graph should have nodes");

        // Check for Class or Controller nodes named HomeController
        let has_class = graph.iter_nodes().any(|n| {
            n.properties.name == "HomeController"
                && (n.label == NodeLabel::Class || n.label == NodeLabel::Controller)
        });
        assert!(has_class, "Should detect HomeController class");

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pipeline_javascript_functions() {
        let dir = create_test_dir();
        let js_file = dir.join("app.js");
        fs::write(
            &js_file,
            r#"
function greet(name) {
    return "Hello, " + name;
}

function processData(items) {
    return items.map(item => greet(item.name));
}

module.exports = { greet, processData };
"#,
        )
        .unwrap();

        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());

        let result = result.unwrap();
        let graph = &result.graph;

        // Should have Function nodes
        let functions: Vec<&str> = graph
            .iter_nodes()
            .filter(|n| n.label == NodeLabel::Function)
            .map(|n| n.properties.name.as_str())
            .collect();

        assert!(
            functions.contains(&"greet"),
            "Should detect greet function, found: {:?}",
            functions
        );
        assert!(
            functions.contains(&"processData"),
            "Should detect processData function, found: {:?}",
            functions
        );

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pipeline_empty_project() {
        let dir = create_test_dir();
        // Empty directory -- should not crash
        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(result.is_ok(), "Empty project should not crash");

        let result = result.unwrap();
        // Empty project may have 0 nodes (no source files to parse)
        // The key assertion is that it didn't error out
        assert_eq!(
            result.total_file_count, 0,
            "Empty project should report 0 files"
        );

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pipeline_error_recovery() {
        let dir = create_test_dir();

        // One valid file
        fs::write(dir.join("valid.js"), "function hello() { return 42; }").unwrap();

        // One malformed file (binary content)
        fs::write(dir.join("corrupt.js"), [0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(result.is_ok(), "Pipeline should recover from bad files");

        let result = result.unwrap();
        // Valid file should still be processed
        assert!(
            result.graph.node_count() > 0,
            "Valid file nodes should exist despite corrupt file"
        );

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pipeline_multiple_languages() {
        let dir = create_test_dir();

        fs::write(dir.join("app.js"), "function jsFunc() {}").unwrap();
        fs::write(dir.join("main.py"), "def py_func():\n    pass").unwrap();
        fs::write(dir.join("lib.rs"), "pub fn rust_func() {}").unwrap();

        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(
            result.is_ok(),
            "Multi-language pipeline failed: {:?}",
            result.err()
        );

        let result = result.unwrap();
        let graph = &result.graph;

        // Should have detected multiple languages
        let languages: std::collections::HashSet<_> = graph
            .iter_nodes()
            .filter_map(|n| n.properties.language)
            .collect();

        assert!(
            languages.len() >= 2,
            "Should detect at least 2 languages, found: {:?}",
            languages
        );

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pipeline_python_classes() {
        let dir = create_test_dir();
        fs::write(
            dir.join("models.py"),
            r#"
class Animal:
    def __init__(self, name):
        self.name = name

    def speak(self):
        pass

class Dog(Animal):
    def speak(self):
        return "Woof!"
"#,
        )
        .unwrap();

        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());

        let result = result.unwrap();
        let graph = &result.graph;

        let classes: Vec<&str> = graph
            .iter_nodes()
            .filter(|n| n.label == NodeLabel::Class)
            .map(|n| n.properties.name.as_str())
            .collect();

        assert!(
            classes.contains(&"Animal"),
            "Should detect Animal class, found: {:?}",
            classes
        );
        assert!(
            classes.contains(&"Dog"),
            "Should detect Dog class, found: {:?}",
            classes
        );

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pipeline_file_count_matches() {
        let dir = create_test_dir();
        fs::write(dir.join("a.js"), "var x = 1;").unwrap();
        fs::write(dir.join("b.js"), "var y = 2;").unwrap();
        fs::write(dir.join("c.py"), "z = 3").unwrap();

        let result = run_pipeline(&dir, None, PipelineOptions::default()).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(
            result.total_file_count, 3,
            "Should report exactly 3 files"
        );

        cleanup(&dir);
    }
}
