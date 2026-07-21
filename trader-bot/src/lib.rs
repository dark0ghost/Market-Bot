pub mod agent;
pub mod analysis;
pub mod backtest;
pub mod client;
pub mod config;
pub mod error;
pub mod execution;
pub mod instrument;
pub mod mcp;
pub mod provider;
pub mod scanner;
pub mod scheduler;
pub mod storage;
pub mod strategy;
pub mod stream;
pub mod telemetry;
pub mod utils;

// ─── New Architecture Modules ────────────────────────────────────────

/// Broker-agnostic core types and traits
pub mod core;

/// Broker implementations (Tinkoff, Mock, etc.)
pub mod broker;

/// Data source implementations (Tinkoff, Finam, etc.)
pub mod datasource;

/// Strategy parameter optimizer
pub mod optimizer;

/// Web dashboard API
pub mod api;

/// ONNX-based ML inference (FinBERT, etc.)
pub mod ml_inference;

// ─── Re-exports for backward compatibility ───────────────────────────

pub use analysis::{NewsArticle, NewsSentiment, Sentiment};
pub use backtest::{BacktestConfig, BacktestResult, backtest_grid, run_backtest};
pub use error::{BotError, NewsAnalysisError, OrderError, StrategyError};
pub use execution::{
    OrderAction, OrderResult, OrderStatus, PositionManager, SignalRecord, TradeJournal,
    TradeRecord, TradingExecutor,
};
pub use strategy::{GridLevel, GridState, GridStrategy, OrderSide};

pub use analysis::fundamental::{
    CompanyRating, DividendMetrics, FinancialHealthMetrics, FundamentalAnalysis,
    FundamentalAnalyzer, GrowthMetrics, ProfitabilityMetrics, ValuationMetrics,
};
pub use analysis::technical::{
    BollingerValues, MacdValues, Recommendation, TechnicalAnalysis, TechnicalAnalyzer, Trend,
    VolumeAnalysis,
};
