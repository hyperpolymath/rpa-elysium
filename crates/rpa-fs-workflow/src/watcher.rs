// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Filesystem watcher implementation using notify

use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    Config, Event as NotifyEvent, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use rpa_core::{Event, EventKind as RpaEventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Filesystem watcher that converts notify events to RPA events
pub struct FsWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<NotifyEvent, notify::Error>>,
    watched_paths: Vec<PathBuf>,
    recursive: bool,
}

impl FsWatcher {
    /// Create a new filesystem watcher
    pub fn new(recursive: bool) -> rpa_core::Result<Self> {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.send(res) {
                    error!("Failed to send watch event: {}", e);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map_err(|e| rpa_core::Error::Watch(e.to_string()))?;

        Ok(Self {
            watcher,
            receiver: rx,
            watched_paths: Vec::new(),
            recursive,
        })
    }

    /// Add a path to watch
    pub fn watch(&mut self, path: impl AsRef<Path>) -> rpa_core::Result<()> {
        let path = path.as_ref().to_path_buf();
        let mode = if self.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher
            .watch(&path, mode)
            .map_err(|e| rpa_core::Error::Watch(format!("Failed to watch {}: {}", path.display(), e)))?;

        info!("Watching path: {}", path.display());
        self.watched_paths.push(path);
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> rpa_core::Result<()> {
        let path = path.as_ref();
        self.watcher
            .unwatch(path)
            .map_err(|e| rpa_core::Error::Watch(format!("Failed to unwatch {}: {}", path.display(), e)))?;

        self.watched_paths.retain(|p| p != path);
        info!("Stopped watching: {}", path.display());
        Ok(())
    }

    /// Get the next event, blocking until one is available
    pub fn next_event(&self) -> Option<Event> {
        match self.receiver.recv() {
            Ok(Ok(event)) => self.convert_event(event),
            Ok(Err(e)) => {
                warn!("Watch error: {}", e);
                None
            }
            Err(_) => None, // Channel closed
        }
    }

    /// Try to get the next event without blocking
    pub fn try_next_event(&self) -> Option<Event> {
        match self.receiver.try_recv() {
            Ok(Ok(event)) => self.convert_event(event),
            Ok(Err(e)) => {
                warn!("Watch error: {}", e);
                None
            }
            Err(_) => None,
        }
    }

    /// Convert a notify event to an RPA event
    fn convert_event(&self, event: NotifyEvent) -> Option<Event> {
        let paths = event.paths;
        if paths.is_empty() {
            return None;
        }

        let kind = match event.kind {
            EventKind::Create(CreateKind::File) | EventKind::Create(CreateKind::Any) => {
                RpaEventKind::FileCreated {
                    path: paths[0].clone(),
                }
            }
            EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
                RpaEventKind::FileModified {
                    path: paths[0].clone(),
                }
            }
            EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
                RpaEventKind::FileDeleted {
                    path: paths[0].clone(),
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if paths.len() >= 2 => {
                RpaEventKind::FileRenamed {
                    from: paths[0].clone(),
                    to: paths[1].clone(),
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                RpaEventKind::FileDeleted {
                    path: paths[0].clone(),
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                RpaEventKind::FileCreated {
                    path: paths[0].clone(),
                }
            }
            _ => {
                debug!("Ignoring event kind: {:?}", event.kind);
                return None;
            }
        };

        let source = paths[0].parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

        Some(Event::new(kind, source))
    }

    /// Get list of watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_creation() {
        let watcher = FsWatcher::new(true);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_directory() {
        let dir = tempdir().unwrap();
        let mut watcher = FsWatcher::new(false).unwrap();

        let result = watcher.watch(dir.path());
        assert!(result.is_ok());
        assert_eq!(watcher.watched_paths().len(), 1);
    }
}
