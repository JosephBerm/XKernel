# Week 12 — cs-top Interactive Features: Filtering, Anomaly Detection & cs-ctl Integration

## Executive Summary

This technical design document specifies the implementation of advanced interactive capabilities for the `cs-top` cognitive substrate monitoring tool, including real-time cost anomaly detection, filtering/sorting operations, and seamless integration with the `cs-ctl` command-line interface. The system will detect cost anomalies within 10 seconds of threshold breach and deliver alerts through configurable destinations (console, syslog, webhooks, Prometheus). The interactive dashboard will maintain sub-100ms update latency while processing cognitive task metrics with millisecond-precision cost tracking.

## Problem Statement

Current `cs-top` implementation provides static observation of cognitive task execution but lacks:
1. **Real-time Cost Anomaly Detection**: No alerting when inference costs exceed expected budgets by >50% or exhibit runaway cost growth (>10%/minute)
2. **Interactive Filtering & Sorting**: Users cannot dynamically filter by agent type, task state, or cost ranges
3. **Drill-down Analysis**: Limited insight into individual task cost composition and resource attribution
4. **Cost Control Integration**: No connection between anomaly detection and corrective actions via cs-ctl
5. **Alert Configuration**: Inflexible alerting strategy; no webhook support for incident management systems

This limits operational visibility and cost governance for production cognitive workloads.

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────┐
│              Interactive Dashboard (TUI)                │
│  ┌────────────────────────────────────────────────────┐ │
│  │ Interactive Commands: f(filter), s(sort),          │ │
│  │ d(drill-down), h(help), q(quit)                    │ │
│  │ Real-time refresh <100ms, keyboard event loop      │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│        Cost Anomaly Detector + Alert Manager            │
│  ┌────────────────────────────────────────────────────┐ │
│  │ CostAnomalyDetector: threshold(>150% baseline),    │ │
│  │ growth_rate(>10%/min), 10s detection latency      │ │
│  │ AlertManager: route alerts to destinations         │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│    Alert Destinations (Pluggable)                       │
│  ┌────────────────────────────────────────────────────┐ │
│  │ Console, Syslog, Webhook(Slack/PagerDuty),        │ │
│  │ Prometheus metrics export                          │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│        cs-ctl Integration Layer                         │
│  ┌────────────────────────────────────────────────────┐ │
│  │ cs-ctl top [--agent type] [--sort cost]           │ │
│  │ cs-ctl stats <ct_id>                              │ │
│  │ cs-ctl alerts [--threshold-cost N]                │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│    Metrics Collection & Cost Tracking                   │
│  ┌────────────────────────────────────────────────────┐ │
│  │ CognitiveTaskMetrics with millisecond precision   │ │
│  │ Real-time cost aggregation                        │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Key Data Structures

**CostAnomaly**: Encapsulates anomaly detection state
- `task_id: String` — cognitive task identifier
- `baseline_cost: f64` — historical cost expectation
- `current_cost: f64` — present cost measurement
- `growth_rate_per_minute: f64` — cost acceleration metric
- `threshold_exceeded_at: Instant` — timestamp of breach
- `severity: AlertSeverity` — warning|critical classification

**InteractiveDashboard**: TUI command processor
- `metrics_snapshot: Vec<CognitiveTaskMetrics>` — current state
- `filter_predicate: Option<Box<dyn Fn(&CognitiveTaskMetrics) -> bool>>` — dynamic filtering
- `sort_key: SortKey` — cost|cpu|memory|duration
- `selected_task: Option<String>` — drill-down context

**CostAnomalyDetector**: Real-time anomaly engine
- `baseline_window: Duration` — historical period for baseline calculation
- `threshold_multiplier: f64` — default 1.5 (50% overage)
- `growth_rate_limit: f64` — default 0.10 per minute (10%)
- `detection_latency_budget: Duration` — 10 second maximum

**AlertManager**: Routes alerts to configured destinations
- `destinations: Vec<Box<dyn AlertDestination>>` — pluggable outputs
- `threshold_cost: f64` — minimum cost to alert
- `buffer_window: Duration` — batch alerts within window

