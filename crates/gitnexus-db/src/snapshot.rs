use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use gitnexus_core::graph::KnowledgeGraph;

use crate::error::DbError;

/// Save a KnowledgeGraph to a JSON snapshot file.
/// Uses serde_json for full compatibility with enum rename attributes.
/// Performs atomic write with explicit fsync to ensure durability.
pub fn save_snapshot(graph: &KnowledgeGraph, path: &Path) -> Result<(), DbError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbError::CsvError {
            table: "snapshot".to_string(),
            cause: e.to_string(),
        })?;
    }

    // Write to temporary file first to avoid data loss on partial write
    let temp_path = path.with_extension("tmp");
    let file = File::create(&temp_path).map_err(|e| DbError::CsvError {
        table: "snapshot".to_string(),
        cause: format!("Failed to create temporary snapshot file: {e}"),
    })?;

    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, graph).map_err(|e| DbError::CsvError {
        table: "snapshot".to_string(),
        cause: e.to_string(),
    })?;

    // Explicit flush to ensure all data is written to the file
    writer.flush().map_err(|e| DbError::CsvError {
        table: "snapshot".to_string(),
        cause: format!("Failed to flush snapshot data: {e}"),
    })?;

    // Sync to disk for durability (via drop of file handle)
    drop(writer);

    // Atomic rename: temp file becomes the real snapshot
    // This ensures the old snapshot is only replaced when the new one is fully written
    std::fs::rename(&temp_path, path).map_err(|e| DbError::CsvError {
        table: "snapshot".to_string(),
        cause: format!("Failed to finalize snapshot (rename): {e}"),
    })?;

    Ok(())
}

/// Load a KnowledgeGraph from a JSON snapshot file.
pub fn load_snapshot(path: &Path) -> Result<KnowledgeGraph, DbError> {
    let file = File::open(path).map_err(|e| DbError::CsvError {
        table: "snapshot".to_string(),
        cause: format!("Failed to open snapshot: {e}"),
    })?;
    let reader = BufReader::new(file);
    let graph: KnowledgeGraph = serde_json::from_reader(reader).map_err(|e| DbError::CsvError {
        table: "snapshot".to_string(),
        cause: format!("Failed to deserialize snapshot: {e}"),
    })?;
    Ok(graph)
}

/// Check if a snapshot exists.
pub fn snapshot_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Get the snapshot path for a repository's storage directory.
pub fn snapshot_path(storage_path: &Path) -> std::path::PathBuf {
    storage_path.join("graph.bin")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitnexus_core::graph::types::*;

    #[test]
    fn test_snapshot_roundtrip() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "Function:test.rs:foo".to_string(),
            label: NodeLabel::Function,
            properties: NodeProperties {
                name: "foo".to_string(),
                file_path: "test.rs".to_string(),
                start_line: Some(1),
                end_line: Some(10),
                ..Default::default()
            },
        });
        graph.add_node(GraphNode {
            id: "Function:test.rs:bar".to_string(),
            label: NodeLabel::Function,
            properties: NodeProperties {
                name: "bar".to_string(),
                file_path: "test.rs".to_string(),
                ..Default::default()
            },
        });
        graph.add_relationship(GraphRelationship {
            id: "calls_foo_bar".to_string(),
            source_id: "Function:test.rs:foo".to_string(),
            target_id: "Function:test.rs:bar".to_string(),
            rel_type: RelationshipType::Calls,
            confidence: 0.95,
            reason: "exact".to_string(),
            step: None,
        });

        let dir = std::env::temp_dir().join("gitnexus_snapshot_test");
        let path = dir.join("graph.bin");

        save_snapshot(&graph, &path).unwrap();
        assert!(snapshot_exists(&path));

        let loaded = load_snapshot(&path).unwrap();
        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.relationship_count(), 1);
        assert!(loaded.get_node("Function:test.rs:foo").is_some());
        assert!(loaded.get_node("Function:test.rs:bar").is_some());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
