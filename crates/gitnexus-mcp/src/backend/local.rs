//! Local backend: resolves repos from the registry and dispatches tool calls
//! to the appropriate handler.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::{json, Value};
use std::time::Instant;
use tracing::info;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::storage::repo_manager::{self, RegistryEntry};
use gitnexus_db::pool::ConnectionPool;
use gitnexus_db::query;

use crate::error::{McpError, Result};
use crate::hints;

/// Local MCP backend: manages connections and dispatches tool calls.
pub struct LocalBackend {
    pool: ConnectionPool,
    registry: Vec<RegistryEntry>,
    /// Cached snapshots keyed by absolute snapshot path, shared across tool calls.
    snapshot_cache: HashMap<PathBuf, Arc<KnowledgeGraph>>,
}

impl LocalBackend {
    /// Create a new LocalBackend.
    pub fn new() -> Self {
        Self {
            pool: ConnectionPool::new(),
            registry: Vec::new(),
            snapshot_cache: HashMap::new(),
        }
    }

    /// Load a snapshot from disk, returning a cached `Arc<KnowledgeGraph>` if already loaded.
    fn load_cached_snapshot(&mut self, snap_path: &std::path::Path) -> Result<Arc<KnowledgeGraph>> {
        let key = snap_path.to_path_buf();
        if let Some(cached) = self.snapshot_cache.get(&key) {
            return Ok(Arc::clone(cached));
        }
        let graph = gitnexus_db::snapshot::load_snapshot(snap_path)
            .map_err(|e| McpError::Internal(format!("Failed to load graph: {e}")))?;
        let arc = Arc::new(graph);
        self.snapshot_cache.insert(key, Arc::clone(&arc));
        Ok(arc)
    }

    /// Initialize the backend: load the global registry and discover repos.
    pub fn init(&mut self) -> Result<()> {
        self.registry = repo_manager::read_registry().map_err(McpError::Core)?;
        info!("LocalBackend: loaded {} repos from registry", self.registry.len());
        Ok(())
    }

    /// Get the current registry.
    pub fn registry(&self) -> &[RegistryEntry] {
        &self.registry
    }

    /// Resolve a repository by name or path.
    ///
    /// If `repo` is None and there's exactly one registered repo, uses that.
    /// Otherwise searches by name or path (case-insensitive).
    pub fn resolve_repo(&self, repo: Option<&str>) -> Result<&RegistryEntry> {
        match repo {
            None => {
                if self.registry.len() == 1 {
                    Ok(&self.registry[0])
                } else if self.registry.is_empty() {
                    Err(McpError::RepoNotFound(
                        "No repositories indexed. Run `gitnexus analyze` first.".into(),
                    ))
                } else {
                    Err(McpError::InvalidArguments {
                        tool: "resolve_repo".into(),
                        reason: format!(
                            "Multiple repos indexed ({}). Specify 'repo' parameter.",
                            self.registry.len()
                        ),
                    })
                }
            }
            Some(name_or_path) => {
                let lower = name_or_path.to_lowercase();
                self.registry
                    .iter()
                    .find(|e| {
                        e.name.to_lowercase() == lower
                            || e.path.to_lowercase() == lower
                            || e.path.to_lowercase().ends_with(&lower)
                    })
                    .ok_or_else(|| McpError::RepoNotFound(name_or_path.to_string()))
            }
        }
    }