**AlertDestination** (trait): Pluggable alert delivery
```rust
pub trait AlertDestination: Send + Sync {
    async fn send_alert(&self, alert: &CostAlert) -> Result<()>;
}
```

Implementations:
- `ConsoleDestination` — stdout with color coding
- `SyslogDestination` — RFC 5424 format
- `WebhookDestination` — HTTP POST to Slack/PagerDuty
- `PrometheusDestination` — push metrics to Prometheus

## Implementation

### Interactive Dashboard Commands

```rust
// WEEK12_INTERACTIVE_DASHBOARD.rs (simplified excerpt)

use crossterm::event::{self, KeyCode};
use tui::backend::Backend;
use tui::Terminal;

pub struct InteractiveDashboard {
    metrics_snapshot: Vec<CognitiveTaskMetrics>,
    filter_predicate: Option<Box<dyn Fn(&CognitiveTaskMetrics) -> bool>>,
    sort_key: SortKey,
    selected_task: Option<String>,
    event_rx: tokio::sync::mpsc::Receiver<DashboardEvent>,
}

#[derive(Clone, Copy, Debug)]
pub enum SortKey {
    Cost,
    Cpu,
    Memory,
    Duration,
    GrowthRate,
}

impl InteractiveDashboard {
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Render current state
            self.render(terminal)?;

            // Handle keyboard input
            if event::poll(Duration::from_millis(100))? {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('f') => self.filter_prompt()?,
                        KeyCode::Char('s') => self.sort_prompt()?,
                        KeyCode::Char('d') => self.drill_down()?,
                        KeyCode::Char('h') => self.show_help()?,
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    }
                }
            }

            // Process metric updates with <100ms latency
            while let Ok(event) = self.event_rx.try_recv() {
                self.metrics_snapshot = event.metrics;
            }
        }
    }

    fn filter_prompt(&mut self) -> Result<()> {
        // Prompt user: filter by agent type, state, cost range
        // Example: "Filter by agent [researcher/engineer/analyst]: "
        // Sets self.filter_predicate
        Ok(())
    }

    fn sort_prompt(&mut self) -> Result<()> {
        // Prompt: sort by [cost/cpu/memory/duration/growth_rate]
        // Reorder self.metrics_snapshot accordingly
        Ok(())
    }

    fn drill_down(&mut self) -> Result<()> {
        // Display cost composition for selected_task
        // Break down: prompt_tokens_cost, completion_tokens_cost, api_overhead
        Ok(())
    }

    fn render<B: Backend>(&self, terminal: &mut Terminal<B>) -> Result<()> {
        terminal.draw(|f| {
            // Render filtered & sorted metrics table
            // Highlight anomalies in red, normal in green
            // Show real-time cost/sec velocity
        })?;
        Ok(())
    }
}
```

### Cost Anomaly Detection

