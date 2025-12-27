// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Rename action implementation with pattern support

use async_trait::async_trait;
use chrono::Utc;
use rpa_core::{Action, Event, EventKind, Result, action::ActionResult, Error};
use std::path::PathBuf;
use tracing::info;

/// Action that renames files using a pattern
///
/// Supported pattern variables:
/// - `{name}` - Original filename without extension
/// - `{ext}` - Original extension
/// - `{date}` - Current date (YYYY-MM-DD)
/// - `{time}` - Current time (HH-MM-SS)
/// - `{datetime}` - Combined date and time
/// - `{counter}` - Auto-incrementing counter (for uniqueness)
pub struct RenameAction {
    pattern: String,
}

impl RenameAction {
    /// Create a new rename action
    pub fn new(pattern: String) -> Self {
        Self { pattern }
    }

    fn apply_pattern(&self, source: &PathBuf) -> PathBuf {
        let name = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = source
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let now = Utc::now();
        let date = now.format("%Y-%m-%d").to_string();
        let time = now.format("%H-%M-%S").to_string();
        let datetime = now.format("%Y%m%d_%H%M%S").to_string();

        let mut new_name = self.pattern.clone();
        new_name = new_name.replace("{name}", &name);
        new_name = new_name.replace("{ext}", &ext);
        new_name = new_name.replace("{date}", &date);
        new_name = new_name.replace("{time}", &time);
        new_name = new_name.replace("{datetime}", &datetime);

        // Handle counter for uniqueness
        if new_name.contains("{counter}") {
            let parent = source.parent().unwrap_or(std::path::Path::new("."));
            let mut counter = 1;
            loop {
                let candidate = new_name.replace("{counter}", &counter.to_string());
                let candidate_path = parent.join(&candidate);
                if !candidate_path.exists() || counter > 9999 {
                    new_name = candidate;
                    break;
                }
                counter += 1;
            }
        }

        source.parent().unwrap_or(std::path::Path::new(".")).join(new_name)
    }
}

#[async_trait]
impl Action for RenameAction {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        let source = match &event.kind {
            EventKind::FileCreated { path } | EventKind::FileModified { path } => path,
            _ => {
                return Ok(ActionResult::failure("Rename action only supports file creation/modification events"));
            }
        };

        if !source.exists() {
            return Ok(ActionResult::failure(format!(
                "Source file does not exist: {}",
                source.display()
            )));
        }

        let dest = self.apply_pattern(source);

        if dest == *source {
            return Ok(ActionResult::success("No rename needed (same name)"));
        }

        if dest.exists() {
            return Ok(ActionResult::failure(format!(
                "Destination already exists: {}",
                dest.display()
            )));
        }

        std::fs::rename(source, &dest)?;
        info!("Renamed {} to {}", source.display(), dest.display());

        Ok(ActionResult::success(format!(
            "Renamed to {}",
            dest.display()
        ))
        .with_paths(vec![dest]))
    }

    fn name(&self) -> &str {
        "rename"
    }

    fn validate(&self) -> Result<()> {
        if self.pattern.is_empty() {
            return Err(Error::Config("Rename pattern cannot be empty".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_name_ext() {
        let action = RenameAction::new("{name}_backup.{ext}".to_string());
        let source = PathBuf::from("/tmp/document.pdf");
        let result = action.apply_pattern(&source);
        assert!(result.to_string_lossy().contains("document_backup.pdf"));
    }
}
