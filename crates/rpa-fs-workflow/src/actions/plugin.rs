// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Plugin action wrapper

use async_trait::async_trait;
use rpa_core::{Action, Event, Result, action::ActionResult, Error};
use rpa_plugin::{PluginContext, PluginHost};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Wrapper that executes plugin actions
pub struct PluginActionWrapper {
    plugin_id: String,
    action_name: String,
    config: HashMap<String, serde_json::Value>,
    host: Option<Arc<PluginHost>>,
}

impl PluginActionWrapper {
    /// Create a new plugin action wrapper
    pub fn new(
        plugin_id: String,
        action_name: String,
        config: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            plugin_id,
            action_name,
            config,
            host: None,
        }
    }

    /// Set the plugin host
    pub fn with_host(mut self, host: Arc<PluginHost>) -> Self {
        self.host = Some(host);
        self
    }
}

#[async_trait]
impl Action for PluginActionWrapper {
    async fn execute(&self, event: &Event) -> Result<ActionResult> {
        let host = self.host.as_ref().ok_or_else(|| {
            Error::ActionFailed {
                action: self.name().to_string(),
                reason: "Plugin host not configured".to_string(),
            }
        })?;

        // Create plugin context from event
        let mut ctx = PluginContext::new(event.clone());
        for (key, value) in &self.config {
            ctx = ctx.with_config(key.clone(), value.clone());
        }

        debug!(
            "Executing plugin action: {}::{}",
            self.plugin_id, self.action_name
        );

        // Execute the plugin action
        match host.execute_action(&self.plugin_id, &self.action_name, &ctx) {
            Ok(result) => Ok(result.into_action_result()),
            Err(e) => Err(Error::ActionFailed {
                action: self.name().to_string(),
                reason: e.to_string(),
            }),
        }
    }

    fn name(&self) -> &str {
        &self.action_name
    }

    fn validate(&self) -> Result<()> {
        if self.plugin_id.is_empty() {
            return Err(Error::Config("Plugin ID cannot be empty".into()));
        }
        if self.action_name.is_empty() {
            return Err(Error::Config("Action name cannot be empty".into()));
        }
        Ok(())
    }
}
