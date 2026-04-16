// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! RPA Resources — Resource allocation and pooling for RPA Elysium
//!
//! Manages shared resources (connections, file handles, semaphores) across
//! concurrent workflow executions. Prevents resource exhaustion and ensures
//! fair allocation.

#![forbid(unsafe_code)]
use async_trait::async_trait;
use rpa_core::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, info};

/// A named resource pool with bounded concurrency
pub struct ResourcePool {
    name: String,
    semaphore: Arc<Semaphore>,
    max_permits: usize,
    active_count: Arc<Mutex<usize>>,
}

impl ResourcePool {
    /// Create a new resource pool with a maximum number of concurrent leases
    pub fn new(name: impl Into<String>, max_concurrent: usize) -> Self {
        let name = name.into();
        info!(
            "ResourcePool '{}': created with {} permits",
            name, max_concurrent
        );
        Self {
            name,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_permits: max_concurrent,
            active_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Acquire a resource lease (blocks until a permit is available)
    pub async fn acquire(&self) -> Result<ResourceLease> {
        debug!("ResourcePool '{}': acquiring permit", self.name);
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| rpa_core::Error::Other(anyhow::anyhow!("Semaphore closed: {}", e)))?;

        let mut count = self.active_count.lock().await;
        *count += 1;
        let current = *count;
        debug!(
            "ResourcePool '{}': permit acquired ({}/{})",
            self.name, current, self.max_permits
        );

        Ok(ResourceLease {
            pool_name: self.name.clone(),
            _permit: permit,
            active_count: self.active_count.clone(),
        })
    }

    /// Try to acquire a resource lease without blocking
    pub fn try_acquire(&self) -> Option<ResourceLease> {
        match self.semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                // We can't async lock here, so we skip the count update
                Some(ResourceLease {
                    pool_name: self.name.clone(),
                    _permit: permit,
                    active_count: self.active_count.clone(),
                })
            }
            Err(_) => None,
        }
    }

    /// Number of currently active leases
    pub async fn active_leases(&self) -> usize {
        *self.active_count.lock().await
    }

    /// Maximum concurrent leases allowed
    pub fn max_concurrent(&self) -> usize {
        self.max_permits
    }

    /// Number of available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Pool name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A lease on a resource pool — dropping releases the permit
pub struct ResourceLease {
    pool_name: String,
    _permit: tokio::sync::OwnedSemaphorePermit,
    active_count: Arc<Mutex<usize>>,
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        // Decrement active count (best effort — can't async in Drop)
        if let Ok(mut count) = self.active_count.try_lock() {
            *count = count.saturating_sub(1);
        }
        debug!("ResourcePool '{}': permit released", self.pool_name);
    }
}

/// Manages multiple named resource pools
pub struct ResourceManager {
    pools: HashMap<String, Arc<ResourcePool>>,
}

impl ResourceManager {
    /// Create a new empty resource manager
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    /// Register a resource pool
    pub fn register_pool(&mut self, pool: ResourcePool) {
        let name = pool.name().to_string();
        info!("ResourceManager: registered pool '{}'", name);
        self.pools.insert(name, Arc::new(pool));
    }

    /// Get a reference to a named pool
    pub fn pool(&self, name: &str) -> Option<&Arc<ResourcePool>> {
        self.pools.get(name)
    }

    /// Acquire a lease from a named pool
    pub async fn acquire(&self, pool_name: &str) -> Result<ResourceLease> {
        let pool = self.pools.get(pool_name).ok_or_else(|| {
            rpa_core::Error::Config(format!("Resource pool '{}' not found", pool_name))
        })?;
        pool.acquire().await
    }

    /// List all registered pool names
    pub fn pool_names(&self) -> Vec<&str> {
        self.pools.keys().map(|s| s.as_str()).collect()
    }

    /// Get a status summary of all pools
    pub fn status_summary(&self) -> Vec<PoolStatus> {
        self.pools
            .values()
            .map(|p| PoolStatus {
                name: p.name().to_string(),
                max_concurrent: p.max_concurrent(),
                available: p.available_permits(),
            })
            .collect()
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a resource pool
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// Pool name
    pub name: String,
    /// Maximum concurrent leases
    pub max_concurrent: usize,
    /// Currently available permits
    pub available: usize,
}

/// Trait for resources that need pool management
#[async_trait]
pub trait PooledResource: Send + Sync {
    /// Resource type name
    fn resource_type(&self) -> &str;
    /// Whether the resource is still valid
    fn is_valid(&self) -> bool;
    /// Reset the resource for reuse
    async fn reset(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_acquire_release() {
        let pool = ResourcePool::new("test", 2);
        assert_eq!(pool.available_permits(), 2);

        let lease1 = pool.acquire().await.expect("TODO: handle error");
        assert_eq!(pool.available_permits(), 1);

        let lease2 = pool.acquire().await.expect("TODO: handle error");
        assert_eq!(pool.available_permits(), 0);

        drop(lease1);
        assert_eq!(pool.available_permits(), 1);

        drop(lease2);
        assert_eq!(pool.available_permits(), 2);
    }

    #[tokio::test]
    async fn test_pool_try_acquire() {
        let pool = ResourcePool::new("test", 1);

        let lease = pool.try_acquire();
        assert!(lease.is_some());

        let lease2 = pool.try_acquire();
        assert!(lease2.is_none());

        drop(lease);
        let lease3 = pool.try_acquire();
        assert!(lease3.is_some());
    }

    #[tokio::test]
    async fn test_resource_manager() {
        let mut manager = ResourceManager::new();
        manager.register_pool(ResourcePool::new("connections", 5));
        manager.register_pool(ResourcePool::new("file-handles", 10));

        assert_eq!(manager.pool_names().len(), 2);
        assert!(manager.pool("connections").is_some());
        assert!(manager.pool("nonexistent").is_none());

        let lease = manager.acquire("connections").await.expect("TODO: handle error");
        assert_eq!(manager.pool("connections").expect("TODO: handle error").available_permits(), 4);
        drop(lease);
    }

    #[tokio::test]
    async fn test_resource_manager_unknown_pool() {
        let manager = ResourceManager::new();
        let result = manager.acquire("nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_status() {
        let mut manager = ResourceManager::new();
        manager.register_pool(ResourcePool::new("test", 3));

        let statuses = manager.status_summary();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "test");
        assert_eq!(statuses[0].max_concurrent, 3);
        assert_eq!(statuses[0].available, 3);
    }
}
