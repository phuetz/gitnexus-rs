//! Snapshot history + diff (Axes B3 full + B4).
//!
//! Persists named copies of `graph.bin` into `<.gitnexus>/snapshots/` so the
//! user can compare the architecture state between two points in time and
//! navigate back to a previous structure without re-indexing.
//!
//! The store is capped (default 10) to keep disk usage bounded — oldest
//! snapshots get evicted FIFO when the limit is hit.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

const MAX_SNAPSHOTS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMeta {
    /// Snapshot id (timestamp-derived, safe filename).
    pub id: String,
    pub label: String,
    pub created_at: i64,
    pub node_count: u32,
    pub edge_count: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotIndex {
    #[serde(default)]
    pub snapshots: Vec<SnapshotMeta>,
}

fn snapshots_dir(storage: &str) -> PathBuf {
    PathBuf::from(storage).join("snapshots")
}

fn index_path(storage: &str) -> PathBuf {
    snapshots_dir(storage).join("index.json")
}

fn snapshot_file_path(storage: &str, id: &str) -> PathBuf {
    let safe: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    snapshots_dir(storage).join(format!("{safe}.bin"))
}

fn load_index(storage: &str) -> SnapshotIndex {
    match std::fs::read_to_string(index_path(storage)) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => SnapshotIndex::default(),
    }
}

fn save_index(storage: &str, idx: &SnapshotIndex) -> Result<(), String> {
    let dir = snapshots_dir(storage);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(idx).map_err(|e| e.to_string())?;
    std::fs::write(index_path(storage), s).map_err(|e| e.to_string())
}

// ─── Tauri commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn snapshot_create(
    state: State<'_, AppState>,
    label: Option<String>,
) -> Result<SnapshotMeta, String> {
    let storage = state.active_storage_path().await?;
    let live_path = PathBuf::from(&storage).join("graph.bin");
    if !live_path.exists() {
        return Err("No graph.bin in storage — analyze the repo first".into());
    }

    let id = format!("snap_{}", chrono::Utc::now().timestamp_millis());
    let dest = snapshot_file_path(&storage, &id);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&live_path, &dest).map_err(|e| e.to_string())?;

    let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);

    // Quickly read counts from the snapshot for the meta entry.
    let (node_count, edge_count) = match gitnexus_db::snapshot::load_snapshot(&dest) {
        Ok(g) => (
            g.iter_nodes().count() as u32,
            g.iter_relationships().count() as u32,
        ),
        Err(_) => (0, 0),
    };

    let meta = SnapshotMeta {
        id: id.clone(),
        label: label.unwrap_or_else(|| "Manual snapshot".into()),
        created_at: chrono::Utc::now().timestamp_millis(),
        node_count,
        edge_count,
        size_bytes: size,
    };

    let mut idx = load_index(&storage);
    idx.snapshots.push(meta.clone());
    // FIFO eviction: drop oldest until under the cap.
    if idx.snapshots.len() > MAX_SNAPSHOTS {
        idx.snapshots
            .sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let drop = idx.snapshots.len() - MAX_SNAPSHOTS;
        for evicted in idx.snapshots.drain(0..drop) {
            let _ = std::fs::remove_file(snapshot_file_path(&storage, &evicted.id));
        }
    }
    // Newest-first display order.
    idx.snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    save_index(&storage, &idx)?;
    Ok(meta)
}

#[tauri::command]
pub async fn snapshot_list(state: State<'_, AppState>) -> Result<Vec<SnapshotMeta>, String> {
    let storage = state.active_storage_path().await?;
    Ok(load_index(&storage).snapshots)
}

#[tauri::command]
pub async fn snapshot_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<SnapshotMeta>, String> {
    let storage = state.active_storage_path().await?;
    let mut idx = load_index(&storage);
    idx.snapshots.retain(|s| s.id != id);
    save_index(&storage, &idx)?;
    let _ = std::fs::remove_file(snapshot_file_path(&storage, &id));
    Ok(idx.snapshots)
}

