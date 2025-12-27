// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Workflow runner that orchestrates watching and action execution

use crate::actions::DynamicAction;
use crate::config::{EventType, RuleConfig, WorkflowConfig};
use crate::watcher::FsWatcher;
use glob::Pattern;
use rpa_core::{Action, Event, EventKind, Result, WorkflowState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Runner that executes a workflow configuration
pub struct WorkflowRunner {
    config: WorkflowConfig,
    state: WorkflowState,
    running: Arc<AtomicBool>,
}

impl WorkflowRunner {
    /// Create a new workflow runner
    pub fn new(config: WorkflowConfig) -> Self {
        let state = WorkflowState::new(&config.workflow.name);
        Self {
            config,
            state,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the current workflow state
    pub fn state(&self) -> &WorkflowState {
        &self.state
    }

    /// Get a handle to stop the runner
    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Run the workflow (blocking)
    pub fn run(&mut self) -> Result<()> {
        info!("Starting workflow: {}", self.config.workflow.name);
        self.state.start();
        self.running.store(true, Ordering::SeqCst);

        // Determine if any watch config wants recursive
        let recursive = self.config.watch.iter().any(|w| w.recursive);
        let mut watcher = FsWatcher::new(recursive)?;

        // Set up watches
        for watch_config in &self.config.watch {
            if !watch_config.path.exists() {
                warn!(
                    "Watch path does not exist, creating: {}",
                    watch_config.path.display()
                );
                std::fs::create_dir_all(&watch_config.path)?;
            }
            watcher.watch(&watch_config.path)?;
        }

        info!(
            "Workflow '{}' is running. Watching {} paths.",
            self.config.workflow.name,
            watcher.watched_paths().len()
        );

        // Main event loop
        while self.running.load(Ordering::SeqCst) {
            if let Some(event) = watcher.next_event() {
                self.state.record_event();
                self.handle_event(&event);
            }
        }

        self.state.stop();
        info!(
            "Workflow '{}' stopped. Events: {}, Actions: {}, Errors: {}",
            self.config.workflow.name,
            self.state.events_processed,
            self.state.actions_executed,
            self.state.error_count
        );

        Ok(())
    }

    /// Handle a single event
    fn handle_event(&mut self, event: &Event) {
        debug!("Handling event: {:?}", event.kind);

        // Clone rules to avoid borrow conflict with mutable self
        let rules: Vec<_> = self.config.rules.iter()
            .filter(|r| r.enabled && Self::rule_matches_static(r, event))
            .cloned()
            .collect();

        for rule in rules {
            info!("Rule '{}' matched event", rule.name);
            self.execute_rule_actions(&rule, event);
        }
    }

    /// Check if a rule matches an event (static version to avoid borrow issues)
    fn rule_matches_static(rule: &RuleConfig, event: &Event) -> bool {
        // Check event type
        let event_type = match &event.kind {
            EventKind::FileCreated { .. } => EventType::Created,
            EventKind::FileModified { .. } => EventType::Modified,
            EventKind::FileDeleted { .. } => EventType::Deleted,
            EventKind::FileRenamed { .. } => EventType::Renamed,
            _ => return false,
        };

        if !rule.events.contains(&event_type) {
            return false;
        }

        // Check file patterns
        if rule.patterns.is_empty() {
            return true; // No patterns = match all
        }

        let path = match &event.kind {
            EventKind::FileCreated { path }
            | EventKind::FileModified { path }
            | EventKind::FileDeleted { path } => path,
            EventKind::FileRenamed { to, .. } => to,
            _ => return false,
        };

        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        for pattern_str in &rule.patterns {
            match Pattern::new(pattern_str) {
                Ok(pattern) => {
                    if pattern.matches(&filename) {
                        return true;
                    }
                }
                Err(e) => {
                    warn!("Invalid glob pattern '{}': {}", pattern_str, e);
                }
            }
        }

        false
    }

    /// Execute all actions for a matched rule
    fn execute_rule_actions(&mut self, rule: &RuleConfig, event: &Event) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        for action_config in &rule.actions {
            let action = DynamicAction::from_config(action_config.clone());

            match runtime.block_on(action.execute(event)) {
                Ok(result) => {
                    self.state.record_action();
                    if result.success {
                        info!(
                            "Action '{}' succeeded: {}",
                            action.name(),
                            result.message
                        );
                    } else {
                        warn!(
                            "Action '{}' failed: {}",
                            action.name(),
                            result.message
                        );
                        self.state.record_error();
                    }
                }
                Err(e) => {
                    error!("Action '{}' error: {}", action.name(), e);
                    self.state.record_error();
                }
            }
        }
    }

    /// Stop the workflow
    pub fn stop(&self) {
        info!("Stopping workflow: {}", self.config.workflow.name);
        self.running.store(false, Ordering::SeqCst);
    }
}
