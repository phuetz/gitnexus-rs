//! Application state managed by Tauri.
//!
//! Loads the knowledge graph directly into memory for O(1) lookups,
//! following the same pattern as the TUI dashboard.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::storage::repo_manager::{self, RegistryEntry};
use gitnexus_db::inmemory::cypher::GraphIndexes;
use gitnexus_db::inmemory::fts::FtsIndex;
use gitnexus_db::snapshot;
use tokio::sync::{Mutex, RwLock};

use crate::types::ChatConfig;

/// A loaded repository with its graph, indexes, and FTS.
pub struct LoadedRepo {
    pub entry: RegistryEntry,
    pub graph: Arc<KnowledgeGraph>,
    pub indexes: Arc<GraphIndexes>,
    pub fts_index: Arc<FtsIndex>,
}

/// The main application state, shared across all Tauri commands.
pub struct AppState {
    /// Loaded repositories keyed by name.
    repos: RwLock<HashMap<String, LoadedRepo>>,
    /// Global registry entries.
    registry: RwLock<Vec<RegistryEntry>>,
    /// Currently active repository name.
    active_repo: RwLock<Option<String>>,
    /// Serializes the slow snapshot-load path so two concurrent calls to
    /// `open_repo("foo")` can't both hit disk + rebuild indexes for the
    /// same repo. The fast path (already-loaded) doesn't take this lock.
    load_lock: Mutex<()>,
    /// Full chat config for the current desktop session. Secrets stay here,
    /// not on disk.
    chat_config: RwLock<Option<ChatConfig>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self {
            repos: RwLock::new(HashMap::new()),
            registry: RwLock::new(Vec::new()),
            active_repo: RwLock::new(None),
            load_lock: Mutex::new(()),
            chat_config: RwLock::new(None),
        }
    }

    /// Load the global registry from disk.
    pub async fn load_registry(&self) -> Result<Vec<RegistryEntry>, String> {
        let entries = repo_manager::read_registry().map_err(|e| e.to_string())?;
        *self.registry.write().await = entries.clone();
        Ok(entries)
    }

    /// Get the current registry entries.
    pub async fn registry(&self) -> Vec<RegistryEntry> {
        self.registry.read().await.clone()
    }

    /// Open a repository by name: load its graph and build indexes.
    pub async fn open_repo(&self, name: &str) -> Result<(), String> {
        // Fast path: already loaded.
        if self.repos.read().await.contains_key(name) {
            *self.active_repo.write().await = Some(name.to_string());
            return Ok(());
        }

        // Slow path: serialize load attempts so two concurrent callers don't
        // both load the same snapshot from disk and rebuild its indexes.
        let _guard = self.load_lock.lock().await;

        // Re-check inside the load lock — a previous waiter may have just
        // finished loading this same repo while we were waiting on the lock.
        if self.repos.read().await.contains_key(name) {
            *self.active_repo.write().await = Some(name.to_string());
            return Ok(());
        }

        // Find entry in registry
        let registry = self.registry.read().await;
        let entry = registry
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("Repository '{}' not found in registry", name))?
            .clone();
        drop(registry);

        // Load snapshot
        let storage_path = Path::new(&entry.storage_path);
        let snap_path = snapshot::snapshot_path(storage_path);

        if !snapshot::snapshot_exists(&snap_path) {
            return Err(format!(
                "No snapshot found for '{}'. Run `gitnexus analyze` first.",
                name
            ));
        }

        let graph = snapshot::load_snapshot(&snap_path).map_err(|e| e.to_string())?;
        let indexes = GraphIndexes::build(&graph);
        let fts_index = FtsIndex::build(&graph);

        let loaded = LoadedRepo {
            entry,
            graph: Arc::new(graph),
            indexes: Arc::new(indexes),
            fts_index: Arc::new(fts_index),
        };

        self.repos.write().await.insert(name.to_string(), loaded);
        *self.active_repo.write().await = Some(name.to_string());

        Ok(())
    }

    /// Get the active repository name.
    pub async fn active_repo_name(&self) -> Option<String> {
        self.active_repo.read().await.clone()
    }

    /// Reload a repo's graph from disk (e.g. after re-indexing).
    /// Removes the old loaded data and re-loads from snapshot.
    pub async fn reload_repo(&self, name: &str) -> Result<(), String> {
        // Remove cached data so next open_repo reloads from disk
        self.repos.write().await.remove(name);
        // Re-open with fresh data
        self.open_repo(name).await
    }

    /// Get a reference to a loaded repo's components.
    /// Returns (graph, indexes, fts_index) or an error.
    pub async fn get_repo(
        &self,
        name: Option<&str>,
    ) -> Result<(Arc<KnowledgeGraph>, Arc<GraphIndexes>, Arc<FtsIndex>, String), String> {
        let repo_name = match name {
            Some(n) => n.to_string(),
            None => self
                .active_repo
                .read()
                .await
                .clone()
                .ok_or_else(|| "No active repository. Open one first.".to_string())?,
        };

        let repos = self.repos.read().await;
        let loaded = repos
            .get(&repo_name)
            .ok_or_else(|| format!("Repository '{}' is not loaded", repo_name))?;

        Ok((
            Arc::clone(&loaded.graph),
            Arc::clone(&loaded.indexes),
            Arc::clone(&loaded.fts_index),
            loaded.entry.path.clone(),
        ))
    }

    /// Get the current session's full chat config, if one has been set.
    pub async fn chat_config(&self) -> Option<ChatConfig> {
        self.chat_config.read().await.clone()
    }

    /// Update the in-memory chat config for the current session.
    pub async fn set_chat_config(&self, config: ChatConfig) {
        *self.chat_config.write().await = Some(config);
    }
}