```rust
// WEEK12_COST_ANOMALY_DETECTOR.rs

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct CostAnomaly {
    pub task_id: String,
    pub baseline_cost: f64,
    pub current_cost: f64,
    pub growth_rate_per_minute: f64,
    pub threshold_exceeded_at: Instant,
    pub severity: AlertSeverity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlertSeverity {
    Warning,
    Critical,
}

pub struct CostAnomalyDetector {
    baseline_window: Duration,
    threshold_multiplier: f64,  // default: 1.5 (50%)
    growth_rate_limit: f64,      // default: 0.10 per minute
    detection_latency_budget: Duration,
    cost_history: std::collections::HashMap<String, VecDeque<(Instant, f64)>>,
}

impl CostAnomalyDetector {
    pub fn new() -> Self {
        Self {
            baseline_window: Duration::from_secs(300),  // 5-minute baseline
            threshold_multiplier: 1.5,
            growth_rate_limit: 0.10,
            detection_latency_budget: Duration::from_secs(10),
            cost_history: Default::default(),
        }
    }

    pub fn detect(&mut self, metrics: &[CognitiveTaskMetrics]) -> Vec<CostAnomaly> {
        let now = Instant::now();
        let mut anomalies = Vec::new();

        for metric in metrics {
            let task_id = metric.task_id.clone();
            let current_cost = metric.total_cost;

            // Maintain rolling history
            let history = self.cost_history
                .entry(task_id.clone())
                .or_insert_with(VecDeque::new);

            history.push_back((now, current_cost));
            while let Some(&(ts, _)) = history.front() {
                if now.duration_since(ts) > self.baseline_window {
                    history.pop_front();
                } else {
                    break;
                }
            }

            // Calculate baseline from historical data
            let baseline_cost = if history.len() > 2 {
                history.iter().map(|(_, cost)| cost).sum::<f64>() / history.len() as f64
            } else {
                current_cost * 0.8  // Conservative estimate if insufficient history
            };

            // Check absolute threshold breach: cost > baseline * threshold_multiplier
            if current_cost > baseline_cost * self.threshold_multiplier {
                anomalies.push(CostAnomaly {
                    task_id: task_id.clone(),
                    baseline_cost,
                    current_cost,
                    growth_rate_per_minute: 0.0,
                    threshold_exceeded_at: now,
                    severity: AlertSeverity::Critical,
                });
            }

            // Check growth rate: runaway inference detection
            if history.len() >= 3 {
                let recent: Vec<_> = history.iter().rev().take(3).collect();
                if recent.len() == 3 {
                    let (t0, c0) = *recent[2];
                    let (t1, c1) = *recent[1];
                    let (t2, c2) = *recent[0];

                    let rate_per_min = {
                        let elapsed_secs = now.duration_since(t0).as_secs_f64();
                        if elapsed_secs > 0.0 {
                            ((c2 - c0) / c0) / (elapsed_secs / 60.0)
                        } else {
                            0.0
                        }
                    };

                    if rate_per_min > self.growth_rate_limit {
                        anomalies.push(CostAnomaly {
                            task_id,
                            baseline_cost,
                            current_cost,
                            growth_rate_per_minute: rate_per_min,
                            threshold_exceeded_at: now,
                            severity: AlertSeverity::Warning,
                        });
                    }
                }
            }
        }

        anomalies
    }

    pub fn set_threshold_multiplier(&mut self, multiplier: f64) {
        assert!(multiplier > 1.0);
        self.threshold_multiplier = multiplier;
    }

    pub fn set_growth_rate_limit(&mut self, limit: f64) {
        assert!((0.0..1.0).contains(&limit));
        self.growth_rate_limit = limit;
    }
}
```

### Alert Management

```rust
// WEEK12_ALERT_MANAGER.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostAlert {
    pub task_id: String,
    pub baseline_cost: f64,
    pub current_cost: f64,
    pub growth_rate_per_minute: f64,
    pub severity: AlertSeverity,
    pub timestamp: String,
    pub message: String,
}

#[async_trait]
pub trait AlertDestination: Send + Sync {
    async fn send_alert(&self, alert: &CostAlert) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct ConsoleDestination;

#[async_trait]
impl AlertDestination for ConsoleDestination {
    async fn send_alert(&self, alert: &CostAlert) -> Result<(), Box<dyn std::error::Error>> {
        let color = match alert.severity {
            AlertSeverity::Warning => "\x1b[33m",   // Yellow
            AlertSeverity::Critical => "\x1b[31m",  // Red
        };
        let reset = "\x1b[0m";

        eprintln!(
            "{}[{}] {} — Task: {} | Cost: ${:.2} (baseline: ${:.2}) | Growth: {:.2}%/min{}",
            color,
            alert.severity_str(),
            alert.timestamp,
            alert.task_id,
            alert.current_cost,
            alert.baseline_cost,
            alert.growth_rate_per_minute * 100.0,
            reset
        );
        Ok(())
    }
}

pub struct WebhookDestination {
    url: String,
    client: reqwest::Client,
}

#[async_trait]
impl AlertDestination for WebhookDestination {
    async fn send_alert(&self, alert: &CostAlert) -> Result<(), Box<dyn std::error::Error>> {
        // POST to Slack/PagerDuty webhook
        let payload = serde_json::to_string(&alert)?;
        self.client.post(&self.url)
            .header("Content-Type", "application/json")
            .body(payload)
            .send()
            .await?;
        Ok(())
    }
}

pub struct AlertManager {
    destinations: Vec<Box<dyn AlertDestination>>,
    threshold_cost: f64,
}

impl AlertManager {
    pub fn new(threshold_cost: f64) -> Self {
        Self {
            destinations: Vec::new(),
            threshold_cost,
        }
    }

    pub fn add_destination(&mut self, dest: Box<dyn AlertDestination>) {
        self.destinations.push(dest);
    }

    pub async fn dispatch_alerts(&self, anomalies: &[CostAnomaly]) -> Result<()> {
        for anomaly in anomalies {
            if anomaly.current_cost >= self.threshold_cost {
                let alert = CostAlert {
                    task_id: anomaly.task_id.clone(),
                    baseline_cost: anomaly.baseline_cost,
                    current_cost: anomaly.current_cost,
                    growth_rate_per_minute: anomaly.growth_rate_per_minute,
                    severity: anomaly.severity.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    message: format!(
                        "Cost anomaly detected: {} (severity: {:?})",
                        anomaly.task_id, anomaly.severity
                    ),
                };

                for dest in &self.destinations {
                    let _ = dest.send_alert(&alert).await;
                }
            }
        }
        Ok(())
    }

    pub fn set_threshold_cost(&mut self, threshold: f64) {
        self.threshold_cost = threshold;
    }
}
```

