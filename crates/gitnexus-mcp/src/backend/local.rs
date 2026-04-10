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

const MAX_QUERY_LIMIT: usize = 100;
const MAX_ANALYTICS_LIMIT: usize = 100;
const MAX_IMPACT_DEPTH: usize = 10;

fn clamp_limit(raw: Option<u64>, default: usize, max: usize) -> usize {
    raw.map(|v| v as usize).unwrap_or(default).clamp(1, max)
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
                        // Exact name / exact path / suffix on a path-segment
                        // boundary. Without the segment guard, "foo" would
                        // also match "/repos/myfoo", silently selecting the
                        // wrong repo for any name that's a tail substring.
                        if e.name.to_lowercase() == lower {
                            return true;
                        }
                        let path_lower = e.path.to_lowercase().replace('\\', "/");
                        if path_lower == lower {
                            return true;
                        }
                        path_lower.ends_with(&format!("/{}", lower))
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
            "business" => self.tool_business(args).await,
            "analyze_execution_trace" => self.tool_analyze_execution_trace(args).await,
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
        let limit = clamp_limit(
            args.get("limit").and_then(|v| v.as_u64()),
            10,
            MAX_QUERY_LIMIT,
        );

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

    async fn tool_impact(&mut self, args: &Value) -> Result<Value> {
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
            .unwrap_or(5)
            .clamp(1, MAX_IMPACT_DEPTH as u64) as usize;

        // Previously this tool issued Cypher queries with variable-length
        // path syntax (`[r:CALLS|USES|IMPORTS*1..N]`). The default in-memory
        // backend's Cypher executor only supports single relationship types
        // and no variable-length paths, so the queries always failed to
        // parse — and the `if let Ok(rows) = ...` pattern silently swallowed
        // the parser error, leaving every caller with an empty result and no
        // diagnostic. Implement the BFS directly against the loaded snapshot
        // so the tool actually returns the blast radius.
        let snap_path = {
            let entry = self.resolve_repo(repo_name)?;
            std::path::Path::new(&entry.storage_path).join("graph.bin")
        };
        let graph = self.load_cached_snapshot(&snap_path)?;

        // Find target node(s) by exact id, then by exact name (case-insensitive).
        let target_lower = target.to_lowercase();
        let target_ids: std::collections::HashSet<String> = graph
            .iter_nodes()
            .filter(|n| n.id == target || n.properties.name.to_lowercase() == target_lower)
            .map(|n| n.id.clone())
            .collect();

        if target_ids.is_empty() {
            return Err(McpError::Internal(format!(
                "Symbol '{}' not found in graph",
                target
            )));
        }

        // Build forward (source -> targets) and reverse (target -> sources)
        // adjacency only over the impact-bearing relationship types so the
        // BFS doesn't drift into structural edges like HasMethod / Defines.
        //
        // Keep this set in sync with the canonical causal-edge filter used
        // by `impact.rs::bfs_impact`, `impact_cmd.rs`, `shell.rs::cmd_impact`
        // and `chat_executor.rs::execute_impact`. Previously this tool only
        // walked Calls/Uses/Imports/CallsAction/CallsService/DependsOn,
        // missing every impact that flows through inheritance (changing a
        // base class affects subclasses), interface implementation, view
        // rendering, routing, HTTP fetch, or entity mapping. Same root
        // cause as #39, #52, #55 in the other impact code paths.
        let want_rel = |rt: RelationshipType| {
            matches!(
                rt,
                RelationshipType::Calls
                    | RelationshipType::CallsAction
                    | RelationshipType::CallsService
                    | RelationshipType::Imports
                    | RelationshipType::Uses
                    | RelationshipType::DependsOn
                    | RelationshipType::Inherits
                    | RelationshipType::Implements
                    | RelationshipType::Extends
                    | RelationshipType::Overrides
                    | RelationshipType::RendersView
                    | RelationshipType::HandlesRoute
                    | RelationshipType::Fetches
                    | RelationshipType::MapsToEntity
            )
        };

        let mut forward: HashMap<String, Vec<String>> = HashMap::new();
        let mut reverse: HashMap<String, Vec<String>> = HashMap::new();
        for rel in graph.iter_relationships() {
            if !want_rel(rel.rel_type) {
                continue;
            }
            forward
                .entry(rel.source_id.clone())
                .or_default()
                .push(rel.target_id.clone());
            reverse
                .entry(rel.target_id.clone())
                .or_default()
                .push(rel.source_id.clone());
        }

        // BFS up to `max_depth`. Returns Vec<(node_id, depth)> in BFS order
        // so the caller sees nearest neighbours first. The seed targets
        // themselves are excluded from the result so callers don't show up
        // as their own impact.
        let bfs = |adjacency: &HashMap<String, Vec<String>>| -> Vec<(String, usize)> {
            let mut visited: std::collections::HashSet<String> = target_ids.iter().cloned().collect();
            let mut queue: std::collections::VecDeque<(String, usize)> =
                target_ids.iter().map(|id| (id.clone(), 0usize)).collect();
            let mut out: Vec<(String, usize)> = Vec::new();
            while let Some((node, depth)) = queue.pop_front() {
                if depth >= max_depth {
                    continue;
                }
                if let Some(neighbors) = adjacency.get(&node) {
                    for n in neighbors {
                        if visited.insert(n.clone()) {
                            out.push((n.clone(), depth + 1));
                            queue.push_back((n.clone(), depth + 1));
                        }
                    }
                }
            }
            out
        };

        let format_rows = |rows: Vec<(String, usize)>| -> Vec<Value> {
            rows.into_iter()
                .filter_map(|(id, depth)| {
                    let node = graph.get_node(&id)?;
                    Some(json!({
                        "name": node.properties.name,
                        "id": node.id,
                        "filePath": node.properties.file_path,
                        "label": node.label.as_str(),
                        "depth": depth,
                    }))
                })
                .collect()
        };

        let mut all_results: Vec<Value> = Vec::new();

        if direction == "upstream" || direction == "both" {
            let callers = format_rows(bfs(&reverse));
            all_results.push(json!({
                "direction": "upstream",
                "callers": callers,
            }));
        }

        if direction == "downstream" || direction == "both" {
            let callees = format_rows(bfs(&forward));
            all_results.push(json!({
                "direction": "downstream",
                "callees": callees,
            }));
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
            .and_then(|v| v.as_u64());
        let limit = clamp_limit(limit, 20, MAX_ANALYTICS_LIMIT);

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
            .and_then(|v| v.as_u64());
        let limit = clamp_limit(limit, 20, MAX_ANALYTICS_LIMIT);

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
            .and_then(|v| v.as_u64());
        let limit = clamp_limit(limit, 20, MAX_ANALYTICS_LIMIT);

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

        // Build class -> methods index. Dead-code status comes from the
        // `is_dead_candidate` property already computed by the dead-code phase,
        // so we no longer need to recompute incoming-call sets here.
        let mut class_methods: HashMap<String, Vec<String>> = HashMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::HasMethod {
                class_methods
                    .entry(rel.source_id.clone())
                    .or_default()
                    .push(rel.target_id.clone());
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
                    graph.get_node(mid).is_some_and(|n| n.properties.is_traced == Some(true))
                }).count();
                // Use the `is_dead_candidate` property computed by the dead-code phase,
                // which correctly excludes entry points, tests, interface methods,
                // view scripts, and ControllerAction-paired methods. Recomputing
                // dead-status here from incoming_calls alone overcounts dead methods.
                let dead = methods.iter().filter(|mid| {
                    graph.get_node(mid)
                        .and_then(|n| n.properties.is_dead_candidate)
                        .unwrap_or(false)
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
            // Same fix as the per-class branch: use is_dead_candidate from the
            // dead-code phase, which respects entry-point/test/interface exclusions.
            let dead = all_methods.iter().filter(|n| n.properties.is_dead_candidate == Some(true)).count();

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

        // The MCP diagram tool currently only emits Mermaid `graph TD` flowcharts.
        // Reject other types up front instead of silently ignoring them — the
        // previous behaviour echoed `diagramType` in `_meta` while always
        // returning a flowchart, misleading callers.
        if diagram_type != "flowchart" {
            return Err(McpError::InvalidArguments {
                tool: "diagram".into(),
                reason: format!(
                    "Unsupported diagram type '{}'; only 'flowchart' is supported",
                    diagram_type
                ),
            });
        }

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
        lines.push("graph TD".to_string());

        let node_id = &start_node.id;
        let node_name = &start_node.properties.name;
        lines.push(format!(
            "    {}[\"{}\"]",
            sanitize_mermaid_id(node_id),
            escape_mermaid_label(node_name)
        ));

        // Pre-build set of child IDs to avoid O(E^2) inner loop
        let child_ids: std::collections::HashSet<&str> = graph.iter_relationships()
            .filter(|r| r.source_id == *node_id && matches!(r.rel_type, RelationshipType::HasMethod | RelationshipType::HasAction))
            .map(|r| r.target_id.as_str())
            .collect();

        // Find methods and their calls
        for rel in graph.iter_relationships() {
            if (&rel.source_id == node_id || child_ids.contains(rel.source_id.as_str()))
                && matches!(rel.rel_type, RelationshipType::Calls | RelationshipType::CallsAction | RelationshipType::CallsService) {
                    if let Some(target_node) = graph.get_node(&rel.target_id) {
                        let src_name = graph.get_node(&rel.source_id)
                            .map(|n| n.properties.name.as_str())
                            .unwrap_or("?");
                        lines.push(format!(
                            "    {}[\"{}\"] --> {}[\"{}\"]",
                            sanitize_mermaid_id(&rel.source_id),
                            escape_mermaid_label(src_name),
                            sanitize_mermaid_id(&rel.target_id),
                            escape_mermaid_label(&target_node.properties.name),
                        ));
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
                            escape_mermaid_label(&member.properties.name),
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

        // Derive the grade from the same rounded value surfaced to the caller
        // so the displayed score and letter grade can't disagree. Previously
        // the grade came from `score as u32` (truncation) while the displayed
        // score rounded to one decimal, so a raw score of 89.95 reported
        // `"90.0" / "B"` — the score crossed the A boundary after rounding
        // but the grade did not. Same bug pattern as health.rs / report.rs.
        let rounded_score = (score * 10.0).round() / 10.0;
        let grade = match rounded_score as u32 {
            90..=100 => "A",
            75..=89 => "B",
            60..=74 => "C",
            40..=59 => "D",
            _ => "E",
        };

        let report = json!({
            "grade": grade,
            "score": rounded_score,
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

    async fn tool_analyze_execution_trace(&mut self, args: &Value) -> Result<Value> {
        let trace_file = args
            .get("trace_file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArguments {
                tool: "analyze_execution_trace".into(),
                reason: "Missing required 'trace_file' parameter".into(),
            })?;

        let repo_name = args.get("repo").and_then(|v| v.as_str());
        
        let (repo_path, snap_path) = {
            let entry = self.resolve_repo(repo_name)?;
            (
                std::path::PathBuf::from(&entry.path),
                std::path::Path::new(&entry.storage_path).join("graph.bin"),
            )
        };

        let graph = self.load_cached_snapshot(&snap_path)?;

        // Resolve `trace_file` against the repo root and require the canonical
        // path to stay inside the repo. Without this, an MCP client could
        // supply an arbitrary absolute path (or `..`-laden relative path) and
        // exfiltrate any file the server can read.
        let candidate = if std::path::Path::new(trace_file).is_absolute() {
            std::path::PathBuf::from(trace_file)
        } else {
            repo_path.join(trace_file)
        };
        let canonical_trace = candidate.canonicalize().map_err(|_| {
            McpError::Internal(format!("Trace file not found: {}", trace_file))
        })?;
        let canonical_repo = repo_path.canonicalize().map_err(|e| {
            McpError::Internal(format!("Failed to canonicalize repo path: {}", e))
        })?;
        if !canonical_trace.starts_with(&canonical_repo) {
            return Err(McpError::Internal(
                "trace_file must be a path inside the repository".to_string(),
            ));
        }

        let trace_content = std::fs::read_to_string(&canonical_trace)
            .map_err(|e| McpError::Internal(format!("Failed to read trace file: {}", e)))?;

        let steps = gitnexus_core::trace::parse_trace(&trace_content)
            .map_err(|e| McpError::Internal(format!("Failed to parse trace: {}", e)))?;

        let name_to_ids = gitnexus_core::trace::build_name_index(&graph);

        let mut enriched_steps = Vec::new();
        let mut matched = 0;

        for step in steps {
            let mut enriched_step = step.clone();
            let method_name_opt = step.get("method").or(step.get("name")).and_then(|v| v.as_str());

            if let Some(full_method_name) = method_name_opt {
                if let Some(node_id) = gitnexus_core::trace::resolve_method_node(&graph, &name_to_ids, full_method_name) {
                    if let Some(node) = graph.get_node(&node_id) {
                        matched += 1;
                        if let Some(obj) = enriched_step.as_object_mut() {
                            obj.insert("nodeId".to_string(), json!(node_id));
                            obj.insert("filePath".to_string(), json!(node.properties.file_path.clone()));
                            obj.insert("startLine".to_string(), json!(node.properties.start_line));
                            obj.insert("endLine".to_string(), json!(node.properties.end_line));

                            // Path traversal guard: the graph node's file_path
                            // comes from a snapshot that may contain `..`
                            // segments (corrupted or hand-crafted), and we
                            // mustn't let them escape the repo root and
                            // exfiltrate arbitrary files into MCP responses.
                            let full_path = repo_path.join(&node.properties.file_path);
                            let source_safe = match (
                                full_path.canonicalize().ok(),
                                repo_path.canonicalize().ok(),
                            ) {
                                (Some(canon), Some(root)) => canon.starts_with(&root),
                                _ => false,
                            };
                            if source_safe {
                                if let (Some(start), Some(end)) = (node.properties.start_line, node.properties.end_line) {
                                    if let Some(source) = gitnexus_core::trace::extract_source_lines(&full_path, start, end) {
                                        obj.insert("sourceCode".to_string(), json!(source));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            enriched_steps.push(enriched_step);
        }

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "totalSteps": enriched_steps.len(),
                    "matchedSteps": matched,
                    "trace": enriched_steps
                })).unwrap_or_default()
            }],
            "_meta": {
                "hint": hints::hint_for("analyze_execution_trace")
            }
        }))
    }

    async fn tool_business(&mut self, args: &Value) -> Result<Value> {
        let repo_name = args.get("repo").and_then(|v| v.as_str());
        let process_filter = args.get("process").and_then(|v| v.as_str());

        let snap_path = {
            let entry = self.resolve_repo(repo_name)?;
            std::path::Path::new(&entry.storage_path).join("graph.bin")
        };

        let graph = self.load_cached_snapshot(&snap_path)?;

        let mut processes = Vec::new();

        // Check for specific Alise business patterns in the graph
        let has_courriers = graph.iter_nodes().any(|n| n.properties.name.contains("Courrier"));
        let has_paiements = graph.iter_nodes().any(|n| n.properties.name.contains("Reglement") || n.properties.name.contains("Facture"));
        let has_baremes = graph.iter_nodes().any(|n| n.properties.name.contains("Bareme"));
        let has_fournisseurs = graph.iter_nodes().any(|n| n.properties.name.contains("Fournisseur"));

        if has_courriers {
            processes.push(json!({
                "id": "courriers",
                "name": "Système de Courriers",
                "description": "Génération de courriers officiels (Accord, Refus, etc.) via Aspose.Words."
            }));
        }
        if has_paiements {
            processes.push(json!({
                "id": "paiements",
                "name": "Cycle de Paiement",
                "description": "Flux financier complet : de la facture à l'export ELODIE."
            }));
        }
        if has_baremes {
            processes.push(json!({
                "id": "baremes",
                "name": "Calcul des Barèmes",
                "description": "Moteur de calcul des droits et plafonds d'aide sociale."
            }));
        }
        if has_fournisseurs {
            processes.push(json!({
                "id": "fournisseurs",
                "name": "Gestion des Fournisseurs",
                "description": "Référentiel des prestataires de services et coordonnées bancaires."
            }));
        }

        let result = if let Some(filter) = process_filter {
            let filter_lower = filter.to_lowercase();
            match filter_lower.as_str() {
                "courriers" if has_courriers => json!({
                    "id": "courriers",
                    "title": "Système de Courriers",
                    "details": "L'application gère 11 types de courriers. Processus : Sélection destinataires -> Mail Merge (ELODIE variables) -> Génération PDF -> Archivage.",
                    "key_entities": ["CourrierController", "RegleCourrierMasse", "CourrierGenerer"]
                }),
                "paiements" | "paiement" if has_paiements => json!({
                    "id": "paiements",
                    "title": "Cycle de Paiement",
                    "details": "États : DemPaiemVal (Création) -> DemGrPrVal (Groupé) -> DemTransmiseELODIE (Export). Export vers ELODIE via Flux3 Excel.",
                    "key_entities": ["FacturesController", "FactureService", "ElodieService"]
                }),
                "baremes" | "bareme" if has_baremes => json!({
                    "id": "baremes",
                    "title": "Calcul des Barèmes",
                    "details": "Calcul basé sur le quotient familial (Ressources / Parts). Détermine le TauxFASS et les plafonds de prise en charge.",
                    "key_entities": ["Bareme", "Tranche", "CalculService"]
                }),
                "fournisseurs" if has_fournisseurs => json!({
                    "id": "fournisseurs",
                    "title": "Gestion des Fournisseurs",
                    "details": "Gestion des tiers payés par la CMCAS. Inclut la recherche, l'historique de paiement et les RIB.",
                    "key_entities": ["Fournisseur", "IBAN", "FournisseurController"]
                }),
                _ => json!({
                    "error": format!("Processus '{}' non trouvé ou non applicable à ce projet.", filter),
                    "available_processes": processes
                })
            }
        } else {
            json!({
                "available_processes": processes,
                "total_found": processes.len()
            })
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result).unwrap_or_default()
            }],
            "_meta": {
                "hint": hints::hint_for("business")
            }
        }))
    }
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace([':', '/', '.', ' ', '<', '>', '(', ')', '{', '}'], "_")
}

/// Escape characters that would break a Mermaid `["..."]` label literal.
/// `sanitize_mermaid_id` is for *identifiers*; this is for the *display label*
/// shown to the user, which must preserve the original characters where
/// possible while not breaking Mermaid's parser.
///
/// Previously this only replaced `"` with `#quot;`, which left every other
/// problem character (`&`, `<`, `>`, `[`, `]`) to corrupt the label and broke
/// C# generics like `List<string>`, indexers like `Foo[int]`, and any symbol
/// name containing ampersands. We now use the standard HTML entity form
/// (`&amp;`, `&quot;`, `&lt;`, `&gt;`, `&#91;`, `&#93;`) to match the rest of
/// the codebase (`process_doc.rs`, `process.rs`, `diagram.rs`, `export.rs`,
/// `generate_aspnet.rs`, `generate/utils.rs`).
fn escape_mermaid_label(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
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
