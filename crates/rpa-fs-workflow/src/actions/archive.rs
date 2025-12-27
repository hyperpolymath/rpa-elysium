// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Archive action implementation

use super::ArchiveFormat;
use async_trait::async_trait;
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use rpa_core::{Action, Event, EventKind, Result, action::ActionResult, Error};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, info};

/// Action that archives files
pub struct ArchiveAction {
    destination: PathBuf,
    format: ArchiveFormat,
    delete_source: bool,
}

impl ArchiveAction {
    /// Create a new archive action
    pub fn new(destination: PathBuf, format: ArchiveFormat, delete_source: bool) -> Self {
        Self {
            destination,
            format,
            delete_source,
        }
    }

    fn generate_archive_name(&self, source: &PathBuf) -> PathBuf {
        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive".to_string());

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let extension = match self.format {
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::Zip => "zip",
        };

        self.destination.join(format!("{}_{}.{}", stem, timestamp, extension))
    }

    fn create_tar_gz(&self, source: &PathBuf, archive_path: &PathBuf) -> Result<()> {
        let file = File::create(archive_path)?;
        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(encoder);

        let file_name = source
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        tar.append_path_with_name(source, &file_name)?;
        tar.finish()?;

        Ok(())
    }

    fn create_zip(&self, source: &PathBuf, archive_path: &PathBuf) -> Result<()> {
        let file = File::create(archive_path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let file_name = source
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        zip.start_file(&file_name, options)
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        let mut source_file = File::open(source)?;
        let mut buffer = Vec::new();
        source_file.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;

        zip.finish().map_err(|e| Error::Io(std::io::Error::other(e)))?;

        Ok(())
    }
}

#[async_trait]
impl Action for ArchiveAction {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        let source = match &event.kind {
            EventKind::FileCreated { path } | EventKind::FileModified { path } => path,
            _ => {
                return Ok(ActionResult::failure("Archive action only supports file creation/modification events"));
            }
        };

        if !source.exists() {
            return Ok(ActionResult::failure(format!(
                "Source file does not exist: {}",
                source.display()
            )));
        }

        // Ensure destination directory exists
        std::fs::create_dir_all(&self.destination)?;

        let archive_path = self.generate_archive_name(source);
        debug!("Archiving {} to {}", source.display(), archive_path.display());

        match self.format {
            ArchiveFormat::TarGz => self.create_tar_gz(source, &archive_path)?,
            ArchiveFormat::Zip => self.create_zip(source, &archive_path)?,
        }

        if self.delete_source {
            std::fs::remove_file(source)?;
            info!(
                "Archived and deleted {} to {}",
                source.display(),
                archive_path.display()
            );
        } else {
            info!("Archived {} to {}", source.display(), archive_path.display());
        }

        Ok(ActionResult::success(format!(
            "Archived to {}",
            archive_path.display()
        ))
        .with_paths(vec![archive_path]))
    }

    fn name(&self) -> &str {
        "archive"
    }

    fn validate(&self) -> Result<()> {
        if self.destination.as_os_str().is_empty() {
            return Err(Error::Config("Archive destination cannot be empty".into()));
        }
        Ok(())
    }
}
