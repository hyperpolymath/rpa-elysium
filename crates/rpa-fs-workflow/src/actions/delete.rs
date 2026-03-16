// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Delete action implementation

use async_trait::async_trait;
use rpa_core::{action::ActionResult, Action, Event, EventKind, Result};
use tracing::info;

/// Action that deletes files
pub struct DeleteAction {
    to_trash: bool,
}

impl DeleteAction {
    /// Create a new delete action
    pub fn new(to_trash: bool) -> Self {
        Self { to_trash }
    }
}

#[async_trait]
impl Action for DeleteAction {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        let source = match &event.kind {
            EventKind::FileCreated { path } | EventKind::FileModified { path } => path,
            _ => {
                return Ok(ActionResult::failure(
                    "Delete action only supports file creation/modification events",
                ));
            }
        };

        if !source.exists() {
            return Ok(ActionResult::success(format!(
                "File already deleted: {}",
                source.display()
            )));
        }

        if self.to_trash {
            // Note: For full trash support, would integrate with trash-rs crate
            // For now, we rename to a .trash suffix as a simple implementation
            let trash_path = source.with_extension(format!(
                "{}.trash",
                source
                    .extension()
                    .map(|e| e.to_string_lossy())
                    .unwrap_or_default()
            ));
            std::fs::rename(source, &trash_path)?;
            info!(
                "Moved to trash: {} -> {}",
                source.display(),
                trash_path.display()
            );
            Ok(ActionResult::success(format!(
                "Moved to trash: {}",
                trash_path.display()
            )))
        } else {
            std::fs::remove_file(source)?;
            info!("Deleted: {}", source.display());
            Ok(ActionResult::success(format!(
                "Deleted: {}",
                source.display()
            )))
        }
    }

    fn name(&self) -> &str {
        "delete"
    }
}
