use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct SignalMetrics {
    pub signals_generated: Arc<AtomicU64>,
    pub signals_executed: Arc<AtomicU64>,
    pub win_rate: Arc<RwLock<f64>>,
    pub total_pnl: Arc<RwLock<f64>>,
}

#[derive(Clone, Default)]
pub struct PerformanceMetrics {
    pub orders_placed: Arc<AtomicU64>,
    pub orders_filled: Arc<AtomicU64>,
    pub orders_rejected: Arc<AtomicU64>,
    pub avg_latency_ms: Arc<RwLock<f64>>,
    pub last_latency_ms: Arc<RwLock<f64>>,
}

#[derive(Clone)]
pub struct StrategyMetrics {
    pub signals: Arc<RwLock<HashMap<String, SignalMetrics>>>,
    pub performance: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
}

impl Default for StrategyMetrics {
    fn default() -> Self {
        StrategyMetrics {
            signals: Arc::new(RwLock::new(HashMap::new())),
            performance: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl StrategyMetrics {
    pub async fn record_signal(&self, strategy: &str) {
        let mut signals = self.signals.write().await;
        let entry = signals
            .entry(strategy.to_string())
            .or_default();
        entry.signals_generated.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn record_execution(&self, strategy: &str, pnl: f64) {
        let mut signals = self.signals.write().await;
        let entry = signals
            .entry(strategy.to_string())
            .or_default();
        entry.signals_executed.fetch_add(1, Ordering::Relaxed);
        let mut total = entry.total_pnl.write().await;
        *total += pnl;
    }

    pub async fn record_order_latency(&self, strategy: &str, latency: Duration) {
        let latency_ms = latency.as_secs_f64() * 1000.0;
        let mut perf = self.performance.write().await;
        let entry = perf
            .entry(strategy.to_string())
            .or_default();
        let mut avg = entry.avg_latency_ms.write().await;
        let mut last = entry.last_latency_ms.write().await;
        let count = entry.orders_placed.load(Ordering::Relaxed) as f64;
        *avg = (*avg * count + latency_ms) / (count + 1.0);
        *last = latency_ms;
    }

    pub async fn record_order_status(&self, strategy: &str, filled: bool, rejected: bool) {
        let mut perf = self.performance.write().await;
        let entry = perf
            .entry(strategy.to_string())
            .or_default();
        entry.orders_placed.fetch_add(1, Ordering::Relaxed);
        if filled {
            entry.orders_filled.fetch_add(1, Ordering::Relaxed);
        }
        if rejected {
            entry.orders_rejected.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn snapshot(&self) -> HashMap<String, HashMap<String, f64>> {
        let signals = self.signals.read().await;
        let perf = self.performance.read().await;
        let mut result = HashMap::new();

        for (strategy, s) in signals.iter() {
            let mut entry = HashMap::new();
            entry.insert(
                "signals_generated".to_string(),
                s.signals_generated.load(Ordering::Relaxed) as f64,
            );
            entry.insert(
                "signals_executed".to_string(),
                s.signals_executed.load(Ordering::Relaxed) as f64,
            );
            let wr = s.win_rate.read().await;
            entry.insert("win_rate".to_string(), *wr);
            let pnl = s.total_pnl.read().await;
            entry.insert("total_pnl".to_string(), *pnl);
            drop(wr);
            drop(pnl);

            if let Some(p) = perf.get(strategy) {
                entry.insert(
                    "orders_placed".to_string(),
                    p.orders_placed.load(Ordering::Relaxed) as f64,
                );
                entry.insert(
                    "orders_filled".to_string(),
                    p.orders_filled.load(Ordering::Relaxed) as f64,
                );
                entry.insert(
                    "orders_rejected".to_string(),
                    p.orders_rejected.load(Ordering::Relaxed) as f64,
                );
                let avg = p.avg_latency_ms.read().await;
                entry.insert("avg_latency_ms".to_string(), *avg);
                let last = p.last_latency_ms.read().await;
                entry.insert("last_latency_ms".to_string(), *last);
                drop(avg);
                drop(last);
            }

            result.insert(strategy.clone(), entry);
        }

        result
    }
}

pub struct TelemetryServer {
    bind_addr: String,
    metrics: StrategyMetrics,
}

impl TelemetryServer {
    pub fn new(bind_addr: &str, metrics: StrategyMetrics) -> Self {
        TelemetryServer {
            bind_addr: bind_addr.to_string(),
            metrics,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let metrics = self.metrics.clone();
        let addr: std::net::SocketAddr = self.bind_addr.parse()?;

        let app = axum::Router::new()
            .route("/metrics", axum::routing::get(move || {
                let metrics = metrics.clone();
                async move {
                    let snapshot = metrics.snapshot().await;
                    let mut output = String::new();
                    output.push_str("# HELP ai_trade_bot_signal_signals_generated Signals generated\n");
                    output.push_str("# TYPE ai_trade_bot_signal_signals_generated gauge\n");

                    for (strategy, data) in &snapshot {
                        for (key, value) in data {
                            output.push_str(&format!(
                                "ai_trade_bot_{}{{strategy=\"{}\",key=\"{}\"}} {}\n",
                                "signals", strategy, key, value
                            ));
                        }
                    }

                    output
                }
            }));

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}
