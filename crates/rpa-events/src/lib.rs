// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! RPA Events — Async event bus for RPA Elysium
//!
//! Provides pub/sub event dispatch with lifecycle hooks. Events flow
//! from sources (filesystem watchers, schedulers, HAR queue) through
//! the bus to registered handlers.
//!
//! # Architecture
//!
//! ```text
//! Sources ──→ EventBus ──→ Handlers
//!                │
//!                ├─→ Pre-hooks (filter, transform)
//!                ├─→ Dispatch (fan-out to subscribers)
//!                └─→ Post-hooks (audit, metrics)
//! ```

#![forbid(unsafe_code)]
use async_trait::async_trait;
use rpa_core::{Event, Result};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};

// Groove notification backend — sends voice alerts via Burble's groove
// protocol when event handlers fail. Wire as a post-hook on the EventBus.
// Groove connectors formally verified via Idris2 (Gossamer's Groove.idr).
pub mod groove;

/// Maximum number of events buffered in the bus channel
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Trait for event handlers that subscribe to the bus
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an incoming event
    async fn handle(&self, event: &Event) -> Result<EventOutcome>;

    /// Name of this handler (for logging/metrics)
    fn name(&self) -> &str;

    /// Whether this handler is interested in a given event
    fn accepts(&self, event: &Event) -> bool {
        let _ = event;
        true // Accept all by default
    }
}

/// Outcome of handling an event
#[derive(Debug, Clone)]
pub enum EventOutcome {
    /// Event was processed successfully
    Handled,
    /// Event was skipped (not relevant to this handler)
    Skipped,
    /// Event processing failed but bus should continue
    Failed(String),
}

/// Lifecycle hook that runs before or after event dispatch
#[async_trait]
pub trait EventHook: Send + Sync {
    /// Run the hook on an event. Return None to filter (drop) the event.
    async fn run(&self, event: Event) -> Option<Event>;

    /// Name of this hook
    fn name(&self) -> &str;
}

/// The main event bus — receives events and dispatches to subscribers
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    handlers: Vec<Arc<dyn EventHandler>>,
    pre_hooks: Vec<Arc<dyn EventHook>>,
    post_hooks: Vec<Arc<dyn EventHook>>,
}

