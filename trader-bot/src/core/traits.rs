use crate::core::types::*;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Broker Trait ────────────────────────────────────────────────────

#[async_trait]
pub trait Broker: Send + Sync {
    fn name(&self) -> &str;
    fn broker_kind(&self) -> BrokerKind;

    async fn candles(
        &self,
        instrument: &str,
        interval: CandleInterval,
        count: u32,
    ) -> Result<Vec<Candle>>;
    async fn last_price(&self, instrument: &str) -> Result<f64>;
    async fn order_book(&self, instrument: &str, depth: u32) -> Result<OrderBook>;
    async fn liquidity(&self, instrument: &str, depth: u32) -> Result<LiquidityInfo>;

    async fn place_order(&self, request: OrderRequest) -> Result<OrderResponse>;
    /// Place a broker-side stop order (StopLoss / TakeProfit).
    ///
    /// Default impl returns an error - brokers that don't support native stop orders
    /// (e.g. Mock) opt out, and callers must keep an in-memory stop as a fallback.
    async fn place_stop_order(&self, _request: StopOrderRequest) -> Result<OrderResponse> {
        Err(anyhow::anyhow!(
            "broker {} does not support native stop orders",
            self.name()
        ))
    }
    async fn cancel_order(&self, order_id: &str) -> Result<()>;
    async fn get_orders(&self, instrument: Option<&str>) -> Result<Vec<OrderResponse>>;
    async fn get_order_status(&self, order_id: &str) -> Result<OrderStatus>;

    async fn portfolio(&self) -> Result<PortfolioView>;
    async fn balance(&self) -> Result<f64>;
    async fn position(&self, instrument: &str) -> Result<Option<PositionView>>;

    fn account_id(&self) -> &str;
}

// ─── DataSource Trait ────────────────────────────────────────────────

#[async_trait]
pub trait DataSource: Send + Sync {
    fn name(&self) -> &str;
    fn source_kind(&self) -> DataSourceKind;

    async fn candles(
        &self,
        ticker: &str,
        interval: CandleInterval,
        days: u32,
    ) -> Result<Vec<Candle>>;
    async fn find_instrument(&self, query: &str) -> Result<Vec<InstrumentInfo>>;
    async fn instruments(&self, kind: Option<InstrumentKind>) -> Result<Vec<InstrumentInfo>>;
}

// ─── Strategy Trait ──────────────────────────────────────────────────

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> StrategyKind;

    async fn on_start(&mut self, broker: &dyn Broker) -> Result<()>;
    async fn analyze(&self, broker: &dyn Broker, instrument: &str) -> Result<Vec<Signal>>;
    async fn on_tick(&mut self, broker: &dyn Broker) -> Result<()>;
    fn validate(&self) -> Result<()>;
}

// ─── Market Regime ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketRegime {
    Trending,
    Ranging,
    Volatile,
    Quiet,
}

impl MarketRegime {
    pub const fn weight_adjustment(&self) -> f64 {
        match self {
            MarketRegime::Trending => 1.2,
            MarketRegime::Ranging => 1.0,
            MarketRegime::Volatile => 0.7,
            MarketRegime::Quiet => 0.9,
        }
    }
}

// ─── Optimizer Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub param_ranges: Vec<ParamRange>,
    pub metric: OptimizationMetric,
    pub method: OptimizationMethod,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationMetric {
    SharpeRatio,
    TotalReturn,
    WinRate,
    CalmarRatio,
    ProfitFactor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationMethod {
    GridSearch,
    RandomSearch,
}

#[derive(Debug, Clone)]
pub struct OptimizationTrial {
    pub params: HashMap<String, f64>,
    pub score: f64,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct OptimizationReport {
    pub strategy_name: String,
    pub config: OptimizerConfig,
    pub best_params: HashMap<String, f64>,
    pub best_score: f64,
    pub trials: Vec<OptimizationTrial>,
    pub total_time_ms: u64,
}