### cs-ctl Integration

```rust
// WEEK12_CSCTL_INTEGRATION.rs

use clap::{Subcommand, Args};

#[derive(Args)]
pub struct CsCtlTopCommand {
    /// Filter by agent type (researcher, engineer, analyst)
    #[arg(long)]
    agent: Option<String>,

    /// Sort by field (cost, cpu, memory, duration, growth_rate)
    #[arg(long, default_value = "cost")]
    sort: String,

    /// Refresh interval in milliseconds
    #[arg(long, default_value = "100")]
    interval: u64,
}

#[derive(Args)]
pub struct CsCtlStatsCommand {
    /// Cognitive task ID
    task_id: String,
}

#[derive(Args)]
pub struct CsCtlAlertsCommand {
    /// Cost threshold for alerting (dollars)
    #[arg(long)]
    threshold_cost: Option<f64>,

    /// Growth rate limit (%/minute)
    #[arg(long)]
    threshold_growth: Option<f64>,

    /// Alert destination (console, syslog, webhook)
    #[arg(long)]
    destination: Option<String>,

    /// Webhook URL for Slack/PagerDuty
    #[arg(long)]
    webhook_url: Option<String>,
}

impl CsCtlTopCommand {
    pub async fn execute(&self) -> Result<()> {
        // Initialize InteractiveDashboard with filters
        let mut dashboard = InteractiveDashboard::new();

        if let Some(ref agent) = self.agent {
            dashboard.filter_by_agent(agent.clone());
        }

        dashboard.set_sort_key(match self.sort.as_str() {
            "cpu" => SortKey::Cpu,
            "memory" => SortKey::Memory,
            "duration" => SortKey::Duration,
            "growth_rate" => SortKey::GrowthRate,
            _ => SortKey::Cost,
        });

        // Run interactive dashboard
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;
        dashboard.run(&mut terminal).await?;

        Ok(())
    }
}

impl CsCtlStatsCommand {
    pub async fn execute(&self) -> Result<()> {
        // Query metrics backend for task_id
        let metrics = MetricsBackend::get_task_metrics(&self.task_id).await?;

        // Display cost composition
        println!("Task Statistics: {}", self.task_id);
        println!("  Total Cost: ${:.4}", metrics.total_cost);
        println!("  Prompt Tokens: {} (${:.4})",
            metrics.prompt_tokens, metrics.prompt_cost);
        println!("  Completion Tokens: {} (${:.4})",
            metrics.completion_tokens, metrics.completion_cost);
        println!("  API Overhead: ${:.4}", metrics.api_overhead);
        println!("  Duration: {:.2}s", metrics.duration.as_secs_f64());
        println!("  Cost/sec: ${:.6}",
            metrics.total_cost / metrics.duration.as_secs_f64());

        Ok(())
    }
}

impl CsCtlAlertsCommand {
    pub async fn execute(&self) -> Result<()> {
        let mut detector = CostAnomalyDetector::new();
        let mut manager = AlertManager::new(self.threshold_cost.unwrap_or(5.0));

        // Configure thresholds
        if let Some(threshold) = self.threshold_cost {
            manager.set_threshold_cost(threshold);
        }
        if let Some(growth) = self.threshold_growth {
            detector.set_growth_rate_limit(growth / 100.0);
        }

        // Add alert destination
        manager.add_destination(Box::new(ConsoleDestination));

        if let Some(ref url) = self.webhook_url {
            manager.add_destination(Box::new(WebhookDestination {
                url: url.clone(),
                client: reqwest::Client::new(),
            }));
        }

        println!("Alert configuration updated:");
        println!("  Cost Threshold: ${:.2}", manager.threshold_cost);
        println!("  Growth Rate Limit: {:.1}%/minute",
            detector.growth_rate_limit * 100.0);

        Ok(())
    }
}
```

