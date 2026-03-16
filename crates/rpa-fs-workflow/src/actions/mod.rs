// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Action handlers for filesystem operations

mod archive;
mod copy;
mod delete;
mod move_file;
mod plugin;
mod rename;

pub use archive::ArchiveAction;
pub use copy::CopyAction;
pub use delete::DeleteAction;
pub use move_file::MoveAction;
pub use plugin::PluginActionWrapper;
pub use rename::RenameAction;

use async_trait::async_trait;
use rpa_core::{action::ActionResult, Action, Event, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for filesystem actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionConfig {
    /// Copy file to destination
    Copy {
        destination: PathBuf,
        #[serde(default)]
        overwrite: bool,
        #[serde(default)]
        preserve_structure: bool,
    },
    /// Move file to destination
    Move {
        destination: PathBuf,
        #[serde(default)]
        overwrite: bool,
    },
    /// Archive file(s) to a compressed archive
    Archive {
        destination: PathBuf,
        #[serde(default = "default_archive_format")]
        format: ArchiveFormat,
        #[serde(default)]
        delete_source: bool,
    },
    /// Delete the file
    Delete {
        #[serde(default)]
        to_trash: bool,
    },
    /// Rename file using pattern
    Rename { pattern: String },
    /// Execute a plugin action
    Plugin {
        /// Plugin ID
        plugin: String,
        /// Action name
        action: String,
        /// Plugin configuration
        #[serde(default)]
        config: HashMap<String, serde_json::Value>,
    },
}

fn default_archive_format() -> ArchiveFormat {
    ArchiveFormat::TarGz
}

/// Archive formats supported
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    #[default]
    TarGz,
    Zip,
}

/// Dynamic action that can be created from config
pub struct DynamicAction {
    inner: Box<dyn Action>,
}

impl DynamicAction {
    /// Create a new dynamic action from config
    pub fn from_config(config: ActionConfig) -> Self {
        let inner: Box<dyn Action> = match config {
            ActionConfig::Copy {
                destination,
                overwrite,
                preserve_structure,
            } => Box::new(CopyAction::new(destination, overwrite, preserve_structure)),
            ActionConfig::Move {
                destination,
                overwrite,
            } => Box::new(MoveAction::new(destination, overwrite)),
            ActionConfig::Archive {
                destination,
                format,
                delete_source,
            } => Box::new(ArchiveAction::new(destination, format, delete_source)),
            ActionConfig::Delete { to_trash } => Box::new(DeleteAction::new(to_trash)),
            ActionConfig::Rename { pattern } => Box::new(RenameAction::new(pattern)),
            ActionConfig::Plugin {
                plugin,
                action,
                config,
            } => Box::new(PluginActionWrapper::new(plugin, action, config)),
        };
        Self { inner }
    }
}

#[async_trait]
impl Action for DynamicAction {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        self.inner.execute(event).await
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn validate(&self) -> Result<()> {
        self.inner.validate()
    }
}
