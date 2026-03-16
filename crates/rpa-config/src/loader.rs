// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Configuration file loader
//!
//! [`ConfigLoader`] is the main entry point for loading configuration from
//! disk. It auto-detects the file format by extension and delegates to the
//! appropriate parser (JSON or Nickel).

use crate::nickel::NickelBridge;
use crate::types::{ConfigError, ConfigFormat, Result};
use std::path::Path;
use tracing::debug;

/// Loads configuration files from disk as [`serde_json::Value`] trees.
///
/// # Examples
///
/// ```no_run
/// use rpa_config::ConfigLoader;
/// use std::path::Path;
///
/// let loader = ConfigLoader::new();
/// let value = loader.load(Path::new("workflow.json")).unwrap();
/// println!("{}", value);
/// ```
#[derive(Debug, Default)]
pub struct ConfigLoader;

impl ConfigLoader {
    /// Create a new loader instance.
    pub fn new() -> Self {
        Self
    }

    /// Load a configuration file, auto-detecting the format from its extension.
    ///
    /// Returns the parsed configuration as a [`serde_json::Value`].
    pub fn load(&self, path: &Path) -> Result<serde_json::Value> {
        let format = self.detect_format(path);
        match format {
            ConfigFormat::Json => self.load_json(path),
            ConfigFormat::Nickel => self.load_nickel(path),
            ConfigFormat::Unknown => {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("<none>")
                    .to_string();
                Err(ConfigError::UnsupportedFormat(ext))
            }
        }
    }

    /// Load a JSON configuration file with detailed error messages.
    ///
    /// On parse failure the error includes the line and column number from
    /// the JSON deserialiser, making it straightforward to locate the issue.
    pub fn load_json(&self, path: &Path) -> Result<serde_json::Value> {
        debug!("Loading JSON config from {}", path.display());
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed {
            path: path.display().to_string(),
            source: e,
        })?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| ConfigError::JsonParse {
                path: path.display().to_string(),
                message: format!("{} (line {}, column {})", e, e.line(), e.column()),
            })?;
        Ok(value)
    }

    /// Load a Nickel configuration file by evaluating it via the CLI.
    ///
    /// The `nickel` binary must be installed and available on `$PATH`.
    /// The Nickel file is exported as JSON and the resulting string is
    /// parsed into a [`serde_json::Value`].
    pub fn load_nickel(&self, path: &Path) -> Result<serde_json::Value> {
        debug!("Loading Nickel config from {}", path.display());
        let bridge = NickelBridge::new();
        let json_str = bridge.evaluate(path)?;
        let value: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| ConfigError::JsonParse {
                path: path.display().to_string(),
                message: format!(
                    "Nickel output is not valid JSON: {} (line {}, column {})",
                    e,
                    e.line(),
                    e.column()
                ),
            })?;
        Ok(value)
    }

    /// Detect the configuration format from a file path's extension.
    pub fn detect_format(&self, path: &Path) -> ConfigFormat {
        ConfigFormat::detect(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_json() {
        let mut tmp = NamedTempFile::with_suffix(".json").unwrap();
        writeln!(tmp, r#"{{"name": "test", "version": 1}}"#).unwrap();
        tmp.flush().unwrap();

        let loader = ConfigLoader::new();
        let value = loader.load_json(tmp.path()).unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["version"], 1);
    }

    #[test]
    fn test_detect_format() {
        let loader = ConfigLoader::new();
        assert_eq!(
            loader.detect_format(Path::new("config.json")),
            ConfigFormat::Json
        );
        assert_eq!(
            loader.detect_format(Path::new("config.ncl")),
            ConfigFormat::Nickel
        );
        assert_eq!(
            loader.detect_format(Path::new("config.toml")),
            ConfigFormat::Unknown
        );
        assert_eq!(
            loader.detect_format(Path::new("no_ext")),
            ConfigFormat::Unknown
        );
    }
}
