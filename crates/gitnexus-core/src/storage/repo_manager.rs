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
    repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf())
        .join(GITNEXUS_DIR)
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

/// Write the global registry to disk atomically (temp file + rename).
///
/// Writing directly with `fs::write` leaves a window where the file is empty
/// or partially written if the process is interrupted, and gives no atomicity
/// guarantees against concurrent writers. The temp+rename pattern ensures
/// any reader either sees the old contents or the complete new contents.
pub fn write_registry(entries: &[RegistryEntry]) -> Result<()> {
    let dir = get_global_dir();
    std::fs::create_dir_all(&dir)?;
    let final_path = get_global_registry_path();
    let json = serde_json::to_string_pretty(entries)?;
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = dir.join(format!("registry.json.tmp.{pid}.{nanos}"));
    std::fs::write(&tmp_path, json)?;
    if let Err(e) = std::fs::rename(&tmp_path, &final_path) {
        // Best-effort cleanup of the orphaned temp file so interrupted writes
        // don't leak accumulating `.tmp.*` files in ~/.gitnexus/ over time
        // (e.g. when the destination is locked on Windows or permission
        // changes mid-run).
        let _ = std::fs::remove_file(&tmp_path);
        return Err(CoreError::Io(e));
    }
    Ok(())
}

/// Acquire an exclusive sentinel-file lock around `op`. Retries briefly so
/// concurrent indexers serialize on the registry instead of clobbering each
/// other's writes. Times out after a few seconds and returns an error.
fn with_registry_lock<T>(op: impl FnOnce() -> Result<T>) -> Result<T> {
    let dir = get_global_dir();
    std::fs::create_dir_all(&dir)?;
    let lock_path = dir.join("registry.json.lock");

    let max_attempts = 50;
    let backoff_ms = 100;
    for attempt in 0..max_attempts {
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(file) => {
                drop(file);
                let result = op();
                // Best-effort cleanup; tolerate races on Windows where another
                // process may have already removed the lock during recovery.
                let _ = std::fs::remove_file(&lock_path);
                return result;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // If the lock file is stale (older than 30s), reclaim it.
                if let Ok(meta) = std::fs::metadata(&lock_path) {
                    if let Ok(modified) = meta.modified() {
                        if let Ok(age) = std::time::SystemTime::now().duration_since(modified) {
                            if age.as_secs() > 30 {
                                let _ = std::fs::remove_file(&lock_path);
                                continue; // retry immediately after reclaiming stale lock
                            }
                        }
                    }
                }
                if attempt + 1 < max_attempts {
                    std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    continue;
                }
                return Err(CoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "Registry lock contention: another GitNexus process is updating the registry",
                )));
            }
            Err(e) => return Err(CoreError::Io(e)),
        }
    }
    Err(CoreError::Io(std::io::Error::other(
        "Registry lock acquisition exhausted",
    )))
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

    // Hold the lock across the read-modify-write so concurrent indexers
    // serialize instead of last-writer-wins clobbering each other.
    with_registry_lock(|| {
        let mut entries = read_registry()?;

        // Find existing entry by path (case-insensitive on Windows)
        let existing = entries.iter().position(|e| paths_equal(&e.path, &resolved));

        let entry = RegistryEntry {
            name: name.clone(),
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
    })
}

/// Remove a repo from the global registry.
pub fn unregister_repo(repo_path: &Path) -> Result<()> {
    let resolved = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    with_registry_lock(|| {
        let mut entries = read_registry()?;
        entries.retain(|e| !paths_equal(&e.path, &resolved));
        write_registry(&entries)
    })
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

/// Save metadata to storage atomically (temp file + rename).
///
/// A direct `fs::write` leaves a window where readers can observe a partial
/// or empty `meta.json` if the process is interrupted, which can prevent the
/// repo from being recognised on the next launch. The temp+rename pattern
/// guarantees readers either see the previous contents or the complete new
/// contents.
pub fn save_meta(storage_path: &Path, meta: &RepoMeta) -> Result<()> {
    std::fs::create_dir_all(storage_path)?;
    let meta_path = storage_path.join("meta.json");
    let json = serde_json::to_string_pretty(meta)?;
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = storage_path.join(format!("meta.json.tmp.{pid}.{nanos}"));
    std::fs::write(&tmp_path, json)?;
    if let Err(e) = std::fs::rename(&tmp_path, &meta_path) {
        // Best-effort cleanup of the orphaned temp file so a failed rename
        // (e.g. destination locked on Windows) does not leave stray
        // `.tmp.*` files accumulating in `.gitnexus/`.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(CoreError::Io(e));
    }
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Resolve the directory in which `.gitnexus/` should live.
///
/// Order of resolution:
/// 1. **`GITNEXUS_HOME`** — explicit override. The path is used as-is, with
///    `.gitnexus` already implied (so `GITNEXUS_HOME=D:\kit\data` makes the
///    global directory `D:\kit\data\.gitnexus`). This is the portable-USB
///    mode: the launcher sets `GITNEXUS_HOME=%~dp0data` and every read of
///    `~/.gitnexus/` then resolves to the kit's own directory rather than
///    the operator's `%USERPROFILE%\.gitnexus`.
/// 2. **`USERPROFILE`** (Windows) / **`HOME`** (other) — standard user home.
/// 3. A safe sentinel fallback (`C:\Users\Default` / `/tmp`) — should never
///    actually be hit; protects against panics if the environment is empty.
fn dirs_or_home() -> PathBuf {
    if let Ok(override_path) = std::env::var("GITNEXUS_HOME") {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
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
    let a_canon = a_path
        .canonicalize()
        .unwrap_or_else(|_| a_path.to_path_buf());
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