impl EventBus {
    /// Create a new event bus with default channel capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Create a new event bus with specified channel capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            handlers: Vec::new(),
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
        }
    }

    /// Subscribe a handler to receive events
    pub fn subscribe(&mut self, handler: Arc<dyn EventHandler>) {
        info!("EventBus: subscribed handler '{}'", handler.name());
        self.handlers.push(handler);
    }

    /// Add a pre-dispatch hook (runs before handlers)
    pub fn add_pre_hook(&mut self, hook: Arc<dyn EventHook>) {
        info!("EventBus: added pre-hook '{}'", hook.name());
        self.pre_hooks.push(hook);
    }

    /// Add a post-dispatch hook (runs after handlers)
    pub fn add_post_hook(&mut self, hook: Arc<dyn EventHook>) {
        info!("EventBus: added post-hook '{}'", hook.name());
        self.post_hooks.push(hook);
    }

    /// Publish an event to the bus
    pub async fn publish(&self, event: Event) -> Result<EventDispatchResult> {
        debug!("EventBus: publishing event {} ({:?})", event.id, event.kind);

        // Run pre-hooks (filter/transform)
        let mut current = event;
        for hook in &self.pre_hooks {
            match hook.run(current).await {
                Some(transformed) => current = transformed,
                None => {
                    debug!("EventBus: event filtered by pre-hook '{}'", hook.name());
                    return Ok(EventDispatchResult {
                        event_id: String::new(),
                        handlers_invoked: 0,
                        handlers_succeeded: 0,
                        handlers_failed: 0,
                        filtered: true,
                    });
                }
            }
        }

        // Dispatch to broadcast channel (non-blocking)
        let _ = self.sender.send(current.clone());

        // Dispatch to handlers
        let mut invoked = 0;
        let mut succeeded = 0;
        let mut failed = 0;

        for handler in &self.handlers {
            if !handler.accepts(&current) {
                continue;
            }
            invoked += 1;
            match handler.handle(&current).await {
                Ok(EventOutcome::Handled) => succeeded += 1,
                Ok(EventOutcome::Skipped) => {}
                Ok(EventOutcome::Failed(reason)) => {
                    debug!("EventBus: handler '{}' failed: {}", handler.name(), reason);
                    failed += 1;
                }
                Err(e) => {
                    debug!("EventBus: handler '{}' error: {}", handler.name(), e);
                    failed += 1;
                }
            }
        }

        // Run post-hooks
        let event_id = current.id.clone();
        let mut post_event = current;
        for hook in &self.post_hooks {
            post_event = match hook.run(post_event).await {
                Some(transformed) => transformed,
                None => break,
            };
        }

        Ok(EventDispatchResult {
            event_id,
            handlers_invoked: invoked,
            handlers_succeeded: succeeded,
            handlers_failed: failed,
            filtered: false,
        })
    }

    /// Get a receiver for raw broadcast events (for external consumers)
    pub fn receiver(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of dispatching an event through the bus
#[derive(Debug, Clone)]
pub struct EventDispatchResult {
    /// ID of the dispatched event
    pub event_id: String,
    /// Number of handlers that were invoked
    pub handlers_invoked: usize,
    /// Number of handlers that succeeded
    pub handlers_succeeded: usize,
    /// Number of handlers that failed
    pub handlers_failed: usize,
    /// Whether the event was filtered by a pre-hook
    pub filtered: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpa_core::{Event, EventKind};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingHandler {
        name: String,
        count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EventHandler for CountingHandler {
        async fn handle(&self, _event: &Event) -> Result<EventOutcome> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(EventOutcome::Handled)
        }
        fn name(&self) -> &str {
            &self.name
        }
    }

    struct FilterHook;

    #[async_trait]
    impl EventHook for FilterHook {
        async fn run(&self, event: Event) -> Option<Event> {
            // Filter out manual events
            match &event.kind {
                EventKind::Manual => None,
                _ => Some(event),
            }
        }
        fn name(&self) -> &str {
            "filter-manual"
        }
    }

    fn test_event() -> Event {
        Event::new(
            EventKind::FileCreated {
                path: PathBuf::from("/tmp/test.txt"),
            },
            "/tmp",
        )
    }

    #[tokio::test]
    async fn test_publish_to_handler() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        bus.subscribe(Arc::new(CountingHandler {
            name: "test".into(),
            count: count.clone(),
        }));

        let result = bus.publish(test_event()).await.expect("TODO: handle error");
        assert_eq!(result.handlers_invoked, 1);
        assert_eq!(result.handlers_succeeded, 1);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_multiple_handlers() {
        let mut bus = EventBus::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        bus.subscribe(Arc::new(CountingHandler {
            name: "h1".into(),
            count: count1.clone(),
        }));
        bus.subscribe(Arc::new(CountingHandler {
            name: "h2".into(),
            count: count2.clone(),
        }));

        bus.publish(test_event()).await.expect("TODO: handle error");
        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_pre_hook_filter() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        bus.add_pre_hook(Arc::new(FilterHook));
        bus.subscribe(Arc::new(CountingHandler {
            name: "test".into(),
            count: count.clone(),
        }));

        // Manual event should be filtered
        let manual_event = Event::new(EventKind::Manual, "manual");
        let result = bus.publish(manual_event).await.expect("TODO: handle error");
        assert!(result.filtered);
        assert_eq!(count.load(Ordering::SeqCst), 0);

        // File event should pass through
        let result = bus.publish(test_event()).await.expect("TODO: handle error");
        assert!(!result.filtered);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_broadcast_receiver() {
        let bus = EventBus::new();
        let mut rx = bus.receiver();

        bus.publish(test_event()).await.expect("TODO: handle error");
        let received = rx.recv().await.expect("TODO: handle error");
        assert!(received.id.starts_with("evt_"));
    }

    #[test]
    fn test_default_bus() {
        let bus = EventBus::default();
        assert_eq!(bus.handler_count(), 0);
    }
}
