//! Connection pool for multi-repository MCP server.
//!
//! Manages a pool of `DbAdapter` connections keyed by database path.
//! Uses `DashMap` for lock-free concurrent access and implements
//! busy-retry logic with exponential backoff.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tracing::{info, warn};

use crate::adapter::{DbAdapter, StubDbBackend};
use crate::error::{DbError, Result};

/// Maximum number of retry attempts when a database is busy.
const MAX_BUSY_ATTEMPTS: u32 = 3;
/// Base backoff duration (multiplied by attempt number).
const BASE_BACKOFF_MS: u64 = 500;

/// A concurrent connection pool for graph database adapters.
///
/// Thread-safe: uses `DashMap` for lock-free concurrent access to
/// connections. Each connection is wrapped in `Arc` so it can be
/// shared across async tasks.
pub struct ConnectionPool {
    connections: DashMap<PathBuf, Arc<DbAdapter>>,
}

impl ConnectionPool {
    /// Create a new empty connection pool.
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    /// Get an existing connection or open a new one for the given database path.
    ///
    /// Implements busy-retry logic: if the database is busy (e.g., another
    /// process holds a lock), retries up to `MAX_BUSY_ATTEMPTS` times with
    /// exponential backoff (500ms * attempt).
    pub fn get_or_open(&self, db_path: &Path) -> Result<Arc<DbAdapter>> {
        let canonical = db_path
            .canonicalize()
            .unwrap_or_else(|_| db_path.to_path_buf());

        // Fast path: connection already exists
        if let Some(conn) = self.connections.get(&canonical) {
            return Ok(Arc::clone(conn.value()));
        }

        // Slow path: open a new connection with retry logic
        for attempt in 1..=MAX_BUSY_ATTEMPTS {
            match self.try_open(&canonical) {
                Ok(adapter) => {
                    let arc = Arc::new(adapter);
                    self.connections.insert(canonical.clone(), Arc::clone(&arc));
                    info!("ConnectionPool: opened database at {}", canonical.display());
                    return Ok(arc);
                }
                Err(DbError::Busy { .. }) if attempt < MAX_BUSY_ATTEMPTS => {
                    let backoff = Duration::from_millis(BASE_BACKOFF_MS * attempt as u64);
                    warn!(
                        "Database busy at {}, attempt {}/{}, backing off {:?}",
                        canonical.display(),
                        attempt,
                        MAX_BUSY_ATTEMPTS,
                        backoff
                    );
                    std::thread::sleep(backoff);
                }
                Err(e) => return Err(e),
            }
        }

        Err(DbError::Busy {
            attempt: MAX_BUSY_ATTEMPTS,
            max_attempts: MAX_BUSY_ATTEMPTS,
        })
    }

    /// Disconnect and remove a connection from the pool.
    ///
    /// Calls `close()` on the adapter if this pool holds the last reference.
    pub fn disconnect(&self, db_path: &Path) -> Result<()> {
        let canonical = db_path
            .canonicalize()
            .unwrap_or_else(|_| db_path.to_path_buf());

        if let Some((path, adapter)) = self.connections.remove(&canonical) {
            info!("ConnectionPool: disconnected {}", path.display());
            if let Ok(mut adapter) = Arc::try_unwrap(adapter) {
                let _ = adapter.close();
            }
        }
        Ok(())
    }

    /// Get the number of active connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// List all connected database paths.
    pub fn connected_paths(&self) -> Vec<PathBuf> {
        self.connections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Remove all connections from the pool.
    pub fn disconnect_all(&self) {
        self.connections.clear();
        info!("ConnectionPool: disconnected all connections");
    }

    /// Attempt to open a database connection.
    fn try_open(&self, db_path: &Path) -> Result<DbAdapter> {
        let mut adapter = DbAdapter::new(Box::new(StubDbBackend::new()));
        adapter.open(db_path)?;
        Ok(adapter)
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_get_or_open() {
        let pool = ConnectionPool::new();
        let path = PathBuf::from("/tmp/test_pool.db");
        let conn = pool.get_or_open(&path).unwrap();
        assert!(conn.is_open());
        assert_eq!(pool.connection_count(), 1);

        // Second call should return the same connection (by Arc)
        let conn2 = pool.get_or_open(&path).unwrap();
        assert_eq!(pool.connection_count(), 1);
        assert!(Arc::ptr_eq(&conn, &conn2));
    }

    #[test]
    fn test_pool_disconnect() {
        let pool = ConnectionPool::new();
        let path = PathBuf::from("/tmp/test_pool2.db");
        pool.get_or_open(&path).unwrap();
        assert_eq!(pool.connection_count(), 1);

        pool.disconnect(&path).unwrap();
        assert_eq!(pool.connection_count(), 0);
    }

    #[test]
    fn test_pool_disconnect_all() {
        let pool = ConnectionPool::new();
        pool.get_or_open(&PathBuf::from("/tmp/a.db")).unwrap();
        pool.get_or_open(&PathBuf::from("/tmp/b.db")).unwrap();
        assert_eq!(pool.connection_count(), 2);

        pool.disconnect_all();
        assert_eq!(pool.connection_count(), 0);
    }
}
