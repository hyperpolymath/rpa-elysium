// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! RPA Filesystem Workflow CLI
//!
//! A command-line tool for running filesystem automation workflows.
//!
//! # Examples
//!
//! Run a workflow from a config file:
//! ```bash
//! rpa-fs run workflow.json
//! ```
//!
//! Generate an example configuration:
//! ```bash
//! rpa-fs init example.json
//! ```
//!
//! Validate a configuration file:
//! ```bash
//! rpa-fs validate workflow.json
//! ```

use clap::{Parser, Subcommand};
use rpa_fs_workflow::{WorkflowConfig, WorkflowRunner};
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(
    name = "rpa-fs",
    about = "RPA Elysium Filesystem Workflow Automation",
    version,
    author
)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a workflow from a configuration file
    Run {
        /// Path to the workflow configuration file (.json or .ncl)
        config: PathBuf,
    },

    /// Generate an example configuration file
    Init {
        /// Output file path (default: workflow.json)
        #[arg(default_value = "workflow.json")]
        output: PathBuf,
    },

    /// Validate a configuration file without running
    Validate {
        /// Path to the configuration file to validate
        config: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    // Set up logging
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .compact()
        .init();

    let result = match cli.command {
        Commands::Run { config } => run_workflow(config),
        Commands::Init { output } => init_workflow(output),
        Commands::Validate { config } => validate_workflow(config),
    };

    if let Err(e) = result {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_workflow(config_path: PathBuf) -> anyhow::Result<()> {
    info!("Loading workflow from: {}", config_path.display());

    let config = WorkflowConfig::load(&config_path)?;
    info!(
        "Loaded workflow '{}' with {} watch paths and {} rules",
        config.workflow.name,
        config.watch.len(),
        config.rules.len()
    );

    let mut runner = WorkflowRunner::new(config);
    let stop_handle = runner.stop_handle();

    // Set up Ctrl+C handler
    let stop_handle_clone = stop_handle.clone();
    ctrlc::set_handler(move || {
        info!("Received interrupt signal, stopping...");
        stop_handle_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    runner.run()?;
    Ok(())
}

fn init_workflow(output_path: PathBuf) -> anyhow::Result<()> {
    if output_path.exists() {
        anyhow::bail!(
            "File already exists: {}. Remove it first or choose a different name.",
            output_path.display()
        );
    }

    let config = WorkflowConfig::example();
    let json = serde_json::to_string_pretty(&config)?;
    std::fs::write(&output_path, json)?;

    info!("Created example workflow configuration: {}", output_path.display());
    info!("Edit the file to customize your workflow, then run with:");
    info!("  rpa-fs run {}", output_path.display());

    Ok(())
}

fn validate_workflow(config_path: PathBuf) -> anyhow::Result<()> {
    info!("Validating: {}", config_path.display());

    let config = WorkflowConfig::load(&config_path)?;

    info!("Configuration is valid!");
    info!("  Workflow: {}", config.workflow.name);
    if let Some(desc) = &config.workflow.description {
        info!("  Description: {}", desc);
    }
    info!("  Watch paths: {}", config.watch.len());
    info!("  Rules: {}", config.rules.len());

    for rule in &config.rules {
        info!(
            "    - {} ({} actions, {} patterns)",
            rule.name,
            rule.actions.len(),
            rule.patterns.len()
        );
    }

    Ok(())
}