    /// Dispatch a tool call by name with the given arguments.
    pub async fn call_tool(&mut self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "list_repos" => self.tool_list_repos(),
            "query" => self.tool_query(args).await,
            "context" => self.tool_context(args).await,
            "impact" => self.tool_impact(args).await,
            "detect_changes" => self.tool_detect_changes(args).await,
            "rename" => self.tool_rename(args).await,
            "cypher" => self.tool_cypher(args).await,
            "hotspots" => self.tool_hotspots(args).await,
            "coupling" => self.tool_coupling(args).await,
            "ownership" => self.tool_ownership(args).await,
            "coverage" => self.tool_coverage(args).await,
            "diagram" => self.tool_diagram(args).await,
            "report" => self.tool_report(args).await,
            _ => Err(McpError::UnknownTool(name.to_string())),
        }
    }

    // ─── Tool Implementations ───────────────────────────────────────

    fn tool_list_repos(&self) -> Result<Value> {
        let repos: Vec<Value> = self
            .registry
            .iter()
            .map(|e| {
                let mut obj = json!({
                    "name": e.name,
                    "path": e.path,
                    "indexedAt": e.indexed_at,
                    "lastCommit": e.last_commit,
                });
                if let Some(stats) = &e.stats {
                    obj["stats"] = serde_json::to_value(stats).unwrap_or_default();
                }
                obj
            })
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&repos).unwrap_or_else(|_| "[]".to_string())
            }],
            "_meta": {
                "hint": hints::hint_for("list_repos")
            }
        }))
    }

    async fn tool_query(&self, args: &Value) -> Result<Value> {
        let query_text = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "query".into(),
                reason: "Missing required 'query' parameter".into(),
            })?;

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let entry = self.resolve_repo(repo_name)?;
        let db_path = std::path::Path::new(&entry.storage_path).join("db");

        let adapter = self.pool.get_or_open(&db_path).map_err(McpError::Db)?;

        // Use best available search strategy (BM25, or hybrid RRF when embeddings are available)
        let results = gitnexus_search::search(&adapter, query_text, limit)
            .map_err(McpError::Db)?;

        let results_json: Vec<Value> = results
            .iter()
            .map(|r| serde_json::to_value(r).unwrap_or_default())
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results_json).unwrap_or_else(|_| "[]".to_string())
            }],
            "_meta": {
                "hint": hints::hint_for("query"),
                "resultCount": results.len()
            }
        }))
    }

    async fn tool_context(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "context".into(),
                reason: "Missing required 'name' parameter".into(),
            })?;

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let uid = args.get("uid").and_then(|v| v.as_str());
        let file_filter = args.get("file").and_then(|v| v.as_str());

        let entry = self.resolve_repo(repo_name)?;
        let db_path = std::path::Path::new(&entry.storage_path).join("db");
        let adapter = self.pool.get_or_open(&db_path).map_err(McpError::Db)?;

        // Build context query
        let escaped_name = query::escape_cypher_string(name);
        let cypher = if let Some(uid_val) = uid {
            let escaped_uid = query::escape_cypher_string(uid_val);
            format!(
                "MATCH (n) WHERE n.id = '{escaped_uid}' \
                 OPTIONAL MATCH (n)-[r]->(m) \
                 OPTIONAL MATCH (p)-[r2]->(n) \
                 RETURN n, collect(DISTINCT {{rel: type(r), target: m.name, targetId: m.id}}) AS outgoing, \
                        collect(DISTINCT {{rel: type(r2), source: p.name, sourceId: p.id}}) AS incoming"
            )
        } else {
            let file_clause = file_filter
                .map(|f| {
                    let ef = query::escape_cypher_string(f);
                    format!(" AND n.filePath = '{ef}'")
                })
                .unwrap_or_default();
            format!(
                "MATCH (n) WHERE n.name = '{escaped_name}'{file_clause} \
                 OPTIONAL MATCH (n)-[r]->(m) \
                 OPTIONAL MATCH (p)-[r2]->(n) \
                 RETURN n, collect(DISTINCT {{rel: type(r), target: m.name, targetId: m.id}}) AS outgoing, \
                        collect(DISTINCT {{rel: type(r2), source: p.name, sourceId: p.id}}) AS incoming"
            )
        };

        let query_start = Instant::now();
        let mut results = adapter.execute_query(&cypher).map_err(McpError::Db)?;
        let duration = query_start.elapsed();

        if duration.as_secs() > 5 {
            tracing::warn!(
                query = %cypher,
                duration_ms = duration.as_millis() as u64,
                "Slow context query detected"
            );
        }

        let total_rows = results.len();
        if total_rows > 1000 {
            tracing::warn!(
                total_rows = total_rows,
                "Context query returned {} results, truncating to 1000",
                total_rows
            );
            results.truncate(1000);
        }

        Ok(json!({
            "content": [{
                "type": "text",
                "text": query::format_query_result(&results)
            }],
            "_meta": {
                "hint": hints::hint_for("context"),
                "durationMs": duration.as_millis() as u64
            }
        }))
    }

    async fn tool_impact(&self, args: &Value) -> Result<Value> {
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "impact".into(),
                reason: "Missing required 'target' parameter".into(),
            })?;

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let direction = args
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("both");
        let max_depth = args
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        let entry = self.resolve_repo(repo_name)?;
        let db_path = std::path::Path::new(&entry.storage_path).join("db");
        let adapter = self.pool.get_or_open(&db_path).map_err(McpError::Db)?;

        let escaped = query::escape_cypher_string(target);

        // Build blast radius queries based on direction
        let mut all_results: Vec<Value> = Vec::new();

        if direction == "upstream" || direction == "both" {
            // Find callers (upstream)
            let upstream_query = format!(
                "MATCH (caller)-[r:CALLS|USES|IMPORTS*1..{max_depth}]->(target) \
                 WHERE target.name = '{escaped}' OR target.id = '{escaped}' \
                 RETURN DISTINCT caller.name AS name, caller.id AS id, \
                        caller.filePath AS filePath, labels(caller) AS label, \
                        length(r) AS depth \
                 ORDER BY depth ASC"
            );
            if let Ok(rows) = adapter.execute_query(&upstream_query) {
                all_results.push(json!({
                    "direction": "upstream",
                    "callers": rows
                }));
            }
        }

        if direction == "downstream" || direction == "both" {
            // Find callees (downstream)
            let downstream_query = format!(
                "MATCH (target)-[r:CALLS|USES|IMPORTS*1..{max_depth}]->(callee) \
                 WHERE target.name = '{escaped}' OR target.id = '{escaped}' \
                 RETURN DISTINCT callee.name AS name, callee.id AS id, \
                        callee.filePath AS filePath, labels(callee) AS label, \
                        length(r) AS depth \
                 ORDER BY depth ASC"
            );
            if let Ok(rows) = adapter.execute_query(&downstream_query) {
                all_results.push(json!({
                    "direction": "downstream",
                    "callees": rows
                }));
            }
        }

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&all_results).unwrap_or_else(|_| "[]".to_string())
            }],
            "_meta": {
                "hint": hints::hint_for("impact"),
                "target": target,
                "direction": direction,
                "maxDepth": max_depth
            }
        }))
    }

    async fn tool_detect_changes(&self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let entry = self.resolve_repo(repo_name)?;
        let repo_path = std::path::Path::new(&entry.path);

        // Use git to detect uncommitted changes
        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .current_dir(repo_path)
            .output();

        let changed_files: Vec<String> = match output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|l| l.to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            }
            _ => Vec::new(),
        };

        // Also get untracked files
        let untracked_output = std::process::Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(repo_path)
            .output();

        let untracked_files: Vec<String> = match untracked_output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|l| l.to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            }
            _ => Vec::new(),
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "modified": changed_files,
                    "untracked": untracked_files,
                    "totalChanges": changed_files.len() + untracked_files.len()
                })).unwrap_or_default()
            }],
            "_meta": {
                "hint": hints::hint_for("detect_changes")
            }
        }))
    }

    async fn tool_rename(&self, args: &Value) -> Result<Value> {
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "rename".into(),
                reason: "Missing required 'target' parameter".into(),
            })?;

        let new_name = args
            .get("new_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "rename".into(),
                reason: "Missing required 'new_name' parameter".into(),
            })?;

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let entry = self.resolve_repo(repo_name)?;
        let db_path = std::path::Path::new(&entry.storage_path).join("db");
        let adapter = self.pool.get_or_open(&db_path).map_err(McpError::Db)?;

        let escaped = query::escape_cypher_string(target);

        // Find all references to the target symbol
        let refs_query = format!(
            "MATCH (n)-[r]->(target) \
             WHERE target.name = '{escaped}' OR target.id = '{escaped}' \
             RETURN n.name AS referenceName, n.id AS referenceId, \
                    n.filePath AS filePath, n.startLine AS startLine, \
                    type(r) AS relationType"
        );

        let references = adapter.execute_query(&refs_query).map_err(McpError::Db)?;

        // Also find the target node itself
        let target_query = format!(
            "MATCH (n) WHERE n.name = '{escaped}' OR n.id = '{escaped}' \
             RETURN n.name AS name, n.id AS id, n.filePath AS filePath, \
                    n.startLine AS startLine, n.endLine AS endLine"
        );

        let target_nodes = adapter.execute_query(&target_query).map_err(McpError::Db)?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "target": target,
                    "newName": new_name,
                    "targetNodes": target_nodes,
                    "references": references,
                    "totalReferences": references.len(),
                    "filesAffected": references.iter()
                        .filter_map(|r| r.get("filePath").and_then(|v| v.as_str()))
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                })).unwrap_or_default()
            }],
            "_meta": {
                "hint": hints::hint_for("rename")
            }
        }))
    }

    async fn tool_cypher(&self, args: &Value) -> Result<Value> {
        let cypher = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "cypher".into(),
                reason: "Missing required 'query' parameter".into(),
            })?;

        // Reject write queries
        if query::is_write_query(cypher) {
            return Err(McpError::WriteQueryRejected);
        }

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let entry = self.resolve_repo(repo_name)?;
        let db_path = std::path::Path::new(&entry.storage_path).join("db");
        let adapter = self.pool.get_or_open(&db_path).map_err(McpError::Db)?;

        let query_start = Instant::now();
        let mut results = adapter.execute_query(cypher).map_err(McpError::Db)?;
        let duration = query_start.elapsed();

        if duration.as_secs() > 5 {
            tracing::warn!(
                query = %cypher,
                duration_ms = duration.as_millis() as u64,
                "Slow Cypher query detected"
            );
        }

        let total_rows = results.len();
        let truncated = total_rows > 1000;
        if truncated {
            tracing::warn!(
                total_rows = total_rows,
                "Query returned {} results, truncating to 1000",
                total_rows
            );
            results.truncate(1000);
        }

        Ok(json!({
            "content": [{
                "type": "text",
                "text": query::format_query_result(&results)
            }],
            "_meta": {
                "hint": hints::hint_for("cypher"),
                "rowCount": results.len(),
                "totalRows": total_rows,
                "truncated": truncated,
                "durationMs": duration.as_millis() as u64
            }
        }))
    }

    async fn tool_hotspots(&self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let since_days = args
            .get("since_days")
            .and_then(|v| v.as_u64())
            .unwrap_or(90) as u32;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let entry = self.resolve_repo(repo_name)?;
        let repo_path = std::path::Path::new(&entry.path);

        let mut hotspots = gitnexus_git::hotspots::analyze_hotspots(repo_path, since_days)
            .map_err(|e| McpError::Internal(e.to_string()))?;

        hotspots.truncate(limit);

        let results: Vec<Value> = hotspots
            .iter()
            .map(|h| serde_json::to_value(h).unwrap_or_default())
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string())
            }],
            "_meta": {
                "hint": hints::hint_for("hotspots"),
                "resultCount": results.len(),
                "sinceDays": since_days
            }
        }))
    }

    async fn tool_coupling(&self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let min_shared = args
            .get("min_shared")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let entry = self.resolve_repo(repo_name)?;
        let repo_path = std::path::Path::new(&entry.path);

        let mut couplings = gitnexus_git::coupling::analyze_coupling(repo_path, min_shared, Some(180))
            .map_err(|e| McpError::Internal(e.to_string()))?;

        couplings.truncate(limit);

        let results: Vec<Value> = couplings
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string())
            }],
            "_meta": {
                "hint": hints::hint_for("coupling"),
                "resultCount": results.len(),
                "minShared": min_shared
            }
        }))
    }

    async fn tool_ownership(&self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let entry = self.resolve_repo(repo_name)?;
        let repo_path = std::path::Path::new(&entry.path);

        let mut ownerships = gitnexus_git::ownership::analyze_ownership(repo_path)
            .map_err(|e| McpError::Internal(e.to_string()))?;

        ownerships.truncate(limit);

        let results: Vec<Value> = ownerships
            .iter()
            .map(|o| serde_json::to_value(o).unwrap_or_default())
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string())
            }],
            "_meta": {
                "hint": hints::hint_for("ownership"),
                "resultCount": results.len()
            }
        }))
    }

    async fn tool_coverage(&mut self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let target = args.get("target").and_then(|v| v.as_str());

        let snap_path = {
            let entry = self.resolve_repo(repo_name)?;
            std::path::Path::new(&entry.storage_path).join("graph.bin")
        };

        let graph = self.load_cached_snapshot(&snap_path)?;

        // Build incoming calls index
        let mut incoming_calls: HashMap<String, Vec<String>> = HashMap::new();
        let mut class_methods: HashMap<String, Vec<String>> = HashMap::new();

        for rel in graph.iter_relationships() {
            match rel.rel_type {
                RelationshipType::Calls | RelationshipType::CallsAction => {
                    incoming_calls
                        .entry(rel.target_id.clone())
                        .or_default()
                        .push(rel.source_id.clone());
                }
                RelationshipType::HasMethod => {
                    class_methods
                        .entry(rel.source_id.clone())
                        .or_default()
                        .push(rel.target_id.clone());
                }
                _ => {}
            }
        }

        let result = if let Some(target_name) = target {
            // Single class coverage
            let target_lower = target_name.to_lowercase();
            let class_node = graph.iter_nodes().find(|n| {
                n.properties.name.to_lowercase() == target_lower
                    && matches!(n.label, NodeLabel::Class | NodeLabel::Service | NodeLabel::Controller)
            });

            if let Some(cn) = class_node {
                let methods = class_methods.get(&cn.id).cloned().unwrap_or_default();
                let total = methods.len();
                let traced = methods.iter().filter(|mid| {
                    graph.get_node(mid).map_or(false, |n| n.properties.is_traced == Some(true))
                }).count();
                let dead = methods.iter().filter(|mid| {
                    !incoming_calls.contains_key(*mid)
                }).count();

                json!({
                    "class": cn.properties.name,
                    "totalMethods": total,
                    "tracedMethods": traced,
                    "deadCodeCandidates": dead,
                    "coveragePct": if total > 0 { (traced as f64 / total as f64 * 100.0).round() } else { 0.0 }
                })
            } else {
                json!({"error": format!("Class '{}' not found", target_name)})
            }
        } else {
            // Global coverage
            let all_methods: Vec<_> = graph.iter_nodes()
                .filter(|n| n.label == NodeLabel::Method || n.label == NodeLabel::Function)
                .collect();
            let total = all_methods.len();
            let traced = all_methods.iter().filter(|n| n.properties.is_traced == Some(true)).count();
            let dead = all_methods.iter().filter(|n| !incoming_calls.contains_key(&n.id)).count();

            json!({
                "totalMethods": total,
                "tracedMethods": traced,
                "deadCodeCandidates": dead,
                "coveragePct": if total > 0 { (traced as f64 / total as f64 * 100.0).round() } else { 0.0 }
            })
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result).unwrap_or_default()
            }],
            "_meta": {
                "hint": hints::hint_for("coverage")
            }
        }))
    }

    async fn tool_diagram(&mut self, args: &Value) -> Result<Value> {
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "diagram".into(),
                reason: "Missing required 'target' parameter".into(),
            })?;

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let diagram_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("flowchart");

        let snap_path = {
            let entry = self.resolve_repo(repo_name)?;
            std::path::Path::new(&entry.storage_path).join("graph.bin")
        };

        let graph = self.load_cached_snapshot(&snap_path)?;

        // Find the target symbol
        let target_lower = target.to_lowercase();
        let mut candidates: Vec<_> = graph.iter_nodes()
            .filter(|n| n.properties.name.to_lowercase() == target_lower)
            .collect();
        candidates.sort_by_key(|n| match n.label {
            NodeLabel::Controller => 0,
            NodeLabel::Class => 1,
            NodeLabel::Service => 2,
            NodeLabel::Interface => 3,
            _ => 10,
        });

        let start_node = candidates.first().ok_or_else(|| {
            McpError::Internal(format!("Symbol '{}' not found", target))
        })?;

        // Generate simple flowchart showing calls from this symbol's methods
        let mut lines = Vec::new();
        lines.push(format!("graph TD"));

        let node_id = &start_node.id;
        let node_name = &start_node.properties.name;
        lines.push(format!("    {}[\"{}\"]", sanitize_mermaid_id(node_id), node_name));

        // Pre-build set of child IDs to avoid O(E^2) inner loop
        let child_ids: std::collections::HashSet<&str> = graph.iter_relationships()
            .filter(|r| r.source_id == *node_id && matches!(r.rel_type, RelationshipType::HasMethod | RelationshipType::HasAction))
            .map(|r| r.target_id.as_str())
            .collect();

        // Find methods and their calls
        for rel in graph.iter_relationships() {
            if &rel.source_id == node_id || child_ids.contains(rel.source_id.as_str()) {
                if matches!(rel.rel_type, RelationshipType::Calls | RelationshipType::CallsAction | RelationshipType::CallsService) {
                    if let Some(target_node) = graph.get_node(&rel.target_id) {
                        let src_name = graph.get_node(&rel.source_id)
                            .map(|n| n.properties.name.as_str())
                            .unwrap_or("?");
                        lines.push(format!(
                            "    {}[\"{}\"] --> {}[\"{}\"]",
                            sanitize_mermaid_id(&rel.source_id), src_name,
                            sanitize_mermaid_id(&rel.target_id), target_node.properties.name,
                        ));
                    }
                }
            }
        }

        if lines.len() == 2 {
            // No calls found, show class members instead
            for rel in graph.iter_relationships() {
                if &rel.source_id == node_id && matches!(rel.rel_type, RelationshipType::HasMethod) {
                    if let Some(member) = graph.get_node(&rel.target_id) {
                        lines.push(format!(
                            "    {} --> {}[\"{}\"]",
                            sanitize_mermaid_id(node_id),
                            sanitize_mermaid_id(&rel.target_id),
                            member.properties.name,
                        ));
                    }
                }
            }
        }

        let mermaid = lines.join("\n");

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("```mermaid\n{}\n```", mermaid)
            }],
            "_meta": {
                "hint": hints::hint_for("diagram"),
                "diagramType": diagram_type,
                "target": target
            }
        }))
    }

    async fn tool_report(&mut self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let (repo_path, snap_path) = {
            let entry = self.resolve_repo(repo_name)?;
            (
                std::path::PathBuf::from(&entry.path),
                std::path::Path::new(&entry.storage_path).join("graph.bin"),
            )
        };
        let repo_path = repo_path.as_path();

        let graph = self.load_cached_snapshot(&snap_path)?;

        let node_count = graph.iter_nodes().count();
        let edge_count = graph.iter_relationships().count();
        let file_count = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::File)
            .count();
        let density = if node_count > 0 { edge_count as f64 / node_count as f64 } else { 0.0 };

        // Git analytics
        let hotspots = gitnexus_git::hotspots::analyze_hotspots(repo_path, 90).unwrap_or_default();
        let couplings = gitnexus_git::coupling::analyze_coupling(repo_path, 3, Some(180)).unwrap_or_default();
        let ownerships = gitnexus_git::ownership::analyze_ownership(repo_path).unwrap_or_default();

        // Compute health score (0-100); healthy projects score ~85-95
        let mut score: f64 = 100.0;
        let hot_files = hotspots.iter().filter(|h| h.score > 0.7).count();
        score -= (hot_files as f64) * 3.0;
        let strong_couples = couplings.iter().filter(|c| c.coupling_strength > 0.7).count();
        score -= (strong_couples as f64) * 2.0;
        let orphan_files = ownerships.iter().filter(|o| o.ownership_pct < 50.0).count();
        score -= (orphan_files as f64) * 0.5;
        score = score.clamp(0.0, 100.0);

        let grade = match score as u32 {
            90..=100 => "A",
            75..=89 => "B",
            60..=74 => "C",
            40..=59 => "D",
            _ => "E",
        };

        let report = json!({
            "grade": grade,
            "score": (score * 10.0).round() / 10.0,
            "graph": {
                "nodes": node_count,
                "edges": edge_count,
                "files": file_count,
                "density": (density * 100.0).round() / 100.0,
            },
            "hotspots": {
                "total": hotspots.len(),
                "critical": hot_files,
                "top3": hotspots.iter().take(3).map(|h| json!({
                    "path": h.path,
                    "score": (h.score * 100.0).round() / 100.0,
                    "commits": h.commit_count,
                })).collect::<Vec<_>>(),
            },
            "coupling": {
                "total": couplings.len(),
                "strong": strong_couples,
            },
            "ownership": {
                "totalFiles": ownerships.len(),
                "orphanedFiles": orphan_files,
            }
        });

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&report).unwrap_or_default()
            }],
            "_meta": {
                "hint": hints::hint_for("report")
            }
        }))
    }
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace([':', '/', '.', ' ', '<', '>', '(', ')', '{', '}'], "_")
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_backend_new() {
        let backend = LocalBackend::new();
        assert!(backend.registry().is_empty());
    }

    #[test]
    fn test_resolve_repo_empty() {
        let backend = LocalBackend::new();
        let result = backend.resolve_repo(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_repo_not_found() {
        let backend = LocalBackend::new();
        let result = backend.resolve_repo(Some("nonexistent"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let mut backend = LocalBackend::new();
        let result = backend.call_tool("nonexistent", &json!({})).await;
        assert!(matches!(result, Err(McpError::UnknownTool(_))));
    }

    #[tokio::test]
    async fn test_cypher_write_rejected() {
        let mut backend = LocalBackend::new();
        // Initialize with a dummy entry so resolve_repo works
        backend.registry.push(RegistryEntry {
            name: "test".to_string(),
            path: "/tmp/test".to_string(),
            storage_path: "/tmp/test/.gitnexus".to_string(),
            indexed_at: "2024-01-01".to_string(),
            last_commit: "abc123".to_string(),
            stats: None,
        });

        let result = backend
            .call_tool("cypher", &json!({"query": "CREATE (n:File {id: '1'})"}))
            .await;
        assert!(matches!(result, Err(McpError::WriteQueryRejected)));
    }
}
