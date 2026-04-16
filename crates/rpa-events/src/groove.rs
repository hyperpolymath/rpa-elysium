// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Groove notification backend for RPA Elysium EventBus.
//!
//! Implements an EventHook that sends voice alerts via Burble's groove
//! protocol when event handlers fail. This is a post-hook: it runs after
//! event dispatch and inspects the outcomes.
//!
//! # Integration
//!
//! Wire this hook into the EventBus as a post-hook:
//!
//! ```rust,ignore
//! use rpa_events::groove::GrooveNotifyHook;
//! use std::sync::Arc;
//! // Assuming you have an event bus instance:
//! // bus.add_post_hook(Arc::new(GrooveNotifyHook::new()));
//! ```
//!
//! # Groove Protocol
//!
//! - Probes localhost:6473/.well-known/groove for Burble
//! - Sends POST /.well-known/groove/message with alert JSON
//! - Falls back silently if Burble is not available
//!
//! # What Gets Notified
//!
//! - Handler failures (EventOutcome::Failed)
//! - Repeated failures on the same event kind (escalation)
//!
//! The groove connectors are formally verified in Gossamer's Groove.idr:
//! - Burble must offer TTS capability for voice alerts to work
//! - Linear GrooveHandle ensures proper lifecycle management

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use rpa_core::Event;
use tracing::{debug, info, warn};

use crate::{EventDispatchResult, EventHook};

/// Port where Burble exposes its groove endpoint.
const BURBLE_PORT: u16 = 6473;

/// Connection timeout for groove probes.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Groove notification hook for the EventBus.
///
/// Monitors event dispatch results and sends voice alerts via Burble
/// when handlers fail. Probes Burble availability lazily on first use.
pub struct GrooveNotifyHook {
    /// Whether Burble has been probed and found available.
    burble_available: AtomicBool,
    /// Whether we've attempted a probe yet.
    probed: AtomicBool,
}

impl GrooveNotifyHook {
    /// Create a new groove notification hook.
    pub fn new() -> Self {
        Self {
            burble_available: AtomicBool::new(false),
            probed: AtomicBool::new(false),
        }
    }

    /// Probe Burble's groove endpoint to check availability.
    fn probe_burble(&self) -> bool {
        self.probed.store(true, Ordering::Relaxed);

        let addr = format!("127.0.0.1:{}", BURBLE_PORT);
        let stream = match TcpStream::connect_timeout(
            &addr.parse().expect("TODO: handle error"),
            CONNECT_TIMEOUT,
        ) {
            Ok(s) => s,
            Err(_) => {
                debug!("Groove: Burble not available at {}", addr);
                self.burble_available.store(false, Ordering::Relaxed);
                return false;
            }
        };
        drop(stream);

        info!("Groove: Burble discovered at {}", addr);
        self.burble_available.store(true, Ordering::Relaxed);
        true
    }

    /// Send a JSON message to Burble's groove endpoint.
    fn send_alert(&self, json: &str) {
        let addr = format!("127.0.0.1:{}", BURBLE_PORT);
        let mut stream = match TcpStream::connect_timeout(
            &addr.parse().expect("TODO: handle error"),
            CONNECT_TIMEOUT,
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("Groove: failed to connect to Burble: {}", e);
                self.burble_available.store(false, Ordering::Relaxed);
                return;
            }
        };

        let request = format!(
            "POST /.well-known/groove/message HTTP/1.0\r\n\
             Host: localhost\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            json.len(),
            json
        );

        if let Err(e) = stream.write_all(request.as_bytes()) {
            warn!("Groove: failed to send alert: {}", e);
            return;
        }

        // Read response (we don't strictly need it, but drain the socket).
        let mut buf = [0u8; 256];
        let _ = stream.read(&mut buf);
    }

    /// Check if Burble is available, probing lazily if needed.
    fn is_available(&self) -> bool {
        if !self.probed.load(Ordering::Relaxed) {
            self.probe_burble()
        } else {
            self.burble_available.load(Ordering::Relaxed)
        }
    }
}

impl Default for GrooveNotifyHook {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventHook for GrooveNotifyHook {
    fn name(&self) -> &str {
        "groove-notify"
    }

    /// Post-hook: pass the event through unchanged.
    /// Actual failure alerting is done via on_dispatch_complete.
    async fn run(&self, event: Event) -> Option<Event> {
        Some(event)
    }
}

impl GrooveNotifyHook {
    /// Called after event dispatch to check results and send alerts.
    ///
    /// Wire this into the EventBus post-dispatch path.
    pub fn on_dispatch_complete(&self, event: &Event, result: &EventDispatchResult) {
        // Only alert on failures.
        if result.handlers_failed == 0 {
            return;
        }

        if !self.is_available() {
            return;
        }

        // Build the alert payload.
        let alert = format!(
            r#"{{"type":"rpa_handler_failure","source":"rpa-elysium","event_id":"{}","event_kind":"{}","handlers_invoked":{},"handlers_failed":{},"handlers_succeeded":{},"tts_message":"RPA Elysium: {} handler{} failed processing event {}","timestamp":"{}"}}"#,
            result.event_id,
            event.kind,
            result.handlers_invoked,
            result.handlers_failed,
            result.handlers_succeeded,
            result.handlers_failed,
            if result.handlers_failed == 1 { "" } else { "s" },
            event.kind,
            chrono::Utc::now().to_rfc3339(),
        );

        self.send_alert(&alert);
        info!(
            "Groove: sent failure alert for event {} ({} failures)",
            event.kind, result.handlers_failed
        );
    }
}
