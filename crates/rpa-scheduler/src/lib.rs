// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! RPA Scheduler — Cron-like task scheduling for RPA Elysium
//!
//! Evaluates cron expressions and dispatches scheduled events to the
//! event bus or directly to workflow runners.

#![forbid(unsafe_code)]
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Timelike, Utc};
use rpa_core::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// A simplified cron expression: minute hour day-of-month month day-of-week
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExpr {
    /// Raw expression string
    pub raw: String,
    minutes: CronField,
    hours: CronField,
    days_of_month: CronField,
    months: CronField,
    days_of_week: CronField,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CronField {
    Any,
    Value(u32),
    Range(u32, u32),
    List(Vec<u32>),
}

impl CronField {
    fn matches(&self, value: u32) -> bool {
        match self {
            Self::Any => true,
            Self::Value(v) => *v == value,
            Self::Range(lo, hi) => value >= *lo && value <= *hi,
            Self::List(values) => values.contains(&value),
        }
    }

    fn parse(s: &str, min: u32, max: u32) -> Result<Self> {
        let s = s.trim();
        if s == "*" {
            return Ok(Self::Any);
        }
        if s.contains(',') {
            let values: std::result::Result<Vec<u32>, _> =
                s.split(',').map(|v| v.trim().parse::<u32>()).collect();
            let values = values.map_err(|e| {
                rpa_core::Error::Config(format!("Invalid cron list '{}': {}", s, e))
            })?;
            for v in &values {
                if *v < min || *v > max {
                    return Err(rpa_core::Error::Config(format!(
                        "Cron value {} out of range {}-{}",
                        v, min, max
                    )));
                }
            }
            return Ok(Self::List(values));
        }
        if s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() != 2 {
                return Err(rpa_core::Error::Config(format!(
                    "Invalid cron range: '{}'",
                    s
                )));
            }
            let lo: u32 = parts[0].trim().parse().map_err(|e| {
                rpa_core::Error::Config(format!("Invalid range start '{}': {}", parts[0], e))
            })?;
            let hi: u32 = parts[1].trim().parse().map_err(|e| {
                rpa_core::Error::Config(format!("Invalid range end '{}': {}", parts[1], e))
            })?;
            if lo < min || hi > max || lo > hi {
                return Err(rpa_core::Error::Config(format!(
                    "Cron range {}-{} out of bounds {}-{}",
                    lo, hi, min, max
                )));
            }
            return Ok(Self::Range(lo, hi));
        }
        let value: u32 = s
            .parse()
            .map_err(|e| rpa_core::Error::Config(format!("Invalid cron value '{}': {}", s, e)))?;
        if value < min || value > max {
            return Err(rpa_core::Error::Config(format!(
                "Cron value {} out of range {}-{}",
                value, min, max
            )));
        }
        Ok(Self::Value(value))
    }
}

impl CronExpr {
    /// Parse a cron expression (5 fields: minute hour dom month dow)
    pub fn parse(expr: &str) -> Result<Self> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(rpa_core::Error::Config(format!(
                "Cron expression must have 5 fields, got {}: '{}'",
                fields.len(),
                expr
            )));
        }
        Ok(Self {
            raw: expr.to_string(),
            minutes: CronField::parse(fields[0], 0, 59)?,
            hours: CronField::parse(fields[1], 0, 23)?,
            days_of_month: CronField::parse(fields[2], 1, 31)?,
            months: CronField::parse(fields[3], 1, 12)?,
            days_of_week: CronField::parse(fields[4], 0, 6)?,
        })
    }

    /// Check if a datetime matches this expression
    pub fn matches(&self, dt: &DateTime<Utc>) -> bool {
        self.minutes.matches(dt.minute())
            && self.hours.matches(dt.hour())
            && self.days_of_month.matches(dt.day())
            && self.months.matches(dt.month())
            && self
                .days_of_week
                .matches(dt.weekday().num_days_from_sunday())
    }
}