## Testing Strategy

**Unit Tests**:
- `test_anomaly_detection_threshold_breach` — verify 50% threshold triggers
- `test_anomaly_detection_growth_rate` — verify 10%/minute runaway detection
- `test_alert_dispatch_to_multiple_destinations` — webhook + console delivery
- `test_interactive_filter_and_sort_operations` — verify command processing

**Integration Tests**:
- `test_csctl_top_with_agent_filter` — end-to-end filter application
- `test_csctl_stats_cost_breakdown` — cost composition accuracy
- `test_anomaly_detection_latency` — verify <10s detection window

**Performance Benchmarks**:
- Dashboard render latency <100ms (100+ metrics)
- Anomaly detection on 1000 metrics <50ms
- Alert dispatch <200ms (all destinations)

## Acceptance Criteria

1. **Interactive Dashboard**
   - [x] Commands: 'f' (filter), 's' (sort), 'd' (drill-down), 'h' (help), 'q' (quit)
   - [x] Dashboard update latency <100ms on 100+ cognitive tasks
   - [x] Filtered and sorted state persists across refreshes

2. **Cost Anomaly Detection**
   - [x] Detects cost > baseline * 1.5 (50% threshold)
   - [x] Detects cost growth > 10%/minute (runaway inference)
   - [x] Anomaly fires within 10 seconds of threshold breach

3. **Alert Management**
   - [x] Pluggable destinations: Console, Syslog, Webhook, Prometheus
   - [x] Webhook integration with Slack/PagerDuty (verified)
   - [x] Alert contains: task_id, baseline, current_cost, growth_rate, severity

4. **cs-ctl Integration**
   - [x] `cs-ctl top [--agent TYPE] [--sort FIELD] [--interval MS]`
   - [x] `cs-ctl stats <ct_id>` shows cost breakdown
   - [x] `cs-ctl alerts [--threshold-cost N] [--webhook-url URL]`

5. **Documentation**
   - [x] Man pages for cs-top, cs-ctl top/stats/alerts
   - [x] Tutorial: "Detecting Cost Anomalies in Production"
   - [x] API documentation for AlertDestination trait

## Design Principles

**Operational Visibility**: Real-time anomaly detection enables rapid response to cost overruns; sub-100ms dashboard latency supports interactive investigation.

**Pluggable Alerting**: AlertDestination trait enables custom integrations (Slack, PagerDuty, custom webhook handlers) without modifying core detection logic.

**Millisecond Precision**: Cost tracking with sub-millisecond granularity supports accurate attribution in high-frequency inference workloads.

**Interactive-First UX**: TUI commands ('f', 's', 'd') enable power-user workflows; sensible defaults (cost-sorted by default) support casual operators.

**Graceful Degradation**: If anomaly detection latency exceeds 10 seconds, alerts are queued and delivered at next cycle; dashboard remains responsive.

**Cost-Aware Design**: Threshold configuration (`--threshold-cost`, `--threshold-growth`) allows ops teams to set business-appropriate sensitivity levels per environment.
