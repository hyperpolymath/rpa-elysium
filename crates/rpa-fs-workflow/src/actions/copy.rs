// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Copy action implementation

use async_trait::async_trait;
use rpa_core::{Action, Event, EventKind, Result, action::ActionResult, Error};
use std::path::PathBuf;
use tracing::{debug, info};

/// Action that copies files to a destination
pub struct CopyAction {
    destination: PathBuf,
    overwrite: bool,
    preserve_structure: bool,
}

impl CopyAction {
    /// Create a new copy action
    pub fn new(destination: PathBuf, overwrite: bool, preserve_structure: bool) -> Self {
        Self {
            destination,
            overwrite,
            preserve_structure,
        }
    }

    fn get_dest_path(&self, source: &PathBuf) -> PathBuf {
        if self.preserve_structure {
            // Preserve directory structure under destination
            self.destination.join(source.file_name().unwrap_or_default())
        } else {
            self.destination.join(source.file_name().unwrap_or_default())
        }
    }
}

#[async_trait]
impl Action for CopyAction {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        let source = match &event.kind {
            EventKind::FileCreated { path } | EventKind::FileModified { path } => path,
            _ => {
                return Ok(ActionResult::failure("Copy action only supports file creation/modification events"));
            }
        };

        if !source.exists() {
            return Ok(ActionResult::failure(format!(
                "Source file does not exist: {}",
                source.display()
            )));
        }

        let dest = self.get_dest_path(source);

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

        debug!("Copying {} to {}", source.display(), dest.display());
        std::fs::copy(source, &dest)?;

        info!("Copied {} to {}", source.display(), dest.display());
        Ok(ActionResult::success(format!(
            "Copied to {}",
            dest.display()
        ))
        .with_paths(vec![dest]))
    }

    fn name(&self) -> &str {
        "copy"
    }

    fn validate(&self) -> Result<()> {
        if self.destination.as_os_str().is_empty() {
            return Err(Error::Config("Copy destination cannot be empty".into()));
        }
        Ok(())
    }
}