/// A scheduled entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub name: String,
    pub cron: CronExpr,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
}

impl ScheduleEntry {
    pub fn new(name: impl Into<String>, cron_expr: &str) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            cron: CronExpr::parse(cron_expr)?,
            enabled: true,
            last_run: None,
        })
    }

    pub fn should_fire(&self, now: &DateTime<Utc>) -> bool {
        if !self.enabled || !self.cron.matches(now) {
            return false;
        }
        if let Some(last) = &self.last_run {
            if last.minute() == now.minute() && last.hour() == now.hour() && last.day() == now.day()
            {
                return false;
            }
        }
        true
    }
}

/// Trait for schedulable tasks
#[async_trait]
pub trait ScheduledTask: Send + Sync {
    async fn execute(&self) -> Result<rpa_core::action::ActionResult>;
    fn name(&self) -> &str;
}

/// The scheduler
pub struct Scheduler {
    entries: Vec<(ScheduleEntry, Box<dyn ScheduledTask>)>,
    running: Arc<AtomicBool>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn add(&mut self, entry: ScheduleEntry, task: Box<dyn ScheduledTask>) {
        info!("Scheduler: added '{}' ({})", entry.name, entry.cron.raw);
        self.entries.push((entry, task));
    }

    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    pub async fn run(&mut self) {
        self.running.store(true, Ordering::SeqCst);
        info!("Scheduler: started with {} entries", self.entries.len());

        while self.running.load(Ordering::SeqCst) {
            let now = Utc::now();
            for (entry, task) in &mut self.entries {
                if entry.should_fire(&now) {
                    info!("Scheduler: firing '{}'", entry.name);
                    entry.last_run = Some(now);
                    match task.execute().await {
                        Ok(result) if result.success => {
                            debug!("Scheduler: '{}' ok: {}", entry.name, result.message);
                        }
                        Ok(result) => {
                            warn!("Scheduler: '{}' failed: {}", entry.name, result.message);
                        }
                        Err(e) => warn!("Scheduler: '{}' error: {}", entry.name, e),
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        info!("Scheduler: stopped");
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_cron_every_minute() {
        let expr = CronExpr::parse("* * * * *").expect("TODO: handle error");
        let dt = Utc.with_ymd_and_hms(2026, 3, 17, 10, 30, 0).expect("TODO: handle error");
        assert!(expr.matches(&dt));
    }

    #[test]
    fn test_cron_weekday_morning() {
        let expr = CronExpr::parse("0 9 * * 1-5").expect("TODO: handle error");
        let monday = Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).expect("TODO: handle error");
        assert!(expr.matches(&monday));
        let sunday = Utc.with_ymd_and_hms(2026, 3, 15, 9, 0, 0).expect("TODO: handle error");
        assert!(!expr.matches(&sunday));
    }

    #[test]
    fn test_cron_list() {
        let expr = CronExpr::parse("0,30 * * * *").expect("TODO: handle error");
        let at_0 = Utc.with_ymd_and_hms(2026, 3, 17, 10, 0, 0).expect("TODO: handle error");
        let at_15 = Utc.with_ymd_and_hms(2026, 3, 17, 10, 15, 0).expect("TODO: handle error");
        assert!(expr.matches(&at_0));
        assert!(!expr.matches(&at_15));
    }

    #[test]
    fn test_cron_invalid() {
        assert!(CronExpr::parse("* * *").is_err());
        assert!(CronExpr::parse("60 * * * *").is_err());
    }

    #[test]
    fn test_schedule_entry() {
        let entry = ScheduleEntry::new("test", "* * * * *").expect("TODO: handle error");
        let now = Utc::now();
        assert!(entry.should_fire(&now));
    }

    #[test]
    fn test_no_double_fire() {
        let mut entry = ScheduleEntry::new("test", "* * * * *").expect("TODO: handle error");
        let now = Utc::now();
        assert!(entry.should_fire(&now));
        entry.last_run = Some(now);
        assert!(!entry.should_fire(&now));
    }
}
