use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

const GITNEXUS_DIR: &str = ".gitnexus";

// ─── Metadata Types ──────────────────────────────────────────────────────

/// Repository index metadata, stored in `.gitnexus/meta.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoMeta {
    pub repo_path: String,
    pub last_commit: String,
    pub indexed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<RepoStats>,
}

/// Statistics about an indexed repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStats {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communities: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<usize>,
}

/// An entry in the global registry (~/.gitnexus/registry.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub name: String,
    pub path: String,
    pub storage_path: String,
    pub indexed_at: String,
    pub last_commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<RepoStats>,
}

// ─── Path Helpers ────────────────────────────────────────────────────────

/// Get the `.gitnexus` storage path for a repository.
pub fn get_storage_path(repo_path: &Path) -> PathBuf {
    repo_path.canonicalize().unwrap_or_else(|_| repo_path.to_path_buf()).join(GITNEXUS_DIR)
}

/// Get paths to key storage files.
pub struct StoragePaths {
    pub storage_path: PathBuf,
    pub lbug_path: PathBuf,
    pub meta_path: PathBuf,
}

pub fn get_storage_paths(repo_path: &Path) -> StoragePaths {
    let storage_path = get_storage_path(repo_path);
    StoragePaths {
        lbug_path: storage_path.join("lbug"),
        meta_path: storage_path.join("meta.json"),
        storage_path,
    }
}

// ─── Global Registry ─────────────────────────────────────────────────────

/// Get the global GitNexus directory (~/.gitnexus/).
pub fn get_global_dir() -> PathBuf {
    dirs_or_home().join(".gitnexus")
}

/// Get the path to the global registry file.
pub fn get_global_registry_path() -> PathBuf {
    get_global_dir().join("registry.json")
}

/// Read the global registry. Returns empty vec if not found.
pub fn read_registry() -> Result<Vec<RegistryEntry>> {
    let path = get_global_registry_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let entries: Vec<RegistryEntry> = serde_json::from_str(&raw)?;
            Ok(entries)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(CoreError::Io(e)),
    }
}

/// Write the global registry to disk.
pub fn write_registry(entries: &[RegistryEntry]) -> Result<()> {
    let dir = get_global_dir();
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(get_global_registry_path(), json)?;
    Ok(())
}

/// Register (add or update) a repo in the global registry.
pub fn register_repo(repo_path: &Path, meta: &RepoMeta) -> Result<()> {
    let resolved = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let name = resolved
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let storage_path = get_storage_path(&resolved);

    let mut entries = read_registry()?;

    // Find existing entry by path (case-insensitive on Windows)
    let existing = entries.iter().position(|e| paths_equal(&e.path, &resolved));

    let entry = RegistryEntry {
        name,
        path: resolved.display().to_string(),
        storage_path: storage_path.display().to_string(),
        indexed_at: meta.indexed_at.clone(),
        last_commit: meta.last_commit.clone(),
        stats: meta.stats.clone(),
    };

    if let Some(idx) = existing {
        entries[idx] = entry;
    } else {
        entries.push(entry);
    }

    write_registry(&entries)
}

/// Remove a repo from the global registry.
pub fn unregister_repo(repo_path: &Path) -> Result<()> {
    let resolved = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let mut entries = read_registry()?;
    entries.retain(|e| !paths_equal(&e.path, &resolved));
    write_registry(&entries)
}

/// Check if a path has a GitNexus index.
pub fn has_index(repo_path: &Path) -> bool {
    let paths = get_storage_paths(repo_path);
    paths.meta_path.exists()
}

/// Load metadata from an indexed repo.
pub fn load_meta(storage_path: &Path) -> Result<Option<RepoMeta>> {
    let meta_path = storage_path.join("meta.json");
    match std::fs::read_to_string(&meta_path) {
        Ok(raw) => {
            let meta: RepoMeta = serde_json::from_str(&raw)?;
            Ok(Some(meta))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::Io(e)),
    }
}

/// Save metadata to storage.
pub fn save_meta(storage_path: &Path, meta: &RepoMeta) -> Result<()> {
    std::fs::create_dir_all(storage_path)?;
    let meta_path = storage_path.join("meta.json");
    let json = serde_json::to_string_pretty(meta)?;
    std::fs::write(meta_path, json)?;
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn dirs_or_home() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
    }
}

fn paths_equal(a: &str, b: &Path) -> bool {
    let a_path = Path::new(a);
    let a_canon = a_path.canonicalize().unwrap_or_else(|_| a_path.to_path_buf());
    let b_canon = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());

    #[cfg(target_os = "windows")]
    {
        a_canon.to_string_lossy().to_lowercase() == b_canon.to_string_lossy().to_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        a_canon == b_canon
    }
}
