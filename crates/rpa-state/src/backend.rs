// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Storage backend trait and implementations
//!
//! Defines the [`StateBackend`] trait for pluggable persistence, and provides
//! [`JsonFileBackend`] as a filesystem-based JSON implementation.

use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, instrument};

/// Trait defining a key-value storage backend for state persistence.
///
/// Implementations must be safe to share across async tasks (`Send + Sync`).
/// Keys are slash-separated strings that backends may interpret as hierarchical
/// paths (e.g., `"workflow/my-workflow"` maps to a file path in [`JsonFileBackend`]).
#[async_trait]
pub trait StateBackend: Send + Sync {
    /// Store a value under the given key, overwriting any existing value.
    async fn put(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Retrieve the value stored under the given key, or `None` if absent.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete the value stored under the given key. No-op if key does not exist.
    async fn delete(&self, key: &str) -> Result<()>;

    /// List all keys that start with the given prefix.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;

    /// Return a human-readable name for this backend (e.g., `"json-file"`).
    fn name(&self) -> &str;
}

/// A [`StateBackend`] implementation that stores state as JSON files on disk.
///
/// Keys are mapped to file paths relative to the base directory, with `.json`
/// appended. For example, the key `"workflow/my-workflow"` becomes
/// `<base_dir>/workflow/my-workflow.json`.
///
/// Parent directories are created automatically on write.
#[derive(Debug, Clone)]
pub struct JsonFileBackend {
    /// Root directory for all persisted state files.
    base_dir: PathBuf,
}

impl JsonFileBackend {
    /// Create a new `JsonFileBackend` rooted at the given directory.
    ///
    /// The directory is created (including parents) if it does not already exist.
    pub fn new(base_dir: PathBuf) -> Self {
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir).unwrap_or_else(|err| {
                tracing::warn!(
                    path = %base_dir.display(),
                    error = %err,
                    "Failed to create base directory for JsonFileBackend"
                );
            });
        }
        Self { base_dir }
    }

    /// Resolve a key to its on-disk file path (`base_dir/<key>.json`).
    fn key_to_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", key))
    }
}

#[async_trait]
impl StateBackend for JsonFileBackend {
    #[instrument(skip(self, value), fields(backend = "json-file", key = %key))]
    async fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        let path = self.key_to_path(key);
        debug!(path = %path.display(), "Writing state");

        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&path, value).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(backend = "json-file", key = %key))]
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.key_to_path(key);
        debug!(path = %path.display(), "Reading state");

        match tokio::fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    #[instrument(skip(self), fields(backend = "json-file", key = %key))]
    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.key_to_path(key);
        debug!(path = %path.display(), "Deleting state");

        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    #[instrument(skip(self), fields(backend = "json-file", prefix = %prefix))]
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let search_dir = self.base_dir.join(prefix);
        debug!(dir = %search_dir.display(), "Listing keys");

        let mut keys = Vec::new();

        // Walk the search directory, collecting .json files as keys
        let walk_dir = if search_dir.is_dir() {
            search_dir.clone()
        } else if let Some(parent) = search_dir.parent() {
            if parent.is_dir() {
                parent.to_path_buf()
            } else {
                return Ok(keys);
            }
        } else {
            return Ok(keys);
        };

        let mut read_dir = tokio::fs::read_dir(&walk_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                // Convert path back to a key relative to base_dir
                if let Ok(relative) = path.strip_prefix(&self.base_dir) {
                    let key = relative.with_extension("").to_string_lossy().to_string();
                    if key.starts_with(prefix) {
                        keys.push(key);
                    }
                }
            }
        }

        keys.sort();
        Ok(keys)
    }

    fn name(&self) -> &str {
        "json-file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_json_file_backend_put_get() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());

        let data = b"hello world";
        backend.put("test/key1", data).await.expect("put failed");

        let result = backend.get("test/key1").await.expect("get failed");
        assert_eq!(result, Some(data.to_vec()));
    }

    #[tokio::test]
    async fn test_json_file_backend_get_missing() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());

        let result = backend.get("nonexistent").await.expect("get failed");
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_json_file_backend_delete() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());

        backend.put("deleteme", b"data").await.expect("put failed");
        assert!(backend.get("deleteme").await.expect("TODO: handle error").is_some());

        backend.delete("deleteme").await.expect("delete failed");
        assert_eq!(backend.get("deleteme").await.expect("TODO: handle error"), None);
    }

    #[tokio::test]
    async fn test_json_file_backend_delete_missing() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());

        // Deleting a nonexistent key should succeed silently
        backend
            .delete("nonexistent")
            .await
            .expect("delete of missing key should not fail");
    }

    #[tokio::test]
    async fn test_json_file_backend_list_keys() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());

        backend
            .put("workflow/alpha", b"a")
            .await
            .expect("put failed");
        backend
            .put("workflow/beta", b"b")
            .await
            .expect("put failed");
        backend
            .put("snapshot/alpha", b"s")
            .await
            .expect("put failed");

        let workflow_keys = backend
            .list_keys("workflow/")
            .await
            .expect("list_keys failed");
        assert_eq!(workflow_keys.len(), 2);
        assert!(workflow_keys.contains(&"workflow/alpha".to_string()));
        assert!(workflow_keys.contains(&"workflow/beta".to_string()));

        let snapshot_keys = backend
            .list_keys("snapshot/")
            .await
            .expect("list_keys failed");
        assert_eq!(snapshot_keys.len(), 1);
        assert!(snapshot_keys.contains(&"snapshot/alpha".to_string()));
    }
}
