//! Local backend: resolves repos from the registry and dispatches tool calls
//! to the appropriate handler.

use serde_json::{json, Value};
use std::time::Instant;
use tracing::info;

use gitnexus_core::storage::repo_manager::{self, RegistryEntry};
use gitnexus_db::pool::ConnectionPool;
use gitnexus_db::query;

use crate::error::{McpError, Result};
use crate::hints;

/// Local MCP backend: manages connections and dispatches tool calls.
pub struct LocalBackend {
    pool: ConnectionPool,
    registry: Vec<RegistryEntry>,
}

impl LocalBackend {
    /// Create a new LocalBackend.
    pub fn new() -> Self {
        Self {
            pool: ConnectionPool::new(),
            registry: Vec::new(),
        }
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

        // Use BM25 FTS search
        let results = gitnexus_search::bm25::search_fts(&adapter, query_text, limit)
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