// ─── Diff ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiffRequest {
    /// "live" means the current graph.bin; otherwise a snapshot id.
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiff {
    pub from_id: String,
    pub to_id: String,
    pub from_node_count: u32,
    pub to_node_count: u32,
    pub from_edge_count: u32,
    pub to_edge_count: u32,
    /// Per-label node deltas (added, removed).
    pub by_label: Vec<LabelDelta>,
    pub added_sample: Vec<DiffNode>,
    pub removed_sample: Vec<DiffNode>,
    pub modified_sample: Vec<ModifiedNode>,
    pub total_added: u32,
    pub total_removed: u32,
    pub total_modified: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelDelta {
    pub label: String,
    pub from_count: u32,
    pub to_count: u32,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffNode {
    pub id: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifiedNode {
    pub id: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
    /// Notable property changes (e.g. is_dead_candidate flipped).
    pub changes: Vec<String>,
}

#[tauri::command]
pub async fn snapshot_diff(
    state: State<'_, AppState>,
    request: SnapshotDiffRequest,
) -> Result<SnapshotDiff, String> {
    let storage = state.active_storage_path().await?;
    let from_path = resolve_snapshot_path(&storage, &request.from)?;
    let to_path = resolve_snapshot_path(&storage, &request.to)?;

    let from = gitnexus_db::snapshot::load_snapshot(&from_path)
        .map_err(|e| format!("Failed to load 'from' snapshot: {e}"))?;
    let to = gitnexus_db::snapshot::load_snapshot(&to_path)
        .map_err(|e| format!("Failed to load 'to' snapshot: {e}"))?;

    let from_ids: HashSet<String> = from.iter_nodes().map(|n| n.id.clone()).collect();
    let to_ids: HashSet<String> = to.iter_nodes().map(|n| n.id.clone()).collect();

    let mut added: Vec<DiffNode> = Vec::new();
    let mut removed: Vec<DiffNode> = Vec::new();
    let mut modified: Vec<ModifiedNode> = Vec::new();
    let mut by_label_from: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut by_label_to: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut by_label_added: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    let mut by_label_removed: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    for n in to.iter_nodes() {
        let label = n.label.as_str().to_string();
        *by_label_to.entry(label.clone()).or_insert(0) += 1;
        if !from_ids.contains(&n.id) {
            *by_label_added.entry(label.clone()).or_insert(0) += 1;
            added.push(DiffNode {
                id: n.id.clone(),
                name: n.properties.name.clone(),
                label,
                file_path: n.properties.file_path.clone(),
            });
        }
    }
    for n in from.iter_nodes() {
        let label = n.label.as_str().to_string();
        *by_label_from.entry(label.clone()).or_insert(0) += 1;
        if !to_ids.contains(&n.id) {
            *by_label_removed.entry(label.clone()).or_insert(0) += 1;
            removed.push(DiffNode {
                id: n.id.clone(),
                name: n.properties.name.clone(),
                label,
                file_path: n.properties.file_path.clone(),
            });
        }
    }

    // Detect "modified" nodes: same id, but key boolean flags flipped.
    for n_from in from.iter_nodes() {
        let Some(n_to) = to.get_node(&n_from.id) else { continue };
        let mut changes: Vec<String> = Vec::new();
        let pf = &n_from.properties;
        let pt = &n_to.properties;
        if pf.is_dead_candidate.unwrap_or(false) != pt.is_dead_candidate.unwrap_or(false) {
            changes.push(format!(
                "isDeadCandidate: {} → {}",
                pf.is_dead_candidate.unwrap_or(false),
                pt.is_dead_candidate.unwrap_or(false)
            ));
        }
        if pf.is_traced.unwrap_or(false) != pt.is_traced.unwrap_or(false) {
            changes.push(format!(
                "isTraced: {} → {}",
                pf.is_traced.unwrap_or(false),
                pt.is_traced.unwrap_or(false)
            ));
        }
        if pf.complexity != pt.complexity {
            changes.push(format!(
                "complexity: {:?} → {:?}",
                pf.complexity, pt.complexity
            ));
        }
        if pf.entry_point_score != pt.entry_point_score {
            changes.push(format!(
                "entryPointScore: {:?} → {:?}",
                pf.entry_point_score, pt.entry_point_score
            ));
        }
        if !changes.is_empty() {
            modified.push(ModifiedNode {
                id: n_from.id.clone(),
                name: n_to.properties.name.clone(),
                label: n_to.label.as_str().to_string(),
                file_path: n_to.properties.file_path.clone(),
                changes,
            });
        }
    }

    let total_added = added.len() as u32;
    let total_removed = removed.len() as u32;
    let total_modified = modified.len() as u32;

    // Sample top-25 by category to keep payload bounded.
    added.truncate(25);
    removed.truncate(25);
    modified.truncate(25);

    // Build by_label deltas (only labels that appear anywhere).
    let mut all_labels: HashSet<String> = HashSet::new();
    all_labels.extend(by_label_from.keys().cloned());
    all_labels.extend(by_label_to.keys().cloned());
    let mut by_label: Vec<LabelDelta> = all_labels
        .into_iter()
        .map(|label| LabelDelta {
            from_count: *by_label_from.get(&label).unwrap_or(&0),
            to_count: *by_label_to.get(&label).unwrap_or(&0),
            added: *by_label_added.get(&label).unwrap_or(&0),
            removed: *by_label_removed.get(&label).unwrap_or(&0),
            label,
        })
        .filter(|d| d.from_count > 0 || d.to_count > 0)
        .collect();
    by_label.sort_by(|a, b| (b.added + b.removed).cmp(&(a.added + a.removed)));

    Ok(SnapshotDiff {
        from_id: request.from.clone(),
        to_id: request.to.clone(),
        from_node_count: from.iter_nodes().count() as u32,
        to_node_count: to.iter_nodes().count() as u32,
        from_edge_count: from.iter_relationships().count() as u32,
        to_edge_count: to.iter_relationships().count() as u32,
        by_label,
        added_sample: added,
        removed_sample: removed,
        modified_sample: modified,
        total_added,
        total_removed,
        total_modified,
    })
}

fn resolve_snapshot_path(storage: &str, id: &str) -> Result<PathBuf, String> {
    if id == "live" || id == "current" {
        let p = PathBuf::from(storage).join("graph.bin");
        if !p.exists() {
            return Err("Live graph.bin not found".into());
        }
        return Ok(p);
    }
    let p = snapshot_file_path(storage, id);
    if !p.exists() {
        return Err(format!("Snapshot '{id}' not found"));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_file_path_strips_unsafe_chars() {
        let p = snapshot_file_path("/tmp/store", "../../etc/passwd");
        let last = p.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(last, "etcpasswd.bin");
    }

    #[test]
    fn test_index_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "gitnexus-snapshots-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(dir.join("snapshots")).unwrap();
        let storage = dir.to_string_lossy().to_string();
        let mut idx = SnapshotIndex::default();
        idx.snapshots.push(SnapshotMeta {
            id: "snap_1".into(),
            label: "test".into(),
            created_at: 1,
            node_count: 10,
            edge_count: 20,
            size_bytes: 1234,
        });
        save_index(&storage, &idx).unwrap();
        let loaded = load_index(&storage);
        assert_eq!(loaded.snapshots.len(), 1);
        assert_eq!(loaded.snapshots[0].id, "snap_1");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
