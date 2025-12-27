// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Move action implementation

use async_trait::async_trait;
use rpa_core::{Action, Event, EventKind, Result, action::ActionResult, Error};
use std::path::PathBuf;
use tracing::{debug, info};

/// Action that moves files to a destination
pub struct MoveAction {
    destination: PathBuf,
    overwrite: bool,
}

impl MoveAction {
    /// Create a new move action
    pub fn new(destination: PathBuf, overwrite: bool) -> Self {
        Self {
            destination,
            overwrite,
        }
    }
}

#[async_trait]
impl Action for MoveAction {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        let source = match &event.kind {
            EventKind::FileCreated { path } | EventKind::FileModified { path } => path,
            _ => {
                return Ok(ActionResult::failure("Move action only supports file creation/modification events"));
            }
        };

        if !source.exists() {
            return Ok(ActionResult::failure(format!(
                "Source file does not exist: {}",
                source.display()
            )));
        }

        let dest = self.destination.join(source.file_name().unwrap_or_default());

        if dest.exists() && !self.overwrite {
            return Ok(ActionResult::failure(format!(
                "Destination already exists and overwrite is disabled: {}",
                dest.display()
            )));
        }

        // Ensure destination directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        debug!("Moving {} to {}", source.display(), dest.display());

        // Try rename first (atomic on same filesystem)
        if std::fs::rename(source, &dest).is_err() {
            // Fall back to copy + delete for cross-filesystem moves
            std::fs::copy(source, &dest)?;
            std::fs::remove_file(source)?;
        }

        info!("Moved {} to {}", source.display(), dest.display());
        Ok(ActionResult::success(format!(
            "Moved to {}",
            dest.display()
        ))
        .with_paths(vec![dest]))
    }

    fn name(&self) -> &str {
        "move"
    }

    fn validate(&self) -> Result<()> {
        if self.destination.as_os_str().is_empty() {
            return Err(Error::Config("Move destination cannot be empty".into()));
        }
        Ok(())
    }
}
